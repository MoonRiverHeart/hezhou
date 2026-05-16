# UI 系统架构设计

> 美观、简洁、高性能的跨平台 UI 框架，支持事件树、手势、动效、C# Thunk 调用

---

## 1. 设计目标

### 1.1 核心特性

- **美观简洁**：现代化设计风格，Material Design 风格基础
- **高性能**：利用 Thunk 函数指针，减少反射开销
- **跨平台**：Rust 核心 + 平台对接层 + C# 上层 API
- **事件树**：完整的事件冒泡、事件分发机制
- **手势系统**：点击、滑动、拖拽、缩放、长按
- **动效动画**：属性动画、过渡动画、曲线动画
- **DFX 集成**：性能监控、布局可视化、事件追踪

### 1.2 技术栈

```
┌─────────────────────────────────────────────────────────────────┐
│  C# UI Layer (Thunk 高性能调用)                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  UIControls.dll (NativeAOT)                                 ││
│  │    ├── Button, Label, Panel, ScrollView                    ││
│  │    ├── Animation, Gesture, Event                            ││
│  │    └── Thunk 导出: [UnmanagedCallersOnly]                   ││
│  └─────────────────────────────────────────────────────────────┘│
│                          ↓ Thunk 函数指针 (~20ns)               │
├─────────────────────────────────────────────────────────────────┤
│  Rust UI Core                                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  engine/ui/                                                  ││
│  │    ├── widget.rs      ← Widget trait + 控件树               ││
│  │    ├── event.rs       ← 事件系统 + 冒泡机制                  ││
│  │    ├── gesture.rs     ← 手势识别                            ││
│  │    ├── layout.rs      ← 布局计算                            ││
│  │    ├── canvas.rs      ← 绘图命令                            ││
│  │    ├── animation.rs   ← 动效系统                            ││
│  │    └── render.rs      ← 渲染数据生成                        ││
│  └─────────────────────────────────────────────────────────────┘│
│                          ↓ UI Render Data                       │
├─────────────────────────────────────────────────────────────────┤
│  RHI UI Layer                                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  engine/rhi/src/ui.rs                                       ││
│  │    ├── UIRenderTarget   ← 渲染目标                          ││
│  │    ├── UIDrawCommand    ← 绘图命令抽象                      ││
│  │    ├── UITexture        ← UI 资源                           ││
│  │    └── UIShader         ← UI shader                         ││
│  └─────────────────────────────────────────────────────────────┘│
│                          ↓ RHI 抽象                             │
├─────────────────────────────────────────────────────────────────┤
│  Vulkan UI Renderer                                             │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  engine/rhi-vulkan/src/ui_renderer.rs                       ││
│  │    ├── VulkanUIPipeline    ← Vulkan pipeline                ││
│  │    ├── VulkanUIFramebuffer ← Framebuffer                    ││
│  │    ├── VulkanUICommandBuffer← 命令缓冲                       ││
│  │    └── VulkanUITexture     ← 纹理上传                        ││
│  └─────────────────────────────────────────────────────────────┘│
│                          ↓ Vulkan API                           │
├─────────────────────────────────────────────────────────────────┤
│  Platform Integration                                           │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  engine/platform/src/ui.rs                                  ││
│  │    ├── TouchInput      ← 触摸事件                           ││
│  │    ├── KeyboardInput   ← 键盘事件                           ││
│  │    ├── WindowEvents    ← 窗口事件                           ││
│  │    └── HarmonyOS NAPI  ← HarmonyOS 对接                     ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. 模块架构

### 2.1 目录结构

```
engine/
├ ui/                        ← UI 核心
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           ← FFI 导出 + DFX 集成
│   │   ├── widget.rs        ← Widget trait + 控件基类
│   │   ├── widget_tree.rs   ← 控件树结构
│   │   ├── event.rs         ← 事件类型 + 事件系统
│   │   ├── event_dispatcher.rs ← 事件分发器 + 冒泡
│   │   ├── gesture.rs       ← 手势识别器
│   │   ├── gesture_recognizer.rs ← 手势系统实现
│   │   ├── layout.rs        ← 布局计算
│   │   ├── layout_engine.rs ← Flex/Grid/Stack 布局
│   │   ├── canvas.rs        ← 绘图上下文
│   │   ├── draw_command.rs  ← 绘图命令定义
│   │   ├── animation.rs     ← 动画系统
│   │   ├── animation_curve.rs ← 动画曲线
│   │   ├── property_animation.rs ← 属性动画
│   │   ├── render_data.rs   ← 渲染数据生成
│   │   └── style.rs         ← 样式系统
│   ├── tests/
│   │   ├── widget_test.rs
│   │   ├── event_test.rs
│   │   ├── gesture_test.rs
│   │   └── animation_test.rs
│   └── examples/
│       └── ui_demo.rs
│
├ rhi/src/ui.rs              ← RHI UI 抽象层
├ rhi-vulkan/src/ui_renderer.rs ← Vulkan UI 渲染器
├ platform/src/ui.rs         ← 平台输入对接
│
└ scripts/UIControls.cs      ← C# UI 控件库
```

---

## 3. 核心类型定义

### 3.1 Widget 控件基类

```rust
// ui/src/widget.rs

pub trait Widget: Send + Sync {
    fn id(&self) -> WidgetId;
    fn parent(&self) -> Option<WidgetId>;
    fn children(&self) -> &[WidgetId];
    
    fn layout(&self) -> &Layout;
    fn style(&self) -> &Style;
    fn state(&self) -> WidgetState;
    
    fn hit_test(&self, point: Point) -> bool;
    fn draw(&self, canvas: &mut Canvas);
    
    fn on_event(&mut self, event: &Event) -> EventResult;
}

#[repr(C)]
pub struct WidgetId {
    id: u64,
}

#[repr(C)]
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub anchor: Anchor,
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,
}

#[repr(C)]
pub struct Style {
    pub background_color: Color,
    pub border_radius: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub opacity: f32,
    pub shadow: Option<Shadow>,
}

#[repr(C)]
pub enum WidgetState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
    Focused,
}

#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[repr(C)]
pub struct Shadow {
    pub color: Color,
    pub offset: Point,
    pub blur_radius: f32,
}
```

### 3.2 事件类型定义

```rust
// ui/src/event.rs

#[repr(C)]
pub enum EventType {
    TouchBegin,
    TouchMove,
    TouchEnd,
    TouchCancel,
    
