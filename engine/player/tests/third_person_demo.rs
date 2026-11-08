//! 第三人称 C# demo 端到端测试（task-59，feature = mono-host）：
//! 真实 `PlayerGame.cs` + ABI v7 输入/相机，Rust 侧断言角色移动/跳跃/相机跟随。

#![cfg(feature = "mono-host")]

mod common;

use common::{compile_engine_player_game, TempDir};
use hezhou_core::ecs::Scene;
use hezhou_player::camera_state::PlayerCameraState;
use hezhou_player::csharp_context::PlayerCSharpContext;
use hezhou_player::gameplay_runtime::PlayerGameplayRuntime;
use hezhou_player::input_state::PlayerInputState;
use hezhou_player::mono_host::PlayerGameScript;

struct DemoEnvGuard;

impl DemoEnvGuard {
    fn set() -> Self {
        std::env::set_var("HEZHOU_PLAYER_DEMO", "third_person");
        std::env::remove_var("HEZHOU_PLAYER_DEMO_AUTOPILOT");
        Self
    }
}

impl Drop for DemoEnvGuard {
    fn drop(&mut self) {
        std::env::remove_var("HEZHOU_PLAYER_DEMO");
    }
}

#[test]
fn csharp_third_person_demo_moves_jumps_and_follows_camera() {
    let temp = TempDir::new("third-person-demo-e2e");
    let Some(dll) = compile_engine_player_game(&temp) else {
        return;
    };
    let log_path = temp.join("third-person.log");
    std::env::set_var("HEZHOU_PLAYER_MONO_LOG", &log_path);
    let _guard = DemoEnvGuard::set();

    let mut scene = Scene::new();
    let mut input = PlayerInputState::new();
    let mut camera = PlayerCameraState::new();
    let mut gameplay = PlayerGameplayRuntime::new(&mut scene as *mut Scene);
    let context = PlayerCSharpContext::new_with_player_states(
        &mut scene as *mut Scene,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        &mut gameplay as *mut PlayerGameplayRuntime,
        std::ptr::null_mut(),
        &mut input as *mut PlayerInputState,
        &mut camera as *mut PlayerCameraState,
    );
    assert!(context.is_supported());

    let mut script = PlayerGameScript::load(&dll).expect("加载 PlayerGame.cs DLL");
    script
        .initialize(context.as_ptr() as usize)
        .expect("Initialize(context)");
    assert!(scene.entity_count() >= 8, "demo 应创建地面+角色部件");
    let controller = camera.target().expect("C# 初始化应设置相机跟随目标");
    assert!(gameplay.has_character(controller), "控制器应有 CharacterMotor");

    // 10 帧 +X 移动。
    for _ in 0..10 {
        input.set(1.0, 0.0, false);
        script.update(1.0 / 60.0).expect("Update");
    }
    let moved = gameplay.position(controller).expect("角色位置");
    assert!(moved.x > 0.2, "角色应向前移动：{moved:?}");

    // 跳跃一帧，随后若干帧观察离地。
    input.set(0.0, 0.0, true);
    script.update(1.0 / 60.0).expect("Update jump");
    let mut saw_airborne = !gameplay.grounded(controller);
    for _ in 0..20 {
        input.set(0.0, 0.0, false);
        script.update(1.0 / 60.0).expect("Update");
        saw_airborne |= !gameplay.grounded(controller);
    }
    assert!(saw_airborne, "跳跃后应出现 grounded=false");
    assert_eq!(camera.target(), Some(controller));
    let pose = camera.pose_for(&scene).expect("相机位姿");
    let feet = gameplay.position(controller).expect("角色位置");
    assert!(
        (pose.y - (feet.y + 2.5)).abs() < 0.2,
        "相机高度应约为脚底+2.5：pose={pose:?} feet={feet:?}"
    );

    script.shutdown();
    assert!(camera.target().is_none(), "Shutdown 应 camera_clear");
    assert!(!gameplay.has_character(controller), "Shutdown 应移除角色");
    assert_eq!(scene.entity_count(), 0, "Shutdown 应销毁 demo 创建的实体");

    let log = std::fs::read_to_string(&log_path).expect("读取 C# 日志");
    assert!(log.contains("ThirdPerson: ground="), "C# 日志：{log}");
    assert!(log.contains("ThirdPerson: frame=1"), "C# 日志：{log}");
    assert!(log.contains("ThirdPerson: jump frame="), "C# 日志：{log}");
    assert!(log.contains("ThirdPerson: Shutdown position="), "C# 日志：{log}");
    assert!(log.contains("ThirdPerson: demo 结束"), "C# 日志：{log}");
}
