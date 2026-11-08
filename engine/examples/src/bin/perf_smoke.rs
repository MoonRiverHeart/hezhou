//! `perf_smoke` — Hezhou 0.3 headless 性能基线 / 门禁。
//!
//! 在**无窗口、无 GPU、无声卡、无编辑器**的环境下，对 0.2/0.3 引擎核心路径做
//! 可复现的 smoke 基准，并输出 JSON 报告供 CI 门禁与本地基线对比：
//!
//! | 指标 | 工作量 |
//! |---|---|
//! | `ecs_10k_entities_frame` | 10_000 实体 × Transform/组件更新（World + System） |
//! | `physics_200_bodies_60_steps` | 200 动态盒 + 地面，60 个固定步（含宽相统计） |
//! | `animation_10k_updates` | AnimationStateMachine update+sample 10_000 次（16 关节） |
//! | `audio_100_voices_1000_frames` | NullBackend、100 循环 voice、混音 1000 帧 |
//! | `asset_1000_entries` | CookManifest 1000 条目的 plan/序列化/反序列化 |
//!
//! 每个指标先预热再跑多轮，取**中位数**（降低首次运行/系统噪声影响），
//! 与内置阈值比较；任一超阈值则退出码非 0。
//!
//! ```text
//! cd engine
//! cargo run --release -p hezhou-examples --bin perf_smoke -- --json
//! cargo run -p hezhou-examples --bin perf_smoke -- --profile local --json target/perf.json
//! ```
//!
//! 报告默认写到仓库根目录 `test-results/perf-baseline/report.json`；
//! 基线说明见 `docs/PERF_BASELINE_0_3.md`，CI 由 `perf-baseline` job 运行
//! `engine/scripts/perf_baseline.ps1`。
//!
//! 注意：本程序只调用能力 crate 的公开 API，**不修改**它们；数值阈值按
//! “同一台机器、相同 release 配置”设计，跨硬件比较请先在本机重建基线。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use hezhou_animation::{
    AnimationClip, AnimationState, AnimationStateMachine, Channel, Keyframe, Quat, Skeleton,
    Transform, Vec3 as AnimVec3,
};
use hezhou_asset::{
    cook::COOK_MANIFEST_FORMAT_VERSION, AssetId, AssetKind, AssetLocator, CookEntry, CookManifest,
    LogicalPath,
};
use hezhou_audio::{AudioEngine, EngineConfig, NullBackend, PlayOptions, SoundData};
use hezhou_core::ecs::{System, TransformComponent, World};
use hezhou_core::math::Vec3;
use hezhou_physics::{ColliderShape, PhysicsWorld, RigidBodyType};

/// 报告格式版本。
const REPORT_FORMAT_VERSION: u32 = 1;
/// 固定仿真步长（与引擎 60Hz 约定一致）。
const DT: f32 = 1.0 / 60.0;
/// 默认报告路径（仓库根目录 `test-results/perf-baseline/report.json`）。
const DEFAULT_REPORT_DIR: &str = "test-results/perf-baseline";

// ===========================================================================
// 报告数据结构
// ===========================================================================

/// 单个基准的结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkResult {
    /// 指标名（稳定标识，用于基线与 CI 对比）。
    pub name: String,
    /// 工作量说明（一次性跑多少东西）。
    pub workload: String,
    /// 工作量内的迭代次数（帧/步/条目数）。
    pub iterations: u32,
    /// 测量轮数（中位数样本数）。
    pub rounds: u32,
    /// 每轮耗时中位数（毫秒）。
    pub median_ms: f64,
    /// 每轮最小/最大耗时（毫秒）。
    pub min_ms: f64,
    /// 每轮最大耗时（毫秒）。
    pub max_ms: f64,
    /// 折算到单次迭代的中位耗时（毫秒）。
    pub per_iteration_ms: f64,
    /// 阈值（毫秒，作用于 `median_ms`）。
    pub threshold_ms: f64,
    /// 是否通过阈值。
    pub passed: bool,
    /// 指标相关的额外信息（候选对、voice 数、条目数等）。
    pub detail: Value,
}

impl BenchmarkResult {
    /// 是否通过。
    pub fn passed(&self) -> bool {
        self.passed
    }
}

