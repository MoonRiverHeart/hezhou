//! ThirdPersonCamera 与 LockOnState：跟随、视角、避障、锁定/解除。

mod support;

use hezhou_gameplay::{
    CombatantId, LockOnConfig, LockOnState, LockTargetCandidate, ThirdPersonCamera,
    ThirdPersonCameraConfig, Vec3,
};
use support::MockCameraQuery;

const DT: f32 = 1.0 / 60.0;

fn test_camera() -> (ThirdPersonCamera, ThirdPersonCameraConfig) {
    let config = ThirdPersonCameraConfig {
        follow_speed: 0.0, // 立即跟随，方便断言
        yaw_sensitivity: 1.0,
        pitch_sensitivity: 1.0,
        ..ThirdPersonCameraConfig::default()
    };
    let mut camera = ThirdPersonCamera::new(config);
    camera.set_yaw(0.0);
    camera.set_pitch(0.0);
    (camera, config)
}

fn candidate(id: u64, position: Vec3) -> LockTargetCandidate {
    LockTargetCandidate::new(CombatantId(id), position)
}

#[test]
fn camera_follows_anchor_at_configured_distance() {
    let (mut camera, config) = test_camera();
    let query = MockCameraQuery::default();
    let anchor = Vec3::new(0.0, 0.0, 0.0);

    let output = camera.update(DT, anchor, &query);

    let expected_look_at =
        anchor + Vec3::UP * config.height_offset + Vec3::RIGHT * config.shoulder_offset;
    assert!(
        output.look_at.distance(&expected_look_at) < 0.01,
        "注视点应位于角色头部：{:?}",
        output.look_at
    );
    assert!(
        (output.distance - config.distance).abs() < 0.01,
        "无遮挡时相机距离应等于配置距离，实际 {}",
        output.distance
    );
    assert!(!output.collided);
    assert!(output.forward.z < -0.9, "yaw=0 时应看向 -Z");
    assert!((output.position.y - config.height_offset).abs() < 0.01);
}

#[test]
fn camera_rotates_with_look_input_and_clamps_pitch() {
    let (mut camera, config) = test_camera();

    camera.apply_look(&hezhou_gameplay::CameraLookInput {
        yaw_delta: 0.5,
        pitch_delta: 0.0,
        zoom_delta: 0.0,
    });
    assert!((camera.yaw() - 0.5).abs() < 1e-4, "yaw 应累加输入");

    camera.apply_look(&hezhou_gameplay::CameraLookInput {
        yaw_delta: 0.0,
        pitch_delta: 10.0,
        zoom_delta: 0.0,
    });
    assert!(
        (camera.pitch() - config.max_pitch).abs() < 1e-4,
        "pitch 应被夹到上限，实际 {}",
        camera.pitch()
    );

    // 俯视时相机应在锚点上方。
    let query = MockCameraQuery::default();
    let output = camera.update(DT, Vec3::ZERO, &query);
    assert!(output.position.y > config.height_offset);
}

#[test]
fn camera_zoom_is_clamped() {
    let (mut camera, config) = test_camera();
    let zoom_in = hezhou_gameplay::CameraLookInput {
        zoom_delta: -100.0,
        ..Default::default()
    };
    camera.apply_look(&zoom_in);
    assert!((camera.distance() - config.min_distance).abs() < 1e-4);

    let zoom_out = hezhou_gameplay::CameraLookInput {
        zoom_delta: 100.0,
        ..Default::default()
    };
    camera.apply_look(&zoom_out);
    assert!((camera.distance() - config.max_distance).abs() < 1e-4);
}

#[test]
fn camera_collision_shortens_distance() {
    let (mut camera, config) = test_camera();
    let query = MockCameraQuery {
        obstacle_ratio: Some(0.5),
        ..MockCameraQuery::default()
    };

    let output = camera.update(DT, Vec3::ZERO, &query);

    assert!(output.collided, "应检测到相机遮挡");
    assert!(
        output.distance < config.distance * 0.6,
        "遮挡时距离应缩短，实际 {}",
        output.distance
    );
    assert!(output.distance > 0.5, "不应把相机推到角色身上");
}

#[test]
fn camera_relative_move_uses_yaw() {
    let (mut camera, _) = test_camera();
    let forward = camera.camera_relative_move(Vec3::new(0.0, 0.0, 1.0));
    assert!((forward - Vec3::FORWARD).magnitude() < 1e-4);

    camera.set_yaw(std::f32::consts::FRAC_PI_2);
    // yaw=90° 时前向为 -X。
    let forward = camera.camera_relative_move(Vec3::new(0.0, 0.0, 1.0));
    assert!(
        (forward - Vec3::LEFT).magnitude() < 1e-4,
        "实际 {forward:?}"
    );
    let right = camera.camera_relative_move(Vec3::new(1.0, 0.0, 0.0));
    assert!((right - Vec3::FORWARD).magnitude() < 1e-4, "实际 {right:?}");
}

