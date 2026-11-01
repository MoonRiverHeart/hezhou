//! mono-host 真实 C# 调用测试（独立进程）：mcs 编译 fixture → 加载 → Initialize/Update/Shutdown。
//!
//! 需要本机 Mono/mcs；没有 mcs 时测试打印跳过信息并返回（不失败）。

#![cfg(feature = "mono-host")]

mod common;

use common::{compile_player_game_fixture, TempDir};
use hezhou_player::mono_host::PlayerGameScript;

#[test]
fn calls_player_game_initialize_update_shutdown() {
    let temp = TempDir::new("mono-call");
    let Some(dll) = compile_player_game_fixture(&temp) else {
        return;
    };
    let log_path = temp.join("mono.log");
    std::env::set_var("HEZHOU_PLAYER_MONO_LOG", &log_path);

    let mut script = PlayerGameScript::load(&dll).expect("加载 PlayerGame.dll");
    script.initialize(0).expect("Initialize(0)");
    script.update(0.016).expect("Update #1");
    script.update(0.032).expect("Update #2");
    assert_eq!(script.stats().initialize_calls, 1);
    assert_eq!(script.stats().update_calls, 2);
    script.shutdown();
    assert_eq!(script.stats().shutdown_calls, 1);
    std::env::remove_var("HEZHOU_PLAYER_MONO_LOG");

    let log = std::fs::read_to_string(&log_path).expect("读取 C# 调用日志");
    assert!(log.contains("Initialize context=0"), "C# 日志：{log}");
    assert!(log.contains("Update #1"), "C# 日志：{log}");
    assert!(log.contains("Update #2"), "C# 日志：{log}");
    assert!(log.contains("Shutdown updates=2"), "C# 日志：{log}");
}
