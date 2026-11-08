# 编辑器动画状态机预览（0.3 task-47）

把动画预览从单 clip 时间轴扩展为 **9 状态 AnimationStateMachine**（复用 `hezhou-animation` 的真实状态机，
不渲染角色）。

| 层 | 文件 |
|---|---|
| 状态图/FFI | `engine/examples/src/editor/animation_ffi.rs`（扩展） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail，基线 **334**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}`（动画 Tab 扩展） |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 334`） |

## 1. 状态映射规则

9 状态固定为 `AnimationState`：**Idle / Run / Jump / Attack / Hit / Death / Land / Dodge / Block**。

- 用 `ImportedScene.animations[]` 的 clip 名称做**大小写不敏感**匹配；
- **Idle 无匹配时回退**：`animation_preview_init_graph(path, clip_index)` 的 `clip_index`（>=0）
  → 否则场景首个 clip；
- 其余状态无匹配则 `has_clip=false`（状态仍可切换、过渡 alpha 可预览，不采样）；
- clip 以 `AnimationClip::new(name, duration)` 绑定到状态机（本轮不构建关键帧通道，因为不渲染姿态）；
- 状态机 `with_default_transition` 使用 crate 默认（0.2s / SmoothStep），切换即进入过渡。

## 2. FFI（新增 4，FfiContext 334）

| FFI | 签名 | 语义 |
|---|---|---|
| `animation_preview_init_graph` | `fn(path, clip_index) -> i32` | 从 glTF 构建 9 状态图；0=成功，-1=路径/导入失败 |
| `animation_preview_set_state` | `fn(name) -> i32` | 请求切换（大小写不敏感）；0=成功，-1 未知状态，-2 未初始化，-3 不允许切换（Death/同状态/过渡目标相同） |
| `animation_preview_state_snapshot` | `fn(buf, len) -> usize` | 快照 JSON：initialized/path/has_skeleton/current_state/current_index/current_time/normalized_time/transitioning/transition_alpha/from_state/to_state/event_count/states[9] |
| `animation_preview_graph_free` | `fn() -> i32` | 释放状态图（时间轴仍可用） |

- `animation_preview_tick(dt, playing)` 在播放时同时 `machine.update(dt)`，累计事件数（transition/enter/exit/clip 事件）；
  状态机与 clip 时间轴共享同一播放/暂停状态；
- 失败安全：未初始化时 set_state 返回 -2、snapshot 返回 `initialized=false`；坏资产初始化返回 -1 只写日志。

快照示例：

```json
{
  "initialized": true,
  "path": "assets/models/hero.gltf",
  "has_skeleton": true,
  "current_state": "Idle",
  "current_index": 0,
  "current_time": 0.0,
  "normalized_time": 0.0,
  "transitioning": false,
  "transition_alpha": 1.0,
  "from_state": null,
  "to_state": null,
  "event_count": 0,
  "states": [
    { "name": "Idle", "has_clip": true, "duration": 1.0 },
    { "name": "Run", "has_clip": false, "duration": 0.0 },
    "... 共 9 个"
  ]
}
```

## 3. 编辑器 UI（资产面板 →「动画」Tab）

- 原有资产路径/加载/浏览、clip 下拉、时间轴 Slider、播放/暂停/回到起点保持不变；
- 新增 **9 状态机** 区：
  - 3×3 状态按钮（Idle/Run/Jump/Attack/Hit/Death/Land/Dodge/Block）；未初始化时点击只提示先初始化；
  - **初始化状态图**（用当前 glTF 路径 + clip 0 兜底）、**释放状态图**；
  - 「状态: …」「过渡 alpha: …」每帧从 snapshot 刷新（稳态显示 `1（稳态）`）；
  - 状态栏提示初始化结果（含 skeleton 是否存在）/切换结果/失败原因；
- 播放时间轴即驱动状态机（`update(dt)`）；无 clip/骨架时初始化仍成功并显示 `skeleton=false`/`has_clip=false` 原因。

## 4. 测试证据

- Rust 单元测试（`cargo test -p hezhou-examples --bin mono_editor_demo --features mono animation_ffi`）：
  **5 passed**（新增 1 项覆盖：9 状态映射/大小写不敏感/Idle 回退/空 clips 安全、过渡 alpha 0→(0,1)→1、Death 终止状态禁止切出、未初始化 snapshot/set_state 安全）。
- 编辑器启动自检：**PASS=96, FAIL=0**（task-45 的 91 + 状态机 5 项），证据（`engine/logs/hezhou_2026-11-08.log` 行 2195-2205）：
  - `9 状态图初始化 ... 绑定 clip 1/9，skeleton=true`
  - 快照含 9 状态/current/skeleton（len=780）
  - `切换到 Run 返回 0`、`过渡中 snapshot transitioning=true`、`释放状态图后 snapshot initialized=false`
  - 自检 fixture 写入 `assets/__ffi_anim_sm_selftest__.gltf`（内嵌 base64），结束删除并重新扫描（无残留）。
- 契约 **334/334/334 OK**；`build_mono.ps1` exit 0（DLL 410,624 字节 / 401 KB）；
  `ci_local.ps1 -SkipBuild` 通过（需可写 TMP/TEMP 供 glfw-sys CMake/MSBuild）。

## 5. 限制 / 后续

1. **无角色渲染**：不采样姿态/蒙皮，仅预览状态与过渡 alpha。
2. **无混合树/参数条件**：切换由 UI 按钮显式触发；无 speed/loop 曲线编辑、无过渡时长配置 UI。
3. **无事件轨**：状态机事件只计数（snapshot.event_count），未展示/接线。
4. **clip 通道未构建**：仅用时长绑定（本轮无采样）；真实姿态预览需接入 animation 运行时与蒙皮上传。
5. 状态- clip 映射固定按名称，不支持自定义映射表；无 profile/配置文件保存。
