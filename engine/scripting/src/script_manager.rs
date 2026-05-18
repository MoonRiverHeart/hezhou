use crate::callback_registry::CallbackRegistry;
use crate::callback_types::*;
use crate::error::{ScriptError, ScriptResult};
use crate::value_bridge::ScriptValue;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use wrapped_mono::*;

pub struct ScriptManager {
    domain: Option<Domain>,
    assemblies: HashMap<String, Assembly>,
    callbacks: Arc<Mutex<CallbackRegistry>>,
}

impl ScriptManager {
    pub fn new() -> ScriptResult<Self> {
        // Set Mono assembly path before initialization
        // Mono needs to find mscorlib.dll, System.dll, etc.
        // The lib path should point to the lib directory, Mono will append mono/4.5/ internally
        let mono_lib_path = "C:\\Program Files\\Mono\\lib";
        let mono_config_path = "C:\\Program Files\\Mono\\etc";
        
        let lib_cstr = std::ffi::CString::new(mono_lib_path).unwrap();
        let config_cstr = std::ffi::CString::new(mono_config_path).unwrap();
        
        unsafe {
            wrapped_mono::binds::mono_set_dirs(lib_cstr.as_ptr(), config_cstr.as_ptr());
        }
        
        let domain = jit::init("ScriptDomain", None);
        Ok(Self {
            domain: Some(domain),
            assemblies: HashMap::new(),
            callbacks: Arc::new(Mutex::new(CallbackRegistry::new())),
        })
    }

    pub fn load_script(&mut self, dll_path: &str) -> ScriptResult<String> {
        let domain = self.domain.as_ref().ok_or(ScriptError::NotInitialized)?;
        
        let dll_name = std::ffi::CString::new("mono_ui_thunk_demo").unwrap();
        let target_dll = std::ffi::CString::new("mono_ui_thunk_demo.exe").unwrap();
        
        unsafe {
            wrapped_mono::binds::mono_dllmap_insert(
                std::ptr::null_mut(),
                dll_name.as_ptr(),
                std::ptr::null(),
                target_dll.as_ptr(),
                std::ptr::null(),
            );
        }

        let assembly = domain
            .assembly_open(dll_path)
            .ok_or_else(|| ScriptError::AssemblyNotFound(dll_path.to_string()))?;

        let path_buf = PathBuf::from(dll_path);
        let name = path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let name_without_ext = name.strip_suffix(".dll").unwrap_or(name);

        self.assemblies
            .insert(name_without_ext.to_string(), assembly);
        Ok(name_without_ext.to_string())
    }

    pub fn unload(&mut self, name: &str) {
        self.assemblies.remove(name);
        self.callbacks.lock().unregister(name);
    }

