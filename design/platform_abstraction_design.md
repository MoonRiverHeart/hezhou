# 平台抽象层设计

> 统一平台接口，支持 HarmonyOS 和 GLFW (Windows/Linux/macOS) 双后端

---

## 1. 设计目标

### 1.1 问题背景

原有 `harmony` 模块仅支持 HarmonyOS 平台：
- 事件类型 (`TouchEvent`, `KeyEvent`) 鸿蒙专用
- 窗口类型 `OH_NativeWindow` 鸿蒙专用
- 无法在 Windows/Linux 上开发测试

### 1.2 解决方案

创建统一的平台抽象层 `engine/platform`：
- **Platform trait**: 定义统一接口
- **统一事件类型**: 跨平台事件表示
- **WindowHandle**: 封装不同原生窗口
- **Feature flags**: 编译时选择后端

---

## 2. 模块架构

```
engine/platform/
├── Cargo.toml              ← feature flags: glfw / harmony
├── src/
│   ├── lib.rs              ← PlatformManager + FFI 导出
│   ├── traits.rs           ← Platform trait (统一接口)
│   ├── event.rs            ← 统一事件类型
│   ├── window.rs           ← WindowHandle (跨平台窗口句柄)
│   ├── glfw_backend.rs     ← GLFW 实现 (Windows/Linux/macOS)
│   └── harmony_backend.rs  ← HarmonyOS 实现
```

---

## 3. Platform Trait

```rust
// traits.rs

pub trait Platform {
    fn name(&self) -> &'static str;
    
    // 生命周期
    fn init(&mut self) -> Result<(), String>;
    fn shutdown(&mut self);
    
    // 窗口管理
    fn create_window(&mut self, title: &str, width: i32, height: i32) -> Result<WindowHandle, String>;
    fn destroy_window(&mut self, window: &WindowHandle);
    fn get_window_handle(&self) -> Option<WindowHandle>;
    
    // 窗口属性
    fn set_window_title(&mut self, window: &WindowHandle, title: &str);
    fn set_window_size(&mut self, window: &WindowHandle, width: i32, height: i32);
    fn get_window_size(&self, window: &WindowHandle) -> (i32, i32);
    
    // 事件系统
    fn poll_events(&mut self) -> Vec<PlatformEvent>;
    fn wait_events(&mut self) -> Vec<PlatformEvent>;
    fn register_event_callback(&mut self, callback: EventCallback);
    
    // 时间与睡眠
    fn get_time(&self) -> f64;
    fn sleep(&self, seconds: f64);
    
    // 运行状态
    fn is_running(&self) -> bool;
    fn request_quit(&mut self);
    
    // 原生显示
    fn get_native_display(&self) -> Option<usize>;
}
```

---

## 4. 统一事件类型

```rust
// event.rs

#[repr(C)]
pub struct PlatformEvent {
    pub kind: PlatformEventKind,
    pub timestamp: u64,
}

#[repr(C)]
pub enum PlatformEventKind {
    Touch,          // 触摸/点击
    Key,            // 键盘
    Mouse,          // 鼠标
    WindowResize,   // 窗口大小变化
    WindowClose,    // 窗口关闭
    Lifecycle,      // 生命周期
}

// 触摸事件 (移动端)
#[repr(C)]
pub struct TouchEvent {
    pub action: TouchAction,    // Begin/Move/End/Cancel
    pub x: f32,
    pub y: f32,
    pub pointer_id: i32,
}

// 键盘事件
#[repr(C)]
pub struct KeyEvent {
    pub action: KeyAction,      // Press/Release/Repeat
    pub keycode: KeyCode,       // A-Z, 0-9, 方向键等
    pub modifiers: KeyModifiers,
}

// 鼠标事件 (PC端)
#[repr(C)]
pub struct MouseEvent {
    pub action: MouseAction,    // Press/Release/Move/Scroll
    pub button: MouseButton,    // Left/Right/Middle
    pub x: f32,
    pub y: f32,
    pub dx: f32,                // 相对移动
    pub dy: f32,
}

// 窗口事件
#[repr(C)]
pub struct WindowEvent {
    pub width: i32,
    pub height: i32,
}

// 生命周期事件 (移动端)
#[repr(C)]
pub struct LifecycleEvent {
    pub state: LifecycleState,  // Create/Start/Resume/Pause/Stop/Destroy
}
```

---

## 5. WindowHandle

