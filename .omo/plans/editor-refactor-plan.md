# Hezhou 编辑器全面重构规划

> 生成时间: 2026-05-23
> 状态: 规划阶段，用户确认后再分阶段实施

---

## 一、需求总览（20+项）

将所有用户需求按性质分为4个阶段，优先级从高到低：

| 阶段 | 类别 | 需求数 | 核心目标 |
|------|------|--------|----------|
| 阶段一 | 基础修复 | 9项 | 编辑器基本可用 — 不崩溃、不卡死、基础交互正常 |
| 阶段二 | UI架构重构 | 8项 | 编辑器架构升级 — MVP结构、面板重构、拖动布局 |
| 阶段三 | 渲染修复 | 3项 | 3D渲染正常 — 光照生效、透视投影、OBJ导入 |
| 阶段四 | 动画和高级UI | 5项 | 编辑器体验提升 — 动画系统、新组件、trace交互 |

---

## 二、阶段一：基础修复（预估1-2周）

> 目标：编辑器基本可用 — 修复所有阻塞性 bug，基础交互正常工作

### 1.1 ESC退出编辑模式

**问题**: 编辑模式下按ESC无法退出

**现状**: 
- `KeyCode::ESC = 39`
- `GameState` 有 Editing(0)/Running(1)/Paused(2) 三种状态
- `Presenter.cs` 中 `OnKey` 回调处理按键事件

**修复方案**:
- 在 `OnKey(KeyCode=39)` 回调中，当 `GameState == Running || Paused` 时切换到 `Editing`
- 调用 `SceneSetGameState(0)` 回到编辑模式
- 重置摄像机位置到默认编辑视角
- 清除选中Entity高亮

**涉及文件**:
- `engine/scripts/EditorScript.Presenter.cs` — OnKey 回调

**验证**:
1. 运行编辑器 → 进入Running模式 → 按ESC → 回到Editing（橙色边框）
2. 暂停模式 → 按ESC → 回到Editing
3. Editing模式 → 按ESC → 无操作（不影响）

---

### 1.2 窗口最大化响应

**问题**: 窗口最大化后缩小，按钮点击无响应

**根因分析**:
- 窗口resize时，UI布局重新计算但点击区域可能与渲染不一致
- 可能是 `widget_tree` 的 `hit_test` 使用的坐标与渲染坐标不同步
- 或者 `global_click` 回调中的坐标没有随窗口缩放调整

**修复方案**:
- 检查 `ui_vulkan_renderer.rs` 中 resize 时 viewport 是否正确更新
- 确认 `WidgetTree::hit_test()` 使用的坐标与渲染坐标一致
- 确认 `measure_and_layout()` 在窗口 resize 后正确执行
- 确认 `generate_render_data()` 使用正确的窗口尺寸
- 检查 GLFW resize callback → Rust `ui_resize()` → C# `OnResize()` 链路完整性

**涉及文件**:
- `engine/ui/src/widget_tree.rs` — hit_test、measure_and_layout
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs` — viewport resize
- `engine/ui/src/lib.rs` — ui_resize FFI
- `engine/scripts/EditorScript.Presenter.cs` — OnResize 回调

**验证**:
1. 启动编辑器 → 最大化窗口 → 点击按钮 → 正常响应
2. 最大化 → 缩小回原尺寸 → 点击按钮 → 正常响应
3. 最大化 → 拖动边框到不同尺寸 → 所有按钮都能点击
4. 不同尺寸下预览窗Entity位置正确

---

### 1.3 菜单栏子菜单文字显示不全

**问题**: PopupMenu 子菜单文字被截断，显示不全

**根因分析**:
- PopupMenu 的 `calculate_size()` 计算宽度时可能没有正确测量文字宽度
- 或者 `layout()` 时给文字分配的空间不足
- FontAtlas 测量文字宽度时可能没有考虑 emoji/中文等宽字符

**修复方案**:
- 检查 `popup_menu.rs` 中 `calculate_size()` 的文字宽度测量
- 确保测量使用 `font_atlas.measure_text_width()` 而不是硬编码宽度
- PopupMenu 最小宽度应基于最长菜单项的文字宽度 + padding + shortcut区域
- 确保菜单项 layout 时文字有足够空间

**涉及文件**:
- `engine/ui/src/widgets/popup_menu.rs` — calculate_size、layout
- `engine/ui/src/font_atlas.rs` — measure_text_width

**验证**:
1. 打开"文件"菜单 → 所有菜单项文字完整显示
2. 打开"保存"菜单 → "保存场景"文字完整
3. 长菜单项（如"另存为"）文字不被截断
4. 快捷键文字（如Ctrl+S）也完整显示

---

### 1.4 emoji文字无法显示

**问题**: emoji字符（如✓、✗、→等）在UI中无法正常渲染

**根因分析**:
- FontAtlas 使用基础ASCII/Latin字符集，不含emoji Unicode范围
- emoji需要特殊字体（如 Noto Color Emoji 或 Segoe UI Emoji）
- 当前文字渲染管线不支持多色glyph

**修复方案**:
- **方案A（推荐）**: 扩展 FontAtlas 支持 Unicode BMP 补充区域（U+2700-U+27BF  dingbats）
  - 加载系统emoji字体作为备用字体
  - FontAtlas glyph查找：先查主字体 → 找不到查emoji备用字体
  - 渲染emoji glyph为单色（不追求彩色emoji，先确保可见）
- **方案B（长期）**: 支持彩色emoji渲染
  - 需要新render pipeline：emoji texture atlas + 专用shader
  - 暂时不做，阶段一先实现方案A

**涉及文件**:
- `engine/ui/src/font_atlas.rs` — 多字体查找、emoji glyph生成
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs` — emoji glyph渲染
- `engine/ui/src/widgets/*.rs` — 使用emoji的控件（Checkbox等）

