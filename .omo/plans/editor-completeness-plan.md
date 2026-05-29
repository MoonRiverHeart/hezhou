# Hezhou编辑器完善规划 — 对比Unity编辑器差距分析

> 生成时间: 2026-05-22
> 基准: Unity Editor 2022 LTS

---

## 一、当前编辑器功能盘点

### 1.1 已有面板

| 面板 | 位置 | 尺寸 | 完成度 | 代码位置 |
|------|------|------|--------|----------|
| 顶部工具栏 | y=0, 全宽 | 40px | 30% | View.cs:65 |
| 项目结构面板 | 左侧 | 250px宽 | 30% | View.cs:92 |
| 资产管理面板 | 左下 | 250×200 | 10% | View.cs:96 |
| 游戏预览面板 | 中央 | 动态 | 50% | View.cs:100 |
| 属性编辑面板 | 右侧 | 250px宽 | 40% | View.cs:110 |
| 状态栏 | 底部 | 全宽×40px | 80% | View.cs:138 |
| 脚本编辑器面板 | 替换中央 | 动态 | 20% | View.cs:339 |
| 工作目录对话框 | 居中弹窗 | 500×400 | OK | View.cs:13 |

### 1.2 已有交互

- Entity射线拾取 (Editing模式点击PreviewWindow)
- 属性编辑 (5种类型: Float/Float3/String/Bool/Int, Enum不支持)
- 摄像机控制 (Running模式: 鼠标拖拽旋转+方向键移动)
- Entity创建 (空Entity/Cube/Sphere/Plane/Cylinder/Light)
- 脚本绑定管理 (Dropdown选择+添加+移除+启禁)
- TreeView节点选中/展开折叠
- 菜单PopupMenu弹出
- 窗口resize动态布局更新
- Hot Reload脚本热重载
- 文件浏览加载脚本

### 1.3 已有Widget (18种)

VStack, HStack, Button, Label, Panel, PreviewWindow, TextEdit, List, ListItem, Dropdown, InputField, TabWidget, TreeView, TreeNode, PopupMenu, GridView, Dialog, FileBrowser

### 1.4 菜单 (大部分空实现!)

| 菜单 | 菜单项 | 快捷键 | 实际功能 |
|------|--------|--------|----------|
| 文件 | 新建场景 | Ctrl+N | **空实现** |
| 文件 | 新建脚本 | 无 | 打开脚本编辑器 |
| 文件 | 退出 | 无 | **空实现** |
| 打开 | 打开场景/项目/资源 | 无 | **空实现** |
| 保存 | 保存场景 | Ctrl+S | **空实现** |
| 保存 | 保存全部/另存为 | 无 | **空实现** |

### 1.5 API已存在但UI未接入的功能

| API | 功能 | 难度 | 代码位置 |
|-----|------|------|----------|
| ProjectCreateNew/Load/Save | 项目创建/加载/保存 | 低 | asset_ffi.rs |
| ProjectSyncToScene/FromScene | 项目↔场景双向同步 | 低 | project.rs |
| SceneRemoveEntity | 删除Entity | 低 | scene_ffi.rs |
| SceneSetScriptBindingEnabled | 启禁脚本绑定 | 低 | scene_ffi.rs |
| AssetLibrary分类浏览 | 分类浏览资产 | 低 | asset_library.rs |
| SceneDestroy | 销毁场景 | 低 | scene_ffi.rs |
| SceneRotateEntity | Y轴旋转Entity | 低 | scene_ffi.rs |
| World set/get_parent | 父子层级 | 中 | ffi.rs |

---

## 二、与Unity编辑器差距分析

### 2.1 核心差距（按优先级排序）

#### P0 — 编辑器无法正常工作的基础缺失

| # | 功能 | Unity对应 | 当前状态 | 影响 |
|---|------|-----------|----------|------|
| 1 | **撤销/重做系统** | Ctrl+Z/Y | 完全缺失 | 编辑器体验的基础，没有Undo任何修改都不可逆 |
| 2 | **Entity删除** | Delete键 | API存在，UI无入口 | 无法删除创建的Entity |
| 3 | **项目保存/加载** | Ctrl+S/N | 14个Project API未接入 | 无法保存/恢复工作进度 |
| 4 | **菜单功能实现** | 全功能菜单 | 6个菜单项空实现 | 所有菜单操作都不工作 |

