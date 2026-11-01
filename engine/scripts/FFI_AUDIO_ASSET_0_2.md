# 0.2 音频 / 资产数据库 编辑器 FFI 接入（task-16）

本文档记录「音频引擎 / 资产数据库 → 编辑器 C#」的 FFI 握手、UI 用法与后续扩展点。
与 `FFI_RENDER_0_2.md`（task-15 画质/PBR）配套：所有 FFI 均以 **FfiContext tail 追加** 方式新增，
`engine/scripts/check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT = 301` 做字段数守卫。

| 层 | 文件 |
|---|---|
| Rust 类型别名 | `engine/scripting/src/ffi_context/audio_asset.rs` |
| Rust FfiContext 字段（tail，顺序 = C# 一致） | `engine/scripting/src/ffi_context/mod.rs` |
| Rust FFI 实现 | `engine/examples/src/editor/audio_asset_ffi.rs` |
| Rust 装配 / 启动 / 每帧推进 / 退出 | `engine/examples/src/editor/mod.rs` |
| C# delegate + IntPtr 字段 + static 包装 | `engine/scripts/UI.cs` |
| C# 工具栏 / 工程资产面板 / 状态栏 | `engine/scripts/EditorScript.{View,State}.cs`、`EditorScript.cs` |
| 契约校验 | `engine/scripts/check_ffi_contract.py` |

## 1. 音频 FFI（9 个字段）

| FFI | 签名 | 语义 |
|---|---|---|
| `audio_init` | `fn() -> i32` | 创建 `AudioEngine`（`default_backend()`；无设备自动降级 Null）；幂等，0 成功 |
| `audio_shutdown` | `fn() -> i32` | 关闭后端并销毁引擎 |
| `audio_play_first_asset` | `fn(i32) -> i32` | 播放工程内第一个音频资产（WAV/OGG）；无音频资产时播放内置 440Hz 测试音；0 成功 |
| `audio_pause` / `audio_resume` / `audio_stop` | `fn() -> i32` | 操作最近一次播放的声部；0 成功，-1 无活动声部 |
| `audio_set_master_volume` | `fn(f32) -> i32` | 主音量（内部 clamp 到 [0,1]） |
| `audio_set_listener` | `fn(f32,f32,f32) -> i32` | 设置 3D 监听者位置（编辑器每帧传相机位置） |
| `audio_get_summary` | `fn(*mut c_char, usize) -> usize` | 摘要：`audio=on backend=cpal mode=Device silent=false sample_rate=48000 master=0.80 voices=0 cached=1 last_voice=-` |

实现要点：

- `engine/examples/Cargo.toml` 启用 `hezhou-audio` 的 `cpal-backend` + `ogg` feature：
  有设备时真实输出；无设备/打开失败自动降级为无声（`EngineStatus::is_silent()`），
  播放/暂停状态机照常；WAV 内置解码器 + OGG/lewton 均可用。
- `AudioEngine::update(dt)` 在编辑器主循环每帧调用（`audio_asset_ffi::tick`）；
  真实设备后端内部自动跳过，不会双倍推进。
- 失败只写 `dfx_warn!/dfx_error!` 并返回错误码，不 panic。

## 2. 资产数据库 FFI（6 个字段）

| FFI | 签名 | 语义 |
|---|---|---|
| `asset_open_project` | `fn(*const c_char) -> i32` | 以路径为工程根打开 `AssetDatabase` 并扫描；空指针/空串=当前工作目录 |
| `asset_scan` | `fn() -> i32` | 重新扫描当前工程；0 成功，-1 未打开 |
| `asset_import_gltf` | `fn(*const c_char) -> i32` | 导入 glTF/GLB（绝对路径或工程相对逻辑路径；未扫描的新文件在 `assets/` 内会自动登记 `.meta`）；0 成功，-1 未打开工程，-2 无法登记/路径不在库，-3 导入失败，-4 路径非法 |
| `asset_count` | `fn() -> i32` | 已注册资产数（文件资产 + 子资产） |
| `asset_get_summary` | `fn(*mut c_char, usize) -> usize` | `project=… assets=26 sub_assets=0 gltf=0 warnings=0` |
| `asset_get_asset` | `fn(i32, *mut c_char, usize, *mut c_char, usize) -> i32` | 按索引查询 `AssetId`（UUID）与定位符（`assets/x.gltf#mesh:0`）；0 成功 |

