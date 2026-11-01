//! 物理查询测试：raycast / sphere cast / capsule sweep / overlap / 层掩码 /
//! 触发器 / 命中盒 / 调试描述 / 固定时间步。

use hezhou_core::math::Vec3;
use hezhou_physics::{
    layers, ColliderShape, DebugShape, FixedTimestep, Hitbox, PhysicsWorld, QueryFilter, Ray,
    RigidBodyType, TriggerEventKind,
};

fn add_box(
    world: &mut PhysicsWorld,
    body_type: RigidBodyType,
    center: Vec3,
    half: Vec3,
    layer: u32,
    mask: u32,
    sensor: bool,
) -> (u64, u64) {
    let body = world.create_body_with_type(body_type);
    let collider = world.create_collider(ColliderShape::box_half_extents(half), body);
    world.get_body_mut(body).unwrap().set_position(center);
    let c = world.get_collider_mut(collider).unwrap();
    c.set_layer(layer);
    c.set_mask(mask);
    c.set_sensor(sensor);
    (body, collider)
}

#[test]
fn raycast_hits_static_box_with_full_hit_info() {
    let mut world = PhysicsWorld::new();
    let (body, collider) = add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::zero(),
        Vec3::new(1.0, 1.0, 1.0),
        layers::WORLD,
        layers::ALL,
        false,
    );

    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    let hit = world.raycast(&ray).expect("raycast should hit the box");
    assert!(hit.is_hit());
    assert!(
        (hit.point().z - 1.0).abs() < 1e-3,
        "point={:?}",
        hit.point()
    );
    assert!(
        (hit.normal().z - 1.0).abs() < 1e-3,
        "normal={:?}",
        hit.normal()
    );
    assert!(
        (hit.distance() - 4.0).abs() < 1e-3,
        "distance={}",
        hit.distance()
    );
    assert_eq!(hit.body_id(), Some(body));
    assert_eq!(hit.collider_id(), Some(collider));
}

#[test]
fn raycast_respects_max_distance_and_layer_mask() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::zero(),
        Vec3::new(1.0, 1.0, 1.0),
        layers::WORLD,
        layers::ALL,
        false,
    );

    let short = Ray::with_max_distance(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0), 3.0);
    assert!(
        world.raycast(&short).is_none(),
        "超过 max_distance 应未命中"
    );

    let long = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(
        world
            .raycast_filtered(&long, QueryFilter::new().with_layer_mask(layers::PLAYER))
            .is_none(),
        "层掩码不包含 WORLD 时应未命中"
    );
    assert!(
        world
            .raycast_filtered(&long, QueryFilter::new().with_layer_mask(layers::WORLD))
            .is_some(),
        "层掩码包含 WORLD 时应命中"
    );
}

#[test]
fn raycast_hits_plane_and_capsule() {
    let mut world = PhysicsWorld::new();

    // 水平无限平面（默认旋转，法线 +Y）
    let plane_body = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::plane(), plane_body);

    let down = Ray::new(Vec3::new(0.0, 2.0, 0.0), Vec3::down());
    let plane_hit = world.raycast(&down).expect("应命中平面");
    assert!((plane_hit.distance() - 2.0).abs() < 1e-3);
    assert!((plane_hit.normal().y - 1.0).abs() < 1e-3);
    assert!((plane_hit.point().y).abs() < 1e-3);

    // 竖直胶囊
    let capsule_body = world.create_body_with_type(RigidBodyType::Static);
    let capsule_collider = world.create_collider(ColliderShape::capsule(1.0, 0.5), capsule_body);
    world
        .get_body_mut(capsule_body)
        .unwrap()
        .set_position(Vec3::new(0.0, 3.0, 0.0));

    let horizontal = Ray::new(Vec3::new(0.0, 3.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    let capsule_hit = world.raycast(&horizontal).expect("应命中胶囊");
    assert!((capsule_hit.point().z - 0.5).abs() < 1e-3);
    assert_eq!(capsule_hit.collider_id(), Some(capsule_collider));
}

#[test]
fn raycast_all_is_sorted_by_distance() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::new(1.0, 1.0, 0.2),
        layers::WORLD,
        layers::ALL,
        false,
    );
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::new(0.0, 0.0, 6.0),
        Vec3::new(1.0, 1.0, 0.2),
        layers::WORLD,
        layers::ALL,
        false,
    );

    let ray = Ray::new(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, -1.0));
    let hits = world.raycast_all(&ray, QueryFilter::new());
    assert_eq!(hits.len(), 2);
    assert!(hits[0].distance() < hits[1].distance());
    assert!((hits[0].distance() - 3.8).abs() < 1e-3);
    assert!((hits[1].distance() - 6.8).abs() < 1e-3);
}