#### P1 — 编辑效率严重受限

| # | 功能 | Unity对应 | 当前状态 | 影响 |
|---|------|-----------|----------|------|
| 5 | **ScrollView** | 所有面板 | 完全缺失 | 内容超出面板时无法滚动查看 |
| 6 | **SplitView (可拖拽分割线)** | 所有面板边界 | 完全缺失 | 面板尺寸固定，无法拖拽调整 |
| 7 | **Slider (滑动条)** | Inspector数值属性 | 完全缺失 | 属性值只能手动输入数字，无拖拽调节 |
| 8 | **Checkbox (勾选框)** | Inspector布尔属性 | 完全缺失 | bool属性用InputField输入"true"/"false" |
| 9 | **Scene视图 (3D编辑视角)** | Scene窗口 | 只有Running预览 | Editing模式下不能自由浏览3D场景 |
| 10 | **Entity多选** | Ctrl+Click | 只有单选 | 无法同时选中多个Entity |
| 11 | **父子层级** | Hierarchy父子 | API存在，UI未接入 | Entity无法嵌套父子关系 |

#### P2 — 专业编辑器体验缺失

| # | 功能 | Unity对应 | 当前状态 | 影响 |
|---|------|-----------|----------|------|
| 12 | **Gizmo工具** | 移动/旋转/缩放Gizmo | 完全缺失 | Transform只能通过InputField修改 |
| 13 | **拖拽系统** | Drag&Drop | 完全缺失 | 无法拖拽排序、拖拽创建、拖拽赋值 |
| 14 | **右键上下文菜单** | 右键菜单 | PopupMenu存在但未接入右键 | 无右键操作 |
| 15 | **Console/Log窗口** | Console面板 | 完全缺失 | 无错误/警告/日志查看 |
| 16 | **游戏暂停** | Pause按钮 | API定义但UI未实现 | 只能运行/停止，不能暂停 |
| 17 | **搜索/过滤** | 搜索框 | 完全缺失 | 资产/Entity/属性无搜索 |
| 18 | **资产导入** | Import Settings | 完全缺失 | 无纹理/模型/音频导入 |
| 19 | **资产预览缩略图** | Project缩略图 | GridView只显示文字 | 资产无视觉预览 |
| 20 | **Component系统** | AddComponent | 完全缺失 | 无组件化架构 |

#### P3 — 高级功能（长期目标）

| # | 功能 | Unity对应 | 当前状态 |
|---|------|-----------|----------|
| 21 | 预制体(Prefab)系统 | Prefabs | 完全缺失 |
| 22 | 多场景编辑 | Multi-Scene | 完全缺失 |
| 23 | 动画编辑器 | Animator/Timeline | 完全缺失 |
| 24 | 物理系统 | PhysX/Collider | 完全缺失 |
| 25 | 音频系统 | AudioSource | 完全缺失 |
| 26 | PBR材质/阴影 | Material/Shadow | 只有Flat着色+1个方向光 |
| 27 | 天空盒 | Skybox | 完全缺失 |
| 28 | 2D系统 | SpriteRenderer | 完全缺失 |

### 2.2 Widget差距

| 优先级 | 缺失Widget | Unity用途 | 预估工作量 |
|--------|-----------|-----------|------------|
| **高** | ScrollView | 所有面板滚动 | 3天 |
| **高** | ScrollBar | ScrollView组成部分 | 2天 |
| **高** | SplitView | 面板可拖拽分割 | 3天 |
| **高** | Slider | Inspector数值调节 | 2天 |
| **高** | Checkbox | Inspector布尔开关 | 1天 |
| **高** | Drag&Drop基础设施 | 资产拖拽、排序 | 5天 |
| **中** | ColorPicker | 材质/灯光颜色 | 3天 |
| **中** | SpinBox | Inspector数值精确调节 | 2天 |
| **中** | ImageButton | 工具栏图标按钮 | 1天 |
| **中** | Tooltip | 悬浮提示 | 1天 |
| **中** | ProgressBar | 加载进度 | 1天 |
| **低** | Accordion | Inspector分组折叠 | 2天 |
| **低** | Toast/Notification | 操作反馈 | 1天 |