**验证**:
1. Label 显示 "✓ 完成" → ✓可见（单色即可）
2. Checkbox 勾选状态显示 ✓ → ✓可见
3. 菜单项显示 "→ 新建场景" → →可见
4. 中文字符正常显示（不是emoji但也是多字节Unicode）

---

### 1.5 InputField光标显示

**问题**: InputField组件没有光标显示当前插入位置

**现状**:
- `input_field.rs` 有 `_cursor_position` (usize) 字段
- 有 `_is_focused` 状态
- `draw()` 方法中没有渲染光标指示器

**修复方案**:
- 在 `draw()` 中，当 `_is_focused == true` 时渲染光标线
- 光标位置：基于 `_cursor_position` 计算x坐标
  - 计算 `_cursor_position` 个字符的累积宽度
  - 光标x = 文本起始x + 累积宽度
- 光标渲染：细竖线（1-2px宽，文本颜色，高度=font_size）
- 光标闪烁：每500ms切换显示/隐藏（基于时间戳）
- 点击InputField → 设置 `_cursor_position` 到点击位置最近的字符边界

**涉及文件**:
- `engine/ui/src/widgets/input_field.rs` — draw() 添加光标渲染
- `engine/ui/src/font_atlas.rs` — 计算字符偏移宽度

**验证**:
1. 点击InputField → 光标出现在点击位置
2. 输入文字 → 光标跟随文字移动
3. 光标闪烁可见（500ms间隔）
4. 不同font_size下光标高度正确
5. 窗口resize后光标位置正确

---

### 1.6 项目结构高亮松开消失

**问题**: 点击项目结构树节点后，松开鼠标仍然高亮，高亮没有在鼠标松开时消失

**根因分析**:
- TreeView节点的选中状态（`_selected`）在点击后设置为true
- 但没有在鼠标松开时取消高亮（应该是按下高亮，松开后恢复普通选中态）
- 或者：选中状态和高亮（pressed/hovered）状态混淆

**修复方案**:
- 明确区分三种状态：
  - `Selected`（选中态） — 节点被选中后持续显示，蓝色背景
  - `Pressed`（按下态） — 按下时显示，深色背景，松开后消失
  - `Hovered`（悬浮态） — 鼠标悬浮时显示，浅色背景
- 修改 TreeNode `draw()` 渲染逻辑：Pressed状态只在按下时显示
- 修改 event_dispatcher：TouchEnd → 从Pressed恢复到Selected或Normal
- 选中后高亮 = Selected态的视觉表示，不是Pressed态

**涉及文件**:
- `engine/ui/src/widgets/tree_view.rs` — TreeNode 状态管理
- `engine/ui/src/widgets/tree_node.rs` — draw() 渲染逻辑
- `engine/ui/src/event_dispatcher.rs` — TouchEnd 状态转换

**验证**:
1. 点击树节点 → 按下时深色高亮
2. 松开鼠标 → 深色高亮消失，变为选中态（蓝色背景）
3. 点击另一个节点 → 前一个节点选中态消失
4. 不点击时悬浮节点 → 浅色悬浮高亮，移开鼠标消失

---

### 1.7 UI测试滚动修复

**问题**: UITestRunner截图超出组件显示范围，且不可滑动

**现状**:
- UITestRunner已使用ScrollView包裹截图列表
- 但可能ScrollView的content_size计算不正确
- 或者ScrollView的滚轮/拖拽事件没有正确传递

**修复方案**:
- 检查 ScrollView 的 `content_size` 是否正确设置（应大于viewport才可滚动）
- 确保截图列表每个item高度正确，总高度 = item_height × item_count
- ScrollView clip_rect 设置正确（超出部分不渲染）
- 滚轮事件正确传递到ScrollView
- 检查 UITestRunner 中 ScrollView 的尺寸设置

**涉及文件**:
- `engine/ui/src/widgets/scroll_view.rs` — content_size、滚动逻辑
- `engine/scripts/UITestRunner.cs` — ScrollView使用

**验证**:
1. 打开UITestRunner → 截图列表只显示可视区域
2. 鼠标滚轮滚动 → 截图列表上下移动
3. 拖拽滚动条 → 截图列表滚动
4. 截图总数 > 可视区域容量 → 滚动条出现

---

### 1.8 UI测试点击图片显示

**问题**: 点击截图只显示图片信息，不能显示真正的图片；需增加Image组件

**修复方案**:
- **新增 Image Widget**（Rust + C#）
  - Rust: `engine/ui/src/widgets/image.rs` — Image widget
    - 持有 texture_id (u64)，从Canvas获取纹理
    - `draw()` 渲染纹理到指定bounds
    - 支持缩放模式：Fill/Crop/Fit/Center
  - FFI: `engine/ui/src/ffi/image.rs` — create/set_texture/set_scale_mode
  - C#: `UI.NewWidgets.cs` — Image 类 + CreateImage/SetTexture/SetScaleMode
- UITestRunner截图预览Dialog中使用Image显示真实截图
  - 截图文件路径 → 加载为纹理 → Image widget显示

