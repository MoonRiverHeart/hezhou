//! 可选 C# 游戏逻辑入口（`feature = "mono-host"`）：加载包内 `scripts/*.dll`，
//! 调用 `Hezhou.PlayerGame.Initialize(IntPtr)` / `Update(float dt)` / `Shutdown()`。
//!
//! # 设计
//!
//! 直接使用 `hezhou-scripting` 的 Mono `ScriptManager`（`MonoUIExecutor` 的底层实现）：
//! - `load_script` 加载 DLL；`execute_with_ptr` 传 `Initialize`/`Shutdown` 的 `IntPtr`；
//! - `execute` 传 `Update` 的 `float dt`（namespace = `Hezhou`，class = `PlayerGame`）。
//!
//! `MonoUIExecutor` 目前没有「namespace + float 参数」的公开入口，因此这里不改
//! scripting crate，直接用同一 `ScriptManager` 完成加载/调用；所有 Mono 调用包
//! `catch_unwind`，失败只返回可读错误，由 host WARN 回退纯 Rust 渲染。
//!
//! # 限制（本轮）
//!
//! - 只加载 manifest.scripts 中**第一个 `.dll`**；
//! - `Initialize(0)` 的 context 指针本轮固定 0（C# 忽略/预留），FFI context 后续接入；
//! - 需要目标机器/包内有 Mono（`C:\Program Files\Mono` 或 exe 旁 `lib/`、`etc/`）；
//! - Mono 运行时不支持卸载 domain 后重载；host 进程退出前调用 `Shutdown`。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use hezhou_scripting::script_manager::ScriptManager;
use hezhou_scripting::ScriptError;

use crate::manifest::{PlayerManifest, SCRIPTS_DIR};
use crate::paths::{normalize_logical_path, to_native_path, PathError};

/// C# Player 入口命名空间。
pub const PLAYER_GAME_NAMESPACE: &str = "Hezhou";
/// C# Player 入口类名。
pub const PLAYER_GAME_CLASS: &str = "PlayerGame";

/// mono-host 错误（可读，不 panic）。
#[derive(Debug)]
pub enum MonoHostError {
    /// Mono ScriptManager 初始化/加载/调用失败。
    Scripting(ScriptError),
    /// `scripts/*.dll` 路径非 UTF-8，无法传给 Mono。
    NonUtf8Path { path: PathBuf },
    /// manifest.scripts 中的路径非法（绝对路径/`..`/空）。
    InvalidScriptPath { raw: String, source: PathError },
    /// 脚本 DLL 不存在。
    MissingScript { path: PathBuf },
    /// Mono 调用 panic（被捕获）。
    Panicked { operation: &'static str, message: String },
}

impl std::fmt::Display for MonoHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scripting(error) => write!(f, "Mono 脚本错误：{error}"),
            Self::NonUtf8Path { path } => {
                write!(f, "脚本路径不是 UTF-8，无法传给 Mono：{}", path.display())
            }
            Self::InvalidScriptPath { raw, source } => {
                write!(f, "manifest.scripts 路径非法 `{raw}`：{source}")
            }
            Self::MissingScript { path } => {
                write!(f, "脚本 DLL 不存在：{}", path.display())
            }
            Self::Panicked { operation, message } => {
                write!(f, "Mono {operation} 发生 panic（已捕获）：{message}")
            }
        }
    }
}

impl std::error::Error for MonoHostError {}

impl From<ScriptError> for MonoHostError {
    fn from(error: ScriptError) -> Self {
        Self::Scripting(error)
    }
}

/// 当前 mono-host 状态统计。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MonoHostStats {
    /// 已调用 Initialize 次数（0/1）。
    pub initialize_calls: u64,
    /// Update 调用成功次数。
    pub update_calls: u64,
    /// Shutdown 调用次数（0/1）。
    pub shutdown_calls: u64,
    /// 调用失败次数（host 会停止继续调用）。
    pub errors: u64,
}

/// 已加载的 C# PlayerGame 入口。
pub struct PlayerGameScript {
    manager: ScriptManager,
    assembly: String,
    dll_path: PathBuf,
    initialized: bool,
    shutdown_done: bool,
    stats: MonoHostStats,
}

impl PlayerGameScript {
    /// 加载 DLL 并定位 assembly；不要求 `PlayerGame` 类型立即存在（旧包加载后
    /// Initialize 才会报 `ClassNotFound`，由 host WARN 回退）。
    pub fn load(dll_path: &Path) -> Result<Self, MonoHostError> {
        if !dll_path.is_file() {
            return Err(MonoHostError::MissingScript {
                path: dll_path.to_path_buf(),
            });
        }
        let path_text = dll_path
            .to_str()
            .ok_or_else(|| MonoHostError::NonUtf8Path {
                path: dll_path.to_path_buf(),
            })?
            .to_string();
        let (manager, assembly) = catch_unwind(AssertUnwindSafe(|| {
            let mut manager = ScriptManager::new()?;
            let assembly = manager.load_script(&path_text)?;
            Ok::<_, ScriptError>((manager, assembly))
        }))
        .map_err(|payload| MonoHostError::Panicked {
            operation: "load",
            message: panic_message(payload),
        })??;
        Ok(Self {
            manager,
            assembly,
            dll_path: dll_path.to_path_buf(),
            initialized: false,
            shutdown_done: false,
            stats: MonoHostStats::default(),
        })
    }

