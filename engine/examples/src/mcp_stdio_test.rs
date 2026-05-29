//! 最小MCP stdio transport测试
//! 直接通过stdin/stdout验证server的tools/list是否正常工作

use std::sync::Arc;
use hezhou_mcp::{HezhouMcpServer, RustBridge, run_stdio};

fn main() {
    let bridge: Arc<dyn hezhou_mcp::EngineBridge> = Arc::new(
        unsafe { RustBridge::from_runtime(
            std::ptr::null_mut(), // scene
            std::ptr::null_mut(), // asset_library
            std::ptr::null_mut(), // project
            false, // renderer_available
        )}
    );
    let server = HezhouMcpServer::new(bridge);
    
    // 通过stdio运行MCP server
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        if let Err(e) = run_stdio(server).await {
            eprintln!("MCP stdio error: {}", e);
        }
    });
}