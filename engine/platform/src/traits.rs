use crate::event::*;
use crate::window::*;

pub type EventCallback = extern "C" fn(&PlatformEvent);

pub trait Platform {
    fn name(&self) -> &'static str;

    fn init(&mut self) -> Result<(), String>;
    fn shutdown(&mut self);

    fn create_window(
        &mut self,
        title: &str,
        width: i32,
        height: i32,
    ) -> Result<WindowHandle, String>;
    fn destroy_window(&mut self, window: &WindowHandle);
    fn get_window_handle(&self) -> Option<WindowHandle>;

    fn set_window_title(&mut self, window: &WindowHandle, title: &str);
    fn set_window_size(&mut self, window: &WindowHandle, width: i32, height: i32);
    fn get_window_size(&self, window: &WindowHandle) -> (i32, i32);

    fn poll_events(&mut self) -> Vec<PlatformEvent>;
    fn wait_events(&mut self) -> Vec<PlatformEvent>;

    fn register_event_callback(&mut self, callback: EventCallback);

    fn get_time(&self) -> f64;
    fn sleep(&self, seconds: f64);

    fn is_running(&self) -> bool;
    fn request_quit(&mut self);

    fn get_native_display(&self) -> Option<usize>;
}