**涉及文件**:
- `engine/ui/src/widgets/image.rs` — 新增
- `engine/ui/src/ffi/image.rs` — 新增
- `engine/ui/src/widgets/mod.rs` — 注册
- `engine/ui/src/thunk_manager.rs` — PendingCallback扩展
- `engine/scripts/UI.NewWidgets.cs` — Image C#类
- `engine/scripts/UI.cs` — FfiContext + 委托
- `engine/scripts/UITestRunner.cs` — 使用Image显示截图

**验证**:
1. 创建Image widget → 设置纹理 → 显示图片
2. 不同缩放模式（Fill/Crop/Fit/Center）正确渲染
3. UITestRunner点击截图 → Dialog显示真实图片
4. 窗口resize → Image自适应

---

### 1.9 UI测试排除隐形组件

**问题**: UI测试点击隐形/内部组件概率太高，应排除

**修复方案**:
- 扩展 `GLOBAL_CLICK_EXCLUDE` 列表，加入更多隐形组件类型
- UITestRunner点击时只选择"有用"的UI组件（Button、Label、InputField、Dropdown等）
- 排除：Panel（纯容器）、VStack/HStack（布局容器）、ScrollBar、ScrollView内部组件
- 增加点击路径多样性：每步随机选择不同类型的可交互组件
- 降低连续点击同一组件的概率

**涉及文件**:
- `engine/ui/src/event_dispatcher.rs` — GLOBAL_CLICK_EXCLUDE
- `engine/scripts/UITestRunner.cs` — 点击目标选择逻辑

**验证**:
1. UITestRunner每步点击不同类型的可交互组件
2. 不点击Panel/VStack等纯容器
3. 点击路径摘要显示有意义的目标名称

---

### 1.10 脚本编辑器返回后目录文字bug（额外发现）

**问题**: 从脚本编辑器返回主界面后，项目结构树上显示了脚本编辑器的目录文字

**根因分析**:
- 脚本编辑器可能是替换了中央面板的内容
- 返回时TreeView没有重新加载项目结构数据
- 或者TreeView的可见性状态没有正确恢复

**修复方案**:
- 切换到脚本编辑器时，保存主界面面板状态
- 返回时，重新设置TreeView内容为项目结构数据
- 确保脚本编辑器的文字内容不影响TreeView的显示

**涉及文件**:
- `engine/scripts/EditorScript.View.cs` — 面板切换逻辑
- `engine/scripts/EditorScript.Presenter.cs` — 状态恢复

**验证**:
1. 进入脚本编辑器 → 编辑脚本 → 返回主界面
2. 项目结构树显示正确的项目文件列表
3. 不显示脚本编辑器的代码内容

---

## 三、阶段二：UI架构重构（预估2-3周）

> 目标：编辑器架构升级 — MVP结构、面板重构、拖动布局、脚本绑定流程优化

### 2.1 MVP架构重构

**问题**: 当前项目结构不符合MVP（Model-View-Presenter）模式

**现状**:
- `EditorScript.cs` 是单一巨型文件
- View创建、Presenter逻辑、Model数据混合在一起
- 已有部分拆分：`EditorScript.State.cs`（Model）、`EditorScript.View.cs`（View）、`EditorScript.Presenter.cs`（Presenter）
- 但仍有大量逻辑交叉

**重构方案**:
- 严格执行MVP三层分离：
  - **Model层** (`EditorScript.State.cs`): 所有数据状态
    - GameState、选中Entity ID、项目数据、脚本列表
    - 纯数据，不含UI创建逻辑
  - **View层** (`EditorScript.View.cs`): UI创建和布局
    - 创建所有widget、设置样式、布局
    - 不含业务逻辑，不含状态修改
    - View通过回调通知Presenter
  - **Presenter层** (`EditorScript.Presenter.cs`): 业务逻辑
    - 处理回调、修改Model、通知View更新
    - 不直接创建widget
- 增加projects目录存放工程文件
  - `projects/` 目录存放 `.project.json`、脚本文件、资产文件
  - 启动时加载 `projects/default/`

**涉及文件**:
- `engine/scripts/EditorScript.State.cs` — Model重构
- `engine/scripts/EditorScript.View.cs` — View重构
- `engine/scripts/EditorScript.Presenter.cs` — Presenter重构
- 新增 `projects/` 目录

**验证**:
1. Model/View/Presenter三层无交叉引用
2. View层只创建UI，不修改状态
3. Presenter层只处理逻辑，不创建widget
4. 项目文件保存到 `projects/` 目录

---

### 2.2 属性面板标签页重构

**问题**: 属性面板不是下拉可选脚本；需要分类用标签页显示

**重构方案**:
- 属性面板使用 `TabWidget` 分为5个标签页：
  - **几何** (Geometry): mesh类型、顶点数
  - **位置** (Transform): position XYZ、rotation XYZ、scale XYZ
  - **渲染** (Rendering): 可见性、材质、颜色
  - **运动** (Motion): 脚本绑定列表、下拉选脚本、绑定按钮
  - **物理** (Physics): collider类型、mass、friction（预留）
- **运动标签页**重构脚本绑定流程：
  - 下拉选择已编写好的脚本（不是当前那样无下拉）
  - 点击"绑定"按钮 → 脚本附加到Entity
  - 已绑定脚本列表：显示名称 + ON/OFF切换 + 移除按钮
- 每个标签页内的属性用 ScrollView 包裹（内容超出时可滚动）
- 数值属性旁边放 Slider（阶段四实现）和 InputField

