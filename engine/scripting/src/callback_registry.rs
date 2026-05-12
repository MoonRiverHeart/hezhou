use crate::callback_types::*;
use crate::value_bridge::ScriptValue;
use crate::error::ScriptError;
use std::collections::HashMap;

pub type TaskId = u64;

pub struct SyncCallbackEntry {
    pub callback: SyncCallback,
    pub descriptor: CallbackDescriptor,
    pub closure_context: Option<usize>,
}

pub struct AsyncCallbackEntry {
    pub callback: AsyncCallback,
    pub descriptor: CallbackDescriptor,
    pub closure_context: Option<usize>,
}

pub struct TaskCallbackEntry {
    pub callback: TaskCallback,
    pub descriptor: CallbackDescriptor,
    pub closure_context: Option<usize>,
    pub supports_progress: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

pub struct RunningTask {
    pub id: TaskId,
    pub progress: f32,
    pub status: TaskStatus,
    pub result: Option<ScriptValue>,
    pub error_message: Option<String>,
}

pub struct CallbackRegistry {
    sync_callbacks: HashMap<String, SyncCallbackEntry>,
    async_callbacks: HashMap<String, AsyncCallbackEntry>,
    task_callbacks: HashMap<String, TaskCallbackEntry>,
    running_tasks: HashMap<TaskId, RunningTask>,
    next_task_id: TaskId,
}

impl CallbackRegistry {
    pub fn new() -> Self {
        Self {
            sync_callbacks: HashMap::new(),
            async_callbacks: HashMap::new(),
            task_callbacks: HashMap::new(),
            running_tasks: HashMap::new(),
            next_task_id: 1,
        }
    }

    pub fn register_sync(&mut self, name: String, callback: SyncCallback, descriptor: CallbackDescriptor, context: usize) {
        let entry = SyncCallbackEntry {
            callback,
            descriptor,
            closure_context: Some(context),
        };
        self.sync_callbacks.insert(name, entry);
    }

    pub fn register_async(&mut self, name: String, callback: AsyncCallback, descriptor: CallbackDescriptor, context: usize) {
        let entry = AsyncCallbackEntry {
            callback,
            descriptor,
            closure_context: Some(context),
        };
        self.async_callbacks.insert(name, entry);
    }

    pub fn register_task(&mut self, name: String, callback: TaskCallback, descriptor: CallbackDescriptor, supports_progress: bool, context: usize) {
        let entry = TaskCallbackEntry {
            callback,
            descriptor,
            closure_context: Some(context),
            supports_progress,
        };
        self.task_callbacks.insert(name, entry);
    }

    pub fn trigger_sync(&self, name: &str, arg: ScriptValue) -> Result<ScriptValue, ScriptError> {
        let entry = self.sync_callbacks.get(name)
            .ok_or_else(|| ScriptError::CallbackNotFound(name.to_string()))?;
        
        let context = entry.closure_context.unwrap_or(0);
        Ok((entry.callback)(arg, context))
    }

    pub fn trigger_async(&self, name: &str, arg: ScriptValue, completion_ptr: usize) -> Result<(), ScriptError> {
        let entry = self.async_callbacks.get(name)
            .ok_or_else(|| ScriptError::CallbackNotFound(name.to_string()))?;
        
        let context = entry.closure_context.unwrap_or(0);
        (entry.callback)(arg, context, completion_ptr);
        Ok(())
    }

    pub fn trigger_task(&mut self, name: &str, arg: ScriptValue, progress_ptr: usize, completion_ptr: usize) -> Result<TaskId, ScriptError> {
        let entry = self.task_callbacks.get(name)
            .ok_or_else(|| ScriptError::CallbackNotFound(name.to_string()))?;
        
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        
        self.running_tasks.insert(task_id, RunningTask {
            id: task_id,
            progress: 0.0,
            status: TaskStatus::Running,
            result: None,
            error_message: None,
        });
        
        let context = entry.closure_context.unwrap_or(0);
        (entry.callback)(arg, context, progress_ptr, completion_ptr);
        Ok(task_id)
    }

    pub fn notify_completion(&mut self, _completion_ptr: usize, result: ScriptValue) {
        for (_, task) in self.running_tasks.iter_mut() {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Completed;
                task.progress = 1.0;
                task.result = Some(result);
                break;
            }
        }
    }

    pub fn notify_progress(&mut self, _progress_ptr: usize, progress: f32) {
        for (_, task) in self.running_tasks.iter_mut() {
            if task.status == TaskStatus::Running {
                task.progress = progress;
                break;
            }
        }
    }

    pub fn query_task_progress(&self, id: TaskId) -> Option<f32> {
        self.running_tasks.get(&id).map(|t| t.progress)
    }

    pub fn query_task_result(&self, id: TaskId) -> Option<Result<ScriptValue, String>> {
        self.running_tasks.get(&id).and_then(|t| {
            if t.status == TaskStatus::Completed {
                t.result.clone().map(Ok)
            } else if t.status == TaskStatus::Failed {
                t.error_message.clone().map(Err)
            } else {
                None
            }
        })
    }

    pub fn cancel_task(&mut self, id: TaskId) -> bool {
        if let Some(task) = self.running_tasks.get_mut(&id) {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Cancelled;
                return true;
            }
        }
        false
    }

    pub fn unregister(&mut self, name: &str) {
        self.sync_callbacks.remove(name);
        self.async_callbacks.remove(name);
        self.task_callbacks.remove(name);
    }

    pub fn list_callbacks(&self) -> Vec<CallbackDescriptor> {
        let mut descriptors = Vec::new();
        for entry in self.sync_callbacks.values() {
            descriptors.push(entry.descriptor.clone());
        }
        for entry in self.async_callbacks.values() {
            descriptors.push(entry.descriptor.clone());
        }
        for entry in self.task_callbacks.values() {
            descriptors.push(entry.descriptor.clone());
        }
        descriptors
    }

    pub fn clear(&mut self) {
        self.sync_callbacks.clear();
        self.async_callbacks.clear();
        self.task_callbacks.clear();
        self.running_tasks.clear();
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}