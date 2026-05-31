use std::ffi::{c_void, c_char};

pub type WidgetTreeHandle = *mut c_void;

pub type GetRootIdFn = extern "C" fn(WidgetTreeHandle) -> u64;
pub type ClearWidgetTreeFn = extern "C" fn(WidgetTreeHandle);
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
pub type DfxSetCounterFn = extern "C" fn(*mut c_void, *const c_char, *const c_char, i64);
pub type DfxPerfBeginFrameFn = extern "C" fn(*mut c_void);
pub type DfxPerfEndFrameFn = extern "C" fn(*mut c_void);
pub type SetStatusTextFn = extern "C" fn(*const c_char);
pub type OnHotReloadCompleteFn = extern "C" fn();
pub type RegisterHotReloadCompleteCallbackFn = extern "C" fn(OnHotReloadCompleteFn);
pub type DebugPrintWidgetTreeFn = extern "C" fn(WidgetTreeHandle);
pub type SetFlexExpandFn = extern "C" fn(WidgetTreeHandle, u64, u32);
pub type SetCrossAxisFillFn = extern "C" fn(WidgetTreeHandle, u64, u32);
pub type SetWidgetBackgroundColorFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32);
pub type SetLabelWrapModeFn = extern "C" fn(WidgetTreeHandle, u64, u32);

pub type EventDispatcherHandle = *mut c_void;

pub type SimulateClickAtFn = extern "C" fn(WidgetTreeHandle, EventDispatcherHandle, f32, f32) -> u64;
pub type WidgetGetTypeFn = extern "C" fn(WidgetTreeHandle, u64, *mut u8, u32) -> u32;
pub type WidgetGetLayoutFn = extern "C" fn(WidgetTreeHandle, u64, *mut f32) -> u32;
pub type WidgetGetAbsoluteLayoutFn = extern "C" fn(WidgetTreeHandle, u64, *mut f32) -> u32;
pub type WidgetGetParentFn = extern "C" fn(WidgetTreeHandle, u64) -> u64;
pub type WidgetGetChildCountFn = extern "C" fn(WidgetTreeHandle, u64) -> u32;
pub type WidgetGetChildIdFn = extern "C" fn(WidgetTreeHandle, u64, u32) -> u64;
pub type WidgetGetTotalCountFn = extern "C" fn(WidgetTreeHandle) -> u32;
pub type DebugDumpTreeToBufferFn = extern "C" fn(WidgetTreeHandle, *mut u8, u32) -> u32;
pub type CaptureScreenshotToFileFn = extern "C" fn(*const c_char) -> i32;

pub type GetFocusedInputFieldFn = extern "C" fn() -> u64;