    Click,
    LongPress,
    DoubleClick,
    
    KeyDown,
    KeyUp,
    
    MouseEnter,
    MouseLeave,
    
    FocusGain,
    FocusLost,
    
    LayoutChanged,
    StyleChanged,
}

#[repr(C)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: u64,
    pub target: WidgetId,
    pub bubbles: bool,          // 是否冒泡
    pub cancelable: bool,       // 是否可取消
    pub stopped: bool,          // 是否已停止
    pub immediate_stopped: bool, // 是否立即停止
    
    // 事件数据
    pub data: EventData,
}

#[repr(C)]
pub union EventData {
    pub touch: TouchData,
    pub key: KeyData,
    pub mouse: MouseData,
    pub layout: LayoutData,
}

#[repr(C)]
pub struct TouchData {
    pub x: f32,
    pub y: f32,
    pub pointer_id: u32,
    pub pressure: f32,
}

#[repr(C)]
pub struct KeyData {
    pub keycode: u32,
    pub modifiers: u32,
    pub unicode_char: u32,
}

#[repr(C)]
pub enum EventResult {
    Ignored,       // 未处理
    Handled,       // 已处理，继续冒泡
    Stopped,       // 已处理，停止冒泡
    ImmediateStop, // 立即停止，不再冒泡
}

#[repr(C)]
pub struct EventPhase {
    pub capturing: bool,  // 捕获阶段
    pub bubbling: bool,   // 冒泡阶段
}
```

### 3.3 手势类型定义

```rust
// ui/src/gesture.rs

#[repr(C)]
pub enum GestureType {
    Tap,
    DoubleTap,
    LongPress,
    Pan,
    Swipe,
    Pinch,
    Rotation,
}

#[repr(C)]
pub struct Gesture {
    pub gesture_type: GestureType,
    pub state: GestureState,
    pub target: WidgetId,
    
    // 手势数据
    pub data: GestureData,
}

#[repr(C)]
pub enum GestureState {
    Possible,    // 可能识别
    Began,       // 开始识别
    Changed,     // 状态改变
    Ended,       // 识别完成
    Cancelled,   // 识别取消
    Failed,      // 识别失败
}

#[repr(C)]
pub union GestureData {
    pub tap: TapData,
    pub pan: PanData,
    pub pinch: PinchData,
    pub rotation: RotationData,
}

#[repr(C)]
pub struct TapData {
    pub x: f32,
    pub y: f32,
    pub tap_count: u32,
}

#[repr(C)]
pub struct PanData {
    pub start_x: f32,
    pub start_y: f32,
    pub current_x: f32,
    pub current_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

#[repr(C)]
pub struct PinchData {
    pub scale: f32,
    pub velocity: f32,
}

#[repr(C)]
pub struct RotationData {
    pub rotation: f32,  // 弧度
    pub velocity: f32,
}
```

### 3.4 动画类型定义

```rust
// ui/src/animation.rs

#[repr(C)]
pub enum AnimationCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Spring,
    Bounce,
    Custom,  // 自定义曲线函数指针
}

#[repr(C)]
pub struct Animation {
    pub id: AnimationId,
    pub duration: f32,      // 秒
    pub delay: f32,        // 秒
    pub curve: AnimationCurve,
    pub repeat_count: u32, // 0 = 无限循环
    pub auto_reverse: bool,
    pub running: bool,
    pub paused: bool,
    
    // 动画目标
    pub target: WidgetId,
    pub property: AnimatedProperty,
    pub from_value: f32,
    pub to_value: f32,
}

#[repr(C)]
pub enum AnimatedProperty {
    Opacity,
    Width,
    Height,
    X,
    Y,
    Scale,
    Rotation,
    BorderRadius,
    Custom(u32),  // 自定义属性 ID
}

#[repr(C)]
pub struct AnimationId {
    id: u64,
}

// 动画回调（Thunk）
pub type AnimationCallback = extern "C" fn(AnimationId, AnimationState);

#[repr(C)]
pub enum AnimationState {
    Started,
    Updated,
    Completed,
    Cancelled,
}
```

---

## 4. 控件树结构

### 4.1 树结构定义

```rust
// ui/src/widget_tree.rs

use std::collections::HashMap;
use parking_lot::Mutex;

pub struct WidgetTree {
    root: Option<WidgetId>,
    nodes: HashMap<WidgetId, WidgetNode>,
    parent_map: HashMap<WidgetId, WidgetId>,      // child -> parent
    children_map: HashMap<WidgetId, Vec<WidgetId>>, // parent -> children
}

struct WidgetNode {
    widget: Box<dyn Widget>,
    dirty_flags: DirtyFlags,
    render_data: Option<RenderData>,
}

#[repr(C)]
pub struct DirtyFlags {
    layout_dirty: bool,
    style_dirty: bool,
    render_dirty: bool,
    children_dirty: bool,
}

pub struct RenderData {
    draw_commands: Vec<DrawCommand>,
    bounds: Rect,
    z_index: i32,
}

impl WidgetTree {
    pub fn add_widget(&mut self, widget: Box<dyn Widget>, parent: Option<WidgetId>);
    pub fn remove_widget(&mut self, id: WidgetId);
    pub fn get_widget(&self, id: WidgetId) -> Option<&dyn Widget>;
    pub fn get_children(&self, id: WidgetId) -> &[WidgetId];
    pub fn get_parent(&self, id: WidgetId) -> Option<WidgetId>;
    
    pub fn mark_dirty(&mut self, id: WidgetId, flags: DirtyFlags);
    pub fn update_layout(&mut self);
    pub fn generate_render_data(&mut self) -> Vec<RenderData>;
    
    pub fn hit_test(&self, point: Point) -> Option<WidgetId>;
    pub fn find_widgets_in_rect(&self, rect: Rect) -> Vec<WidgetId>;
}
```

### 4.2 控件树操作

```rust
// 树遍历（深度优先）
pub fn traverse_depth_first(tree: &WidgetTree, callback: fn(WidgetId));

// 树遍历（广度优先）
pub fn traverse_breadth_first(tree: &WidgetTree, callback: fn(WidgetId));

// 查找路径（从根到指定节点）
pub fn find_path(tree: &WidgetTree, target: WidgetId) -> Vec<WidgetId>;

