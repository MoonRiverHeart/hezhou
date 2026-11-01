//! 角色运动测试：地面检测 / move_and_slide / 台阶 / 斜坡 / 传感器穿透。

use hezhou_core::math::Vec3;
use hezhou_physics::{
    layers, CharacterMotor, ColliderShape, FixedTimestep, PhysicsWorld, QueryFilter, RigidBodyType,
};

/// 地面：y=0 的水平无限平面。
fn add_floor(world: &mut PhysicsWorld) -> u64 {
    let body = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::plane(), body);
    body
}

fn add_static_box(world: &mut PhysicsWorld, center: Vec3, half: Vec3) -> u64 {
    let body = world.create_body_with_type(RigidBodyType::Static);
    let collider = world.create_collider(ColliderShape::box_half_extents(half), body);
    world.get_body_mut(body).unwrap().set_position(center);
    let c = world.get_collider_mut(collider).unwrap();
    c.set_layer(layers::WORLD);
    c.set_mask(layers::ALL);
    body
}

#[test]
fn motor_lands_on_plane_and_reports_grounded() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);
    let mut motor = CharacterMotor::new(0.0, 1.0, 0.0);

    for _ in 0..120 {
        motor.step(&world, 1.0 / 60.0, Vec3::zero(), false);
    }
    assert!(motor.is_grounded(), "角色应站在地面上");
    assert!(
        motor.position.y.abs() < 0.05,
        "脚底应贴近 y=0，实际 {}",
        motor.position.y
    );
}

#[test]
fn motor_ground_check_detects_floor() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);

    let mut motor = CharacterMotor::new(0.0, 0.0, 0.0);
    assert!(motor.ground_check(&world), "脚底贴地时应检测到地面");

    motor.teleport(Vec3::new(0.0, 1.0, 0.0));
    assert!(!motor.ground_check(&world), "离地 1 米时不应着地");
}

#[test]
fn motor_moves_horizontally_on_ground() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);
    let mut motor = CharacterMotor::new(0.0, 0.0, 0.0);
    motor.ground_check(&world);

    for _ in 0..60 {
        motor.step(&world, 1.0 / 60.0, Vec3::new(2.0, 0.0, 0.0), false);
    }
    assert!(motor.is_grounded());
    assert!(
        (motor.position.x - 2.0).abs() < 0.1,
        "1 秒应移动约 2 米，实际 {}",
        motor.position.x
    );
    assert!(motor.position.y.abs() < 0.05);
}

#[test]
fn motor_jumps_and_returns_to_ground() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);
    let mut motor = CharacterMotor::new(0.0, 0.0, 0.0);
    motor.ground_check(&world);

    let result = motor.step(&world, 1.0 / 60.0, Vec3::zero(), true);
    assert!(!result.grounded, "起跳当帧不应着地");
    assert!(motor.velocity.y > 0.0);

    let mut peak = motor.position.y;
    for _ in 0..180 {
        motor.step(&world, 1.0 / 60.0, Vec3::zero(), false);
        peak = peak.max(motor.position.y);
    }
    assert!(peak > 0.5, "跳跃最高点应明显离地，实际 {}", peak);
    assert!(motor.is_grounded(), "最终应落回地面");
    assert!(motor.position.y.abs() < 0.05);
}

#[test]
fn motor_move_and_slide_stops_at_wall() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);
    // 墙：z ∈ [1.5, 2.5]，高 2 米。
    add_static_box(
        &mut world,
        Vec3::new(0.0, 1.0, 2.0),
        Vec3::new(2.0, 1.0, 0.5),
    );

    let mut motor = CharacterMotor::new(0.0, 0.0, -1.0);
    let result = motor.move_and_slide(&world, Vec3::new(0.0, 0.0, 4.0));

    assert!(result.touched_wall(), "应撞到墙面");
    // 胶囊半径 0.3 + 皮肤 0.02 → 停在 z ≈ 1.18
    assert!(
        (motor.position.z - 1.18).abs() < 0.05,
        "应被墙挡住，实际 z={}",
        motor.position.z
    );
    assert!(result.moved.z > 2.0 && result.moved.z < 2.3);
}

