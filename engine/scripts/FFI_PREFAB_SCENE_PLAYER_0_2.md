# 0.2 Prefab / SceneManager / Player 导出编辑器 FFI（task-21）

本文档记录编辑器侧的 **Prefab 捕获/实例化**、**场景槽切换（SceneManager）**、
**Player 导出 dry-run** 的 FFI 契约与 UI 用法。与 `FFI_RENDER_0_2.md`（task-15）、
`FFI_AUDIO_ASSET_0_2.md`（task-16）配套：所有 FFI 以 **FfiContext tail 追加**，
`engine/scripts/check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT = 313` 做字段数守卫。

| 层 | 文件 |
|---|---|
| Rust 类型别名 | `engine/scripting/src/ffi_context/prefab_scene_player.rs` |
| Rust FFI 实现 | `engine/examples/src/editor/prefab_scene_player_ffi.rs` |
| Rust 装配/启动 | `engine/examples/src/editor/mod.rs` |
| C# delegate + IntPtr + static 包装 | `engine/scripts/UI.cs` |
| C# UI（资产面板「工程」Tab） | `engine/scripts/EditorScript.{View,State}.cs`、`EditorScript.cs` |
| 契约校验 | `engine/scripts/check_ffi_contract.py` |
| 依赖 | `core`（Prefab/SceneManager/Project）、`hezhou-player`（PlayerBuilder/CookManifest） |

## 1. Prefab FFI（2）

| FFI | 签名 | 语义 |
|---|---|---|
| `prefab_capture_from_scene` | `fn(scene, entity_id, name, path) -> i32` | `Prefab::capture_from_scene` + `save_file`；0=成功，-1=实体/场景无效，-2=保存失败 |
| `prefab_instantiate` | `fn(scene, path, parent_id, x, y, z) -> u64` | `Prefab::load_file` + `Scene::instantiate_prefab_with`（位置覆盖）；返回新根实体 ID，0=失败 |

- 捕获会展开嵌套实例为普通实体（0.2 语义，来源信息不写入工程文件）。
- 实例化后由 C# 刷新层级树/属性面板并选中新根实体；不修改 Project 格式。

## 2. SceneManager FFI（7）

| FFI | 签名 | 语义 |
|---|---|---|
| `scene_manager_save_current` | `fn(name) -> i32` | 当前编辑场景快照写入命名场景槽（不存在创建/存在替换）；0 成功 |
| `scene_manager_load_file` | `fn(name, path) -> i32` | 读取 Project JSON 文件注册为场景槽（不切换） |
| `scene_manager_switch` | `fn(name) -> i32` | 先把当前编辑状态写回当前槽，再把目标槽恢复到编辑场景；0 成功，-2=当前场景尚未保存为槽 |
| `scene_manager_unload` | `fn(name) -> i32` | 卸载场景槽；0 成功，-1 未找到 |
| `scene_manager_count` | `fn() -> i32` | 场景槽数量 |
| `scene_manager_current_name` | `fn(buf, len) -> usize` | 当前槽名（空串表示无） |
| `scene_manager_names` | `fn(buf, len) -> usize` | 全部槽名（NUL 分隔、排序） |

实现要点（`SceneManager` 与编辑器指针稳定）：

- 编辑器始终编辑同一个 `Scene` 对象（`SCENE` 指针不变）；场景槽内容通过
  `Project::snapshot_to_json` / `restore_from_json` 在**同一对象内替换**，
  因此渲染器/C# 不需要更新场景指针；
- 切走前自动把当前编辑状态写回当前槽（`snapshot_to_json` 重新注册）；
- 首次切换要求先「保存当前场景」（避免丢失未保存编辑），`-2` 时 UI 会提示。

## 3. Player 导出 FFI（dry-run + 真实构建，3）

| FFI | 签名 | 语义 |
|---|---|---|
| `player_export_dry_run` | `fn(request_json) -> i32` | 生成 Windows + Linux 双平台 dry-run 计划；0 成功，-1 失败（失败原因写入摘要） |
| `player_export_summary` | `fn(buf, len) -> usize` | 最近一次导出摘要（多行文本） |

请求 JSON（编辑器自动构造）：

```json
{
  "project_root": "<工程/工作目录>",
  "output_dir": "<工程>/logs/player_dry_run",
  "product_name": "<Project.GetName() 或 HezhouGame>",
  "version": "0.1.0",
  "start_scene": "scenes/main.scene.json",
  "scripts_dir": "<root>/scripts",
  "runtime_dir": "<root>/runtime/mono",
  "required_font": "fonts/HarmonyOS_Sans_SC.ttf"
}
```