```rust
// window.rs

#[repr(C)]
pub enum NativeWindowType {
    Unknown = 0,
    GLFW = 1,               // GLFW 窗口
    HarmonyOHNativeWindow = 2,  // 鸿蒙原生窗口
    Win32 = 3,              // Windows HWND
    X11 = 4,                // Linux X11
    Wayland = 5,            // Linux Wayland
}

#[repr(C)]
pub struct WindowHandle {
    pub window_type: NativeWindowType,
    pub ptr: usize,         // 原生窗口指针
    pub width: i32,
    pub height: i32,
}

impl WindowHandle {
    pub fn new(window_type: NativeWindowType, ptr: usize, width: i32, height: i32) -> Self;
    pub fn null() -> Self;
    pub fn is_valid(&self) -> bool;
    pub fn get_native_ptr(&self) -> *mut c_void;
    pub fn get_size(&self) -> (i32, i32);
}
```

---

## 6. GLFW 后端实现

### 6.1 结构

```rust
// glfw_backend.rs

pub struct GLFWPlatform {
    glfw: Option<glfw::Glfw>,
    window: Option<glfw::Window>,
    event_receiver: Option<Receiver<(f64, glfw::WindowEvent)>>,
    event_callbacks: Arc<Mutex<Vec<EventCallback>>>,
    running: bool,
    last_mouse_x: f64,
    last_mouse_y: f64,
}
```

### 6.2 实现要点

- 使用 `glfw::init::<()>(None)` 初始化
- `window.make_current()` 需要 `use glfw::Context`
- 事件通过 `glfw::flush_messages(&receiver)` 获取
- GLFW 事件转换为统一 `PlatformEvent`
- 需链接 `shell32.lib` (Windows)

### 6.3 KeyCode 映射

```rust
fn convert_glfw_key(key: glfw::Key) -> KeyCode {
    match key {
        glfw::Key::A => KeyCode::A,
        glfw::Key::Space => KeyCode::Space,
        glfw::Key::Enter => KeyCode::Enter,
        glfw::Key::Escape => KeyCode::Escape,
        // ... 完整映射表
        _ => KeyCode::Unknown,
    }
}
```

---

## 7. HarmonyOS 后端实现

### 7.1 结构

```rust
// harmony_backend.rs

pub struct HarmonyPlatform {
    window_ctx: Arc<Mutex<NativeWindowContext>>,
    event_bus: Arc<Mutex<EventBus>>,
    running: bool,
    event_callbacks: Arc<Mutex<Vec<EventCallback>>>,
}

pub struct NativeWindowContext {
    window: *mut OH_NativeWindow,
    width: i32,
    height: i32,
}
```

### 7.2 外部回调

```rust
// 由 ArkTS/XComponent 调用
#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_on_surface_created(
    platform: *mut HarmonyPlatform,
    window: *mut OH_NativeWindow,
    width: i32,
    height: i32,
);

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_on_touch_event(
    platform: *mut HarmonyPlatform,
    action: i32,    // 0=Begin, 1=Move, 2=End
    x: f32,
    y: f32,
    pointer_id: i32,
    timestamp: u64,
);
```

---

## 8. PlatformManager

```rust
// lib.rs

pub enum PlatformBackend {
    #[cfg(feature = "glfw")]
    GLFW(GLFWPlatform),
    #[cfg(feature = "harmony")]
    Harmony(HarmonyPlatform),
}

pub struct PlatformManager {
    backend: Option<PlatformBackend>,
    event_queue: Arc<Mutex<Vec<PlatformEvent>>>,
}

impl PlatformManager {
    pub fn new() -> Self;
    
    #[cfg(feature = "glfw")]
    pub fn create_glfw_platform(&mut self) -> Result<(), String>;
    
    #[cfg(feature = "harmony")]
    pub fn create_harmony_platform(&mut self) -> Result<(), String>;
    
    pub fn poll_events(&mut self) -> Vec<PlatformEvent>;
}
```

---

## 9. FFI 导出

```rust
// lib.rs

#[unsafe(no_mangle)]
pub extern "C" fn platform_manager_create() -> *mut PlatformManager;

#[unsafe(no_mangle)]
pub extern "C" fn platform_manager_destroy(manager: *mut PlatformManager);

#[cfg(feature = "glfw")]
#[unsafe(no_mangle)]
pub extern "C" fn platform_init_glfw(manager: *mut PlatformManager) -> i32;

#[cfg(feature = "harmony")]
#[unsafe(no_mangle)]
pub extern "C" fn platform_init_harmony(manager: *mut PlatformManager) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn platform_create_window(
    manager: *mut PlatformManager,
    title: *const c_char,
    width: i32,
    height: i32,
) -> WindowHandle;

#[unsafe(no_mangle)]
pub extern "C" fn platform_poll_events(manager: *mut PlatformManager) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn platform_is_running(manager: *mut PlatformManager) -> bool;

#[unsafe(no_mangle)]
pub extern "C" fn platform_get_time(manager: *mut PlatformManager) -> f64;
```

