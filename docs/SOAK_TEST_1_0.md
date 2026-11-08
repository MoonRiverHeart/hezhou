# Hezhou 1.0 长时间稳定性（soak）测试

> 状态：1.0 稳定性第一批（2026-11-08）
> 入口：`engine/examples/src/bin/soak_smoke.rs` + `engine/scripts/soak_test.ps1` + CI `soak-quick` job
> 定位：headless、可复现的**长时间运行冒烟**；重点抓内存持续增长、每 tick 耗时劣化和循环/泄漏类问题，
> 不是精细 profiler（细粒度热点分析仍用 DFX Trace / Profiler）。

## 1. 目标与非目标

### 目标

- 用固定 tick rate 长时间驱动 0.4/1.0 的核心循环：ECS 实体创建/销毁与组件更新、`PhysicsWorld::step`、
  `AnimationStateMachine::update + sample`、Player-ish（`CharacterMotor` + physics 查询）；
- 周期性采样进程 RSS，输出**趋势 JSON**（start/end/peak/growth%、采样序列）与每 tick 耗时
  （avg / p95 / max / overruns）；
- 预热后统计，排除首次分配/懒初始化；内存增长或帧耗时超阈值 → 非 0 退出；
- 一条命令支持 60s（CI quick）、300s（本地验证）、1800s（30 分钟）三种模式，并带超时保护。

### 非目标

- GPU/渲染/蒙皮上传稳定性（需要 GPU runner，当前为 headless CPU 路径）；
- 真实音频设备/驱动、网络、磁盘 I/O 长时间稳定性；
- 精细内存归因（per-allocator/堆栈）；本测试只给 RSS 级趋势；
- 多线程竞争/死锁（当前循环单线程，后续可加并发 soak）；
- 编辑器/Player 进程级 24h 稳定性（建议在 1.0 发布前用自托管 runner 夜间跑 30min/2h）。

## 2. 每个 tick 驱动什么

| 循环 | 每 tick 内容 |
|---|---|
| ECS | 创建 `ecs_churn_per_tick` 个实体（Transform + `SoakVelocity` 组件），销毁最旧实体保持稳态数量；`World::update` 由自定义 System 更新全部实体 Transform |
| Physics | `PhysicsWorld::step(1/60)`；每 60 tick 销毁最旧动态盒并创建一个新动态盒（churn） |
| Animation | `AnimationStateMachine::update + sample`；每 120 tick 在 Idle/Run/Attack 间切换（覆盖过渡/事件路径） |
| Player-ish | `CharacterMotor::step` 通过 gameplay `PhysicsQuery`（胶囊扫掠/地面吸附）；周期性跳跃、掉出边界传送复位 |

默认工作量（`--quick` 为缩减档）：

| 参数 | full | quick |
|---|---:|---:|
| ECS 稳态实体数 | 2_000 | 1_000 |
| 每 tick churn 实体 | 4 | 2 |
| 物理动态盒 | 64 | 32 |
| 动画关节数 | 16 | 12 |
| tick rate | 60 Hz | 60 Hz |

## 3. RSS 采样（无第三方依赖）

| 平台 | 实现 | 说明 |
|---|---|---|
| Windows | `kernel32!K32GetProcessMemoryInfo` + `PROCESS_MEMORY_COUNTERS.WorkingSetSize` | `extern "system"` 直接 FFI，不引入 windows-sys/crate |
| Linux | `/proc/self/status` 的 `VmRSS`（kB → bytes） | 纯文件读取 |
| 其它平台 | `None` | 报告 `memory = null` 并门禁失败（需要补平台实现） |

RSS 的含义与局限：

- RSS 是**驻留物理内存**，包含分配器缓存/栈/映射；不等价于“活跃对象内存”；
- Windows Working Set 会随系统内存压力与访问模式波动（可能短暂上升/下降）；
- Linux VmRSS 不含被换出页、部分内核缓存；
- 因此判断泄漏看**预热后长期趋势**（增长是否持续、是否随运行时间线性上升），
  单个采样点的 ±几 MB 抖动不代表泄漏。

