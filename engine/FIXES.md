# Bug Fixes Log

## 2026-10-18: MSIX 与启动器

- Windows 安装包同时发布 MSIX（MakeAppx + 自签名证书，`.msix` 与 `.cer`）和 MSI（WiX v5）。
- 新增 `hezhou_launcher.exe` GUI 启动器：创建无窗口子进程运行编辑器，
  去掉了安装后点击出现的终端。
- 编辑器启动时检测安装目录是否可写；MSIX/Program Files 只读时把
  scripts/assets/fonts 复制到 `%LOCALAPPDATA%\Hezhou\Editor` 并从该目录运行。
- 仓库加入 `engine/fonts/HarmonyOS_Sans_SC.ttf`，安装包包含字体。
- 修复 Linux CI 的 Unix 信号处理、CMake 生成器、Mono 安装和 DEB C# 编译列表。


## 2026-10-18: 安装包内置运行时依赖

- Windows/ Linux 打包脚本在构建机上存在 Mono 时，复制最小 Mono 运行时到
  `runtime/mono`：bin、etc、lib/mono/4.5、4.5-api。
- Windows 额外复制 `vulkan-1.dll`、`vcruntime140.dll`、`vcruntime140_1.dll`、
  `msvcp140.dll` 到安装根目录。
- 编辑器启动时自动把 `runtime/mono/bin` 加入 PATH，并设置 `MONO_PATH`、
  `MONO_CFG_DIR`、`MONO_GAC_PREFIX`；hot reload 优先使用内置 `mcs`。
- Linux 启动器设置同样的 PATH/LD_LIBRARY_PATH/MONO_* 环境；检测到内置 Mono 时
  从 deb Depends 中移除 mono-runtime/mono-mcs。
- 包内 scripts 目录只保留 `.cs` 与 `EditorScript.dll`，不再携带 obj/测试产物。


## 2026-10-18: Windows MSI / Linux DEB 打包

- `installer/windows/HezhouEditor.wxs`：WiX v4 MSI，安装到 Program Files，
  含开始菜单快捷方式和工作目录；`build_msi.ps1` 负责构建、暂存和调用 wix。
- `installer/linux/build_deb.sh` + `build_deb.py`：在 Linux 构建 deb；
  Python 直接写 ar + control.tar.gz + data.tar.gz，不依赖 dpkg-deb。
- 安装目录 `/opt/hezhou/editor`，`/usr/bin/hezhou-editor` 启动器，
  desktop entry 和 SVG 图标。
- CI 增加手动触发的 `package-windows` / `package-linux` 任务。


## 2026-10-18: 重力 / 角色 / 动画 / 天空盒

- `CharacterController`：水平移动、重力积分、跳跃、地面吸附；3 个物理用例。
- 动画：`AnimationClip`/`AnimationKeyframe`/`AnimationPlayer` + `AnimationSystem`，
  支持位置/旋转/缩放关键帧、循环、slerp 插值；加入 Scene 默认系统。
- 天空盒：`sky.vert/sky.frag` 全屏渐变 pipeline，RasterPipeline 在实体前绘制；
  BGR 写出 + Vulkan UV 翻转修正，渐变顶部深蓝、底部浅蓝。
- 新增 `core/tests/animation_system.rs`、`physics/tests/character.rs`。
- visual 33 / intent 26 全通过；平均帧 6.94ms。


## 2026-10-18: 3D renderer Gizmo 轴线段

- `RasterPipeline` 在实体绘制后增加 Gizmo 轴线段绘制：以实体世界坐标为原点，
  三轴各两条交叉的 0.05 宽薄四边形，避免依赖 editor 的 UI 叠加层。
- 复用 outline pipeline；`rotation.frag` 的 outline 输出改为 `pc.outline_color`，
  这样三轴可以分别用红/绿/蓝绘制，选中描边仍传橙色。
- `RenderContext` 新增 `gizmo_visible/gizmo_position`；UIVulkanRenderer 新增
  `set_gizmo_3d` FFI（283 字段），C# 每帧同步选中实体坐标。
- UI Button 轴条保留为透明命中区域用于拖拽，不参与显示；Button 背景 alpha=0
  时不再绘制边框。


## 2026-10-18: PNG 完整解码 + UI 纹理上传

- 新增 `png` 依赖和 `FontAtlas::load_png_image`：完整解码 PNG（RGBA/RGB/
  灰度/灰度透明），统一转 RGBA。
- 解码像素写入字体图集空闲区，注册 `ImageRegion`，通过字体图集上传到 GPU；
  `img:<id>` 缩略图直接采样该区域，支持 alpha。
- `ui_load_image_texture(path)` FFI；GridView 新增 `img:<id>` 分支。
- 新增 `assets/textures/spectrum.png`，纹理页显示真实 PNG 缩略图。
- FFI 282 字段；intent `texture_png`（26/26）。


## 2026-10-18: 字体清晰度与控件视觉风格