---

## 三、分阶段完善计划

### 第一阶段：基础可用 (预估2周)

> 目标：编辑器基本功能闭环 — 能创建、编辑、保存、删除

#### 1.1 撤销/重做系统 (P0-1)
- 设计Command模式: CreateEntityCommand, DeleteEntityCommand, SetPropertyCommand, TransformCommand
- 实现UndoStack (最大100步) + RedoStack
- Ctrl+Z/Y快捷键处理
- C#层: UndoManager类 + RegisterCommand/Undo/Redo API
- Rust层: scene_ffi新增 undo/redo 函数

#### 1.2 Entity删除功能 (P0-2)
- Delete键删除选中Entity
- TreeView右键菜单 → "删除"
- 删除前确认Dialog
- Undo支持 (DeleteEntityCommand)

#### 1.3 项目保存/加载 (P0-3)
- 接入ProjectCreateNew/Load/Save API
- Ctrl+S → 保存当前项目
- Ctrl+N → 新建项目 (Dialog确认)
- 菜单"打开场景/项目" → FileBrowser选择.project.json
- 状态栏显示实际项目名称 (替换硬编码"项目: 未命名")
- 启动时自动加载上次项目

#### 1.4 菜单功能实现 (P0-4)
- 文件菜单: 新建场景、新建脚本、退出(关闭窗口)
- 打开菜单: 打开项目(FileBrowser)、打开场景
- 保存菜单: 保存项目(Ctrl+S)、另存为(FileBrowser)
- 所有菜单actionId对应实际逻辑

#### 1.5 暂停功能 (P2-16, 低成本)
- 运行按钮旁添加暂停按钮 (或双状态: 运行/暂停)
- GameState切换: Editing→Running→Paused→Editing
- 暂停时: 场景不update但渲染继续, 绿色边框

### 第二阶段：编辑效率提升 (预估3周)

> 目标：编辑体验接近Unity基础水平 — 可视化编辑、面板灵活

#### 2.1 ScrollView + ScrollBar (P1-5)
- 提取TextEdit内部滚动逻辑为通用ScrollView
- ScrollView包含: content容器 + vertical/horizontal ScrollBar
- ScrollBar: 拖拽thumb滚动、点击轨道跳转、鼠标滚轮
- 应用到: 项目结构面板、属性面板、资产面板

#### 2.2 SplitView (P1-6)
- 可拖拽分割线面板容器
- 支持水平/垂直分割
- 拖拽时实时调整相邻面板尺寸
- 最小宽度限制 (防止面板缩到消失)
- 应用到: 左侧面板↔预览↔右侧面板、项目树↔资产面板

#### 2.3 Slider (P1-7)
- 水平/垂直滑动条
- min/max/step/value
- 拖拽调节 + 精确输入框
- Inspector中: Position XYZ各一个Slider、Rotation、Scale
- 事件: OnValueChanged

#### 2.4 Checkbox (P1-8)
- 勾选/取消勾选框
- 事件: OnCheckedChanged
- Inspector中: 替换bool属性的InputField
- 轻量Widget，1天可完成

#### 2.5 Scene视图 (P1-9, 架构级)
- Editing模式下3D场景自由浏览:
  - 轨道摄像机 (鼠标右键拖拽旋转)
  - 中键平移
  - 滚轮缩放
  - 选中Entity时摄像机聚焦
- 这是最大的架构差距，需要:
  - 轨道摄像机组件 (orbit_camera)
  - Editing模式下的摄像机更新逻辑
  - PreviewWindow与Scene视图的分离 (当前是同一个)

#### 2.6 Entity多选 + 父子层级 (P1-10,11)
- Ctrl+Click多选
- Shift+Click范围选择
- 属性面板显示多选共同属性
- TreeView拖拽设置父子关系
- Hierarchy面板重命名 (双击节点)
- World set_parent/get_children接入TreeView

### 第三阶段：专业体验 (预估4周)

> 目标：接近Unity编辑器日常使用体验

