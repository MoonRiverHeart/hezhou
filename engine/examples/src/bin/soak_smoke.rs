//! `soak_smoke` — Hezhou 1.0 长时间稳定性（soak）测试。
//!
//! 在**无窗口、无 GPU、无声卡、无编辑器**的环境下，以固定 tick rate 驱动一组
//! 长期运行的引擎循环，采样进程 RSS 与每 tick 耗时，输出 JSON 趋势报告；
//! 内存增长/帧时间超过阈值时以非 0 退出码失败。
//!
//! 每个 tick 驱动：
//!
//! | 循环 | 内容 |
//! |---|---|
//! | ECS | 创建/销毁实体（环形 churn）+ `World::update` 更新 Transform/自定义组件 |
//! | Physics | `PhysicsWorld::step`（动态盒 + 地面），周期性销毁/重建刚体 |
//! | Animation | `AnimationStateMachine::update + sample`，周期性切换状态触发过渡 |
//! | Player-ish | `CharacterMotor::step`（通过 physics 查询扫掠/吸附）+ 跳跃/落地 |
//!
//! 内存采样**不引入第三方依赖**：
//! - Windows：`K32GetProcessMemoryInfo`（kernel32）→ WorkingSetSize；
//! - Linux：`/proc/self/status` 的 `VmRSS`；
//! - 其它平台：报告 `rss_source = "unavailable"` 并门禁失败（需补平台实现）。
//!
//! ## 用法
//!
//! ```text
//! cd engine
//! # CI quick soak：60 秒，release
//! cargo run --release -p hezhou-examples --bin soak_smoke --no-default-features -- --json
//! # 本地 30 分钟稳定性
//! cargo run --release -p hezhou-examples --bin soak_smoke --no-default-features -- --seconds 1800
//! # 脚本（推荐，带超时保护/模式）
//! pwsh -File engine/scripts/soak_test.ps1 -Mode smoke
//! pwsh -File engine/scripts/soak_test.ps1 -Mode soak
//! ```
//!
//! 报告默认 `test-results/soak/report.json`；默认时长 60s、tick rate 60Hz、
//! 预热 2s（预热期不计入帧时间与内存增长统计）。说明见 `docs/SOAK_TEST_1_0.md`。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use hezhou_animation::{
    AnimationClip, AnimationState, AnimationStateMachine, Channel, Keyframe, Quat, Skeleton,
    Transform, TransitionCurve, Vec3 as AnimVec3,
};
use hezhou_core::ecs::{Entity, System, TransformComponent, World};
use hezhou_core::math::Vec3;
use hezhou_gameplay::{CharacterMotor, MotorConfig, MotorInput, PhysicsQuery};
use hezhou_physics::{ColliderShape, PhysicsWorld, RigidBodyType};

/// 报告格式版本。
const REPORT_FORMAT_VERSION: u32 = 1;
/// 默认 soak 时长（秒）——CI quick soak 使用该值。
const DEFAULT_SECONDS: f64 = 60.0;
/// 默认测量 tick rate。
const DEFAULT_TICK_RATE: f64 = 60.0;
/// 默认预热时长（秒）。
const DEFAULT_WARMUP_SECONDS: f64 = 2.0;
/// 默认内存采样间隔（秒）。
const DEFAULT_MEMORY_INTERVAL: f64 = 1.0;
/// 固定仿真步长（与引擎 60Hz 约定一致）。
const DT: f32 = 1.0 / 60.0;
/// 默认报告目录（仓库根目录下）。
const DEFAULT_REPORT_DIR: &str = "test-results/soak";

// ===========================================================================
// RSS 采样（平台 API，无第三方依赖）
// ===========================================================================

/// RSS 数据来源描述（写入报告，便于跨平台解释）。
pub fn rss_source_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows:k32getprocessmemoryinfo"
    }
    #[cfg(target_os = "linux")]
    {
        "linux:/proc/self/status:VmRSS"
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        "unavailable"
    }
}

/// 当前进程 RSS（字节）；平台不支持或采样失败返回 `None`。
pub fn rss_bytes() -> Option<u64> {
    platform_rss::rss_bytes()
}

#[cfg(target_os = "windows")]
mod platform_rss {
    use core::ffi::c_void;

    /// Windows `PROCESS_MEMORY_COUNTERS`（psapi.h 的公开布局）。
    #[repr(C)]
    #[derive(Debug, Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }

    pub fn rss_bytes() -> Option<u64> {
        let mut counters = ProcessMemoryCounters {
            cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
            ..ProcessMemoryCounters::default()
        };
        // SAFETY: GetCurrentProcess 返回伪句柄（-1），K32GetProcessMemoryInfo 只读进程信息；
        // counters 是栈上有效、布局与 Windows PUBLIC 结构一致的可写内存，长度由 cb 传入。
        let ok =
            unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
        if ok == 0 {
            None
        } else {
            Some(counters.working_set_size as u64)
        }
    }
}

#[cfg(target_os = "linux")]
mod platform_rss {
    pub fn rss_bytes() -> Option<u64> {
        // /proc/self/status 的 VmRSS 单位为 kB；statm 也可，但 status 带单位更不易误读。
        let text = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
                return Some(kb.saturating_mul(1024));
            }
        }
        None
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod platform_rss {
    pub fn rss_bytes() -> Option<u64> {
        None
    }
}

