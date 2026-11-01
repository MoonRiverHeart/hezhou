//! C# PlayerGame 在 context=0 时安全空转（feature = mono-host，独立测试进程）。

#![cfg(feature = "mono-host")]

mod common;

use common::{compile_engine_player_game, TempDir};
use hezhou_player::mono_host::PlayerGameScript;

#[test]
fn csharp_player_game_handles_null_context_safely() {
    let temp = TempDir::new("csharp-context-null");
    let Some(dll) = compile_engine_player_game(&temp) else {
        return;
    };
    let log_path = temp.join("csharp-null.log");
    std::env::set_var("HEZHOU_PLAYER_MONO_LOG", &log_path);

    let mut script = PlayerGameScript::load(&dll).expect("加载真实 PlayerGame.cs DLL");
    script.initialize(0).expect("Initialize(0) 不应失败");
    script.update(0.016).expect("Update");
    script.shutdown();
    std::env::remove_var("HEZHOU_PLAYER_MONO_LOG");

    let log = std::fs::read_to_string(&log_path).expect("读取 C# 日志");
    assert!(
        log.contains("context=0x0，无 Scene Context，空转"),
        "C# 日志：{log}"
    );
    assert!(log.contains("PlayerGame: Shutdown"), "C# 日志：{log}");
}
