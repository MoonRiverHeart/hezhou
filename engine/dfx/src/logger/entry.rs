// engine/dfx/src/logger/log_entry.rs
use super::types::LogLevel;
use chrono::{Local, DateTime};

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub module: String,
    pub timestamp: DateTime<Local>,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: String, module: String) -> Self {
        Self {
            level,
            message,
            module,
            timestamp: Local::now(),
        }
    }
    
    // 格式化日志（带颜色）
    pub fn format_colored(&self) -> String {
        format!(
            "{}{} [{}] [{:5}] [{}] {}\x1b[0m",
            self.level.color_code(),
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.module,
            self.level.name(),
            std::thread::current().name().unwrap_or("main"),
            self.message
        )
    }
    
    // 格式化日志（纯文本）
    pub fn format_plain(&self) -> String {
        format!(
            "{} [{}] [{:5}] [{}] {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.module,
            self.level.name(),
            std::thread::current().name().unwrap_or("main"),
            self.message
        )
    }
    
    // JSON 格式
    pub fn format_json(&self) -> String {
        format!(
            r#"{{"timestamp":"{}","level":"{}","module":"{}","thread":"{}","message":"{}"}}"#,
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.level.name(),
            self.module,
            std::thread::current().name().unwrap_or("main"),
            serde_json::json!(self.message)
        )
    }
}