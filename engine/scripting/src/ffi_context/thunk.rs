use std::ffi::c_void;

pub type SetOnClickThunkPtrFn = extern "C" fn(super::types::WidgetTreeHandle, u64, *const c_void);
pub type RegisterUpdateThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterResizeThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterGlobalClickThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterKeyThunkPtrFn = extern "C" fn(*const c_void);
pub type RegisterMouseMoveThunkPtrFn = extern "C" fn(*const c_void);