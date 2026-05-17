# UI系统架构设计文档

## 总览

UI系统采用分层架构，支持纯Rust实现和**Thunk + Mono JIT**脚本控制两种模式。

### Thunk + Mono JIT 方案

| 特性 | Thunk调用 | Mono JIT加载 |
|------|----------|-------------|
| **调用开销** | ~10-20ns（函数指针） | 加载时反射查找 |
| **热重载** | ✅ 支持（Mono JIT） | ✅ 支持（重新加载DLL） |
| **性能** | 高（接近NativeAOT） | 低（反射查找） |
| **运行时依赖** | 需CLR运行时 | 需Mono SDK |
| **适用场景** | 每帧高频调用 + 开发期热更新 |

**核心思想**：
1. Mono JIT加载C# DLL（支持热重载）
2. C#通过`[UnmanagedCallersOnly]`标记方法，CLR自动生成Thunk
3. C#将Thunk函数指针注册到Rust
4. Rust直接调用函数指针（高性能，无反射）

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

### 2. Thunk + Mono JIT模式（推荐）

**架构图**：

```
┌─────────────────────────────────────────────────────────────────────┐
│  C# UI脚本 (Mono JIT加载)                                            │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  UIScript.cs                                                 │   │
│  │    ├── Initialize() → 创建UI + 注册Thunk                     │   │
│  │    │     ├── AddButton("Click Me") → Rust FFI               │   │
│  │    │     ├── button.SetOnClick(OnClickCallback)             │   │
│  │    │     └── register_ui_callback(thunk_ptr)                │   │
│  │    │                                                          │   │
│  │    ├── [UnmanagedCallersOnly] OnClick(widgetId)             │   │
│  │    │     ├── DFX.Log("Button clicked!")                     │   │
│  │    │     └── SetText(widgetId, "Clicked!")                  │   │
│  │    │                                                          │   │
│  │    ├── [UnmanagedCallersOnly] Update(dt) → 每帧调用          │   │
│  │    └── DFX.Log() → Rust日志系统                              │   │
│  │                                                              │   │
│  │  CLR生成Thunk：                                               │   │
│  │    OnClickThunk → 0x7FF1234567                               │   │
│  │    UpdateThunk → 0x7FF1234589                                │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓ register_ui_callback(thunk)          │
├─────────────────────────────────────────────────────────────────────┤
│  Rust Mono执行器                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  MonoExecutor                                                │   │
│  │    ├── load("UIScript.dll") → Mono JIT加载                  │   │
│  │    ├── call("Initialize") → C#创建UI                        │   │
│  │    ├── reload() → 热重载新DLL                                │   │
│  │    │                                                          │   │
│  │  Thunk回调缓存：                                              │   │
│  │    UI_CALLBACKS: Mutex<HashMap<String, ThunkPtr>>           │   │
│  │      ├── "update" → 0x7FF1234589                            │   │
│  │      ├── "onclick" → 0x7FF1234567                           │   │
│  │                                                              │   │
│  │  每帧调用：                                                   │   │
│  │    update_callback(0.016) → thunk(0.016) → C# Update(dt)    │   │
│  │    调用开销：~10ns（函数指针直接调用）                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                              ↓ ui_widget_xxx FFI
┌─────────────────────────────────────────────────────────────────────┐
│  Rust UI核心层                                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  UISystem                                                    │   │
│  │    ├── WidgetTree → 管理UI层级                               │   │
│  │    ├── EventDispatcher → 分发事件                           │   │
│  │    ├── FontAtlas → 字体纹理                                  │   │
│  │    └── DfxSystem → 日志系统                                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

**调用流程**：

```
T0: 初始化阶段
    Rust: MonoExecutor.load("UIScript.dll")
         → Mono JIT加载DLL
         → 反射查找Initialize方法
         → mono_runtime_invoke(Initialize)
    C#: Initialize()
         → AddButton("Click Me")
         → Rust FFI: ui_widget_tree_add_button()
         → buttonId = 123
         → OnClickThunk = &OnClick
         → register_ui_callback("onclick", OnClickThunk)
         → Rust保存: CALLBACKS["onclick"] = 0x7FF1234567
         
