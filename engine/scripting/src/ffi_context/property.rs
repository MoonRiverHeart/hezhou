use std::ffi::{c_void, c_char};

pub type EntityGetPropertyCountFn = extern "C" fn() -> u32;
pub type EntityGetPropertyNameFn = extern "C" fn(u32) -> *const c_char;
pub type EntityGetPropertyTypeFn = extern "C" fn(u32) -> u32;
pub type EntityGetPropertyCategoryFn = extern "C" fn(u32) -> *const c_char;
pub type EntityGetPropertyReadOnlyFn = extern "C" fn(u32) -> bool;
pub type EntityGetPropertyValueFloat3Fn = extern "C" fn(*mut c_void, u64, *const c_char, *mut f32, *mut f32, *mut f32) -> bool;
pub type EntitySetPropertyValueFloat3Fn = extern "C" fn(*mut c_void, u64, *const c_char, f32, f32, f32) -> bool;
pub type EntityGetPropertyValueStringFn = extern "C" fn(*mut c_void, u64, *const c_char, *mut c_char, u32) -> u32;
pub type EntitySetPropertyValueStringFn = extern "C" fn(*mut c_void, u64, *const c_char, *const c_char) -> bool;