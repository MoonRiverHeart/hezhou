use crate::log_types::*;
use chrono::Local;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::ffi::CString;
use std::io::Write;
use std::sync::Arc;

pub struct Logger {
    level: LogLevel,
    buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    buffer_size: usize,
    callbacks: Arc<Mutex<Vec<LogCallback>>>,
    output_file: Option<std::fs::File>,
    output_console: bool,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            level: LogLevel::Info,
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            buffer_size: 1000,
            callbacks: Arc::new(Mutex::new(Vec::new())),
            output_file: None,
            output_console: true,
        }
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    pub fn get_level(&self) -> LogLevel {
        self.level
    }

    pub fn set_buffer_size(&mut self, size: usize) {
        self.buffer_size = size;
    }

    pub fn enable_file_output(&mut self, path: &str) -> Result<(), String> {
        let file =
            std::fs::File::create(path).map_err(|e| format!("Failed to create log file: {}", e))?;
        self.output_file = Some(file);
        Ok(())
    }

    pub fn disable_file_output(&mut self) {
        self.output_file = None;
    }

    pub fn enable_console_output(&mut self) {
        self.output_console = true;
    }

    pub fn disable_console_output(&mut self) {
        self.output_console = false;
    }

    pub fn register_callback(&mut self, callback: LogCallback) {
        self.callbacks.lock().push(callback);
    }

    pub fn log(&mut self, level: LogLevel, module: &str, message: &str, file: &str, line: u32) {
        if (level as u8) < self.level as u8 {
            return;
        }

        let timestamp = Local::now().timestamp_millis() as u64;
        let thread_id = Self::get_thread_id();

        let module_c = CString::new(module).unwrap();
        let message_c = CString::new(message).unwrap();
        let file_c = CString::new(file).unwrap();

        let entry = LogEntry {
            level,
            timestamp,
            thread_id,
            module: module_c.as_ptr(),
            message: message_c.as_ptr(),
            file: file_c.as_ptr(),
            line,
        };

        if self.output_console {
            self.write_console(&entry);
        }

        if let Some(ref file) = self.output_file {
            self.write_file(file, &entry);
        }

        {
            let mut buffer = self.buffer.lock();
            if buffer.len() >= self.buffer_size {
                buffer.pop_front();
            }
            buffer.push_back(LogEntry {
                level,
                timestamp,
                thread_id,
                module: module_c.as_ptr(),
                message: message_c.as_ptr(),
                file: file_c.as_ptr(),
                line,
            });
        }

        for callback in self.callbacks.lock().iter() {
            callback(&entry);
        }
    }

    fn write_console(&self, entry: &LogEntry) {
        let time = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let module = unsafe {
            std::ffi::CStr::from_ptr(entry.module)
                .to_str()
                .unwrap_or("")
        };
        let message = unsafe {
            std::ffi::CStr::from_ptr(entry.message)
                .to_str()
                .unwrap_or("")
        };

        println!(
            "[{}][{}][{}] {}",
            time,
            entry.level.as_str(),
            module,
            message
        );
    }

    fn write_file(&self, file: &std::fs::File, entry: &LogEntry) {
        let time = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let module = unsafe {
            std::ffi::CStr::from_ptr(entry.module)
                .to_str()
                .unwrap_or("")
        };
        let message = unsafe {
            std::ffi::CStr::from_ptr(entry.message)
                .to_str()
                .unwrap_or("")
        };
        let file_name = unsafe { std::ffi::CStr::from_ptr(entry.file).to_str().unwrap_or("") };

        let line = format!(
            "[{}][{}][T:{}][{}][{}:{}] {}\n",
            time,
            entry.level.as_str(),
            entry.thread_id,
            module,
            file_name,
            entry.line,
            message
        );

        let mut file = file.try_clone().unwrap();
        file.write_all(line.as_bytes()).unwrap();
    }

    fn get_thread_id() -> u64 {
        #[cfg(windows)]
        {
            unsafe { winapi::um::processthreadsapi::GetCurrentThreadId() as u64 }
        }

        #[cfg(unix)]
        {
            unsafe { libc::pthread_self() as u64 }
        }

        #[cfg(not(any(windows, unix)))]
        {
            0
        }
    }

    pub fn get_buffer(&self) -> Vec<LogEntry> {
        self.buffer.lock().iter().cloned().collect()
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.lock().clear();
    }

    pub fn trace(&mut self, module: &str, message: &str, file: &str, line: u32) {
        self.log(LogLevel::Trace, module, message, file, line);
    }

    pub fn debug(&mut self, module: &str, message: &str, file: &str, line: u32) {
        self.log(LogLevel::Debug, module, message, file, line);
    }

    pub fn info(&mut self, module: &str, message: &str, file: &str, line: u32) {
        self.log(LogLevel::Info, module, message, file, line);
    }

    pub fn warn(&mut self, module: &str, message: &str, file: &str, line: u32) {
        self.log(LogLevel::Warn, module, message, file, line);
    }

    pub fn error(&mut self, module: &str, message: &str, file: &str, line: u32) {
        self.log(LogLevel::Error, module, message, file, line);
    }

    pub fn fatal(&mut self, module: &str, message: &str, file: &str, line: u32) {
        self.log(LogLevel::Fatal, module, message, file, line);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LogManager {
    loggers: parking_lot::Mutex<std::collections::HashMap<String, Box<Logger>>>,
    default_logger: Mutex<Box<Logger>>,
}

impl LogManager {
    pub fn new() -> Self {
        Self {
            loggers: parking_lot::Mutex::new(std::collections::HashMap::new()),
            default_logger: Mutex::new(Box::new(Logger::new())),
        }
    }

    pub fn get_logger(&self, name: &str) -> Option<std::sync::Arc<Mutex<Box<Logger>>>> {
        let loggers = self.loggers.lock();
        if loggers.contains_key(name) {
            Some(std::sync::Arc::new(Mutex::new(Box::new(Logger::new()))))
        } else {
            None
        }
    }

    pub fn create_logger(&mut self, name: &str) {
        let logger = Logger::new();
        self.loggers
            .lock()
            .insert(name.to_string(), Box::new(logger));
    }

    pub fn get_default(&self) -> parking_lot::MutexGuard<Box<Logger>> {
        self.default_logger.lock()
    }
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new()
    }
}
