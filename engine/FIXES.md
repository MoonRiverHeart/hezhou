# Bug Fixes Log

## 2026-10-18: 状态栏 v2 + CI 环境固化

- 状态栏新增：选择数量、主题、错误数；选择/范围选择/主题切换时即时更新。
- 修复自动测试模式下状态栏不刷新（原先只在正常 Update 更新）。
- CI 固化：
  - Rust 1.95.0、Mono 6.12.0.199、Python 3.11
  - `CMAKE_GENERATOR=Visual Studio 17 2022`、`CMAKE_BUILD_PARALLEL_LEVEL=1`
  - `Swatinem/rust-cache`
  - 图标生成器一致性检查 `gen_icons.py` + `git diff --exit-code`
  - `workflow_dispatch`、并发取消、超时限制


## 2026-10-18: Command Palette（Ctrl+Shift+P）

- 新增 `EditorScript.CommandPalette.cs`：Cmd/Ctrl+Shift+P 打开，Esc 关闭。
- 支持输入过滤；命令覆盖新建/保存/撤销/重做/运行暂停/Console/主题/Gallery/
  最大化属性面板/重置 Dock/打开项目。
- 遮罩层 + 居中卡片，非阻塞编辑器状态。
- intent `intent.command_palette`（11/11）；visual `14_command_palette`、
  `15_palette_closed`，总计 15 张。


## 2026-10-18: Shift 范围多选（Selection v1）

- Hierarchy 维护当前可见实体顺序 `_entityDisplayOrder`。
- 记录 Shift/Ctrl 修饰键与选择锚点 `_selectionAnchorEntityId`。
- Shift+点击树节点：按可见顺序选中 anchor..target，同步 Rust selection、C# 多选列表、
  TreeView 高亮与属性面板。
- 新增 `intent.shift_range_select`（10/10）与 visual `13_shift_range_select.png`。
- 多选 outline 渲染仍待后续验证加粗/描边颜色。


## 2026-10-18: Dock v1 — 布局持久化与面板最大化

- 新增 `EditorScript.Dock.cs`：保存/恢复 outer/leftCenter/inner SplitView 比例、
  属性 Tab、资产 Tab、右侧面板最大化状态（`.hezhou_layout`）。
- 右侧面板标题栏增加最大化/还原按钮；最大化时右栏铺满工作区。
- 主题切换、窗口 resize 后重新应用 Dock 布局。
- visual 新增 `11_right_panel_max.png`、`12_dock_restored.png`，总计 12 张。
- 当前 Dock v1 范围：比例持久化 + 最大化/恢复；Tab 拖拽与跨区域停靠留到 v2。


## 2026-10-18: P0 Widget Gallery（双主题控件基线）

- 新增 `EditorScript.WidgetGallery.cs`：整页显示 Buttons/Inputs/Icons/Tabs/Theme Colors。
- Gallery 隐藏主 SplitView/状态栏，关闭后恢复，不影响 Scene 与选中实体。
- visual 新增 `08_widget_gallery_dark.png`、`09_widget_gallery_light.png`、
  `10_editor_restored.png`，总计 10 张。
- 修复下拉框中文选项显示 `??`：Gallery 使用英文分类项。
- 控件画廊用于后续控件状态与布局回归。


## 2026-10-18: SVG 图标系统（P0）

- 新增 `engine/assets/icons/*.svg`（24 个线性图标，16×16 viewBox）。
- `engine/scripts/gen_icons.py` 解析 line/polyline/polygon/rect/circle，生成：
  - `ui/src/icons.rs`：归一化线段表
  - `scripts/IconIds.cs`：稳定 IconId 枚举
- 新增 Rust `IconWidget`：按布局缩放、主题着色、任意 DPI 矢量绘制。
- FFI：`ui_create_icon_in_parent`、`ui_icon_set_color`、`ui_icon_count`。
- 编辑器工具栏、预览标题、属性标题已接入图标。
- FFI 契约 274 / 274 / 274 OK。


## 2026-10-18: P0 双主题 Design Tokens + 视觉基线

### Added
- Rust `ui/src/theme.rs`：Dark/Light 两套语义色板、`ThemeRole`、全局主题模式。
- FFI：`ui_theme_get_mode/set_mode/color/role_count`、`ui_widget_set_text_color`、
  `ui_clear_widget_tree_keep_callbacks`。
- C# `UI.Theme.cs`：`ThemeRole`/`ThemeColor`/`Theme`，C# 只使用语义角色。
- 工具栏“浅色/深色”切换按钮；主题写入 `.hezhou_theme`，重启恢复。
- `visual` 模式新增 `06_theme_light.png`、`07_theme_dark.png`，走真实按钮路径。