#[test]
fn capsule_sweep_hits_static_box() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::zero(),
        Vec3::new(1.0, 1.0, 1.0),
        layers::WORLD,
        layers::ALL,
        false,
    );

    // 胶囊中点 (0, 0, 5)，圆柱段半高 0.6，半径 0.3；沿 -Z 扫掠 10 米。
    let hit = world
        .sweep_capsule(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -10.0),
            0.3,
            0.6,
            QueryFilter::new(),
        )
        .expect("胶囊扫掠应命中");
    assert!(
        (hit.distance() - 3.7).abs() < 1e-2,
        "distance={}",
        hit.distance()
    );
    assert!((hit.normal().z - 1.0).abs() < 1e-2);
    assert!((hit.point().z - 1.0).abs() < 1e-2);
}

#[test]
fn capsule_sweep_misses_when_moving_away() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::zero(),
        Vec3::new(1.0, 1.0, 1.0),
        layers::WORLD,
        layers::ALL,
        false,
    );

    let away = world.sweep_capsule(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, 10.0),
        0.3,
        0.6,
        QueryFilter::new(),
    );
    assert!(away.is_none(), "朝反方向扫掠不应命中");
}

#[test]
fn sphere_cast_detects_wall() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(2.0, 2.0, 0.5),
        layers::WORLD,
        layers::ALL,
        false,
    );

    // 相机碰撞：半径 0.3 的球从原点向 +Z 扫 10 米，墙面在 z=4.5。
    let hit = world
        .sphere_cast(
            Vec3::zero(),
            0.3,
            Vec3::new(0.0, 0.0, 1.0),
            10.0,
            QueryFilter::new(),
        )
        .expect("sphere cast 应命中墙面");
    assert!(
        (hit.distance() - 4.2).abs() < 1e-2,
        "distance={}",
        hit.distance()
    );
    assert!((hit.normal().z + 1.0).abs() < 1e-2);
}

#[test]
fn overlap_sphere_reports_depth_and_ids() {
    let mut world = PhysicsWorld::new();
    let target_body = world.create_body_with_type(RigidBodyType::Static);
    let target_collider = world.create_collider(ColliderShape::sphere(0.5), target_body);
    world
        .get_body_mut(target_body)
        .unwrap()
        .set_position(Vec3::new(0.0, 0.0, 1.4));

    let hits = world.overlap_sphere(Vec3::zero(), 1.0, QueryFilter::new());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].body_id(), target_body);
    assert_eq!(hits[0].collider_id(), target_collider);
    // 中心距 1.4，半径和 1.5 → 穿透 0.1
    assert!(
        (hits[0].depth() - 0.1).abs() < 1e-2,
        "depth={}",
        hits[0].depth()
    );
    // 法线从目标球指向查询球（-Z 方向）
    assert!((hits[0].normal().z + 1.0).abs() < 1e-2);
}

