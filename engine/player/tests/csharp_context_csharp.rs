//! C# ↔ Rust PlayerCSharpContext 端到端测试（feature = mono-host）：
//! 编译真实 `engine/scripts/PlayerGame.cs`，通过 context 指针让 C# 创建 cube、
//! 设置/读取 position，Rust 侧断言 Scene 实际变化。
//!
//! 独立测试进程（Mono domain 只初始化一次）；无 mcs 时跳过。

#![cfg(feature = "mono-host")]

mod common;

use common::{compile_engine_player_game, write_test_wav, TempDir};
use hezhou_asset::{AssetId, AssetKind};
use hezhou_core::ecs::Scene;
use hezhou_player::animation_control::{AnimationClipEntry, PlayerAnimationState};
use hezhou_animation::gltf::AnimationAssetData;
use hezhou_animation::{
    AnimationClip, Quat, Skin, Skeleton, Transform, Vec3 as AnimVec3,
};
use hezhou_player::animation_graph::PlayerAnimationGraph;
use hezhou_player::asset_catalog::{PlayerAssetCatalog, PlayerAssetEntry};
use hezhou_player::audio_runtime::PlayerAudioRuntime;
use hezhou_player::csharp_context::PlayerCSharpContext;
use hezhou_player::gameplay_runtime::PlayerGameplayRuntime;
use hezhou_player::mono_host::PlayerGameScript;