#### 3.1 Gizmo工具 (P2-12)
- 3种Gizmo模式: 移动(W)、旋转(E)、缩放(R)
- 工具栏切换 Gizmo模式按钮组
- Gizmo渲染: 独立render pass, 半透明箭头/圆环/方块
- 鼠标拖拽Gizmo → 实时修改Entity Transform
- 与属性面板双向联动

#### 3.2 拖拽系统 (P2-13)
- 事件层: DragBegin/DragMove/DragEnd
- 视觉层: 拖拽ghost widget跟随鼠标
- 应用场景:
  - 资产拖拽到场景 → 创建Entity
  - Hierarchy拖拽 → 重排序/设置父子
  - 资产拖拽到属性 → 赋值(纹理→材质)

#### 3.3 右键菜单 (P2-14)
- Hierarchy右键: 创建Entity、删除、复制、粘贴
- 资产面板右键: 导入、删除、重命名
- 预览面板右键: 摄像机重置、截图

#### 3.4 Console窗口 (P2-15)
- 底部新增Console面板 (可折叠)
- 显示: dfx日志 + 脚本print输出 + 错误堆栈
- 过滤: All/Info/Warning/Error
- 搜索框
- 点击错误跳转到脚本行

#### 3.5 搜索/过滤 (P2-17)
- Hierarchy搜索框 (过滤Entity名称)
- 资产面板搜索框 (过滤资产名称/类型)
- 属性面板搜索 (过滤属性名)

#### 3.6 资产预览 (P2-19)
- GridView显示缩略图 (Cube/Sphere等用3D渲染缩略图)
- 属性面板底部: 选中Entity的3D预览 (小窗口)

### 第四阶段：高级功能 (长期)

- 资产导入管线 (纹理PNG→Vulkan texture, OBJ模型→mesh buffer)
- Component系统 (AddComponent/RemoveComponent UI)
- Prefab系统 (保存Entity模板 + 实例化)
- 动画编辑器 (关键帧时间线)
- 物理系统 (Collider/Rigidbody组件)
- PBR材质 + 阴影 + 天空盒

---

## 四、实施顺序建议（近期工作）

按"快速见效+低成本优先"原则排序:

### 立即可做 (1-2天/项, 共约7天)

1. **Entity删除** — Delete键 + TreeView右键菜单 + SceneRemoveEntity API接入
2. **项目保存/加载** — Ctrl+S/N + Project API接入 + 菜单实现
3. **Checkbox Widget** — bool属性替换InputField
4. **暂停功能** — 运行/暂停按钮 + GameState切换
5. **菜单功能实现** — 填充6个空实现的菜单项

### 下一批 (3-5天/项, 共约15天)

6. **ScrollView + ScrollBar** — 提取TextEdit滚动逻辑
7. **Slider Widget** — Inspector数值拖拽调节
8. **SplitView** — 可拖拽面板分割
9. **撤销/重做系统** — Command模式 + UndoStack
10. **Entity多选** — Ctrl+Click + Shift+Click

### 中期 (5-10天/项)

11. **Scene视图** — 轨道摄像机 + Editing模式3D浏览
12. **Gizmo工具** — 移动/旋转/缩放拖拽操作
13. **Console窗口** — 日志面板
14. **拖拽系统** — DragBegin/Move/End事件

---

## 五、技术实现要点

### 5.1 新Widget实现模式 (4文件模式)

每个新Widget需要:
1. `ui/src/widgets/<name>.rs` — Widget trait实现
2. `ui/src/ffi/<name>.rs` — FFI extern "C"函数
3. `ui/src/widgets/mod.rs` — 导出注册
4. `ui/src/thunk_manager.rs` — 回调类型注册 (PendingCallback::XxxSelect/Change/Click)
5. `scripts/UI.NewWidgets.cs` — C#包装类
6. `scripts/UI.cs` — FfiContext字段 + 委托注册

### 5.2 撤销/重做架构

