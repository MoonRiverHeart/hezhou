use crate::callback_types::*;
use crate::script_manager_lite::ScriptManager;
use crate::value_bridge::ScriptValue;
use parking_lot::Mutex;
use std::sync::Arc;

static SCRIPT_MANAGER: Mutex<Option<Arc<Mutex<ScriptManager>>>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub extern "C" fn scripting_init() -> *mut Arc<Mutex<ScriptManager>> {
    let manager = Arc::new(Mutex::new(ScriptManager::new()));
    *SCRIPT_MANAGER.lock() = Some(Arc::clone(&manager));
    Box::into_raw(Box::new(manager))
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_shutdown(manager: *mut Arc<Mutex<ScriptManager>>) {
    if !manager.is_null() {
        unsafe {
            let _ = Box::from_raw(manager);
        }
        *SCRIPT_MANAGER.lock() = None;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_register_sync_callback(
    manager: *mut Arc<Mutex<ScriptManager>>,
    name: *const std::os::raw::c_char,
    callback: SyncCallback,
    descriptor: *const std::os::raw::c_char,
    signature: *const std::os::raw::c_char,
    context: usize,
) {
    if manager.is_null() || name.is_null() {
        return;
    }

    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name)
            .to_string_lossy()
            .into_owned();
        let desc_str = if descriptor.is_null() {
            "".to_string()
        } else {
            std::ffi::CStr::from_ptr(descriptor)
                .to_string_lossy()
                .into_owned()
        };
        let sig_str = if signature.is_null() {
            "".to_string()
        } else {
            std::ffi::CStr::from_ptr(signature)
                .to_string_lossy()
                .into_owned()
        };

        let arc = &*manager;
        let mut mgr = arc.lock();

        let cb_desc =
            crate::callback_types::CallbackDescriptor::new_sync(&name_str, &desc_str, &sig_str);

        mgr.register_sync_callback(name_str, callback, cb_desc, context);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_trigger_sync(
    manager: *mut Arc<Mutex<ScriptManager>>,
    name: *const std::os::raw::c_char,
    arg: ScriptValue,
) -> ScriptValue {
    if manager.is_null() || name.is_null() {
        return ScriptValue::null();
    }

    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name)
            .to_string_lossy()
            .into_owned();
        let arc = &*manager;
        let mgr = arc.lock();

        match mgr.trigger_sync(&name_str, arg) {
            Ok(result) => result,
            Err(_) => ScriptValue::null(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scripting_unregister_callback(
    manager: *mut Arc<Mutex<ScriptManager>>,
    name: *const std::os::raw::c_char,
) {
    if manager.is_null() || name.is_null() {
        return;
    }

    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name)
            .to_string_lossy()
            .into_owned();
        let arc = &*manager;
        let mut mgr = arc.lock();
        mgr.unregister_callback(&name_str);
    }
}
