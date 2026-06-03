use super::types::LogLevel;
use super::entry::LogEntry;
use super::file_writer::FileWriter;
use crossbeam_channel::{unbounded, Sender, Receiver};
use std::thread;
use std::sync::Arc;
use crate::internal_log;

pub enum LogMessage {
    Entry(LogEntry),
    Flush,
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct AsyncLogger {
    sender: Sender<LogMessage>,
}

impl AsyncLogger {
    pub fn new(log_file_path: Option<String>) -> Self {
        let (sender, receiver) = unbounded();
        
        // 创建 FileWriter（如果需要）
        let file_writer = log_file_path.and_then(|path| {
            match FileWriter::new(&path) {
                Ok(writer) => {
                    // internal_log!(LogLevel::Info, "文件写入器创建成功: {}", path);
                    Some(writer)
                }
                Err(e) => {
                    internal_log!(LogLevel::Error, "文件写入器创建失败: {}, 错误: {}", path, e);
                    None
                }
            }
        });
        
        // 启动工作线程
        Self::start_worker(receiver, file_writer);
        
        Self { sender }
    }
    
    fn start_worker(receiver: Receiver<LogMessage>, file_writer: Option<FileWriter>) {
        thread::Builder::new()
            .name("async-logger".to_string())
            .spawn(move || {
                internal_log!(LogLevel::Info, "工作线程启动");
                
                for msg in receiver {
                    match msg {
                        LogMessage::Entry(entry) => {
                            // 输出到控制台（带颜色）
                            let colored = entry.format_colored();
                            println!("{}", colored);
                            
                            // 输出到文件（纯文本）
                            if let Some(ref writer) = file_writer {
                                let plain = entry.format_plain();
                                if let Err(e) = writer.write(&plain) {
                                    internal_log!(LogLevel::Error, "写入文件失败: {}", e);
                                }
                            }
                        }
                        LogMessage::Flush => {
                            internal_log!(LogLevel::Debug, "刷新日志缓冲区");
                            if let Some(ref writer) = file_writer {
                                if let Err(e) = writer.flush() {
                                    internal_log!(LogLevel::Error, "刷新文件失败: {}", e);
                                }
                            }
                        }
                        LogMessage::Shutdown => {
                            internal_log!(LogLevel::Info, "关闭工作线程");
                            if let Some(ref writer) = file_writer {
                                let _ = writer.flush();
                            }
                            break;
                        }
                    }
                }
                
                internal_log!(LogLevel::Info, "工作线程退出");
            })
            .unwrap();
    }
    
    pub fn log(&self, entry: LogEntry) {
        let _ = self.sender.send(LogMessage::Entry(entry));
    }
    
    pub fn flush(&self) {
        let _ = self.sender.send(LogMessage::Flush);
    }
    
    pub fn shutdown(&self) {
        let _ = self.sender.send(LogMessage::Shutdown);
    }
}