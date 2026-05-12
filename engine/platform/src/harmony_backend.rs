use crate::event::*;
use crate::traits::*;
use crate::window::*;
use parking_lot::Mutex;
use std::sync::Arc;

#[repr(C)]
pub struct OH_NativeWindow {
    _private: [u8; 0],
}

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

pub struct EventBus {
    handlers: Vec<(String, EventCallbackWithContext)>,
}

pub type EventCallbackWithContext = extern "C" fn(PlatformEvent, usize);

impl NativeWindowContext {
    pub fn new() -> Self {
        Self {
            window: std::ptr::null_mut(),
            width: 0,
            height: 0,
        }
    }
    
    pub fn get_window(&self) -> *mut OH_NativeWindow {
        self.window
    }
    
    pub fn get_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

impl Default for NativeWindowContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    
    pub fn register_event_handler(&mut self, name: &str, callback: EventCallbackWithContext, context: usize) {
        self.handlers.push((name.to_string(), callback));
    }
    
    pub fn dispatch(&self, event: PlatformEvent) {
        for (_, handler) in &self.handlers {
            handler(event, 0);
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl HarmonyPlatform {
    pub fn new() -> Self {
        Self {
            window_ctx: Arc::new(Mutex::new(NativeWindowContext::new())),
            event_bus: Arc::new(Mutex::new(EventBus::new())),
            running: false,
            event_callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn on_surface_created(&mut self, window: *mut OH_NativeWindow, width: i32, height: i32) {
        let mut ctx = self.window_ctx.lock();
        ctx.window = window;
        ctx.width = width;
        ctx.height = height;
        self.running = true;
        
        let event = PlatformEvent {
            kind: PlatformEventKind::WindowResize,
            timestamp: 0,
        };
        
        for callback in self.event_callbacks.lock().iter() {
            callback(&event);
        }
    }
    
    pub fn on_surface_resize(&mut self, width: i32, height: i32) {
        let mut ctx = self.window_ctx.lock();
        ctx.width = width;
        ctx.height = height;
        
        let event = PlatformEvent {
            kind: PlatformEventKind::WindowResize,
            timestamp: 0,
        };
        
        for callback in self.event_callbacks.lock().iter() {
            callback(&event);
        }
    }
    
    pub fn on_surface_destroyed(&mut self) {
        let mut ctx = self.window_ctx.lock();
        ctx.window = std::ptr::null_mut();
        ctx.width = 0;
        ctx.height = 0;
        self.running = false;
    }
    
    pub fn on_touch_event(&mut self, touch: TouchEvent) {
        let event = PlatformEvent {
            kind: PlatformEventKind::Touch,
            timestamp: touch.timestamp,
        };
        
        self.event_bus.lock().dispatch(event);
        
        for callback in self.event_callbacks.lock().iter() {
            callback(&event);
        }
    }
    
    pub fn on_key_event(&mut self, key: KeyEvent) {
        let event = PlatformEvent {
            kind: PlatformEventKind::Key,
            timestamp: 0,
        };
        
        self.event_bus.lock().dispatch(event);
        
        for callback in self.event_callbacks.lock().iter() {
            callback(&event);
        }
    }
}

impl Default for HarmonyPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for HarmonyPlatform {
    fn name(&self) -> &'static str {
        "HarmonyOS"
    }
    
    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    fn shutdown(&mut self) {
        self.running = false;
    }
    
    fn create_window(&mut self, _title: &str, width: i32, height: i32) -> Result<WindowHandle, String> {
        let ctx = self.window_ctx.lock();
        Ok(WindowHandle::new(
            NativeWindowType::HarmonyOHNativeWindow,
            ctx.window as usize,
            width,
            height,
        ))
    }
    
    fn destroy_window(&mut self, _window: &WindowHandle) {
        self.on_surface_destroyed();
    }
    
    fn get_window_handle(&self) -> Option<WindowHandle> {
        let ctx = self.window_ctx.lock();
        if ctx.window.is_null() {
            None
        } else {
            Some(WindowHandle::new(
                NativeWindowType::HarmonyOHNativeWindow,
                ctx.window as usize,
                ctx.width,
                ctx.height,
            ))
        }
    }
    
    fn set_window_title(&mut self, _window: &WindowHandle, _title: &str) {
    }
    
    fn set_window_size(&mut self, _window: &WindowHandle, width: i32, height: i32) {
        let mut ctx = self.window_ctx.lock();
        ctx.width = width;
        ctx.height = height;
    }
    
    fn get_window_size(&self, _window: &WindowHandle) -> (i32, i32) {
        let ctx = self.window_ctx.lock();
        (ctx.width, ctx.height)
    }
    
    fn poll_events(&mut self) -> Vec<PlatformEvent> {
        Vec::new()
    }
    
    fn wait_events(&mut self) -> Vec<PlatformEvent> {
        Vec::new()
    }
    
    fn register_event_callback(&mut self, callback: EventCallback) {
        self.event_callbacks.lock().push(callback);
    }
    
    fn get_time(&self) -> f64 {
        0.0
    }
    
    fn sleep(&self, seconds: f64) {
        std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
    }
    
    fn is_running(&self) -> bool {
        self.running
    }
    
    fn request_quit(&mut self) {
        self.running = false;
    }
    
    fn get_native_display(&self) -> Option<usize> {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_init() -> *mut HarmonyPlatform {
    let platform = Box::new(HarmonyPlatform::new());
    Box::into_raw(platform)
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_shutdown(platform: *mut HarmonyPlatform) {
    if !platform.is_null() {
        unsafe {
            let _ = Box::from_raw(platform);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_on_surface_created(
    platform: *mut HarmonyPlatform,
    window: *mut OH_NativeWindow,
    width: i32,
    height: i32,
) {
    if !platform.is_null() {
        unsafe {
            (*platform).on_surface_created(window, width, height);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_on_surface_resize(
    platform: *mut HarmonyPlatform,
    width: i32,
    height: i32,
) {
    if !platform.is_null() {
        unsafe {
            (*platform).on_surface_resize(width, height);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_on_surface_destroyed(platform: *mut HarmonyPlatform) {
    if !platform.is_null() {
        unsafe {
            (*platform).on_surface_destroyed();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_on_touch_event(
    platform: *mut HarmonyPlatform,
    action: i32,
    x: f32,
    y: f32,
    pointer_id: i32,
    timestamp: u64,
) {
    if !platform.is_null() {
        unsafe {
            let touch_action = match action {
                0 => TouchAction::Begin,
                1 => TouchAction::Move,
                2 => TouchAction::End,
                _ => TouchAction::Cancel,
            };
            
            let touch = TouchEvent {
                action: touch_action,
                x,
                y,
                pointer_id,
            };
            
            (*platform).on_touch_event(touch);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_platform_get_window_handle(
    platform: *mut HarmonyPlatform,
) -> WindowHandle {
    if platform.is_null() {
        return WindowHandle::null();
    }
    
    unsafe {
        (*platform).get_window_handle().unwrap_or(WindowHandle::null())
    }
}

unsafe impl Send for OH_NativeWindow {}