**涉及文件**:
- `engine/scripts/EditorScript.View.cs` — 属性面板TabWidget创建
- `engine/scripts/EditorScript.Presenter.cs` — 标签页切换逻辑
- `engine/scripts/EditorScript.State.cs` — 属性面板状态

**验证**:
1. 选中Entity → 属性面板显示5个标签页
2. 切换标签页 → 内容切换正确
3. 位置标签页 → position/rotation/scale XYZ可编辑
4. 运动标签页 → 下拉选脚本 → 绑定 → 显示绑定关系
5. 不同Entity类型显示不同默认标签页（Cube→位置，Light→渲染）

---

### 2.3 资产面板重构

**问题**: 资产面板要横跨项目结构和游戏预览窗的宽度，也要用标签页分类

**重构方案**:
- 资产面板从左下角移到底部，横跨项目结构和预览窗宽度
- 使用 TabWidget 分类：
  - **模型** (Models): OBJ/fbx等3D模型资产
  - **纹理** (Textures): PNG/JPG纹理资产
  - **脚本** (Scripts): C#脚本资产
  - **音频** (Audio): 音频资产（预留）
  - **其他** (Others): 其他资产
- 每个标签页用 GridView 显示资产缩略图+名称
- 底部面板高度约200px，可拖动调整（配合SplitView）

**涉及文件**:
- `engine/scripts/EditorScript.View.cs` — 底部资产面板布局
- `engine/scripts/EditorScript.State.cs` — 资产面板状态

**验证**:
1. 资产面板在底部，横跨左侧和中央
2. 切换标签页 → 显示不同类型资产
3. 资产名称+缩略图显示正确
4. 面板高度可拖动调整

---

### 2.4 SplitView拖动布局

**问题**: 预览窗、项目结构、资产、属性面板要能相互拖动分割线调整大小

**现状**:
- `SplitView` widget 已有拖拽分割线实现
  - `dragging`、`divider_hovered`、`divider_thickness`
  - 但未用于编辑器主布局

**重构方案**:
- 编辑器主布局改为嵌套 SplitView：
  ```
  ┌──────────────────────────────────────────────┐
  │  工具栏 (40px, 固定)                          │
  ├─────────┬──────────────────────┬─────────────┤
  │ 项目结构 │     预览窗          │   属性面板   │
  │ (250px) │                    │  (250px)     │
  │         │                    │              │
  │         │                    │              │
  ├─────────┴──────────────────────┴─────────────┤
  │  资产面板 (200px, 标签页分类)                  │
  └──────────────────────────────────────────────┘
  │  状态栏 (30px, 固定)                          │
  └──────────────────────────────────────────────┘
  ```
- 外层垂直SplitView：顶部(主区域) ↔ 底部(资产)
- 内层水平SplitView：左(项目结构) ↔ 中(预览) ↔ 右(属性)
- SplitView增强功能：
  - 分割线附近鼠标变为拖动形状（需要cursor形状API）
  - 分割线hover时高亮加粗（3px → 5px，颜色变亮）
  - 最小宽度限制（防止面板缩到消失）：项目结构100px、预览300px、属性150px、资产100px
- **新增 Cursor Shape API**：
  - Rust FFI: `ui_set_cursor_shape(shape: u32)` — 设置鼠标光标形状
  - 形状枚举: Default(0), Arrow(1), Text(2), Crosshair(3), Move(4), ResizeH(5), ResizeV(6), ResizeAll(7), Hand(8), NotAllowed(9)
  - SplitView hover分割线 → set_cursor_shape(ResizeH/ResizeV)
  - GLFW: `glfwSetCursor(window, cursor)` 实现不同光标形状

**涉及文件**:
- `engine/ui/src/widgets/split_view.rs` — 增强拖拽交互
- `engine/ui/src/ffi/misc.rs` — 新增 ui_set_cursor_shape
- `engine/scripts/EditorScript.View.cs` — SplitView嵌套布局
- `engine/scripts/EditorScript.State.cs` — SplitView状态

**验证**:
1. 鼠标靠近分割线 → 光标变为拖动形状
2. hover分割线 → 分割线高亮加粗
3. 拖动分割线 → 面板动态调整大小
4. 面板不会缩到最小宽度以下
5. 所有面板内容随resize自适应

---

### 2.5 脚本绑定流程优化

**问题**: 当前脚本绑定流程不清晰，属性面板不是下拉可选脚本

**优化方案**:
完整流程（用户描述）：
1. 添加Entity → 进入编辑器
2. 创建脚本 → 修改并保存脚本
3. 编辑模式下选中Entity
4. 属性面板→运动标签页→下拉选中已编写好的脚本
5. 点击"绑定" → 属性面板显示绑定关系

实现要点：
- 脚本下拉列表：扫描 `scripts/` 目录下所有 `.cs` 文件
- Dropdown显示脚本类名（不是文件名）
- 绑定按钮：调用 `SceneBindScript(entity, script_class_name)`
- 绑定后显示列表：脚本名 + ON/OFF + 移除按钮
- ON/OFF：调用 `SceneSetScriptBindingEnabled(entity, index, bool)`
- 移除：调用 `SceneUnbindScript(entity, index)`
- 脚本编辑器返回后不污染项目结构树（见1.10）

**涉及文件**:
- `engine/scripts/EditorScript.View.cs` — 运动标签页UI
- `engine/scripts/EditorScript.Presenter.cs` — 绑定逻辑
- `engine/ui/src/widgets/dropdown.rs` — 下拉列表

