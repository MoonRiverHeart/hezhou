# 编辑器动画时间轴（第一批，0.3 task-43）

glTF/GLB 资产 → `hezhou-asset` 导入 → 浏览 clip/骨骼 → 时间轴拖动/播放（本轮无角色渲染）。

| 层 | 文件 |
|---|---|
| 查询/缓存/预览状态 + FFI | `engine/examples/src/editor/animation_ffi.rs`（新增） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail，基线 **329**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}`（资产面板「动画」Tab） |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 329`） |

## 1. 数据来源

- 调用 `hezhou_asset::import::gltf::import_gltf_file(path, &GltfImportSettings::default())`
  得到 `ImportedScene`（asset crate 公开 API，未修改 asset 源码）；
- 提取：
  - `animations[]` → clip 名称（缺省 `clip_<index>`）、时长、通道数；
  - `skeletons[]` → 骨架名、joint 数、root joint（summary 同时给 `skeleton` 首项与 `skeletons[]` 列表）；
  - 导入 warnings 一并返回；
- 查询缓存：按「绝对路径 + mtime」缓存 summary JSON 与 clip 列表；文件修改后自动失效。

## 2. FFI（新增 7，FfiContext 329）

| FFI | 签名 | 语义 |
|---|---|---|
| `animation_asset_summary` | `fn(path, buf, len) -> usize` | 查询摘要 JSON；坏资产/缺文件返回 `{"error":...}`，不崩溃 |
| `animation_preview_tick` | `fn(dt, playing) -> f64` | 每帧驱动时间（playing>=0 时设置播放状态），返回当前秒 |
| `animation_preview_set_clip` | `fn(index) -> i32` | 选择 clip，0=成功，越界 -1；时间归零 |
| `animation_preview_set_time` | `fn(t) -> f64` | clamp 到 `[0,duration]`，返回实际时间 |
| `animation_preview_play` | `fn() -> i32` | 开始播放 |
| `animation_preview_pause` | `fn() -> i32` | 暂停 |
| `animation_preview_snapshot` | `fn(buf, len) -> usize` | 状态 JSON：playing/time/duration/clip_index/clip_count/clip_name |

summary JSON 示例：

```json
{
  "path": "assets/models/hero.gltf",
  "skeleton": { "name": "HeroRig", "joint_count": 24 },
  "skeletons": [ { "index": 0, "name": "HeroRig", "joint_count": 24, "root_joint": 0 } ],
  "clips": [ { "index": 0, "name": "Idle", "duration": 2.0, "channels": 12 } ],
  "clip_count": 1,
  "warnings": []
}
```

- 时间推进按 `dt` 累加并在 `duration` 上循环；`duration == 0` 时保持 0。
- 所有函数 null/越界/坏值安全（失败只日志/错误 JSON）。

## 3. 编辑器 UI（资产面板 →「动画」Tab）

- **资产路径**输入框 + 「加载」+「浏览…」（FileBrowser 选择 `.gltf`；选中后自动加载）；
- **clip 下拉**：summary 中 clips 名称列表，切换即 `animation_preview_set_clip` 并重置时间轴；
- **时间轴 Slider**：范围 `0..duration`，拖动 → `animation_preview_set_time`（暂停）；
- **播放 / 暂停 / 回到起点** 按钮；时间标签 `时间: t / duration s`；
- 状态标签显示 `clip 数 / skeleton joints / 时长`，或加载失败原因；
- 播放由编辑器 `Update` 每帧驱动（`AnimationPreviewTick(deltaTime, playing)`），
  不阻塞 UI；暂停/拖动后保持当前时间。

## 4. 测试证据

- Rust 单元测试（4 项，`cargo test -p hezhou-examples --bin mono_editor_demo --features mono animation_ffi`）：
  - summary 含 skeleton/clips（ImportedScene 构造）；
  - 预览状态机（clip 选择/播放/暂停/时间 clamp）；
  - 空路径/无 clip 安全；
  - **真实最小 glTF fixture**（内嵌 animation+skin，`import_gltf_file` 成功，clip=Idle/1.0s、joint=1）。
- 编辑器启动自检：**PASS=88, FAIL=0**（task-41 的 84 + 动画 4 项）：
  - 缺失资产返回安全错误 JSON（len=139）
  - `Play` 后 snapshot `playing=true`、`Pause` 后 `playing=false`
  - 无 clip 时 `set_time` clamp 到 0
- 契约 **329/329/329 OK**；`build_mono.ps1` exit 0（DLL 396,800 字节 / 387.5 KB）；
  `ci_local.ps1 -SkipBuild` **OK**（需可写 TMP/TEMP 供 glfw-sys CMake/MSBuild）。

## 5. 限制 / 后续

1. **本轮无角色渲染**：只浏览 clip/时间轴，不驱动蒙皮；渲染接入留给后续（可用 renderer 的
   `upload_skin_palette`/`set_entity_pbr_material` 等接口）。
2. 无混合树/状态机/事件轨；无关键帧可视化编辑；无 onion skin。
3. 时间轴为单 clip 线性播放，不支持 root motion 提取/循环区间设置。
4. 资产选择依赖路径输入/FileBrowser；未与资产面板双击/拖拽自动关联。
5. 缓存按路径+mtime，不做文件 watcher；外部修改后点「加载」重新读取。
