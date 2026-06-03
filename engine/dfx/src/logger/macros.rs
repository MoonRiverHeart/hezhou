// engine/dfx/src/logger/macros.rs
use chrono::Local;

// 基础宏
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::get_global_logger()
            .lock()
            .unwrap()
            .error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::get_global_logger()
            .lock()
            .unwrap()
            .warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::get_global_logger()
            .lock()
            .unwrap()
            .info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::get_global_logger()
            .lock()
            .unwrap()
            .debug(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::logger::get_global_logger()
            .lock()
            .unwrap()
            .trace(&format!($($arg)*))
    };
}

// 带模块的宏
#[macro_export]
macro_rules! log_error_mod {
    ($module:expr, $($arg:tt)*) => {
        $crate::logger::get_global_logger()
            .lock()
            .unwrap()
            .log_with_module($crate::logger::types::LogLevel::Error, $module, &format!($($arg)*))
    };
}

// 自动使用当前文件路径作为模块名
#[macro_export]
macro_rules! log_error_file {
    ($($arg:tt)*) => {
        $crate::logger::get_global_logger()
            .lock()
            .unwrap()
            .log_with_module(
                $crate::logger::types::LogLevel::Error,
                file!(),
                &format!($($arg)*)
            )
    };
}

// 内部日志宏，用于 AsyncLogger 自身的日志输出
#[macro_export]
macro_rules! internal_log {
    ($level:expr, $($arg:tt)*) => {
        let now = ::chrono::Local::now();
        let thread_name = std::thread::current()
            .name()
            .unwrap_or("main")
            .to_string();
        let message = format!($($arg)*);
        
        match $level {
            LogLevel::Error => {
                eprintln!("\x1b[31m{} [AsyncLogger] [ERROR] [{}] {}\x1b[0m", 
                    now.format("%Y-%m-%d %H:%M:%S%.3f"), thread_name, message);
            }
            LogLevel::Warn => {
                println!("\x1b[33m{} [AsyncLogger] [WARN] [{}] {}\x1b[0m", 
                    now.format("%Y-%m-%d %H:%M:%S%.3f"), thread_name, message);
            }
            LogLevel::Info => {
                println!("\x1b[32m{} [AsyncLogger] [INFO] [{}] {}\x1b[0m", 
                    now.format("%Y-%m-%d %H:%M:%S%.3f"), thread_name, message);
            }
            LogLevel::Debug => {
                println!("\x1b[36m{} [AsyncLogger] [DEBUG] [{}] {}\x1b[0m", 
                    now.format("%Y-%m-%d %H:%M:%S%.3f"), thread_name, message);
            }
            _ => {
                println!("{} [AsyncLogger] [{}] {}", 
                    now.format("%Y-%m-%d %H:%M:%S%.3f"), thread_name, message);
            }
        }
    };
}