/// 机器/构建信息（用于解释跨机器差异）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MachineInfo {
    /// 操作系统（`std::env::consts::OS`）。
    pub os: String,
    /// CPU 架构（`std::env::consts::ARCH`）。
    pub arch: String,
    /// 可用逻辑核数。
    pub cpu_cores: usize,
    /// 构建 profile：`debug` / `release`（`cfg!(debug_assertions)`）。
    pub build_profile: String,
    /// 是否在 CI 中运行（环境变量 `CI`）。
    pub ci: bool,
    /// 可选的 git commit（CI 的 `GITHUB_SHA` 或脚本注入的 `GIT_COMMIT`）。
    pub git_commit: Option<String>,
}

/// 完整报告。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerfReport {
    /// 报告格式版本。
    pub format_version: u32,
    /// 工具名。
    pub tool: String,
    /// 引擎版本（examples crate 版本）。
    pub engine_version: String,
    /// UTC 生成时间（RFC3339）。
    pub generated_at_utc: String,
    /// 机器/构建信息。
    pub machine: MachineInfo,
    /// 门禁 profile：`ci` / `local`。
    pub gate_profile: String,
    /// 用户指定的阈值缩放倍率（1.0 = 使用内置阈值）。
    pub threshold_scale: f64,
    /// 各指标结果。
    pub benchmarks: Vec<BenchmarkResult>,
    /// 是否全部通过。
    pub passed: bool,
    /// 失败项说明。
    pub failures: Vec<String>,
    /// 报告文件路径。
    pub report_path: String,
}

// ===========================================================================
// 阈值
// ===========================================================================

/// 门禁档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateProfile {
    /// CI 档：release 构建、按本机 release 实测留足余量。
    Ci,
    /// 本地档：debug 构建也可跑通，阈值更宽松。
    Local,
}

impl GateProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ci => "ci",
            Self::Local => "local",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "ci" | "release" => Some(Self::Ci),
            "local" | "debug" => Some(Self::Local),
            _ => None,
        }
    }
}

/// 每个指标一轮工作量的阈值（毫秒）。
///
/// 取值策略见 `docs/PERF_BASELINE_0_3.md`：以“本机 release 实测 × 3~4”为 CI 档，
/// 给共享 CI runner 的噪声留余量；local 档再放宽，覆盖 debug 构建。
#[derive(Debug, Clone, Copy)]
pub struct BenchLimits {
    /// 10k 实体、10 帧 ECS 更新。
    pub ecs_10k_10_frames_ms: f64,
    /// 200 动态体、60 步物理。
    pub physics_200_bodies_60_steps_ms: f64,
    /// 10k 次动画 update+sample。
    pub animation_10k_updates_ms: f64,
    /// 100 voice、1000 帧混音。
    pub audio_100_voices_1000_frames_ms: f64,
    /// 1000 条 CookManifest 序列化+反序列化。
    pub asset_1000_entries_ms: f64,
}

impl BenchLimits {
    fn for_profile(profile: GateProfile) -> Self {
        match profile {
            // CI（release）阈值：本机 release 实测（Windows x64 / 12 核）约
            // ECS 11.5ms、物理 56ms、动画 10.8ms、音频 1.8ms、资产 1.3ms；
            // 共享 runner 通常慢 2~3x，这里留 5~10x 余量，作为"明显回归"闸门。
            GateProfile::Ci => Self {
                ecs_10k_10_frames_ms: 150.0,
                physics_200_bodies_60_steps_ms: 600.0,
                animation_10k_updates_ms: 200.0,
                audio_100_voices_1000_frames_ms: 80.0,
                asset_1000_entries_ms: 80.0,
            },
            // Local（debug）阈值：本机 debug 实测约 ECS 101ms、物理 532ms、
            // 动画 33ms、音频 15ms、资产 14ms；留 ~10x 余量覆盖较慢开发机。
            GateProfile::Local => Self {
                ecs_10k_10_frames_ms: 1_200.0,
                physics_200_bodies_60_steps_ms: 6_000.0,
                animation_10k_updates_ms: 500.0,
                audio_100_voices_1000_frames_ms: 300.0,
                asset_1000_entries_ms: 300.0,
            },
        }
    }