#[test]
fn hitbox_box_overlaps_enemy_capsule_with_mask() {
    let mut world = PhysicsWorld::new();
    let enemy_body = world.create_body_with_type(RigidBodyType::Static);
    let enemy_collider = world.create_collider(ColliderShape::capsule(0.6, 0.3), enemy_body);
    world
        .get_body_mut(enemy_body)
        .unwrap()
        .set_position(Vec3::new(0.0, 0.9, 3.0));
    let enemy = world.get_collider_mut(enemy_collider).unwrap();
    enemy.set_layer(layers::ENEMY);
    enemy.set_mask(layers::ALL);

    // 玩家在原点，攻击盒中心 (0, 0.9, 2.0)，z 半尺寸 1.0 → 覆盖到 z=3.0。
    let hitbox = Hitbox::box_half_extents(Vec3::new(0.5, 0.6, 1.0))
        .with_offset(Vec3::new(0.0, 0.9, 2.0))
        .with_layers(layers::PLAYER, layers::ENEMY);

    let player_origin = Vec3::zero();
    let hits = hitbox.overlaps(
        &world,
        player_origin,
        QueryFilter::new().with_layer_mask(layers::PLAYER | layers::ENEMY),
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].body_id(), enemy_body);

    // 命中盒 mask 不含 WORLD 时不应命中世界层目标
    let world_mask_hitbox = hitbox.with_layers(layers::PLAYER, layers::WORLD);
    let masked = world_mask_hitbox.overlaps(&world, player_origin, QueryFilter::new());
    assert!(masked.is_empty(), "命中盒 mask 不匹配时不应命中");
}

#[test]
fn layer_mask_blocks_sweep() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::zero(),
        Vec3::new(1.0, 1.0, 1.0),
        layers::WORLD,
        layers::ALL,
        false,
    );

    let center = Vec3::new(0.0, 0.0, 5.0);
    let displacement = Vec3::new(0.0, 0.0, -10.0);
    let blocked = world.sweep_capsule(
        center,
        displacement,
        0.3,
        0.6,
        QueryFilter::new().with_layer_mask(layers::PLAYER),
    );
    assert!(blocked.is_none(), "层掩码不含 WORLD 时扫掠应忽略该碰撞体");

    let allowed = world.sweep_capsule(
        center,
        displacement,
        0.3,
        0.6,
        QueryFilter::new().with_layer_mask(layers::WORLD),
    );
    assert!(allowed.is_some());
}

#[test]
fn trigger_enter_and_exit_events() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::zero());

    // 触发器区域：中心原点，半尺寸 1。
    let (trigger_body, trigger_collider) = add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::zero(),
        Vec3::new(1.0, 1.0, 1.0),
        layers::TRIGGER,
        layers::ALL,
        true,
    );

    // 动态物体（无重力，保持不动）。
    let (other_body, other_collider) = add_box(
        &mut world,
        RigidBodyType::Dynamic,
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.5, 0.5, 0.5),
        layers::DEFAULT,
        layers::ALL,
        false,
    );

    world.step(1.0 / 60.0);
    assert!(world.trigger_events().is_empty());

    // 进入触发器。
    world
        .get_body_mut(other_body)
        .unwrap()
        .set_position(Vec3::new(0.0, 0.0, 0.5));
    world.step(1.0 / 60.0);
    let enter = world.trigger_events();
    assert_eq!(enter.len(), 1);
    assert_eq!(enter[0].kind, TriggerEventKind::Enter);
    assert_eq!(enter[0].trigger_body, trigger_body);
    assert_eq!(enter[0].trigger_collider, trigger_collider);
    assert_eq!(enter[0].other_body, other_body);
    assert_eq!(enter[0].other_collider, other_collider);

    // 传感器不参与位置分离：动态物体位置保持。
    let pos = *world.get_body(other_body).unwrap().position();
    assert!((pos.z - 0.5).abs() < 1e-5, "sensor 不应推开动态刚体");

    // 静止重叠不重复触发 Enter。
    world.step(1.0 / 60.0);
    assert!(world.trigger_events().is_empty());

    // 离开触发器。
    world
        .get_body_mut(other_body)
        .unwrap()
        .set_position(Vec3::new(0.0, 0.0, 8.0));
    world.step(1.0 / 60.0);
    let exit = world.trigger_events();
    assert_eq!(exit.len(), 1);
    assert_eq!(exit[0].kind, TriggerEventKind::Exit);
    assert_eq!(exit[0].trigger_body, trigger_body);
    assert_eq!(exit[0].other_body, other_body);
}

