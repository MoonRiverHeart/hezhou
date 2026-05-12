use parking_lot::Mutex;
use std::sync::Arc;

#[repr(C)]
pub struct OH_NativeWindow {
    _private: [u8; 0],
}

pub type SurfaceCallback = extern "C" fn(*mut OH_NativeWindow, i32, i32);

pub struct NativeWindowContext {
    window: *mut OH_NativeWindow,
    width: i32,
    height: i32,
    surface_callbacks: Arc<Mutex<Vec<SurfaceCallback>>>,
}

impl NativeWindowContext {
    pub fn new() -> Self {
        Self {
            window: std::ptr::null_mut(),
            width: 0,
            height: 0,
            surface_callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn register_surface_callback(&mut self, callback: SurfaceCallback) {
        self.surface_callbacks.lock().push(callback);
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

#[unsafe(no_mangle)]
pub extern "C" fn harmony_on_surface_created(
    ctx: *mut NativeWindowContext,
    window: *mut OH_NativeWindow,
    width: i32,
    height: i32,
) {
    if ctx.is_null() {
        return;
    }
    
    let context = unsafe { &mut *ctx };
    context.window = window;
    context.width = width;
    context.height = height;
    
    for callback in context.surface_callbacks.lock().iter() {
        callback(window, width, height);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_on_surface_resize(
    ctx: *mut NativeWindowContext,
    width: i32,
    height: i32,
) {
    if ctx.is_null() {
        return;
    }
    
    let context = unsafe { &mut *ctx };
    context.width = width;
    context.height = height;
    
    for callback in context.surface_callbacks.lock().iter() {
        callback(context.window, width, height);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn harmony_on_surface_destroyed(ctx: *mut NativeWindowContext) {
    if ctx.is_null() {
        return;
    }
    
    let context = unsafe { &mut *ctx };
    context.window = std::ptr::null_mut();
    context.width = 0;
    context.height = 0;
}