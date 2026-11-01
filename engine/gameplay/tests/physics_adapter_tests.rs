//! physics 适配层集成测试（需要 `--features physics`）。
//!
//! 这些测试验证 gameplay 的 4 个查询 trait 能真正接到 `hezhou-physics` 定稿 API 上；
//! 默认 `cargo test -p hezhou-gameplay` 不会编译本文件。

#![cfg(feature = "physics")]

use hezhou_core::math::Vec3 as CoreVec3;
use hezhou_gameplay::physics_adapter::{PhysicsHitQuery, PhysicsQuery};
use hezhou_gameplay::{
    CameraCollisionQuery, CharacterCollisionQuery, CombatantId, HitDetectionQuery, Hitbox,
    HitboxPose, HitboxShape, Vec3, VisibilityQuery,
};
use hezhou_physics::{ColliderShape, PhysicsWorld, RigidBodyType};

fn add_static_box(world: &mut PhysicsWorld, center: CoreVec3, size: CoreVec3) -> u64 {
    let body = world.create_body_with_type(RigidBodyType::Static);
    world
        .get_body_mut(body)
        .expect("刚体应存在")
        .set_position(center);
    world.create_collider(ColliderShape::box_size(size), body);
    body
}

/// 一块顶面在 y=0 的大地面。
fn flat_world() -> (PhysicsWorld, u64) {
    let mut world = PhysicsWorld::new();
    let ground = add_static_box(
        &mut world,
        CoreVec3::new(0.0, -0.5, 0.0),
        CoreVec3::new(60.0, 1.0, 60.0),
    );
    (world, ground)
}

#[test]
fn ground_probe_returns_snap_point_and_normal() {
    let (world, _) = flat_world();
    let query = PhysicsQuery::new(&world);

    let hit = query
        .ground_probe(Vec3::new(0.0, 0.3, 0.0), 0.35, 1.0)
        .expect("脚下应有地面");

    assert!(
        (hit.point.y - 0.0).abs() < 0.02,
        "吸附点应在地面 y=0，实际 {:?}",
        hit.point
    );
    assert!(
        (hit.distance - 0.3).abs() < 0.02,
        "脚底到地面距离应约 0.3，实际 {}",
        hit.distance
    );
    assert!(hit.normal.y > 0.99, "地面法线应朝上，实际 {:?}", hit.normal);

    // 距离超出探测范围时返回 None。
    assert!(query
        .ground_probe(Vec3::new(0.0, 3.0, 0.0), 0.35, 1.0)
        .is_none());
}

#[test]
fn move_capsule_stops_on_ground() {
    let (world, _) = flat_world();
    let query = PhysicsQuery::new(&world);

    let result = query.move_capsule(
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -2.0, 0.0),
        0.35,
        1.8,
    );

    assert!(
        result.position.y.abs() < 0.05,
        "角色应落到地面，实际 y={}",
        result.position.y
    );
    assert!(!result.contacts.is_empty(), "下落应产生接触面");
    assert!(
        result.contacts.iter().any(|contact| contact.normal.y > 0.9),
        "应包含朝上的地面接触"
    );
    assert!(
        result.position.x.abs() < 1e-4 && result.position.z.abs() < 1e-4,
        "垂直下落不应产生水平偏移"
    );
}

#[test]
fn move_capsule_slides_along_wall() {
    let (mut world, _) = flat_world();
    // 墙：中心 z=-2.5，半厚 0.25 → 正面在 z=-2.25。
    add_static_box(
        &mut world,
        CoreVec3::new(0.0, 2.0, -2.5),
        CoreVec3::new(6.0, 4.0, 0.5),
    );
    let query = PhysicsQuery::new(&world);

    let result = query.move_capsule(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -3.0),
        0.35,
        1.8,
    );

    assert!(
        result.position.z >= -2.26,
        "不应穿墙，实际 z={}",
        result.position.z
    );
    assert!(
        result.contacts.iter().any(|contact| contact.normal.z > 0.9),
        "应包含朝 +Z 的墙面接触"
    );
}