// 查找祖先
pub fn find_ancestors(tree: &WidgetTree, id: WidgetId) -> Vec<WidgetId>;

// 查找后代
pub fn find_descendants(tree: &WidgetTree, id: WidgetId) -> Vec<WidgetId>;
```

---

## 5. 布局系统详细设计

### 5.1 布局引擎架构

```
┌─────────────────────────────────────────────────────────────────┐
│  LayoutEngine                                                    │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  Input: WidgetTree + Parent Layout                          ││
│  │                                                              ││
│  │  Layout Strategies:                                         ││
│  │    ├─ AbsoluteLayout (固定位置)                              ││
│  │    ├─ FlexLayout (弹性布局)                                  ││
│  │    ├─ GridLayout (网格布局)                                  ││
│  │    └─ StackLayout (堆叠布局)                                 ││
│  │                                                              ││
│  │  Layout Process:                                             ││
│  │    1. Measure Phase: 计算控件期望尺寸                        ││
│  │       └─ 递归遍历子控件                                       ││
│  │       ─ 紧凑内容尺寸 vs 显式尺寸                              ││
│  │                                                              ││
│  │    2. Layout Phase: 分配实际位置和尺寸                       ││
│  │       ─ 应用父控件约束                                        ││
│  │       ─ 计算子控件位置                                        ││
│  │       ─ 处理对齐和边距                                        ││
│  │                                                              ││
│  │  Output: Updated Layout for each Widget                     ││
│  │    ├─ x, y (绝对坐标)                                        ││
│  │    ├─ width, height (实际尺寸)                               ││
│  │    └─ dirty_layout = false                                  ││
│  └─────────────────────────────────────────────────────────────┘│
│                          ↓                                       │
│  Constraints System                                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  min_width, max_width                                        ││
│  │  min_height, max_height                                      ││
│  │  preferred_width, preferred_height                           ││
│  │  aspect_ratio (可选)                                         ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 Flex布局算法（类似CSS Flexbox）

```rust
// ui/src/layout_engine.rs

pub struct FlexLayoutEngine;

impl FlexLayoutEngine {
    pub fn layout(container: &Layout, children: &[WidgetId], tree: &mut WidgetTree) {
        let direction = container.flex_direction;
        let main_axis = Self::get_main_axis(direction);
        let cross_axis = Self::get_cross_axis(direction);
        
        // Phase 1: Measure all children
        let child_sizes: Vec<Size> = children.iter()
            .map(|id| Self::measure_child(*id, tree))
            .collect();
        
        // Phase 2: Calculate total main size
        let total_main_size = child_sizes.iter()
            .map(|s| Self::get_size_axis(s, main_axis))
            .sum::<f32>() + container.gap * (children.len() - 1) as f32;
        
        // Phase 3: Distribute available space (justify)
        let available_main = Self::get_size_axis(&container.size(), main_axis);
        let extra_space = available_main - total_main_size;
        
        let positions = Self::distribute_positions(
            children.len(),
            container.justify,
            child_sizes,
            main_axis,
            extra_space,
            container.gap,
        );
        
        // Phase 4: Align items on cross axis
        let cross_size = Self::get_size_axis(&container.size(), cross_axis);
        
        for (i, child_id) in children.iter().enumerate() {
            let child_layout = tree.get_widget_mut(*child_id).unwrap();
            
            // Set position
            let x = if main_axis == Axis::X {
                positions[i]
            } else {
                Self::align_cross_axis(
                    child_sizes[i],
                    cross_size,
                    container.align,
                    cross_axis,
                )
            };
            
            let y = if main_axis == Axis::Y {
                positions[i]
            } else {
                Self::align_cross_axis(
                    child_sizes[i],
                    cross_size,
                    container.align,
                    cross_axis,
                )
            };
            
            child_layout.set_layout(Layout::new(x, y, child_sizes[i].width, child_sizes[i].height));
        }
    }
    
    fn measure_child(id: WidgetId, tree: &mut WidgetTree) -> Size {
        let widget = tree.get_widget(id).unwrap();
        let layout = widget.layout();
        
        // 如果有显式尺寸，直接使用
        if layout.width > 0.0 && layout.height > 0.0 {
            return layout.size();
        }
        
        // 否则计算紧凑尺寸（基于内容）
        let children = tree.get_children(id).to_vec();
        if children.is_empty() {
            // 叶子节点：返回最小尺寸
            Size::new(10.0, 10.0) // 默认最小尺寸
        } else {
            // 容器节点：递归计算子控件尺寸
            Self::measure_container(id, &children, tree)
        }
    }
    
    fn measure_container(id: WidgetId, children: &[WidgetId], tree: &mut WidgetTree) -> Size {
        // 递归布局子控件
        let child_sizes: Vec<Size> = children.iter()
            .map(|child| Self::measure_child(*child, tree))
            .collect();
        
        // 根据布局方向计算总尺寸
        let widget = tree.get_widget(id).unwrap();
        match widget.layout().layout_type {
            LayoutType::Flex => {
                let flex = widget.layout().flex_layout;
                let main_size = child_sizes.iter()
                    .map(|s| Self::get_size_axis(s, Self::get_main_axis(flex.direction)))
                    .sum::<f32>();
                
                let cross_size = child_sizes.iter()
                    .map(|s| Self::get_size_axis(s, Self::get_cross_axis(flex.direction)))
                    .max()
                    .unwrap_or(0.0);
                
                Size::new(main_size, cross_size)
            }
            _ => Size::new(100.0, 100.0)
        }
    }
    
    fn distribute_positions(
        count: usize,
        justify: FlexJustify,
        sizes: Vec<Size>,
        axis: Axis,
        extra: f32,
        gap: f32,
    ) -> Vec<f32> {
        let mut positions = Vec::with_capacity(count);
        
        match justify {
            FlexJustify::Start => {
                let mut pos = 0.0;
                for (i, size) in sizes.iter().enumerate() {
                    positions.push(pos);
                    pos += Self::get_size_axis(size, axis) + gap;
                }
            }
            
            FlexJustify::Center => {
                let start_offset = extra / 2.0;
                let mut pos = start_offset;
                for size in &sizes {
                    positions.push(pos);
                    pos += Self::get_size_axis(size, axis) + gap;
                }
            }
            
            FlexJustify::End => {
                let mut pos = extra;
                for size in &sizes {
                    positions.push(pos);
                    pos += Self::get_size_axis(size, axis) + gap;
                }
            }
            
            FlexJustify::SpaceBetween => {
                let space_between = if count > 1 {
                    extra / (count - 1) as f32
                } else {
                    0.0
                };
                
                let mut pos = 0.0;
                for (i, size) in sizes.iter().enumerate() {
                    positions.push(pos);
                    pos += Self::get_size_axis(size, axis);
                    if i < count - 1 {
                        pos += space_between + gap;
                    }
                }
            }
            
            FlexJustify::SpaceAround => {
                let space = extra / count as f32;
                let mut pos = space / 2.0;
                for size in &sizes {
                    positions.push(pos);
                    pos += Self::get_size_axis(size, axis) + space + gap;
                }
            }
            
            FlexJustify::SpaceEvenly => {
                let space = extra / (count + 1) as f32;
                let mut pos = space;
                for size in &sizes {
                    positions.push(pos);
                    pos += Self::get_size_axis(size, axis) + space + gap;
                }
            }
        }
        
        positions
    }
}

enum Axis { X, Y }
```

