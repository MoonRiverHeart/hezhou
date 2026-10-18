#![no_main]

use libfuzzer_sys::fuzz_target;

// 目标：ProjectFile 的 JSON 反序列化面对任意字节流都只能返回 Err，不允许 panic。
fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<hezhou_core::ProjectFile>(text);
    }
});