```
UndoManager (C# side)
├── UndoStack: List<ICommand> (max 100)
├── RedoStack: List<ICommand>
├── RegisterCommand(cmd) → Push UndoStack, Clear RedoStack
├── Undo() → Pop UndoStack, Execute Reverse, Push RedoStack
└── Redo() → Pop RedoStack, Execute Forward, Push UndoStack

ICommand interface:
├── Execute() — 正向操作
├── Undo() — 反向操作
└── Description — 显示文字 ("创建Cube", "删除Entity_5")

Command types:
├── CreateEntityCommand — Execute: create, Undo: remove
├── DeleteEntityCommand — Execute: remove, Undo: restore (save snapshot)
├── SetPropertyCommand — Execute: set new value, Undo: set old value
├── TransformCommand — Execute: new transform, Undo: old transform
```

### 5.3 Scene视图架构

```
Editing模式:
├── OrbitCamera (新增Rust组件)
│   ├── target: Vec3 (聚焦点)
│   ├── distance: f32 (缩放距离)
│   ├── yaw/pitch: f32 (轨道角度)
│   ├── move_speed: f32
│   ├── zoom_speed: f32
│   └── Mouse Right Drag → yaw/pitch
│   ├── Mouse Middle Drag → pan
│   └── Mouse Wheel → zoom
│
├── EditingPreviewWindow (与RunningPreviewWindow分离)
│   ├── 不渲染Entity自动旋转
│   ├── 渲染Gizmo (移动/旋转/缩放箭头)
│   └── 点击拾取 → 选中Entity + 高亮outline
│
└── 状态切换
    ├── Editing → OrbitCamera激活
    ├── Running → FPS Camera激活 (现有)
    └── ESC → 回到Editing
```

---

## 六、风险与约束

| 风险 | 影响 | 缓解 |
|------|------|------|
| ScrollView与现有TextEdit滚动逻辑冲突 | 重构TextEdit | 先提取共用ScrollEngine，再重构 |
| SplitView拖拽与事件分发冲突 | 需要新增Drag事件类型 | 事件层先增加DragBegin/Move/End |
| Scene视图需要分离PreviewWindow渲染 | 大改动 | 先实现暂停功能，再逐步分离 |
| Gizmo渲染需要新render pass | 增加管线复杂度 | Gizmo作为UI层overlay渲染 |
| 拖拽系统是跨层架构改动 | 涉及UI+Event+渲染 | 分3步: 事件→视觉→交互 |

---

## 七、关键约束 (不变)

- 所有 `#[unsafe(no_mangle)] pub extern "C"` 函数签名必须保持不变
- C#回调委托必须存储为static字段（不能local，GC会回收）
- NEVER trigger thunk callbacks inside WidgetTree.lock()
- NEVER use Button for menu items — use PopupMenu
- NEVER use static Labels for tree structures — use TreeView/TreeNode
- Render layers: Background(0) Content(1) Popup(2) Overlay(3)
- Push constant std430: vec3 16-byte aligned, range=128B
- viewMatrix convention: forward must be negated (row 2 = -forward)

---

## 八、阶段一 QA 验证场景

### 任务1: Entity删除

**QA-1.1: Delete键删除选中Entity**
- 工具: 启动编辑器 (cargo run --release --features mono --bin mono_editor_demo)
- 步骤:
  1. 点击对话框进入主界面
  2. 点击预览窗中的Cube Entity → 属性面板显示Cube信息
  3. 按下Delete键
- 预期结果:
  - Cube从预览窗消失（不再渲染）
  - Cube从项目结构树的Entities分类下消失
  - 属性面板清空（无选中Entity）
  - 状态栏显示"已删除 Entity: Cube"

**QA-1.2: Delete键无选中Entity时无操作**
- 步骤: 不选中任何Entity，按下Delete键
- 预期结果: 无任何变化，不报错

**QA-1.3: 删除后Undo恢复**
- 步骤: 选中Cube → Delete删除 → Ctrl+Z撤销
- 预期结果: Cube恢复出现在预览窗和项目树中（需Undo系统完成后验证）

**QA-1.4: 树节点右键菜单删除**
- 步骤: 右键点击Entities下的Cube节点 → 弹出上下文菜单 → 点击"删除"
- 预期结果: 同QA-1.1，Cube被删除

