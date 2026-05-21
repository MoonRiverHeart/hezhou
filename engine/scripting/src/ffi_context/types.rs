use std::ffi::{c_void, c_char};

pub type WidgetTreeHandle = *mut c_void;

pub type GetRootIdFn = extern "C" fn(WidgetTreeHandle) -> u64;
pub type SetWidgetLayoutFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32);
pub type SetPositionFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type SetSizeFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32);
pub type SetWidgetLayerFn = extern "C" fn(WidgetTreeHandle, u64, u32);
pub type GetWidgetLayerFn = extern "C" fn(WidgetTreeHandle, u64) -> u32;
pub type RemoveWidgetFn = extern "C" fn(WidgetTreeHandle, u64);
pub type CreatePreviewWindowFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, u64) -> u64;
pub type SetPreviewTextureFn = extern "C" fn(WidgetTreeHandle, u64, u64);
pub type IsPreviewWindowSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> bool;
pub type SetPreviewWindowSelectedFn = extern "C" fn(WidgetTreeHandle, u64, bool);
pub type SetPreviewWindowEditModeFn = extern "C" fn(WidgetTreeHandle, u64, bool);

pub type DfxLogFn = extern "C" fn(*mut c_void, u8, *const c_char, *const c_char, *const c_char, u32);
pub type DfxTraceBeginFn = extern "C" fn(*mut c_void, *const c_char, *const c_char);
pub type DfxTraceEndFn = extern "C" fn(*mut c_void, *const c_char, *const c_char);
pub type SetStatusTextFn = extern "C" fn(*const c_char);
pub type OnHotReloadCompleteFn = extern "C" fn();
pub type DebugPrintWidgetTreeFn = extern "C" fn(WidgetTreeHandle);