### Changed
- 基础控件默认色迁移到 Theme：Button/Label/InputField/Dropdown/TabWidget/
  TreeView/TreeNode/ScrollView/TextEdit/PopupMenu/GridView/Dialog/Panel/ListItem。
- 编辑器外壳 Panel/工具栏按钮/菜单/状态栏改用 `ThemeRole`。
- 属性页内容从 ScrollView 改为 VStack，规避“隐藏 Tab 首次显示时 ScrollView
  内容保持 0 尺寸”的布局缺陷。
- 主题切换使用 `ClearWidgetTreeKeepCallbacks` 重建 UI，保留回调、Scene、选中实体
  和当前属性 Tab。

### Verification
- FFI contract: 274 / 274 / 274 OK
- Visual: 7 / 7 screenshots OK（含 Dark/Light）

## 2026-10-18: UI Visual Regression Fixes

### Problems
0. 实际点击“绑定”没反应，而不是属性行渲染问题：
   - `TestRunner` 启动自检修改 `RotatingEntity.cs` 后把 `_sourceFilesModified` 留为 `true`
   - 因此每次绑定都触发 hot reload
   - hot reload 会重建 C# assembly，`_pendingBindEntityId` / `_selectedScriptIndex`
     静态字段被清零，`OnHotReloadComplete` 拿不到待绑定信息
   - 同时 reload 后 `_selectedEntityId` 也丢失，属性面板不会重建
1. `RotationScript.cs` 绑定后无脚本行/无属性：文件中的类是 `RotationController`，
   而绑定逻辑按文件名推导 className=`RotationScript`，类型注册表中不存在。
2. 资产面板搜索框被放进 ScrollView 直接子级，ScrollView 会把子节点拉伸到
   整个视口，输入框占满面板。
3. 属性面板 5 个 Tab 在 320px 宽度下显示成“几...”，Min/Max/Step/Init 行
   横向溢出、互相覆盖。
4. Console Dialog 使用 760x480 逻辑尺寸，超过 853x480 的逻辑屏幕，标题被裁剪。
5. 新建场景/加载工程后，旧脚本绑定 UI 变成“幽灵行”残留。

### Fixes
- 绑定改为**即时生效**：先 `AttachScriptBinding` + `CreateInstance` + 刷新属性面板，
  不再依赖 hot reload 的 pending 状态；只有 `.cs` 被编辑过才触发 reload
- `TestRunner` 源文件自检结束后清除 `_sourceFilesModified`，避免普通绑定被误判为需要重编译
- hot reload 不再重复 attach binding，改为 `reset_script_instance_ids()`；
  reload 后从 Rust Scene 恢复 `selected_entity_id` 并重建属性面板
- `RotationScript.cs` 新增逐实体 `RotationScript` 类（`[Expose]` 属性、
  CreateInstance/DestroyInstance/UpdateInstance/Get/SetPropertyValue），
  并注册到 `_scriptTypeRegistry`。
- 资产模型 Tab 使用 `ScrollView → VStack → InputField + GridView`，搜索框固定
  320x24，不再被 ScrollView 拉伸。
- 属性面板 Tab 宽度提升（outer split 0.72）；`TabWidget::truncate_text` 先判断
  完整文字是否放得下，避免“几何”被截成“几...”。
- Min/Max/Step/Init 改为两行布局（每行 40 label + 70 input）。
- Console Dialog 改为 680x360，TextEdit 250 高。
- `ClearScriptBindingUI` 按容器实际子节点清理，避免旧绑定行残留。
- 工具栏配色统一为深蓝灰 + 绿色运行 + 蓝色编辑器 + 橙色强调线；项目树加宽。

### Verification
新增 `-Mode visual`：输出 5 张关键 UI 截图，人工/工具检查后修复了
Tab 截断、配置行溢出、Console 越界、幽灵绑定、搜索框拉伸问题。

```text
test-results/visual/visual/
├── 01_rotation_script_binding.png   # RotationScript 行 + 属性可见
├── 02_console.png                   # 对话框完整、不裁剪
├── 03_asset_search.png              # 搜索框不再占满
├── 04_project_search.png
└── 05_multi_select.png
```

---

## 2026-10-18: Editor P1 — Multi-select Outline, Console Filters, Search, CI