T1: 每帧渲染
    Rust: update_callback(deltaTime)
         → thunk(deltaTime) ← 函数指针直接调用（~10ns）
         → CLR Thunk切换上下文
         → C#: Update(deltaTime)
              → DFX.Log("Frame update")
              → Rust FFI: dfx_log()
              
T2: Button点击
    Rust: input_handler.on_mouse_event()
         → hit_test命中Button
         → trigger_widget_callback(123, "onclick")
         → onclick_callback(123)
         → thunk(123) ← 函数指针调用
         → C#: OnClick(123)
              → DFX.Log("Button clicked!", LogLevel.Info)
              → ui_widget_set_text(123, "Clicked!")
              
T3: 热重载（按R键）
    Rust: recompile + reload
         → mcs编译新DLL（UIScript_{timestamp}.dll）
         → MonoExecutor.reload(new_dll)
         → call("Initialize") → 重新创建UI + 注册新Thunk
         → 新逻辑生效
```

**性能对比**：

| 调用方式 | 调用开销 | 热重载 | 适用场景 |
|---------|---------|--------|----------|
| **Thunk函数指针** | ~10ns | ✅ Mono JIT支持 | 每帧Update、高频回调 |
| Mono反射invoke | ~100μs | ✅ 支持 | Initialize等一次性调用 |
| NativeAOT DLL | ~20ns | ❌ 不支持 | 发布版 |

### C#简单API设计

**目标**：C#接口必须简单，一行代码创建UI，DFX日志一行调用。

```csharp
// UIScript.cs - 极简API
using System;
using System.Runtime.InteropServices;
using Hezhou;

public static class UIScript
{
    private static ulong _buttonId;
    private static ulong _labelId;
    private static DFX _dfx;
    
    // Mono JIT入口：Initialize方法名固定
    public static void Initialize()
    {
        // 1. 初始化DFX日志
        _dfx = DFX.Create();
        _dfx.SetLogLevel(LogLevel.Info);
        _dfx.Log("UI初始化开始", LogLevel.Info);
        
        // 2. 创建UI（一行一个控件）
        var root = UI.CreateRootPanel(800, 600);
        _labelId = UI.AddLabel(root, "Hello Mono UI!");
        _buttonId = UI.AddButton(root, "Click Me");
        
        // 3. 注册点击回调（Thunk方式）
        UI.SetOnClick(_buttonId, OnButtonClick);
        
        // 4. 注册Update回调（Thunk方式）
        UI.RegisterUpdateCallback(Update);
        
        _dfx.Log("UI初始化完成", LogLevel.Info);
    }
    
    // Thunk回调：按钮点击
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static void OnButtonClick(ulong widgetId)
    {
        _dfx.Log($"按钮{widgetId}被点击", LogLevel.Info);
        UI.SetText(widgetId, "Clicked!");
        UI.SetText(_labelId, "Button was clicked!");
    }
    
    // Thunk回调：每帧更新
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static void Update(float deltaTime)
    {
        // 每帧逻辑（可选）
        // _dfx.TraceBegin("update", "ui");
        // ...
        // _dfx.TraceEnd("update", "ui");
    }
    
