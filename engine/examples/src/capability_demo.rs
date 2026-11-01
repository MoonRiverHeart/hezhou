//! `capability_demo` — Hezhou 0.2 引擎能力演示工程（headless）。
//!
//! 这是一个**能力组合示例**，不是具体游戏：它把 0.2 交付的六个能力模块
//! 按“一个角色在竞技场里跑一段固定 60Hz 战斗循环”的方式串起来，用来证明
//! 这些引擎 API 可以协同工作。示例里没有关卡、剧情、敌人 AI 或数值设计——
//! 所有数值都只是为了让各能力模块产生可观测输出。
//!
//! 组合的能力：
//!
//! | 能力 | 本 demo 中的用法 |
//! |---|---|
//! | physics | `PhysicsCombatAdapter`：地面/墙/角色/动态道具、固定步、查询 |
//! | gameplay | `CharacterMotor`、`ThirdPersonCamera`、`LockOnState`、`ComboGraph`、`Health`、`resolve_damage` |
//! | animation | 程序化骨架 + clip + `AnimationStateMachine`（经 integration 驱动） |
//! | integration | `AnimationActionAdapter`（逐招式 clip/事件）+ `PhysicsCombatAdapter`（单一积分来源） |
//! | asset | 生成 fixture 工程 → `AssetDatabase` 扫描 → `Cooker` cook → `CookManifest` 读取校验 |
//! | audio | `AudioEngine::null()` 无声后端：加载/播放/暂停/恢复/停止/3D 衰减（无真实声卡） |
//! | player | `PlayerBuilder` 双平台 dry-run 计划 + `PlayerManifest` 落盘/校验 |
//!
//! ## 每帧顺序（固定 60Hz，与 integration README 一致）
//!
//! ```text
//! input -> motor.step(fixed_dt, &input, &adapter.character_query_for(player))
//!       -> adapter.sync_character_transform(player, output.position)
//!       -> combo.press(...) / combo.update(fixed_dt, &graph)
//!       -> 命中窗口内 HitboxRuntime.detect(&adapter.combat_hit_query_for(player), ...)
//!       -> DamageEvent::from_contact -> resolve_damage
//!       -> 动画：combat.action_state() / locomotion_state -> ActionAnimator -> ActionAdapter.update
//!       -> camera.update(..., &adapter.camera_query_for(player)) + LockOn
//!       -> adapter.advance(frame_dt)  // 只推进动态刚体，不积分角色
//!       -> audio.update(fixed_dt)
//! ```
//!
//! ## 输出
//!
//! 默认写 `test-results/capability-demo/report.json`（仓库根目录下），包含能力项、
//! 步数、命中/伤害、状态机、物理位置、通过/失败。退出码：全部能力通过 `0`，否则非 0。
//!
//! ```text
//! cargo run -p hezhou-examples --bin capability_demo
//! cargo run -p hezhou-examples --bin capability_demo -- --steps 300 --out target/demo-report.json
//! cargo test -p hezhou-examples --bin capability_demo
//! ```
//!
//! 不依赖窗口、Vulkan、Mono 或编辑器：任何 headless CI 都能跑。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use hezhou_animation::{
    AnimationClip, AnimationEvent as ClipEvent, AnimationState, Channel, Keyframe, Skeleton,
    Transform, Vec3 as AnimVec3,
};
use hezhou_asset::{
    fnv1a64_hex, AssetDatabase, CookManifest as AssetCookManifest, Cooker, LogicalPath,
};
use hezhou_audio::{AudioEngine, PlaybackState, SoundData};
use hezhou_gameplay::{
    from_core_vec3, locomotion_state, resolve_damage, ActionAnimator, ActionId, AttackDefinition,
    AttackId, AttackKind, CharacterMotor, CombatantId, ComboConfig, ComboGraph, ComboInputResult,
    ComboState, DamageKind, DamageResult, Health, Hitbox, HitboxPose, HitboxRuntime, HitboxShape,
    LockOnConfig, LockOnState, LockTargetCandidate, MotorConfig, MotorInput, ThirdPersonCamera,
    ThirdPersonCameraConfig, Vec3,
};
use hezhou_integration::{AnimationActionAdapter, PhysicsCombatAdapter};
use hezhou_physics::layers;
use hezhou_player::{
    ArtifactCategory, ManifestAsset, PlayerBuildConfig, PlayerBuildInput, PlayerBuilder,
    PlayerManifest, QualityTier, ScriptAssembly, TargetPlatform, WindowConfig,
};

/// 报告格式版本（字段变化时递增）。
const REPORT_FORMAT_VERSION: u32 = 1;
/// 固定步频率。
const FIXED_HZ: u32 = 60;
/// 默认真实模拟步数（6 秒 @60Hz）。
const DEFAULT_STEPS: u32 = 360;
/// 演示场景的固定位置（不写死任何游戏规则，仅为可观测输出）。
const ENEMY_CENTER: Vec3 = Vec3::new(0.0, 1.0, -2.0);
const WALL_CENTER: Vec3 = Vec3::new(0.0, 2.0, 2.5);

// ===========================================================================
// 报告数据结构（serde：同时用于写 JSON 与测试断言）
// ===========================================================================

