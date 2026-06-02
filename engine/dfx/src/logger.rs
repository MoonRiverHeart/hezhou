mod logger;
mod types;
mod entry;
mod macros;

// 重新导出常用类型 - 让外部可以直接使用
pub use crate::logger::types::LogLevel;
pub use crate::logger::logger::{Logger, LogFormat};
pub use crate::logger::entry::LogEntry;

// 重新导出宏
pub use crate::logger::macros::*;