### 5.3 Grid布局算法

```rust
pub struct GridLayoutEngine;

impl GridLayoutEngine {
    pub fn layout(container: &Layout, children: &[WidgetId], tree: &mut WidgetTree) {
        let grid = container.grid_layout;
        let cols = grid.columns as usize;
        let rows = grid.rows as usize;
        
        let cell_width = (container.width - (cols - 1) as f32 * grid.column_gap) / cols as f32;
        let cell_height = (container.height - (rows - 1) as f32 * grid.row_gap) / rows as f32;
        
        for (i, child_id) in children.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            
            let x = col as f32 * (cell_width + grid.column_gap);
            let y = row as f32 * (cell_height + grid.row_gap);
            
            let child_layout = tree.get_widget_mut(*child_id).unwrap();
            child_layout.set_layout(Layout::new(x, y, cell_width, cell_height));
        }
    }
}
```

### 5.4 布局约束系统

```rust
#[repr(C)]
pub struct LayoutConstraints {
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub preferred_width: f32,
    pub preferred_height: f32,
    pub aspect_ratio: Option<f32>,
}

impl LayoutConstraints {
    pub fn clamp_size(&self, size: Size) -> Size {
        let width = size.width.clamp(
            self.min_width.unwrap_or(0.0),
            self.max_width.unwrap_or(f32::MAX),
        );
        
        let height = size.height.clamp(
            self.min_height.unwrap_or(0.0),
            self.max_height.unwrap_or(f32::MAX),
        );
        
        // 应用 aspect_ratio
        if let Some(ratio) = self.aspect_ratio {
            let current_ratio = width / height;
            if current_ratio > ratio {
                Size::new(width, width / ratio)
            } else {
                Size::new(height * ratio, height)
            }
        } else {
            Size::new(width, height)
        }
    }
    
    pub fn unconstrained() -> Self {
        Self {
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            preferred_width: 0.0,
            preferred_height: 0.0,
            aspect_ratio: None,
        }
    }
    
    pub fn fixed(width: f32, height: f32) -> Self {
        Self {
            min_width: Some(width),
            max_width: Some(width),
            min_height: Some(height),
            max_height: Some(height),
            preferred_width: width,
            preferred_height: height,
            aspect_ratio: None,
        }
    }
}
```

### 5.5 布局性能优化

```rust
pub struct LayoutOptimizer {
    dirty_widgets: HashSet<WidgetId>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl LayoutOptimizer {
    pub fn mark_dirty(&mut self, id: WidgetId) {
        self.dirty_widgets.insert(id);
        
        // 标记所有祖先为 dirty
        let tree = self.widget_tree.lock();
        let ancestors = tree.find_ancestors(id);
        for ancestor in ancestors {
            self.dirty_widgets.insert(ancestor);
        }
    }
    
    pub fn update_layout(&mut self, tree: &mut WidgetTree) {
        let _trace = ScopedTrace::new("ui_layout_update");
        
        // 只更新 dirty 的控件
        for id in &self.dirty_widgets {
            Self::layout_widget(*id, tree);
        }
        
        self.dirty_widgets.clear();
        
        self.dfx.lock().get_perf_monitor().lock().record_counter(
            "ui_layout_updates",
            self.dirty_widgets.len() as f32
        );
    }
    
    fn layout_widget(id: WidgetId, tree: &mut WidgetTree) {
        let widget = tree.get_widget(id).unwrap();
        let layout_type = widget.layout().layout_type;
        
        let children = tree.get_children(id).to_vec();
        
        match layout_type {
            LayoutType::Flex => FlexLayoutEngine::layout(widget.layout(), &children, tree),
            LayoutType::Grid => GridLayoutEngine::layout(widget.layout(), &children, tree),
            LayoutType::Stack => StackLayoutEngine::layout(widget.layout(), &children, tree),
            LayoutType::Absolute => {} // 子控件自己决定位置
        }
    }
}
```

### 5.6 响应式布局支持

```rust
#[repr(C)]
pub struct ResponsiveLayout {
    breakpoints: Vec<Breakpoint>,
}

#[repr(C)]
pub struct Breakpoint {
    pub min_width: f32,
    pub layout_config: LayoutConfig,
}

#[repr(C)]
pub struct LayoutConfig {
    pub columns: u32,
    pub direction: FlexDirection,
    pub gap: f32,
}

impl ResponsiveLayout {
    pub fn get_layout_for_width(&self, width: f32) -> &LayoutConfig {
        self.breakpoints.iter()
            .filter(|bp| width >= bp.min_width)
            .last()
            .map(|bp| &bp.layout_config)
            .unwrap_or(&self.breakpoints[0].layout_config)
    }
}

// 预定义断点（类似CSS媒体查询）
pub fn default_breakpoints() -> Vec<Breakpoint> {
    vec![
        Breakpoint { min_width: 0.0, layout_config: LayoutConfig { columns: 1, direction: FlexDirection::Column, gap: 8.0 } },
        Breakpoint { min_width: 600.0, layout_config: LayoutConfig { columns: 2, direction: FlexDirection::Row, gap: 16.0 } },
        Breakpoint { min_width: 900.0, layout_config: LayoutConfig { columns: 3, direction: FlexDirection::Row, gap: 24.0 } },
        Breakpoint { min_width: 1200.0, layout_config: LayoutConfig { columns: 4, direction: FlexDirection::Row, gap: 32.0 } },
    ]
}
```

