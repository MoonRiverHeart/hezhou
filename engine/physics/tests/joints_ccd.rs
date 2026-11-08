//! 0.3 第二批测试：固定关节（相对位置锁定）、动态-动态 CCD、关节 API。

use hezhou_core::math::Vec3;
use hezhou_physics::{ColliderShape, FixedJoint, PhysicsWorld, RigidBodyType};

fn add_dynamic_box(world: &mut PhysicsWorld, center: Vec3, half: Vec3, mass: f32) -> (u64, u64) {
    let body = world.create_body();
    let collider = world.create_collider(ColliderShape::box_half_extents(half), body);
    let b = world.get_body_mut(body).unwrap();
    b.set_position(center);
    b.set_mass(mass);
    (body, collider)
}

fn add_dynamic_sphere(
    world: &mut PhysicsWorld,
    center: Vec3,
    radius: f32,
    mass: f32,
) -> (u64, u64) {
    let body = world.create_body();
    let collider = world.create_collider(ColliderShape::sphere(radius), body);
    let b = world.get_body_mut(body).unwrap();
    b.set_position(center);
    b.set_mass(mass);
    (body, collider)
}

#[test]
fn fixed_joint_keeps_relative_position() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::zero());

    let half = Vec3::new(0.5, 0.5, 0.5);
    let (body_a, _) = add_dynamic_box(&mut world, Vec3::new(-2.0, 0.0, 0.0), half, 1.0);
    let (body_b, _) = add_dynamic_box(&mut world, Vec3::new(0.0, 1.0, 0.0), half, 1.0);
    let joint_index = world.add_fixed_joint(body_a, body_b);
    assert_eq!(joint_index, 0);
    assert_eq!(world.fixed_joint_count(), 1);
    let initial_offset = world.fixed_joints()[0]
        .current_offset(&world)
        .expect("关节两端应存在");
    assert!(
        (initial_offset - Vec3::new(2.0, 1.0, 0.0)).magnitude() < 1e-5,
        "初始相对位置应为 (2,1,0)，实际 {:?}",
        initial_offset
    );

    // 给 A 一个速度，B 应像“焊死”一样跟随（相对平移锁定）。
    world
        .get_body_mut(body_a)
        .unwrap()
        .set_velocity(Vec3::new(2.0, 3.0, 0.0));

    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    let offset = world.fixed_joints()[0]
        .current_offset(&world)
        .expect("关节两端应存在");
    assert!(
        (offset - Vec3::new(2.0, 1.0, 0.0)).magnitude() < 0.05,
        "固定关节应保持相对位置 (2,1,0)，实际 {:?}",
        offset
    );

    // 相对速度应被消除（两刚体同速运动）。
    let a = world.get_body(body_a).unwrap();
    let b = world.get_body(body_b).unwrap();
    let relative_velocity = *b.velocity() - *a.velocity();
    assert!(
        relative_velocity.magnitude() < 0.1,
        "固定关节应消除相对速度，实际 {:?}",
        relative_velocity
    );
    // 确实整体移动了（不是被原地锁死）：两刚体共享平均速度，一起前进/上升。
    assert!(
        a.position().x > -1.0,
        "整体应向前移动，实际 x={}",
        a.position().x
    );
    assert!(
        b.position().y > 2.0,
        "整体应向上移动，实际 y={}",
        b.position().y
    );
}

#[test]
fn fixed_joint_holds_under_gravity() {
    let mut world = PhysicsWorld::new(); // 默认重力 -9.81

    let half = Vec3::new(0.5, 0.5, 0.5);
    let (body_a, _) = add_dynamic_box(&mut world, Vec3::new(0.0, 5.0, 0.0), half, 1.0);
    let (body_b, _) = add_dynamic_box(&mut world, Vec3::new(1.5, 5.0, 0.0), half, 1.0);
    world.add_fixed_joint(body_a, body_b);

    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    let offset = world.fixed_joints()[0]
        .current_offset(&world)
        .expect("关节两端应存在");
    assert!(
        (offset - Vec3::new(1.5, 0.0, 0.0)).magnitude() < 0.05,
        "自由落体时固定关节应保持相对位置，实际 {:?}",
        offset
    );
    assert!(
        world.get_body(body_a).unwrap().position().y < 0.0,
        "两刚体应一起下落"
    );
}

