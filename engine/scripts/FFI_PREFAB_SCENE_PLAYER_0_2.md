# 0.2 Prefab / SceneManager / Player 导出编辑器 FFI（task-21）

本文档记录编辑器侧的 **Prefab 捕获/实例化**、**场景槽切换（SceneManager）**、
**Player 导出 dry-run** 的 FFI 契约与 UI 用法。与 `FFI_RENDER_0_2.md`（task-15）、
`FFI_AUDIO_ASSET_0_2.md`（task-16）配套：所有 FFI 以 **FfiContext tail 追加**，
`engine/scripts/check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT = 312` 做字段数守卫。

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

## 3. Player 导出 dry-run FFI（2）

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
- **不做真实构建**：不复制 exe/assets/scripts/runtime；运行时 host 与真实构建在后续任务实现。

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

- 场景槽状态标签：`场景: <当前槽>  槽=N`；导出摘要显示在「Player 导出: …」标签。
- 所有失败只写 Console 日志 + 状态栏提示，不崩溃。

## 5. 启动自检（真实运行证据）

`TestRunner.TestPrefabScenePlayerFfi()` 在 C# Initialize 阶段运行：
Prefab 捕获→实例化→删除新增实体、SceneManager 保存/切换/卸载、Player 双平台 dry-run 计划生成。

2026-11-01 本机冒烟（debug 编辑器）：

```
[Prefab] 捕获 Prefab 成功: __ffi_selftest_prefab（entity=1）-> logs\...prefab.json
[Prefab] 实例化 Prefab 成功: ...（root=2, entities=1, parent=0）
[SceneManager] 保存场景槽: __ffi_selftest_scene__（实体 1，场景槽 1）
[SceneManager] 切换场景: __ffi_selftest_scene__ -> __ffi_selftest_scene__（实体数 1）
[Player] Player 导出 dry-run 完成:
  资产: 28（合成条目 2）  [windows] 文件 30 ...  [linux] 文件 30 ...
[TestRunner] ===== 测试完成: PASS=68, FAIL=0 =====
```

## 6. 剩余 TODO

1. **Player 真实构建**：复制 exe/assets/scripts/runtime，生成可启动产物；`hezhou-player`
   `build()` 已存在，但编辑器侧需要构建配置面板（目标平台/质量档/启动场景选择）与进度反馈。
2. **Player runtime host 集成**：编辑器“运行 Player”/导出后冒烟（当前 dry-run 不启动 host）。
3. **Prefab 嵌套/覆盖 UI**：`PrefabRegistry` 已由 core 提供，编辑器需要注册表加载嵌套 Prefab
   与 override（位置/名称/可见性）编辑面板。
4. **场景槽下拉/快捷键**：当前为循环切换按钮，后续用 Dropdown（UI.ComplexWidgets）与
   Ctrl+Shift+S 等快捷键；场景槽持久化（当前仅运行时内存）。
5. **Prefab 资产入库**：捕获的 Prefab JSON 目前写到 `assets/prefabs/`，尚未自动注册到
   AssetDatabase（可调用 `asset_scan` 后作为普通 JSON 资产）。

新增 FFI 时必须沿用 tail 追加顺序、同步 C# delegate/IntPtr、更新
`check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT`，并跑
`cargo test -p hezhou-core`、`check_ffi_contract.py`、`build_mono.ps1`、
`ci_local.ps1 -SkipBuild` 与编辑器启动冒烟。
