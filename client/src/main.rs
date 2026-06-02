use dfx::{logger::Logger, logger::LogLevel, logger::LogFormat};

fn main() {
    // 创建 Logger
    let logger = Logger::new(LogLevel::Debug)
        .with_format(LogFormat::Colored)
        .with_module("test");
    
    // 输出日志
    logger.info("程序启动");
    logger.debug(&format!("用户登录: {}", "admin"));
    logger.warn("配置文件缺失，使用默认配置");
    logger.error("数据库连接失败");
}