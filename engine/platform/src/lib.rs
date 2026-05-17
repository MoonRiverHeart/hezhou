pub mod event;
pub mod traits;
pub mod window;

#[cfg(feature = "glfw")]
pub mod glfw_backend;

#[cfg(feature = "harmony")]
pub mod harmony_backend;

pub use event::*;
pub use traits::*;
pub use window::*;

#[cfg(feature = "glfw")]
pub use glfw_backend::GLFWPlatform;

#[cfg(feature = "harmony")]
pub use harmony_backend::HarmonyPlatform;

use parking_lot::Mutex;
use std::sync::Arc;

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
    pub fn new() -> Self {
        Self {
            backend: None,
            event_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[cfg(feature = "glfw")]
    pub fn create_glfw_platform(&mut self) -> Result<(), String> {
        let mut platform = GLFWPlatform::new();
        platform.init()?;
        self.backend = Some(PlatformBackend::GLFW(platform));
        Ok(())
    }

    #[cfg(feature = "harmony")]
    pub fn create_harmony_platform(&mut self) -> Result<(), String> {
        let mut platform = HarmonyPlatform::new();
        platform.init()?;
        self.backend = Some(PlatformBackend::Harmony(platform));
        Ok(())
    }

    pub fn get_platform(&self) -> Option<&dyn Platform> {
        match &self.backend {
            #[cfg(feature = "glfw")]
            Some(PlatformBackend::GLFW(p)) => Some(p),
            #[cfg(feature = "harmony")]
            Some(PlatformBackend::Harmony(p)) => Some(p),
            None => None,
        }
    }

    pub fn get_platform_mut(&mut self) -> Option<&mut dyn Platform> {
        match &mut self.backend {
            #[cfg(feature = "glfw")]
            Some(PlatformBackend::GLFW(p)) => Some(p),
            #[cfg(feature = "harmony")]
            Some(PlatformBackend::Harmony(p)) => Some(p),
            None => None,
        }
    }

    pub fn poll_events(&mut self) -> Vec<PlatformEvent> {
        if let Some(platform) = self.get_platform_mut() {
            let events = platform.poll_events();
            self.event_queue.lock().extend(events.clone());
            events
        } else {
            Vec::new()
        }
    }

    pub fn get_queued_events(&self) -> Vec<PlatformEvent> {
        self.event_queue.lock().clone()
    }

    pub fn clear_event_queue(&mut self) {
        self.event_queue.lock().clear();
    }
}

impl Default for PlatformManager {
    fn default() -> Self {
        Self::new()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_manager_create() -> *mut PlatformManager {
    let manager = Box::new(PlatformManager::new());
    Box::into_raw(manager)
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_manager_destroy(manager: *mut PlatformManager) {
    if !manager.is_null() {
        unsafe {
            let mut mgr = Box::from_raw(manager);
            if let Some(platform) = mgr.get_platform_mut() {
                platform.shutdown();
            }
        }
    }
}

#[cfg(feature = "glfw")]
#[unsafe(no_mangle)]
pub extern "C" fn platform_init_glfw(manager: *mut PlatformManager) -> i32 {
    if manager.is_null() {
        return -1;
    }

    unsafe {
        match (*manager).create_glfw_platform() {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("GLFW init failed: {}", e);
                -1
            }
        }
    }
}

#[cfg(feature = "harmony")]
#[unsafe(no_mangle)]
pub extern "C" fn platform_init_harmony(manager: *mut PlatformManager) -> i32 {
    if manager.is_null() {
        return -1;
    }

    unsafe {
        match (*manager).create_harmony_platform() {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("Harmony init failed: {}", e);
                -1
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_create_window(
    manager: *mut PlatformManager,
    title: *const std::os::raw::c_char,
    width: i32,
    height: i32,
) -> WindowHandle {
    if manager.is_null() || title.is_null() {
        return WindowHandle::null();
    }

    unsafe {
        let title_str = std::ffi::CStr::from_ptr(title).to_str().unwrap_or("");

        if let Some(platform) = (*manager).get_platform_mut() {
            match platform.create_window(title_str, width, height) {
                Ok(handle) => handle,
                Err(_) => WindowHandle::null(),
            }
        } else {
            WindowHandle::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_poll_events(manager: *mut PlatformManager) -> i32 {
    if manager.is_null() {
        return 0;
    }

    unsafe {
        let events = (*manager).poll_events();
        events.len() as i32
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_is_running(manager: *mut PlatformManager) -> bool {
    if manager.is_null() {
        return false;
    }

    unsafe {
        (*manager)
            .get_platform()
            .map(|p| p.is_running())
            .unwrap_or(false)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_request_quit(manager: *mut PlatformManager) {
    if manager.is_null() {
        return;
    }

    unsafe {
        if let Some(platform) = (*manager).get_platform_mut() {
            platform.request_quit();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_get_time(manager: *mut PlatformManager) -> f64 {
    if manager.is_null() {
        return 0.0;
    }

    unsafe {
        (*manager)
            .get_platform()
            .map(|p| p.get_time())
            .unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_get_window_handle(manager: *mut PlatformManager) -> WindowHandle {
    if manager.is_null() {
        return WindowHandle::null();
    }

    unsafe {
        (*manager)
            .get_platform()
            .and_then(|p| p.get_window_handle())
            .unwrap_or(WindowHandle::null())
    }
}
