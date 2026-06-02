// engine/dfx/src/logger/mod.rs
use std::sync::Mutex;
use once_cell::sync::OnceCell;
use super::entry::LogEntry;
use super::types::LogLevel;

static GLOBAL_LOGGER: OnceCell<Mutex<Logger>> = OnceCell::new();

#[derive(Debug)]
pub struct Logger {
    min_level: LogLevel,
    use_colors: bool,
    format: LogFormat,
    current_module: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogFormat {
    Plain,      // 纯文本
    Colored,    // 带颜色
    Json,       // JSON 格式
}

impl Logger {
    pub fn new(min_level: LogLevel) -> Self {
        Self {
            min_level,
            use_colors: true,
            format: LogFormat::Colored,
            current_module: String::new(),
        }
    }
    
    pub fn with_format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self.use_colors = format == LogFormat::Colored;
        self
    }
    
    pub fn with_module(mut self, module: &str) -> Self {
        self.current_module = module.to_string();
        self
    }
    
    pub fn set_module(&mut self, module: &str) {
        self.current_module = module.to_string();
    }
    
    fn log(&self, level: LogLevel, message: &str) {
        // 关键修复：改成 <= 因为 Error(1) 比 Debug(4) 更严重
        if level <= self.min_level {
            let entry = LogEntry::new(
                level,
                message.to_string(),
                self.current_module.clone(),
            );
            
            let output = match self.format {
                LogFormat::Plain => entry.format_plain(),
                LogFormat::Colored => entry.format_colored(),
                LogFormat::Json => entry.format_json(),
            };
            
            println!("{}", output);
        }
    }
    
    // 便捷方法
    pub fn error(&self, msg: &str) { self.log(LogLevel::Error, msg); }
    pub fn warn(&self, msg: &str)  { self.log(LogLevel::Warn, msg); }
    pub fn info(&self, msg: &str)  { self.log(LogLevel::Info, msg); }
    pub fn debug(&self, msg: &str) { self.log(LogLevel::Debug, msg); }
    pub fn trace(&self, msg: &str) { self.log(LogLevel::Trace, msg); }
    
    // 带模块的日志
    pub fn log_with_module(&self, level: LogLevel, module: &str, message: &str) {
        // 关键修复：改成 <=
        if level <= self.min_level {
            let entry = LogEntry::new(
                level,
                message.to_string(),
                module.to_string(),
            );
            
            let output = match self.format {
                LogFormat::Plain => entry.format_plain(),
                LogFormat::Colored => entry.format_colored(),
                LogFormat::Json => entry.format_json(),
            };
            
            println!("{}", output);
        }
    }
    
    // 宏辅助方法
    pub fn log_at(&self, level: LogLevel, module: &str, message: &str) {
        self.log_with_module(level, module, message);
    }
}

// 全局日志实例
pub fn init_global_logger(level: LogLevel) {
    let logger = Logger::new(level);
    GLOBAL_LOGGER.set(Mutex::new(logger)).unwrap();
}

pub fn get_global_logger() -> &'static Mutex<Logger> {
    GLOBAL_LOGGER.get().expect("Logger not initialized")
}