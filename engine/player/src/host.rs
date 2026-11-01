//! `PlayerHost` 接口与默认实现（本轮只做“等待 game host 接入”的明确契约）。
//!
//! 设计目的：把"运行时启动校验 + 资产索引"与"真正进入游戏循环"解耦。
//! 后续 game host 需要实现 [`PlayerHost`]，由 `hezhou-player --run --host <name>` 选择，
//! 运行时负责：读清单 → 校验布局 → 加载资产索引 → 调用 host。
//!
//! 下一轮 game host 的接入点（按顺序）：
//! 1. `platform`：创建窗口/GLFW + 输入 + DPI；
//! 2. `rhi` / `rhi-vulkan`：Vulkan device/swapchain/pipeline；
//! 3. `render` / `render-pipeline`：相机、PBR/阴影/后处理（quality 取自清单）；
//! 4. `scripting`：加载 `scripts/*.dll`，调用 C# 静态 `Initialize()`；
//! 5. `gameplay` / `animation` / `physics` / `audio`：加载场景与资产索引中的
//!    `imported/*.json`、音频/纹理字节；
//! 6. 进入帧循环；退出时返回 [`HostOutcome::Running`]（本 trait 的返回值即退出码语义）。

use std::fmt;
use std::path::Path;

use hezhou_core::Scene;

use crate::asset_index::PlayerAssetIndex;
use crate::logging::{FileLogger, LogLevel};
use crate::manifest::PlayerManifest;
use crate::scene_loader::StartSceneSource;

/// 默认 host 名称。
pub const DEFAULT_HOST_NAME: &str = "default";
/// `HostOutcome::Pending` 对应的进程退出码（区别于 0=成功、2=启动检查失败）。
pub const HOST_PENDING_EXIT_CODE: i32 = 3;

/// host 写日志的抽象，避免 host 直接依赖 [`FileLogger`]。
pub trait HostLog {
    /// 写一行 host 日志。
    fn host_log(&mut self, level: LogLevel, message: &str);
}

impl HostLog for FileLogger {
    fn host_log(&mut self, level: LogLevel, message: &str) {
        let _ = self.log(level, message);
    }
}

/// host 运行时上下文。
pub struct PlayerHostContext<'a> {
    /// Player 包根目录。
    pub root: &'a Path,
    /// 包内资产目录（`<root>/assets`）。
    pub assets_dir: &'a Path,
    /// 已校验的 Player manifest。
    pub manifest: &'a PlayerManifest,
    /// 已加载并校验 output 的资产索引。
    pub index: &'a PlayerAssetIndex,
    /// 已加载的起始场景（Project JSON 恢复；失败时为内置 cube+light 回退）。
    pub start_scene: &'a mut Scene,
    /// 起始场景来源（供 host 日志区分真实场景/回退）。
    pub start_scene_source: &'a StartSceneSource,
    /// 写入 `player.log` 的日志句柄。
    pub log: &'a mut dyn HostLog,
}

/// Player 游戏宿主接口。
///
/// 实现者从 [`PlayerHostContext`] 拿到包根、manifest 与资产索引，自行创建窗口/渲染/
/// 脚本运行时；`Ok(HostOutcome::Running { .. })` 表示完成一局/一次运行并正常退出。
pub trait PlayerHost {
    /// host 名称（CLI `--host` 使用）。
    fn name(&self) -> &'static str;

    /// 加载索引后由运行时调用。
    fn run(&mut self, context: &mut PlayerHostContext<'_>) -> Result<HostOutcome, HostError>;
}

/// host 运行结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostOutcome {
    /// 已进入游戏循环并正常结束。
    Running {
        /// 已渲染帧数（本轮的默认 host 不会产生）。
        frames: u64,
    },
    /// host 尚未接入：启动校验与索引加载已完成，但未进入游戏循环。
    Pending {
        /// 明确原因（写入日志/CLI 输出）。
        reason: String,
    },
}

impl HostOutcome {
    /// 进程退出码语义：`Running`=0；`Pending`=3（见 [`HOST_PENDING_EXIT_CODE`]）。
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Running { .. } => 0,
            Self::Pending { .. } => HOST_PENDING_EXIT_CODE,
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }

    pub fn pending_reason(&self) -> Option<&str> {
        match self {
            Self::Pending { reason } => Some(reason),
            Self::Running { .. } => None,
        }
    }
}