    fn scaled(self, scale: f64) -> Self {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        Self {
            ecs_10k_10_frames_ms: self.ecs_10k_10_frames_ms * scale,
            physics_200_bodies_60_steps_ms: self.physics_200_bodies_60_steps_ms * scale,
            animation_10k_updates_ms: self.animation_10k_updates_ms * scale,
            audio_100_voices_1000_frames_ms: self.audio_100_voices_1000_frames_ms * scale,
            asset_1000_entries_ms: self.asset_1000_entries_ms * scale,
        }
    }
}

// ===========================================================================
// 命令行参数
// ===========================================================================

/// 运行参数。
#[derive(Debug, Clone)]
pub struct PerfOptions {
    /// JSON 输出路径。
    pub json_path: PathBuf,
    /// 门禁档位。
    pub gate_profile: GateProfile,
    /// 阈值缩放倍率。
    pub threshold_scale: f64,
    /// 快速模式（单测/冒烟用，显著缩减工作量）。
    pub quick: bool,
    /// 覆盖测量轮数（`None` 用默认）。
    pub rounds_override: Option<u32>,
}

impl PerfOptions {
    /// 默认报告路径（仓库根目录）。
    pub fn default_json_path() -> PathBuf {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or(manifest_dir);
        repo_root.join(DEFAULT_REPORT_DIR).join("report.json")
    }

    /// 从命令行参数解析。
    pub fn from_args<I: IntoIterator<Item = String>>(args: I) -> Self {
        let mut options = Self {
            json_path: Self::default_json_path(),
            gate_profile: GateProfile::Ci,
            threshold_scale: 1.0,
            quick: false,
            rounds_override: None,
        };
        let mut iter = args.into_iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                // `--json` 可带可选路径；不带则用默认路径。
                "--json" => {
                    if let Some(next) = iter.peek() {
                        if !next.starts_with('-') {
                            options.json_path = PathBuf::from(iter.next().unwrap());
                        }
                    }
                }
                "--profile" => {
                    if let Some(value) = iter.next() {
                        match GateProfile::parse(&value) {
                            Some(profile) => options.gate_profile = profile,
                            None => eprintln!("未知 profile {value:?}，使用 ci"),
                        }
                    }
                }
                "--threshold-scale" => {
                    if let Some(value) = iter.next() {
                        if let Ok(scale) = value.parse::<f64>() {
                            options.threshold_scale = scale;
                        }
                    }
                }
                "--rounds" => {
                    if let Some(value) = iter.next() {
                        if let Ok(rounds) = value.parse::<u32>() {
                            options.rounds_override = Some(rounds.max(1));
                        }
                    }
                }
                "--quick" => options.quick = true,
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("忽略未知参数: {arg}");
                }
            }
        }
        options
    }
}

fn print_help() {
    println!("Hezhou 0.3 headless 性能基线 / 门禁");
    println!();
    println!("用法: perf_smoke [--json [path]] [--profile ci|local] [--threshold-scale <f64>]");
    println!("                 [--rounds <n>] [--quick]");
    println!();
    println!("默认报告: {}", PerfOptions::default_json_path().display());
    println!("退出码: 全部通过 0，超阈值 1，运行错误 2。");
}

// ===========================================================================
// 工作量配置
// ===========================================================================

/// 基准工作量配置（生产与单测共用）。
#[derive(Debug, Clone, Copy)]
pub struct BenchScale {
    pub ecs_entities: usize,
    pub ecs_frames: u32,
    pub physics_bodies: usize,
    pub physics_steps: u32,
    pub animation_iterations: u32,
    pub animation_joints: usize,
    pub audio_voices: usize,
    pub audio_frames: u32,
    pub asset_entries: usize,
    pub rounds: u32,
    pub warmup_rounds: u32,
}

impl BenchScale {
    /// 生产基线工作量（与 docs/CI 阈值对应）。
    pub fn full() -> Self {
        Self {
            ecs_entities: 10_000,
            ecs_frames: 10,
            physics_bodies: 200,
            physics_steps: 60,
            animation_iterations: 10_000,
            animation_joints: 16,
            audio_voices: 100,
            audio_frames: 1_000,
            asset_entries: 1_000,
            rounds: 5,
            warmup_rounds: 2,
        }
    }