#[test]
fn motor_move_and_slide_slides_along_wall() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);
    add_static_box(
        &mut world,
        Vec3::new(0.0, 1.0, 2.0),
        Vec3::new(2.0, 1.0, 0.5),
    );

    let mut motor = CharacterMotor::new(0.0, 0.0, -1.0);
    // 斜向撞墙：Z 分量被挡住，X 分量应沿墙面滑动。
    motor.move_and_slide(&world, Vec3::new(2.0, 0.0, 4.0));

    assert!(
        (motor.position.z - 1.18).abs() < 0.06,
        "z={}",
        motor.position.z
    );
    assert!(
        motor.position.x > 1.2,
        "应沿墙滑行，实际 x={}",
        motor.position.x
    );
}

#[test]
fn motor_climbs_small_step() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);
    // 台阶：高 0.32 米（略高于胶囊半径 0.3，需要台阶抬升逻辑），z ∈ [1, 3]。
    add_static_box(
        &mut world,
        Vec3::new(0.0, 0.16, 2.0),
        Vec3::new(2.0, 0.16, 1.0),
    );

    let mut motor = CharacterMotor::new(0.0, 0.0, -0.5);
    let result = motor.move_and_slide(&world, Vec3::new(0.0, 0.0, 3.0));

    assert!(result.stepped, "应触发台阶抬升");
    assert!(
        motor.position.y > 0.3 && motor.position.y < 0.4,
        "应站上台阶顶面，实际 y={}",
        motor.position.y
    );
}

#[test]
fn motor_cannot_climb_tall_wall() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);
    // 高墙：高 1.0 米，远超 step_height 0.35。
    add_static_box(
        &mut world,
        Vec3::new(0.0, 0.5, 2.0),
        Vec3::new(2.0, 0.5, 0.5),
    );

    let mut motor = CharacterMotor::new(0.0, 0.0, -1.0);
    let result = motor.move_and_slide(&world, Vec3::new(0.0, 0.0, 3.0));

    assert!(!result.stepped, "高墙不应触发台阶抬升");
    assert!(
        motor.position.y.abs() < 0.05,
        "不应被抬升，y={}",
        motor.position.y
    );
    assert!(
        motor.position.z < 1.25,
        "应被高墙挡住，z={}",
        motor.position.z
    );
}

#[test]
fn motor_grounded_on_walkable_slope() {
    let mut world = PhysicsWorld::new();
    let plane_body = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::plane(), plane_body);
    // 绕 Z 轴倾斜 30°：法线 (-sin30, cos30, 0)，属于可行走坡度。
    world
        .get_body_mut(plane_body)
        .unwrap()
        .set_rotation_euler(0.0, 0.0, 30.0f32.to_radians());

    let mut motor = CharacterMotor::new(0.0, 2.0, 0.0);
    for _ in 0..120 {
        motor.step(&world, 1.0 / 60.0, Vec3::zero(), false);
    }
    assert!(motor.is_grounded(), "30° 斜坡应可站立");
    assert!(
        motor.ground_normal.y > 0.8,
        "地面法线应接近竖直，实际 {:?}",
        motor.ground_normal
    );
}

#[test]
fn motor_slides_down_steep_slope_without_grounding() {
    let mut world = PhysicsWorld::new();
    let plane_body = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::plane(), plane_body);
    // 60° 斜坡：cos60 = 0.5 < cos50 ≈ 0.64 → 不可行走。
    world
        .get_body_mut(plane_body)
        .unwrap()
        .set_rotation_euler(0.0, 0.0, 60.0f32.to_radians());

    let mut motor = CharacterMotor::new(0.0, 2.0, 0.0);
    let mut touched_slope = false;
    for _ in 0..120 {
        let result = motor.step(&world, 1.0 / 60.0, Vec3::zero(), false);
        if result.hits.iter().any(|hit| hit.normal().y > 0.3) {
            touched_slope = true;
        }
    }
    assert!(touched_slope, "应碰到斜坡");
    assert!(!motor.is_grounded(), "60° 陡坡不应判定为着地");
}