    // 热重载重置
    public static void ResetAll()
    {
        _buttonId = 0;
        _labelId = 0;
        _dfx?.Dispose();
        Initialize();
    }
}
```

### DFX C#封装

```csharp
// DFX.cs - 日志系统封装
using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public enum LogLevel : byte
    {
        Trace = 0,
        Debug = 1,
        Info = 2,
        Warn = 3,
        Error = 4,
        Fatal = 5
    }
    
    public class DFX : IDisposable
    {
        private IntPtr _handle;
        
        public static DFX Create()
        {
            var handle = NativeMethods.dfx_create();
            return new DFX(handle);
        }
        
        private DFX(IntPtr handle) => _handle = handle;
        
        public void SetLogLevel(LogLevel level)
        {
            NativeMethods.dfx_set_log_level(_handle, (byte)level);
        }
        
        public void Log(string message, LogLevel level = LogLevel.Info)
        {
            NativeMethods.dfx_log(
                _handle,
                (byte)level,
                "UIScript",
                message,
                "UIScript.cs",
                0
            );
        }
        
        public void TraceBegin(string name, string category = "ui")
        {
            NativeMethods.dfx_trace_begin(_handle, name, category);
        }
        
        public void TraceEnd(string name, string category = "ui")
        {
            NativeMethods.dfx_trace_end(_handle, name, category);
        }
        
        public float GetFPS() => NativeMethods.dfx_get_fps(_handle);
        
        public void Dispose()
        {
            if (_handle != IntPtr.Zero)
            {
                NativeMethods.dfx_destroy(_handle);
                _handle = IntPtr.Zero;
            }
        }
        
        private static class NativeMethods
        {
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern IntPtr dfx_create();
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_destroy(IntPtr system);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_set_log_level(IntPtr system, byte level);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_log(
                IntPtr system, byte level, string module, 
                string message, string file, uint line);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_trace_begin(IntPtr system, string name, string category);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_trace_end(IntPtr system, string name, string category);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern float dfx_get_fps(IntPtr system);
        }
    }
}
```

### UI C#封装

```csharp
// UI.cs - UI系统封装
using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static class UI
    {
        private static IntPtr _widgetTree;
        private static IntPtr _eventDispatcher;
        
        // 初始化（由Rust MonoExecutor调用后设置handle）
        public static void SetHandles(IntPtr widgetTree, IntPtr eventDispatcher)
        {
            _widgetTree = widgetTree;
            _eventDispatcher = eventDispatcher;
        }
        
        // 创建根面板
        public static ulong CreateRootPanel(float width, float height)
        {
            return NativeMethods.ui_widget_tree_create_root_panel(
                _widgetTree, 0, 0, width, height);
        }
        
        // 添加按钮
        public static ulong AddButton(ulong parentId, string text, 
            float x = 0, float y = 0, float width = 200, float height = 40)
        {
            return NativeMethods.ui_widget_tree_add_button(
                _widgetTree, parentId, x, y, width, height, text);
        }
        
        // 添加标签
        public static ulong AddLabel(ulong parentId, string text,
            float x = 0, float y = 0, float width = 300, float height = 30)
        {
            return NativeMethods.ui_widget_tree_add_label(
                _widgetTree, parentId, x, y, width, height, text);
        }
        
        // 设置文本
        public static void SetText(ulong widgetId, string text)
        {
            NativeMethods.ui_widget_set_text(_widgetTree, widgetId, text);
        }
        
        // 设置背景色
        public static void SetBackgroundColor(ulong widgetId, 
            float r, float g, float b, float a = 1.0f)
        {
            NativeMethods.ui_widget_set_background_color(
                _widgetTree, widgetId, r, g, b, a);
        }
        
        // 注册点击回调（Thunk方式）
        public static unsafe void SetOnClick(ulong widgetId, 
            delegate* unmanaged[Cdecl]<ulong, void> callback)
        {
            NativeMethods.ui_button_set_on_click_thunk(
                _widgetTree, widgetId, callback);
        }
        
        // 注册Update回调（Thunk方式）
        public static unsafe void RegisterUpdateCallback(
            delegate* unmanaged[Cdecl]<float, void> callback)
        {
            NativeMethods.ui_register_update_callback(callback);
        }
        
        // 分发触摸事件
        public static void DispatchTouchBegin(float x, float y, 
            uint pointerId = 0, ulong timestamp = 0)
        {
            NativeMethods.ui_event_dispatcher_dispatch_touch_begin(
                _eventDispatcher, x, y, pointerId, timestamp);
        }
        
        private static class NativeMethods
        {
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong ui_widget_tree_create_root_panel(
                IntPtr handle, float x, float y, float w, float h);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong ui_widget_tree_add_button(
                IntPtr handle, ulong parent, float x, float y, 
                float w, float h, string text);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong ui_widget_tree_add_label(
                IntPtr handle, ulong parent, float x, float y,
                float w, float h, string text);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_widget_set_text(
                IntPtr handle, ulong widgetId, string text);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_widget_set_background_color(
                IntPtr handle, ulong widgetId, float r, float g, float b, float a);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern unsafe void ui_button_set_on_click_thunk(
                IntPtr handle, ulong widgetId, 
                delegate* unmanaged[Cdecl]<ulong, void> callback);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern unsafe void ui_register_update_callback(
                delegate* unmanaged[Cdecl]<float, void> callback);
            
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_event_dispatcher_dispatch_touch_begin(
                IntPtr handle, float x, float y, uint pointerId, ulong timestamp);
        }
    }
}
```

## Rust端Thunk注册实现

### Thunk回调管理器

```rust
// ui/src/thunk_manager.rs
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::LazyLock;

