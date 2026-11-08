# Hezhou 0.3 性能基线 / 门禁（headless）

> 状态：0.3 基线 v1（2026-11-08）
> 入口：`engine/examples/src/bin/perf_smoke.rs` + `engine/scripts/perf_baseline.ps1` + CI `perf-baseline` job
> 原则：**可复现、headless、先宽松后收紧**；只作为“明显回归”的冒烟闸门，不是精细性能分析工具。

## 1. 目标与非目标

### 目标

- 在无窗口 / 无 GPU / 无声卡 / 无编辑器依赖的环境下，对 0.2/0.3 引擎核心 CPU 路径做
  **可复现的 smoke 基准**：ECS 组件更新、物理固定步（含宽相候选对）、动画状态机采样、
  音频混音、资产 CookManifest 序列化；
- 每个指标先预热、多轮取**中位数**，降低首次运行/系统噪声；
- 输出机器可读 JSON（`test-results/perf-baseline/report.json`）与人类可读摘要；
- CI 以 release 构建运行并做阈值门禁，报告作为 artifact 上传；超阈值 → job 失败。

### 非目标（0.3 基线不覆盖）

- GPU/渲染性能（PBR、阴影、后处理、蒙皮上传）——需要 GPU runner，留到后续；
- 编辑器/Player 启动时间、包体大小、内存峰值；
- 真实音频设备回调、磁盘 cook I/O（只测 CPU 侧 manifest plan/序列化）；
- 多线程扩展性（当前基准全部单线程串行）；
- 绝对跨硬件对比（阈值只在“同一档硬件 + 同构建 profile”下有意义）。

## 2. 指标定义与当前基线

每个指标 = 一轮固定工作量的耗时（毫秒），门禁作用于**多轮中位数**。
实测环境：

| 项 | 值 |
|---|---|
| OS | Windows 11 家庭版（中文） |
| CPU | Intel Core Ultra 5 338H（12 逻辑核） |
| Rust | rustc/cargo 1.95.0 |
| 引擎版本 | workspace `0.1.0`（0.3 开发期） |
| 采集时间 | 2026-11-08（Asia/Shanghai） |

### 2.1 本机实测（同一份代码、同一台机器）

| 指标 | 工作量 | release 中位 | debug 中位 | CI 阈值 | local 阈值 |
|---|---|---:|---:|---:|---:|
| `ecs_10k_entities_frame` | 10_000 实体 × 10 帧，每帧读 `PerfVelocity` + 写 `TransformComponent`（World + System 调度） | **9.99 ms** | 102.79 ms | 150 ms | 1200 ms |
| `physics_200_bodies_60_steps` | 200 动态盒 + 静态地面，60 个 1/60s 固定步（宽相 + 窄相 + 分离） | **48.27 ms**（0.80 ms/step） | 525.93 ms（8.77 ms/step） | 600 ms | 6000 ms |
| `animation_10k_updates` | 16 关节合成骨架 + 循环 clip，`AnimationStateMachine::update` + `sample` ×10_000 | **9.98 ms** | 32.99 ms | 200 ms | 500 ms |
| `audio_100_voices_1000_frames` | `NullBackend` + 100 个循环 voice，`AudioEngine::update` 推进 1000 帧 @48kHz | **1.59 ms** | 15.06 ms | 80 ms | 300 ms |
| `asset_1000_entries` | 1000 条 `CookManifest`：构造 + `to_json` + 反序列化（JSON ≈ 344 KB） | **1.22 ms** | 14.68 ms | 80 ms | 300 ms |

附：物理宽相观测值（release/debug 一致，取决于场景而非 profile）：

```json
{ "broadphase_body_count": 201, "candidate_pairs": 912, "spatial_index": true, "steps": 60 }
```

> 宽相使用空间索引，201 个碰撞体 60 步后的候选对为 912，远小于 brute-force 的
> `201*200/2 = 20100`；该字段用于观察宽相退化（若突然接近 brute-force，说明空间索引失效）。

### 2.2 JSON 报告结构