**验证**:
1. 创建Entity → 创建脚本 → 选中Entity
2. 运动标签页下拉显示可用脚本
3. 选择脚本 → 点击绑定 → 绑定关系显示
4. ON/OFF切换 → 脚本启禁生效
5. 移除按钮 → 解除绑定

---

### 2.6 hot reload修复

**问题**: hot reload会失败

**根因分析**:
- Mono assembly reload流程可能有问题
- 或者编译脚本失败时没有正确错误处理
- 或者 `ResetAll()` 调用时机不对

**修复方案**:
- 检查 `build_mono.ps1` 编译是否可靠
- 检查 `executor.reload()` 是否正确unload旧assembly
- 增加 reload 失败时的错误提示（状态栏显示"热重载失败: xxx"）
- 确保 reload 后静态变量正确重新初始化
- 确保 `ResetAll()` 在新assembly加载后调用

**涉及文件**:
- `engine/scripting/src/mono_executor.rs` — reload逻辑
- `engine/scripts/EditorScript.Presenter.cs` — hot reload触发
- `scripts/build_mono.ps1` — 编译脚本

**验证**:
1. 修改脚本 → 按R键 → 热重载成功
2. 修改脚本有语法错误 → 按R键 → 状态栏显示错误信息
3. 热重载后静态变量重新初始化
4. 连续多次热重载不崩溃

---

### 2.7 projects目录

**问题**: 所有工程所需文件应该额外创建一个projects目录存放

**方案**:
- 创建 `projects/` 目录存放用户项目文件
- 默认项目: `projects/default/default.project.json`
- 项目结构:
  ```
  projects/default/
  ├── default.project.json   ← 项目配置
  ├── scenes/                 ← 场景文件
  │   └── main.scene.json
  ├── scripts/                ← 用户脚本
  │   └── MyScript.cs
  ├── assets/                 ← 资产文件
  │   ├── models/
  │   ├── textures/
  │   └── audio/
  ```
- 启动时加载默认项目或上次项目
- Ctrl+S保存到项目目录

**涉及文件**:
- 新增 `projects/default/` 目录
- `engine/scripts/EditorScript.State.cs` — 项目路径
- `engine/scripts/EditorScript.Presenter.cs` — 保存/加载

**验证**:
1. 启动编辑器 → 自动加载 `projects/default/`
2. Ctrl+S → 项目保存到 `projects/default/`
3. 创建新项目 → 新目录在 `projects/` 下

---

### 2.8 UI层级和边界明确化

**问题**: UI组件内部边界、外部边界不明确

**方案**:
- 每个Widget明确区分 padding（内部边界）和 margin（外部边界）
- `Layout` 结构体增加 `padding: EdgeInsets` 和 `margin: EdgeInsets` 字段
- 布局计算时：
  - 外部尺寸 = width + margin.left + margin.right
  - 内部内容区域 = width - padding.left - padding.right
  - 子widget布局在内容区域内
- Visual padding: 绘制背景色时只填充 padding 内区域
- Visual margin: margin 区域透明/父widget背景

**涉及文件**:
- `engine/ui/src/widget.rs` — Layout 增加 padding/margin
- `engine/ui/src/layout.rs` — 布局计算考虑 padding/margin
- `engine/ui/src/widgets/*.rs` — 各widget使用 padding/margin

**验证**:
1. Button padding=10 → 文字与按钮边缘有10px间距
2. Button margin=5 → 按钮之间有5px间距
3. Panel padding=15 → 子widget从内容区域开始布局
4. InputField padding=5 → 文字与输入框边缘有5px间距

---

## 四、阶段三：渲染修复（预估1-2周）

> 目标：3D渲染正常 — 光照生效、透视投影、OBJ模型导入渲染

### 3.1 所有render pass光照不生效

**问题**: 所有render pass光照都不生效

**根因分析**:
- Vulkan渲染管线中光照数据可能没有正确传递到shader
- push constants中可能缺少光照参数
- shader中光照计算逻辑可能有bug
- 或者uniform buffer中光照数据未正确绑定

**修复方案**:
- 检查Vulkan render pipeline：
  - 确认 `push_constants` 包含光照参数（light_position, light_color, light_direction）
  - 确认 fragment shader 接收光照参数
  - 确认光照计算逻辑正确（至少实现Lambertian diffuse）
- 实现基础光照模型：
  - 方向光（DirectionalLight）：Lambertian diffuse + constant ambient
  - 光照公式: `color = ambient + max(dot(normal, light_dir), 0) * diffuse * light_color`
- 确认 Entity::Light 正确设置光照参数

**涉及文件**:
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs` — push constants 光照数据
- `engine/rhi-vulkan/src/shaders/` — fragment shader 光照计算
- `engine/geometry/src/primitive.rs` — mesh normals

**验证**:
1. 创建Cube + Light → Cube表面有光照效果（亮面暗面）
2. 旋转Cube → 光照随旋转变化
3. 无Light → Cube只有ambient颜色（暗色）
4. 多个Light → 光照叠加正确

---

### 3.2 透视投影修复

**问题**: 预览窗编辑模式下透视投影不生效

**根因分析**:
- 编辑模式摄像机可能使用正交投影而非透视投影
- 或者viewMatrix计算不正确
- `viewMatrix convention: forward must be negated (row 2 = -forward)`

**修复方案**:
- 确认编辑模式使用透视投影（Perspective projection）
  - FOV=60°, near=0.1, far=100
  - aspect_ratio = 预览窗width / 预览窗height
- 确认摄像机 viewMatrix 正确计算
  - `forward must be negated`
  - `row 2 = -forward`
- 确认 push constants 中 projectionMatrix + viewMatrix 正确传递
- 确认 render pipeline 使用正确的矩阵组合

**涉及文件**:
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs` — projection/view矩阵
- `engine/scripts/EditorScript.Presenter.cs` — 摄像机参数