- 修复字体 3x 超采样后未降采样就写入图集的问题：UV 覆盖 3x 位图，
  绘制时被双线性缩小 3 倍，笔画明显发虚。现按 3x3 box filter 降采样到 1x。
- 降采样 alpha 增加 gamma 0.55，避免小字号笔画过淡。
- 字形落点四舍五入到像素网格，减少 LINEAR 采样模糊。
- 默认字号 14→15（按钮/Label/TextEdit/ListItem/Tab），状态栏 15，GridView 13。
- 新增圆角矩形绘制路径（三角形扇），按钮/输入框/Tab 使用 6–7px 圆角。
- Button 默认边框改为主题 border，hover/pressed 由基础色明暗派生，不再硬编码蓝色。
- Inspector 标签宽度 96→150，Float3/字符串输入宽度相应调整，修复字号增大后的截断。
- Tab 激活项增加 2px accent 指示条。


## 2026-10-18: 引擎基础能力：ECS 系统、物理碰撞、几何射线

- ECS：`World::update` 真正驱动 `SystemScheduler`，新增 `LifetimeComponent` /
  `LifetimeSystem`，Scene 默认注册。
- 物理：`PhysicsWorld` 新增 AABB 碰撞检测、碰撞事件、简单重叠分离；球/盒碰撞测试。
- 几何：`BoundingBox::intersects_ray` slab 射线求交，含测试。
- Rust 测试：core 全通过，physics 2 用例，geometry 2 用例。


## 2026-10-18: 拖拽式跨区域 Dock

- TabWidget 拖拽结束时通过 `on_tab_drag_end` 回调把 Tab 标题/坐标交给 C#。
- C# 判断拖到右侧属性区/左侧区域，调用纹理页停靠切换。
- 修复停靠重建后外部分割比例异常（强制恢复 0.72）。
- FFI 281 字段；intent `texture_drag_dock`（25/25）；visual `32_drag_dock_right`、`33_drag_dock_back`。


## 2026-10-18: 文件纹理缩略图（BMP → 8x8 像素）

- 新增 `assets/textures/gradient.bmp`，启动时通过 AssetLibrary 加载。
- C# 解析 24/32-bit BMP 并降采样为 8x8 `pixels:RRGGBB...`。
- GridView 新增 `pixels:` 缩略图分支，按像素块绘制真实文件纹理预览。
- intent `texture_bmp`（24/24）；visual `31_texture_bmp`。


## 2026-10-18: Gizmo 跟随实体投影位置

- C# 内按 rotation.vert 的 view/projection 公式实现世界坐标到预览屏幕坐标投影。
- Gizmo 轴条中心现在跟随选中实体的投影位置，不再按窗口比例居中。
- 轴条改用 Button（Panel 会被 PreviewWindow 覆盖）。
- visual `19_gizmo_translate` 已对齐 Cube。


## 2026-10-18: Gizmo 轴条居中

- 发现 UI Panel 叠加层会被 PreviewWindow 覆盖，但 Button 不会被覆盖。
- 轴条从 Panel 改为彩色 Button（X/Y/Z），按窗口比例居中到预览区。
- visual `19_gizmo_translate` 已显示居中轴条。


## 2026-10-18: Dock v2 跨区域停靠（纹理页）

- 纹理页可停靠到右侧属性 Tab（`纹理页停靠右侧/回到底部` 命令）。
- 状态持久化 `.hezhou_layout` `textureDocked`；重建 UI 保持 Scene。
- intent `texture_dock`（23/23）；visual `29_texture_docked_right`、`30_texture_dock_restored`。
- 拖拽式跨区域停靠仍未实现，当前为命令式停靠。


## 2026-10-18: Asset Browser 纹理 Tab 与程序化缩略图

- 新增“纹理”Tab，读取 AssetLibrary Textures 分类。
- GridView 支持 `tex:solid` / `tex:checker` / `tex:uv` 程序化纹理缩略图。
- intent `texture_browser`（22/22）；visual `28_texture_tab`。
- PNG 文件纹理缩略图仍未实现。


## 2026-10-18: 材质赋值持久化

- `ProjectEntity` 新增 `material` 字段（serde default 兼容旧工程）。
- Project 保存/加载保留 `RenderableComponent.material_path`。
- `project_roundtrip` 增加材质断言，core 测试通过。


## 2026-10-18: CI 本地镜像校验

- 新增 `scripts/ci_local.ps1`：本地复刻 GitHub Actions 的 Rust 测试、FFI 契约、
  fuzz check、C# 编译、图标生成一致性。
- 默认离线（`CARGO_NET_OFFLINE=true`）并使用临时 CARGO_HOME。
- 本地执行通过：`CI LOCAL OK`。
- GitHub runner 仍需要推送后实跑。


## 2026-10-18: Dock v2 浮动面板拖动

