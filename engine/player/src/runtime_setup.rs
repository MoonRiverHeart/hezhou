//! 内置 Mono 运行时启动准备（task-38）：检测 Player 包内 `runtime/mono`，
//! 在加载 C# 前把 Mono 的可执行文件/库目录前置到进程环境变量。
//!
//! # 包布局（可选）
//!
//! ```text
//! <PlayerRoot>/runtime/
//! └── mono/
//!     ├── bin/   # mono.exe、libmono-2.0.dll 等（Windows PATH / Linux 可执行）
//!     ├── lib/   # Mono 库与托管程序集（MONO_PATH；Linux LD_LIBRARY_PATH）
//!     └── etc/   # mono 配置（可选，由 Mono 自身读取）
//! ```
//!
//! # 行为
//!
//! - 找到 `runtime/mono`：
//!   - Windows：`PATH` **前置** `runtime/mono/bin`（无 bin 时为 mono 根），
//!     `MONO_PATH` **追加** `runtime/mono/lib/mono/4.5`（存在 4.5-api 时一并追加）；
//!   - Linux：`LD_LIBRARY_PATH` **前置** `runtime/mono/lib`（无 lib 时为 mono 根），
//!     `MONO_PATH` **追加** `runtime/mono/lib/mono/4.5`（存在 4.5-api 时一并追加）；
//! - 未找到：**no-op**（报告 `found=false`，不改任何环境变量）；
//! - 幂等：目标路径已在变量首位/列表中时不重复追加；重复调用返回相同的 `modified` 状态；
//! - 只追加/前置，不删除用户已有的 PATH/LD_LIBRARY_PATH/MONO_PATH 条目。
//!
//! 调用点：`runtime::run`（`--validate`/启动流程，日志之后、加载 C# 之前）和
//! `WindowPlayerHost`（加载 C# DLL 之前，幂等兜底）。

use std::path::{Path, PathBuf};

use crate::platform::TargetPlatform;

/// 包内 Mono runtime 目录名。
pub const MONO_RUNTIME_DIR: &str = "mono";
/// Mono 可执行文件目录名。
pub const MONO_BIN_DIR: &str = "bin";
/// Mono 库/程序集目录名。
pub const MONO_LIB_DIR: &str = "lib";
/// 托管程序集搜索变量。
pub const MONO_PATH_ENV: &str = "MONO_PATH";

/// 内置 Mono 运行时探测/环境准备结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonoRuntimeSetup {
    /// 是否找到 `runtime/mono`。
    pub found: bool,
    /// `runtime/mono` 路径。
    pub root: Option<PathBuf>,
    /// 前置到 PATH/LD_LIBRARY_PATH 的目录。
    pub native_dir: Option<PathBuf>,
    /// 追加到 MONO_PATH 的目录（4.5/4.5-api，按存在顺序）。
    pub managed_dirs: Vec<PathBuf>,
    /// 第一个 MONO_PATH 目录（兼容/展示）。
    pub managed_dir: Option<PathBuf>,
    /// 实际修改的环境变量（排序，便于日志/测试）。
    pub modified: Vec<&'static str>,
    /// 目标平台。
    pub platform: TargetPlatform,
}

impl MonoRuntimeSetup {
    /// 未发现 runtime/mono 的 no-op 结果。
    pub fn not_found(platform: TargetPlatform) -> Self {
        Self {
            found: false,
            root: None,
            native_dir: None,
            managed_dirs: Vec::new(),
            managed_dir: None,
            modified: Vec::new(),
            platform,
        }
    }

    /// 单行摘要（启动日志）。
    pub fn summary(&self) -> String {
        if !self.found {
            return "内置 Mono 运行时：未发现 runtime/mono（no-op，使用系统 Mono/其它搜索路径）"
                .to_string();
        }
        let root = self
            .root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        let native = self
            .native_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        let managed = if self.managed_dirs.is_empty() {
            "<none>".to_string()
        } else {
            self.managed_dirs
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(";")
        };
        let modified = if self.modified.is_empty() {
            "<none>".to_string()
        } else {
            self.modified.join(",")
        };
        format!(
            "内置 Mono 运行时：已找到 {}（native={} managed={}；前置环境变量 {}）",
            root, native, managed, modified
        )
    }
}

/// 检测包内 `runtime/mono` 并准备 Mono 环境变量（当前宿主平台）。
pub fn setup_mono_runtime(player_root: &Path) -> MonoRuntimeSetup {
    setup_mono_runtime_for(player_root, TargetPlatform::host())
}

/// 检测包内 `runtime/mono` 并准备 Mono 环境变量（指定平台，测试/交叉验证用）。
pub fn setup_mono_runtime_for(
    player_root: &Path,
    platform: TargetPlatform,
) -> MonoRuntimeSetup {
    let mono_root = player_root
        .join(crate::manifest::RUNTIME_DIR)
        .join(MONO_RUNTIME_DIR);
    if !mono_root.is_dir() {
        return MonoRuntimeSetup::not_found(platform);
    }

    let bin = mono_root.join(MONO_BIN_DIR);
    let lib = mono_root.join(MONO_LIB_DIR);
    let native_dir = match platform {
        // Windows：mono.exe/libmono DLL 通常在 bin，PATH 需要 bin。
        TargetPlatform::Windows => {
            if bin.is_dir() {
                bin
            } else {
                mono_root.clone()
            }
        }
        // Linux：libmono.so 在 lib，LD_LIBRARY_PATH 需要 lib。
        TargetPlatform::Linux => {
            if lib.is_dir() {
                lib.clone()
            } else {
                mono_root.clone()
            }
        }
    };
    let managed_dir = if lib.is_dir() { lib } else { mono_root.clone() };

    let mut modified = Vec::new();
    let native_env = match platform {
        TargetPlatform::Windows => "PATH",
        TargetPlatform::Linux => "LD_LIBRARY_PATH",
    };
    if prepend_env_path(native_env, &native_dir, platform) {
        modified.push(native_env);
    }
    if prepend_env_path(MONO_PATH_ENV, &managed_dir, platform) {
        modified.push(MONO_PATH_ENV);
    }
    modified.sort_unstable();
    modified.dedup();

    MonoRuntimeSetup {
        found: true,
        root: Some(mono_root),
        native_dir: Some(native_dir),
        managed_dir: Some(managed_dir),
        modified,
        platform,
    }
}

/// 把 `path` 前置到环境变量 `name`（平台对应分隔符）；已位于首位时返回 false。
///
/// 不删除已有条目；必要时创建变量。空路径/无法解析为字符串时返回 false。
fn prepend_env_path(name: &str, path: &Path, platform: TargetPlatform) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    let separator = match platform {
        TargetPlatform::Windows => ';',
        TargetPlatform::Linux => ':',
    };
    let current = std::env::var(name).unwrap_or_default();
    let first = current.split(separator).next().unwrap_or("");
    if first == text {
        return false;
    }
    let new_value = if current.is_empty() {
        text.to_string()
    } else {
        format!("{text}{separator}{current}")
    };
    std::env::set_var(name, new_value);
    true
}
