# UI系统架构设计文档

## 总览

UI系统采用分层架构，支持纯Rust实现和C#脚本控制两种模式。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                     应用层 (Application Layer)                       │
├─────────────────────────────────────────────────────────────────────┤
│ mono_ui_demo.rs                                                     │
│   ├── UIVulkanRenderer (Vulkan渲染器)                               │
│   │     ├── setup_ui() → Rust创建UI控件                            │
│   │     ├── process_events() → GLFW事件                            │
│   │     ├── draw_frame() → 渲染                                    │
│   │     └── get_button_id() → 返回Button ID                        │
│   │                                                                 │
│   └── MonoUIExecutor (Mono执行器)                                   │
│         ├── new(dll_path) → 加载C# DLL                             │
│         └── call_static_void() → 调用C#静态方法                     │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     脚本层 (Script Layer)                           │
├─────────────────────────────────────────────────────────────────────┤
│ MonoUIScript.cs (C#脚本)                                            │
│   ├── Initialize() → 创建UISystem                                   │
│   │     ├── UISystem.CreateRootPanel(800,600)                      │
│   │     ├── UISystem.AddButton("Click Me")                         │
│   │     └── UISystem.AddLabel("Hello Mono UI!")                    │
│   │                                                                 │
│   ├── Update(deltaTime) → 每帧更新                                  │
│   ├── OnTouchBegin(x,y) → 处理触摸                                  │
│   ├── OnKeyDown(keycode) → 处理按键                                 │
│   └── Cleanup() → 清理                                              │
│                                                                     │
│ UISystem.cs (C#封装类)                                              │
│   ├── CreateRootPanel() → FFI调用 ui_widget_tree_create_root_panel │
│   ├── AddButton() → FFI调用 ui_widget_tree_add_button              │
│   ├── AddLabel() → FFI调用 ui_widget_tree_add_label                │
│   ├── DispatchTouchBegin() → FFI调用 ui_event_dispatcher_dispatch  │
│   └── SetWidgetBackgroundColor() → FFI调用 ui_widget_set_background│
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ DllImport (P/Invoke)
┌─────────────────────────────────────────────────────────────────────┐
│                     FFI层 (Foreign Function Interface)              │
├─────────────────────────────────────────────────────────────────────┤
│ ui/src/ffi.rs (Rust暴露给C#的接口)                                  │
│   ├── ui_system_create() → 创建UISystem                            │
│   ├── ui_widget_tree_create_root_panel(x,y,w,h) → 创建根面板       │
│   ├── ui_widget_tree_add_button(parent,text) → 创建Button          │
│   ├── ui_widget_tree_add_label(parent,text) → 创建Label            │
│   ├── ui_event_dispatcher_dispatch_touch_begin(x,y) → 分发触摸     │
│   ├── ui_widget_set_background_color(r,g,b,a) → 设置颜色           │
│   └── ui_widget_set_text(widget_id,text) → 设置文字                │
│                                                                     │
│ scripting/src/ffi.rs (脚本FFI)                                      │
│   ├── scripting_init() → 初始化ScriptManager                       │
│   └── scripting_register_sync_callback() → 注册回调                │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     核心层 (Core Layer)                             │
├─────────────────────────────────────────────────────────────────────┤
│ ui/src/lib.rs                                                       │
│   ├── UISystem                                                      │
│   │     ├── widget_tree: Arc<Mutex<WidgetTree>>                    │
│   │     ├── event_dispatcher: Arc<Mutex<EventDispatcher>>          │
│   │     └── font_atlas: Arc<FontAtlas>                             │
│   │                                                                 │
│   ├── WidgetTree                                                    │
│   │     ├── set_root(Panel) → 设置根节点                           │
│   │     ├── add_widget(Button/Label) → 添加子节点                  │
│   │     ├── perform_layout() → 执行布局                            │
│   │     └── generate_render_data() → 生成渲染数据                  │
│   │                                                                 │
│   ├── EventDispatcher                                               │
│   │     ├── dispatch_event(Event) → 分发事件                       │
│   │     ├── hit_test(Point) → 坐标命中测试                         │
│   │     └── GestureRecognizer → 手势识别(Tap/DoubleTap)           │
│   │                                                                 │
│   ├── Widgets                                                       │
│   │     ├── Button                                                  │
│   │     │     ├── set_text("hello") → 设置文字                     │
│   │     │     ├── set_on_click(callback) → 设置回调                │
│   │     │     └── trigger_click() → 触发点击                       │
│   │     ├── Label                                                   │
│   │     ├── Panel                                                   │
│   │     ├── VStack/HStack → 布局容器                               │
│   │                                                                 │
│   └── Canvas                                                        │
│         ├── draw_rect() → 绘制矩形                                  │
│         ├── draw_text() → 绘制文字                                  │
│         └── DrawCommand → 渲染命令列表                             │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ RenderData
┌─────────────────────────────────────────────────────────────────────┐
│                     渲染层 (Render Layer)                           │
├─────────────────────────────────────────────────────────────────────┤
│ rhi-vulkan/src/ui_vulkan_renderer.rs                               │
│   ├── draw_frame()                                                  │
│   │     ├── generate_render_data() → 获取DrawCommand              │
│   │     ├── 生成vertices (Rect/Text)                               │
│   │     ├── 字体纹理 → font_atlas.get_atlas_texture()             │
│   │     ├── Vulkan渲染 → pipeline + command_buffer                │
│   │     └── Swapchain → 窗口显示                                   │
│   │                                                                 │
│   ├── process_events()                                              │
│   │     ├── GLFW MouseButton → MouseEvent                         │
│   │     ├── Y坐标翻转 → screen_height - y                         │
│   │     └── input_handler.on_mouse_event()                        │
│   │                                                                 │
│   └── resize支持                                                    │
│         ├── recreate_swapchain() → 重建Swapchain                   │
│         └── update_ui_layout() → 更新UI布局                        │
└─────────────────────────────────────────────────────────────────────┘
```

## 两种运行模式

### 1. 纯Rust模式（当前Demo）

```
┌─────────────────────────────────────────────────────────┐
│ mono_ui_demo.rs (Rust主程序)                            │
│   - 初始化 UIVulkanRenderer                             │
│   - setup_ui() 创建 Button/Label                        │
│   - 主循环检测空格键                                     │
│   - 直接修改 Button.set_text("hello")                   │
└─────────────────────────────────────────────────────────┘
```

**调用链**：
```
空格键按下 → renderer.is_space_pressed()
         → tree_guard.get_widget_mut(button_id)
         → button.set_text("hello")
         → draw_frame() → 文字变为"hello"
```

### 2. C#脚本模式

C#通过FFI层调用Rust核心功能：

```csharp
// C#脚本
UISystem uiSystem = new UISystem();
ulong rootId = uiSystem.CreateRootPanel(0, 0, 800, 600);
ulong buttonId = uiSystem.AddButton(rootId, 50, 50, 200, 40, "Click Me");

// 触摸事件
uiSystem.DispatchTouchBegin(x, y, pointerId, timestamp);
```

FFI调用：
```
C# AddButton → DllImport → ui_widget_tree_add_button (Rust FFI)
             → WidgetTree.add_widget(Box::new(Button))
             → 返回widget_id
```

## 关键调用路径

### Button点击 → 文字变化（纯Rust）

```
用户点击屏幕(420,324)
  ↓ GLFW MouseButton event
process_events()
  ↓
input_handler.on_mouse_event(y = 600-324 = 276) [Y翻转]
  ↓
event_dispatcher.dispatch_event(TouchBegin)
  ↓
hit_test(Point(420,276)) → Button命中
  ↓
GestureRecognizer → Tap手势识别
  ↓
button.trigger_click() → 回调执行
  ↓ button_clicked.store(true)
主循环检测 button_clicked
  ↓
tree_guard.get_widget_mut(button_id)
  ↓
button.set_text("hello")
  ↓
draw_frame() → 渲染"hello"
```

### C#调用流程

```
C#: UISystem.AddButton(parent, "Click Me")
  ↓ DllImport
Rust FFI: ui_widget_tree_add_button(handle, parent_id, text)
  ↓
WidgetTree.add_widget(Box::new(Button), parent)
  ↓
Button创建 → 返回widget_id
```

```
C#: UISystem.DispatchTouchBegin(x, y)
  ↓ DllImport
Rust FFI: ui_event_dispatcher_dispatch_touch_begin(handle, x, y)
  ↓
EventDispatcher.dispatch_event(TouchBegin)
  ↓
hit_test → Button命中 → Tap识别 → trigger_click()
```

## 数据流向

```
┌──────────┐  Create   ┌──────────┐  Add      ┌──────────┐
│ C#脚本   │ ────────► │ UISystem │ ────────► │WidgetTree│
│          │           │          │           │  (Rust)  │
└──────────┘           └──────────┘           └──────────┘
                                                 │
                                                 │ perform_layout()
                                                 ▼
                                           ┌──────────┐
                                           │ Canvas   │
                                           │DrawCommand│
                                           └──────────┘
                                                 │
                                                 │ generate_render_data()
                                                 ▼
                                           ┌──────────┐
                                           │Vulkan    │
                                           │Renderer  │
                                           └──────────┘
                                                 │
                                                 │ vertices → GPU
                                                 ▼
                                           ┌──────────┐
                                           │  Screen  │
                                           └──────────┘
```

## 坐标系统

### Vulkan Y轴翻转

Shader统一处理Y翻转，vertex生成不翻转：

```glsl
// ui.vert
gl_Position = vec4(clip_pos.x, -clip_pos.y, 0.0, 1.0);
```

### Hit Test坐标翻转

点击坐标翻转Y以匹配逻辑bounds：

```rust
// input_handler.rs
pub fn on_mouse_event(&mut self, mouse: &MouseEvent, timestamp: u64) {
    let y = self.screen_height - mouse.y;  // 翻转Y
}
```

### 坐标转换流程

```
用户点击(420, 324) → screen_height=600
                  → flipped_y = 600-324 = 276
                  → hit_test(Point(420, 276))
                  → Button bounds(y=257)命中
                  → trigger_click()
```

## 窗口Resize处理

### Swapchain重建流程

```
WindowEvent::Size → needs_resize=true
                  → new_extent={width,height}

draw_frame() → 检测needs_resize
             → recreate_swapchain()
                 ├── 销毁framebuffers/image_views
                 ├── 用old_swapchain重建swapchain
                 ├── 创建新framebuffers/image_views
                 └── 更新extent
             → update_ui_layout()
                 ├── root_panel.set_layout(new_size)
                 ├── recenter_widget(vstack_id)
                 └── perform_layout()
             → 清除needs_resize
             → 继续渲染
```

## 核心模块说明

### UISystem

```rust
pub struct UISystem {
    widget_tree: Arc<Mutex<WidgetTree>>,
    event_dispatcher: Arc<Mutex<EventDispatcher>>,
    font_atlas: Arc<FontAtlas>,
}
```

职责：
- 持有WidgetTree和EventDispatcher
- 提供FFI接口给C#调用
- 管理字体纹理缓存

### WidgetTree

```rust
pub struct WidgetTree {
    root: Option<WidgetId>,
    nodes: HashMap<WidgetId, WidgetNode>,
    parent_map: HashMap<WidgetId, WidgetId>,
    children_map: HashMap<WidgetId, Vec<WidgetId>>,
}
```

职责：
- 管理Widget层级结构
- 执行布局算法（VStack/HStack）
- 生成渲染数据

### EventDispatcher

```rust
pub struct EventDispatcher {
    widget_tree: Arc<Mutex<WidgetTree>>,
    gesture_recognizer: Arc<Mutex<GestureRecognizer>>,
}
```

职责：
- 分发事件到目标Widget
- hit_test命中测试
- 手势识别（Tap/DoubleTap）

### Canvas

```rust
pub struct Canvas {
    commands: Vec<DrawCommand>,
    transform: Transform2D,
    opacity: f32,
}
```

职责：
- 收集绘制命令
- 支持变换和透明度
- 生成DrawCommand列表

## FFI接口列表

### ui/src/ffi.rs

| 函数名 | 功能 | 参数 |
|--------|------|------|
| `ui_system_create` | 创建UISystem | - |
| `ui_system_destroy` | 销毁UISystem | system指针 |
| `ui_system_update` | 更新UI | system, delta_time |
| `ui_widget_tree_create_root_panel` | 创建根面板 | handle, x,y,w,h |
| `ui_widget_tree_add_button` | 添加Button | handle, parent, x,y,w,h, text |
| `ui_widget_tree_add_label` | 添加Label | handle, parent, x,y,w,h, text |
| `ui_widget_set_layout` | 设置布局 | handle, widget_id, x,y,w,h |
| `ui_widget_set_background_color` | 设置背景色 | handle, widget_id, r,g,b,a |
| `ui_widget_set_text` | 设置文字 | handle, widget_id, text |
| `ui_event_dispatcher_dispatch_touch_begin` | 分发触摸开始 | handle, x,y,pointer_id,timestamp |
| `ui_event_dispatcher_dispatch_touch_end` | 分发触摸结束 | handle, x,y,pointer_id,timestamp |
| `ui_event_dispatcher_dispatch_key_down` | 分发按键按下 | handle, keycode,modifiers,timestamp |
| `ui_event_dispatcher_dispatch_key_up` | 分发按键释放 | handle, keycode,modifiers,timestamp |

## 渲染流程

### draw_frame()

```rust
pub fn draw_frame(&mut self) -> Result<bool, String> {
    // 1. 处理resize
    if needs_resize {
        recreate_swapchain();
        update_ui_layout();
    }
    
    // 2. 获取渲染数据
    let render_data = widget_tree.generate_render_data();
    
    // 3. 生成vertices
    for cmd in render_data.draw_commands {
        match cmd {
            DrawCommand::Rect { bounds, .. } => {
                // 生成6个顶点（2个三角形）
            }
            DrawCommand::Text { bounds, text, .. } => {
                // 每个glyph生成6个顶点
            }
        }
    }
    
    // 4. 上传vertex buffer
    device.map_memory(vertex_buffer_memory);
    copy_vertices_to_gpu();
    device.unmap_memory(vertex_buffer_memory);
    
    // 5. Vulkan渲染
    cmd_bind_pipeline(pipeline);
    cmd_bind_vertex_buffers(vertex_buffer);
    cmd_bind_descriptor_sets(descriptor_set); // 字体纹理
    cmd_draw(vertex_count);
    
    // 6. Present
    queue_present(swapchain);
}
```

## 测试命令

```bash
# 纯Rust模式
cargo run --bin mono_ui_demo --features mono --release

# 验证功能
1. 第一排：Label "Welcome to Hezhou UI!"
2. 第二排：Button "Click Me"
3. 点击Button → 文字变为"hello"
4. 窗口最大化 → 文字清晰不模糊
```

## 扩展方向

1. **C#脚本支持**：通过FFI完整实现UI控制
2. **更多控件**：Slider、CheckBox、TextField等
3. **动画系统**：Transition、KeyframeAnimation
4. **主题系统**：StyleTheme、DynamicStyle
5. **多窗口**：WindowManager、多Swapchain支持