    /// 快速工作量（单测/本地冒烟；不作为 CI 基线）。
    pub fn quick() -> Self {
        Self {
            ecs_entities: 200,
            ecs_frames: 3,
            physics_bodies: 20,
            physics_steps: 5,
            animation_iterations: 500,
            animation_joints: 8,
            audio_voices: 10,
            audio_frames: 100,
            asset_entries: 50,
            rounds: 2,
            warmup_rounds: 1,
        }
    }

    fn with_rounds(mut self, rounds: Option<u32>) -> Self {
        if let Some(rounds) = rounds {
            self.rounds = rounds.max(1);
        }
        self
    }
}

// ===========================================================================
// 测量工具
// ===========================================================================

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000.0
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

/// 按轮测量：每轮前可执行一次不计时的准备，然后计时执行 `run`。
fn measure_rounds<F>(rounds: u32, mut run: F) -> Vec<f64>
where
    F: FnMut(),
{
    let mut samples = Vec::with_capacity(rounds as usize);
    for _ in 0..rounds.max(1) {
        let start = Instant::now();
        run();
        samples.push(elapsed_ms(start));
    }
    samples
}

fn aggregate(
    name: &str,
    workload: impl Into<String>,
    iterations: u32,
    rounds: u32,
    samples: Vec<f64>,
    threshold_ms: f64,
    detail: Value,
) -> BenchmarkResult {
    let mut samples = samples;
    let median_ms = median(&mut samples);
    let min_ms = samples.first().copied().unwrap_or(0.0);
    let max_ms = samples.last().copied().unwrap_or(0.0);
    let per_iteration_ms = if iterations > 0 {
        median_ms / f64::from(iterations)
    } else {
        median_ms
    };
    BenchmarkResult {
        name: name.to_string(),
        workload: workload.into(),
        iterations,
        rounds,
        median_ms,
        min_ms,
        max_ms,
        per_iteration_ms,
        threshold_ms,
        passed: median_ms <= threshold_ms,
        detail,
    }
}

// ===========================================================================
// 基准 1：ECS 10k 实体组件更新
// ===========================================================================

/// 演示用的速度组件（perf_smoke 内部组件，不进入核心 crate）。
#[derive(Clone, Debug)]
struct PerfVelocity {
    velocity: Vec3,
}

hezhou_core::define_component!(PerfVelocity, 0x9001);

/// 每帧遍历所有实体，读速度组件、更新 Transform 组件。
struct EcsUpdateSystem;

impl System for EcsUpdateSystem {
    fn name(&self) -> &'static str {
        "perf_ecs_update"
    }

    fn update(&mut self, world: &mut World, delta_time: f32) {
        for entity in world.all_entities() {
            let velocity = world
                .get_component::<PerfVelocity>(entity)
                .map(|component| component.velocity)
                .unwrap_or_else(Vec3::zero);
            if let Some(mut transform) = world.get_component::<TransformComponent>(entity) {
                transform.position.x += velocity.x * delta_time;
                transform.position.y += velocity.y * delta_time;
                transform.position.z += velocity.z * delta_time;
                world.add_component(entity, transform);
            }
        }
    }
}

fn build_ecs_world(entities: usize) -> World {
    let mut world = World::new();
    for index in 0..entities {
        let entity = world.create_entity();
        let position = Vec3::new((index % 100) as f32, (index / 100) as f32, 0.0);
        world.add_component(
            entity,
            TransformComponent {
                position,
                ..TransformComponent::default()
            },
        );
        world.add_component(
            entity,
            PerfVelocity {
                velocity: Vec3::new(1.0, 0.0, 0.5),
            },
        );
    }
    world.add_system(Box::new(EcsUpdateSystem));
    world
}

fn bench_ecs(scale: &BenchScale, limits: &BenchLimits) -> BenchmarkResult {
    let mut world = build_ecs_world(scale.ecs_entities);
    for _ in 0..scale.warmup_rounds {
        for _ in 0..scale.ecs_frames {
            world.update(DT);
        }
    }
    let samples = measure_rounds(scale.rounds, || {
        for _ in 0..scale.ecs_frames {
            world.update(DT);
        }
    });
    aggregate(
        "ecs_10k_entities_frame",
        format!(
            "{} entities x {} frames (World + System)",
            scale.ecs_entities, scale.ecs_frames
        ),
        scale.ecs_frames,
        scale.rounds,
        samples,
        limits.ecs_10k_10_frames_ms,
        json!({
            "entities": scale.ecs_entities,
            "frames": scale.ecs_frames,
            "components_per_entity": 2,
        }),
    )
}