/// 一项能力检查。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityCheck {
    /// 能力项名称（点分命名，如 `physics.ground_probe`）。
    pub name: String,
    /// `pass` / `fail`。
    pub status: String,
    /// 人类可读的细节。
    pub detail: String,
}

impl CapabilityCheck {
    fn new(name: &str, passed: bool, detail: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: if passed { "pass" } else { "fail" }.to_string(),
            detail: detail.into(),
        }
    }

    /// 是否通过。
    pub fn passed(&self) -> bool {
        self.status == "pass"
    }
}

/// 物理能力输出。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicsReport {
    /// 模拟结束时的角色脚底位置。
    pub final_position: [f32; 3],
    /// 是否落在地面。
    pub grounded: bool,
    /// 是否观测到相机避障。
    pub camera_collided: bool,
    /// 锁定目标（`CombatantId` 原始值）。
    pub lock_on_target: Option<u64>,
    /// 物理世界刚体数量。
    pub world_bodies: usize,
    /// 动态道具最终高度（固定步推进的证据）。
    pub dynamic_prop_y: f32,
}

/// 动画能力输出。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationReport {
    /// 状态机中出现过的 animation 状态。
    pub states_seen: Vec<String>,
    /// 动画命中帧（clip 事件 → gameplay `HitFrame`）数量。
    pub hit_frame_events: u32,
    /// `ActionStarted` 事件数量。
    pub action_started_events: u32,
    /// 模拟结束时的 animation 状态。
    pub current_state: String,
}

/// 战斗能力输出。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CombatReport {
    /// 起手次数。
    pub attacks_started: u32,
    /// 排队输入次数。
    pub inputs_queued: u32,
    /// 实际连招衔接次数。
    pub chains: u32,
    /// 命中次数（按目标去重后）。
    pub hits: u32,
    /// 总伤害。
    pub damage_total: f32,
    /// 敌人剩余生命值。
    pub enemy_health: f32,
    /// 敌人是否死亡。
    pub enemy_killed: bool,
    /// 出现过的攻击阶段。
    pub phases_seen: Vec<String>,
}

/// 资产能力输出。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetReport {
    /// fixture 工程根目录。
    pub project_root: String,
    /// 资产数据库中的资产数量。
    pub asset_count: usize,
    /// cook manifest 条目数量。
    pub cooked_entries: usize,
    /// cook manifest 路径。
    pub manifest_path: String,
    /// cook 输出目录。
    pub cook_output_dir: String,
    /// 原样拷贝的文件数。
    pub copied_files: usize,
    /// 是否验证了源文件哈希。
    pub hash_verified: bool,
}

/// 音频能力输出。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioReport {
    /// 后端名称（无声后端为 `null`）。
    pub backend: String,
    /// 是否为无声输出。
    pub silent: bool,
    /// 加载的资源数量。
    pub sounds_loaded: usize,
    /// 播放过的 voice 数量。
    pub voices_played: u32,
    /// 主音量。
    pub master_volume: f32,
    /// 循环 BGM 在模拟结束时的状态。
    pub final_state: String,
}

/// Player 能力输出。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerReport {
    /// 清单是否通过校验。
    pub manifest_valid: bool,
    /// 双平台 dry-run 计划文件总数。
    pub planned_files: usize,
    /// Windows 计划的可执行文件名。
    pub planned_executable: String,
    /// 计划警告数量。
    pub warnings: usize,
    /// 文件清单指纹。
    pub fingerprint: String,
    /// 计划覆盖的目标平台。
    pub targets: Vec<String>,
    /// 落盘的 `manifest.json` 路径。
    pub manifest_path: String,
    /// 启动场景。
    pub start_scene: String,
}

/// 完整报告。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DemoReport {
    /// 报告格式版本。
    pub format_version: u32,
    /// 演示名称。
    pub demo: String,
    /// 引擎版本（`CARGO_PKG_VERSION`）。
    pub engine_version: String,
    /// UTC 生成时间（RFC3339）。
    pub generated_at_utc: String,
    /// 固定步频率。
    pub fixed_hz: u32,
    /// 实际执行步数。
    pub steps: u32,
    /// 模拟时长（秒）。
    pub simulated_seconds: f32,
    /// 能力检查列表。
    pub capabilities: Vec<CapabilityCheck>,
    /// 物理输出。
    pub physics: PhysicsReport,
    /// 动画输出。
    pub animation: AnimationReport,
    /// 战斗输出。
    pub combat: CombatReport,
    /// 资产输出。
    pub asset: AssetReport,
    /// 音频输出。
    pub audio: AudioReport,
    /// Player 输出。
    pub player: PlayerReport,
    /// 是否全部通过。
    pub passed: bool,
    /// 失败项说明。
    pub failures: Vec<String>,
    /// 报告文件路径。
    pub report_path: String,
}

// ===========================================================================
// 命令行参数
// ===========================================================================

/// demo 运行参数。
#[derive(Debug, Clone)]
pub struct DemoOptions {
    /// 报告输出路径。
    pub report_path: PathBuf,
    /// 模拟步数（60Hz 固定步）。
    pub steps: u32,
}

impl DemoOptions {
    /// 默认报告路径：仓库根目录 `test-results/capability-demo/report.json`。
    pub fn default_report_path() -> PathBuf {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or(manifest_dir);
        repo_root
            .join("test-results")
            .join("capability-demo")
            .join("report.json")
    }

