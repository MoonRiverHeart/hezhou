//! 0.3 第三批测试：空间索引宽相一致性、旋转盒保守接触、200 体性能对比。

use std::time::{Duration, Instant};

use hezhou_core::math::Vec3;
use hezhou_physics::{ColliderShape, PhysicsWorld, RigidBodyType};

fn add_static_box(world: &mut PhysicsWorld, center: Vec3, half: Vec3) -> u64 {
    let body = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::box_half_extents(half), body);
    world.get_body_mut(body).unwrap().set_position(center);
    body
}

fn add_dynamic_box(
    world: &mut PhysicsWorld,
    center: Vec3,
    half: Vec3,
    rotation: Option<(f32, f32, f32)>,
) -> u64 {
    let body = world.create_body();
    world.create_collider(ColliderShape::box_half_extents(half), body);
    let b = world.get_body_mut(body).unwrap();
    b.set_position(center);
    if let Some((x, y, z)) = rotation {
        b.set_rotation_euler(x, y, z);
    }
    body
}

fn lcg(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 40) as f32) / ((1u32 << 24) as f32)
}

/// 构造同一确定性场景并跑 `steps` 步，返回位置/事件计数/宽相候选对。
fn simulate(spatial: bool, steps: usize) -> (Vec<Vec3>, usize, usize, usize) {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::new(0.0, -9.81, 0.0));
    world.set_spatial_index_enabled(spatial);

    // 地面（很大的静态盒，会走“大物体”分支）。
    add_static_box(
        &mut world,
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(20.0, 0.5, 20.0),
    );

    // 几个传感器，覆盖触发器事件路径。
    for (index, position) in [
        Vec3::new(4.0, 2.0, 0.0),
        Vec3::new(-6.0, 3.0, 4.0),
        Vec3::new(8.0, 1.5, -8.0),
    ]
    .iter()
    .enumerate()
    {
        let body = world.create_body_with_type(RigidBodyType::Static);
        let collider = world.create_collider(
            ColliderShape::box_half_extents(Vec3::new(2.0, 2.0, 2.0)),
            body,
        );
        world.get_body_mut(body).unwrap().set_position(*position);
        let c = world.get_collider_mut(collider).unwrap();
        c.set_sensor(true);
        if index == 0 {
            c.set_layer(hezhou_physics::layers::TRIGGER);
        }
    }

    // 40 个动态盒（部分带旋转），确定性伪随机摆放。
    let mut seed = 42u64;
    let mut bodies = Vec::new();
    for index in 0..40 {
        let x = -18.0 + lcg(&mut seed) * 36.0;
        let z = -18.0 + lcg(&mut seed) * 36.0;
        let y = 1.0 + lcg(&mut seed) * 4.0;
        let rotation = if index % 5 == 0 {
            Some((0.0, lcg(&mut seed) * std::f32::consts::TAU, 0.0))
        } else {
            None
        };
        bodies.push(add_dynamic_box(
            &mut world,
            Vec3::new(x, y, z),
            Vec3::new(0.5, 0.5, 0.5),
            rotation,
        ));
    }

    let mut collision_total = 0usize;
    let mut trigger_total = 0usize;
    for _ in 0..steps {
        world.step(1.0 / 60.0);
        collision_total += world.collision_events().len();
        trigger_total += world.trigger_events().len();
    }

    let positions = bodies
        .iter()
        .map(|body| *world.get_body(*body).unwrap().position())
        .collect();
    let stats = world.broadphase_stats();
    (
        positions,
        collision_total,
        trigger_total,
        stats.candidate_pairs,
    )
}

#[test]
fn spatial_index_matches_brute_force() {
    let steps = 90;
    let (brute_positions, brute_collisions, brute_triggers, brute_pairs) = simulate(false, steps);
    let (spatial_positions, spatial_collisions, spatial_triggers, spatial_pairs) =
        simulate(true, steps);

    // 结果一致：位置、碰撞/触发器事件计数。
    assert_eq!(
        brute_collisions, spatial_collisions,
        "空间索引与 brute force 的碰撞事件数应一致"
    );
    assert_eq!(
        brute_triggers, spatial_triggers,
        "空间索引与 brute force 的触发器事件数应一致"
    );
    assert_eq!(brute_positions.len(), spatial_positions.len());
    for (index, (a, b)) in brute_positions
        .iter()
        .zip(spatial_positions.iter())
        .enumerate()
    {
        assert!(
            (*a - *b).magnitude() < 1e-5,
            "第 {} 个刚体位置不一致：brute={:?} spatial={:?}",
            index,
            a,
            b
        );
    }

    // 候选对：brute force = n*(n-1)/2（44 个碰撞体 = 40 动态 + 3 传感器 + 1 地面）；
    // 空间索引应显著更少。
    const BODY_COUNT: usize = 44;
    assert_eq!(brute_pairs, BODY_COUNT * (BODY_COUNT - 1) / 2);
    assert!(
        spatial_pairs < brute_pairs / 2,
        "空间索引候选对应显著少于 brute force：spatial={} brute={}",
        spatial_pairs,
        brute_pairs
    );
}

