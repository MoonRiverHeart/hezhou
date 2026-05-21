use hezhou_dfx::*;
use std::sync::Arc;
use parking_lot::Mutex;

pub fn setup_dfx() -> (Arc<Mutex<DfxSystem>>, String) {
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Info);
    dfx.lock().get_trace_analyzer().lock().enable();
    
    let log_path = format!("logs/hezhou_{}.log", chrono::Local::now().format("%Y-%m-%d"));
    std::fs::create_dir_all("logs").ok();
    if let Err(e) = dfx.lock().get_logger().lock().enable_file_output(&log_path) {
        dfx_error!("Demo", "Failed to enable file output: {}", e);
    }
    
    (dfx, log_path)
}