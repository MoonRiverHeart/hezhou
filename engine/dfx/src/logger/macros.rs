// engine/dfx/src/logger/macros.rs

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