#[test]
fn rotated_box_conservative_contact() {
    // 静态盒绕 Y 轴 45°：保守 AABB 半尺寸 = (√2, 1, √2)。
    let make_world = |dynamic_position: Vec3| {
        let mut world = PhysicsWorld::new();
        world.set_gravity(Vec3::zero());
        let static_body = world.create_body_with_type(RigidBodyType::Static);
        world.create_collider(
            ColliderShape::box_half_extents(Vec3::new(1.0, 1.0, 1.0)),
            static_body,
        );
        world
            .get_body_mut(static_body)
            .unwrap()
            .set_rotation_euler(0.0, 45.0f32.to_radians(), 0.0);
        let dynamic_body =
            add_dynamic_box(&mut world, dynamic_position, Vec3::new(0.2, 0.2, 0.2), None);
        (world, dynamic_body)
    };

    // (1.3, 0, 0.6) 在保守 AABB 内、真实 OBB 外（局部坐标约 (0.49, 0, 1.34)）。
    let (mut world, dynamic_body) = make_world(Vec3::new(1.3, 0.0, 0.6));
    world.step(1.0 / 60.0);
    assert!(
        !world.collision_events().is_empty(),
        "保守 AABB 应检测到旋转盒接触"
    );
    let pushed = *world.get_body(dynamic_body).unwrap().position();
    assert!(
        pushed.x > 1.4,
        "动态盒应被沿 +X 推出保守 AABB，实际 {:?}",
        pushed
    );

    // 保守 AABB 之外 → 无接触。
    let (mut world, dynamic_body) = make_world(Vec3::new(2.0, 0.0, 0.0));
    world.step(1.0 / 60.0);
    assert!(
        world.collision_events().is_empty(),
        "保守 AABB 之外的动态盒不应产生接触"
    );
    let stayed = *world.get_body(dynamic_body).unwrap().position();
    assert!(
        (stayed - Vec3::new(2.0, 0.0, 0.0)).magnitude() < 1e-5,
        "无接触时不应被推动，实际 {:?}",
        stayed
    );
}

#[test]
fn rotated_box_lands_on_conservative_aabb() {
    let mut world = PhysicsWorld::new();
    // 地面顶面 y=0。
    add_static_box(
        &mut world,
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(10.0, 0.5, 10.0),
    );

    let body = add_dynamic_box(
        &mut world,
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
        // 绕 Z 轴 45°：世界 AABB 半高 = |sin|*0.5 + |cos|*0.5 ≈ 0.707。
        Some((0.0, 0.0, 45.0f32.to_radians())),
    );

    for _ in 0..180 {
        world.step(1.0 / 60.0);
    }

    let y = world.get_body(body).unwrap().position().y;
    assert!(
        (0.68..0.75).contains(&y),
        "旋转 45° 的盒应停在保守 AABB 高度 ≈0.707，实际 {}",
        y
    );
}

#[test]
fn spatial_index_200_bodies_performance() {
    fn build_and_run(spatial: bool) -> (Duration, usize, Vec<Vec3>) {
        let mut world = PhysicsWorld::new();
        world.set_gravity(Vec3::new(0.0, -9.81, 0.0));
        world.set_spatial_index_enabled(spatial);
        // 200x200 地面（大物体分支）。
        add_static_box(
            &mut world,
            Vec3::new(0.0, -0.5, 0.0),
            Vec3::new(100.0, 0.5, 100.0),
        );

        let mut bodies = Vec::new();
        for index in 0..200 {
            let x = (index % 20) as f32 * 4.0 - 38.0;
            let z = (index / 20) as f32 * 4.0 - 18.0;
            let y = 2.0 + ((index * 7) % 13) as f32 * 0.3;
            bodies.push(add_dynamic_box(
                &mut world,
                Vec3::new(x, y, z),
                Vec3::new(0.5, 0.5, 0.5),
                None,
            ));
        }

        let start = Instant::now();
        for _ in 0..60 {
            world.step(1.0 / 60.0);
        }
        let elapsed = start.elapsed();

        let positions = bodies
            .iter()
            .map(|body| *world.get_body(*body).unwrap().position())
            .collect();
        (elapsed, world.broadphase_stats().candidate_pairs, positions)
    }

    let (brute_time, brute_pairs, brute_positions) = build_and_run(false);
    let (spatial_time, spatial_pairs, spatial_positions) = build_and_run(true);

    println!(
        "[broadphase perf] 200 动态刚体 + 1 地面, 60 steps (debug build): brute_force={:?} candidate_pairs={}, spatial_index={:?} candidate_pairs={}",
        brute_time, brute_pairs, spatial_time, spatial_pairs
    );

    // 宽相候选对：brute force = 201*200/2；空间索引应显著更少。
    assert_eq!(brute_pairs, 201 * 200 / 2);
    assert!(
        spatial_pairs < brute_pairs / 4,
        "200 体场景空间索引候选对应显著少于 brute force：spatial={} brute={}",
        spatial_pairs,
        brute_pairs
    );

    // 结果一致（宽相只影响候选对，不应改变物理结果）。
    for (index, (a, b)) in brute_positions
        .iter()
        .zip(spatial_positions.iter())
        .enumerate()
    {
        assert!(
            (*a - *b).magnitude() < 1e-4,
            "第 {} 个刚体位置不一致：brute={:?} spatial={:?}",
            index,
            a,
            b
        );
    }
}