#[test]
fn camera_sweep_shortens_on_wall() {
    let (mut world, _) = flat_world();
    add_static_box(
        &mut world,
        CoreVec3::new(0.0, 2.0, -2.5),
        CoreVec3::new(6.0, 4.0, 0.5),
    );
    let query = PhysicsQuery::new(&world);

    let from = Vec3::new(0.0, 1.5, 0.0);
    let to = Vec3::new(0.0, 1.5, -5.0);
    let sweep = query.sweep_camera(from, to, 0.25);

    assert!(sweep.hit, "相机向墙扫掠应发生遮挡");
    assert!(
        sweep.position.z > -2.3 && sweep.position.z < 0.0,
        "安全位置应在墙前，实际 {:?}",
        sweep.position
    );

    // 没有遮挡的方向应返回原始终点。
    let clear = query.sweep_camera(from, Vec3::new(0.0, 1.5, 5.0), 0.25);
    assert!(!clear.hit);
}

#[test]
fn line_of_sight_reports_occlusion() {
    let (mut world, _) = flat_world();
    add_static_box(
        &mut world,
        CoreVec3::new(0.0, 2.0, -2.5),
        CoreVec3::new(6.0, 4.0, 0.5),
    );
    let query = PhysicsQuery::new(&world);

    assert!(
        !query.has_line_of_sight(Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 1.5, -5.0)),
        "墙后的目标应被遮挡"
    );
    assert!(
        query.has_line_of_sight(Vec3::new(0.0, 1.5, 0.0), Vec3::new(5.0, 1.5, 0.0)),
        "侧向目标应可见"
    );
}

#[test]
fn hitbox_query_maps_body_to_combatant() {
    let mut world = PhysicsWorld::new();
    let enemy = world.create_body_with_type(RigidBodyType::Static);
    world
        .get_body_mut(enemy)
        .expect("刚体应存在")
        .set_position(CoreVec3::new(0.0, 1.0, -0.5));
    world.create_collider(ColliderShape::sphere(0.5), enemy);

    let hit_query = PhysicsHitQuery::new(&world);
    let hitbox = Hitbox::new(HitboxShape::sphere(0.5)).with_offset(Vec3::new(0.0, 1.0, 0.5));
    let pose = HitboxPose::new(Vec3::ZERO, Vec3::FORWARD);
    let contacts = hit_query.query_hitbox(&hitbox, &pose);

    assert_eq!(contacts.len(), 1, "应命中一个目标");
    assert_eq!(contacts[0].target, CombatantId::from(enemy));
    assert!(
        (contacts[0].point - Vec3::new(0.0, 1.0, -0.5)).magnitude() < 1.2,
        "接触点应在敌人附近，实际 {:?}",
        contacts[0].point
    );

    // 命中盒偏移挪远后不应命中。
    let far_hitbox = Hitbox::new(HitboxShape::sphere(0.5)).with_offset(Vec3::new(0.0, 1.0, 8.0));
    assert!(
        hit_query.query_hitbox(&far_hitbox, &pose).is_empty(),
        "偏移过远不应命中"
    );
}

#[test]
fn query_filter_excludes_self_and_limits_layers() {
    let (world, ground) = flat_world();

    let exclude_self = PhysicsQuery::new(&world).exclude_body(ground);
    assert!(
        exclude_self
            .ground_probe(Vec3::new(0.0, 0.3, 0.0), 0.35, 1.0)
            .is_none(),
        "排除地面刚体后不应再探到地面"
    );

    // 地面碰撞体使用默认层（DEFAULT），只查 WORLD 层时不应命中。
    let world_layer_only = PhysicsQuery::new(&world).with_layer_mask(hezhou_physics::layers::WORLD);
    assert!(
        world_layer_only
            .ground_probe(Vec3::new(0.0, 0.3, 0.0), 0.35, 1.0)
            .is_none(),
        "层掩码不匹配时不应命中"
    );
    let default_layer_only =
        PhysicsQuery::new(&world).with_layer_mask(hezhou_physics::layers::DEFAULT);
    assert!(
        default_layer_only
            .ground_probe(Vec3::new(0.0, 0.3, 0.0), 0.35, 1.0)
            .is_some(),
        "默认层地面应能被查询到"
    );
}