// ===========================================================================
// 报告数据结构
// ===========================================================================

/// 机器 / 构建信息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MachineInfo {
    /// 操作系统。
    pub os: String,
    /// CPU 架构。
    pub arch: String,
    /// 可用逻辑核数。
    pub cpu_cores: usize,
    /// `debug` / `release`。
    pub build_profile: String,
    /// 是否在 CI 中运行（环境变量 `CI`）。
    pub ci: bool,
    /// git commit（CI 的 `GITHUB_SHA` 或脚本注入的 `GIT_COMMIT`）。
    pub git_commit: Option<String>,
}

/// soak 配置快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoakConfig {
    /// 测量时长（秒，不含预热）。
    pub duration_s: f64,
    /// 预热时长（秒）。
    pub warmup_s: f64,
    /// 目标 tick rate（Hz）。
    pub tick_rate: f64,
    /// 内存采样间隔（秒）。
    pub memory_interval_s: f64,
    /// ECS 稳态实体数。
    pub ecs_max_entities: usize,
    /// 每 tick ECS 创建/销毁实体数。
    pub ecs_churn_per_tick: usize,
    /// 物理动态体数量。
    pub physics_bodies: usize,
    /// 动画骨架关节数。
    pub animation_joints: usize,
}

/// 帧耗时统计（毫秒，仅测量阶段）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameReport {
    /// 目标 tick 周期（毫秒）。
    pub target_period_ms: f64,
    /// 平均每 tick 工作量耗时。
    pub avg_ms: f64,
    /// p95 耗时。
    pub p95_ms: f64,
    /// 最大耗时。
    pub max_ms: f64,
    /// 超过目标周期的 tick 数（掉帧/过载次数）。
    pub overruns: u32,
}

/// 一次 RSS 采样。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemorySample {
    /// 相对测量阶段开始的时间（秒）。
    pub t_s: f64,
    /// RSS（MB）。
    pub rss_mb: f64,
}

/// 内存趋势报告。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryReport {
    /// 数据来源（如 `windows:k32getprocessmemoryinfo`）。
    pub source: String,
    /// 预热结束后的起始 RSS（MB）。
    pub start_mb: f64,
    /// 测量结束 RSS（MB）。
    pub end_mb: f64,
    /// 峰值 RSS（MB）。
    pub peak_mb: f64,
    /// 增长量（MB）。
    pub growth_mb: f64,
    /// 增长率（相对起始 RSS，%）。
    pub growth_percent: f64,
    /// 采样序列。
    pub samples: Vec<MemorySample>,
}

/// 各循环累计工作量。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadReport {
    /// ECS 稳态实体数。
    pub ecs_entities_alive: usize,
    /// ECS 累计创建实体数。
    pub ecs_created: u64,
    /// ECS 累计销毁实体数。
    pub ecs_destroyed: u64,
    /// ECS `World::update` 次数。
    pub ecs_updates: u64,
    /// 物理动态体数量。
    pub physics_bodies: usize,
    /// 物理固定步数。
    pub physics_steps: u64,
    /// 物理刚体 churn 次数。
    pub physics_churn: u64,
    /// 动画 update/sample 次数。
    pub animation_updates: u64,
    /// 动画状态切换次数。
    pub animation_state_changes: u64,
    /// Player-ish（CharacterMotor）tick 数。
    pub player_ticks: u64,
}

/// 门禁阈值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoakThresholds {
    /// 允许的最大内存增长率（%）。
    pub max_growth_percent: f64,
    /// 允许的最大内存增长量（MB）。
    pub max_growth_mb: f64,
    /// 增长率判定的最小绝对增长量（MB）。
    ///
    /// 进程 RSS 基数只有几十 MB 时，1~2 MB 的分配器/工作集抖动就可能超过 10%；
    /// 只有增长量也超过该下限时才用增长率判定“疑似泄漏”，避免小基数误报。
    pub min_growth_mb_for_percent: f64,
    /// 允许的最大峰值 RSS（MB）。
    pub max_rss_mb: f64,
    /// 允许的最大平均 tick 耗时（ms）。
    pub max_avg_frame_ms: f64,
    /// 允许的最大 p95 tick 耗时（ms）。
    pub max_p95_frame_ms: f64,
}

/// 完整 soak 报告。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoakReport {
    /// 报告格式版本。
    pub format_version: u32,
    /// 工具名。
    pub tool: String,
    /// 引擎版本（examples crate 版本）。
    pub engine_version: String,
    /// UTC 生成时间（RFC3339）。
    pub generated_at_utc: String,
    /// 机器 / 构建信息。
    pub machine: MachineInfo,
    /// 阈值档位：`ci` / `local`。
    pub profile: String,
    /// 配置快照。
    pub config: SoakConfig,
    /// 测量阶段 tick 数。
    pub ticks: u64,
    /// 预热阶段 tick 数。
    pub warmup_ticks: u64,
    /// 测量阶段实际耗时（秒）。
    pub duration_actual_s: f64,
    /// 帧耗时统计。
    pub frame: FrameReport,
    /// 内存趋势（平台不支持时为 `None`）。
    pub memory: Option<MemoryReport>,
    /// 各循环累计工作量。
    pub workloads: WorkloadReport,
    /// 阈值。
    pub thresholds: SoakThresholds,
    /// 运行期错误/告警（如 RSS 不可用）。
    pub errors: Vec<String>,
    /// 是否通过。
    pub passed: bool,
    /// 失败项说明。
    pub failures: Vec<String>,
    /// 报告文件路径。
    pub report_path: String,
}