实现要点：

- 启动时 `audio_asset_ffi::startup_init()` 以当前工作目录为工程根自动扫描（安装版工作目录
  已是用户数据目录，`assets/` 默认存在）；失败只日志，不阻塞编辑器。
- 导入使用 `AssetDatabase::import_gltf`：写 `.meta`、注册 Mesh/Material/Texture/Skeleton/Animation
  子资产、建立依赖图（与 asset crate 的 cook 契约一致）。
- 资产列表按 `BTreeMap<AssetId>` 顺序返回，确定可复现。

## 3. 编辑器 UI 用法

- **工具栏「音频测试」**：未播放时加载/播放工程内第一个音频资产（无则内置测试音），
  播放中再次点击停止；状态栏显示 `音频: ON/cpal` / `OFF`。
- **资产面板 →「工程」Tab**：显示 AssetDatabase 摘要，提供：
  - 「扫描工程资产」按钮（重新扫描，结果显示在标签与 Console）；
  - 「导入 glTF/GLB」按钮（文件浏览器选择 `.gltf/.glb` → `asset_import_gltf` → 重新扫描）。
- **状态栏**：`资产: assets=…`、`音频: ON/backend`，每帧刷新；音频监听者跟随编辑器相机
  （`UI.AudioSetListener(_cameraX, _cameraY, _cameraZ)`）。
- 所有失败只写 Console 日志 + 状态栏提示；默认工作流不崩溃。

## 4. 启动自检（真实运行证据）

`TestRunner.TestAudioAssetFfi()` 在 C# Initialize 阶段运行，覆盖：
`AssetScan`、资产摘要、`AssetCount`、`AssetGetAsset(0)` 的 AssetId/定位符、
临时最小 glTF 的 `AssetImportGltf` 导入 + 清理、音频引擎就绪、`AudioInit` 幂等、
`AudioPlayFirstAsset` / `Pause` / `Resume` / `Stop` / `SetMasterVolume` / `SetListener`。

2026-11-01 本机冒烟（debug 编辑器，真实 cpal 设备）：

```
[Audio] 音频后端就绪: cpal @ 48000Hz
[Asset] 资产数据库就绪: project=…\engine assets=26 sub_assets=0 gltf=0 warnings=0
[Asset] glTF 导入成功: assets/__ffi_selftest__.gltf | mesh=1 material=1 texture=0 skeleton=0 animation=0\n[Audio] 工程内无音频资产，播放内置 440Hz 测试音
[TestRunner] ===== 测试完成: PASS=57, FAIL=0 =====
```

## 5. 后续 build/export 接入点（示意，不在本任务范围）

| 能力 | 建议 FFI | 说明 |
|---|---|---|
| build/export | `build_export_player(config_json_ptr) -> i32`、`build_get_progress(buf,len) -> usize`、`build_cancel() -> i32` | `hezhou-player` 构建器；FFI 只发起/轮询，长任务放后台线程 |
| 资产 cook | `asset_cook(roots_json_ptr) -> i32`、`asset_cook_summary(buf,len)` | 复用 `hezhou_asset::Cooker`；编辑器「构建」面板调用 |
| 音频预览增强 | `audio_set_bus_volume(bus_ptr, f32)`、`audio_play_3d(path_ptr, x,y,z)` | 现有 AudioEngine API 已具备，按需追加 FFI 字段 |
| 构建产物 | `build_get_manifest(buf,len)` | 输出 Player 目录/manifest 路径，供状态栏与日志展示 |

新增 FFI 时必须沿用 tail 追加顺序、同步 C# delegate/IntPtr、更新
`check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT`，并跑
`cargo test -p hezhou-core`、`python engine/scripts/check_ffi_contract.py`、
`build_mono.ps1`、`ci_local.ps1 -SkipBuild` 与编辑器启动冒烟。

