mod event_types;
mod native_window;
mod event_bus;

pub use event_types::*;
pub use native_window::{NativeWindowContext, OH_NativeWindow};
pub use event_bus::EventBus;

use std::os::raw::c_void;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct HarmonyEngine {
    window_ctx: Arc<Mutex<NativeWindowContext>>,
    event_bus: Arc<Mutex<EventBus>>,
    script_manager: *mut c_void,
}

impl HarmonyEngine {
    pub fn new() -> Self {
        Self {
            window_ctx: Arc::new(Mutex::new(NativeWindowContext::new())),
            event_bus: Arc::new(Mutex::new(EventBus::new())),
            script_manager: std::ptr::null_mut(),
        }
    }
}

impl Default for HarmonyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_engine_init() -> *mut HarmonyEngine {
    let engine = Box::new(HarmonyEngine::new());
    Box::into_raw(engine)
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_engine_shutdown(engine: *mut HarmonyEngine) {
    if !engine.is_null() {
        unsafe {
            Box::from_raw(engine);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_get_window_context(engine: *mut HarmonyEngine) -> *mut NativeWindowContext {
    if engine.is_null() {
        return std::ptr::null_mut();
    }
    
    unsafe {
        let e = &*engine;
        Arc::as_ptr(&e.window_ctx) as *mut NativeWindowContext
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_get_event_bus(engine: *mut HarmonyEngine) -> *mut EventBus {
    if engine.is_null() {
        return std::ptr::null_mut();
    }
    
    unsafe {
        let e = &*engine;
        Arc::as_ptr(&e.event_bus) as *mut EventBus
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_register_event_callback(
    engine: *mut HarmonyEngine,
    event_name: *const std::os::raw::c_char,
    callback: hezhou_scripting::SyncCallback,
    description: *const std::os::raw::c_char,
    context: usize,
) {
    if engine.is_null() || event_name.is_null() {
        return;
    }
    
    unsafe {
        let e = &*engine;
        let name = std::ffi::CStr::from_ptr(event_name).to_str().unwrap_or("");
        let desc = if description.is_null() {
            ""
        } else {
            std::ffi::CStr::from_ptr(description).to_str().unwrap_or("")
        };
        
        let descriptor = hezhou_scripting::CallbackDescriptor::new_sync(name, desc, "");
        e.event_bus.lock().register_event_handler(name, callback, descriptor, context);
    }
}