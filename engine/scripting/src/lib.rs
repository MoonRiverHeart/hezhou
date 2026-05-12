pub mod value_bridge;
pub mod error;
pub mod callback_types;
pub mod callback_registry;
pub mod script_manager_lite;

pub use value_bridge::*;
pub use error::*;
pub use callback_types::*;
pub use callback_registry::*;

use script_manager_lite::ScriptManager;
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn scripting_init() -> *mut ScriptManager {
    let manager = ScriptManager::new();
    Box::into_raw(Box::new(manager))
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_shutdown(manager: *mut ScriptManager) {
    if !manager.is_null() {
        unsafe {
            let mut mgr = Box::from_raw(manager);
            mgr.shutdown();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_register_sync_callback(
    manager: *mut ScriptManager,
    name: *const c_char,
    callback: SyncCallback,
    description: *const c_char,
    signature: *const c_char,
    context: usize,
) {
    if manager.is_null() || name.is_null() {
        return;
    }
    
    unsafe {
        let mgr = &mut *manager;
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap().to_string();
        let desc_str = if description.is_null() {
            ""
        } else {
            std::ffi::CStr::from_ptr(description).to_str().unwrap()
        };
        let sig_str = if signature.is_null() {
            ""
        } else {
            std::ffi::CStr::from_ptr(signature).to_str().unwrap()
        };
        
        let descriptor = CallbackDescriptor::new_sync(&name_str, desc_str, sig_str);
        mgr.register_sync_callback(name_str, callback, descriptor, context);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_register_async_callback(
    manager: *mut ScriptManager,
    name: *const c_char,
    callback: AsyncCallback,
    description: *const c_char,
    context: usize,
) {
    if manager.is_null() || name.is_null() {
        return;
    }
    
    unsafe {
        let mgr = &mut *manager;
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap().to_string();
        let desc_str = if description.is_null() {
            ""
        } else {
            std::ffi::CStr::from_ptr(description).to_str().unwrap()
        };
        
        let descriptor = CallbackDescriptor::new_async(&name_str, desc_str);
        mgr.register_async_callback(name_str, callback, descriptor, context);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_register_task_callback(
    manager: *mut ScriptManager,
    name: *const c_char,
    callback: TaskCallback,
    description: *const c_char,
    supports_progress: bool,
    context: usize,
) {
    if manager.is_null() || name.is_null() {
        return;
    }
    
    unsafe {
        let mgr = &mut *manager;
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap().to_string();
        let desc_str = if description.is_null() {
            ""
        } else {
            std::ffi::CStr::from_ptr(description).to_str().unwrap()
        };
        
        let descriptor = CallbackDescriptor::new_task(&name_str, desc_str, supports_progress);
        mgr.register_task_callback(name_str, callback, descriptor, supports_progress, context);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_trigger_sync(
    manager: *mut ScriptManager,
    name: *const c_char,
    arg: ScriptValue,
) -> ScriptValue {
    if manager.is_null() || name.is_null() {
        return ScriptValue::err("Invalid parameters");
    }
    
    unsafe {
        let mgr = &*manager;
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap();
        match mgr.trigger_sync(name_str, arg) {
            Ok(result) => result,
            Err(e) => ScriptValue::err(&e.to_string()),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_notify_completion(
    manager: *mut ScriptManager,
    result: ScriptValue,
) {
    if !manager.is_null() {
        unsafe {
            let mgr = &mut *manager;
            mgr.callbacks.lock().notify_completion(manager as usize, result);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_notify_progress(
    manager: *mut ScriptManager,
    progress: f32,
) {
    if !manager.is_null() {
        unsafe {
            let mgr = &mut *manager;
            mgr.callbacks.lock().notify_progress(manager as usize, progress);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_unregister_callback(
    manager: *mut ScriptManager,
    name: *const c_char,
) {
    if manager.is_null() || name.is_null() {
        return;
    }
    
    unsafe {
        let mgr = &mut *manager;
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap();
        mgr.unregister_callback(name_str);
    }
}