#[test]
fn fixed_joint_add_remove_api() {
    let mut world = PhysicsWorld::new();
    let half = Vec3::new(0.5, 0.5, 0.5);
    let (body_a, _) = add_dynamic_box(&mut world, Vec3::zero(), half, 1.0);
    let (body_b, _) = add_dynamic_box(&mut world, Vec3::new(2.0, 0.0, 0.0), half, 1.0);

    // 直接 push 一个自定义锚点/刚度的关节。
    let joint = FixedJoint::new(body_a, body_b, Vec3::new(2.0, 0.0, 0.0))
        .with_anchors(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
        .with_stiffness(0.5);
    world.push_fixed_joint(joint);
    assert_eq!(world.fixed_joint_count(), 1);
    assert!((world.fixed_joints()[0].stiffness - 0.5).abs() < 1e-6);

    let removed = world.remove_fixed_joint(0).expect("关节应存在");
    assert_eq!(removed.body_a, body_a);
    assert_eq!(world.fixed_joint_count(), 0);
    assert!(world.remove_fixed_joint(0).is_none());

    // clear_joints 同时清空距离关节与固定关节。
    world.add_fixed_joint(body_a, body_b);
    world.add_distance_joint(body_a, body_b, 2.0);
    assert_eq!(world.joint_count(), 2);
    world.clear_joints();
    assert_eq!(world.joint_count(), 0);
}

#[test]
fn dynamic_dynamic_ccd_prevents_tunneling() {
    fn simulate(ccd: bool) -> (f32, f32) {
        let mut world = PhysicsWorld::new();
        world.set_gravity(Vec3::zero());
        world.set_ccd_enabled(ccd);

        // 目标：静止动态球（半径 0.5，位于原点）。
        let (target, _) = add_dynamic_sphere(&mut world, Vec3::zero(), 0.5, 1.0);
        // 高速球：从 -10 以 1000 m/s 撞向目标（单步位移 16.67 米，远超半径）。
        let (fast, _) = add_dynamic_sphere(&mut world, Vec3::new(-10.0, 0.0, 0.0), 0.5, 1.0);
        {
            let b = world.get_body_mut(fast).unwrap();
            b.set_velocity(Vec3::new(1000.0, 0.0, 0.0));
            b.set_ccd_enabled(ccd);
        }

        for _ in 0..30 {
            world.step(1.0 / 60.0);
        }
        (
            world.get_body(fast).unwrap().position().x,
            world.get_body(target).unwrap().position().x,
        )
    }

    let (fast_without, target_without) = simulate(false);
    assert!(
        fast_without > target_without,
        "未启用 CCD 时高速球应穿过动态目标（fast={} target={}）",
        fast_without,
        target_without
    );

    let (fast_with, target_with) = simulate(true);
    assert!(
        fast_with < target_with,
        "启用动态-动态 CCD 后高速球不应越过目标（fast={} target={}）",
        fast_with,
        target_with
    );
    assert!(
        (target_with - fast_with) > 0.4,
        "两球应保持在接触距离附近，实际间距 {}",
        target_with - fast_with
    );
}

#[test]
fn dynamic_dynamic_ccd_head_on_keeps_order() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::zero());
    world.set_ccd_enabled(true);

    let (body_a, _) = add_dynamic_sphere(&mut world, Vec3::new(-10.0, 0.0, 0.0), 0.5, 1.0);
    let (body_b, _) = add_dynamic_sphere(&mut world, Vec3::new(10.0, 0.0, 0.0), 0.5, 1.0);
    {
        let a = world.get_body_mut(body_a).unwrap();
        a.set_velocity(Vec3::new(1000.0, 0.0, 0.0));
        a.set_ccd_enabled(true);
    }
    {
        let b = world.get_body_mut(body_b).unwrap();
        b.set_velocity(Vec3::new(-1000.0, 0.0, 0.0));
        b.set_ccd_enabled(true);
    }

    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }

    let a = world.get_body(body_a).unwrap().position().x;
    let b = world.get_body(body_b).unwrap().position().x;
    assert!(a < b, "对向高速球不应互相穿过（a={} b={}）", a, b);
    assert!(a.is_finite() && b.is_finite());
}