## 4. 指标与 JSON

`test-results/soak/report.json` 主要字段：

```json
{
  "format_version": 1,
  "tool": "hezhou-soak-smoke",
  "engine_version": "0.1.0",
  "machine": { "os": "windows", "arch": "x86_64", "cpu_cores": 12,
               "build_profile": "release", "ci": false, "git_commit": "..." },
  "profile": "ci",
  "config": { "duration_s": 60, "warmup_s": 2, "tick_rate": 60, "memory_interval_s": 1,
              "ecs_max_entities": 2000, "ecs_churn_per_tick": 4,
              "physics_bodies": 64, "animation_joints": 16 },
  "ticks": 3600, "warmup_ticks": 120, "duration_actual_s": 60.0,
  "frame": { "target_period_ms": 16.67, "avg_ms": 0.844, "p95_ms": 1.236,
             "max_ms": 4.525, "overruns": 0 },
  "memory": { "source": "windows:k32getprocessmemoryinfo",
              "start_mb": 5.79, "end_mb": 6.15, "peak_mb": 6.27,
              "growth_mb": 0.37, "growth_percent": 6.35,
              "samples": [ { "t_s": 0.0, "rss_mb": 5.79 }, ... ] },
  "workloads": { "ecs_entities_alive": 2000, "ecs_created": 14880, "ecs_destroyed": 14880,
                 "ecs_updates": 3720, "physics_bodies": 64, "physics_steps": 3720,
                 "physics_churn": 62, "animation_updates": 3720,
                 "animation_state_changes": 31, "player_ticks": 3720 },
  "thresholds": { "max_growth_percent": 10, "max_growth_mb": 64,
                  "min_growth_mb_for_percent": 8, "max_rss_mb": 512,
                  "max_avg_frame_ms": 100, "max_p95_frame_ms": 200 },
  "errors": [],
  "passed": true,
  "failures": [],
  "report_path": "..."
}
```

指标解释：

- `ticks`/`warmup_ticks`：测量阶段/预热阶段实际 tick 数（60s × 60Hz ≈ 3600）；
- `frame.avg/p95/max`：每 tick **工作量**耗时（不含节拍 sleep），用于发现每 tick 分配/查询劣化；
- `overruns`：工作量超过目标周期（16.67ms）的 tick 数，>0 说明单帧可能掉帧；
- `memory.growth_percent` / `growth_mb`：预热结束 → 测量结束的 RSS 变化；
- `workloads`：确认各循环真的在跑（created==destroyed、physics_churn>0、state_changes>0）。

## 5. 如何运行

### 5.1 脚本（推荐；带超时保护与模式）

```powershell
# 仓库根目录
pwsh -File engine/scripts/soak_test.ps1                 # smoke：60s release
pwsh -File engine/scripts/soak_test.ps1 -Mode long      # 300s
pwsh -File engine/scripts/soak_test.ps1 -Mode soak      # 1800s（30 分钟）
pwsh -File engine/scripts/soak_test.ps1 -Quick          # CI 同款快速模式（60s）
pwsh -File engine/scripts/soak_test.ps1 -Seconds 120 -Profile debug
pwsh -File engine/scripts/soak_test.ps1 -Offline        # 无网络环境（cargo 缓存）
```

常用参数：

