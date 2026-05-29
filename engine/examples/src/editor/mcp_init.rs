//! MCP server启动模块
//!
//! 检测HEZHOU_MCP环境变量，在编辑器主线程中创建bridge，
//! 在独立tokio线程中运行MCP server。
//!
//! Transport模式由HEZHOU_MCP_TRANSPORT决定:
//!   "stdio" → stdio transport (独立进程模式)
//!   "http" → Streamable HTTP transport (编辑器内嵌模式，默认)
//!   HEZHOU_MCP_PORT → HTTP端口(默认3000)
//!
//! Bridge模式由HEZHOU_MCP_MODE决定:
//!   "rust" → RustBridge (纯Rust路径)
//!   "ffi" → FfiBridge (C# FFI路径，默认)

use std::sync::Arc;
use std::ffi::c_void;

/// 启动MCP server
///
/// 在编辑器主线程调用，创建bridge并spawn tokio线程运行MCP server。
/// Scene和FfiContext指针通过super模块的static mut获取。
pub fn start_mcp_server() {
    let mcp_enabled = std::env::var("HEZHOU_MCP").unwrap_or_default();
    if mcp_enabled != "1" {
        hezhou_dfx::dfx_info!("MCP", "HEZHOU_MCP not set, skipping MCP server");
        return;
    }

    let mcp_mode = std::env::var("HEZHOU_MCP_MODE").unwrap_or_else(|_| "ffi".to_string());
    let transport = std::env::var("HEZHOU_MCP_TRANSPORT").unwrap_or_else(|_| "http".to_string());
    let port: u16 = std::env::var("HEZHOU_MCP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    
    hezhou_dfx::dfx_info!("MCP", "Starting MCP server (mode={}, transport={}, port={})", mcp_mode, transport, port);

    // 获取Scene和FfiContext指针(已在editor主线程验证为Some)
    let scene_raw: *mut hezhou_core::Scene = unsafe { super::SCENE.unwrap() };
    let ffi_ctx_ptr: *const hezhou_scripting::FfiContext = unsafe { super::FFI_PTR.unwrap() };

    // 创建EngineBridge
    let bridge: Arc<dyn hezhou_mcp::EngineBridge> = match mcp_mode.as_str() {
        "rust" => {
            hezhou_dfx::dfx_info!("MCP", "Using RustBridge");
            Arc::new(unsafe {
                hezhou_mcp::RustBridge::from_runtime(
                    scene_raw,
                    std::ptr::null_mut(), // AssetLibrary暂不可用
                    std::ptr::null_mut(), // Project暂不可用
                    true, // renderer_available (编辑器内嵌)
                )
            })
        }
        _ => {
            hezhou_dfx::dfx_info!("MCP", "Using FfiBridge");
            // FfiContext位拷贝创建Arc — 所有字段是fn指针(Copy)，无Drop风险
            // std::ptr::read做bitwise copy，原始Box<FfiContext>仍拥有原数据
            let ffi_ctx_copy: hezhou_scripting::FfiContext = unsafe { std::ptr::read(ffi_ctx_ptr) };
            let ffi_ctx_arc = Arc::new(ffi_ctx_copy);
            Arc::new(unsafe {
                hezhou_mcp::FfiBridge::from_ffi_context(
                    ffi_ctx_arc,
                    scene_raw as *mut c_void,
                )
            })
        }
    };

    // 根据transport模式选择启动方式
    match transport.as_str() {
        "http" => {
            // Streamable HTTP模式: 编辑器内嵌，OpenCode等远程连接
            let bridge_clone = bridge.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for MCP server");
                rt.block_on(async {
                    hezhou_dfx::dfx_info!("MCP", "MCP HTTP server tokio runtime started");
                    if let Err(e) = hezhou_mcp::run_http(bridge_clone, port).await {
                        hezhou_dfx::dfx_error!("MCP", "MCP HTTP server error: {}", e);
                    }
                    hezhou_dfx::dfx_info!("MCP", "MCP HTTP server stopped");
                });
            });
            hezhou_dfx::dfx_info!("MCP", "MCP HTTP server thread spawned (port={})", port);
        }
        _ => {
            // stdio模式: 独立进程，Host通过stdin/stdout通信
            let server = hezhou_mcp::HezhouMcpServer::new(bridge);
            std::thread::spawn(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for MCP server");
                rt.block_on(async {
                    hezhou_dfx::dfx_info!("MCP", "MCP stdio server tokio runtime started");
                    if let Err(e) = hezhou_mcp::run_stdio(server).await {
                        hezhou_dfx::dfx_error!("MCP", "MCP stdio server error: {}", e);
                    }
                    hezhou_dfx::dfx_info!("MCP", "MCP stdio server stopped");
                });
            });
            hezhou_dfx::dfx_info!("MCP", "MCP stdio server thread spawned");
        }
    }
}