// ===========================================================================
// 基准 2：物理 200 动态体 × 60 固定步
// ===========================================================================

fn build_physics_world(bodies: usize) -> PhysicsWorld {
    let mut world = PhysicsWorld::new();
    let ground = world.create_body_with_type(RigidBodyType::Static);
    if let Some(body) = world.get_body_mut(ground) {
        body.set_position(hezhou_core::math::Vec3::new(0.0, -0.5, 0.0));
    }
    world.create_collider(
        ColliderShape::box_size(hezhou_core::math::Vec3::new(400.0, 1.0, 400.0)),
        ground,
    );

    let columns = 20usize;
    for index in 0..bodies {
        let body = world.create_body_with_type(RigidBodyType::Dynamic);
        let x = (index % columns) as f32 * 2.0;
        let z = (index / columns) as f32 * 2.0;
        if let Some(body_ref) = world.get_body_mut(body) {
            body_ref.set_position(hezhou_core::math::Vec3::new(x, 1.0, z));
        }
        world.create_collider(
            ColliderShape::box_size(hezhou_core::math::Vec3::new(1.0, 1.0, 1.0)),
            body,
        );
    }
    world
}

fn bench_physics(scale: &BenchScale, limits: &BenchLimits) -> BenchmarkResult {
    for _ in 0..scale.warmup_rounds {
        let mut world = build_physics_world(scale.physics_bodies);
        for _ in 0..scale.physics_steps {
            world.step(DT);
        }
    }
    let mut candidate_pairs = 0usize;
    let mut spatial_index = false;
    let mut body_count = 0usize;
    let samples = measure_rounds(scale.rounds, || {
        let mut world = build_physics_world(scale.physics_bodies);
        for _ in 0..scale.physics_steps {
            world.step(DT);
        }
        let stats = world.broadphase_stats();
        candidate_pairs = stats.candidate_pairs;
        spatial_index = stats.spatial_index;
        body_count = stats.body_count;
    });
    let total_ms = median(&mut samples.clone());
    aggregate(
        "physics_200_bodies_60_steps",
        format!(
            "{} dynamic boxes + ground, {} fixed steps",
            scale.physics_bodies, scale.physics_steps
        ),
        1,
        scale.rounds,
        samples,
        limits.physics_200_bodies_60_steps_ms,
        json!({
            "bodies": scale.physics_bodies,
            "steps": scale.physics_steps,
            "step_ms": total_ms / f64::from(scale.physics_steps.max(1)),
            "candidate_pairs": candidate_pairs,
            "broadphase_body_count": body_count,
            "spatial_index": spatial_index,
        }),
    )
}

// ===========================================================================
// 基准 3：动画状态机 10k update/sample
// ===========================================================================

fn build_animation_scene(joints: usize) -> (Skeleton, AnimationStateMachine) {
    let mut skeleton = Skeleton::new("perf_skeleton");
    let root = skeleton.add_root_joint("root", Transform::IDENTITY);
    let mut parent = root;
    for index in 1..joints.max(1) {
        let translation = AnimVec3::new(0.0, 0.1 * index as f32, 0.0);
        parent = skeleton
            .add_joint(
                format!("joint_{index}"),
                parent,
                Transform::from_trs(translation, Quat::IDENTITY),
            )
            .expect("合成骨架关节应合法");
    }

    let duration = 1.0f32;
    let mut clip = AnimationClip::new("PerfIdle", duration);
    for joint in 0..joints.max(1) {
        let mut channel = Channel::translation(joint);
        channel
            .add_key(Keyframe::translation(0.0, AnimVec3::ZERO))
            .expect("关键帧合法");
        channel
            .add_key(Keyframe::translation(
                duration,
                AnimVec3::new(0.0, 0.1 * joint as f32, 0.0),
            ))
            .expect("关键帧合法");
        clip.add_channel(channel);
    }

    let mut machine = AnimationStateMachine::new();
    machine.set_clip(AnimationState::Idle, clip);
    (skeleton, machine)
}

