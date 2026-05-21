use std::ffi::c_char;
use super::types::WidgetTreeHandle;

pub type CreateButtonFn = extern "C" fn(WidgetTreeHandle, f32, f32, f32, f32, *const c_char) -> u64;
pub type CreateLabelFn = extern "C" fn(WidgetTreeHandle, f32, f32, f32, f32, *const c_char) -> u64;
pub type CreatePanelFn = extern "C" fn(WidgetTreeHandle, f32, f32, f32, f32) -> u64;
pub type CreateVStackFn = extern "C" fn(WidgetTreeHandle, f32) -> u64;
pub type CreateVStackInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32) -> u64;
pub type CreateHStackFn = extern "C" fn(WidgetTreeHandle, f32) -> u64;
pub type CreateHStackInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32) -> u64;
pub type CreateButtonInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, *const c_char) -> u64;
pub type CreateLabelInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, *const c_char) -> u64;
pub type CreatePanelInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32, f32, f32, f32, f32, f32, f32) -> u64;

pub type CreateListFn = extern "C" fn(WidgetTreeHandle, f32, u32) -> u64;
pub type CreateListInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, u32) -> u64;
pub type CreateListItemFn = extern "C" fn(WidgetTreeHandle, *const c_char) -> u64;
pub type CreateListItemInParentFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, u32) -> u64;
pub type ListItemSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type ListItemSetFontSizeFn = extern "C" fn(WidgetTreeHandle, u64, f32);