构建链：`AssetDatabase` 文件资产 → `hezhou_player::CookManifest`（相对 `assets/` 路径 +
size/checksum/asset_id）→ `PlayerBuildInput`（脚本目录 + 运行时目录）→
`PlayerBuilder::plan_both_targets()` → 每平台 `PlayerBuildPlan::write_plan`。

输出布局（dry-run 不复制二进制）：

```text
<output_dir>/
├── export_summary.txt
├── windows/
│   ├── manifest.json     # PlayerManifest（启动场景/窗口/质量档/资产清单）
│   ├── plan.json         # PlayerBuildPlan（文件清单/警告/fingerprint）
│   └── file-list.txt
└── linux/
    └── ...（同上）
```

- 启动场景/字体在 AssetDatabase 中不存在时会生成"合成资产条目"进入计划（`synthetic_count`），
  便于在真实场景资产接入前验证整条链；C# 状态栏/Console 会展示警告码
  （`runtime_payload_empty` / `scripts_empty` / `linux_rpath_required` 等）。
- dry-run **不复制二进制**；真实构建见 §3.1（`player_export_build`）。

### 3.1 真实构建 `player_export_build`（task-25，新增）

| FFI | 签名 | 语义 |
|---|---|---|
| `player_export_build` | `fn(request_json) -> i32` | Cooker + `PlayerBuilder::build` 复制当前平台产物并跑 `--validate`；0 成功（validate 通过），-1 失败（摘要含原因，半成品已清理） |

请求 JSON 与 dry-run 相同，额外字段：

```json
{ "executable_path": "<hezhou-player 可执行文件；空=自动查找编辑器同目录>" }
```

构建链（真实复制）：

1. 以 `project_root` 独立打开 `AssetDatabase`（支持 fixture/非当前工程）并扫描；
2. 启动场景：`start_scene` 显式优先，否则取第一个 `AssetKind::Scene` 资产（记录 locator `assets/<start_scene>`）；
3. 字体：`required_font` 指定的资产不存在时自动关闭必检并告警（避免无字体工程无法构建）；
4. `Cooker::plan` + `Cooker::cook_to_dir(<output>/cooked)` 生成 cook manifest/`imported/`/`assets/`；
5. `CookedAssets::load` → `PlayerBuildInput::with_cooked`，脚本目录 `ScriptAssembly::from_dir`、运行时目录 `RuntimePayload::from_dir`（缺失则警告）；
6. `PlayerBuilder::build(TargetPlatform::host(), BuildSources::new(exe, assets_root), <output>/<platform>)`：
   staging → 预检 → 复制 → 校验 → 原子提交；
7. 用**产物内**可执行文件执行 `--validate --player-root <pkg> --manifest <manifest>`，
   失败则返回 -1 并清理 `target/` 与 `cooked/`；成功摘要包含 validate 输出。

输出布局：

```text
<output_dir>/                     # 默认 <工程>/exports/player
├── cooked/                       # Cooker 输出（cook_manifest.json + imported/ + assets/）
└── windows/ 或 linux/            # 仅当前平台（TargetPlatform::host()）
    ├── <executable>.exe / 无后缀
    ├── manifest.json
    ├── assets/（cook 产物 + cook_manifest.json）
    ├── scripts/（预编译 C# DLL）
    └── runtime/（存在时复制）
```

限制：

- **只构建当前平台**：另一平台需在对应平台运行构建（`BuildError::CrossPlatformBuildUnsupported`），
  摘要中会注明；
- Player 可执行文件默认查找编辑器可执行文件同目录的 `hezhou-player[.exe]`（开发环境
  `cargo build -p hezhou-player` 产物）；安装版可按请求 `executable_path` 指定；
- 真实 C# 程序集/运行时需由工程提供（`scripts/bin/Mono`、`runtime/mono`）；缺失时只产生 warning，
  包仍生成但运行依赖系统 Mono（见 `runtime_payload_empty` / `scripts_empty`）；
- 真实构建同步执行（fixture 约 1~2 秒），构建期间编辑器 UI 不刷新；大工程可后续搬到后台线程。

## 4. 编辑器 UI 用法

资产面板 →「工程」Tab 新增：