- 浮动资产面板支持按住标题区域拖动；位置实时更新。
- 新增 `TestMoveFloatingPanel`。
- intent `floating_move`（21/21）；visual `26_floating_moved`、`27_floating_closed`。
- 跨区域停靠仍未实现。


## 2026-10-18: Dock v2 浮动资产面板

- Command Palette 新增“浮动资产面板”。
- Dialog 浮动窗口展示资产网格与缩略图，可关闭。
- intent `floating_panel`（20/20）；visual `25_floating_panel`、`26_floating_closed`。
- 可拖拽停靠的浮动 Dock 仍未实现。


## 2026-10-18: Dock v2 面板收起/展开

- 项目面板、资产面板支持收起/展开（SplitView ratio 1.0）。
- 收起状态写入 `.hezhou_layout`；Command Palette 新增两条命令。
- intent `dock_collapse`（19/19）；visual `23_asset_collapsed`、`24_dock_restored_2`。
- 跨区域停靠/浮动面板仍未实现。


## 2026-10-18: Dock v2 Tab 拖拽重排

- TabWidget 支持 reorderable 拖拽重排；同步 Rust children 顺序，避免内容错位。
- 资产页 Tab 开启拖拽；FFI `ui_tab_widget_move_tab` / `ui_tab_widget_set_reorderable`，契约 280 字段。
- intent `tab_reorder`（18/18）；visual `22_tab_reorder`。
- 跨区域停靠/浮动面板仍未实现。


## 2026-10-18: Gizmo 缩放/旋转模式

- Gizmo 支持平移/缩放/旋转三种模式（Ctrl+G 循环）。
- 缩放按轴改变实体 scale；旋转轴拖拽调用 SceneRotateEntity。
- intent `gizmo_scale`（17/17）；visual `21_gizmo_scale`。


## 2026-10-18: 材质拖放/赋给实体

- Scene 新增 renderable material 读写；FFI `scene_set_entity_material` / `scene_get_entity_material`，契约 278 字段。
- 材质 Tab 使用按钮；点击赋给选中实体。
- 全局拖拽：从材质按钮拖到实体节点/预览后赋材质。
- intent `material_assign`（16/16）；visual `20_material_assign`。


## 2026-10-18: Gizmo v1 平移

- 预览区叠加 X/Y/Z 三轴彩色条（layer 3）。
- 拖拽轴条更新选中实体位置；状态栏显示实时 X/Y。
- 新增 `TestGizmoNudge` / `TestGizmoVisible`。
- intent `gizmo_translate`（15/15）；visual `19_gizmo_translate`。
- 当前轴条位置基于 PreviewWindow 布局，尚未做到投影中心跟随。


## 2026-10-18: Asset Browser 材质列表

- 材质 Tab 不再是“待实现”，改为读取 AssetLibrary Materials 分类并列出材质。
- 材质项显示名称；空名称回退 `(unnamed)`。
- 新增 `intent.material_browser`；visual `18_material_tab`。
- 材质拖放/赋给实体仍未实现。


## 2026-10-18: Hierarchy 拖拽 reparent

- 全局鼠标拖拽：在已选实体节点上按下并拖到另一个节点，释放后 SetParent。
- 拖拽中状态栏显示目标实体；循环层级由 Scene 校验拒绝。
- 新增 `TestHierarchyDrop`、`TestExpandHierarchy`。
- intent `hierarchy_drag`（13/13）；visual `17_hierarchy_drag`。


## 2026-10-18: Inspector 属性行重做

- 属性面板从“标签一行 + 输入一行”改为单行 Label + 输入控件。
- Float3 使用一行 Label + X/Y/Z 三个输入；String/Int/Float/Bool 单行。
- 长属性名截断到 11 字符，避免挤压输入框。
- visual 新增 `16_inspector_rows.png`。


## 2026-10-18: Console 源码定位与跳转

- `Log.Warn/Error/Fatal` 增加 `[CallerFilePath]` / `[CallerLineNumber]`。
- `TestTelemetry` 记录 SourceFile/SourceLine，Console 行尾显示 `[文件:行]`。
- Console 新增“跳转源码”按钮：取最近 Error/Fatal，打开脚本编辑器并定位到对应行。
- TextEdit 新增 `set_cursor_to_line` + FFI `ui_text_edit_set_cursor_line`。
- FFI 276 字段；visual 02 Console 显示跳转按钮。\n- intent `console_source_location` 验证 CallerFilePath/LineNumber（12/12）。


## 2026-10-18: 资产缩略图（SVG 图标）

- GridView 支持 `icon:<id>` 缩略图，并在 `thumbnail` 缺失时按资产名回退到图标。
- 资产模型网格为 Cube/Sphere/Plane/Cylinder/CornellBox 绘制矢量缩略图。
- 新增 FFI `ui_grid_view_set_item_thumbnail`；FFI 275 字段。
- visual `03_asset_search` 已显示 Cube 缩略图。


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