#[test]
fn sensors_are_ignored_by_default_queries() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::zero(),
        Vec3::new(1.0, 1.0, 1.0),
        layers::TRIGGER,
        layers::ALL,
        true,
    );

    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(world.raycast(&ray).is_none(), "默认应忽略 sensor");
    assert!(
        world
            .raycast_filtered(&ray, QueryFilter::new().with_sensors(true))
            .is_some(),
        "显式包含 sensor 时应命中"
    );
}

#[test]
fn debug_shapes_describe_colliders_and_hitboxes() {
    let mut world = PhysicsWorld::new();
    add_box(
        &mut world,
        RigidBodyType::Static,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.5, 2.0),
        layers::WORLD,
        layers::ALL,
        false,
    );
    let capsule_body = world.create_body_with_type(RigidBodyType::Dynamic);
    world.create_collider(ColliderShape::capsule(0.6, 0.3), capsule_body);
    world
        .get_body_mut(capsule_body)
        .unwrap()
        .set_position(Vec3::new(2.0, 0.0, 0.0));

    let shapes = world.debug_shapes();
    assert_eq!(shapes.len(), 2);

    let box_shape = shapes
        .iter()
        .find(|shape| matches!(shape, DebugShape::Box { .. }))
        .expect("应包含盒调试描述");
    assert_eq!(box_shape.line_segments().len(), 12, "盒应为 12 条棱边");

    let capsule_shape = shapes
        .iter()
        .find(|shape| matches!(shape, DebugShape::Capsule { .. }))
        .expect("应包含胶囊调试描述");
    assert!(!capsule_shape.line_segments().is_empty());

    // 命中盒调试描述
    let hitbox = Hitbox::capsule(0.6, 0.3).with_offset(Vec3::new(0.0, 0.9, 1.5));
    let hitbox_shape = hitbox.debug_shape(Vec3::zero());
    assert!(matches!(hitbox_shape, DebugShape::Capsule { .. }));
    assert!(!hitbox_shape.line_segments().is_empty());
    match hitbox_shape {
        DebugShape::Capsule { a, b, radius, .. } => {
            assert!((radius - 0.3).abs() < 1e-5);
            assert!((b.y - a.y - 1.2).abs() < 1e-5);
        }
        _ => unreachable!(),
    }
}

#[test]
fn fixed_timestep_accumulates_consistent_steps() {
    // 60 FPS 帧率：1 秒应产生 60 个固定步。
    let mut timestep = FixedTimestep::new(60.0);
    let mut steps = 0;
    for _ in 0..60 {
        steps += timestep.advance(1.0 / 60.0);
    }
    assert_eq!(steps, 60);
    assert!(timestep.alpha() >= 0.0 && timestep.alpha() <= 1.0);

    // 120 FPS 帧率：1 秒同样应产生 60 个固定步（帧率无关）。
    let mut timestep = FixedTimestep::new(60.0);
    let mut steps = 0;
    for _ in 0..120 {
        steps += timestep.advance(1.0 / 120.0);
    }
    assert_eq!(steps, 60);

    // 大 dt 被 max_steps 限制，不产生螺旋。
    let mut timestep = FixedTimestep::new(60.0);
    let steps = timestep.advance(1.0);
    assert_eq!(steps, FixedTimestep::DEFAULT_MAX_STEPS);
    assert!(timestep.accumulator() < timestep.step());
}

#[test]
fn fixed_timestep_world_advance_matches_between_frame_rates() {
    fn simulate(frames: u32) -> Vec3 {
        let mut world = PhysicsWorld::new();
        world.set_gravity(Vec3::new(0.0, -10.0, 0.0));
        let body = world.create_body();
        let mut timestep = FixedTimestep::new(60.0);
        let frame_dt = 1.0 / frames as f32;
        for _ in 0..frames {
            world.advance_fixed(&mut timestep, frame_dt);
        }
        *world.get_body(body).unwrap().position()
    }

    let at_60 = simulate(60);
    let at_120 = simulate(120);
    assert!(
        (at_60.y - at_120.y).abs() < 1e-4,
        "不同帧率下固定步结果应一致：{:?} vs {:?}",
        at_60,
        at_120
    );
}