fn test_animation_graph() -> PlayerAnimationGraph {
    let mut skeleton = Skeleton::new("hero");
    let root = skeleton.add_root_joint("root", Transform::IDENTITY);
    skeleton
        .add_joint(
            "hand",
            root,
            Transform::from_trs(AnimVec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
        )
        .unwrap();
    skeleton.rebuild_inverse_bind_matrices().unwrap();
    let skin = Skin::from_skeleton(&skeleton);
    let clips = ["Idle", "Run", "Attack"]
        .iter()
        .map(|name| AnimationClip::new(*name, 1.0))
        .collect();
    PlayerAnimationGraph::from_asset_data(
        AnimationAssetData {
            skeleton,
            skins: vec![skin],
            clips,
        },
        Vec::new(),
    )
    .expect("animation graph")
}

#[test]
fn csharp_creates_and_positions_entity_through_context() {    let temp = TempDir::new("csharp-context-e2e");
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

    let mut animation = PlayerAnimationState::new(vec![
        AnimationClipEntry {
            name: Some("Spin".to_string()),
            duration: 1.0,
        },
        AnimationClipEntry {
            name: Some("Walk".to_string()),
            duration: 0.5,
        },
    ]);
    let catalog = PlayerAssetCatalog::from_entries(vec![
        PlayerAssetEntry {
            id: AssetId::parse("11111111-1111-4111-8111-111111111111").unwrap(),
            kind: AssetKind::Scene,
            output: "assets/main.scene".to_string(),
        },
        PlayerAssetEntry {
            id: AssetId::parse("22222222-2222-4222-8222-222222222222").unwrap(),
            kind: AssetKind::Audio,
            output: "assets/audio/test.wav".to_string(),
        },
    ]);
    let wav = write_test_wav(&temp, "audio/test.wav");
    let mut audio = PlayerAudioRuntime::null();
    audio.load_wav_file("test", &wav).expect("加载测试 WAV");
    let mut gameplay = PlayerGameplayRuntime::new(&mut scene as *mut Scene);
    let mut graph = test_animation_graph();
    let mut context = PlayerCSharpContext::new_with_animation_graph(
        &mut scene as *mut Scene,
        &mut animation as *mut PlayerAnimationState,
        &catalog as *const PlayerAssetCatalog as *mut PlayerAssetCatalog,
        &mut audio as *mut PlayerAudioRuntime,
        &mut gameplay as *mut PlayerGameplayRuntime,
        &mut graph as *mut PlayerAnimationGraph,
    );
    assert!(context.is_supported());

    let mut script = PlayerGameScript::load(&dll).expect("加载真实 PlayerGame.cs DLL");
    script
        .initialize(context.as_ptr() as usize)
        .expect("Initialize(context)");
    // C# Initialize 创建一个 cube + 设置 position/rotation/scale。
    assert_eq!(scene.entity_count(), 3);
    let created = scene
        .get_all_entities()
        .into_iter()
        .find(|entity| !before.contains(&entity.id))
        .expect("C# 应创建新实体");
    let position = scene.get_entity_position(created).expect("新实体有位置");
    assert_eq!((position.x, position.y, position.z), (2.5, 0.5, -1.5));
    let rotation = scene.get_entity_rotation(created).expect("新实体有旋转");
    let euler = rotation.to_euler_degrees();
    assert!((euler[1] - 90.0).abs() < 0.2, "rotation={euler:?}");
    let scale = scene.get_entity_scale(created).expect("新实体有缩放");
    assert_eq!((scale.x, scale.y, scale.z), (1.5, 1.5, 1.5));
    // context ABI v2 exists 函数。
    assert_eq!(
        unsafe { hezhou_player::scene_entity_exists(context.scene, created.id) },
        1
    );
    // context ABI v3 动画：C# Initialize 播放 clip0 并设置 time/speed。
    assert_eq!(animation.current(), 0);
    assert!((animation.time() - 0.25).abs() < 1e-6);
    assert!((animation.speed() - 1.5).abs() < 1e-6);
    assert!(animation.is_playing());
    // context ABI v4 音频：C# play→pause→resume→volume=0.5，最终 playing。
    assert!(audio.is_playing(), "C# Initialize 后应恢复播放");
    assert!((audio.volume() - 0.5).abs() < 1e-6);
    // context ABI v5 角色：C# 已对创建的 cube 调用 character_create。
    assert!(gameplay.has_character(created.id));
    assert_eq!(
        gameplay.position(created.id).map(|p| (p.x, p.y, p.z)),
        Some((2.5, 0.5, -1.5))
    );
    // context ABI v6 状态机：Initialize 后初始 Idle。
    assert_eq!(graph.current_state().name(), "Idle");

    // 每帧 C# Update 调用 character_step(+X 1m/s) 并写回 Scene；窗口 host 同时推进状态图。
    for _ in 0..90 {
        script.update(1.0 / 60.0).expect("Update");
        graph.update(1.0 / 60.0);
    }
    let gameplay_position = gameplay.position(created.id).expect("角色位置");
    assert!(
        gameplay_position.x > 2.5 + 0.2,
        "角色应沿 +X 位移：{gameplay_position:?}"
    );
    assert!(gameplay_position.y.abs() < 0.05, "应贴地：{gameplay_position:?}");
    assert!(gameplay.grounded(created.id), "应处于 grounded");
    let scene_position = scene.get_entity_position(created).expect("Scene 位置");
    assert!(
        (scene_position.x - gameplay_position.x).abs() < 1e-5,
        "Scene 位置应与角色位置同步"
    );
    script.shutdown();
    // C# Shutdown 销毁自己创建的 cube：实体数回到 2，exists=false。
    assert_eq!(scene.entity_count(), 2, "C# Shutdown 应销毁创建的 cube");
    assert_eq!(
        unsafe { hezhou_player::scene_entity_exists(context.scene, created.id) },
        0
    );
    assert!(!gameplay.has_character(created.id), "Shutdown 应移除角色");
    // C# 在 frame1 触发 Run、frame15 触发 Attack；最终状态 Attack。
    assert_eq!(graph.current_state().name(), "Attack");
    assert!(graph.transition_count() >= 2);
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
        log.contains("rotationSet=0") && log.contains("rotationGet=0"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("scaleSet=0") && log.contains("scaleGet=0") && log.contains("scale=(1.500,1.500,1.500)"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("SceneContext: frame=1 entities=3 cube="),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("SceneContext: Shutdown entities=2")
            && log.contains("existsBefore=1")
            && log.contains("destroyResult=0")
            && log.contains("existsAfter=0"),
        "C# 日志：{log}"
    );
    // v3 动画枚举/播放/时间/速度。
    assert!(
        log.contains("Animation: clips=2 [0:Spin(1.000s), 1:Walk(0.500s)]"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("Animation: play clip=0")
            && log.contains("time=0.250")
            && log.contains("speed=1.500"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("Animation: Shutdown current=0 time=0.250 speed=1.500"),
        "C# 日志：{log}"
    );
    // C# Shutdown 不改变 Rust 动画状态。
    assert_eq!(animation.current(), 0);
    assert!((animation.time() - 0.25).abs() < 1e-6);
    assert!((animation.speed() - 1.5).abs() < 1e-6);
    // C# Shutdown 停止音频。
    assert!(!audio.is_playing(), "C# Shutdown 应停止音频");

    // v4 资产/音频日志。
    assert!(
        log.contains("Assets: count=2 ["),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("kind=audio") || log.contains(":audio:assets/audio/test.wav:"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("Audio: backend=null")
            && log.contains("playResult=0")
            && log.contains("isPlayingAfterPause=0")
            && log.contains("resumeResult=0")
            && log.contains("isPlayingAfterResume=1")
            && log.contains("volumeResult=0 volume=0.500"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("Audio: Shutdown isPlaying=1 stopResult=0 isPlayingAfterStop=0"),
        "C# 日志：{log}"
    );
    // v5 角色控制日志。
    assert!(
        log.contains("Character: create entity=")
            && log.contains("radius=0.300 height=1.800 result=0"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("Character: frame=1 stepResult=0")
            && log.contains("position=("),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("Character: Shutdown posResult=0")
            && log.contains("grounded=1")
            && log.contains("removeResult=0"),
        "C# 日志：{log}"
    );
    // v6 状态机日志。
    assert!(
        log.contains("AnimGraph: initial state=Idle"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("AnimGraph: trigger=Run result=0"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("AnimGraph: trigger=Attack result=0"),
        "C# 日志：{log}"
    );
    assert!(
        log.contains("AnimGraph: Shutdown state=Attack"),
        "C# 日志：{log}"
    );
    // Rust 侧统计：Initialize/Update×90/Shutdown 各调用成功。
    let stats = script.stats();
    assert_eq!(stats.initialize_calls, 1);
    assert_eq!(stats.update_calls, 90);
}
