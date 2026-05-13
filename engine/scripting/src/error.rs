use std::fmt;

#[derive(Debug)]
pub enum ScriptError {
    AssemblyNotFound(String),
    ClassNotFound(String),
    MethodNotFound(String),
    CallbackNotFound(String),
    InvokeFailed(String),
    TypeConversionFailed(String),
    RuntimeError(String),
    TaskNotFound(u64),
    TaskFailed(String),
    AsyncTimeout,
    InvalidArgument,
    NotInitialized,
    LoadFailed(String),
    SymbolNotFound(String),
    UnsupportedOperation(String),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AssemblyNotFound(name) => write!(f, "Assembly not found: {}", name),
            Self::ClassNotFound(name) => write!(f, "Class not found: {}", name),
            Self::MethodNotFound(name) => write!(f, "Method not found: {}", name),
            Self::CallbackNotFound(name) => write!(f, "Callback not found: {}", name),
            Self::InvokeFailed(msg) => write!(f, "Invoke failed: {}", msg),
            Self::TypeConversionFailed(msg) => write!(f, "Type conversion failed: {}", msg),
            Self::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            Self::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            Self::TaskFailed(msg) => write!(f, "Task failed: {}", msg),
            Self::AsyncTimeout => write!(f, "Async operation timeout"),
            Self::InvalidArgument => write!(f, "Invalid argument"),
            Self::NotInitialized => write!(f, "ScriptManager not initialized"),
            Self::LoadFailed(msg) => write!(f, "Load failed: {}", msg),
            Self::SymbolNotFound(msg) => write!(f, "Symbol not found: {}", msg),
            Self::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
        }
    }
}

impl std::error::Error for ScriptError {}

pub type ScriptResult<T> = Result<T, ScriptError>;