### Added
- Additive scene selection API: `Scene::{add_to_selection,remove_from_selection,toggle_selection,get_selected_entity_ids}` + 3 FFI functions, so multi-select now drives the renderer's real outline list (not only primary selection).
- Multi-select delete removes all selected entities (single undo snapshot); confirmation dialog shows count.
- Undo/redo toolbar buttons with operation counts; `Ctrl+Z/Y` still works.
- Project tree search InputField; typing filters entities live (flat result list while searching).
- Asset panel model-tab search InputField; filters Cube/Sphere/Plane/Cylinder/CornellBox.
- Console panel:
  - live refresh every 30 frames while open
  - level filter `All/Info/Warn+/Error+`
  - module/message search
  - manual refresh button
- GitHub Actions `.github/workflows/ci.yml`:
  - Rust UT/UI/DFX
  - FFI contract
  - fuzz harness `cargo check`
  - C# build on Windows
- Core test for additive selection/toggle.
- Intent suite remains 7 cases and passes; random autotest 30/30.

### Verified
```text
cargo test -j 1 -p hezhou-core -p hezhou-ui --offline   PASS (14 tests)
python scripts/check_ffi_contract.py                    265/265/265 PASS
run_autotest.ps1 -Mode intent                            7/7 PASS
run_autotest.ps1 -Mode random 30 steps                   30/30 PASS, avg 7.12ms
```

---

## 2026-10-18: Layered Tests, DFX Automation, Editor P0

### Added
- **UT**: core scene/property/hierarchy/cycle tests; UI hit-test test; deterministic fuzz smoke.
- **FUZZ**: `engine/fuzz` cargo-fuzz `project_json` target + 3000-round malformed JSON smoke.
- **Interface**: FFI contract checker for all 264 FfiContext fields across Rust, C#, and editor assembly; FFI null/roundtrip tests.
- **E2E**: `scripts/run_autotest.ps1`, `UITestRunner.Report.cs`, `TestTelemetry.cs`
  - random/sequential/stress/intent modes
  - stable seed, interactive-widget filtering, periodic screenshots
  - `report.json`, `summary.txt`, `ui_tree.txt`, `final.png`, `DONE`
  - frame metrics, stalls, C# Warn/Error/Fatal telemetry
- **Intent**: `IntentTestRunner.cs` for new scene, create/select/delete, parent/child, undo/redo, multi-select batch edit, Console, run/pause.
- **P0 editor**:
  - Undo/redo via scene JSON snapshots (`Ctrl+Z/Y`), ID-preserving restore
  - Entity hierarchy UI (recursive tree, context menu attach/detach, cycle rejection, orphan promotion)
  - Multi-select (`Ctrl+Click`) + batch property edits
  - Console panel reading recent logs
  - C# script instance cleanup before scene replacement
- **DFX**: UI tree dump query-size support, dynamic buffer; C# log ring buffer.

### Verified
```text
scripts/run_tests.ps1 -Offline
  Rust UT/integration: PASS
  fuzz smoke: PASS
  FFI contract: 261/261/261 PASS
  random autotest: 30/30 PASS
  intent autotest: 7/7 PASS
  stress autotest: 79/79 PASS over 8s
```

---

## 2026-10-18: Project Save/Load Round-Trip + Editor File Shortcuts

### Problems
1. `Project::sync_to_scene()` appended entities to the current scene instead of
   replacing it, so loading a project duplicated the existing scene.
2. Directional lights and empty entities were saved as `"Cube"` and restored as
   cube meshes. Light direction/color/intensity and `visible` were not persisted.
3. `Project::load()` only accepted a directory and always appended
   `project.json`; the editor passed a `.json` file path, so Open Scene/Project
   failed.
4. `Ctrl+S`/`Ctrl+O`/`Ctrl+N` were displayed in menus but not handled in `OnKey`.
   “Save As” created a directory named `project.project.json` because the editor
   passed a file path to `Project::create_new()`.
5. Tree refresh code iterated `_entityNodeMap.Keys` while removing entries,
   which could throw `InvalidOperationException` when loading/new-scene.
6. Right-click “Delete Entity” bypassed the confirmation dialog and left stale
   selection/property state.

### Fixes
- Added `World::clear()` + `Scene::clear()`; project load now replaces the scene.
- Added `ProjectLight`, `visible` and `light` fields with serde defaults for
  backward compatibility with old project files.
- `sync_from_scene()` writes `"DirectionalLight"` / `"None"` for non-mesh
  entities and persists light + visibility data.
- `sync_to_scene()` recreates lights/empty entities/renderables and resets the
  scene to `Editing`.
- `Project::load()` accepts either a project directory or a direct JSON file;
  `Project` now tracks the actual project file path, so Save works for both.
