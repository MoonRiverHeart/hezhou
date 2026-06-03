use std::sync::Mutex;
use once_cell::sync::OnceCell;
use super::entry::LogEntry;
use super::types::LogLevel;
use super::async_logger::AsyncLogger;
use super::file_writer::FileWriter;
use crate::internal_log;

static GLOBAL_LOGGER: OnceCell<Mutex<Logger>> = OnceCell::new();

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogFormat {
    Plain,
    Colored,
    Json,
}

#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub min_level: LogLevel,
    pub format: LogFormat,
    pub async_mode: bool,
    pub log_file_path: Option<String>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            format: LogFormat::Colored,
            async_mode: false,
            log_file_path: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logger {
    min_level: LogLevel,
    format: LogFormat,
    current_module: String,
    async_logger: Option<AsyncLogger>,
    log_file_path: Option<String>,
}

impl Logger {
    pub fn new(config: LoggerConfig) -> Self {
        let async_logger = if config.async_mode {
            Some(AsyncLogger::new(config.log_file_path.clone()))
        } else {
            None
        };
        
        Self {
            min_level: config.min_level,
            format: config.format,
            current_module: String::new(),
            async_logger,
            log_file_path: config.log_file_path,
        }
    }
    
    pub fn with_module(mut self, module: &str) -> Self {
        self.current_module = module.to_string();
        self
    }
    
    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.min_level {
            let entry = LogEntry::new(
                level,
                message.to_string(),
                self.current_module.clone(),
            );
            
            if let Some(async_logger) = &self.async_logger {
                // 异步模式
                async_logger.log(entry);
            } else {
                // 同步模式
                self.log_sync(entry);
            }
        }
    }
    
    fn log_sync(&self, entry: LogEntry) {
        let output = match self.format {
            LogFormat::Plain => entry.format_plain(),
            LogFormat::Colored => entry.format_colored(),
            LogFormat::Json => entry.format_json(),
        };
        // 输出到控制台
        // println!("{}", output);

        // 创建 FileWriter（如果需要）
        let file_writer = self.log_file_path.as_ref().and_then(|path| {
            match FileWriter::new(path) {
                Ok(writer) => {
                    // internal_log!(LogLevel::Info, "[SyncLogger] 文件写入器创建成功: {}", path);
                    Some(writer)
                }
                Err(e) => {
                    internal_log!(LogLevel::Error, "[SyncLogger] 文件写入器创建失败: {}, 错误: {}", path, e);
                    None
                }
            }
        });
        
        // 输出到文件（纯文本）
        if let Some(ref writer) = file_writer {
            let output = match self.format {
                LogFormat::Plain => entry.format_plain(),
                LogFormat::Colored => entry.format_plain(),
                LogFormat::Json => entry.format_json(),
            };
            if let Err(e) = writer.write(&output) {
                internal_log!(LogLevel::Error, "[SyncLogger] 写入文件失败: {}", e);
            }
        }
    }
    
    pub fn error(&self, msg: &str) { self.log(LogLevel::Error, msg); }
    pub fn warn(&self, msg: &str)  { self.log(LogLevel::Warn, msg); }
    pub fn info(&self, msg: &str)  { self.log(LogLevel::Info, msg); }
    pub fn debug(&self, msg: &str) { self.log(LogLevel::Debug, msg); }
    pub fn trace(&self, msg: &str) { self.log(LogLevel::Trace, msg); }
    
    pub fn flush(&self) {
        if let Some(async_logger) = &self.async_logger {
            async_logger.flush();
        }
    }
}

pub fn init_global_logger(config: LoggerConfig) {
    let logger = Logger::new(config);
    GLOBAL_LOGGER.set(Mutex::new(logger)).unwrap();
}

pub fn get_global_logger() -> &'static Mutex<Logger> {
    GLOBAL_LOGGER.get().expect("Logger not initialized")
}