---

## 10. Cargo.toml 配置

```toml
# engine/platform/Cargo.toml

[package]
name = "hezhou-platform"
version = "0.1.0"
edition = "2021"

[lib]
name = "hezhou_platform"
path = "src/lib.rs"

[dependencies]
hezhou-scripting = { path = "../scripting" }
parking_lot = "0.12"

[target.'cfg(not(target_os = "harmony"))'.dependencies]
glfw = { version = "0.9", optional = true }

[features]
default = ["glfw"]
glfw = ["dep:glfw"]
harmony = []
```

---

## 11. 使用方式

### 11.1 GLFW (Windows/Linux/macOS)

```bash
cd engine
cargo run -p hezhou-examples --bin glfw_demo
```

### 11.2 HarmonyOS 编译

```bash
cargo build -p hezhou-platform --no-default-features --features harmony
```

### 11.3 代码示例

```rust
// 创建平台管理器
let manager = platform_manager_create();

// 初始化 GLFW 后端
platform_init_glfw(manager);

// 创建窗口
let title = CString::new("Hezhou Engine");
let window = platform_create_window(manager, title.as_ptr(), 800, 600);

// 主循环
while platform_is_running(manager) {
    platform_poll_events(manager);
    let time = platform_get_time(manager);
    // 渲染逻辑...
}

// 清理
platform_manager_destroy(manager);
```

---

## 12. Demo 输出

```
=== Hezhou Engine - GLFW Platform Demo ===

[1] 初始化 GLFW 平台...
    GLFW 初始化成功!

[2] 创建窗口...
    Window: type=1, ptr=1435854068640, size=800x600

[3] 初始化引擎...
    Engine started

[4] 初始化脚本系统...
    Script callback registered

[5] 主循环...
    Frame 1: time=0.276s, events=1
    Frame 30: time=0.757s, events=0
    Frame 60: time=1.251s, events=0
    Frame 90: time=1.744s, events=0

[6] 清理...
    清理完成

=== Demo Complete ===
```

---

## 13. 设计优势

| 特性 | 说明 |
|------|------|
| **统一 API** | 同一套接口操作不同平台 |
| **零开销** | Feature flags 编译时选择，无运行时分支 |
| **类型安全** | WindowHandle 封装原生窗口，避免裸指针 |
| **事件统一** | TouchEvent/KeyEvent/MouseEvent 跨平台表示 |
| **FFI 导出** | C#/ArkTS 可直接调用 |
| **开发便捷** | GLFW 后端支持本地开发测试 |

---

## 14. 与现有模块集成

```
┌─────────────────────────────────────────────────────────────┐
│  C# Script Layer                                            │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  Engine.Render.DrawMesh()                              │ │
│  │  Engine.Physics.Raycast()                              │ │
│  │  Platform.GetWindowHandle() ← 调用平台抽象层           │ │
│  └───────────────────────────────────────────────────────┘ │
│                          ↓ FFI                              │
├─────────────────────────────────────────────────────────────┤
│  Rust Engine Core                                           │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  platform/ ← 平台抽象层                                │ │
│  │  ├── GLFWPlatform                                      │ │
│  │  ├── HarmonyPlatform                                   │ │
│  │  └───────────────────────────────────────────────── │ │
│  │  core/ ← ECS/EventBus                                  │ │
│  │  render/ ← 渲染                                        │ │
│  └───────────────────────────────────────────────────────┘ │
│                          ↓ Native Window                    │
├─────────────────────────────────────────────────────────────┤
│  Platform Layer                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  GLFW (Windows/Linux/macOS)                            │ │
│  │  OH_NativeWindow (HarmonyOS)                           │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## 15. 下一步扩展

### 15.1 原生窗口支持

- **Win32**: 直接使用 HWND (Windows)
- **X11/Wayland**: Linux 原生支持

### 15.2 输入设备扩展

- 游戏手柄 (Gamepad)
- 触控板 (Touchpad)
- VR 控制器

### 15.3 多窗口支持

```rust
fn create_window(&mut self, id: u32, title: &str, width: i32, height: i32);
fn get_window(&self, id: u32) -> Option<WindowHandle>;
fn destroy_window(&mut self, id: u32);
```

---

*文档版本: 1.0 | 创建时间: 2026-05-12*