---

## 6. 事件系统

### 5.1 事件分发流程

```
用户触摸屏幕：
┌─────────────────────────────────────────────────────────────────┐
│  Platform Layer                                                  │
│  TouchInput → TouchBegin(x=100, y=200, pointer_id=0)           │
└─────────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│  Event Dispatcher                                                │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  1. Hit Test: find target widget at (100, 200)              ││
│  │     Result: Button(id=123)                                  ││
│  │                                                              ││
│  │  2. Build Event Path: root → panel → button                 ││
│  │     [WidgetId(1), WidgetId(2), WidgetId(123)]               ││
│  │                                                              ││
│  │  3. Dispatch Event:                                         ││
│  │     Phase 1: Capturing (root → panel → button)              ││
│  │       ├─ root.on_touch_begin(event)  → Ignored              ││
│  │       ├─ panel.on_touch_begin(event) → Ignored              ││
│  │       └─ button.on_touch_begin(event) → Handled             ││
│  │                                                              ││
│  │     Phase 2: Bubbling (button → panel → root)               ││
│  │       ├─ button.on_click(event)      → Stopped              ││
│  │       │  (停止冒泡，不再传递给 panel 和 root)                 ││
│  │       └─ [结束]                                              ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 事件分发器实现

```rust
// ui/src/event_dispatcher.rs

pub struct EventDispatcher {
    widget_tree: Arc<Mutex<WidgetTree>>,
    gesture_recognizer: Arc<Mutex<GestureRecognizer>>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl EventDispatcher {
    pub fn dispatch_event(&mut self, event: &mut Event) {
        let _trace = ScopedTrace::new("ui_event_dispatch");
        
        // 1. 查找目标控件
        let target = self.find_target(event);
        event.target = target;
        
        // 2. 构建事件路径
        let path = self.build_event_path(target);
        
        // 3. 捕获阶段（从根到目标）
        self.dispatch_capturing(&path, event);
        
        // 4. 冒泡阶段（从目标到根）
        if !event.immediate_stopped {
            self.dispatch_bubbling(&path, event);
        }
        
        // 5. 手势识别
        self.gesture_recognizer.lock().process_event(event);
        
        // DFX: 记录事件分发
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Debug,
            "EventDispatcher",
            format!("Event dispatched: type={}, target={}, result={}", 
                event.event_type, event.target.id, event.stopped)
        );
    }
    
    fn dispatch_capturing(&mut self, path: &[WidgetId], event: &mut Event) {
        for widget_id in path {
            if event.immediate_stopped { break; }
            
            let mut tree = self.widget_tree.lock();
            let widget = tree.get_widget(*widget_id).unwrap();
            
            let result = widget.on_event(event);
            match result {
                EventResult::ImmediateStop => {
                    event.immediate_stopped = true;
                }
                _ => {}
            }
        }
    }
    
    fn dispatch_bubbling(&mut self, path: &[WidgetId], event: &mut Event) {
        for widget_id in path.iter().rev() {
            if event.stopped || event.immediate_stopped { break; }
            
            let mut tree = self.widget_tree.lock();
            let widget = tree.get_widget(*widget_id).unwrap();
            
            let result = widget.on_event(event);
            match result {
                EventResult::Stopped => {
                    event.stopped = true;
                }
                EventResult::ImmediateStop => {
                    event.immediate_stopped = true;
                }
                _ => {}
            }
        }
    }
    
    fn build_event_path(&self, target: WidgetId) -> Vec<WidgetId> {
        self.widget_tree.lock().find_path(target)
    }
    
    fn find_target(&self, event: &Event) -> WidgetId {
        match event.data {
            EventData::Touch(touch) => {
                self.widget_tree.lock()
                    .hit_test(Point { x: touch.x, y: touch.y })
                    .unwrap_or_default()
            }
            _ => WidgetId::default()
        }
    }
}
```

---

## 6. 手势系统

### 6.1 手势识别流程

```
用户操作：点击按钮
┌─────────────────────────────────────────────────────────────────┐
│  Touch Sequence                                                  │
│  ├─ TouchBegin (100, 200, t=0ms)                                │
│  ├─ TouchEnd   (100, 200, t=150ms)                              │
│  └───────────────────────────────────────────────────────────── │
│                          ↓                                       │
│  Gesture Recognizer                                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  1. TapRecognizer:                                          ││
│  │     ├─ TouchBegin: state = Possible                         ││
│  │     ├─ 检查: 位置未移动 (100,200 → 100,200)                  ││
│  │     ├─ 检查: 时间 < 500ms (150ms)                            ││
│  │     ├─ TouchEnd: state = Ended                              ││
│  │     └─ 识别成功: TapGesture(x=100, y=200)                   ││
│  │                                                              ││
│  │  2. 发送事件:                                                ││
│  │     Event { type: Click, target: Button(123), ... }         ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 手势识别器实现

```rust
// ui/src/gesture_recognizer.rs

pub struct GestureRecognizer {
    recognizers: HashMap<WidgetId, Vec<Box<dyn GestureRecognizerTrait>>>,
    active_gestures: HashMap<u32, ActiveGesture>,
    dfx: Arc<Mutex<DfxSystem>>,
}

trait GestureRecognizerTrait {
    fn gesture_type(&self) -> GestureType;
    fn process_event(&mut self, event: &Event) -> GestureState;
    fn reset(&mut self);
}

struct TapRecognizer {
    state: GestureState,
    start_time: u64,
    start_pos: Point,
    tap_count: u32,
    last_tap_time: u64,
}

impl GestureRecognizerTrait for TapRecognizer {
    fn gesture_type(&self) -> GestureType { GestureType::Tap }
    
    fn process_event(&mut self, event: &Event) -> GestureState {
        match event.event_type {
            EventType::TouchBegin => {
                self.state = GestureState::Possible;
                self.start_time = event.timestamp;
                let touch = unsafe { event.data.touch };
                self.start_pos = Point { x: touch.x, y: touch.y };
                self.state
            }
            
            EventType::TouchEnd => {
                let touch = unsafe { event.data.touch };
                let elapsed = event.timestamp - self.start_time;
                
                // 判断是否为 Tap
                if elapsed < 500 && 
                   self.distance(self.start_pos, Point { x: touch.x, y: touch.y }) < 10.0 {
                    self.state = GestureState::Ended;
                    
                    // 检查是否为 Double Tap
                    if event.timestamp - self.last_tap_time < 300 {
                        self.tap_count += 1;
                    } else {
                        self.tap_count = 1;
                    }
                    self.last_tap_time = event.timestamp;
                } else {
                    self.state = GestureState::Failed;
                }
                self.state
            }
            
            EventType::TouchCancel => {
                self.state = GestureState::Cancelled;
                self.state
            }
            
            _ => self.state
        }
    }
    
    fn reset(&mut self) {
        self.state = GestureState::Possible;
        self.start_time = 0;
        self.start_pos = Point::default();
    }
}
```

