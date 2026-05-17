use crate::callback_registry::CallbackRegistry;
use crate::callback_types::*;
use crate::value_bridge::ScriptValue;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct ScriptManager {
    pub callbacks: Arc<Mutex<CallbackRegistry>>,
}

impl ScriptManager {
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(Mutex::new(CallbackRegistry::new())),
        }
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

    pub fn trigger_sync(
        &self,
        name: &str,
        arg: ScriptValue,
    ) -> Result<ScriptValue, crate::error::ScriptError> {
        self.callbacks.lock().trigger_sync(name, arg)
    }

    pub fn unregister_callback(&mut self, name: &str) {
        self.callbacks.lock().unregister(name);
    }

    pub fn shutdown(&mut self) {
        self.callbacks.lock().clear();
    }
}
