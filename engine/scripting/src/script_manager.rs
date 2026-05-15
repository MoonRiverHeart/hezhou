use crate::callback_registry::CallbackRegistry;
use crate::callback_types::*;
use crate::value_bridge::ScriptValue;
use crate::error::{ScriptError, ScriptResult};
use wrapped_mono::*;
use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct ScriptManager {
    domain: Option<Domain>,
    assemblies: HashMap<String, Assembly>,
    callbacks: Arc<Mutex<CallbackRegistry>>,
}

impl ScriptManager {
    pub fn new() -> ScriptResult<Self> {
        let domain = jit::init("ScriptDomain", None);
        Ok(Self {
            domain: Some(domain),
            assemblies: HashMap::new(),
            callbacks: Arc::new(Mutex::new(CallbackRegistry::new())),
        })
    }

    pub fn load_script(&mut self, dll_path: &str) -> ScriptResult<String> {
        let domain = self.domain.as_ref().ok_or(ScriptError::NotInitialized)?;
        
        let assembly = domain.assembly_open(dll_path)
            .ok_or_else(|| ScriptError::AssemblyNotFound(dll_path.to_string()))?;
        
        let name = PathBuf::from(dll_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        self.assemblies.insert(name.clone(), assembly);
        Ok(name)
    }

    pub fn unload(&mut self, name: &str) {
        self.assemblies.remove(name);
        self.callbacks.lock().unregister(name);
    }

    pub fn reload(&mut self, dll_path: &str) -> ScriptResult<String> {
        let name = PathBuf::from(dll_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        self.assemblies.remove(&name);
        self.load_script(dll_path)
    }

    pub fn execute(
        &self,
        assembly_name: &str,
        namespace: &str,
        class_name: &str,
        method_name: &str,
        args: (i32, i32),
        param_count: i32,
    ) -> ScriptResult<ScriptValue> {
        let assembly = self.assemblies.get(assembly_name)
            .ok_or_else(|| ScriptError::AssemblyNotFound(assembly_name.to_string()))?;
        
        let image = assembly.get_image();
        let class = Class::from_name(&image, namespace, class_name)
            .ok_or_else(|| ScriptError::ClassNotFound(format!("{}.{}", namespace, class_name)))?;
        
        let method = Method::get_from_name(&class, method_name, param_count)
            .ok_or_else(|| ScriptError::MethodNotFound(method_name.to_string()))?;
        
        let result = method.invoke(None, args)
            .map_err(|e| ScriptError::InvokeFailed(format!("{:?}", e)))?;
        
        Ok(result.map_or(ScriptValue::null(), |_| ScriptValue::from_int(0)))
    }

    pub fn register_sync_callback(
        &mut self,
        name: String,
        callback: SyncCallback,
        descriptor: CallbackDescriptor,
        context: usize,
    ) {
        self.callbacks.lock().register_sync(name, callback, descriptor, context);
    }

    pub fn register_async_callback(
        &mut self,
        name: String,
        callback: AsyncCallback,
        descriptor: CallbackDescriptor,
        context: usize,
    ) {
        self.callbacks.lock().register_async(name, callback, descriptor, context);
    }

    pub fn register_task_callback(
        &mut self,
        name: String,
        callback: TaskCallback,
        descriptor: CallbackDescriptor,
        supports_progress: bool,
        context: usize,
    ) {
        self.callbacks.lock().register_task(name, callback, descriptor, supports_progress, context);
    }

    pub fn trigger_sync(&self, name: &str, arg: ScriptValue) -> ScriptResult<ScriptValue> {
        self.callbacks.lock().trigger_sync(name, arg)
    }

    pub fn trigger_async(&self, name: &str, arg: ScriptValue, completion_ptr: usize) -> ScriptResult<()> {
        self.callbacks.lock().trigger_async(name, arg, completion_ptr)
    }

    pub fn trigger_task(&self, name: &str, arg: ScriptValue, progress_ptr: usize, completion_ptr: usize) -> ScriptResult<u64> {
        self.callbacks.lock().trigger_task(name, arg, progress_ptr, completion_ptr)
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
        self.callbacks.lock().notify_completion(completion_ptr, result);
    }

    pub fn notify_progress(&mut self, progress_ptr: usize, progress: f32) {
        self.callbacks.lock().notify_progress(progress_ptr, progress);
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