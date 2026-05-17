#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            5 => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LogEntry {
    pub level: LogLevel,
    pub timestamp: u64,
    pub thread_id: u64,
    pub module: *const std::os::raw::c_char,
    pub message: *const std::os::raw::c_char,
    pub file: *const std::os::raw::c_char,
    pub line: u32,
}

pub type LogCallback = extern "C" fn(&LogEntry);