fn bench_animation(scale: &BenchScale, limits: &BenchLimits) -> BenchmarkResult {
    let (skeleton, mut machine) = build_animation_scene(scale.animation_joints);
    for _ in 0..scale.warmup_rounds {
        for _ in 0..scale.animation_iterations {
            let _ = machine.update(DT);
            std::hint::black_box(machine.sample(&skeleton));
        }
    }
    let samples = measure_rounds(scale.rounds, || {
        for _ in 0..scale.animation_iterations {
            let _ = machine.update(DT);
            std::hint::black_box(machine.sample(&skeleton));
        }
    });
    aggregate(
        "animation_10k_updates",
        format!(
            "{} x (state machine update + sample), {} joints",
            scale.animation_iterations, scale.animation_joints
        ),
        scale.animation_iterations,
        scale.rounds,
        samples,
        limits.animation_10k_updates_ms,
        json!({
            "iterations": scale.animation_iterations,
            "joints": scale.animation_joints,
            "current_state": machine.current_state().name(),
        }),
    )
}

// ===========================================================================
// 基准 4：音频 100 voice × 1000 帧混音
// ===========================================================================

fn bench_audio(scale: &BenchScale, limits: &BenchLimits) -> BenchmarkResult {
    let sample_rate = 48_000u32;
    let engine = AudioEngine::with_config(
        NullBackend::boxed(),
        EngineConfig {
            sample_rate,
            max_voices: scale.audio_voices + 8,
        },
    );
    engine.load_sound(
        "perf_tone",
        SoundData::sine("perf_tone", 440.0, 0.05, sample_rate).expect("合成正弦音效合法"),
    );
    let mut voices = 0usize;
    for _ in 0..scale.audio_voices {
        if engine
            .play_with(PlayOptions::new("perf_tone").looping(true).volume(0.5))
            .is_some()
        {
            voices += 1;
        }
    }
    let frame_dt = 1.0 / f64::from(sample_rate) as f32;
    for _ in 0..scale.warmup_rounds {
        for _ in 0..scale.audio_frames {
            let _ = engine.update(frame_dt);
        }
    }
    let samples = measure_rounds(scale.rounds, || {
        for _ in 0..scale.audio_frames {
            let _ = engine.update(frame_dt);
        }
    });
    aggregate(
        "audio_100_voices_1000_frames",
        format!(
            "{} looping voices, {} mixed frames @ {} Hz (null backend)",
            scale.audio_voices, scale.audio_frames, sample_rate
        ),
        scale.audio_frames,
        scale.rounds,
        samples,
        limits.audio_100_voices_1000_frames_ms,
        json!({
            "voices_requested": scale.audio_voices,
            "voices_active": voices,
            "frames": scale.audio_frames,
            "sample_rate": sample_rate,
            "backend": engine.status().backend(),
            "silent": engine.status().is_silent(),
        }),
    )
}

// ===========================================================================
// 基准 5：资产 CookManifest 1000 条目
// ===========================================================================

fn build_cook_entries(entries: usize) -> Vec<CookEntry> {
    let mut result = Vec::with_capacity(entries);
    for index in 0..entries {
        let path = LogicalPath::new(format!("assets/perf/asset_{index:05}.bin"))
            .expect("合成逻辑路径合法");
        let id = AssetId::new();
        result.push(CookEntry {
            id,
            kind: AssetKind::Binary,
            locator: AssetLocator::file(path),
            output: format!("assets/{}.bin", id.simple()),
            dependencies: Vec::new(),
            source_size: 64 + index as u64,
            source_hash: format!("fnv1a64:{:016x}", index as u64),
        });
    }
    result
}