pub type UpdateCallback = extern "C" fn(f32);
pub type WidgetCallback = extern "C" fn(u64);
pub type InitCallback = extern "C" fn();

static UI_CALLBACKS: LazyLock<Mutex<UICallbacks>> = 
    LazyLock::new(|| Mutex::new(UICallbacks::new()));

pub struct UICallbacks {
    update: Option<UpdateCallback>,
    onclicks: HashMap<u64, WidgetCallback>,
}

impl UICallbacks {
    pub fn new() -> Self {
        Self {
            update: None,
            onclicks: HashMap::new(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_update_callback(callback: UpdateCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.update = Some(callback);
    dfx_log!(Info, "UI", "注册Update回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_onclick_callback(widget_id: u64, callback: WidgetCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.onclicks.insert(widget_id, callback);
    dfx_log!(Info, "UI", "注册OnClick回调: widget={} callback={:?}", widget_id, callback);
}

pub fn trigger_update_callback(delta_time: f32) {
    let callbacks = UI_CALLBACKS.lock();
    if let Some(cb) = callbacks.update {
        cb(delta_time);  // ~10ns 函数指针调用
    }
}

pub fn trigger_onclick_callback(widget_id: u64) {
    let callbacks = UI_CALLBACKS.lock();
    if let Some(cb) = callbacks.onclicks.get(&widget_id) {
        cb(widget_id);  // ~10ns 函数指针调用
    }
}
```

### Button集成Thunk回调

```rust
// ui/src/widgets/button.rs
impl Button {
    pub fn trigger_click(&mut self) {
        dfx_log!(Info, "UI", "Button {} 点击触发", self.id);
        
        // 优先调用Thunk回调（高性能）
        if let Some(cb) = UI_CALLBACKS.lock().onclicks.get(&self.id.id) {
            cb(self.id.id);
            return;
        }
        
        // 回退到Box<dyn FnMut>回调
        if let Some(ref mut callback) = self.on_click {
            callback();
        }
    }
}
```

### FFI接口扩展

```rust
// ui/src/ffi.rs (新增Thunk接口)

#[unsafe(no_mangle)]
pub extern "C" fn ui_button_set_on_click_thunk(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback: extern "C" fn(u64),
) {
    if handle.is_null() {
        return;
    }
    
    // 直接注册到全局Thunk管理器
    ui_register_onclick_callback(widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_update_callback(
    callback: extern "C" fn(f32),
) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.update = Some(callback);
}
```

## MonoExecutor集成

```rust
// scripting/src/mono_ui_executor.rs
use wrapped_mono::*;

pub struct MonoUIExecutor {
    domain: Domain,
    assembly: Option<Assembly>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl MonoUIExecutor {
    pub fn new(dfx: Arc<Mutex<DfxSystem>>) -> Self {
        let domain = jit::init("UIScriptDomain", None);
        Self {
            domain,
            assembly: None,
            dfx,
        }
    }
    
    pub fn load(&mut self, dll_path: &str) -> Result<(), String> {
        dfx_log!(Info, "Mono", "加载UI脚本: {}", dll_path);
        
        let assembly = self.domain.assembly_open(dll_path)
            .ok_or("无法加载程序集")?;
        
        self.assembly = Some(assembly);
        
        // 反射调用Initialize（一次性）
        self.call_initialize()?;
        
        Ok(())
    }
    
    fn call_initialize(&self) -> Result<(), String> {
        let asm = self.assembly.as_ref().ok_or("未加载")?;
        let image = asm.get_image();
        let class = Class::from_name(&image, "", "UIScript")
            .ok_or("找不到UIScript类")?;
        
        // 查找Initialize方法
        let method = find_method_by_name(&class, "Initialize")?;
        
        // 反射调用（~100μs，仅初始化时调用一次）
        method.invoke(None, vec![])?;
        
        dfx_log!(Info, "Mono", "Initialize调用成功");
        Ok(())
    }
    
    pub fn reload(&mut self, dll_path: &str) -> Result<(), String> {
        dfx_log!(Info, "Mono", "热重载UI脚本: {}", dll_path);
        
        self.assembly = None;
        self.load(dll_path)?;
        
        // 调用ResetAll重置静态变量
        self.call_reset_all()?;
        
        Ok(())
    }
    
    fn find_method_by_name(class: &Class, name: &str) -> Result<Method, String> {
        let mut iter = std::ptr::null_mut();
        loop {
            let method_ptr = mono_class_get_methods(class.get_ptr(), &mut iter);
            if method_ptr.is_null() {
                break;
            }
            let method_name = mono_method_get_name(method_ptr);
            if method_name == name {
                return Ok(Method::from_ptr(method_ptr));
            }
        }
        Err(format!("找不到方法: {}", name))
    }
}
```

## 渲染器集成Thunk回调

```rust
// rhi-vulkan/src/ui_vulkan_renderer.rs
impl UIVulkanRenderer {
    pub fn draw_frame(&mut self) -> Result<bool, String> {
        // 1. 处理resize
        if self.needs_resize {
            self.recreate_swapchain()?;
            self.update_ui_layout();
        }
        
        // 2. 调用C# Update回调（Thunk方式）
        let delta_time = self.last_frame_time.elapsed().as_secs_f32();
        trigger_update_callback(delta_time);  // ~10ns
        
        // 3. 获取渲染数据
        let render_data = self.widget_tree.lock().generate_render_data();
        
        // 4. Vulkan渲染
        self.render_vulkan(&render_data)?;
        
        // 5. Present
        self.present()?;
        
        Ok(true)
    }
    
    fn process_events(&mut self) {
        // GLFW事件处理
        for event in self.glfw_events.iter() {
            match event {
                MouseButtonPress(x, y) => {
                    let flipped_y = self.screen_height - y;
                    self.input_handler.on_mouse_event(x, flipped_y);
                    
                    // 如果命中Button，触发Thunk回调
                    if let Some(widget_id) = self.hit_test_result {
                        trigger_onclick_callback(widget_id);  // ~10ns
                    }
                }
                KeyPress(key) if key == Key::R => {
                    // 热重载
                    self.reload_ui_script();
                }
            }
        }
    }
    
    fn reload_ui_script(&mut self) {
        dfx_log!(Info, "HotReload", "按R键触发热重载");
        
        // 1. 重新编译C#脚本
        let new_dll = recompile_mono_dll("UIScript.cs");
        
        // 2. 加载新DLL
        self.mono_executor.reload(&new_dll);
        
        dfx_log!(Info, "HotReload", "热重载完成");
    }
}
```

## 热重载编译脚本

```powershell
# build_mono_ui.ps1
param(
    [string]$ScriptName = "UIScript",
    [string]$OutputDir = "scripts/bin/Mono"
)

$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$outputDll = "$OutputDir/${ScriptName}_${timestamp}.dll"

$mcsArgs = @(
    "-target:library",
    "-out:$outputDll",
    "-r:System.dll",
    "scripts/${ScriptName}.cs",
    "scripts/DFX.cs",
    "scripts/UI.cs"
)

& mcs $mcsArgs

Write-Host "[Success] $outputDll compiled"
Write-Output $outputDll
```

## 关键调用路径

### Thunk + Mono JIT模式完整流程

```
T0: 启动初始化
    Rust: mono_executor.load("UIScript.dll")
         → Mono JIT加载DLL
         → 反射查找Initialize（~100μs）
         → mono_runtime_invoke(Initialize)
    C#: Initialize()
         → DFX.Log("初始化开始") → Rust dfx_log()
         → UI.AddButton("Click Me")
              → Rust FFI: ui_widget_tree_add_button()
              → buttonId = 123
         → UI.SetOnClick(123, OnClick)
              → OnClickThunk = &OnClick (CLR生成)
              → Rust FFI: ui_register_onclick_callback(123, thunk_ptr)
              → Rust: UI_CALLBACKS.onclicks[123] = 0x7FF1234567
         → UI.RegisterUpdateCallback(Update)
              → Rust: UI_CALLBACKS.update = 0x7FF1234589

T1: 每帧渲染
    Rust: draw_frame()
         → trigger_update_callback(0.016) ← 函数指针
              → thunk(0.016) ← ~10ns
              → C#: Update(0.016)
                   → DFX.TraceBegin("update")
                   → DFX.TraceEnd("update")
         → generate_render_data()
         → render_vulkan()

T2: Button点击
    Rust: process_events()
         → GLFW MouseButton event
         → hit_test命中Button(123)
         → trigger_onclick_callback(123)
              → thunk(123) ← ~10ns
              → C#: OnClick(123)
                   → DFX.Log("Button clicked!")
                   → UI.SetText(123, "Clicked!")
         → draw_frame() → 渲染新文字

T3: 热重载（按R键）
    Rust: reload_ui_script()
         → recompile_mono_dll()
              → mcs编译: UIScript_{timestamp}.dll
         → mono_executor.reload(new_dll)
              → 加载新Assembly
              → call_reset_all()
         → C#: ResetAll()
              → 清除静态变量
              → Initialize() → 新逻辑生效
```

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
# 纯Rust模式（当前Demo）
cargo run --bin mono_ui_demo --features mono --release

# Thunk + Mono JIT模式（推荐）
cargo run --bin mono_ui_thunk_demo --features mono --release

# 验证功能
1. 第一排：Label "Hello Mono UI!"
2. 第二排：Button "Click Me"
3. 点击Button → DFX日志输出"Button clicked!" → 文字变为"Clicked!"
4. 窗口最大化 → 文字清晰不模糊
5. 按R键 → 热重载C#脚本 → 新逻辑生效
```

## 文件结构

```
engine/
├ ui/src/
│   ├── thunk_manager.rs      ← Thunk回调管理器（新增）
│   ├── ffi.rs                ← FFI接口（扩展Thunk接口）
│   ├── lib.rs                ← UI核心系统
│   ├── widgets/button.rs     ← Button集成Thunk
│   └── font_atlas.rs         ← 字体缓存
│
├ scripting/src/
│   ├── mono_ui_executor.rs   ← Mono UI执行器（新增）
│   ├── mono_executor.rs      ← Mono基础封装
│   └── lib.rs                ← 脚本系统
│
├ dfx/src/
│   ├── lib.rs                ← DFX系统 + FFI接口
│   ├── logger.rs             ← 日志记录
│   ├── trace.rs              ← 性能追踪
│   └── perf.rs               ← 性能监控
│
├ rhi-vulkan/src/
│   └ ui_vulkan_renderer.rs   ← Vulkan渲染器 + Thunk调用
│
├ scripts/
│   ├── UIScript.cs           ← C# UI脚本（极简API）
│   ├── DFX.cs                ← DFX C#封装
│   ├── UI.cs                 ← UI C#封装
│   └ build_mono_ui.ps1       ← Mono编译脚本
│   └ bin/Mono/
│       └ UIScript_{timestamp}.dll  ← 编译产物
│
├ examples/src/
│   ├── mono_ui_demo.rs       ← 纯Rust Demo
│   └ mono_ui_thunk_demo.rs   ← Thunk + Mono Demo（新增）
│
└ docs/
    └ ui_architecture.md      ← 本文档
    └ ui_click_callback.md    ← 点击回调技术文档
```

## 扩展方向

1. **更多控件**：Slider、CheckBox、TextField、ScrollView
2. **动画系统**：Transition、KeyframeAnimation（C# Thunk回调驱动）
3. **主题系统**：StyleTheme、DynamicStyle（C#配置）
4. **多窗口**：WindowManager、多Swapchain支持
5. **布局系统**：FlexBox、Grid（C# API简化配置）
6. **数据绑定**：MVVM模式、双向绑定（C# ViewModel）

## 实现路线图

```
Phase 1: Thunk + Mono JIT基础
├── ui/src/thunk_manager.rs     ← Thunk回调管理器
├── scripting/src/mono_ui_executor.rs ← Mono UI执行器
├── scripts/UIScript.cs         ← C#极简API
├── scripts/DFX.cs              ← DFX日志封装
├── scripts/UI.cs               ← UI系统封装
└── examples/mono_ui_thunk_demo.rs ← Demo程序
│
Phase 2: 热重载支持
├── build_mono_ui.ps1           ← 编译脚本（时间戳命名）
├── mono_executor.reload()      ← 热重载逻辑
├── UIScript.ResetAll()         ← C#重置静态变量
└── 按R键触发重载
│
Phase 3: 更多控件 + Thunk回调
├── Slider → OnValueChanged Thunk
├── TextField → OnTextChanged Thunk
├── CheckBox → OnCheckedChanged Thunk
└── C#事件系统统一
│
Phase 4: 性能优化
├── Thunk批量调用（减少CLR切换）
├── UI批量更新（减少FFI调用）
├── DFX异步日志
└── Trace性能分析集成
```

## 性能基准

| 场景 | Mono反射 | Thunk函数指针 | NativeAOT |
|------|----------|--------------|-----------|
| Initialize（一次性） | ~100μs | - | - |
| Update（每帧） | - | ~10ns | ~20ns |
| OnClick（事件） | - | ~10ns | ~20ns |
| 热重载 | ✅ ~500ms | ✅ ~500ms | ❌ 不支持 |
| 内存占用 | 较大（Mono运行时） | 较大（Mono运行时） | 最小 |
| 适用场景 | 开发期 | **开发期 + 高频调用** | 发布版 |

## C#代码示例

### 最简示例（5行创建UI）

```csharp
public static void Initialize()
{
    var root = UI.CreateRootPanel(800, 600);
    var btn = UI.AddButton(root, "Click Me");
    UI.SetOnClick(btn, OnClick);
}

[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
public static void OnClick(ulong id) => UI.SetText(id, "Clicked!");
```

### DFX日志示例

```csharp
// 一行日志
DFX.Log("Button clicked!");

// 性能追踪
DFX.TraceBegin("render");
// ... 渲染逻辑
DFX.TraceEnd("render");

// 获取FPS
var fps = DFX.GetFPS();
DFX.Log($"当前FPS: {fps}");
```

### 热重载示例

```csharp
// 修改代码后按R键重载
public static void Initialize()
{
    // 修改这里
    UI.AddButton(root, "New Button Text");  // ← 改这行
}

// 按R键 → Rust重新编译 → 新DLL加载 → Initialize重调用 → 新文字生效
```