/// host 错误。
#[derive(Debug)]
pub struct HostError {
    /// 可读错误信息。
    pub message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HostError {}

/// 本轮默认 host：加载索引后明确报告"等待 game host 接入"，不创建窗口/渲染/脚本运行时。
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultPlayerHost;

impl DefaultPlayerHost {
    pub fn new() -> Self {
        Self
    }
}

impl PlayerHost for DefaultPlayerHost {
    fn name(&self) -> &'static str {
        DEFAULT_HOST_NAME
    }

    fn run(&mut self, context: &mut PlayerHostContext<'_>) -> Result<HostOutcome, HostError> {
        let manifest = context.manifest;
        let index = context.index;
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "PlayerHost `{}`：包根 {}；资产目录 {}",
                self.name(),
                context.root.display(),
                context.assets_dir.display()
            ),
        );
        context
            .log
            .host_log(LogLevel::Info, &format!("{}", index.summary()));
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "起始场景：{}（运行时已加载 {} 个实体）",
                context.start_scene_source.summary(),
                context.start_scene.get_all_entities().len()
            ),
        );
        if context.start_scene_source.is_fallback() {
            context.log.host_log(
                LogLevel::Warn,
                "起始场景使用内置回退（cube + directional light），不是包内真实场景",
            );
        }
        if index.is_empty() {
            context.log.host_log(
                LogLevel::Warn,
                "包内没有 assets/cook_manifest.json 或 asset_id：资产索引为空（legacy 包），\
                 host 无法按 AssetId 加载资源",
            );
        }
        if let Some(scene) = index.start_scene() {
            context.log.host_log(
                LogLevel::Info,
                &format!(
                    "启动场景：AssetId {}（kind {}，output {}）",
                    scene.id,
                    scene.kind.as_str(),
                    scene.output
                ),
            );
        }
        if let Some(font) = index.font() {
            context.log.host_log(
                LogLevel::Info,
                &format!(
                    "字体：AssetId {}（kind {}，output {}）",
                    font.id,
                    font.kind.as_str(),
                    font.output
                ),
            );
        }
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "窗口配置：{}x{} {}；质量档 {}；目标平台 {}",
                manifest.window.width,
                manifest.window.height,
                manifest.window.mode.as_str(),
                manifest.quality.as_str(),
                manifest.target
            ),
        );
        context.log.host_log(
            LogLevel::Warn,
            "等待 game host 接入：本轮不创建窗口、不初始化 Vulkan/Mono、不调用 C# Initialize()",
        );
        context.log.host_log(
            LogLevel::Info,
            "下一轮接入顺序：platform(window/input) → rhi-vulkan → render → scripting(Mono/C# Initialize) \
             → gameplay/animation/physics/audio → 帧循环；资产按 AssetId 从索引读取。",
        );

        Ok(HostOutcome::Pending {
            reason: "game host（窗口/Vulkan/Mono/C# Initialize）尚未接入；本轮仅完成启动校验与资产索引加载"
                .to_string(),
        })
    }
}

/// 按名称选择 host 实现。
///
/// - `default`：编译了 `window-host` feature 时使用真实窗口渲染 host
///   （[`crate::window_host::WindowPlayerHost`]），否则使用无窗口的
///   [`DefaultPlayerHost`]（返回 `Pending`，退出码 3）；
/// - `window`：显式选择窗口 host；未编译 feature 时返回可读错误；
/// - `headless` / `null`：强制无窗口 host（CI/无 GPU 环境）。
pub fn resolve_host(name: &str) -> Result<Box<dyn PlayerHost>, HostError> {
    match name.trim() {
        DEFAULT_HOST_NAME => Ok(default_host()),
        "headless" | "null" => Ok(Box::new(DefaultPlayerHost::new())),
        "window" => window_host(),
        other => Err(HostError::new(format!(
            "未知 host `{other}`：支持 default/window/headless/null（window 需 window-host feature）"
        ))),
    }
}

fn default_host() -> Box<dyn PlayerHost> {
    #[cfg(feature = "window-host")]
    {
        Box::new(crate::window_host::WindowPlayerHost::new())
    }
    #[cfg(not(feature = "window-host"))]
    {
        Box::new(DefaultPlayerHost::new())
    }
}

fn window_host() -> Result<Box<dyn PlayerHost>, HostError> {
    #[cfg(feature = "window-host")]
    {
        Ok(Box::new(crate::window_host::WindowPlayerHost::new()))
    }
    #[cfg(not(feature = "window-host"))]
    {
        Err(HostError::new(
            "host `window` 需要编译 `window-host` feature：\
             cargo build -p hezhou-player --features window-host",
        ))
    }
}