    /// 脚本 DLL 路径。
    pub fn dll_path(&self) -> &Path {
        &self.dll_path
    }

    /// assembly 名（DLL 文件名去扩展名）。
    pub fn assembly(&self) -> &str {
        &self.assembly
    }

    /// 统计。
    pub fn stats(&self) -> &MonoHostStats {
        &self.stats
    }

    /// 是否已成功 Initialize。
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 调用 `Hezhou.PlayerGame.Initialize(0)`。
    pub fn initialize(&mut self, context: usize) -> Result<(), MonoHostError> {
        catch_unwind(AssertUnwindSafe(|| {
            self.manager.execute_with_ptr(
                &self.assembly,
                PLAYER_GAME_NAMESPACE,
                PLAYER_GAME_CLASS,
                "Initialize",
                context,
                1,
            )
        }))
        .map_err(|payload| MonoHostError::Panicked {
            operation: "Initialize",
            message: panic_message(payload),
        })?
        .map_err(MonoHostError::Scripting)?;
        self.initialized = true;
        self.stats.initialize_calls += 1;
        Ok(())
    }

    /// 调用 `Hezhou.PlayerGame.Update(float dt)`。
    pub fn update(&mut self, dt_seconds: f32) -> Result<(), MonoHostError> {
        let _result = catch_unwind(AssertUnwindSafe(|| {
            self.manager.execute(
                &self.assembly,
                PLAYER_GAME_NAMESPACE,
                PLAYER_GAME_CLASS,
                "Update",
                Some(dt_seconds),
                1,
            )
        }))
        .map_err(|payload| MonoHostError::Panicked {
            operation: "Update",
            message: panic_message(payload),
        })?
        .map_err(MonoHostError::Scripting)?;
        self.stats.update_calls += 1;
        Ok(())
    }

    /// 调用 `Hezhou.PlayerGame.Shutdown()` 并关闭 Mono domain；可重复调用。
    pub fn shutdown(&mut self) {
        if !self.shutdown_done && self.initialized {
            let result = catch_unwind(AssertUnwindSafe(|| {
                self.manager.execute_with_ptr(
                    &self.assembly,
                    PLAYER_GAME_NAMESPACE,
                    PLAYER_GAME_CLASS,
                    "Shutdown",
                    0,
                    0,
                )
            }));
            match result {
                Ok(Ok(())) => self.stats.shutdown_calls += 1,
                Ok(Err(_error)) => self.stats.errors += 1,
                Err(_payload) => self.stats.errors += 1,
            }
        }
        catch_unwind(AssertUnwindSafe(|| self.manager.shutdown())).ok();
        self.shutdown_done = true;
    }

    /// 记录一次调用错误（host 停止继续 Update）。
    pub fn record_error(&mut self) {
        self.stats.errors += 1;
    }
}

impl std::fmt::Debug for PlayerGameScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerGameScript")
            .field("assembly", &self.assembly)
            .field("dll_path", &self.dll_path)
            .field("initialized", &self.initialized)
            .field("shutdown_done", &self.shutdown_done)
            .field("stats", &self.stats)
            .finish()
    }
}

impl Drop for PlayerGameScript {
    fn drop(&mut self) {
        // host 正常路径会先显式 shutdown；这里兜底避免 Mono domain 泄漏时 panic。
        if !self.shutdown_done {
            self.shutdown();
        }
    }
}

/// 找到 manifest.scripts 中第一个 `.dll`，返回 `(逻辑文件名, 本机绝对路径)`。
///
/// 无 `.dll` 返回 `Ok(None)`（旧包：host WARN 后纯 Rust 渲染）。
pub fn find_player_game_script(
    manifest: &PlayerManifest,
    player_root: &Path,
) -> Result<Option<(String, PathBuf)>, MonoHostError> {
    for script in &manifest.scripts {
        if !script.to_ascii_lowercase().ends_with(".dll") {
            continue;
        }
        let logical =
            normalize_logical_path(script).map_err(|source| MonoHostError::InvalidScriptPath {
                raw: script.clone(),
                source,
            })?;
        let relative = to_native_path(&logical, manifest.target);
        let path = player_root.join(SCRIPTS_DIR).join(relative);
        return Ok(Some((logical, path)));
    }
    Ok(None)
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}