// ===========================================================================
// 阈值档位
// ===========================================================================

/// 门禁档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateProfile {
    /// CI：release 构建、60s quick soak。
    Ci,
    /// 本地：debug/长时 soak 使用更宽松阈值。
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

    fn thresholds(self) -> SoakThresholds {
        match self {
            // CI 共享 runner：内存增长 10% / 64MB、峰值 512MB、帧时间留足余量。
            Self::Ci => SoakThresholds {
                max_growth_percent: 10.0,
                max_growth_mb: 64.0,
                min_growth_mb_for_percent: 8.0,
                max_rss_mb: 512.0,
                max_avg_frame_ms: 100.0,
                max_p95_frame_ms: 200.0,
            },
            // 本地（debug/长时）：允许更多分配抖动；30 分钟 soak 仍以增长率为主要判据。
            Self::Local => SoakThresholds {
                max_growth_percent: 20.0,
                max_growth_mb: 128.0,
                min_growth_mb_for_percent: 16.0,
                max_rss_mb: 1024.0,
                max_avg_frame_ms: 250.0,
                max_p95_frame_ms: 500.0,
            },
        }
    }
}

// ===========================================================================
// 命令行参数
// ===========================================================================

/// soak 运行参数。
#[derive(Debug, Clone)]
pub struct SoakOptions {
    /// 测量时长（秒）。
    pub seconds: f64,
    /// 预热时长（秒）。
    pub warmup_seconds: f64,
    /// 目标 tick rate。
    pub tick_rate: f64,
    /// 内存采样间隔（秒）。
    pub memory_interval: f64,
    /// JSON 输出路径。
    pub json_path: PathBuf,
    /// 门禁档位。
    pub gate_profile: GateProfile,
    /// 快速模式（缩减循环工作量，CI quick soak 使用）。
    pub quick: bool,
    /// 覆盖：最大增长率（%）。
    pub max_growth_percent: Option<f64>,
    /// 覆盖：最大增长量（MB）。
    pub max_growth_mb: Option<f64>,
    /// 覆盖：增长率判定的最小绝对增长量（MB）。
    pub min_growth_mb_for_percent: Option<f64>,
    /// 覆盖：最大峰值 RSS（MB）。
    pub max_rss_mb: Option<f64>,
    /// 覆盖：最大平均 tick 耗时（ms）。
    pub max_avg_frame_ms: Option<f64>,
    /// 覆盖：最大 p95 tick 耗时（ms）。
    pub max_p95_frame_ms: Option<f64>,
}

impl SoakOptions {
    /// 默认报告路径（仓库根目录 `test-results/soak/report.json`）。
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
            seconds: DEFAULT_SECONDS,
            warmup_seconds: DEFAULT_WARMUP_SECONDS,
            tick_rate: DEFAULT_TICK_RATE,
            memory_interval: DEFAULT_MEMORY_INTERVAL,
            json_path: Self::default_json_path(),
            gate_profile: GateProfile::Ci,
            quick: false,
            max_growth_percent: None,
            max_growth_mb: None,
            min_growth_mb_for_percent: None,
            max_rss_mb: None,
            max_avg_frame_ms: None,
            max_p95_frame_ms: None,
        };
        let mut iter = args.into_iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--seconds" => {
                    if let Some(value) = iter.next() {
                        if let Ok(seconds) = value.parse::<f64>() {
                            options.seconds = seconds.max(0.05);
                        }
                    }
                }
                "--warmup" => {
                    if let Some(value) = iter.next() {
                        if let Ok(seconds) = value.parse::<f64>() {
                            options.warmup_seconds = seconds.max(0.0);
                        }
                    }
                }
                "--tick-rate" => {
                    if let Some(value) = iter.next() {
                        if let Ok(rate) = value.parse::<f64>() {
                            options.tick_rate = rate.clamp(1.0, 10_000.0);
                        }
                    }
                }
                "--memory-interval" => {
                    if let Some(value) = iter.next() {
                        if let Ok(seconds) = value.parse::<f64>() {
                            options.memory_interval = seconds.max(0.01);
                        }
                    }
                }
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
                "--quick" => options.quick = true,
                "--max-growth-percent" => {
                    options.max_growth_percent = iter.next().and_then(|v| v.parse().ok());
                }
                "--max-growth-mb" => {
                    options.max_growth_mb = iter.next().and_then(|v| v.parse().ok());
                }
                "--min-growth-mb" => {
                    options.min_growth_mb_for_percent = iter.next().and_then(|v| v.parse().ok());
                }
                "--max-rss-mb" => {
                    options.max_rss_mb = iter.next().and_then(|v| v.parse().ok());
                }
                "--max-avg-frame-ms" => {
                    options.max_avg_frame_ms = iter.next().and_then(|v| v.parse().ok());
                }
                "--max-p95-frame-ms" => {
                    options.max_p95_frame_ms = iter.next().and_then(|v| v.parse().ok());
                }
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => eprintln!("忽略未知参数: {arg}"),
            }
        }
        options
    }

    fn effective_thresholds(&self) -> SoakThresholds {
        let mut thresholds = self.gate_profile.thresholds();
        if let Some(value) = self.max_growth_percent {
            thresholds.max_growth_percent = value;
        }
        if let Some(value) = self.max_growth_mb {
            thresholds.max_growth_mb = value;
        }
        if let Some(value) = self.min_growth_mb_for_percent {
            thresholds.min_growth_mb_for_percent = value;
        }
        if let Some(value) = self.max_rss_mb {
            thresholds.max_rss_mb = value;
        }
        if let Some(value) = self.max_avg_frame_ms {
            thresholds.max_avg_frame_ms = value;
        }
        if let Some(value) = self.max_p95_frame_ms {
            thresholds.max_p95_frame_ms = value;
        }
        thresholds
    }
}

