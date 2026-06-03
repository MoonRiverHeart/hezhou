use dfx::logger::{Logger, LogLevel, LogFormat, LoggerConfig};
use std::time::Duration;
use std::thread;

fn main() {
    // 示例1：同步模式，只输出到控制台
    println!("=== 同步模式（控制台）===");
    let config = LoggerConfig {
        min_level: LogLevel::Debug,
        format: LogFormat::Colored,
        async_mode: false,
        log_file_path: None,
    };
    
    let logger = Logger::new(config).with_module("sync_test");
    logger.info("同步日志信息");
    logger.debug("调试信息");
    logger.error("错误信息");
    
    // 示例2：异步模式，输出到文件
    // println!("\n=== 异步模式（文件）===");
    let config = LoggerConfig {
        min_level: LogLevel::Debug,
        format: LogFormat::Colored,
        async_mode: true,
        log_file_path: Some("logs/app.log".to_string()),
    };
    
    let logger = Logger::new(config).with_module("async_test");
    
    // 大量日志测试
    for i in 0..10 {
        logger.info(&format!("异步日志消息 {}", i));
        logger.error("错误信息");
    }
    
    logger.flush(); // 确保所有日志都写入
    // println!("日志已写入到 logs/app.log");
    
    // 示例3：混合模式
    println!("\n=== 异步模式（控制台+文件）===");
    let config = LoggerConfig {
        min_level: LogLevel::Info,
        format: LogFormat::Json,
        async_mode: false,
        log_file_path: Some("logs/json.log".to_string()),
    };
    
    let logger = Logger::new(config).with_module("json_test");
    logger.info("JSON 格式日志");
    logger.error("错误信息");
    
    logger.flush();
    // println!("JSON 日志已写入到 logs/json.log");

    thread::sleep(Duration::from_secs(1));
}