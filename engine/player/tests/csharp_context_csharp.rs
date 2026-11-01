//! C# ↔ Rust PlayerCSharpContext 端到端测试（feature = mono-host）：
//! 编译真实 `engine/scripts/PlayerGame.cs`，通过 context 指针让 C# 创建 cube、
//! 设置/读取 position，Rust 侧断言 Scene 实际变化。
//!
//! 独立测试进程（Mono domain 只初始化一次）；无 mcs 时跳过。

#![cfg(feature = "mono-host")]

mod common;

use common::{compile_engine_player_game, TempDir};
use hezhou_core::ecs::Scene;
use hezhou_player::csharp_context::PlayerCSharpContext;
use hezhou_player::mono_host::PlayerGameScript;

#[test]
fn csharp_creates_and_positions_entity_through_context() {
    let temp = TempDir::new("csharp-context-e2e");
    let Some(dll) = compile_engine_player_game(&temp) else {
        return;
    };
    let log_path = temp.join("csharp-context.log");
    std::env::set_var("HEZHOU_PLAYER_MONO_LOG", &log_path);

    let mut scene = Scene::new();
    let _existing_a = scene.create_entity();
    let _existing_b = scene.create_cube();
    assert_eq!(scene.entity_count(), 2);
    let before: Vec<u64> = scene.get_all_entities().iter().map(|e| e.id).collect();

    let mut context = PlayerCSharpContext::new(&mut scene as *mut Scene);
    assert!(context.is_supported());

    let mut script = PlayerGameScript::load(&dll).expect("加载真实 PlayerGame.cs DLL");
    script
        .initialize(context.as_ptr() as usize)
        .expect("Initialize(context)");
    // C# Initialize 创建一个 cube + 设置 position。
    assert_eq!(scene.entity_count(), 3);
    let created = scene
        .get_all_entities()
        .into_iter()
        .find(|entity| !before.contains(&entity.id))
        .expect("C# 应创建新实体");
    let position = scene.get_entity_position(created).expect("新实体有位置");
    assert_eq!((position.x, position.y, position.z), (2.5, 0.5, -1.5));

    script.update(0.016).expect("Update");
    script.update(0.032).expect("Update");
    script.shutdown();
    assert_eq!(scene.entity_count(), 3, "C# Shutdown 不删除实体");
    std::env::remove_var("HEZHOU_PLAYER_MONO_LOG");

    let log = std::fs::read_to_string(&log_path).expect("读取 C# 日志");
    assert!(
        log.contains("SceneContext: Initialize 创建 cube="),
        "C# 日志：{log}"
    );
    assert!(log.contains("entities 2->3"), "C# 日志：{log}");
    assert!(
        log.contains("position=(2.500,0.500,-1.500)"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("SceneContext: frame=1 entities=3 cube="),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("SceneContext: Shutdown entities=3"),
        "C# 日志：{log}"
    );
    // Rust 侧统计：Initialize/Update×2/Shutdown 各调用成功。
    let stats = script.stats();
    assert_eq!(stats.initialize_calls, 1);
    assert_eq!(stats.update_calls, 2);
}