fn print_help() {
    println!("Hezhou 1.0 soak smoke（长时间稳定性，headless）");
    println!();
    println!("用法: soak_smoke [--seconds <s>] [--warmup <s>] [--tick-rate <hz>]");
    println!("                 [--memory-interval <s>] [--json [path]] [--profile ci|local]");
    println!("                 [--quick] [--max-growth-percent <p>] [--max-growth-mb <mb>]");
    println!("                 [--min-growth-mb <mb>] [--max-rss-mb <mb>]");
    println!("                 [--max-avg-frame-ms <ms>] [--max-p95-frame-ms <ms>]");
    println!();
    println!(
        "默认: --seconds {DEFAULT_SECONDS} --warmup {DEFAULT_WARMUP_SECONDS} --tick-rate {DEFAULT_TICK_RATE} --memory-interval {DEFAULT_MEMORY_INTERVAL}"
    );
    println!("默认报告: {}", SoakOptions::default_json_path().display());
    println!("退出码: 全部通过 0，超阈值 1，运行错误 2。");
}

// ===========================================================================
// 工作量配置与循环实现
// ===========================================================================

/// 循环工作量配置（quick 模式缩减）。
#[derive(Debug, Clone, Copy)]
pub struct WorkConfig {
    pub ecs_max_entities: usize,
    pub ecs_churn_per_tick: usize,
    pub physics_bodies: usize,
    pub animation_joints: usize,
    pub physics_churn_interval_ticks: u64,
    pub animation_state_cycle_ticks: u64,
}

impl WorkConfig {
    fn full() -> Self {
        Self {
            ecs_max_entities: 2_000,
            ecs_churn_per_tick: 4,
            physics_bodies: 64,
            animation_joints: 16,
            physics_churn_interval_ticks: 60,
            animation_state_cycle_ticks: 120,
        }
    }

    fn quick() -> Self {
        Self {
            ecs_max_entities: 1_000,
            ecs_churn_per_tick: 2,
            physics_bodies: 32,
            animation_joints: 12,
            physics_churn_interval_ticks: 60,
            animation_state_cycle_ticks: 120,
        }
    }
}

/// 简单的确定性伪随机（避免引入 rand 依赖）。
#[derive(Debug, Clone)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    fn next_unit(&mut self) -> f32 {
        (self.next_u32() % 10_000) as f32 / 10_000.0
    }
}

/// soak 用的速度组件（本二进制内部组件）。
#[derive(Clone, Debug)]
struct SoakVelocity {
    velocity: Vec3,
}

hezhou_core::define_component!(SoakVelocity, 0x9101);

/// 每 tick 遍历所有实体更新 Transform（模拟典型 ECS 系统）。
struct SoakEcsUpdateSystem;

impl System for SoakEcsUpdateSystem {
    fn name(&self) -> &'static str {
        "soak_ecs_update"
    }

    fn update(&mut self, world: &mut World, delta_time: f32) {
        for entity in world.all_entities() {
            let velocity = world
                .get_component::<SoakVelocity>(entity)
                .map(|component| component.velocity)
                .unwrap_or_else(Vec3::zero);
            if let Some(mut transform) = world.get_component::<TransformComponent>(entity) {
                transform.position.x += velocity.x * delta_time;
                transform.position.y += velocity.y * delta_time;
                transform.position.z += velocity.z * delta_time;
                if transform.position.y.abs() > 10_000.0 {
                    transform.position.y = 0.0;
                }
                world.add_component(entity, transform);
            }
        }
    }
}

/// soak 的模拟世界：ECS + 物理 + 动画 + Player-ish。
struct SoakSimulation {
    config: WorkConfig,
    world: World,
    entities: Vec<Entity>,
    physics: PhysicsWorld,
    physics_bodies: Vec<u64>,
    skeleton: Skeleton,
    machine: AnimationStateMachine,
    animation_state_index: usize,
    motor: CharacterMotor,
    tick: u64,
    rng: Lcg,
    counters: WorkloadReport,
}

