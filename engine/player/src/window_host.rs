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
use crate::animation_control::PlayerAnimationState;
use crate::asset_catalog::PlayerAssetCatalog;
use crate::audio_runtime::PlayerAudioRuntime;
use crate::gameplay_runtime::PlayerGameplayRuntime;
#[cfg(feature = "mono-host")]
use crate::mono_host::{find_player_game_script, PlayerGameScript};
use crate::skinned_asset::{select_first_skinned_asset, SelectedSkinnedAsset};
use crate::skinned_asset::PLAYER_SKINNED_ENTITY_ID;

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
        // 质量档 → PBR draw path（0=Low/1=Medium/2=High；Ultra 本轮用 High）。
        let quality_preset = match context.manifest.quality {
            crate::config::QualityTier::Low => 0,
            crate::config::QualityTier::Medium => 1,
            crate::config::QualityTier::High | crate::config::QualityTier::Ultra => 2,
        };
        renderer.set_render_quality_preset_u32(quality_preset);
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "渲染质量档：{}（preset {}）；PBR draw path 将在首帧创建",
                context.manifest.quality.as_str(),
                quality_preset
            ),
        );

        // ---- 真实蒙皮资产：ImportedPrimitive → SkinnedVertex → task-28 上传 ----
        let mut skinned: Option<PlayerSkinnedState> = None;
        match select_first_skinned_asset(context.imported_assets) {
            Ok(Some(asset)) => {
                context.log.host_log(LogLevel::Info, &format!("{}", asset.summary()));
                match asset.palette_at(0.0) {
                    Ok(palette) => {
                        let upload = hezhou_rhi_vulkan::SkinnedMeshUpload {
                            entity_id: PLAYER_SKINNED_ENTITY_ID,
                            vertices: asset.vertices.clone(),
                            indices: asset.indices.clone(),
                            material: None,
                            texture_path: None,
                        };
                        match renderer.upload_skinned_mesh(upload) {
                            Ok(()) => {
                                let palette_bytes = palette.to_le_bytes();
                                match renderer.upload_skin_palette(
                                    PLAYER_SKINNED_ENTITY_ID,
                                    &palette_bytes,
                                ) {
                                    Ok(_) => {
                                        context.log.host_log(
                                            LogLevel::Info,
                                            &format!(
                                                "蒙皮网格上传成功：{} 顶点 / {} 三角形 / {} 关节（entity {}），palette 每帧更新",
                                                asset.vertex_count(),
                                                asset.triangle_count(),
                                                palette.len(),
                                                PLAYER_SKINNED_ENTITY_ID
                                            ),
                                        );
                                        skinned = Some(PlayerSkinnedState {
                                            animated: asset.animation.is_some(),
                                            asset,
                                        });
                                    }
                                    Err(error) => {
                                        let _ = renderer
                                            .remove_skinned_mesh(PLAYER_SKINNED_ENTITY_ID);
                                        context.log.host_log(
                                            LogLevel::Warn,
                                            &format!(
                                                "初始 joint palette 上传失败：{error}；回退已加载场景（内置 cube）"
                                            ),
                                        );
                                    }
                                }
                            }
                            Err(error) => context.log.host_log(
                                LogLevel::Warn,
                                &format!(
                                    "upload_skinned_mesh 失败：{error}；回退已加载场景（内置 cube）"
                                ),
                            ),
                        }
                    }
                    Err(error) => context.log.host_log(
                        LogLevel::Warn,
                        &format!(
                            "蒙皮 palette 计算失败：{error}；回退已加载场景（内置 cube）"
                        ),
                    ),
                }
            }
            Ok(None) => context.log.host_log(
                LogLevel::Warn,
                "包内没有带 joints/weights 的蒙皮 mesh：渲染已加载场景/内置 cube；不创建真实蒙皮绘制",
            ),
            Err(error) => context.log.host_log(
                LogLevel::Warn,
                &format!("蒙皮资产解析失败：{error}；回退已加载场景（内置 cube）"),
            ),
        }
        let mut palette_updates: u64 = 0;

        // 动画运行时状态：clip 列表/当前索引/时间/速度；palette 由本状态驱动（task-34）。
        let mut animation_state =
            PlayerAnimationState::from_selected_asset(skinned.as_ref().map(|state| &state.asset));
        let mut last_animation_tick = Instant::now();
        context.log.host_log(
            LogLevel::Info,
            &format!("蒙皮动画运行时：{}", animation_state.summary()),
        );

        // 资产目录 + 音频运行时（task-35）：默认 null backend，可选 audio-cpal。
        let asset_catalog = PlayerAssetCatalog::from_index(&context.index);
        context.log.host_log(LogLevel::Info, &asset_catalog.summary());
        let mut audio_runtime = PlayerAudioRuntime::new();
        context.log.host_log(
            LogLevel::Info,
            &format!(
                "音频引擎：backend={}（silent={}）",
                audio_runtime.backend(),
                audio_runtime.is_silent()
            ),
        );
        match audio_runtime.load_first_audio(&context.assets_dir, &context.index) {
            Ok(Some(sound_id)) => context.log.host_log(
                LogLevel::Info,
                &format!(
                    "音频：加载首个 Audio 资产 {}（backend={}，source={}）",
                    sound_id,
                    audio_runtime.backend(),
                    audio_runtime
                        .source()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "<none>".to_string())
                ),
            ),
            Ok(None) => context.log.host_log(
                LogLevel::Warn,
                "包内没有 Audio 资产：音频 API 安全空转（play_first 返回失败）",
            ),
            Err(error) => context.log.host_log(
                LogLevel::Warn,
                &format!("加载首个音频资产失败：{error}；音频 API 安全空转"),
            ),
        }
        let mut last_audio_tick = Instant::now();

        // 角色控制运行时（task-36）：y=0 地面 Plane + 每实体 CharacterMotor。
        let mut gameplay_runtime =
            PlayerGameplayRuntime::new(context.start_scene as *mut Scene);
        context.log.host_log(
            LogLevel::Info,
            &format!("{}", gameplay_runtime.summary()),
        );

        // ---- 可选 C# PlayerGame 入口（feature = mono-host）----
        #[cfg(feature = "mono-host")]
        let mut mono_state: Option<MonoHostState> = None;
        // PlayerCSharpContext 必须覆盖整个帧循环（C# 只在 Initialize/Update/Shutdown 内调用）。
        #[cfg(feature = "mono-host")]
        let csharp_context = crate::csharp_context::PlayerCSharpContext::new_with_gameplay(
            context.start_scene as *mut Scene,
            &mut animation_state as *mut PlayerAnimationState,
            &asset_catalog as *const PlayerAssetCatalog as *mut PlayerAssetCatalog,
            &mut audio_runtime as *mut PlayerAudioRuntime,
            &mut gameplay_runtime as *mut PlayerGameplayRuntime,
        );
        #[cfg(feature = "mono-host")]
        match find_player_game_script(context.manifest, context.root) {
            Ok(Some((name, path))) => {
                if !path.is_file() {
                    context.log.host_log(
                        LogLevel::Warn,
                        &format!(
                            "mono-host：脚本 DLL 不存在 {}；继续纯 Rust 渲染",
                            path.display()
                        ),
                    );
                } else {
                    context.log.host_log(
                        LogLevel::Info,
                        &format!(
                            "mono-host：加载 C# 入口 `{}`（{}）",
                            name,
                            path.display()
                        ),
                    );
                    context.log.host_log(
                        LogLevel::Info,
                        &format!(
                            "mono-host：Scene context 已提供（初始 {} 实体，ABI v{}，scene={:?}）",
                            context.start_scene.get_all_entities().len(),
                            csharp_context.version,
                            csharp_context.scene
                        ),
                    );
                    let context_ptr = if csharp_context.is_supported() {
                        csharp_context.as_ptr() as usize
                    } else {
                        0
                    };
                    match PlayerGameScript::load(&path) {
                        Ok(mut script) => match script.initialize(context_ptr) {
                            Ok(()) => {
                                context.log.host_log(
                                    LogLevel::Info,
                                    &format!(
                                        "mono-host：Hezhou.PlayerGame.Initialize(context={:#x}) 成功（assembly {}）；C# Initialize 后 Scene 实体 {}",
                                        context_ptr,
                                        script.assembly(),
                                        context.start_scene.get_all_entities().len()
                                    ),
                                );
                                mono_state = Some(MonoHostState {
                                    script,
                                    last_update: Instant::now(),
                                });
                            }
                            Err(error) => {
                                script.record_error();
                                context.log.host_log(
                                    LogLevel::Warn,
                                    &format!(
                                        "mono-host：Initialize 失败：{error}；继续纯 Rust 渲染（旧包/无 PlayerGame 入口）"
                                    ),
                                );
                            }
                        },
                        Err(error) => context.log.host_log(
                            LogLevel::Warn,
                            &format!(
                                "mono-host：加载 C# DLL 失败：{error}；继续纯 Rust 渲染"
                            ),
                        ),
                    }
                }
            }
            Ok(None) => context.log.host_log(
                LogLevel::Warn,
                "mono-host：manifest.scripts 没有 .dll，跳过 C# 入口（纯 Rust 渲染）",
            ),
            Err(error) => context.log.host_log(
                LogLevel::Warn,
                &format!("mono-host：解析脚本路径失败：{error}；继续纯 Rust 渲染"),
            ),
        }

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

            // 首帧后确认 PBR 真实蒙皮绘制路径状态（便于自动化/手工排查）。
            if frames == 1 {
                context.log.host_log(
                    LogLevel::Info,
                    &format!(
                        "PBR draw path 激活：{}；质量摘要：{}；已上传蒙皮网格：{}",
                        renderer.pbr_forward_active(),
                        renderer.render_quality_summary(),
                        renderer.skinned_mesh_count()
                    ),
                );
            }

            // 每帧推进动画状态并上传当前 clip/time 的 joint palette（无动画时静态姿态）。
            if let Some(state) = &skinned {
                let frame_dt = last_animation_tick.elapsed().as_secs_f32();
                last_animation_tick = Instant::now();
                animation_state.advance(frame_dt);
                let clip = animation_state.current_index();
                let time = animation_state.time();
                match state.asset.palette_for_clip(clip, time) {
                    Ok(palette) => {
                        if let Err(error) = renderer.upload_skin_palette(
                            PLAYER_SKINNED_ENTITY_ID,
                            &palette.to_le_bytes(),
                        ) {
                            context.log.host_log(
                                LogLevel::Warn,
                                &format!("第 {frames} 帧 joint palette 上传失败：{error}"),
                            );
                        } else {
                            palette_updates += 1;
                        }
                    }
                    Err(error) => context.log.host_log(
                        LogLevel::Warn,
                        &format!("第 {frames} 帧 joint palette 计算失败：{error}"),
                    ),
                }
            }

            // 可选：每帧推进音频状态机（null backend 必需；cpal 设备线程也会推进）。
            let audio_dt = last_audio_tick.elapsed().as_secs_f32();
            last_audio_tick = Instant::now();
            audio_runtime.update(audio_dt);
            gameplay_runtime.update(audio_dt);

            // 可选：每帧驱动 C# PlayerGame.Update(dt)。
            #[cfg(feature = "mono-host")]
            {
                let mut mono_error: Option<String> = None;
                if let Some(state) = &mut mono_state {
                    let dt = state.last_update.elapsed().as_secs_f32();
                    state.last_update = Instant::now();
                    if let Err(error) = state.script.update(dt) {
                        state.script.record_error();
                        mono_error = Some(format!("Update 失败：{error}"));
                    }
                }
                if let Some(message) = mono_error {
                    context.log.host_log(
                        LogLevel::Warn,
                        &format!(
                            "第 {frames} 帧 mono-host {message}；停止 C# 调用，继续纯 Rust 渲染"
                        ),
                    );
                    mono_state = None;
                }
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

        // 可选：Shutdown C# PlayerGame 并写统计。
        #[cfg(feature = "mono-host")]
        if let Some(mut state) = mono_state.take() {
            let dll = state.script.dll_path().display().to_string();
            state.script.shutdown();
            let stats = state.script.stats().clone();
            context.log.host_log(
                LogLevel::Info,
                &format!(
                    "mono-host：Shutdown 完成（Initialize {} 次，Update {} 次，Shutdown {} 次，错误 {} 次，DLL {}）；C# Shutdown 后 Scene 实体 {}",
                    stats.initialize_calls,
                    stats.update_calls,
                    stats.shutdown_calls,
                    stats.errors,
                    dll,
                    context.start_scene.get_all_entities().len()
                ),
            );
        } else {
            context.log.host_log(
                LogLevel::Info,
                "mono-host：本次未加载 C# PlayerGame 入口，纯 Rust 渲染",
            );
        }

        // 音频结束统计（C# Shutdown 通常会 stop；兜底停止并记录）。
        context.log.host_log(
            LogLevel::Info,
            &format!("音频统计：{}", audio_runtime.summary()),
        );
        audio_runtime.stop();
        context.log.host_log(
            LogLevel::Info,
            &format!("角色统计：{}", gameplay_runtime.summary()),
        );

        // 不调用 renderer.cleanup()：见模块文档"已知限制（上游 rhi-vulkan）"。
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        if let Some(state) = &skinned {
            context.log.host_log(
                LogLevel::Info,
                &format!(
                    "蒙皮绘制统计：{} 顶点 / {} 三角形 / {} 关节；palette 更新 {} 次（{}）；动画 {}",
                    state.asset.vertex_count(),
                    state.asset.triangle_count(),
                    state.asset.joint_count(),
                    palette_updates,
                    if state.animated {
                        "动画驱动"
                    } else {
                        "静态绑定姿态"
                    },
                    animation_state.summary()
                ),
            );
        }
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

/// 当前窗口 host 已上传的真实蒙皮资产状态。
struct PlayerSkinnedState {
    /// CPU 侧 mesh/skeleton/animation 与 palette 采样数据。
    asset: SelectedSkinnedAsset,
    /// 是否有动画 clip（无动画时每帧上传静态绑定姿态 palette）。
    animated: bool,
}

/// 当前 mono-host 状态（feature = mono-host）。
#[cfg(feature = "mono-host")]
struct MonoHostState {
    /// 已加载并 Initialize 成功的 C# PlayerGame。
    script: PlayerGameScript,
    /// 上一帧 Update 时间（计算 dt）。
    last_update: Instant,
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
