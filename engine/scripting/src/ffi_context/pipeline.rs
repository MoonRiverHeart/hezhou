use std::ffi::c_char;
use crate::ffi_context::types::WidgetTreeHandle;

/// 获取所有已注册管线名称 — 返回NUL分隔的管线名列表
///
/// FFI契约: get_pipeline_names(handle, buffer_ptr, buffer_size) → usize
/// 返回值: 实际写入buffer的字节数（不含末尾\0）
/// buffer内容: "raster\0ray_tracing\0" 格式（NUL分隔）
pub type GetPipelineNamesFn = extern "C" fn(WidgetTreeHandle, *mut c_char, usize) -> usize;

/// 切换活跃渲染管线
///
/// FFI契约: switch_pipeline(handle, pipeline_name_ptr) → i32
/// 返回值: 0=成功, -1=失败
pub type SwitchPipelineFn = extern "C" fn(WidgetTreeHandle, *const c_char) -> i32;

/// 获取当前活跃管线名称
///
/// FFI契约: get_active_pipeline_name(handle, buffer_ptr, buffer_size) → usize
/// 返回值: 实际写入buffer的字节数（不含末尾\0）
pub type GetActivePipelineNameFn = extern "C" fn(WidgetTreeHandle, *mut c_char, usize) -> usize;