**验证**:
1. 编辑模式 → 预览窗显示3D透视效果（远处物体更小）
2. 旋转摄像机 → 透视效果正确变化
3. 正交模式对比 → 明确看到透视/正交差异
4. 窗口resize → 投影矩阵aspect_ratio正确更新

---

### 3.3 OBJ模型导入渲染

**问题**: 需支持导入OBJ格式模型并渲染

**方案**:
- **使用 `tobj` crate**（轻量、纯Rust、支持MTL材质）
- OBJ加载流程：
  1. 用户通过FileBrowser或资产面板选择.obj文件
  2. Rust调用 `tobj::load_obj()` 解析OBJ文件
  3. 生成 mesh buffer（vertex + index）→ 提交到Vulkan
  4. 创建Entity + 设置mesh组件
  5. 渲染时使用加载的mesh buffer
- 新增FFI:
  - `scene_load_obj_model(path: *const c_char) -> EntityId` — 加载OBJ创建Entity
  - `scene_get_obj_mesh_info(entity_id) -> MeshInfo` — 获取模型信息（顶点数、面数）
- MTL材质支持：
  - 解析 .mtl 文件获取材质属性（diffuse_color、specular、texture）
  - 暂时只支持 diffuse_color（无纹理）
- 资产面板"模型"标签页显示已导入的OBJ模型