impl SoakSimulation {
    fn new(config: WorkConfig) -> Self {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(config.ecs_max_entities);
        let mut rng = Lcg::new(0x5A17_0001);
        for index in 0..config.ecs_max_entities {
            let entity = world.create_entity();
            world.add_component(
                entity,
                TransformComponent {
                    position: Vec3::new(index as f32 * 0.01, 0.0, 0.0),
                    ..TransformComponent::default()
                },
            );
            world.add_component(
                entity,
                SoakVelocity {
                    velocity: Vec3::new(1.0, 0.0, 0.5),
                },
            );
            entities.push(entity);
        }
        world.add_system(Box::new(SoakEcsUpdateSystem));

        let mut physics = PhysicsWorld::new();
        let ground = physics.create_body_with_type(RigidBodyType::Static);
        if let Some(body) = physics.get_body_mut(ground) {
            body.set_position(hezhou_core::math::Vec3::new(0.0, -0.5, 0.0));
        }
        physics.create_collider(
            ColliderShape::box_size(hezhou_core::math::Vec3::new(400.0, 1.0, 400.0)),
            ground,
        );
        let mut physics_bodies = Vec::with_capacity(config.physics_bodies);
        for index in 0..config.physics_bodies {
            physics_bodies.push(spawn_dynamic_box(&mut physics, index, &mut rng));
        }

        let (skeleton, machine) = build_animation_scene(config.animation_joints);

        let motor = CharacterMotor::with_position(
            MotorConfig::default(),
            hezhou_gameplay::Vec3::new(0.0, 0.5, 0.0),
        );

        Self {
            config,
            world,
            entities,
            physics,
            physics_bodies,
            skeleton,
            machine,
            animation_state_index: 0,
            motor,
            tick: 0,
            rng,
            counters: WorkloadReport {
                ecs_entities_alive: config.ecs_max_entities,
                ecs_created: 0,
                ecs_destroyed: 0,
                ecs_updates: 0,
                physics_bodies: config.physics_bodies,
                physics_steps: 0,
                physics_churn: 0,
                animation_updates: 0,
                animation_state_changes: 0,
                player_ticks: 0,
            },
        }
    }

    /// 执行一个 tick。
    fn tick(&mut self, dt: f32) {
        self.tick += 1;

        // 1) ECS churn：创建新实体并销毁最旧的实体，保持稳态数量。
        for _ in 0..self.config.ecs_churn_per_tick {
            let entity = self.world.create_entity();
            let x = self.rng.next_unit() * 100.0;
            self.world.add_component(
                entity,
                TransformComponent {
                    position: Vec3::new(x, 0.0, 0.0),
                    ..TransformComponent::default()
                },
            );
            self.world.add_component(
                entity,
                SoakVelocity {
                    velocity: Vec3::new(1.0, 0.0, 0.5),
                },
            );
            self.entities.push(entity);
            self.counters.ecs_created += 1;
        }
        while self.entities.len() > self.config.ecs_max_entities {
            let oldest = self.entities.remove(0);
            self.world.destroy_entity(oldest);
            self.counters.ecs_destroyed += 1;
        }
        self.world.update(dt);
        self.counters.ecs_updates += 1;
        self.counters.ecs_entities_alive = self.entities.len();

        // 2) 物理：固定步 + 周期性刚体 churn（销毁最旧、创建新盒）。
        if self
            .tick
            .is_multiple_of(self.config.physics_churn_interval_ticks)
        {
            if let Some(oldest) = self.physics_bodies.first().copied() {
                self.physics.destroy_body(oldest);
                self.physics_bodies.remove(0);
                let new_body =
                    spawn_dynamic_box(&mut self.physics, self.physics_bodies.len(), &mut self.rng);
                self.physics_bodies.push(new_body);
                self.counters.physics_churn += 1;
            }
        }
        self.physics.step(dt);
        self.counters.physics_steps += 1;
        self.counters.physics_bodies = self.physics_bodies.len();

        // 3) 动画：update + sample，周期性切换状态以覆盖过渡/事件路径。
        if self
            .tick
            .is_multiple_of(self.config.animation_state_cycle_ticks)
        {
            self.cycle_animation_state();
            self.counters.animation_state_changes += 1;
        }
        let _ = self.machine.update(dt);
        std::hint::black_box(self.machine.sample(&self.skeleton));
        self.counters.animation_updates += 1;

        // 4) Player-ish：CharacterMotor 通过 physics 查询移动/跳跃/地面吸附。
        let input = MotorInput {
            move_direction: hezhou_gameplay::Vec3::new(1.0, 0.0, 0.3),
            jump_pressed: self.tick.is_multiple_of(90),
            ..MotorInput::default()
        };
        let output = {
            let query = PhysicsQuery::new(&self.physics);
            self.motor.step(dt, &input, &query)
        };
        if output.position.y < -10.0 {
            self.motor
                .teleport(hezhou_gameplay::Vec3::new(0.0, 2.0, 0.0));
        }
        self.counters.player_ticks += 1;
    }

    fn cycle_animation_state(&mut self) {
        const STATES: [AnimationState; 3] = [
            AnimationState::Idle,
            AnimationState::Run,
            AnimationState::Attack,
        ];
        self.animation_state_index = (self.animation_state_index + 1) % STATES.len();
        self.machine
            .transition_to(STATES[self.animation_state_index], 0.1);
    }
}

fn spawn_dynamic_box(physics: &mut PhysicsWorld, index: usize, rng: &mut Lcg) -> u64 {
    let body = physics.create_body_with_type(RigidBodyType::Dynamic);
    let x = (index % 8) as f32 * 1.5 + rng.next_unit() * 0.1;
    let z = (index / 8) as f32 * 1.5;
    if let Some(body_ref) = physics.get_body_mut(body) {
        body_ref.set_position(hezhou_core::math::Vec3::new(x, 1.5, z));
    }
    physics.create_collider(
        ColliderShape::box_size(hezhou_core::math::Vec3::new(0.8, 0.8, 0.8)),
        body,
    );
    body
}