    /// 从命令行参数解析（`--out <path>`、`--steps <n>`、`-h`）。
    pub fn from_args<I: IntoIterator<Item = String>>(args: I) -> Self {
        let mut options = Self {
            report_path: Self::default_report_path(),
            steps: DEFAULT_STEPS,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--out" => {
                    if let Some(value) = iter.next() {
                        options.report_path = PathBuf::from(value);
                    }
                }
                "--steps" => {
                    if let Some(value) = iter.next() {
                        if let Ok(steps) = value.parse::<u32>() {
                            options.steps = steps.max(1);
                        }
                    }
                }
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        options
    }
}

fn print_help() {
    println!("Hezhou 0.2 capability demo（headless 能力组合示例）");
    println!();
    println!("用法: capability_demo [--out <report.json>] [--steps <n>]");
    println!(
        "默认: --out {}/report.json --steps {DEFAULT_STEPS}",
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .map(|root| root
                .join("test-results/capability-demo")
                .display()
                .to_string())
            .unwrap_or_else(|| "test-results/capability-demo".to_string())
    );
}

// ===========================================================================
// 主流程
// ===========================================================================

fn main() {
    let options = DemoOptions::from_args(std::env::args().skip(1));
    println!("Hezhou 0.2 capability demo（headless 能力组合示例）");
    println!("report: {}", options.report_path.display());

    let report = match run_demo(&options) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("capability demo 失败: {error}");
            std::process::exit(2);
        }
    };

    print_summary(&report);
    if report.passed {
        println!("RESULT: PASS（退出码 0）");
    } else {
        eprintln!("RESULT: FAIL（退出码 1）");
        std::process::exit(1);
    }
}

