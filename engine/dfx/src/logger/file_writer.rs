use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use chrono::Local;
use std::sync::Mutex;
use crate::internal_log;
use crate::logger::LogLevel;

pub struct FileWriter {
    writer: Arc<Mutex<BufWriter<File>>>,
    path_prefix: PathBuf,
    // 使用 RwLock，读多写少时性能更好
    current_date: Arc<RwLock<String>>,
}

impl FileWriter {
    pub fn new(path_prefix: &str) -> std::io::Result<Self> {
        let path_prefix = PathBuf::from(path_prefix);
        // internal_log!(LogLevel::Info, "初始化，路径前缀: {:?}", path_prefix);
        
        if let Some(parent) = path_prefix.parent() {
            if !parent.exists() {
                // internal_log!(LogLevel::Info, "创建目录: {:?}", parent);
                std::fs::create_dir_all(parent)?;
            }
        }
        
        let current_date = Local::now().format("%Y-%m-%d").to_string();
        let file_path = Self::get_file_path(&path_prefix, &current_date);
        
        // internal_log!(LogLevel::Info, "当前日期: {}, 文件: {:?}", current_date, file_path);
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        
        Ok(Self {
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
            path_prefix,
            current_date: Arc::new(RwLock::new(current_date)),
        })
    }
    
    fn get_file_path(prefix: &Path, date: &str) -> PathBuf {
        let file_name = prefix
            .file_name()
            .map(|name| format!("{}_{}.log", name.to_string_lossy(), date))
            .unwrap_or_else(|| format!("log_{}.log", date));
        
        prefix.with_file_name(file_name)
    }
    
    fn rotate_if_needed(&self) -> std::io::Result<()> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        
        // 读取当前日期
        let current_date_guard = self.current_date.read().unwrap();
        
        if today != *current_date_guard {
            // 需要写入，升级为写锁
            drop(current_date_guard);  // 释放读锁
            let mut current_date_guard = self.current_date.write().unwrap();
            
            // 双重检查（防止其他线程已经更新）
            if today != *current_date_guard {
                internal_log!(LogLevel::Info, "日期变更，轮转日志: {} -> {}", *current_date_guard, today);
                
                // 创建新文件
                let new_path = Self::get_file_path(&self.path_prefix, &today);
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&new_path)?;
                
                // 替换 writer
                let mut writer = self.writer.lock().unwrap();
                *writer = BufWriter::new(file);
                
                // 更新当前日期
                *current_date_guard = today;
            }
        }
        
        Ok(())
    }
    
    pub fn write(&self, content: &str) -> std::io::Result<()> {
        self.rotate_if_needed()?;
        
        let mut writer = self.writer.lock().unwrap();
        writeln!(writer, "{}", content)?;
        writer.flush()?;
        Ok(())
    }
    
    pub fn flush(&self) -> std::io::Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.flush()?;
        Ok(())
    }
}

impl Clone for FileWriter {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
            path_prefix: self.path_prefix.clone(),
            current_date: Arc::clone(&self.current_date),
        }
    }
}