- Editor: Ctrl+S save, Ctrl+Shift+S save-as, Ctrl+O open project, Ctrl+N new
  scene. Save As now selects a parent directory and creates a project folder.
- Loading refreshes the entity tree, selection and property panel; Delete from
  the context menu now uses the same confirmation flow as the Delete key.
- Added `engine/core/tests/project_roundtrip.rs` covering entity kinds,
  transforms, visibility, lights, scripts, replacement-on-load, direct JSON
  loading and Save-As file paths.
- Updated `TestRunner.cs` expectations for the new `RotatingEntity` defaults
  (`Max=100`, `Initial=10`); startup self-test now reports `PASS=35, FAIL=0`.

### Verification
```bash
cd engine
cargo test -p hezhou-core --offline   # 2 project round-trip tests pass
cd scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1   # C# compile passes
```

---

## 2026-05-20: Outline Winding Order Fix

### Problem
- Outline not rendered when cube rotates from initial back face to front
- Initial state: back face is normal, has outline
- After 180° rotation: back face becomes front, but no outline visible

### Root Cause
- Back face defined with CCW winding from背面 perspective (indices: 0,2,1)
- After 180° Y-axis rotation, same vertices appear CW from正面 perspective
- Vulkan Y-flip: physical CW → screen CCW
- Pipeline `front_face: CLOCKWISE` + `cull_mode: BACK` → rotated back face incorrectly culled as "back face"

### Solution
- Changed outline pipeline `cull_mode` from `BACK` to `NONE`
- Outline only needs to render visible faces, no culling required
- Depth test still works correctly with `LESS_OR_EQUAL` compare

### Files Changed
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs:848` - `cull_mode: NONE`
- `engine/shaders/rotation.frag:alpha_threshold` - 0.01 (unchanged, already correct)

### Technical Details

**Winding Order Analysis:**
```
Back face vertices (from -Z): v0(-0.5,-0.5,-0.5), v1(+0.5,-0.5,-0.5), v2(+0.5,+0.5,-0.5), v3(-0.5,+0.5,-0.5)
Indices: 0,2,1 (triangle 1), 0,3,2 (triangle 2)

Physical CCW from背面 → Physical CW from正面 after 180° rotation
Vulkan Y-flip → Screen CCW → Identified as "back face" by CLOCKWISE front_face
Result: Culled when cull_mode=BACK
```

**Correct Behavior with cull_mode=NONE:**
- All visible faces rendered regardless of winding order
- Depth test ensures proper occlusion
- Alpha blend creates transparent overlay effect

### Verification
Run: `cd engine && cargo run --bin mono_editor_demo --features mono --release`
- All 6 faces should have orange outline at any rotation angle
- Outline alpha=0.3, renders on top of game geometry

---

## 2026-05-20: ESC Key Handling in Running Mode

### Problem
- ESC key in Running mode only canceled PreviewWindow selection
- Status bar showed "退出预览窗", but cube kept rotating
- Clicking "编辑" button could switch back, but ESC was broken

### Root Cause
- C# OnKey() handled ESC by only calling `UI.SetPreviewWindowSelected(false)`
- This canceled the PreviewWindow selection state
- But **renderer.game_state** stayed at Running(1)
- Cube rotation is controlled by `renderer.game_state`, not PreviewWindow selection

### Solution
- Running mode ESC now switches to Editing mode
- Calls `_gameScene.SetGameState(GameState.Editing)`
- Calls `UI.SetRendererGameState(0)`
- Calls `UI.SetPreviewWindowEditMode(true)` for orange border
- Updates button text to "运行"
- Restores saved camera position

### Files Changed
- `engine/scripts/EditorScript.cs:173-223` - OnKey() ESC handling logic

### Code Flow Comparison

**Before (broken):**
```
ESC pressed → SetPreviewWindowSelected(false) → PreviewWindow deselected
             → renderer.game_state still = 1 → Cube keeps rotating
```

**After (fixed):**
```
ESC pressed → SetGameState(Editing) → Scene state = Editing
             → SetRendererGameState(0) → renderer.game_state = 0
             → SetPreviewWindowEditMode(true) → Orange border
             → SetPreviewWindowSelected(false) → PreviewWindow deselected
             → Cube stops rotating ✅
```

### Verification
1. Click "运行" → cube starts rotating, PreviewWindow has blue border
2. Press ESC → cube stops, PreviewWindow has orange border, button shows "运行"
3. Press ESC again (Editing mode) → clears entity selection if any