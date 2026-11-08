//! # hezhou-plugin — 0.4 编辑器声明式插件 API
//!
//! 只做「声明式」插件：清单 + registry + 启用状态 + 内置 action 白名单，不加载动态 Rust 代码。
//!
//! | 能力 | 入口 |
//! |---|---|
//! | 插件清单校验 | [`manifest::PluginManifest`] |
//! | 扫描/重复/未知 action | [`registry::PluginRegistry::scan`] |
//! | 启用/禁用持久化 | [`registry::PluginRegistry::set_enabled`] |
//! | 启用插件命令列表 | [`registry::PluginRegistry::enabled_commands`] |
//!
//! ## 目录布局
//!
//! ```text
//! <project>/plugins/<plugin-id>/plugin.json   # 本地插件注册表（只读）
//! <project>/plugins/enabled.json              # 启用状态（本工具写入）
//! ```
//!
//! 命令只能引用 [`manifest::ACTION_WHITELIST`] 中的内置动作，不执行任意代码。

#![deny(missing_docs)]

pub mod manifest;
pub mod registry;

pub use manifest::{
    is_valid_plugin_id, PluginCommand, PluginManifest, ACTION_WHITELIST, API_VERSION,
};
pub use registry::{EnabledState, PluginEntry, PluginRegistry, ENABLED_FILE_NAME};
