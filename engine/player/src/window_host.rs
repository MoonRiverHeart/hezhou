//! 可选的真实窗口渲染 host（`feature = "window-host"`）。
//!
//! 目标：让 Player 能真正打开一个窗口并渲染最小测试场景，验证
//! "清单窗口配置 → GLFW 窗口 → Vulkan 渲染器 → 场景帧循环" 的链路；
//! 不接 cooked 场景/C#/资产加载（那些留给后续 game host）。
//!
//! 行为：
//! - 按 `manifest.window`（title/width/height）创建 `UIVulkanRenderer`；
//! - 使用运行时加载的起始场景（`context.start_scene`：包内 Project JSON 恢复，
//!   失败时由 `scene_loader` 回退到内置 cube + directional light）；
//!   host 不再无条件自建 cube，只记录实体数/来源；
//! - `set_game_state(1)`（Running）+ 固定相机 + 天空盒；
//! - 运行固定帧数（默认 [`DEFAULT_FRAME_LIMIT`]，可用环境变量
//!   [`FRAME_LIMIT_ENV`] 覆盖）或直到用户关闭窗口；
//! - 每帧写 `player.log`，返回 `HostOutcome::Running { frames }`（退出码 0）；
//! - 窗口/Vulkan/GPU 不可用或渲染 panic 时返回可读 [`HostError`]，不 panic、不退出进程。
//!
//! # 已知限制（上游 rhi-vulkan）
//!
//! 本 host **不在运行结束时调用 `UIVulkanRenderer::cleanup()`**：当前
//! `UIVulkanRenderer::cleanup()` 在"仅渲染内置场景、未初始化编辑器 UI/C#"的场景下会触发
//! Windows 堆损坏（exit `0xC0000374`），属于上游 rhi-vulkan 的资源释放问题（未在本任务
//! 范围内修复）。窗口 host 是一次性运行、返回后进程即将退出，因此让操作系统回收
//! Vulkan/GLFW 资源；显式 cleanup 修复后可重新启用（见 README TODO）。
//!
//! 关闭 feature 时本模块不编译，`resolve_host("default")` 仍返回无窗口的
//! `DefaultPlayerHost`（`Pending`，退出码 3）。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

use hezhou_core::Scene;
use hezhou_rhi_vulkan::UIVulkanRenderer;

use crate::host::{HostError, HostOutcome, PlayerHost, PlayerHostContext};
use crate::logging::LogLevel;

/// `--host window` 使用的 host 名称。
pub const WINDOW_HOST_NAME: &str = "window";
/// 默认渲染帧数（约 2 秒 @60fps）。
pub const DEFAULT_FRAME_LIMIT: u32 = 120;
/// 覆盖帧数的环境变量（例如 `HEZHOU_PLAYER_WINDOW_FRAMES=30`，便于手工/CI 冒烟）。
pub const FRAME_LIMIT_ENV: &str = "HEZHOU_PLAYER_WINDOW_FRAMES";
/// 帧数上限（防止误传超大值导致长时间占用）。
pub const MAX_FRAME_LIMIT: u32 = 100_000;
/// 内建场景的默认相机（yaw/pitch/xyz）。
const CAMERA: (f32, f32, f32, f32, f32) = (45.0, -30.0, 0.0, 1.5, 5.0);
/// 每帧休眠（约 60fps）。
const FRAME_SLEEP: Duration = Duration::from_millis(16);

/// 真实窗口 + Vulkan 渲染 host。
#[derive(Clone, Copy, Debug)]
pub struct WindowPlayerHost {
    frame_limit: u32,
}

impl WindowPlayerHost {
    /// 使用环境变量/默认帧数创建 host。
    pub fn new() -> Self {
        Self {
            frame_limit: frame_limit_from_env(),
        }
    }

    /// 指定渲染帧数（clamp 到 `1..=MAX_FRAME_LIMIT`）。
    pub fn with_frame_limit(frame_limit: u32) -> Self {
        Self {
            frame_limit: frame_limit.clamp(1, MAX_FRAME_LIMIT),
        }
    }

    /// 当前渲染帧数上限。
    pub fn frame_limit(&self) -> u32 {
        self.frame_limit
    }
}