fn build_animation_scene(joints: usize) -> (Skeleton, AnimationStateMachine) {
    let mut skeleton = Skeleton::new("soak_skeleton");
    let root = skeleton.add_root_joint("root", Transform::IDENTITY);
    let mut parent = root;
    for index in 1..joints.max(1) {
        parent = skeleton
            .add_joint(
                format!("joint_{index}"),
                parent,
                Transform::from_trs(AnimVec3::new(0.0, 0.1 * index as f32, 0.0), Quat::IDENTITY),
            )
            .expect("合成骨架关节应合法");
    }

    let make_clip = |name: &str, duration: f32| {
        let mut clip = AnimationClip::new(name, duration);
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
        clip
    };

    let mut machine =
        AnimationStateMachine::new().with_default_transition(0.1, TransitionCurve::SmoothStep);
    machine.set_clip(AnimationState::Idle, make_clip("SoakIdle", 1.0));
    machine.set_clip(AnimationState::Run, make_clip("SoakRun", 0.8));
    machine.set_clip(AnimationState::Attack, make_clip("SoakAttack", 0.6));
    (skeleton, machine)
}

// ===========================================================================
// 运行与报告
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

fn frame_stats(samples: &[f32], target_period_ms: f64) -> FrameReport {
    if samples.is_empty() {
        return FrameReport {
            target_period_ms,
            avg_ms: 0.0,
            p95_ms: 0.0,
            max_ms: 0.0,
            overruns: 0,
        };
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let avg = samples.iter().map(|value| f64::from(*value)).sum::<f64>() / samples.len() as f64;
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    let p95 = f64::from(sorted[index]);
    let max = f64::from(*sorted.last().unwrap_or(&0.0));
    let overruns = samples
        .iter()
        .filter(|value| f64::from(**value) > target_period_ms)
        .count() as u32;
    FrameReport {
        target_period_ms,
        avg_ms: avg,
        p95_ms: p95,
        max_ms: max,
        overruns,
    }
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

/// 运行 soak 并写出 JSON 报告；测试复用该入口。
pub fn run_soak(options: &SoakOptions) -> Result<SoakReport, String> {
    let thresholds = options.effective_thresholds();
    let work_config = if options.quick {
        WorkConfig::quick()
    } else {
        WorkConfig::full()
    };
    let interval = Duration::from_secs_f64(options.memory_interval.max(0.01));
    let period = Duration::from_secs_f64(1.0 / options.tick_rate.max(1.0));
    let period_ms = period.as_secs_f64() * 1_000.0;

    let mut simulation = SoakSimulation::new(work_config);
    let mut errors: Vec<String> = Vec::new();

    // ---- 预热（不计入帧时间/内存统计）----
    let warmup_start = Instant::now();
    let warmup_duration = Duration::from_secs_f64(options.warmup_seconds.max(0.0));
    let warmup_end_at = warmup_start + warmup_duration;
    let mut warmup_ticks = 0u64;
    let mut next_tick = Instant::now();
    while Instant::now() < warmup_end_at {
        simulation.tick(DT);
        warmup_ticks += 1;
        next_tick += period;
        let now = Instant::now();
        if now < next_tick {
            std::thread::sleep(next_tick - now);
        } else if now > next_tick + period {
            // 落后太多时重置节拍，避免预热期 burst。
            next_tick = now;
        }
    }

    // ---- 测量阶段 ----
    let measure_start = Instant::now();
    let measure_duration = Duration::from_secs_f64(options.seconds.max(0.05));
    let measure_end_at = measure_start + measure_duration;
    let start_rss = rss_bytes();
    let mut memory_samples: Vec<MemorySample> = Vec::new();
    if let Some(rss) = start_rss {
        memory_samples.push(MemorySample {
            t_s: 0.0,
            rss_mb: mb(rss),
        });
    } else {
        errors.push(format!(
            "RSS 采样不可用（source={}），无法做内存门禁",
            rss_source_name()
        ));
    }
    let mut next_memory_sample = measure_start + interval;
    let mut frame_samples: Vec<f32> = Vec::new();
    let mut ticks = 0u64;

    next_tick = measure_start;
    while Instant::now() < measure_end_at {
        let tick_start = Instant::now();
        simulation.tick(DT);
        let work_ms = tick_start.elapsed().as_secs_f64() * 1_000.0;
        frame_samples.push(work_ms as f32);
        ticks += 1;

        let now = Instant::now();
        if now >= next_memory_sample {
            if let Some(rss) = rss_bytes() {
                memory_samples.push(MemorySample {
                    t_s: now.duration_since(measure_start).as_secs_f64(),
                    rss_mb: mb(rss),
                });
            }
            next_memory_sample = now + interval;
        }

        next_tick += period;
        let now = Instant::now();
        if now < next_tick {
            std::thread::sleep(next_tick - now);
        } else if now > next_tick + period {
            next_tick = now;
        }
    }
    let duration_actual_s = measure_start.elapsed().as_secs_f64();

    // 结束采样（保证至少 start/end 两点）。
    let end_rss = rss_bytes();
    if let Some(rss) = end_rss {
        let t = duration_actual_s;
        if memory_samples
            .last()
            .map(|sample| t - sample.t_s > 0.000_1)
            .unwrap_or(true)
        {
            memory_samples.push(MemorySample {
                t_s: t,
                rss_mb: mb(rss),
            });
        }
    }

    let frame = frame_stats(&frame_samples, period_ms);
    let memory = match (start_rss, end_rss) {
        (Some(start), Some(end)) => {
            let peak = memory_samples
                .iter()
                .map(|sample| sample.rss_mb)
                .fold(0.0f64, f64::max)
                .max(mb(start))
                .max(mb(end));
            let growth_mb = mb(end) - mb(start);
            let growth_percent = if start > 0 {
                growth_mb / mb(start) * 100.0
            } else {
                0.0
            };
            Some(MemoryReport {
                source: rss_source_name().to_string(),
                start_mb: mb(start),
                end_mb: mb(end),
                peak_mb: peak,
                growth_mb,
                growth_percent,
                samples: memory_samples,
            })
        }
        _ => None,
    };

    if ticks == 0 {
        errors.push("测量阶段没有执行任何 tick".to_string());
    }

    // ---- 门禁 ----
    let mut failures: Vec<String> = Vec::new();
    if let Some(memory) = &memory {
        if memory.growth_percent > thresholds.max_growth_percent
            && memory.growth_mb > thresholds.min_growth_mb_for_percent
        {
            failures.push(format!(
                "内存增长率 {:.2}% > 阈值 {:.2}%",
                memory.growth_percent, thresholds.max_growth_percent
            ));
        }
        if memory.growth_mb > thresholds.max_growth_mb {
            failures.push(format!(
                "内存增长 {:.2} MB > 阈值 {:.2} MB",
                memory.growth_mb, thresholds.max_growth_mb
            ));
        }
        if memory.peak_mb > thresholds.max_rss_mb {
            failures.push(format!(
                "峰值 RSS {:.2} MB > 阈值 {:.2} MB",
                memory.peak_mb, thresholds.max_rss_mb
            ));
        }
    } else {
        failures.push("RSS 不可用，无法验证内存稳定性".to_string());
    }
    if frame.avg_ms > thresholds.max_avg_frame_ms {
        failures.push(format!(
            "平均 tick 耗时 {:.3} ms > 阈值 {:.3} ms",
            frame.avg_ms, thresholds.max_avg_frame_ms
        ));
    }
    if frame.p95_ms > thresholds.max_p95_frame_ms {
        failures.push(format!(
            "p95 tick 耗时 {:.3} ms > 阈值 {:.3} ms",
            frame.p95_ms, thresholds.max_p95_frame_ms
        ));
    }
    let passed = failures.is_empty();

    // ---- 写报告 ----
    if let Some(parent) = options.json_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建报告目录 {} 失败: {error}", parent.display()))?;
    }
    let report = SoakReport {
        format_version: REPORT_FORMAT_VERSION,
        tool: "hezhou-soak-smoke".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at_utc: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        machine: machine_info(),
        profile: options.gate_profile.as_str().to_string(),
        config: SoakConfig {
            duration_s: options.seconds,
            warmup_s: options.warmup_seconds,
            tick_rate: options.tick_rate,
            memory_interval_s: options.memory_interval,
            ecs_max_entities: work_config.ecs_max_entities,
            ecs_churn_per_tick: work_config.ecs_churn_per_tick,
            physics_bodies: work_config.physics_bodies,
            animation_joints: work_config.animation_joints,
        },
        ticks,
        warmup_ticks,
        duration_actual_s,
        frame,
        memory,
        workloads: simulation.counters.clone(),
        thresholds,
        errors,
        passed,
        failures,
        report_path: options.json_path.display().to_string(),
    };
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("序列化 soak 报告失败: {error}"))?;
    fs::write(&options.json_path, format!("{json}\n"))
        .map_err(|error| format!("写 soak 报告 {} 失败: {error}", options.json_path.display()))?;
    Ok(report)
}

