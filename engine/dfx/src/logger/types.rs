// engine/dfx/src/logger/types.rs
use std::fmt;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl LogLevel {
    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
    
    pub fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Error => "\x1b[31m",      // 红色
            LogLevel::Warn => "\x1b[33m",       // 黄色
            LogLevel::Info => "\x1b[32m",       // 绿色
            LogLevel::Debug => "\x1b[36m",      // 青色
            LogLevel::Trace => "\x1b[90m",      // 灰色
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}