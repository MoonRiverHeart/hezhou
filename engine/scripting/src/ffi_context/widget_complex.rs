use std::ffi::{c_void, c_char};
use super::types::WidgetTreeHandle;

pub type CreateTextEditFn = extern "C" fn(WidgetTreeHandle, f32, f32) -> u64;
pub type CreateTextEditInParentFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type TextEditSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type TextEditInsertCharFn = extern "C" fn(WidgetTreeHandle, u64, c_char);
pub type TextEditDeleteCharFn = extern "C" fn(WidgetTreeHandle, u64);
pub type TextEditGetTextLenFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type TextEditGetTextFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize);
pub type TextEditShowLineNumbersFn = extern "C" fn(WidgetTreeHandle, u64, bool);

pub type CreateDropdownFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type DropdownSetOptionsFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char, usize);
pub type DropdownSetSelectedFn = extern "C" fn(WidgetTreeHandle, u64, usize);
pub type DropdownGetSelectedFn = extern "C" fn(WidgetTreeHandle, u64) -> usize;
pub type DropdownSetOnSelectThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);

pub type CreateInputFieldFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;
pub type InputFieldSetTextFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);
pub type InputFieldGetTextFn = extern "C" fn(WidgetTreeHandle, u64, *mut c_char, usize) -> usize;
pub type InputFieldSetOnChangeThunkPtrFn = extern "C" fn(WidgetTreeHandle, u64, *const c_void);
pub type InputFieldSetPlaceholderFn = extern "C" fn(WidgetTreeHandle, u64, *const c_char);