```json
{
  "format_version": 1,
  "tool": "hezhou-perf-smoke",
  "engine_version": "0.1.0",
  "generated_at_utc": "2026-11-08T...Z",
  "machine": { "os": "windows", "arch": "x86_64", "cpu_cores": 12,
               "build_profile": "release", "ci": false, "git_commit": "..." },
  "gate_profile": "ci",
  "threshold_scale": 1.0,
  "benchmarks": [
    {
      "name": "physics_200_bodies_60_steps",
      "workload": "200 dynamic boxes + ground, 60 fixed steps",
      "iterations": 1, "rounds": 5,
      "median_ms": 50.30, "min_ms": 49.76, "max_ms": 51.58,
      "per_iteration_ms": 50.30, "threshold_ms": 600.0, "passed": true,
      "detail": { "bodies": 200, "steps": 60, "step_ms": 0.838,
                  "candidate_pairs": 912, "broadphase_body_count": 201,
                  "spatial_index": true }
    }
  ],
  "passed": true,
  "failures": [],
  "report_path": "..."
}
```

字段约定：`median_ms` 是门禁对象；`per_iteration_ms` 便于比较“单帧/单步”成本；
`threshold_ms` 是本次运行实际使用的阈值（含 `--threshold-scale`）。

## 3. 如何本地运行

```powershell
# 仓库根目录执行；release（CI 同款门禁）
pwsh -File engine/scripts/perf_baseline.ps1

# 本地 debug（阈值更宽松，适合开发机）
pwsh -File engine/scripts/perf_baseline.ps1 -Profile debug

# 更快：缩减工作量（只做冒烟，不用于基线）
pwsh -File engine/scripts/perf_baseline.ps1 -Profile debug -Quick

# 直接运行二进制（等价）
cd engine
cargo run --release -p hezhou-examples --bin perf_smoke --no-default-features -- --json
cargo run -p hezhou-examples --bin perf_smoke --no-default-features -- --profile local --json target/perf-debug.json
```

常用参数：

| 参数 | 作用 |
|---|---|
| `-Profile release\|debug` | `release` 用 `ci` 阈值；`debug` 用 `local` 阈值 |
| `-OutPath <path>` | 报告路径（默认 release → `test-results/perf-baseline/report.json`，debug → `report-debug.json`） |
| `-ThresholdScale <f64>` | 放大/缩小阈值（调试阈值噪声；不要用于 CI） |
| `-Rounds <n>` | 覆盖测量轮数 |
| `-Quick` | 缩减工作量（单测/本地冒烟） |
| `-Offline` | 本地无网络时走 cargo 缓存 |
| `-BaselineJson <path> -MaxRegression <x>` | 与历史基线逐项比较，超过 `x` 倍判定回归 |
| `--json [path]` | 直接运行二进制时的 JSON 输出路径 |
| `--profile ci\|local` | 直接运行二进制时的阈值档位 |

退出码：`0` 全部通过；桥阈值/回归为 `1`；运行失败（如构建/IO）为 `2`。
CI 只认退出码，报告 JSON 作为 artifact 保留用于排查。

## 4. CI 门禁

`.github/workflows/ci.yml` 新增 `perf-baseline` job：

- runner：`ubuntu-latest`（安装 CMake/Ninja/X11/Wayland/ALSA 等构建依赖）；
- 工具链：与其它 job 一致的 Rust 1.95.0；
- 命令：`pwsh -File engine/scripts/perf_baseline.ps1 -Profile release`
  （内部 `cargo run --release -p hezhou-examples --bin perf_smoke --no-default-features -- --json ...`，
  即 headless release 基准）；
- 失败条件：任一指标超过 CI 阈值 → 非 0 退出 → job 红；
- artifact：`perf-baseline-report`（`test-results/perf-baseline/**`，含 `report.json`），
  同时把摘要写入 GitHub Step Summary（脚本在 `GITHUB_STEP_SUMMARY` 存在时自动追加表格）。

> 说明：任务要求“报告 CI 结果”。本次改动**未 commit/push**，因此没有触发 GitHub Actions；
> 本地已用与 CI 完全相同的脚本 + 命令（release/no-default-features）验证通过，见 §2 实测。
> 首次真实 CI 运行应在 PR 中观察 runner 耗时，若阈值误报按 §5 流程处理。

## 5. 阈值与噪声策略

### 5.1 为什么阈值这么宽

共享 CI runner（尤其 ubuntu-latest）CPU 型号/负载不固定，release 实测可能与本地差 2~3 倍；
另外 Rust 版本、链接器、是否命中缓存也会影响。因此：

