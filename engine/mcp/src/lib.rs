//! # Hezhou MCP — 大语言模型集成层
//!
//! 混合架构MCP Server，两种实现路径明确分离：
//!
//! ## 纯Rust路径 (rust-bridge feature)
//! - 可脱离编辑器独立运行
//! - 直接调用 core/ui crate 的Rust API
//! - 能力边界: 读取类操作 + 简单写入操作
//!   - 场景树查询、实体列表、截图、transform设置、项目保存
//!   - **无法**: 执行C#脚本、创建复杂UI、接收Thunk回调
//!
//! ## C# FFI路径 (ffi-bridge feature)
//! - 需要编辑器+Mono运行时环境
//! - 通过FfiContext函数指针调用C#层
//! - 能力边界: 复杂写入操作 + UI交互
//!   - 创建Entity(含资产模板)、创建UI widget、执行C#脚本
//!   - 接收Thunk回调(按钮点击、输入框变更等)
//!   - **依赖**: Mono SDK、scripting crate初始化完成
//!
//! ## 架构核心: EngineBridge trait
//! 所有MCP tool通过统一的EngineBridge trait访问引擎能力。
//! 根据运行模式选择具体实现：
//! - `RustBridge` — 纯Rust，独立运行
//! - `FfiBridge` — C# FFI，编辑器内嵌

pub mod bridge;
pub mod server;
pub mod transport;

#[cfg(feature = "rust-bridge")]
pub mod rust_bridge;

#[cfg(feature = "ffi-bridge")]
pub mod ffi_bridge;

// Re-export关键类型，方便外部crate使用
pub use bridge::EngineBridge;
pub use server::HezhouMcpServer;
pub use transport::{run_stdio, run_with_io, run_http};

#[cfg(feature = "rust-bridge")]
pub use rust_bridge::RustBridge;

#[cfg(feature = "ffi-bridge")]
pub use ffi_bridge::FfiBridge;