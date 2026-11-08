//! # hezhou-package — 0.4 最小本地包管理器
//!
//! 只做「本地目录 registry + 依赖解析 + 安装到工程 + lockfile」：
//!
//! | 能力 | 入口 |
//! |---|---|
//! | 包清单校验 | [`manifest::PackageManifest`]、[`manifest::PackageId`] |
//! | 版本/版本范围 | [`manifest::Version`]、[`manifest::VersionReq`]（精确/`~`/`^`/`*`） |
//! | 本地 registry 扫描 | [`registry::PackageRegistry::scan`] |
//! | 依赖解析（拓扑/循环/缺失） | [`resolver::resolve_closure`] |
//! | 安装/卸载 + lockfile | [`installer::PackageInstaller`]、[`lockfile::Lockfile`] |
//!
//! ## 目录布局
//!
//! ```text
//! <registry>/<pkg>/package.json          # 本地包注册表（只读）
//! <project>/packages/<pkg>/...           # 安装产物
//! <project>/packages/lock.json           # 安装记录（name/version/hash/dependencies）
//! ```
//!
//! 限制：仅本地目录、无联网/签名/版本迁移；版本范围语义为最小子集。

#![deny(missing_docs)]

pub mod installer;
pub mod lockfile;
pub mod manifest;
pub mod registry;
pub mod resolver;

pub use installer::{default_registry_dir, InstallOutcome, PackageInstaller};
pub use lockfile::{LockedPackage, Lockfile, LOCK_FILE_NAME};
pub use manifest::{PackageId, PackageManifest, Version, VersionReq};
pub use registry::{PackageEntry, PackageRegistry};
pub use resolver::{resolve_closure, ResolvedPackage, ResolveError};

