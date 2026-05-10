// src/lib.rs

// ✅ 正确写法
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

// 如果需要返回字符串的示例
use std::ffi::CString;
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn get_message() -> *mut c_char {
    let msg = CString::new("Hello from Rust").unwrap();
    msg.into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn free_message(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}