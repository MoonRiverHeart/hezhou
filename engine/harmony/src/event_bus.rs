use crate::event_types::*;
use hezhou_scripting::{CallbackRegistry, CallbackDescriptor, ScriptValue, SyncCallback};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct EventBus {
    callbacks: Arc<Mutex<CallbackRegistry>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(Mutex::new(CallbackRegistry::new())),
        }
    }
    
    pub fn register_event_handler(
        &mut self,
        event_type: &str,
        callback: SyncCallback,
        descriptor: CallbackDescriptor,
        context: usize,
    ) {
        self.callbacks.lock().register_sync(event_type.to_string(), callback, descriptor, context);
    }
    
    pub fn dispatch_touch_event(&self, event: &TouchEvent) {
        let script_value = ScriptValue::from_int(event.action as i32);
        self.callbacks.lock().trigger_sync("OnTouch", script_value).ok();
    }
    
    pub fn dispatch_key_event(&self, event: &KeyEvent) {
        let script_value = ScriptValue::from_int(event.keycode);
        self.callbacks.lock().trigger_sync("OnKey", script_value).ok();
    }
    
    pub fn dispatch_size_event(&self, event: &SizeEvent) {
        let script_value = ScriptValue::from_int(event.width);
        self.callbacks.lock().trigger_sync("OnResize", script_value).ok();
    }
    
    pub fn dispatch_lifecycle_event(&self, event: &LifecycleEvent) {
        let script_value = ScriptValue::from_int(event.state as i32);
        self.callbacks.lock().trigger_sync("OnLifecycle", script_value).ok();
    }
    
    pub fn callbacks(&self) -> Arc<Mutex<CallbackRegistry>> {
        self.callbacks.clone()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_on_touch_event(bus: *mut EventBus, event: *const TouchEvent) {
    if bus.is_null() || event.is_null() {
        return;
    }
    
    let event_bus = unsafe { &*bus };
    event_bus.dispatch_touch_event(unsafe { &*event });
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_on_key_event(bus: *mut EventBus, event: *const KeyEvent) {
    if bus.is_null() || event.is_null() {
        return;
    }
    
    let event_bus = unsafe { &*bus };
    event_bus.dispatch_key_event(unsafe { &*event });
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_on_size_event(bus: *mut EventBus, event: *const SizeEvent) {
    if bus.is_null() || event.is_null() {
        return;
    }
    
    let event_bus = unsafe { &*bus };
    event_bus.dispatch_size_event(unsafe { &*event });
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_on_lifecycle_event(bus: *mut EventBus, event: *const LifecycleEvent) {
    if bus.is_null() || event.is_null() {
        return;
    }
    
    let event_bus = unsafe { &*bus };
    event_bus.dispatch_lifecycle_event(unsafe { &*event });
}