fn bench_asset(scale: &BenchScale, limits: &BenchLimits) -> BenchmarkResult {
    let templates = build_cook_entries(scale.asset_entries);
    let roots: Vec<AssetId> = templates.iter().map(|entry| entry.id).collect();
    let build_manifest = || CookManifest {
        format_version: COOK_MANIFEST_FORMAT_VERSION,
        roots: roots.clone(),
        entries: templates.clone(),
    };
    // 预校验一次，避免把 set-up 错误算进计时。
    let json_bytes = {
        let manifest = build_manifest();
        let text = manifest.to_json().expect("CookManifest 序列化应成功");
        let parsed: CookManifest =
            serde_json::from_str(&text).expect("CookManifest 反序列化应成功");
        assert_eq!(parsed.entries.len(), scale.asset_entries);
        text.len()
    };
    for _ in 0..scale.warmup_rounds {
        let manifest = build_manifest();
        let text = manifest.to_json().expect("CookManifest 序列化应成功");
        std::hint::black_box(serde_json::from_str::<CookManifest>(&text).ok());
    }
    let samples = measure_rounds(scale.rounds, || {
        let manifest = build_manifest();
        let text = manifest.to_json().expect("CookManifest 序列化应成功");
        let parsed: CookManifest =
            serde_json::from_str(&text).expect("CookManifest 反序列化应成功");
        std::hint::black_box(parsed);
    });
    aggregate(
        "asset_1000_entries",
        format!(
            "{} CookManifest entries: build + serialize + parse",
            scale.asset_entries
        ),
        scale.asset_entries as u32,
        scale.rounds,
        samples,
        limits.asset_1000_entries_ms,
        json!({
            "entries": scale.asset_entries,
            "json_bytes": json_bytes,
        }),
    )
}

// ===========================================================================
// 主流程
// ===========================================================================

fn machine_info() -> MachineInfo {
    MachineInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cpu_cores: std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1),
        build_profile: if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        },
        ci: std::env::var("CI").is_ok(),
        git_commit: std::env::var("GITHUB_SHA")
            .ok()
            .or_else(|| std::env::var("GIT_COMMIT").ok())
            .filter(|value| !value.is_empty()),
    }
}

/// 运行全部基准并写出 JSON 报告；测试复用该入口。
pub fn run_perf(options: &PerfOptions) -> Result<PerfReport, String> {
    let scale = if options.quick {
        BenchScale::quick()
    } else {
        BenchScale::full()
    }
    .with_rounds(options.rounds_override);
    let limits = BenchLimits::for_profile(options.gate_profile).scaled(options.threshold_scale);

    let benchmarks = vec![
        bench_ecs(&scale, &limits),
        bench_physics(&scale, &limits),
        bench_animation(&scale, &limits),
        bench_audio(&scale, &limits),
        bench_asset(&scale, &limits),
    ];

    let failures: Vec<String> = benchmarks
        .iter()
        .filter(|benchmark| !benchmark.passed())
        .map(|benchmark| {
            format!(
                "{}: median {:.2} ms > threshold {:.2} ms",
                benchmark.name, benchmark.median_ms, benchmark.threshold_ms
            )
        })
        .collect();
    let passed = failures.is_empty();

    if let Some(parent) = options.json_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建报告目录 {} 失败: {error}", parent.display()))?;
    }

    let report = PerfReport {
        format_version: REPORT_FORMAT_VERSION,
        tool: "hezhou-perf-smoke".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at_utc: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        machine: machine_info(),
        gate_profile: options.gate_profile.as_str().to_string(),
        threshold_scale: options.threshold_scale,
        benchmarks,
        passed,
        failures,
        report_path: options.json_path.display().to_string(),
    };
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("序列化性能报告失败: {error}"))?;
    fs::write(&options.json_path, format!("{json}\n"))
        .map_err(|error| format!("写性能报告 {} 失败: {error}", options.json_path.display()))?;
    Ok(report)
}

/// 打印人类可读摘要。
pub fn print_summary(report: &PerfReport) {
    println!("--- hezhou perf smoke ---");
    println!(
        "build={} gate_profile={} os={} arch={} cores={} ci={}",
        report.machine.build_profile,
        report.gate_profile,
        report.machine.os,
        report.machine.arch,
        report.machine.cpu_cores,
        report.machine.ci
    );
    for benchmark in &report.benchmarks {
        println!(
            "[{:4}] {:<32} median {:>10.3} ms (min {:>9.3} / max {:>9.3}) threshold {:>10.3} ms | {}",
            if benchmark.passed { "pass" } else { "fail" },
            benchmark.name,
            benchmark.median_ms,
            benchmark.min_ms,
            benchmark.max_ms,
            benchmark.threshold_ms,
            benchmark.workload
        );
        if !benchmark.detail.is_null() {
            println!("        detail: {}", benchmark.detail);
        }
    }
    println!("report: {}", report.report_path);
}

