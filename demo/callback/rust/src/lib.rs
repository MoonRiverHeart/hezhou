use std::sync::Mutex;

type CallbackFn = extern "C" fn(i32) -> i32;

static CALLBACK: Mutex<Option<CallbackFn>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub extern "C" fn register_callback(callback: CallbackFn) {
    let mut cb = CALLBACK.lock().unwrap();
    *cb = Some(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn trigger_callback(value: i32) -> i32 {
    let cb = CALLBACK.lock().unwrap();
    match *cb {
        Some(callback) => {
            println!("[Rust] 调用 C# 回调，参数: {}", value);
            let result = callback(value);
            println!("[Rust] C# 回调返回: {}", result);
            result
        }
        None => {
            println!("[Rust] 未注册回调");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn clear_callback() {
    let mut cb = CALLBACK.lock().unwrap();
    *cb = None;
    println!("[Rust] 回调已清除");
}