#[test]
fn lock_on_selects_front_candidate() {
    let config = LockOnConfig::default();
    let mut lock = LockOnState::new();
    let camera_position = Vec3::new(0.0, 1.5, 4.0);
    let camera_forward = Vec3::FORWARD;
    let candidates = [
        candidate(2, Vec3::new(0.0, 0.0, -2.0)),
        candidate(3, Vec3::new(0.0, 0.0, 8.0)),
    ];

    let selected = lock.select(camera_position, camera_forward, &candidates, &config, None);
    assert_eq!(selected, Some(CombatantId(2)));
    assert!(lock.is_locked());
    let aim = lock.aim_point().expect("应有瞄准点");
    assert!(
        (aim.y - (0.0 + config.aim_height)).abs() < 1e-4,
        "瞄准点应在目标上方"
    );
}

#[test]
fn lock_on_ignores_behind_and_out_of_range_candidates() {
    let config = LockOnConfig::default();
    let mut lock = LockOnState::new();

    let behind = [candidate(2, Vec3::new(0.0, 0.0, 8.0))];
    assert_eq!(
        lock.select(Vec3::ZERO, Vec3::FORWARD, &behind, &config, None),
        None,
        "背后的目标不应被锁定"
    );

    let too_far = [candidate(3, Vec3::new(0.0, 0.0, -40.0))];
    assert_eq!(
        lock.select(Vec3::ZERO, Vec3::FORWARD, &too_far, &config, None),
        None,
        "超出 max_distance 的目标不应被锁定"
    );
}

#[test]
fn lock_on_respects_visibility_query() {
    let config = LockOnConfig::default();
    let mut lock = LockOnState::new();
    let visibility = MockCameraQuery {
        visible: false,
        ..MockCameraQuery::default()
    };
    let candidates = [candidate(2, Vec3::new(0.0, 0.0, -3.0))];

    assert_eq!(
        lock.select(
            Vec3::ZERO,
            Vec3::FORWARD,
            &candidates,
            &config,
            Some(&visibility)
        ),
        None,
        "被遮挡目标不应被锁定"
    );
}

#[test]
fn lock_on_keeps_target_with_hysteresis() {
    let config = LockOnConfig::default();
    let mut lock = LockOnState::new();
    let candidates = [candidate(2, Vec3::new(0.0, 0.0, -3.0))];
    assert_eq!(
        lock.select(Vec3::ZERO, Vec3::FORWARD, &candidates, &config, None),
        Some(CombatantId(2))
    );

    // 相机已经转开、目标在身后，但仍在 break_distance 内：应保持锁定。
    assert_eq!(
        lock.select(Vec3::ZERO, Vec3::BACK, &candidates, &config, None),
        Some(CombatantId(2)),
        "已锁定目标在 break_distance 内不应被切换"
    );
}

#[test]
fn lock_on_breaks_when_target_out_of_range() {
    let config = LockOnConfig::default();
    let mut lock = LockOnState::new();
    let near = [candidate(2, Vec3::new(0.0, 0.0, -3.0))];
    lock.select(Vec3::ZERO, Vec3::FORWARD, &near, &config, None);
    assert!(lock.is_locked());

    let far = [candidate(2, Vec3::new(0.0, 0.0, -40.0))];
    let update = lock.update(Vec3::ZERO, &far, &config);
    assert!(update.broken, "超距应自动解除锁定");
    assert_eq!(update.target, None);
    assert!(!lock.is_locked());

    // 目标消失也应解除。
    let mut lock = LockOnState::new();
    lock.select(Vec3::ZERO, Vec3::FORWARD, &near, &config, None);
    let update = lock.update(Vec3::ZERO, &[], &config);
    assert!(update.broken);
    assert!(!lock.is_locked());
}

#[test]
fn camera_aims_at_locked_target() {
    let (mut camera, _) = test_camera();
    let query = MockCameraQuery::default();
    let anchor = Vec3::ZERO;
    camera.update(DT, anchor, &query);

    // 目标在正右方（+X），yaw 需要转向 -90°。
    let target = Vec3::new(5.0, 1.5, 0.0);
    let mut arrived = false;
    for _ in 0..180 {
        arrived = camera.aim_at_point(anchor, target, DT, 6.0);
        if arrived {
            break;
        }
    }
    assert!(arrived, "相机应能锁定转向目标");
    let forward = camera.forward();
    assert!(forward.x > 0.99, "转向后应面向 +X，实际 {forward:?}");
}