---

## 7. 绘图系统

### 7.1 Canvas 绘图上下文

```rust
// ui/src/canvas.rs

pub struct Canvas {
    commands: Vec<DrawCommand>,
    clip_rect: Option<Rect>,
    transform: Transform,
    opacity: f32,
}

#[repr(C)]
pub enum DrawCommand {
    Rect {
        bounds: Rect,
        fill_color: Color,
        stroke_color: Option<Color>,
        stroke_width: f32,
        border_radius: f32,
    },
    
    Text {
        bounds: Rect,
        text: *const c_char,
        font_size: f32,
        font_color: Color,
        alignment: TextAlignment,
    },
    
    Image {
        bounds: Rect,
        texture_id: u64,
        uv: Rect,
    },
    
    Line {
        start: Point,
        end: Point,
        color: Color,
        width: f32,
    },
    
    Path {
        points: Vec<Point>,
        fill_color: Option<Color>,
        stroke_color: Color,
        stroke_width: f32,
    },
    
    Shadow {
        bounds: Rect,
        shadow: Shadow,
    },
    
    ClipRect {
        rect: Rect,
    },
    
    Transform {
        transform: Transform,
    },
    
    Opacity {
        opacity: f32,
    },
}

impl Canvas {
    pub fn draw_rect(&mut self, bounds: Rect, style: &Style);
    pub fn draw_text(&mut self, bounds: Rect, text: &str, style: &TextStyle);
    pub fn draw_image(&mut self, bounds: Rect, texture: TextureId);
    pub fn draw_line(&mut self, start: Point, end: Point, color: Color, width: f32);
    
    pub fn set_clip_rect(&mut self, rect: Rect);
    pub fn set_transform(&mut self, transform: Transform);
    pub fn set_opacity(&mut self, opacity: f32);
    
    pub fn get_commands(&self) -> &[DrawCommand];
    pub fn clear(&mut self);
}
```

---

## 8. 动效系统

### 8.1 动画引擎

```rust
// ui/src/animation.rs

pub struct AnimationEngine {
    animations: HashMap<AnimationId, Animation>,
    running_animations: Vec<AnimationId>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl AnimationEngine {
    pub fn create_animation(&mut self, target: WidgetId, property: AnimatedProperty,
                           from: f32, to: f32, duration: f32, curve: AnimationCurve) -> AnimationId;
    
    pub fn start_animation(&mut self, id: AnimationId);
    pub fn pause_animation(&mut self, id: AnimationId);
    pub fn cancel_animation(&mut self, id: AnimationId);
    
    pub fn update(&mut self, delta_time: f32) {
        let _trace = ScopedTrace::new("ui_animation_update");
        
        for id in &self.running_animations {
            let anim = self.animations.get_mut(id).unwrap();
            
            if anim.paused { continue; }
            
            // 计算当前值
            let progress = self.calculate_progress(anim, delta_time);
            let value = self.interpolate(anim.from_value, anim.to_value, progress, &anim.curve);
            
            // 应用到控件
            self.apply_animation_value(anim.target, anim.property, value);
            
            // 检查是否完成
            if progress >= 1.0 {
                if anim.repeat_count > 0 {
                    anim.repeat_count -= 1;
                    if anim.auto_reverse {
                        std::mem::swap(&mut anim.from_value, &mut anim.to_value);
                    }
                } else {
                    anim.running = false;
                    self.on_animation_complete(*id);
                }
            }
            
            // DFX: 记录动画更新
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Trace,
                "AnimationEngine",
                format!("Animation {} updated: progress={}, value={}", id.id, progress, value)
            );
        }
        
        // 移除已完成的动画
        self.running_animations.retain(|id| {
            self.animations.get(id).map(|a| a.running).unwrap_or(false)
        });
    }
    
    fn interpolate(from: f32, to: f32, progress: f32, curve: &AnimationCurve) -> f32 {
        let t = match curve {
            AnimationCurve::Linear => progress,
            AnimationCurve::EaseIn => progress * progress,
            AnimationCurve::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            AnimationCurve::EaseInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    1.0 - 2.0 * (1.0 - progress) * (1.0 - progress)
                }
            }
            AnimationCurve::Spring => self.spring_curve(progress),
            AnimationCurve::Bounce => self.bounce_curve(progress),
            AnimationCurve::Custom(func) => func(progress),
        };
        
        from + (to - from) * t
    }
}
```

---

## 9. RHI UI 对接层

### 9.1 RHI UI 抽象

```rust
// rhi/src/ui.rs

pub struct UIRenderTarget {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

pub struct UIDrawCommand {
    pub command_type: UIDrawCommandType,
    pub bounds: Rect,
    pub material: UIMaterial,
}

#[repr(C)]
pub enum UIDrawCommandType {
    Rect,
    Text,
    Image,
    Line,
    Path,
}

pub struct UIMaterial {
    pub color: Color,
    pub texture: Option<TextureHandle>,
    pub shader: ShaderHandle,
}

pub trait UIRendererTrait {
    fn create_render_target(&mut self, width: u32, height: u32) -> Result<UIRenderTarget, RhiError>;
    fn destroy_render_target(&mut self, target: UIRenderTarget);
    
    fn begin_frame(&mut self, target: &UIRenderTarget);
    fn end_frame(&mut self);
    
    fn draw_commands(&mut self, commands: &[UIDrawCommand]);
    
    fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> Result<TextureHandle, RhiError>;
    fn destroy_texture(&mut self, texture: TextureHandle);
}
```

---

## 10. Vulkan UI 渲染器

### 10.1 Vulkan UI Pipeline

