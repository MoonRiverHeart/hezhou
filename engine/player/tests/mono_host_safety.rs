//! mono-host 加载失败安全测试（独立测试进程，避免 Mono domain 重复初始化）。

#![cfg(feature = "mono-host")]

mod common;

use common::{cooked_fixture_dir, TempDir};
use hezhou_player::mono_host::{MonoHostError, PlayerGameScript};

#[test]
fn invalid_dll_load_failure_is_safe_and_readable() {
    let temp = TempDir::new("mono-invalid-dll");
    // cooked fixture 的 scripts/Game.dll 是 21 字节占位文件（不是有效 .NET 程序集）。
    let source = cooked_fixture_dir().join("scripts").join("Game.dll");
    let dll = temp.write("Game.dll", &std::fs::read(source).unwrap());
    let error = PlayerGameScript::load(&dll).expect_err("无效 DLL 必须失败");
    assert!(
        matches!(error, MonoHostError::Scripting(_)),
        "实际错误：{error}"
    );
    let text = error.to_string();
    assert!(text.contains("Mono 脚本错误"), "错误：{text}");
}
