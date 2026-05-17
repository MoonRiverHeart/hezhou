pub type SyncCallback = extern "C" fn(ScriptValue, usize) -> ScriptValue;
pub type AsyncCallback = extern "C" fn(ScriptValue, usize, usize);
pub type TaskCallback = extern "C" fn(ScriptValue, usize, usize, usize);

use crate::value_bridge::ScriptValue;

#[derive(Debug, Clone)]
pub enum CallbackType {
    Sync,
    Async,
    Task,
}

#[derive(Debug, Clone)]
pub struct CallbackDescriptor {
    pub name: String,
    pub callback_type: CallbackType,
    pub description: String,
    pub signature: String,
    pub is_closure: bool,
    pub priority: u32,
}

impl CallbackDescriptor {
    pub fn new_sync(name: &str, description: &str, signature: &str) -> Self {
        Self {
            name: name.to_string(),
            callback_type: CallbackType::Sync,
            description: description.to_string(),
            signature: signature.to_string(),
            is_closure: false,
            priority: 0,
        }
    }

    pub fn new_async(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            callback_type: CallbackType::Async,
            description: description.to_string(),
            signature: String::new(),
            is_closure: false,
            priority: 0,
        }
    }

    pub fn new_task(name: &str, description: &str, _supports_progress: bool) -> Self {
        Self {
            name: name.to_string(),
            callback_type: CallbackType::Task,
            description: description.to_string(),
            signature: String::new(),
            is_closure: false,
            priority: 0,
        }
    }

    pub fn with_closure(mut self) -> Self {
        self.is_closure = true;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}