```rust
// rhi-vulkan/src/ui_renderer.rs

use ash::vk;

pub struct VulkanUIRenderer {
    device: ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    uniform_buffer: vk::Buffer,
    
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    
    dfx: Arc<Mutex<DfxSystem>>,
}

impl UIRendererTrait for VulkanUIRenderer {
    fn begin_frame(&mut self, target: &UIRenderTarget) {
        let _trace = ScopedTrace::new("vulkan_ui_begin_frame");
        
        // 开始 UI 渲染 pass
        self.begin_render_pass(target);
    }
    
    fn end_frame(&mut self) {
        self.end_render_pass();
        
        // DFX: 性能统计
        self.dfx.lock().get_perf_monitor().lock().record_frame();
    }
    
    fn draw_commands(&mut self, commands: &[UIDrawCommand]) {
        let _trace = ScopedTrace::new("vulkan_ui_draw_commands");
        
        // 转换为 Vulkan 绘图命令
        let vertices = self.generate_vertices(commands);
        let indices = self.generate_indices(commands);
        
        // 更新缓冲区
        self.update_buffers(&vertices, &indices);
        
        // 绘制
        self.drawIndexed(indices.len() as u32);
    }
}
```

---

## 11. 平台对接层

### 11.1 平台输入事件

```rust
// platform/src/ui.rs

pub struct PlatformInputHandler {
    event_dispatcher: Arc<Mutex<EventDispatcher>>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl PlatformInputHandler {
    pub fn on_touch_event(&mut self, action: TouchAction, x: f32, y: f32, pointer_id: u32) {
        let event_type = match action {
            TouchAction::Down => EventType::TouchBegin,
            TouchAction::Move => EventType::TouchMove,
            TouchAction::Up => EventType::TouchEnd,
            TouchAction::Cancel => EventType::TouchCancel,
        };
        
        let event = Event {
            event_type,
            timestamp: self.get_timestamp(),
            target: WidgetId::default(),
            bubbles: true,
            cancelable: true,
            stopped: false,
            immediate_stopped: false,
            data: EventData::Touch(TouchData {
                x, y, pointer_id, pressure: 1.0
            }),
        };
        
        self.event_dispatcher.lock().dispatch_event(event);
        
        // DFX: 记录输入事件
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Debug,
            "PlatformInput",
            format!("Touch event: action={}, x={}, y={}", action, x, y)
        );
    }
    
    pub fn on_key_event(&mut self, action: KeyAction, keycode: u32, modifiers: u32) {
        let event_type = match action {
            KeyAction::Press => EventType::KeyDown,
            KeyAction::Release => EventType::KeyUp,
        };
        
        let event = Event {
            event_type,
            timestamp: self.get_timestamp(),
            target: WidgetId::default(),
            bubbles: true,
            cancelable: true,
            stopped: false,
            immediate_stopped: false,
            data: EventData::Key(KeyData {
                keycode, modifiers, unicode_char: 0
            }),
        };
        
        self.event_dispatcher.lock().dispatch_event(event);
    }
}
```

---

## 12. C# UI 层设计

### 12.1 C# 控件 API

```csharp
// scripts/UIControls.cs

#if NATIVEAOT
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

namespace Hezhou.UI
{
    public abstract class Widget
    {
        private WidgetId _id;
        private WidgetId _parentId;
        private List<WidgetId> _children = new();
        
        private Layout _layout;
        private Style _style;
        
        // Thunk 导出函数
        [UnmanagedCallersOnly(EntryPoint = "ui_widget_on_event", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static EventResult OnEventThunk(Event* event, nuint context)
        {
            var widget = (Widget)GCHandle.FromIntPtr((IntPtr)context).Target!;
            return widget.OnEvent(event);
        }
        
        // 事件处理（子类重写）
        protected virtual EventResult OnEvent(Event* event)
        {
            return EventResult.Ignored;
        }
        
        // 绘制（子类重写）
        protected abstract void Draw(Canvas canvas);
        
        // Thunk 导出绘制
        [UnmanagedCallersOnly(EntryPoint = "ui_widget_draw", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void DrawThunk(Canvas* canvas, nuint context)
        {
            var widget = (Widget)GCHandle.FromIntPtr((IntPtr)context).Target!;
            widget.Draw(*canvas);
        }
        
        // 动画
        public AnimationId StartAnimation(AnimatedProperty property, float from, float to, 
                                          float duration, AnimationCurve curve)
        {
            return NativeMethods.ui_animation_create(_id, property, from, to, duration, curve);
        }
        
        public void StopAnimation(AnimationId id)
        {
            NativeMethods.ui_animation_cancel(id);
        }
    }
    
    public class Button : Widget
    {
        private string _text;
        private Color _textColor;
        
        protected override EventResult OnEvent(Event* event)
        {
            if (event->EventType == EventType.Click)
            {
                OnClick?.Invoke();
                return EventResult.Stopped;
            }
            return EventResult.Handled;
        }
        
        protected override void Draw(Canvas canvas)
        {
            // 绘制按钮背景
            canvas.DrawRect(_layout.Bounds, _style);
            
            // 绘制文字
            canvas.DrawText(_layout.Bounds, _text, new TextStyle
            {
                FontSize = 16.0f,
                FontColor = _textColor,
                Alignment = TextAlignment.Center
            });
        }
        
        public event Action OnClick;
    }
    
    public class ScrollView : Widget
    {
        private float _scrollOffset;
        private float _contentHeight;
        
        protected override EventResult OnEvent(Event* event)
        {
            if (event->EventType == EventType.TouchMove)
            {
                var touch = event->Data.Touch;
                // 处理滚动
                _scrollOffset += touch.VelocityY;
                return EventResult.Handled;
            }
            return EventResult.Ignored;
        }
        
        protected override void Draw(Canvas canvas)
        {
            // 设置裁剪区域
            canvas.SetClipRect(_layout.Bounds);
            
            // 绘制内容（偏移）
            canvas.SetTransform(Transform.Translate(0, -_scrollOffset));
            DrawChildren(canvas);
            
            // 恢复裁剪
            canvas.ResetClip();
        }
    }
    
    // 动画 API
    public static class AnimationExtensions
    {
        public static AnimationId FadeIn(this Widget widget, float duration = 0.3f)
        {
            return widget.StartAnimation(AnimatedProperty.Opacity, 0.0f, 1.0f, duration, AnimationCurve.EaseInOut);
        }
        
        public static AnimationId FadeOut(this Widget widget, float duration = 0.3f)
        {
            return widget.StartAnimation(AnimatedProperty.Opacity, 1.0f, 0.0f, duration, AnimationCurve.EaseInOut);
        }
        
        public static AnimationId ScaleTo(this Widget widget, float scale, float duration = 0.2f)
        {
            return widget.StartAnimation(AnimatedProperty.Scale, 1.0f, scale, duration, AnimationCurve.Spring);
        }
    }
}

// NativeMethods FFI 绑定
internal static class NativeMethods
{
    [DllImport("hezhou_ui", CallingConvention = CallingConvention.Cdecl)]
    public static extern AnimationId ui_animation_create(WidgetId target, AnimatedProperty property,
        float from, float to, float duration, AnimationCurve curve);
    
    [DllImport("hezhou_ui", CallingConvention = CallingConvention.Cdecl)]
    public static extern void ui_animation_cancel(AnimationId id);
    
    [DllImport("hezhou_ui", CallingConvention = CallingConvention.Cdecl)]
    public static extern void ui_canvas_draw_rect(Canvas* canvas, Rect* bounds, Style* style);
    
    [DllImport("hezhou_ui", CallingConvention = CallingConvention.Cdecl)]
    public static extern void ui_canvas_draw_text(Canvas* canvas, Rect* bounds, string text, TextStyle* style);
}
#endif
```