| 按钮 | 行为 | 成功提示 |
|---|---|---|
| 保存当前场景 | 保存为当前槽名；首次为 `main` | 状态栏「场景已保存: main」 |
| 切换场景 | 按名字排序循环切换到下一个槽 | 层级/属性面板刷新，状态栏「切换场景: xxx」 |
| 卸载当前场景 | 卸载当前槽（编辑内容保留，提示已卸载） | 「已卸载场景槽: xxx」 |
| 加载场景文件 | FileBrowser 选择 Project JSON → 注册槽并切换（首次自动把当前场景存为 `main`） | 「加载并切换场景: xxx」 |
| 捕获选中为 Prefab | 当前选中实体 → `assets/prefabs/prefab_<id>_<hhmmss>.prefab.json` | 「Prefab 已保存: …」 |
| 实例化 Prefab | FileBrowser 选择 `.prefab.json` → 实例化到根/位置 (0,0,0) 并选中 | 「Prefab 实例化: root=…」 |
| 导出 Player（DryRun） | 生成双平台计划到 `logs/player_dry_run` | 「Player dry-run 完成」 |
| 导出 Player（Build） | 真实构建当前平台到 `exports/player/<platform>/`（Cooker + build + validate） | 「Player 真实构建 完成」/「失败（详见 Console）」 |

- 场景槽状态标签：`场景: <当前槽>  槽=N`；导出摘要显示在「Player 导出: …」标签。
- 所有失败只写 Console 日志 + 状态栏提示，不崩溃。

## 5. 启动自检（真实运行证据）

`TestRunner.TestPrefabScenePlayerFfi()` / `TestPlayerBuildFfi()` 在 C# Initialize 阶段运行：
Prefab 捕获→实例化→删除新增实体、SceneManager 保存/切换/卸载、Player 双平台 dry-run 计划生成，
以及 logs/ 下最小 fixture 工程的 **真实构建 + `--validate`**。

2026-11-01 本机冒烟（debug 编辑器）：

```
[Prefab] 捕获 Prefab 成功: __ffi_selftest_prefab（entity=1）-> logs\...prefab.json
[Prefab] 实例化 Prefab 成功: ...（root=2, entities=1, parent=0）
[SceneManager] 保存场景槽: __ffi_selftest_scene__（实体 1，场景槽 1）
[SceneManager] 切换场景: __ffi_selftest_scene__ -> __ffi_selftest_scene__（实体数 1）
[Player] Player 导出 dry-run 完成:
  资产: 28（合成条目 2）  [windows] 文件 30 ...  [linux] 文件 30 ...
[Player] 真实构建完成: 8 个文件，2005340 B -> ...\logs\player_build_selftest\windows
[Player] Player 真实构建成功:
  validate: 通过（hezhoubuildselftest.exe --validate）
  hezhou-player 0.1.0 启动检查通过：HezhouBuildSelfTest v0.1.0（Windows，质量 high）；资产索引 3 条
[TestRunner] ===== 测试完成: PASS=71, FAIL=0 =====
```

真实构建自检 fixture（`logs/player_build_fixture/`，自检后清理）：
`assets/scenes/main.scene` + `assets/fonts/HarmonyOS_Sans_SC.ttf` + `assets/audio/test.wav` + `runtime/README.txt`，
经 `player_export_build` 生成 `logs/player_build_selftest/windows/` 并由产物内 exe 执行 `--validate` 通过。
（fixture 场景内容是占位 JSON，validate 会回退内置测试场景但不影响启动检查；真实工程应放有效 Project JSON。）

## 6. 剩余 TODO

1. **Player 真实构建的平台/CI 固化**：在 Windows/Linux CI 各自构建 `hezhou-player` 产物并跑
   `player_export_build` + `--validate`；编辑器侧可加构建配置面板（目标平台/质量档/启动场景选择）。
2. **Player runtime host 集成**：导出后“运行 Player”冒烟（当前 validate 只做启动检查，不进游戏循环）。
3. **Prefab 嵌套/覆盖 UI**：`PrefabRegistry` 已由 core 提供，编辑器需要注册表加载嵌套 Prefab
   与 override（位置/名称/可见性）编辑面板。
4. **场景槽下拉/快捷键**：当前为循环切换按钮，后续用 Dropdown（UI.ComplexWidgets）与
   Ctrl+Shift+S 等快捷键；场景槽持久化（当前仅运行时内存）。
5. **Prefab 资产入库**：捕获的 Prefab JSON 目前写到 `assets/prefabs/`，尚未自动注册到
   AssetDatabase（可调用 `asset_scan` 后作为普通 JSON 资产）。
6. **真实构建后台化**：大工程同步构建会阻塞 UI；可把 `player_export_build` 放到后台线程 + 进度轮询
   （FFI 只发起/查询，参考 FFI_AUDIO_ASSET_0_2.md 的 build 接入点）。

新增 FFI 时必须沿用 tail 追加顺序、同步 C# delegate/IntPtr、更新
`check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT`，并跑
`cargo test -p hezhou-core`、`check_ffi_contract.py`、`build_mono.ps1`、
`ci_local.ps1 -SkipBuild` 与编辑器启动冒烟。