| 参数 | 作用 |
|---|---|
| `-Mode smoke\|ci\|long\|soak` | 60s / 60s / 300s / 1800s |
| `-Seconds <s>` | 自定义测量时长（覆盖 Mode） |
| `-Warmup <s>` | 预热时长（默认 quick 1s，否则 2s） |
| `-TickRate <hz>` | 目标 tick rate（默认 60） |
| `-MemoryInterval <s>` | RSS 采样间隔（默认 1s） |
| `-Profile release\|debug` | 构建 profile + 阈值档（debug 用 local 阈值） |
| `-Quick` | 缩减循环工作量（CI quick soak） |
| `-OutPath <path>` | 报告路径（默认 release → `test-results/soak/report.json`，debug → `report-debug.json`） |
| `-TimeoutSeconds <s>` | 超时保护；超时 kill 进程并返回 3 |
| `-MaxGrowthPercent/-MaxGrowthMb/-MinGrowthMb/-MaxRssMb/-MaxAvgFrameMs/-MaxP95FrameMs` | 临时覆盖阈值 |

退出码：`0` 通过；`1` 超阈值；`2` 构建/报告错误；`3` 超时被终止。

### 5.2 直接运行二进制（等价）

```bash
cd engine
cargo run --release -p hezhou-examples --bin soak_smoke --no-default-features -- --json
cargo run --release -p hezhou-examples --bin soak_smoke --no-default-features -- --seconds 1800 --json
cargo run -p hezhou-examples --bin soak_smoke --no-default-features -- --seconds 60 --profile local
cargo test -p hezhou-examples --bin soak_smoke --no-default-features   # 轻量单测（短 duration）
```

## 6. 当前基线（本机 release）

实测环境：Windows 11、Intel Core Ultra 5 338H（12 逻辑核）、rustc 1.95.0、release。

### 6.1 60s 完整工作量（CI 门禁同款参数）

| 指标 | 值 |
|---|---:|
| duration / warmup / tick | 60.0s / 2.0s / 60Hz |
| ticks / warmup_ticks | 3600 / 120 |
| frame avg / p95 / max | **1.353 ms / 2.388 ms / 3.282 ms**（多轮会随机器负载在 ~0.8–1.4 ms 波动） |
| overruns | 0 |
| memory start → end（peak） | 5.76 → 6.15 MB（peak 6.18 MB） |
| growth | **0.39 MB（6.71%）** |
| ECS created/destroyed | 14 880 / 14 880（alive=2000） |
| physics steps / churn | 3720 / 62 |
| animation updates / state changes | 3720 / 31 |
| player ticks | 3720 |
| 结果 | **PASS（exit 0）** |

### 6.2 60s CI-equivalent quick 工作量（`-Quick`）

| 指标 | 值 |
|---|---:|
| ticks | 3600 |
| frame avg / p95 | 0.469 ms / 0.743 ms |
| memory start → end（growth） | 5.37 → 5.68 MB（0.31 MB / 5.82%） |
| ECS alive / created / destroyed | 1000 / 7320 / 7320 |
| physics bodies / churn | 32 / 61 |
| 结果 | **PASS（exit 0）** |

> 本机 30 分钟模式（`-Mode soak`）未在本次验证中实跑（耗时 30 分钟），脚本已支持；
> 建议 1.0 发布前/夜间在固定机器上跑至少一次 1800s，并保存报告作为长期基线。

## 7. 阈值与内存泄漏判断

### 7.1 阈值档

| 阈值 | CI（release） | Local（debug/长时） |
|---|---:|---:|
| `max_growth_percent` | 10% | 20% |
| `max_growth_mb` | 64 MB | 128 MB |
| `min_growth_mb_for_percent` | 8 MB | 16 MB |
| `max_rss_mb` | 512 MB | 1024 MB |
| `max_avg_frame_ms` | 100 ms | 250 ms |
| `max_p95_frame_ms` | 200 ms | 500 ms |

设计说明：

- 进程 RSS 基数只有几 MB~几十 MB 时，1~2 MB 的分配器/工作集抖动就可能超过 10%，
  因此**增长率判定要求增长量同时超过 `min_growth_mb_for_percent`**，避免小基数误报；