---

## 13. DFX 集成

### 13.1 UI 性能监控

```rust
// ui/src/lib.rs

use hezhou_dfx::*;

pub struct UISystem {
    widget_tree: Arc<Mutex<WidgetTree>>,
    event_dispatcher: Arc<Mutex<EventDispatcher>>,
    gesture_recognizer: Arc<Mutex<GestureRecognizer>>,
    animation_engine: Arc<Mutex<AnimationEngine>>,
    render_engine: Arc<Mutex<dyn UIRendererTrait>>,
    
    dfx: Arc<Mutex<DfxSystem>>,
}

impl UISystem {
    pub fn update(&mut self, delta_time: f32) {
        let _trace = ScopedTrace::new("ui_system_update");
        
        // 更新动画
        let anim_trace = ScopedTrace::new("ui_animation_update");
        self.animation_engine.lock().update(delta_time);
        anim_trace.finish();
        
        // 更新布局
        let layout_trace = ScopedTrace::new("ui_layout_update");
        self.widget_tree.lock().update_layout();
        layout_trace.finish();
        
        // 生成渲染数据
        let render_trace = ScopedTrace::new("ui_render_generate");
        let render_data = self.widget_tree.lock().generate_render_data();
        render_trace.finish();
        
        // 绘制
        let draw_trace = ScopedTrace::new("ui_draw");
        self.render_engine.lock().draw_commands(&render_data);
        draw_trace.finish();
        
        // DFX: 记录性能数据
        let perf = self.dfx.lock().get_perf_monitor().lock();
        perf.record_counter("ui_widgets", self.widget_tree.lock().nodes.len() as f32);
        perf.record_counter("ui_animations", self.animation_engine.lock().running_animations.len() as f32);
        perf.record_counter("ui_draw_commands", render_data.len() as f32);
    }
}
```

### 13.2 UI 布局可视化（调试工具）

```rust
// ui/src/debug.rs

pub struct UIDebugVisualizer {
    enabled: bool,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl UIDebugVisualizer {
    pub fn visualize_layout(&self, canvas: &mut Canvas, tree: &WidgetTree) {
        if !self.enabled { return; }
        
        // 绘制控件边框（调试模式）
        for (id, node) in &tree.nodes {
            let bounds = node.widget.layout().bounds;
            
            // 绘制边框
            canvas.draw_rect(bounds, &Style {
                background_color: Color::transparent(),
                border_color: Color::red(),
                border_width: 1.0,
                border_radius: 0.0,
                opacity: 0.5,
                shadow: None,
            });
            
            // 绘制 ID 标签
            canvas.draw_text(bounds, format!("Widget {}", id.id), &TextStyle {
                font_size: 10.0,
                font_color: Color::black(),
                alignment: TextAlignment::TopLeft,
            });
        }
        
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Debug,
            "UIDebug",
            format!("Layout visualized: {} widgets", tree.nodes.len())
        );
    }
}
```

---

## 14. 实现路线图

### Phase 1: UI 核心（2周）

```
Week 1:
├── ui 模块创建
├── Widget trait + 基础控件
├── WidgetTree 实现
├── Canvas + DrawCommand
└── DFX 集成

Week 2:
├── Event 系统定义
├── EventDispatcher 实现
├── 事件冒泡机制
└── 基础事件处理测试
```

### Phase 2: 手势 + 布局（2周）

```
Week 3:
├── Gesture 系统定义
├── GestureRecognizer 实现
├── Tap/Pan/Pinch 识别器
└── 手势测试

Week 4:
├── LayoutEngine 实现
├── Flex/Grid/Stack 布局
├── 布局计算优化
└── 布局测试
```

### Phase 3: RHI + Vulkan（1周）

```
Week 5:
├── rhi/src/ui.rs 抽象层
├── VulkanUIRenderer 实现
├── UI Pipeline + Shader
└── UI 渲染测试
```

### Phase 4: 动效 + C# 层（2周）

```
Week 6:
├── AnimationEngine 实现
├── AnimationCurve 曲线
├── PropertyAnimation
└── 动效测试

Week 7:
├── C# UIControls.cs 实现
├── Thunk 导出函数
├── FFI 绑定
└── C# API 测试
```

### Phase 5: 平台对接 + 控件库（2周）

```
Week 8:
├── PlatformInputHandler 实现
├── HarmonyOS NAPI 对接
├── 平台事件测试

Week 9:
├── Button/Label/Panel 实现
├── ScrollView 实现
├── 完整 UI Demo
└── 性能优化
```

---

## 15. 性能目标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| UI 渲染帧率 | ≥ 60 FPS | 复杂界面保持流畅 |
| 事件分发延迟 | < 1ms | 事件快速响应 |
| 手势识别延迟 | < 50ms | 手势识别准确快速 |
| 动画帧率 | ≥ 60 FPS | 动画流畅无卡顿 |
| 控件树遍历 | < 5ms | 100个控件遍历耗时 |
| 布局计算 | < 10ms | 布局更新耗时 |
| 内存占用 | < 50MB | UI 系统总内存 |

---

*文档版本: 1.0 | 创建时间: 2026-05-16*