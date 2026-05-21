use std::ffi::c_char;

pub type GetButtonIdFn = extern "C" fn() -> u64;
pub type SetButtonIdFn = extern "C" fn(u64);
pub type SetTextFn = extern "C" fn(super::types::WidgetTreeHandle, u64, *const c_char);
pub type TriggerResizeFn = extern "C" fn(f32, f32);
pub type GetScreenSizeFn = extern "C" fn(*mut f32, *mut f32);
pub type SetContentScaleFn = extern "C" fn(f32);
pub type GetContentScaleFn = extern "C" fn() -> f32;
pub type TriggerHotReloadFn = extern "C" fn();