/// 运行 demo 并写出报告；不调用 `process::exit`，便于测试复用。
pub fn run_demo(options: &DemoOptions) -> Result<DemoReport, String> {
    let work_dir = options
        .report_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("test-results/capability-demo"));
    fs::create_dir_all(&work_dir).map_err(|error| format!("创建输出目录失败: {error}"))?;

    let asset = run_asset_capability(&work_dir)?;
    let player = run_player_capability(&work_dir)?;

    let fixed_dt = 1.0 / FIXED_HZ as f32;
    let steps = options.steps.max(1);

    // ---- 物理世界（integration 的 PhysicsCombatAdapter）-------------------
    let mut physics = PhysicsCombatAdapter::new();
    physics.spawn_static_box_with_layers(
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(60.0, 1.0, 60.0),
        layers::WORLD,
        layers::ALL,
    );
    physics.spawn_static_box_with_layers(
        WALL_CENTER,
        Vec3::new(6.0, 4.0, 0.5),
        layers::WORLD,
        layers::ALL,
    );
    let player_id = CombatantId::new(1);
    physics.spawn_character(player_id, Vec3::new(0.0, 0.2, 0.0), 0.35, 1.8);
    let enemy_id = CombatantId::new(2);
    let enemy_body = physics.spawn_static_box_with_layers(
        ENEMY_CENTER,
        Vec3::new(0.7, 1.8, 0.7),
        layers::ENEMY,
        layers::ALL,
    );
    physics.register_combatant(enemy_id, enemy_body);
    // 命中查询只看 ENEMY 层（避免地面/墙体参与）。
    physics.set_hit_layer_mask(layers::ENEMY);
    // 相机/视线只看 WORLD 层：相机避障仍会撞墙，但视线不会被角色/敌人自身遮挡
    // （0.2 的 QueryFilter 只能排除一个 body；用层掩码表达"只被场景遮挡"）。
    physics.set_camera_layer_mask(layers::WORLD);
    // 动态道具：证明 physics 固定步只积分动态刚体（角色是 Kinematic，不参与）。
    let prop = physics.spawn_dynamic_box(Vec3::new(3.0, 6.0, 0.0), Vec3::new(0.5, 0.5, 0.5));

    // ---- gameplay：角色 / 相机 / 锁定 / 连招 / 生命 -----------------------
    let mut motor = CharacterMotor::with_position(MotorConfig::default(), Vec3::new(0.0, 0.2, 0.0));
    let mut camera = ThirdPersonCamera::new(ThirdPersonCameraConfig::default());
    let lock_config = LockOnConfig::default();
    let mut lock_on = LockOnState::new();
    let lock_candidates = [LockTargetCandidate::new(enemy_id, ENEMY_CENTER)];

    let mut graph = ComboGraph::new();
    graph.add_attack(
        AttackDefinition::new(AttackId(1), AttackKind::Light)
            .with_timing(0.12, 0.10, 0.25)
            .with_next([AttackId(2)])
            .with_hitbox(light_hitbox())
            .with_damage(20.0, DamageKind::Light)
            .with_knockback(2.0)
            .with_action(ActionId::new(1)),
    );
    graph.add_attack(
        AttackDefinition::new(AttackId(2), AttackKind::Light)
            .with_timing(0.10, 0.08, 0.22)
            .as_combo_only()
            .with_hitbox(light_hitbox())
            .with_damage(25.0, DamageKind::Light)
            .with_action(ActionId::new(2)),
    );
    graph.add_attack(
        AttackDefinition::new(AttackId(3), AttackKind::Heavy)
            .with_timing(0.20, 0.12, 0.35)
            .with_hitbox(heavy_hitbox())
            .with_damage(40.0, DamageKind::Heavy)
            .with_knockback(4.0)
            .with_action(ActionId::new(3)),
    );
    let combo_config = ComboConfig::default();
    let mut combo = ComboState::new();
    let mut enemy_health = Health::new(100.0);
    let mut hitbox_runtime = HitboxRuntime::new();
    let mut last_attack: Option<AttackId> = None;
    let mut attacks_started = 0u32;
    let mut inputs_queued = 0u32;
    let mut chains = 0u32;
    let mut phases_seen: BTreeSet<String> = BTreeSet::new();

    // 固定脚本：起手轻击 -> 预输入第二段 -> 重击（演示用，不是游戏规则）。
    let scripted_attacks: [(u32, AttackId); 3] =
        [(30, AttackId(1)), (38, AttackId(2)), (180, AttackId(3))];

    // ---- animation：程序化骨架 + clip + 状态机（经 integration 驱动）----
    let mut skeleton = Skeleton::new("capability-hero");
    skeleton.add_root_joint("root", Transform::IDENTITY);
    let mut anim = AnimationActionAdapter::with_default_transition(0.1);
    anim.set_clip(AnimationState::Idle, translation_clip("Idle", 1.0))
        .set_clip(AnimationState::Run, translation_clip("Run", 0.8));
    anim.set_action_clip(ActionId::new(1), clip_with_event("Attack_1", 0.47, 0.18))
        .set_action_clip(ActionId::new(2), clip_with_event("Attack_2", 0.40, 0.12))
        .set_action_clip(
            ActionId::new(3),
            clip_with_event("Heavy_Attack", 0.67, 0.28),
        );
    let mut animator = ActionAnimator::new();
    let mut animation_states: BTreeSet<String> = BTreeSet::new();
    let mut hit_frame_events = 0u32;
    let mut action_started_events = 0u32;

    // ---- audio：无声后端（无真实声卡）------------------------------------
    let audio_engine = AudioEngine::null();
    audio_engine.load_sound(
        "hit",
        SoundData::sine("hit", 440.0, 0.08, 48_000)
            .map_err(|error| format!("音频资源: {error}"))?,
    );
    audio_engine.load_sound(
        "step",
        SoundData::sine("step", 220.0, 0.05, 48_000)
            .map_err(|error| format!("音频资源: {error}"))?,
    );
    let bgm = audio_engine
        .play_with(
            hezhou_audio::PlayOptions::new("step")
                .looping(true)
                .volume(0.4),
        )
        .ok_or_else(|| "无声后端应允许逻辑播放".to_string())?;
    let mut voices_played = 1u32;
    let sound_count = audio_engine.cached_sound_ids().len() as u32;
    let mut audio_paused = false;
    let mut audio_resumed = false;

    // ---- 模拟循环 ---------------------------------------------------------
    let mut camera_collided = false;
    let mut lock_target: Option<u64> = None;
    let mut hits = 0u32;
    let mut damage_total = 0.0f32;
    let mut enemy_killed = false;

    for step in 0..steps {
        // 1) input：前 15 步向前走，之后停下（演示 locomotion）。
        let mut input = if step < 15 {
            MotorInput::with_direction(Vec3::FORWARD)
        } else {
            MotorInput::default()
        };
        if combo.is_attacking() {
            input.speed_scale = 0.3;
        }

        // 2) gameplay motor：唯一积分来源；physics 只提供 move_capsule/ground_probe 查询。
        let motor_output = {
            let query = physics.character_query_for(player_id);
            motor.step(fixed_dt, &input, &query)
        };
        physics.sync_character_transform(player_id, motor_output.position);

        // 3) 相机 + 锁定（相机避障走 physics 球扫；视线走 raycast）。
        let camera_output = {
            let query = physics.camera_query_for(player_id);
            camera.update(fixed_dt, motor_output.position, &query)
        };
        if camera_output.collided {
            camera_collided = true;
        }
        {
            let visibility = physics.camera_query_for(player_id);
            lock_on.select(
                camera_output.position,
                camera_output.forward,
                &lock_candidates,
                &lock_config,
                Some(&visibility),
            );
        }
        let lock_update = lock_on.update(camera_output.position, &lock_candidates, &lock_config);
        if let Some(target) = lock_update.target {
            lock_target = Some(target.raw());
        }

        // 4) 连招输入（脚本化）。
        for (at_step, attack) in &scripted_attacks {
            if step == *at_step {
                match combo.press(*attack, &graph, &combo_config) {
                    ComboInputResult::Started(_) => attacks_started += 1,
                    ComboInputResult::Queued(_) => inputs_queued += 1,
                    ComboInputResult::Rejected => {}
                }
            }
        }
        let combo_update = combo.update(fixed_dt, &graph);
        if combo_update.chained.is_some() {
            chains += 1;
        }
        if let Some(phase) = combo_update.phase {
            phases_seen.insert(format!("{phase:?}"));
        }

        // 5) 命中检测（只在判定窗口内；HitboxRuntime 保证一次攻击只命中同一目标一次）。
        let current_attack = combo.current_attack();
        if current_attack != last_attack {
            if current_attack.is_some() {
                hitbox_runtime.begin();
            }
            last_attack = current_attack;
        }
        if combo_update.hitbox_active {
            if let Some(hitbox) = combo_update.hitbox {
                let pose = HitboxPose::new(motor_output.position, Vec3::FORWARD);
                let contacts = {
                    let query = physics.combat_hit_query_for(player_id);
                    hitbox_runtime.detect(&query, &hitbox, &pose)
                };
                if !contacts.is_empty() {
                    if let Some(attack) = combo.current_attack() {
                        if let Some(definition) = graph.attack(attack) {
                            for event in
                                definition.damage_events(player_id, Vec3::FORWARD, &contacts)
                            {
                                let outcome = resolve_damage(
                                    &mut enemy_health,
                                    None,
                                    None,
                                    Vec3::FORWARD,
                                    &event,
                                );
                                if outcome.result == DamageResult::Applied {
                                    hits += 1;
                                    damage_total += outcome.damage;
                                }
                                if outcome.killed {
                                    enemy_killed = true;
                                }
                            }
                            if let Some(hit_sound) = audio_engine.play_3d(
                                "hit",
                                hezhou_audio::Vec3::new(
                                    ENEMY_CENTER.x,
                                    ENEMY_CENTER.y,
                                    ENEMY_CENTER.z,
                                ),
                            ) {
                                let _ = hit_sound;
                                voices_played += 1;
                            }
                        }
                    }
                }
            }
        }

        // 6) 动画：战斗状态优先，否则 locomotion；事件由 integration 映射。
        let desired = match combo.action_state(&graph) {
            Some(action_state) => action_state,
            None => locomotion_state(&motor_output, 4.0),
        };
        if animator.current() != Some(desired) {
            if desired.is_one_shot() {
                animator.play_one_shot(desired, anim.bridge_mut());
            } else {
                animator.sync(desired, anim.bridge_mut());
            }
        }
        for event in anim.update(fixed_dt) {
            match event {
                hezhou_gameplay::AnimationEvent::HitFrame { .. } => hit_frame_events += 1,
                hezhou_gameplay::AnimationEvent::ActionStarted(_) => action_started_events += 1,
                _ => {}
            }
        }
        animation_states.insert(anim.current_state().name().to_string());

        // 7) physics 固定步（只推进动态刚体；角色位置由 motor 同步）。
        physics.advance(fixed_dt);

        // 8) 音频逻辑时间推进（无声后端）。
        let _ = audio_engine.update(fixed_dt);
        if step == 60 && !audio_paused {
            bgm.pause();
            audio_paused = true;
        }
        if step == 90 && audio_paused && !audio_resumed {
            bgm.resume();
            audio_resumed = true;
        }
    }

    let final_prop_y = from_core_vec3(
        *physics
            .world()
            .get_body(prop)
            .ok_or_else(|| "动态道具 body 存在".to_string())?
            .position(),
    )
    .y;
    bgm.stop();
    let audio_report = AudioReport {
        backend: audio_engine.status().backend().to_string(),
        silent: audio_engine.status().is_silent(),
        sounds_loaded: sound_count as usize,
        voices_played,
        master_volume: audio_engine.master_volume(),
        final_state: playback_state_name(bgm.state()),
    };

    let physics_report = PhysicsReport {
        final_position: vec3_array(motor.position()),
        grounded: motor.is_grounded(),
        camera_collided,
        lock_on_target: lock_target,
        world_bodies: physics.world().body_count(),
        dynamic_prop_y: final_prop_y,
    };
    let animation_report = AnimationReport {
        states_seen: animation_states.into_iter().collect(),
        hit_frame_events,
        action_started_events,
        current_state: anim.current_state().name().to_string(),
    };
    let combat_report = CombatReport {
        attacks_started,
        inputs_queued,
        chains,
        hits,
        damage_total,
        enemy_health: enemy_health.current(),
        enemy_killed,
        phases_seen: phases_seen.into_iter().collect(),
    };

    let capabilities = vec![
        CapabilityCheck::new(
            "physics.world",
            physics_report.world_bodies >= 4,
            format!(
                "bodies={}（地面/墙/角色/敌人/道具）",
                physics_report.world_bodies
            ),
        ),
        CapabilityCheck::new(
            "physics.character_motor",
            physics_report.grounded,
            format!(
                "grounded={} final_position={:?}",
                physics_report.grounded, physics_report.final_position
            ),
        ),
        CapabilityCheck::new(
            "physics.camera_collision",
            physics_report.camera_collided,
            format!("camera_collided={}", physics_report.camera_collided),
        ),
        CapabilityCheck::new(
            "physics.hit_detection",
            combat_report.hits > 0,
            format!(
                "hits={} damage={:.1}",
                combat_report.hits, combat_report.damage_total
            ),
        ),
        CapabilityCheck::new(
            "physics.fixed_step_dynamic",
            physics_report.dynamic_prop_y < 6.0,
            format!("dynamic_prop_y={:.2}", physics_report.dynamic_prop_y),
        ),
        CapabilityCheck::new(
            "gameplay.combo",
            combat_report.chains >= 1 && combat_report.attacks_started >= 2,
            format!(
                "started={} queued={} chains={}",
                combat_report.attacks_started, combat_report.inputs_queued, combat_report.chains
            ),
        ),
        CapabilityCheck::new(
            "gameplay.damage",
            combat_report.damage_total > 0.0 && combat_report.enemy_health < 100.0,
            format!(
                "damage={:.1} enemy_health={:.1}",
                combat_report.damage_total, combat_report.enemy_health
            ),
        ),
        CapabilityCheck::new(
            "gameplay.lock_on",
            physics_report.lock_on_target.is_some(),
            format!("lock_on_target={:?}", physics_report.lock_on_target),
        ),
        CapabilityCheck::new(
            "animation.state_machine",
            animation_report.hit_frame_events > 0 && animation_report.states_seen.len() >= 2,
            format!(
                "states={:?} hit_frame_events={}",
                animation_report.states_seen, animation_report.hit_frame_events
            ),
        ),
        CapabilityCheck::new(
            "animation.action_adapter",
            animation_report.action_started_events > 0,
            format!(
                "action_started_events={} current_state={}",
                animation_report.action_started_events, animation_report.current_state
            ),
        ),
        CapabilityCheck::new(
            "asset.database",
            asset.asset_count >= 3,
            format!(
                "asset_count={} project={}",
                asset.asset_count, asset.project_root
            ),
        ),
        CapabilityCheck::new(
            "asset.cook_manifest",
            asset.cooked_entries >= 3 && asset.hash_verified && asset.copied_files >= 3,
            format!(
                "entries={} copied={} hash_verified={}",
                asset.cooked_entries, asset.copied_files, asset.hash_verified
            ),
        ),
        CapabilityCheck::new(
            "audio.null_backend",
            audio_report.silent
                && audio_report.sounds_loaded >= 2
                && audio_report.voices_played >= 1,
            format!(
                "backend={} silent={} voices={}",
                audio_report.backend, audio_report.silent, audio_report.voices_played
            ),
        ),
        CapabilityCheck::new(
            "player.dry_run",
            player.manifest_valid && player.planned_files > 0 && player.targets.len() == 2,
            format!(
                "planned_files={} targets={:?}",
                player.planned_files, player.targets
            ),
        ),
    ];
    let failures: Vec<String> = capabilities
        .iter()
        .filter(|check| !check.passed())
        .map(|check| format!("{}: {}", check.name, check.detail))
        .collect();
    let passed = failures.is_empty();

    let report = DemoReport {
        format_version: REPORT_FORMAT_VERSION,
        demo: "hezhou-capability-demo".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at_utc: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        fixed_hz: FIXED_HZ,
        steps,
        simulated_seconds: steps as f32 * fixed_dt,
        capabilities,
        physics: physics_report,
        animation: animation_report,
        combat: combat_report,
        asset,
        audio: audio_report,
        player,
        passed,
        failures,
        report_path: options.report_path.display().to_string(),
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("序列化报告失败: {error}"))?;
    fs::write(&options.report_path, format!("{json}\n"))
        .map_err(|error| format!("写报告 {} 失败: {error}", options.report_path.display()))?;
    Ok(report)
}

