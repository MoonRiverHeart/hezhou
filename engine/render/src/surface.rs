use hezhou_harmony::OH_NativeWindow;
use hezhou_core::math::*;

pub struct RenderSurface {
    window: *mut OH_NativeWindow,
    width: i32,
    height: i32,
    initialized: bool,
}

impl RenderSurface {
    pub fn create(window: *mut OH_NativeWindow, width: i32, height: i32) -> Self {
        Self {
            window,
            width,
            height,
            initialized: false,
        }
    }
    
    pub fn init(&mut self) -> i32 {
        self.initialized = true;
        0
    }
    
    pub fn resize(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }
    
    pub fn make_current(&self) {
    }
    
    pub fn present(&self) {
    }
    
    pub fn destroy(&mut self) {
        self.initialized = false;
    }
    
    pub fn get_width(&self) -> i32 {
        self.width
    }
    
    pub fn get_height(&self) -> i32 {
        self.height
    }
    
    pub fn get_window(&self) -> *mut OH_NativeWindow {
        self.window
    }
}

impl Drop for RenderSurface {
    fn drop(&mut self) {
        self.destroy();
    }
}