    pub fn reload(&mut self, dll_path: &str) -> ScriptResult<String> {
        let path_buf = PathBuf::from(dll_path);
        let name = path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let name_without_ext = name.strip_suffix(".dll").unwrap_or(name);

        self.assemblies.remove(name_without_ext);

        // Copy DLL to temp path to bypass Mono's assembly cache
        let temp_dir = std::env::temp_dir();
        let temp_dll_name = format!(
            "{}_{}.dll",
            name_without_ext,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let temp_dll_path = temp_dir.join(&temp_dll_name);

        std::fs::copy(dll_path, &temp_dll_path)
            .map_err(|e| ScriptError::InvokeFailed(format!("Failed to copy DLL to temp: {}", e)))?;

        let new_name = self.load_script(temp_dll_path.to_str().unwrap())?;

        Ok(new_name)
    }

    pub fn execute(
        &self,
        assembly_name: &str,
        namespace: &str,
        class_name: &str,
        method_name: &str,
        arg_float: Option<f32>,
        param_count: i32,
    ) -> ScriptResult<ScriptValue> {
        let _domain = self.domain.as_ref().ok_or(ScriptError::NotInitialized)?;
        let assembly = self
            .assemblies
            .get(assembly_name)
            .ok_or_else(|| ScriptError::AssemblyNotFound(assembly_name.to_string()))?;

        let image = assembly.get_image();

        let class = Class::from_name(&image, namespace, class_name)
            .or_else(|| Class::from_name_case(&image, namespace, class_name))
            .ok_or_else(|| ScriptError::ClassNotFound(format!("{}.{}", namespace, class_name)))?;

        let mut iter = std::ptr::null_mut::<std::os::raw::c_void>();
        let mut found_method_ptr: *mut wrapped_mono::binds::MonoMethod = std::ptr::null_mut();
        loop {
            let method_ptr = unsafe {
                wrapped_mono::binds::mono_class_get_methods(
                    class.get_ptr(),
                    &mut iter as *mut *mut std::os::raw::c_void,
                )
            };
            if method_ptr.is_null() {
                break;
            }
            let name_cstr = unsafe { wrapped_mono::binds::mono_method_get_name(method_ptr) };
            let name = unsafe { std::ffi::CStr::from_ptr(name_cstr) }
                .to_str()
                .unwrap_or("?");
            if name == method_name {
                found_method_ptr = method_ptr;
                break;
            }
        }

        if found_method_ptr.is_null() {
            return Err(ScriptError::MethodNotFound(format!(
                "{} ({} params)",
                method_name, param_count
            )));
        }

        let mut params: Vec<*mut std::os::raw::c_void> = Vec::new();
        if param_count > 0 {
            if let Some(float_val) = arg_float {
                let float_ptr = &float_val as *const f32 as *mut std::os::raw::c_void;
                params.push(float_ptr);
            }
        }

        let mut exc_ptr: *mut wrapped_mono::binds::MonoObject = std::ptr::null_mut();
        let result_obj = unsafe {
            wrapped_mono::binds::mono_runtime_invoke(
                found_method_ptr,
                std::ptr::null_mut(),
                if params.is_empty() {
                    std::ptr::null_mut()
                } else {
                    params.as_mut_ptr() as *mut *mut std::os::raw::c_void
                },
                &mut exc_ptr as *mut *mut wrapped_mono::binds::MonoObject,
            )
        };

        if !exc_ptr.is_null() {
            return Err(ScriptError::InvokeFailed(
                "Exception during method call".to_string(),
            ));
        }

        if result_obj.is_null() {
            Ok(ScriptValue::null())
        } else {
            let boxed_value = unsafe { wrapped_mono::binds::mono_object_unbox(result_obj) };
            let float_ptr = boxed_value as *const f32;
            let float_value = unsafe { *float_ptr };
            Ok(ScriptValue::from_float(float_value))
        }
    }

    pub fn execute_with_ptr(
        &self,
        assembly_name: &str,
        namespace: &str,
        class_name: &str,
        method_name: &str,
        ptr_arg: usize,
        param_count: i32,
    ) -> ScriptResult<()> {
        let _domain = self.domain.as_ref().ok_or(ScriptError::NotInitialized)?;
        let assembly = self
            .assemblies
            .get(assembly_name)
            .ok_or_else(|| ScriptError::AssemblyNotFound(assembly_name.to_string()))?;

        let image = assembly.get_image();

        let class = Class::from_name(&image, namespace, class_name)
            .or_else(|| Class::from_name_case(&image, namespace, class_name))
            .ok_or_else(|| ScriptError::ClassNotFound(format!("{}.{}", namespace, class_name)))?;

        let mut iter = std::ptr::null_mut::<std::os::raw::c_void>();
        let mut found_method_ptr: *mut wrapped_mono::binds::MonoMethod = std::ptr::null_mut();
        loop {
            let method_ptr = unsafe {
                wrapped_mono::binds::mono_class_get_methods(
                    class.get_ptr(),
                    &mut iter as *mut *mut std::os::raw::c_void,
                )
            };
            if method_ptr.is_null() {
                break;
            }
            let name_cstr = unsafe { wrapped_mono::binds::mono_method_get_name(method_ptr) };
            let name = unsafe { std::ffi::CStr::from_ptr(name_cstr) }
                .to_str()
                .unwrap_or("?");
            if name == method_name {
                found_method_ptr = method_ptr;
                break;
            }
        }

        if found_method_ptr.is_null() {
            return Err(ScriptError::MethodNotFound(format!(
                "{} ({} params)",
                method_name, param_count
            )));
        }

        let mut params: Vec<*mut std::os::raw::c_void> = Vec::new();
        let mut ptr_storage: usize = ptr_arg;
        if param_count > 0 {
            params.push(&mut ptr_storage as *mut usize as *mut std::os::raw::c_void);
        }

        let mut exc_ptr: *mut wrapped_mono::binds::MonoObject = std::ptr::null_mut();
        unsafe {
            wrapped_mono::binds::mono_runtime_invoke(
                found_method_ptr,
                std::ptr::null_mut(),
                if params.is_empty() {
                    std::ptr::null_mut()
                } else {
                    params.as_mut_ptr() as *mut *mut std::os::raw::c_void
                },
                &mut exc_ptr as *mut *mut wrapped_mono::binds::MonoObject,
            )
        };

        if !exc_ptr.is_null() {
            return Err(ScriptError::InvokeFailed(
                "Exception during method call".to_string(),
            ));
        }

        Ok(())
    }

    pub fn register_sync_callback(
        &mut self,
        name: String,
        callback: SyncCallback,
        descriptor: CallbackDescriptor,
        context: usize,
    ) {
        self.callbacks
            .lock()
            .register_sync(name, callback, descriptor, context);
    }

    pub fn register_async_callback(
        &mut self,
        name: String,
        callback: AsyncCallback,
        descriptor: CallbackDescriptor,
        context: usize,
    ) {
        self.callbacks
            .lock()
            .register_async(name, callback, descriptor, context);
    }

    pub fn register_task_callback(
        &mut self,
        name: String,
        callback: TaskCallback,
        descriptor: CallbackDescriptor,
        supports_progress: bool,
        context: usize,
    ) {
        self.callbacks
            .lock()
            .register_task(name, callback, descriptor, supports_progress, context);
    }

    pub fn trigger_sync(&self, name: &str, arg: ScriptValue) -> ScriptResult<ScriptValue> {
        self.callbacks.lock().trigger_sync(name, arg)
    }

    pub fn trigger_async(
        &self,
        name: &str,
        arg: ScriptValue,
        completion_ptr: usize,
    ) -> ScriptResult<()> {
        self.callbacks
            .lock()
            .trigger_async(name, arg, completion_ptr)
    }

    pub fn trigger_task(
        &self,
        name: &str,
        arg: ScriptValue,
        progress_ptr: usize,
        completion_ptr: usize,
    ) -> ScriptResult<u64> {
        self.callbacks
            .lock()
            .trigger_task(name, arg, progress_ptr, completion_ptr)
    }

    pub fn query_task_progress(&self, id: u64) -> Option<f32> {
        self.callbacks.lock().query_task_progress(id)
    }

    pub fn query_task_result(&self, id: u64) -> Option<Result<ScriptValue, String>> {
        self.callbacks.lock().query_task_result(id)
    }

    pub fn cancel_task(&self, id: u64) -> bool {
        self.callbacks.lock().cancel_task(id)
    }

    pub fn unregister_callback(&mut self, name: &str) {
        self.callbacks.lock().unregister(name);
    }

    pub fn notify_completion(&mut self, completion_ptr: usize, result: ScriptValue) {
        self.callbacks
            .lock()
            .notify_completion(completion_ptr, result);
    }

    pub fn notify_progress(&mut self, progress_ptr: usize, progress: f32) {
        self.callbacks
            .lock()
            .notify_progress(progress_ptr, progress);
    }

    pub fn list_callbacks(&self) -> Vec<CallbackDescriptor> {
        self.callbacks.lock().list_callbacks()
    }

    pub fn shutdown(&mut self) {
        self.assemblies.clear();
        self.callbacks.lock().clear();
        self.domain = None;
    }
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