**QA-1.5: 删除唯一Entity后场景不空崩溃**
- 步骤: 删除所有Entity (Cube+Plane+Light)
- 预期结果: 预览窗显示空白（无entity渲染），编辑器不崩溃，可继续操作

**QA-1.6: Rust构建验证**
- 工具: cargo build --release --features mono
- 预期结果: 0 errors, SceneRemoveEntity FFI函数在scene_ffi.rs中存在且签名不变

**QA-1.7: C# DLL编译验证**
- 工具: powershell -File build_mono.ps1
- 预期结果: DLL编译成功，size > 0

---

### 任务2: 项目保存/加载

**QA-2.1: Ctrl+S保存当前项目**
- 工具: 启动编辑器
- 步骤:
  1. 进入主界面，创建一个新Cube
  2. 按Ctrl+S
- 预期结果:
  - 状态栏显示"项目已保存"
  - 工作目录下生成 `.project.json` 文件
  - JSON文件包含3个entity (Cube+Plane+Light)的position/rotation/scale

**QA-2.2: 重新打开保存的项目**
- 步骤:
  1. 保存项目后关闭编辑器
  2. 重新启动编辑器
  3. 进入相同工作目录
- 预期结果:
  - 预览窗显示保存时的Entity（Cube+Plane+Light+新Cube）
  - 项目树包含所有保存的Entity
  - 属性面板可选中并编辑恢复的Entity

**QA-2.3: 保存空项目**
- 步骤: 删除所有Entity后Ctrl+S
- 预期结果: .project.json包含空entity列表，重新加载后预览窗空白不崩溃

**QA-2.4: Ctrl+N新建项目**
- 步骤: Ctrl+N → Dialog弹出"是否保存当前项目？" → 确认
- 预期结果: 场景清空，新.project.json创建，状态栏显示新项目名

**QA-2.5: 菜单保存/另存为**
- 步骤: 点击保存菜单 → 保存场景 → 另存为(FileBrowser选择路径)
- 预期结果: 项目保存到指定路径

**QA-2.6: Project JSON文件格式验证**
- 工具: 读取保存的.project.json
- 预期结果: JSON包含 name, path, settings(width/height/fps), entities数组(entity_id, name, position, rotation, scale, scripts)

**QA-2.7: Rust构建 + FFI签名不变**
- 预期结果: ProjectCreateNew/Load/Save/GetName/GetPath/IsLoaded/SyncToScene/SyncFromScene FFI签名不变

---

### 任务3: 菜单功能实现

**QA-3.1: 文件→新建场景**
- 步骤: 点击文件菜单 → 新建场景
- 预期结果: Dialog确认"是否保存？"→ 确认后场景清空，创建默认Cube+Plane+Light

**QA-3.2: 文件→新建脚本**
- 步骤: 点击文件菜单 → 新建脚本
- 预期结果: 切换到脚本编辑器面板，TextEdit显示空脚本模板

**QA-3.3: 文件→退出**
- 步骤: 点击文件菜单 → 退出
- 预期结果: 编辑器窗口关闭，进程退出

**QA-3.4: 打开→打开项目**
- 步骤: 点击打开菜单 → 打开项目 → FileBrowser选择.project.json文件
- 预期结果: 加载项目，Entity恢复到预览窗和树中

**QA-3.5: 保存→保存场景 (Ctrl+S)**
- 步骤: Ctrl+S快捷键
- 预期结果: 同QA-2.1，项目保存到.project.json

**QA-3.6: 保存→另存为**
- 步骤: 保存菜单 → 另存为 → FileBrowser选择新路径
- 预期结果: 项目复制到新路径保存

**QA-3.7: Ctrl+N/Ctrl+S按键处理**
- 工具: 读取OnKey回调代码
- 预期结果: OnKey中识别Ctrl+N(KeyCode=4)和Ctrl+S(KeyCode=13)组合键，调用对应函数

---

### 任务4: 暂停功能

**QA-4.1: 运行→暂停切换**
- 步骤:
  1. 点击"运行"按钮 → GameState切换到Running(蓝色边框)
  2. 点击"暂停"按钮 → GameState切换到Paused(绿色边框)