#[test]
fn motor_passes_through_sensor_but_is_blocked_by_solid() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);

    // 传感器墙（trigger）：z ∈ [1.5, 2.5]
    let sensor_body = world.create_body_with_type(RigidBodyType::Static);
    let sensor_collider = world.create_collider(
        ColliderShape::box_half_extents(Vec3::new(2.0, 1.0, 0.5)),
        sensor_body,
    );
    world
        .get_body_mut(sensor_body)
        .unwrap()
        .set_position(Vec3::new(0.0, 1.0, 2.0));
    let sensor = world.get_collider_mut(sensor_collider).unwrap();
    sensor.set_layer(layers::TRIGGER);
    sensor.set_sensor(true);

    let mut motor = CharacterMotor::new(0.0, 0.0, -0.5);
    motor.move_and_slide(&world, Vec3::new(0.0, 0.0, 3.0));
    assert!(
        (motor.position.z - 2.5).abs() < 0.05,
        "应穿过传感器，实际 z={}",
        motor.position.z
    );

    // 同样的位置换成实体墙后应被挡住。
    let mut world2 = PhysicsWorld::new();
    add_floor(&mut world2);
    add_static_box(
        &mut world2,
        Vec3::new(0.0, 1.0, 2.0),
        Vec3::new(2.0, 1.0, 0.5),
    );
    let mut motor2 = CharacterMotor::new(0.0, 0.0, -0.5);
    motor2.move_and_slide(&world2, Vec3::new(0.0, 0.0, 3.0));
    assert!(
        (motor2.position.z - 1.18).abs() < 0.05,
        "实体墙应挡住角色，实际 z={}",
        motor2.position.z
    );
}

#[test]
fn motor_depenetrates_when_spawned_inside_wall() {
    let mut world = PhysicsWorld::new();
    add_static_box(
        &mut world,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 1.0),
    );

    let mut motor = CharacterMotor::new(0.0, 0.0, 0.0);
    let result = motor.move_and_slide(&world, Vec3::zero());

    assert!(result.depenetrated, "初始穿透应被纠正");
    assert!(
        result.moved.magnitude() > 0.1,
        "应被推出墙体，moved={:?}",
        result.moved
    );
    assert!(
        world
            .resolve_capsule_penetration(
                motor.capsule_center(),
                motor.radius,
                motor.half_height,
                hezhou_physics::QueryFilter::new()
            )
            .is_none(),
        "纠正后不应再穿透"
    );
}

#[test]
fn motor_step_is_stable_under_fixed_timestep_across_frame_rates() {
    // 同一段逻辑用固定步长驱动：不同渲染帧率下角色位移应一致。
    fn simulate(frames_per_second: u32) -> Vec3 {
        let mut world = PhysicsWorld::new();
        add_floor(&mut world);
        let mut motor = CharacterMotor::new(0.0, 0.5, 0.0);
        let mut timestep = FixedTimestep::new(60.0);
        let frame_dt = 1.0 / frames_per_second as f32;
        for _ in 0..frames_per_second {
            let steps = timestep.advance(frame_dt);
            for _ in 0..steps {
                motor.step(&world, timestep.step(), Vec3::new(3.0, 0.0, 0.0), false);
            }
        }
        motor.position
    }

    let at_60 = simulate(60);
    let at_144 = simulate(144);
    assert!(
        (at_60.x - at_144.x).abs() < 0.05,
        "固定步下位移应一致：60Hz={} vs 144Hz={}",
        at_60.x,
        at_144.x
    );
    assert!((at_60.y - at_144.y).abs() < 0.05);
}

#[test]
fn world_helpers_ground_probe_and_move_capsule() {
    let mut world = PhysicsWorld::new();
    add_floor(&mut world);

    // 脚底在 0.5 米高处向下探测：应命中地面，距离约 0.5。
    let probe = world
        .ground_probe(Vec3::new(0.0, 0.5, 0.0), 0.3, 0.6, 2.0, QueryFilter::new())
        .expect("地面探测应命中");
    assert!(
        (probe.distance() - 0.5).abs() < 0.02,
        "distance={}",
        probe.distance()
    );
    assert!((probe.normal().y - 1.0).abs() < 1e-2);

    // 地面超出探测距离：返回 None。
    assert!(world
        .ground_probe(Vec3::new(0.0, 1.0, 0.0), 0.3, 0.6, 0.7, QueryFilter::new(),)
        .is_none());

    // 无状态胶囊移动：脚底 (0,0,0) 沿 +X 移动 1 米。
    let result = world.move_capsule(
        Vec3::zero(),
        Vec3::new(1.0, 0.0, 0.0),
        0.3,
        0.6,
        QueryFilter::new(),
    );
    assert!(
        (result.position.x - 1.0).abs() < 0.05,
        "position={:?}",
        result.position
    );
}