fn main() {
    let options = PerfOptions::from_args(std::env::args().skip(1));
    println!("Hezhou 0.3 perf smoke（headless）");
    println!("report: {}", options.json_path.display());
    let report = match run_perf(&options) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("perf smoke 运行失败: {error}");
            std::process::exit(2);
        }
    };
    print_summary(&report);
    if report.passed {
        println!("RESULT: PASS（退出码 0）");
    } else {
        eprintln!(
            "RESULT: FAIL（退出码 1），失败项: {}",
            report.failures.join("; ")
        );
        std::process::exit(1);
    }
}

// ===========================================================================
// 测试（快速模式，验证报告 schema 与门禁逻辑；不依赖 GPU/编辑器）
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_report_path(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join("perf-smoke")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("report.json")
    }

    #[test]
    fn perf_report_has_expected_schema_and_passes() {
        let options = PerfOptions {
            json_path: temp_report_path("pass"),
            gate_profile: GateProfile::Local,
            threshold_scale: 1_000.0,
            quick: true,
            rounds_override: Some(2),
        };
        let report = run_perf(&options).expect("perf smoke 应能运行");
        assert!(report.passed, "失败项: {:?}", report.failures);
        assert_eq!(report.benchmarks.len(), 5, "应有 5 个基准");
        let names: Vec<&str> = report
            .benchmarks
            .iter()
            .map(|benchmark| benchmark.name.as_str())
            .collect();
        assert!(names.contains(&"ecs_10k_entities_frame"));
        assert!(names.contains(&"physics_200_bodies_60_steps"));
        assert!(names.contains(&"animation_10k_updates"));
        assert!(names.contains(&"audio_100_voices_1000_frames"));
        assert!(names.contains(&"asset_1000_entries"));
        for benchmark in &report.benchmarks {
            assert!(benchmark.rounds >= 1);
            assert!(benchmark.median_ms >= 0.0);
            assert!(benchmark.threshold_ms > 0.0);
        }
        // 物理宽相统计必须出现在报告里。
        let physics = report
            .benchmarks
            .iter()
            .find(|benchmark| benchmark.name == "physics_200_bodies_60_steps")
            .expect("物理基准存在");
        assert!(physics.detail.get("candidate_pairs").is_some());

        // 落盘 JSON 可回读且关键字段一致（耗时字段按浮点容差比较）。
        let text = fs::read_to_string(&options.json_path).expect("报告应写出");
        let parsed: PerfReport = serde_json::from_str(&text).expect("报告应为合法 JSON");
        assert_eq!(parsed.format_version, report.format_version);
        assert_eq!(parsed.tool, report.tool);
        assert_eq!(parsed.gate_profile, report.gate_profile);
        assert_eq!(parsed.machine, report.machine);
        assert_eq!(parsed.passed, report.passed);
        assert_eq!(parsed.report_path, report.report_path);
        assert_eq!(parsed.benchmarks.len(), report.benchmarks.len());
        for (left, right) in parsed.benchmarks.iter().zip(report.benchmarks.iter()) {
            assert_eq!(left.name, right.name);
            assert_eq!(left.iterations, right.iterations);
            assert_eq!(left.rounds, right.rounds);
            assert!((left.median_ms - right.median_ms).abs() < 1e-9);
            assert!((left.threshold_ms - right.threshold_ms).abs() < 1e-9);
            assert_eq!(left.passed, right.passed);
        }
    }

    #[test]
    fn perf_gate_fails_when_threshold_is_impossible() {
        let options = PerfOptions {
            json_path: temp_report_path("fail"),
            gate_profile: GateProfile::Local,
            // 极小阈值：任何真实耗时都会超限，用于验证门禁逻辑。
            threshold_scale: 1e-9,
            quick: true,
            rounds_override: Some(1),
        };
        let report = run_perf(&options).expect("perf smoke 应能运行");
        assert!(!report.passed, "极小阈值下应判定失败");
        assert!(!report.failures.is_empty());
        assert!(report
            .benchmarks
            .iter()
            .any(|benchmark| !benchmark.passed()));
    }
}