- 绝对增长（`max_growth_mb`）与峰值 RSS 是独立闸门，任何一项超限都会失败；
- 共享 CI runner 性能波动大，帧时间阈值只做“明显劣化”兜底（正常 <2ms，阈值 100/200ms）。

### 7.2 判断是否内存泄漏

1. **看趋势不看单点**：预热后 `growth_percent` 在 10% 内且 `samples` 曲线走平（sawtooth 可接受）；
2. **多轮复跑**：同一机器连跑 2~3 次 60s/300s，若每次结束 RSS 都阶梯上升（不回落），优先怀疑泄漏；
3. **30 分钟确认**：`-Mode soak` 观察 growth 是否随时间线性上升；线性斜率 ≈ `growth_mb / 分钟`
   可用于估算小时级增长；
4. **结合工作量计数**：`ecs_created == ecs_destroyed`、`physics_churn > 0`、`overruns == 0` 说明循环本身正常，
   排除“测试没跑起来”；
5. **定位手段**：RSS 只给趋势，进一步用 DFX Trace / 分配器统计 / 分模块开关循环二分定位。

## 8. CI 门禁

`.github/workflows/ci.yml` 新增 `soak-quick` job：

- runner：`ubuntu-latest`（`needs: perf-baseline`，复用其 rust-cache `shared-key: hezhou-ci-rust`；
  安装 CMake/Ninja/X11/Wayland/ALSA 等构建依赖）；
- 命令：`pwsh -File engine/scripts/soak_test.ps1 -Mode smoke -Quick -Profile release`
  （内部 `cargo build --release -p hezhou-examples --bin soak_smoke --no-default-features` 后运行 60s）；
- 失败条件：内存/帧时间超阈值 → 脚本返回 1 → job 红；超时（默认 60+1+180s 保护）→ 返回 3；
- artifact：`soak-report`（`test-results/soak/**`，含 `report.json`），同时写 GitHub Step Summary。

> 说明：本次改动**未 commit/push**，GitHub Actions 未真实触发；本地已用与 CI 相同命令
> （release + quick + 60s）验证通过，见 §6.2。夜间 30 分钟可在自托管 runner 上手动
> `pwsh -File engine/scripts/soak_test.ps1 -Mode soak`，或后续增加 `schedule` job（当前不默认开启以控成本）。

## 9. 硬件差异与可复现性

- 帧耗时与机器强相关（CI runner 常比本地慢 2~3 倍），本测试的门禁阈值因此留大余量；
- RSS 起点受二进制大小/静态数据/平台影响（本机 Windows release ≈6MB 工作集），
  **跨机器/跨 OS 不要直接比较绝对值**，只看同机趋势与增长率；
- 长期基线建议固定一台机器（或自托管 runner）保存 `report.json` 历史，比较同参数报告；
- `machine` 字段（os/arch/cores/build_profile/ci/git_commit）与 `config` 字段用于解释差异。

## 10. 已知限制

1. headless CPU 路径：不加载 Vulkan/真实音频/编辑器，GPU/驱动/设备类泄漏覆盖不到；
2. 单线程：不检测并发竞争/死锁；
3. RSS 粒度粗：小对象泄漏（KB 级/分钟）在 60s 内可能不可见，需要 30min+ 或堆分析；
4. 60s CI quick 主要抓“粗粒度回归/明显泄漏”，不是长时间验收；
5. 非 Windows/Linux 平台 RSS 不可用（门禁失败），需要时补平台实现；
6. 本测试的 ECS/物理/动画数值为稳定性压测数据，不代表真实游戏负载；
7. 30 分钟模式受机器休眠/系统更新影响，CI 建议用自托管固定 runner。

## 11. 变更记录

| 日期 | 变更 |
|---|---|
| 2026-11-08 | 建立 1.0 soak 第一批：`soak_smoke`（ECS/物理/动画/Player-ish + RSS + JSON + 阈值）、`soak_test.ps1`、CI `soak-quick` job、本文件 |