- **CI 阈值 ≈ 本机 release 实测 × 5~10**：作为“明显回归/环境故障”的闸门
  （例如物理宽相退化为 O(n²)、动画每帧分配爆炸、音频混音器退化等可达 10x 量级的问题）；
- **local 阈值 ≈ 本机 debug 实测 × 10**：让开发机 debug 构建也能一键跑通，
  不把 debug 的 ~9x 慢误判为回归；
- 真正的性能优化/回归比较请用 `-BaselineJson` 与同机同 profile 的历史报告做
  逐项 ratio 对比（默认 `-MaxRegression 1.5`）。

### 5.2 测量方法

每个指标：

1. 构造场景（不计时）；
2. 预热若干轮（不计入统计）；
3. 连续测量 `rounds` 轮（默认 5），每轮计总耗时；
4. 取轮耗时**中位数**作为结果（避免单轮 GC/调度噪声）；
5. 同时记录 min/max，便于发现抖动。

### 5.3 更新基线流程

1. 在同档硬件、release 构建下运行 `perf_baseline.ps1`，保存 `report.json` 作为历史基线；
2. 若指标变化来自**预期改动**（功能/数据量变化），在 PR 中说明原因，更新
   `BenchLimits` 阈值与本文 §2 表格，并保留旧报告用于对比；
3. 若指标变慢但无法解释，优先排查而不是直接放宽阈值：
   - 物理：看 `candidate_pairs` 是否接近 brute-force、`spatial_index` 是否变 false；
   - ECS：检查是否新增每帧分配/字符串操作/重复组件克隆；
   - 动画：检查 `Pose`/事件 Vec 分配、clip 通道数；
   - 音频：检查 mixer voice 上限、重采样/锁竞争（null backend 下是纯 CPU）；
   - 资产：检查 manifest 条目/JSON 大小、依赖闭包排序。
4. 阈值调整必须同时更新 `engine/examples/src/bin/perf_smoke.rs` 的 `BenchLimits`
   与本文档，避免“代码与文档漂移”。

## 6. 指标实现细节（便于扩展）

| 指标 | 实现位置 | 要点 |
|---|---|---|
| ECS | `bench_ecs` | `World` + 自定义 `PerfVelocity` 组件 + `EcsUpdateSystem`；每帧 `all_entities` + `get/add_component`，覆盖调度与组件读写 |
| 物理 | `bench_physics` | 重建世界（场景构造不计时），60 步 `PhysicsWorld::step(1/60)`；结束后读 `broadphase_stats()` |
| 动画 | `bench_animation` | 16 关节链 + 16 通道循环 clip；`update` + `sample` 成对调用 |
| 音频 | `bench_audio` | `AudioEngine::with_config(NullBackend::boxed(), 48kHz, max_voices=108)`；100 个 `looping` voice |
| 资产 | `bench_asset` | 1000 条 `CookEntry`（`AssetKind::Binary`）→ `CookManifest::to_json` → 反序列化；只测 CPU 侧 plan/序列化 |

扩展新指标：在 `BenchScale`/`BenchLimits` 增加工作量与阈值，新增 `bench_*` 返回
`BenchmarkResult`，并加入 `run_perf` 的 `benchmarks` 列表；CI 与脚本无需改动。

## 7. 已知限制

1. **单机基线**：阈值按“同档机器”设计；跨机器/跨 OS 请先本地跑一次重建基线，
   GitHub runner 与本地机器不可直接比较绝对值；
2. **单线程**：未覆盖多线程扩展性；`available_parallelism` 仅记录在报告里；
3. **无 GPU**：渲染/蒙皮上传/present 不在本基线内；
4. **音频为 NullBackend**：测的是混音器 CPU 路径，不含真实设备回调/驱动开销；
5. **资产只测 manifest**：不含文件扫描、`.meta` 写盘、glTF 导入、cook 拷贝 I/O；
6. **debug/local 阈值很宽**：仅用于“能跑通”，不是性能判据；
7. **perf job 与其它 CI job 并行**：共享 runner 时会有资源竞争，阈值已留余量；
   若长期出现噪声，可改为夜间/自托管固定 runner（0.3 后续）。

## 8. 变更记录

| 日期 | 变更 |
|---|---|
| 2026-11-08 | 建立 0.3 性能基线 v1：`perf_smoke` 5 项指标、阈值门禁、脚本、CI `perf-baseline` job、本文件 |