/// 打印人类可读摘要。
pub fn print_summary(report: &SoakReport) {
    println!("--- hezhou soak smoke ---");
    println!(
        "build={} profile={} os={} arch={} cores={} ci={}",
        report.machine.build_profile,
        report.profile,
        report.machine.os,
        report.machine.arch,
        report.machine.cpu_cores,
        report.machine.ci
    );
    println!(
        "duration={:.1}s (warmup {:.1}s) ticks={} warmup_ticks={} tick_rate={}Hz",
        report.duration_actual_s,
        report.config.warmup_s,
        report.ticks,
        report.warmup_ticks,
        report.config.tick_rate
    );
    println!(
        "frame: avg={:.3}ms p95={:.3}ms max={:.3}ms overruns={} target={:.3}ms",
        report.frame.avg_ms,
        report.frame.p95_ms,
        report.frame.max_ms,
        report.frame.overruns,
        report.frame.target_period_ms
    );
    match &report.memory {
        Some(memory) => println!(
            "memory: source={} start={:.2}MB end={:.2}MB peak={:.2}MB growth={:.2}MB ({:.2}%) samples={}",
            memory.source,
            memory.start_mb,
            memory.end_mb,
            memory.peak_mb,
            memory.growth_mb,
            memory.growth_percent,
            memory.samples.len()
        ),
        None => println!("memory: unavailable"),
    }
    println!(
        "workloads: ecs_alive={} created={} destroyed={} updates={} | physics_bodies={} steps={} churn={} | anim_updates={} state_changes={} | player_ticks={}",
        report.workloads.ecs_entities_alive,
        report.workloads.ecs_created,
        report.workloads.ecs_destroyed,
        report.workloads.ecs_updates,
        report.workloads.physics_bodies,
        report.workloads.physics_steps,
        report.workloads.physics_churn,
        report.workloads.animation_updates,
        report.workloads.animation_state_changes,
        report.workloads.player_ticks
    );
    if !report.errors.is_empty() {
        println!("errors: {}", report.errors.join("; "));
    }
    println!("report: {}", report.report_path);
}