// ===========================================================================
// 各能力辅助
// ===========================================================================

fn light_hitbox() -> Hitbox {
    Hitbox::new(HitboxShape::sphere(0.8)).with_offset(Vec3::new(0.0, 1.05, 1.25))
}

fn heavy_hitbox() -> Hitbox {
    Hitbox::new(HitboxShape::sphere(0.9)).with_offset(Vec3::new(0.0, 1.1, 1.35))
}

fn translation_clip(name: &str, duration: f32) -> AnimationClip {
    let mut clip = AnimationClip::new(name, duration);
    let mut channel = Channel::translation(0);
    channel
        .add_key(Keyframe::translation(0.0, AnimVec3::ZERO))
        .expect("关键帧合法");
    channel
        .add_key(Keyframe::translation(
            duration,
            AnimVec3::new(0.0, 0.1, 0.0),
        ))
        .expect("关键帧合法");
    clip.add_channel(channel);
    clip
}

fn clip_with_event(name: &str, duration: f32, hit_time: f32) -> AnimationClip {
    let mut clip = translation_clip(name, duration);
    clip.add_event(ClipEvent::new(hit_time, "attack_hit"));
    clip
}

fn vec3_array(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

fn playback_state_name(state: PlaybackState) -> String {
    match state {
        PlaybackState::Stopped => "stopped".to_string(),
        PlaybackState::Playing => "playing".to_string(),
        PlaybackState::Paused => "paused".to_string(),
    }
}

// ---- asset ---------------------------------------------------------------

fn run_asset_capability(work_dir: &Path) -> Result<AssetReport, String> {
    let project_root = work_dir.join("fixture_project");
    let assets = project_root.join("assets");
    fs::create_dir_all(assets.join("scenes"))
        .and_then(|_| fs::create_dir_all(assets.join("audio")))
        .and_then(|_| fs::create_dir_all(assets.join("fonts")))
        .map_err(|error| format!("创建资产 fixture 目录失败: {error}"))?;
    fs::write(
        assets.join("scenes").join("main.scene"),
        "{\n  \"format_version\": 1,\n  \"entities\": [],\n  \"note\": \"capability demo fixture\"\n}\n",
    )
    .map_err(|error| format!("写场景 fixture 失败: {error}"))?;
    fs::write(
        assets.join("audio").join("hit.ogg"),
        b"OggS\x00capability-demo-placeholder",
    )
    .map_err(|error| format!("写音频 fixture 失败: {error}"))?;
    fs::write(
        assets.join("fonts").join("HarmonyOS_Sans_SC.ttf"),
        b"capability-demo-font-placeholder",
    )
    .map_err(|error| format!("写字体 fixture 失败: {error}"))?;

    let mut db =
        AssetDatabase::open(&project_root).map_err(|error| format!("打开资产工程失败: {error}"))?;
    let scene_path = LogicalPath::new("assets/scenes/main.scene")
        .map_err(|error| format!("逻辑路径: {error}"))?;
    let audio_path =
        LogicalPath::new("assets/audio/hit.ogg").map_err(|error| format!("逻辑路径: {error}"))?;
    let font_path = LogicalPath::new("assets/fonts/HarmonyOS_Sans_SC.ttf")
        .map_err(|error| format!("逻辑路径: {error}"))?;

    let scene_id = db
        .id_of(&scene_path)
        .ok_or_else(|| "场景资产未注册".to_string())?;
    let audio_id = db
        .id_of(&audio_path)
        .ok_or_else(|| "音频资产未注册".to_string())?;
    let font_id = db
        .id_of(&font_path)
        .ok_or_else(|| "字体资产未注册".to_string())?;
    db.set_dependencies(scene_id, [audio_id, font_id])
        .map_err(|error| format!("设置依赖失败: {error}"))?;

    let manifest = Cooker::new(&db)
        .plan(&[scene_id])
        .map_err(|error| format!("生成 cook 计划失败: {error}"))?;
    let cook_root = work_dir.join("cooked");
    fs::create_dir_all(&cook_root).map_err(|error| format!("创建 cook 目录失败: {error}"))?;
    let manifest_path = cook_root.join("cook_manifest.json");
    manifest
        .save(&manifest_path)
        .map_err(|error| format!("保存 cook manifest 失败: {error}"))?;
    let loaded = AssetCookManifest::load(&manifest_path)
        .map_err(|error| format!("读取 cook manifest 失败: {error}"))?;
    let output = Cooker::new(&db)
        .cook_to_dir(&loaded, &cook_root)
        .map_err(|error| format!("执行 cook 失败: {error}"))?;

    // 校验资产读取：文件存在、大小与哈希与 manifest 一致。
    let mut hash_verified = false;
    for entry in &loaded.entries {
        let path = cook_root.join(entry.output.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Ok(bytes) = fs::read(&path) {
            if fnv1a64_hex(&bytes) == entry.source_hash && bytes.len() as u64 == entry.source_size {
                hash_verified = true;
            }
        }
    }

    Ok(AssetReport {
        project_root: project_root.display().to_string(),
        asset_count: db.len(),
        cooked_entries: loaded.entries.len(),
        manifest_path: manifest_path.display().to_string(),
        cook_output_dir: cook_root.display().to_string(),
        copied_files: output.copied_files,
        hash_verified,
    })
}

// ---- player --------------------------------------------------------------

fn run_player_capability(work_dir: &Path) -> Result<PlayerReport, String> {
    let config = PlayerBuildConfig::new(
        "Hezhou Capability Demo",
        "0.2.0",
        "scenes/main.scene",
        TargetPlatform::Windows,
    )
    .with_executable_name("capability-demo")
    .with_window(WindowConfig::new("Hezhou Capability Demo").with_size(1280, 720))
    .with_quality(QualityTier::High);
    config
        .validate()
        .map_err(|errors| format!("Player 配置校验失败: {}", errors.summary()))?;

    let cook = hezhou_player::builder::CookManifest::new(vec![
        ManifestAsset::new("scenes/main.scene", "scene").with_size(96),
        ManifestAsset::new("audio/hit.ogg", "audio").with_size(32),
        ManifestAsset::new("fonts/HarmonyOS_Sans_SC.ttf", "font").with_size(32),
    ]);
    let mut input = PlayerBuildInput::new(config, cook);
    input.scripts = vec![ScriptAssembly::new("Game.dll")];
    let builder = PlayerBuilder::new(input);
    let plans = builder
        .plan_both_targets()
        .map_err(|error| format!("Player dry-run 计划失败: {error:?}"))?;

    let windows_plan = plans
        .first()
        .ok_or_else(|| "Player 双平台计划为空".to_string())?;
    windows_plan
        .manifest
        .validate()
        .map_err(|error| format!("Player manifest 校验失败: {error:?}"))?;

    let player_dir = work_dir.join("player");
    fs::create_dir_all(&player_dir).map_err(|error| format!("创建 Player 目录失败: {error}"))?;
    let manifest_path = player_dir.join("manifest.json");
    windows_plan
        .manifest
        .save(&manifest_path)
        .map_err(|error| format!("保存 Player manifest 失败: {error:?}"))?;
    let reloaded = PlayerManifest::load(&manifest_path)
        .map_err(|error| format!("读取 Player manifest 失败: {error:?}"))?;
    reloaded
        .validate()
        .map_err(|error| format!("Player manifest 回读校验失败: {error:?}"))?;

    let planned_executable = windows_plan
        .files_in(ArtifactCategory::Executable)
        .first()
        .map(|file| file.relative_path.clone())
        .unwrap_or_default();

    Ok(PlayerReport {
        manifest_valid: true,
        planned_files: plans.iter().map(|plan| plan.files.len()).sum(),
        planned_executable,
        warnings: plans.iter().map(|plan| plan.warnings.len()).sum(),
        fingerprint: windows_plan.fingerprint.clone(),
        targets: plans
            .iter()
            .map(|plan| plan.target.as_str().to_string())
            .collect(),
        manifest_path: manifest_path.display().to_string(),
        start_scene: reloaded.start_scene.clone(),
    })
}

// ===========================================================================
// 摘要输出
// ===========================================================================

/// 打印人类可读摘要。
pub fn print_summary(report: &DemoReport) {
    println!("--- capability demo summary ---");
    println!(
        "steps={} fixed_hz={} simulated={:.2}s engine={}",
        report.steps, report.fixed_hz, report.simulated_seconds, report.engine_version
    );
    for check in &report.capabilities {
        println!("[{:4}] {:<26} {}", check.status, check.name, check.detail);
    }
    println!(
        "combat: hits={} damage={:.1} enemy_health={:.1} chains={}",
        report.combat.hits,
        report.combat.damage_total,
        report.combat.enemy_health,
        report.combat.chains
    );
    println!(
        "physics: grounded={} camera_collided={} final_position={:?} prop_y={:.2}",
        report.physics.grounded,
        report.physics.camera_collided,
        report.physics.final_position,
        report.physics.dynamic_prop_y
    );
    println!(
        "animation: current={} states={:?} hit_frames={}",
        report.animation.current_state,
        report.animation.states_seen,
        report.animation.hit_frame_events
    );
    println!(
        "asset: entries={} copied={} hash_verified={} manifest={}",
        report.asset.cooked_entries,
        report.asset.copied_files,
        report.asset.hash_verified,
        report.asset.manifest_path
    );
    println!(
        "audio: backend={} silent={} voices={} final_state={}",
        report.audio.backend,
        report.audio.silent,
        report.audio.voices_played,
        report.audio.final_state
    );
    println!(
        "player: manifest_valid={} planned_files={} targets={:?} fingerprint={}",
        report.player.manifest_valid,
        report.player.planned_files,
        report.player.targets,
        report.player.fingerprint
    );
    println!("report: {}", report.report_path);
}

// ===========================================================================
// 测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_options(name: &str) -> DemoOptions {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join("capability-demo")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        DemoOptions {
            report_path: dir.join("report.json"),
            steps: 240,
        }
    }

    #[test]
    fn capability_demo_runs_headless_and_passes() {
        let options = test_options("pass");
        let report = run_demo(&options).expect("demo 应成功运行");

        assert!(report.passed, "失败能力项: {:?}", report.failures);
        assert!(report.capabilities.iter().all(CapabilityCheck::passed));
        assert_eq!(report.fixed_hz, FIXED_HZ);
        assert_eq!(report.steps, options.steps);
        assert!((report.simulated_seconds - options.steps as f32 / 60.0).abs() < 1e-4);

        // 物理/战斗闭环
        assert!(report.physics.grounded, "角色应落在地面");
        assert!(report.physics.camera_collided, "相机应被墙遮挡");
        assert!(report.physics.lock_on_target.is_some(), "应锁定敌人");
        assert!(report.combat.hits >= 2, "至少两段攻击命中: {report:?}");
        assert!(report.combat.damage_total > 0.0);
        assert!(report.combat.chains >= 1, "应发生连招衔接");

        // 动画状态机/事件
        assert!(!report.animation.states_seen.is_empty());
        assert!(report.animation.hit_frame_events > 0, "动画命中帧应透传");

        // 资产/音频/Player
        assert!(report.asset.hash_verified);
        assert!(report.audio.silent);
        assert!(report.player.manifest_valid);
        assert_eq!(report.player.targets, vec!["windows", "linux"]);

        // 报告文件可读且字段一致
        let text = fs::read_to_string(&options.report_path).expect("报告应写出");
        let parsed: DemoReport = serde_json::from_str(&text).expect("报告应为合法 JSON");
        assert_eq!(parsed.passed, report.passed);
        assert_eq!(parsed.capabilities, report.capabilities);
        assert_eq!(parsed.combat, report.combat);
        assert_eq!(parsed.animation, report.animation);
        assert_eq!(parsed.physics.final_position, report.physics.final_position);
        assert_eq!(parsed.report_path, report.report_path);
    }

    #[test]
    fn capability_demo_is_deterministic_for_same_asset_fixture() {
        let first = run_demo(&test_options("deterministic-a")).expect("第一次运行");
        let second = run_demo(&test_options("deterministic-b")).expect("第二次运行");

        assert_eq!(first.combat, second.combat);
        assert_eq!(first.animation, second.animation);
        assert_eq!(first.physics, second.physics);
        // 能力状态序列应一致（detail 里含各自的 fixture 绝对路径，不参与比较）。
        let statuses = |report: &DemoReport| {
            report
                .capabilities
                .iter()
                .map(|check| check.status.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(statuses(&first), statuses(&second));
        assert_eq!(
            first.asset.cooked_entries, second.asset.cooked_entries,
            "同一 fixture 的 cook 条目数量应一致"
        );
        assert_eq!(first.player.fingerprint, second.player.fingerprint);
        assert_eq!(first.passed, second.passed);
    }
}
