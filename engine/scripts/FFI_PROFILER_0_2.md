# 编辑器 Profiler 面板（0.3 task-39）

集成性能面板：CPU 帧时间 ring buffer、FPS、启动耗时、场景/渲染/资产/音频统计。
配套：`FFI_RENDER_0_2.md`（画质/PBR 通道）、`FFI_PBR_MATERIAL_0_2.md`（每实体材质）。

| 层 | 文件 |
|---|---|
| 指标采集 + FFI | `engine/examples/src/editor/profiler_ffi.rs`（新增） |
| 主循环接入 | `engine/examples/src/editor/mod.rs`（启动计时 + 每帧 `record_frame`/`record_dfx`） |
| 只读统计来源 | `engine/examples/src/editor/audio_asset_ffi.rs`（`asset_stats` / `audio_backend_name`） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail 追加，基线 **319**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}` |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 319`） |

## 1. 指标定义

| 指标 | 定义 |
|---|---|
| `frame_ms_avg/p95/max/last` | 主循环每帧墙钟耗时（含 UI 更新 + 渲染提交），最近 120 帧 ring buffer；p95 为样本排序后的最近秩估计 |
| `fps` | `1000 / frame_ms_avg`（ring buffer 平均值）；`dfx_fps` 为 DFX PerfMonitor 独立统计值 |
| `startup_ms` | `editor::run()` 开始到 C# `Initialize` 完成（含运行时准备、C# 编译/加载、UI 初始化） |
| `frame_count` | 自上次 reset 后累计记录帧数（单调递增） |
| `entity_count` | 当前 Scene 实体数 |
| `pbr_entity_count` | renderer 每实体 PBR 材质覆盖数（与 Inspector PBR 编辑同步） |
| `skinned_mesh_count` | renderer 已上传真实蒙皮网格数 |
| `asset_count/asset_total_count` | AssetDatabase 文件资产数 / 全部条目数（含子资产） |
| `audio_backend` | 音频后端名（`off`=未初始化，无声卡时为 null backend） |
| `cpu_percent` / `memory_used_mb` / `memory_available_mb` | DFX PerfMonitor 平台可用指标（不可用为 0） |

限制：**CPU 帧时间（墙钟）**，非 GPU timestamp；不做逐 pass GPU 计时；ring buffer 仅保留最近 120 帧。

## 2. FFI（新增 3，FfiContext 319）

| FFI | 签名 | 语义 |
|---|---|---|
| `profiler_snapshot_json` | `fn(buf, len) -> usize` | 写入快照 JSON，返回字节数；0=失败/缓冲区无效 |
| `profiler_reset` | `fn() -> i32` | 重置帧统计（保留启动耗时）；0=成功 |
| `profiler_frame_count` | `fn() -> i32` | 累计记录帧数（饱和到 i32::MAX） |

快照 JSON 示例（字段稳定）：

```json
{
  "frame_count": 361, "sample_count": 120, "ring_capacity": 120,
  "fps": 59.8, "frame_ms_avg": 16.7, "frame_ms_p95": 18.2,
  "frame_ms_max": 25.1, "frame_ms_last": 16.4,
  "startup_ms": 2450.5, "dfx_fps": 59.6, "cpu_percent": 12.0,
  "memory_used_mb": 128.5, "memory_available_mb": 4096.0,
  "entity_count": 12, "pbr_entity_count": 3, "skinned_mesh_count": 2,
  "asset_count": 28, "asset_total_count": 28, "audio_backend": "cpal",
  "frames": [16.7, 16.9, ...]
}
```

C# 包装：`UI.ProfilerSnapshotJson()`、`UI.ProfilerReset()`、`UI.ProfilerFrameCount()`。

## 3. Profiler 面板用法

左侧资产面板新增 **Profiler** Tab（在「工程」之后）：

- 第 1 行：FPS（DFX 对照）、帧 ms 平均/p95/max
- 第 2 行：启动耗时、采样数/容量、累计帧数
- 第 3 行：实体数、PBR 材质数、蒙皮网格数
- 第 4 行：资产文件数/总条目数、音频后端
- 第 5 行：内存占用/可用、CPU%
- 按钮：**暂停刷新**（保留当前数据，再点恢复）、**重置统计**（清空帧样本与计数，启动耗时保留）
- 刷新策略：`EditorScript.Update` 每 30 帧读取一次快照（暂停时跳过），不阻塞 UI；首次拿到数据时 Console 输出一条 `[Profiler] snapshot: ...` 便于自动测试断言

## 4. 测试证据

- 编辑器启动自检：**PASS=80, FAIL=0**（task-26 的 77 + Profiler 3 项）
  - `ProfilerSnapshotJson 包含必要字段（len=373）`（fps/frame_ms_avg/p95/startup_ms/entity/PBR/skinned/asset）
  - `ProfilerReset 返回 0`、`重置后帧计数为 0`
- 运行约 30 帧后日志：`[Profiler] snapshot: frames=29 samples=29 fps=18.4 avg_ms=54.218 startup_ms=7107.8 entities=1 pbr=0 skinned=0 assets=26/26 audio=cpal`
  （启动阶段 C# 编译/加载占主要耗时；运行期帧率随场景复杂度变化）
- 契约 `check_ffi_contract.py`：319/319/319 OK；`build_mono.ps1` exit 0；
  `ci_local.ps1 -SkipBuild` 通过。

## 5. 已知限制 / 后续

1. 仅 CPU 墙钟帧时间；GPU timestamp/pass 级耗时需 renderer 提供 query 接口（不在本任务范围）。
2. 启动耗时包含 C# 编译（首次启动明显偏大）；后续可拆分「引擎初始化 / C# 编译 / UI 初始化」分段计时。
3. 资产/音频统计直接读编辑器全局（单线程），未做采样平滑；reset 不影响 DFX 自身统计。
4. 面板为纯文本行，无曲线图；后续可用 Canvas/Icon 绘制帧时间趋势。
5. Skinned/PBR 计数来自 renderer 覆盖表，legacy 通道下 PBR 覆盖仍计数但不参与绘制。