fn main() {
    let options = SoakOptions::from_args(std::env::args().skip(1));
    println!("Hezhou 1.0 soak smoke（长时间稳定性，headless）");
    println!("report: {}", options.json_path.display());
    let report = match run_soak(&options) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("soak smoke 运行失败: {error}");
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
// 测试（短 duration，验证报告 schema、RSS 采样与门禁逻辑）
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_report_path(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join("soak-smoke")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("report.json")
    }

    fn quick_options(name: &str) -> SoakOptions {
        SoakOptions {
            seconds: 0.6,
            warmup_seconds: 0.1,
            tick_rate: 120.0,
            memory_interval: 0.2,
            json_path: temp_report_path(name),
            gate_profile: GateProfile::Local,
            quick: true,
            max_growth_percent: Some(10_000.0),
            max_growth_mb: Some(10_000.0),
            min_growth_mb_for_percent: Some(0.0),
            max_rss_mb: Some(100_000.0),
            max_avg_frame_ms: Some(100_000.0),
            max_p95_frame_ms: Some(100_000.0),
        }
    }

    #[test]
    fn soak_report_has_memory_and_frame_trends() {
        let options = quick_options("pass");
        let report = run_soak(&options).expect("soak 应能运行");
        assert!(report.passed, "失败项: {:?}", report.failures);
        assert!(report.ticks > 0, "测量阶段应有 tick");
        assert!(report.warmup_ticks > 0, "预热阶段应有 tick");
        assert!(report.duration_actual_s >= 0.5);
        assert!(report.frame.avg_ms >= 0.0);
        assert!(report.frame.p95_ms >= 0.0);
        // 所有循环都被驱动。
        assert!(report.workloads.ecs_created > 0);
        assert!(report.workloads.ecs_destroyed > 0);
        assert!(report.workloads.physics_steps > 0);
        assert!(report.workloads.animation_updates > 0);
        assert!(report.workloads.player_ticks > 0);

        // Windows/Linux 上必须能采样 RSS，并保证有 start/end 两点。
        assert!(
            matches!(std::env::consts::OS, "windows" | "linux"),
            "测试假设 Windows/Linux RSS 可用"
        );
        let memory = report.memory.as_ref().expect("RSS 应可用");
        assert!(memory.source.contains("windows") || memory.source.contains("linux"));
        assert!(memory.samples.len() >= 2, "至少 start/end 两次采样");
        assert!(memory.peak_mb >= memory.start_mb);

        // 报告 JSON 可回读且关键字段一致。
        let text = fs::read_to_string(&options.json_path).expect("报告应写出");
        let parsed: SoakReport = serde_json::from_str(&text).expect("报告应为合法 JSON");
        assert_eq!(parsed.format_version, report.format_version);
        assert_eq!(parsed.tool, report.tool);
        assert_eq!(parsed.ticks, report.ticks);
        assert_eq!(parsed.passed, report.passed);
        assert_eq!(parsed.memory, report.memory);
        assert_eq!(parsed.workloads, report.workloads);
    }

    #[test]
    fn soak_gate_fails_when_growth_threshold_is_impossible() {
        let mut options = quick_options("fail");
        options.max_growth_mb = Some(-1.0);
        let report = run_soak(&options).expect("soak 应能运行");
        assert!(!report.passed, "负增长阈值下应判定失败");
        assert!(report
            .failures
            .iter()
            .any(|failure| failure.contains("内存增长")));
        assert!(report.memory.is_some());
    }

    #[test]
    fn frame_percentile_handles_edges() {
        let stats = frame_stats(&[], 16.6);
        assert_eq!(stats.overruns, 0);
        assert_eq!(stats.avg_ms, 0.0);
        let stats = frame_stats(&[1.0, 2.0, 3.0, 100.0], 16.6);
        assert!(stats.avg_ms > 0.0);
        assert!(stats.p95_ms >= 3.0);
        assert_eq!(stats.max_ms, 100.0);
        assert_eq!(stats.overruns, 1);
    }

    #[test]
    fn rss_sampler_returns_bytes_on_supported_platform() {
        if matches!(std::env::consts::OS, "windows" | "linux") {
            let rss = rss_bytes().expect("Windows/Linux 应能采样 RSS");
            assert!(rss > 1024, "RSS 应大于 1KB，实际 {rss}");
        }
    }

    #[test]
    fn help_placeholder_is_stable() {
        // 防止未使用告警：确认 rss_source_name 会返回平台标识。
        assert!(!rss_source_name().is_empty());
    }
}
