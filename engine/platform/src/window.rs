#[repr(C)]
pub enum NativeWindowType {
    Unknown = 0,
    GLFW = 1,
    HarmonyOHNativeWindow = 2,
    Win32 = 3,
    X11 = 4,
    Wayland = 5,
}

#[repr(C)]
pub struct WindowHandle {
    pub window_type: NativeWindowType,
    pub ptr: usize,
    pub width: i32,
    pub height: i32,
}

impl WindowHandle {
    pub fn new(window_type: NativeWindowType, ptr: usize, width: i32, height: i32) -> Self {
        Self {
            window_type,
            ptr,
            width,
            height,
        }
    }

    pub fn null() -> Self {
        Self {
            window_type: NativeWindowType::Unknown,
            ptr: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.ptr != 0 && self.width > 0 && self.height > 0
    }

    pub fn get_native_ptr(&self) -> *mut std::ffi::c_void {
        self.ptr as *mut std::ffi::c_void
    }

    pub fn get_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}