- 预期结果:
  - Running: Cube自动旋转，方向键移动摄像机
  - Paused: Cube停止旋转，摄像机不动，预览窗仍然渲染当前帧（不黑屏）
  - 边框颜色: Running=蓝色, Paused=绿色

**QA-4.2: 暂停→继续运行**
- 步骤: 暂停状态 → 再次点击运行按钮
- 预期结果: GameState切换回Running，Cube恢复旋转

**QA-4.3: 暂停→ESC退出到Editing**
- 步骤: 暂停状态 → 按ESC
- 预期结果: GameState切换到Editing(橙色边框)，摄像机恢复默认位置

**QA-4.4: Paused状态下Entity不可编辑**
- 步骤: 暂停状态下点击预览窗Entity
- 预期结果: 不触发Entity拾取（Paused模式下点击不选中Entity）

**QA-4.5: GameState API验证**
- 工具: set_game_state(2) + get_game_state() == 2
- 预期结果: Rust scene_ffi.rs SceneSetGameState/SceneGetGameState支持值2(Paused)

---

### 任务5: Checkbox Widget

**QA-5.1: Checkbox基本交互**
- 步骤:
  1. 创建Checkbox widget
  2. 点击Checkbox → 勾选状态
  3. 再次点击 → 取消勾选
- 预期结果:
  - 勾选时显示✓图标+文字
  - 取消勾选时显示□图标+文字
  - OnCheckedChanged回调触发，传递bool值

**QA-5.2: Inspector bool属性替换InputField**
- 步骤:
  1. 选中Entity → 属性面板显示属性列表
  2. 找到bool类型属性（如visible）
  3. 点击Checkbox勾选/取消勾选
- 预期结果:
  - bool属性不再用InputField输入"true"/"false"
  - Checkbox切换立即更新Entity属性值
  - 取消勾选 → Entity.visible=false → 预览窗中Entity消失

**QA-5.3: Checkbox FFI验证**
- 工具: cargo build --release
- 预期结果: ui_create_checkbox, ui_checkbox_set_checked, ui_checkbox_get_checked, ui_checkbox_set_on_change_thunk_ptr FFI函数存在且签名正确

**QA-5.4: Checkbox渲染验证**
- 预期结果: Checkbox渲染为 [✓] 文字 或 [□] 文字，背景色适当，点击区域准确

---

### 任务6: 脚本启用/禁用toggle

**QA-6.1: 脚本列表显示ON/OFF标签**
- 步骤:
  1. 选中带脚本的Entity
  2. 属性面板Scripts标签页显示脚本列表
- 预期结果:
  - 每个脚本行显示: 脚本名 + 类名 + [ON]绿色标签 或 [OFF]红色标签
  - [ON]/[OFF]标签可点击切换

**QA-6.2: 点击ON/OFF切换脚本启用状态**
- 步骤: 点击[ON]标签 → 变为[OFF]
- 预期结果:
  - SceneSetScriptBindingEnabled(entity, index, false) 被调用
  - 运行模式下该脚本不再执行
  - 再次点击[OFF] → 变为[ON] → 脚本恢复执行

**QA-6.3: Running模式下禁用脚本效果验证**
- 步骤:
  1. 给Entity附加RotationScript (让Cube旋转)
  2. 运行模式 → Cube旋转
  3. 点击[ON] → [OFF]
- 预期结果: Cube停止旋转

**QA-6.4: 多脚本Entity切换**
- 步骤: Entity有2个脚本 → 禁用第1个 → 第2个继续执行
- 预期结果: 只有禁用的脚本停止，其他脚本正常运行

---

### 通用验证流程（每个任务完成后必须执行）

1. **Rust构建**: `cargo build --release --features mono` → 0 errors
2. **C#编译**: `powershell -File build_mono.ps1` → DLL > 0 bytes
3. **编辑器启动**: `cargo run --release --features mono --bin mono_editor_demo` → 无崩溃
4. **对话框交互**: 点击进入主界面 → 预览窗显示Entity（不黑屏）
5. **LSP诊断**: 所有修改的Rust/C#文件无type error
6. **无副作用**: 修改不破坏已有功能（菜单弹出、TreeView选中、Entity拾取、属性编辑、摄像机控制均正常）