**涉及文件**:
- `engine/Cargo.toml` — 增加 tobj 依赖
- 新增 `engine/geometry/src/obj_loader.rs` — OBJ加载
- `engine/geometry/src/primitive.rs` — mesh生成扩展
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs` — 渲染加载的mesh
- `engine/scripts/EditorScript.Presenter.cs` — OBJ导入触发
- `engine/scripts/EditorScript.View.cs` — 资产面板模型标签页

**验证**:
1. FileBrowser选择.obj文件 → 模型出现在预览窗
2. 模型光照正确（有明暗面）
3. 模型大小合适（自动缩放或居中）
4. 多面模型（如torus、icosphere）渲染正确
5. 模型旋转/缩放与Cube相同操作

---

## 五、阶段四：动画和高级UI（预估2-3周）

> 目标：编辑器体验提升 — 动画系统、新组件、trace交互

### 4.1 显式动画和隐式动画

**问题**: UI需要显式动画和隐式动画，默认使用隐式动画

**现状**:
- `animation.rs` + `animation_engine.rs` 已有基本结构
- AnimationCurve: Linear/EaseIn/EaseOut/EaseInOut/Spring/Bounce
- create/start/pause/cancel/update API 已实现

**扩展方案**:
- **隐式动画（Implicit Animation）**:
  - Widget属性变化时自动触发动画
  - 默认曲线: EaseInOut, 默认时长: 250ms
  - 支持属性: opacity, position, size, border_radius, color
  - 实现: `Widget::set_property()` → AnimationEngine自动创建过渡动画
  - 新增FFI: `ui_widget_set_implicit_animation_duration(widget_id, duration)`
  - 新增FFI: `ui_widget_set_implicit_animation_curve(widget_id, curve)`
- **显式动画（Explicit Animation）**:
  - 用户手动创建、控制动画
  - 支持序列动画、并行动画、重复动画
  - 已有API: `create_animation/start/pause/cancel`
  - 新增: `ui_animation_set_on_complete_thunk_ptr(animation_id, thunk)` → 完成回调
  - 新增: `ui_create_animation_sequence(animations[])` → 序列动画
- **为现有UI组件添加基本动画效果**:
  - Button: 按下 → scale(0.95)隐式动画，松开 → scale(1.0)隐式动画
  - Panel: 显示/隐藏 → opacity隐式动画(0→1/1→0)
  - Dialog: 弹出 → scale(0.8→1.0)+opacity(0→1)显式动画
  - TabWidget: 切换 → opacity隐式动画
  - Dropdown: 展开 → height隐式动画(0→max_height)
  - PopupMenu: 弹出 → opacity(0→1)+scale(0.9→1.0)显式动画

**涉及文件**:
- `engine/ui/src/animation.rs` — 隐式动画扩展
- `engine/ui/src/animation_engine.rs` — 隐式动画触发
- `engine/ui/src/widget.rs` — set_property → 隐式动画钩子
- `engine/ui/src/widgets/*.rs` — 各组件动画效果
- `engine/ui/src/ffi/animation.rs` — 新FFI函数
- `engine/scripts/UI.NewWidgets.cs` — C#动画API

**验证**:
1. Button按下 → 缩小动画（250ms），松开 → 恢复动画
2. Dialog弹出 → 缩放+透明度动画
3. Dropdown展开 → 高度动画
4. 自定义隐式动画时长和曲线
5. 显式动画序列播放正确

---

### 4.2 Image组件（已在1.8中规划）

见阶段一 1.8 节。Image组件是阶段一的基础修复中需要的，但完整功能（缩放模式、texture加载管线）在阶段四完善。

---

### 4.3 Slider组件

**问题**: 部分属性除了可以用input修改，还可以使用滑块组件修改

**方案**:
- 新增 Slider Widget:
  - 水平/垂直方向
  - min/max/step/value 属性
  - 拖拽调节 + InputField精确输入
  - 外观: 轨道（灰色条）+ thumb（圆形/方形指示器）+ 值显示
  - 事件: OnValueChanged(float)
- FFI:
  - `ui_create_slider(direction, min, max, step, value) -> WidgetId`
  - `ui_slider_set_value(slider_id, value)`
  - `ui_slider_get_value(slider_id) -> float`
  - `ui_slider_set_on_change_thunk_ptr(slider_id, thunk_ptr, context)`
- Inspector中使用Slider：
  - Position X/Y/Z: InputField + Slider(min=-100, max=100, step=0.1)
  - Rotation X/Y/Z: InputField + Slider(min=-180, max=180, step=1)
  - Scale X/Y/Z: InputField + Slider(min=0.1, max=10, step=0.1)
  - Light intensity: InputField + Slider(min=0, max=1, step=0.01)

**涉及文件**:
- `engine/ui/src/widgets/slider.rs` — 新增
- `engine/ui/src/ffi/slider.rs` — 新增
- `engine/ui/src/widgets/mod.rs` — 注册
- `engine/ui/src/thunk_manager.rs` — PendingCallback::SliderChange
- `engine/scripts/UI.NewWidgets.cs` — Slider C#类
- `engine/scripts/EditorScript.View.cs` — 属性面板使用Slider

**验证**:
1. 创建Slider → 拖拽thumb → 值变化
2. min/max/step正确约束值范围
3. InputField显示当前值 → 手动输入 → Slider同步
4. Inspector中属性使用Slider + InputField组合

---

### 4.4 Trace泳道图交互增强

**问题**: trace泳道图需要：展开看函数耗时、时间轴拖动、鼠标hover自动放大附近±100ms

**方案**:
- `trace_viewer_demo` 增强：
  - 泳道可展开/折叠 → 点击泳道标题 → 展开显示子函数耗时
  - 时间轴可拖动 → 鼠标拖拽水平滚动
  - hover自动放大 → 鼠标悬浮时，以鼠标为中心放大±100ms范围
  - zoom级别: 1ms/5ms/10ms/50ms/100ms/500ms/1s
  - 点击泳道条 → 显示详细信息（函数名、耗时、开始时间）
- 新增UI组件用于trace viewer:
  - `TimelineWidget` — 时间轴（可拖动、可缩放）
  - `SwimlaneWidget` — 泳道条（可展开）

**涉及文件**:
- `engine/examples/src/trace_viewer_demo.rs` — 增强
- `engine/ui/src/widgets/*.rs` — TimelineWidget/SwimlaneWidget
- `engine/ui/src/animation.rs` — hover动画效果

**验证**:
1. 加载trace文件 → 泳道图正确显示
2. 点击泳道标题 → 展开/折叠
3. 鼠标hover → 时间轴以鼠标为中心放大±100ms
4. 拖拽时间轴 → 水平滚动
5. 点击泳道条 → 显示详细信息

---

### 4.5 日志规则整改

**问题**: 只有关键点位和用户主动调用时才打日志

**方案**:
- 删除所有非关键日志（Debug/Trace级别）
- 保留日志的点位：
  - 错误发生：Rust panic、C#异常、Vulkan错误
  - 用户主动操作：保存项目、加载模型、热重载
  - 关键状态变化：GameState切换、Entity创建/删除
- 日志级别策略：
  - Trace: 删除所有
  - Debug: 删除所有
  - Info: 仅关键状态变化
  - Warn: 仅可恢复的异常
  - Error: 仅不可恢复的错误
  - Fatal: 仅崩溃前

**涉及文件**:
- `engine/ui/src/*.rs` — 删除Debug/Trace日志
- `engine/rhi-vulkan/src/*.rs` — 删除Debug/Trace日志
- `engine/scripting/src/*.rs` — 删除Debug/Trace日志
- `engine/core/src/*.rs` — 删除Debug/Trace日志

**验证**:
1. 正常运行 → 日志文件只有Info/Warn/Error级别
2. 保存项目 → Info日志记录"项目已保存"
3. 无操作 → 无日志输出（不持续打日志）
4. 错误发生 → Error日志正确记录

---

## 六、交叉依赖关系

某些需求之间存在依赖，需要按顺序实施：

```
阶段一基础修复 → 阶段二UI重构 → 阶段三渲染修复 → 阶四动画高级UI

关键依赖链：
1. SplitView(2.4) → 资产面板重构(2.3) → 属性面板标签页(2.2)
2. Image组件(1.8) → UITest截图预览(1.7)
3. emoji显示(1.4) → Checkbox(阶段四)
4. 光照修复(3.1) → OBJ导入(3.3)
5. Slider(4.3) → 属性面板标签页(2.2)完善
6. 隐式动画(4.1) → 各UI组件动画效果
7. Cursor API(2.4) → SplitView拖动交互
8. MVP重构(2.1) → 所有其他UI重构（前置条件）
```

---

## 七、已完成的修复

以下bug已在之前session中修复，不需要重复工作：

| 修复 | 文件 | 说明 |
|------|------|------|
| FontAtlas死锁 | `input_field.rs` draw() | `canvas.get_font_atlas()` 避免重入锁 |
| Button thunk回调 | `event_dispatcher.rs` | Button Tap 不受 event.stopped 影响 |
| UITestRunner全面重写 | `UITestRunner.cs` | 清理Dialog+ScrollView+点击路径 |
| z-index layer继承 | `widget_tree.rs` | child继承parent layer |
| Dialog auto-sizing | `dialog.rs` | 自动扩展高度 |
| global_click排除列表 | `event_dispatcher.rs` | GLOBAL_CLICK_EXCLUDE const array |
| 递归measure_and_layout修复 | layout相关 | 改为measure() only |
| 防重复创建Dialog | `UITestRunner.cs` | _configDialogId != 0 return |
| 诊断日志移除 | layout相关 | layout_dialog_children诊断日志 |
| Release build成功 | Rust+C# | 0 errors编译通过 |

---

## 八、硬约束（不变）

1. 所有 `#[unsafe(no_mangle)] pub extern "C"` 函数签名必须保持不变
2. C#回调委托必须存储为static字段（不能local，GC会回收）
3. **NEVER trigger thunk callbacks inside WidgetTree.lock()** — 使用 `queue_callback()` + `flush_pending_callbacks()`
4. **NEVER use Button for menu items** — 使用 PopupMenu with shortcuts and separators
5. **NEVER define delegates or FfiContext fields outside UI.cs main file**
6. **NEVER use static Labels for tree structures** — use TreeView/TreeNode
7. **NEVER duplicate static fields across partial class files**
8. Render layers: Background(0) Content(1) Popup(2) Overlay(3)
9. Push constant std430: vec3 16-byte aligned, range=128B
10. viewMatrix convention: forward must be negated (row 2 = -forward)
11. parking_lot::Mutex 不可重入 — 同线程二次获取同一锁会死锁
12. Mono mcs compiler — 无 LINQ、async、nullable reference types
13. KeyCode值: A=1..Z=26, 0-9=27-36, Space=37, Enter=38, ESC=39, Backspace=40...
14. Modifier bitmask: Shift=bit0(1), Ctrl=bit1(2), Alt=bit2(4)
15. GameState枚举: Editing(0), Running(1), Paused(2)

---

## 九、新增Widget/FFI一览

| Widget | Rust文件 | FFI文件 | C#文件 | 阶段 |
|--------|----------|---------|---------|------|
| Image | `widgets/image.rs` | `ffi/image.rs` | `UI.NewWidgets.cs` | 一 |
| Slider | `widgets/slider.rs` | `ffi/slider.rs` | `UI.NewWidgets.cs` | 四 |
| Cursor Shape API | — | `ffi/misc.rs` | `UI.cs` | 二 |
| 隐式动画API | — | `ffi/animation.rs` | `UI.NewWidgets.cs` | 四 |
| OBJ加载API | — | `scene_ffi.rs` | `EditorScript.cs` | 三 |

---

## 十、风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| MVP重构改动面太大 | 可能破坏现有功能 | 先重构State.cs，逐步迁移View/Presenter |
| SplitView嵌套布局复杂 | 事件传递、hit_test可能冲突 | 先单层SplitView验证，再嵌套 |
| emoji需要多字体支持 | FontAtlas架构需改动 | 先单色emoji，后期彩色 |
| OBJ加载需要tobj依赖 | 新crate引入风险 | tobj是轻量纯Rust，风险低 |
| 隐式动画与现有渲染管线冲突 | draw()调用频率变化 | AnimationEngine独立update，不影响render |
| 光照修复涉及shader改动 | shader编译/调试困难 | 先Lambertian，逐步增强 |
| 窗口最大化bug根因不明 | 可能是多处问题叠加 | 系统排查resize链路 |

---

## 十一、实施节奏建议

### 第1周：阶段一基础修复（前半）

1. ESC退出 (1.1) — 1天
2. 高亮松开消失 (1.6) — 1天
3. 菜单文字不全 (1.3) — 1天
4. 光标显示 (1.5) — 2天

### 第2周：阶段一基础修复（后半）+ MVP重构

5. 窗口最大化 (1.2) — 2天（需要排查）
6. emoji显示 (1.4) — 2天
7. MVP重构 (2.1) — 3天
8. Image组件 (1.8) — 2天

### 第3周：阶段二UI架构重构

9. 属性面板标签页 (2.2) — 3天
10. SplitView拖动 (2.4) + Cursor API — 3天
11. 资产面板重构 (2.3) — 2天

### 第4周：阶段二完成 + 阶段三开始

12. 脚本绑定流程 (2.5) — 2天
13. 脚本编辑器返回bug (1.10) — 1天
14. hot reload修复 (2.6) — 1天
15. 光照修复 (3.1) — 2天

### 第5周：阶段三完成 + 阶段四开始

16. 透视投影 (3.2) — 1天
17. OBJ导入 (3.3) — 3天
18. 隐式/显式动画 (4.1) — 3天

### 第6周：阶段四完成

19. Slider组件 (4.3) — 2天
20. Trace泳道图 (4.4) — 2天
21. 日志整改 (4.5) — 1天
22. UI测试排除隐形 (1.9) — 1天
23. 全面集成测试 — 2天

---

## 十二、QA验证通用流程（每个阶段完成后）

1. **Rust构建**: `cargo build --release --features mono` → 0 errors
2. **C#编译**: `powershell -File build_mono.ps1` → DLL > 0 bytes
3. **编辑器启动**: `cargo run --release --features mono --bin mono_editor_demo` → 无崩溃
4. **对话框交互**: 点击进入主界面 → 预览窗显示Entity（不黑屏）
5. **LSP诊断**: 所有修改的Rust/C#文件无type error
6. **无副作用**: 修改不破坏已有功能
7. **新增Widget FFI签名**: 所有新增FFI函数签名正确且不破坏已有签名