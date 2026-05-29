//! MCP Server transport启动
//!
//! 三种transport模式:
//! 1. stdio — Host(Claude Desktop等) spawn MCP server进程
//! 2. 自定义io — 编辑器内嵌，自定义读写通道
//! 3. Streamable HTTP — 编辑器内嵌，HTTP/SSE endpoint，OpenCode等远程连接
//!
//! Streamable HTTP是MCP 2025-11-25规范定义的新transport，
//! rmcp v1.7.0内置支持，通过axum HTTP server提供服务。

use crate::server::HezhouMcpServer;
use rmcp::ServiceExt;
use std::sync::Arc;
use crate::bridge::EngineBridge;

/// 启动MCP server (stdio transport)
///
/// 用法:
/// ```rust
/// let bridge: Arc<dyn EngineBridge> = Arc::new(RustBridge::new_empty());
/// let server = HezhouMcpServer::new(bridge);
/// tokio::spawn(run_stdio(server));
/// ```
pub async fn run_stdio(server: HezhouMcpServer) -> anyhow::Result<()> {
    let transport = rmcp::transport::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

/// 启动MCP server (自定义io transport — 用于编辑器内嵌)
///
/// 编辑器内嵌时不用真正的stdin/stdout，而是用自定义的
/// 读写通道(tokio mpsc或跨线程channel)。
pub async fn run_with_io(
    server: HezhouMcpServer,
    reader: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    writer: impl tokio::io::AsyncWrite + Unpin + Send + 'static,
) -> anyhow::Result<()> {
    let transport = (reader, writer);
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}

/// 启动MCP server (Streamable HTTP transport — 编辑器内嵌模式)
///
/// 在指定端口启动axum HTTP server，提供MCP Streamable HTTP endpoint。
/// MCP Host(OpenCode/Claude Desktop等)通过HTTP/SSE连接。
///
/// Streamable HTTP规范: POST /mcp → JSON-RPC请求
///                       GET /mcp → SSE事件流(session续连)
///                       DELETE /mcp → 关闭session
///
/// 用法:
/// ```rust
/// let bridge: Arc<dyn EngineBridge> = Arc::new(RustBridge::new_empty());
/// tokio::spawn(run_http(bridge, 3000));
/// ```
pub async fn run_http(bridge: Arc<dyn EngineBridge>, port: u16) -> anyhow::Result<()> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig,
        StreamableHttpService,
        session::local::LocalSessionManager,
    };

    // service_factory: 每个新session创建一个HezhouMcpServer实例
    // Arc<dyn EngineBridge>是共享状态，所有session共用同一个bridge
    let bridge_for_factory = bridge.clone();
    let service_factory = move || Ok(HezhouMcpServer::new(bridge_for_factory.clone()));

    let session_manager = Arc::new(LocalSessionManager::default());

    let config = StreamableHttpServerConfig::default()
        .with_sse_keep_alive(Some(std::time::Duration::from_secs(15)))
        .with_stateful_mode(true);

    let mcp_service = StreamableHttpService::new(
        service_factory,
        session_manager,
        config,
    );

    // axum路由: /mcp → StreamableHttpService作为tower Service嵌入
    // nest_service是rmcp官方推荐方式(rmcp v1.7.0 tests中使用)
    // StreamableHttpService实现了tower Service<Request<RequestBody>> → Response<BoxResponse>
    // nest_service让axum正确处理SSE流式响应，而不是手动handle()导致流截断
    let app = axum::Router::new().nest_service("/mcp", mcp_service);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    hezhou_dfx::dfx_info!("MCP", "Streamable HTTP server listening on http://{}/mcp", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}