impl Default for WindowPlayerHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerHost for WindowPlayerHost {
    fn name(&self) -> &'static str {
        WINDOW_HOST_NAME
    }

    fn run(&mut self, context: &mut PlayerHostContext<'_>) -> Result<HostOutcome, HostError> {
        let window = &context.manifest.window;
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "WindowPlayerHost：创建窗口 {}x{} mode={} title={:?}",
                window.width,
                window.height,
                window.mode.as_str(),
                window.title
            ),
        );

        // 使用运行时已加载的起始场景（Project JSON 恢复；失败时已回退内置 cube+light）。
        let entities = context.start_scene.get_all_entities().len();
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "使用起始场景：{entities} 个实体（来源：{}）",
                context.start_scene_source.summary()
            ),
        );
        context.log.host_log(
            LogLevel::Info,
            &format!("{}", context.imported_assets.summary()),
        );
        if context.start_scene_source.is_fallback() {
            context.log.host_log(
                LogLevel::Warn,
                "起始场景使用内置回退（cube + directional light），不是包内真实场景",
            );
        }

        // 创建窗口 + Vulkan 渲染器；失败/panic 都转为可读 HostError。
        let mut renderer = match catch_unwind(AssertUnwindSafe(|| {
            UIVulkanRenderer::new(window.width, window.height, &window.title)
        })) {
            Ok(Ok(renderer)) => renderer,
            Ok(Err(error)) => {
                return Err(HostError::new(format!(
                    "创建窗口/Vulkan 渲染器失败（{}x{}）：{error}；\
                     请确认显示环境、显卡驱动与 Vulkan 运行时可用",
                    window.width, window.height
                )))
            }
            Err(_) => {
                return Err(HostError::new(
                    "创建窗口/Vulkan 渲染器时发生 panic（已捕获）；请检查显示环境/驱动",
                ))
            }
        };
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "窗口与 Vulkan 渲染器创建成功：{}x{}，quality={}，frame_limit={}",
                renderer.get_extent().0,
                renderer.get_extent().1,
                context.manifest.quality.as_str(),
                self.frame_limit
            ),
        );

        renderer.set_scene(context.start_scene as *mut Scene);
        renderer.set_camera_params(CAMERA.0, CAMERA.1, CAMERA.2, CAMERA.3, CAMERA.4);
        renderer.set_skybox([0.08, 0.10, 0.16], [0.02, 0.02, 0.03]);
        // GameState::Running = 1（见 engine/AGENTS.md）
        renderer.set_game_state(1);

        let started = Instant::now();
        let mut frames: u32 = 0;
        let mut closed_by_user = false;
        while frames < self.frame_limit {
            let keep_running = match catch_unwind(AssertUnwindSafe(|| {
                renderer.process_events();
                renderer.draw_frame()
            })) {
                Ok(Ok(running)) => running,
                Ok(Err(error)) => {
                    return Err(HostError::new(format!(
                        "渲染第 {} 帧失败：{error}",
                        frames + 1
                    )))
                }
                Err(_) => {
                    return Err(HostError::new(format!(
                        "渲染第 {} 帧时发生 panic（已捕获）",
                        frames + 1
                    )))
                }
            };
            frames += 1;
            if !keep_running {
                closed_by_user = true;
                break;
            }
            if frames % 30 == 0 || frames == self.frame_limit {
                context.log.host_log(
                    LogLevel::Info,
                    &format!(
                        "窗口渲染进度：{}/{} 帧，{:.1} ms",
                        frames,
                        self.frame_limit,
                        started.elapsed().as_secs_f64() * 1000.0
                    ),
                );
            }
            std::thread::sleep(FRAME_SLEEP);
        }

        // 不调用 renderer.cleanup()：见模块文档"已知限制（上游 rhi-vulkan）"。
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "窗口 host 结束：{frames} 帧，{elapsed_ms:.1} ms（{}）",
                if closed_by_user {
                    "窗口被用户关闭"
                } else {
                    "达到固定帧数"
                }
            ),
        );
        Ok(HostOutcome::Running {
            frames: u64::from(frames),
        })
    }
}

/// 从环境变量解析帧数；非法/缺省时用 [`DEFAULT_FRAME_LIMIT`]。
pub fn frame_limit_from_env() -> u32 {
    match std::env::var(FRAME_LIMIT_ENV) {
        Ok(value) => parse_frame_limit(&value).unwrap_or(DEFAULT_FRAME_LIMIT),
        Err(_) => DEFAULT_FRAME_LIMIT,
    }
}

/// 解析帧数字符串（供环境变量与测试使用）。
pub fn parse_frame_limit(value: &str) -> Option<u32> {
    let parsed = value.trim().parse::<u32>().ok()?;
    if parsed == 0 {
        None
    } else {
        Some(parsed.min(MAX_FRAME_LIMIT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_limit_parsing_is_strict_and_clamped() {
        assert_eq!(parse_frame_limit("30"), Some(30));
        assert_eq!(parse_frame_limit("  1 "), Some(1));
        assert_eq!(parse_frame_limit("0"), None);
        assert_eq!(parse_frame_limit("-1"), None);
        assert_eq!(parse_frame_limit("abc"), None);
        assert_eq!(parse_frame_limit(""), None);
        assert_eq!(parse_frame_limit("999999999"), Some(MAX_FRAME_LIMIT));
    }

    #[test]
    fn with_frame_limit_clamps() {
        assert_eq!(WindowPlayerHost::with_frame_limit(5).frame_limit(), 5);
        assert_eq!(WindowPlayerHost::with_frame_limit(0).frame_limit(), 1);
        assert_eq!(
            WindowPlayerHost::with_frame_limit(u32::MAX).frame_limit(),
            MAX_FRAME_LIMIT
        );
        assert_eq!(WindowPlayerHost::default().frame_limit(), DEFAULT_FRAME_LIMIT);
        assert_eq!(WindowPlayerHost::new().name(), WINDOW_HOST_NAME);
    }
}
