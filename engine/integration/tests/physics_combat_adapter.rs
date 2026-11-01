//! physics ↔ gameplay 集成适配层验收测试。
//!
//! 查询 trait 的实现由 gameplay 的 `physics` feature 提供（gameplay-owner 已实现），
//! 本文件只验证 integration 层的**组合**能力：
//! 固定步、层掩码/排除自身透传、`CombatantId` 映射与去重、单一积分来源、命中结算闭环。

use hezhou_gameplay::{
    resolve_damage, CameraCollisionQuery, CharacterCollisionQuery, CharacterMotor, CombatantId,
    DamageEvent, DamageKind, DamageResult, Health, HitDetectionQuery, Hitbox, HitboxPose,
    HitboxRuntime, HitboxShape, MotorConfig, MotorInput, Vec3, VisibilityQuery,
};
use hezhou_integration::PhysicsCombatAdapter;
use hezhou_physics::{layers, QueryFilter};

// ---------------------------------------------------------------------------
// 测试辅助
// ---------------------------------------------------------------------------

/// 一块顶面在 y=0 的大地面。
fn flat_adapter() -> PhysicsCombatAdapter {
    let mut adapter = PhysicsCombatAdapter::new();
    adapter.spawn_static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(60.0, 1.0, 60.0));
    adapter
}

/// 在 z=-2.5 处加一堵墙（正面朝 +Z，位于 z=-2.25）。
fn wall(adapter: &mut PhysicsCombatAdapter) -> u64 {
    adapter.spawn_static_box(Vec3::new(0.0, 2.0, -2.5), Vec3::new(6.0, 4.0, 0.5))
}

// ---------------------------------------------------------------------------
// 1. 地面探测 / 胶囊移动（复用 gameplay PhysicsQuery）
// ---------------------------------------------------------------------------

#[test]
fn ground_probe_through_physics_query() {
    let adapter = flat_adapter();
    let query = adapter.character_query();

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
        "脚底距离应约 0.3，实际 {}",
        hit.distance
    );
    assert!(hit.normal.y > 0.99, "地面法线应朝上");

    // gameplay motor 的胶囊扫掠落到地面。
    let result = query.move_capsule(
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -2.0, 0.0),
        0.35,
        1.8,
    );
    assert!(result.position.y.abs() < 0.05, "胶囊应落到地面");
    assert!(result.is_blocked(), "下落应产生接触面");
}

/// gameplay motor 用 physics 扫掠做移动：唯一积分来源是 gameplay motor。
#[test]
fn gameplay_motor_integrates_with_physics_sweep_only() {
    let mut adapter = flat_adapter();
    let player = CombatantId::new(1);
    adapter.spawn_character(player, Vec3::new(0.0, 2.0, 0.0), 0.35, 1.8);

    let mut motor = CharacterMotor::with_position(MotorConfig::default(), Vec3::new(0.0, 2.0, 0.0));
    let input = MotorInput::default();
    let fixed_dt = adapter.timestep().step();

    // 掉落 2 秒（120 个固定步）。
    for _ in 0..120 {
        let output = {
            let query = adapter.character_query_for(player);
            motor.step(fixed_dt, &input, &query)
        };
        // 同步角色 body；physics 的 advance 不参与角色积分。
        assert!(adapter.sync_character_transform(player, output.position));
        adapter.advance(fixed_dt);
    }

    assert!(motor.is_grounded(), "角色应落到地面");
    assert!(
        motor.position().y.abs() < 0.05,
        "脚底应贴地 y=0，实际 {:?}",
        motor.position()
    );
    // physics 里的角色 body 已被 motor 同步到同一脚底位置。
    let physics_feet = adapter.character_feet(player).expect("角色 body 应存在");
    assert!(
        (physics_feet - motor.position()).magnitude() < 1e-4,
        "physics body 应与 gameplay motor 位置一致"
    );
}

// ---------------------------------------------------------------------------
// 2. 排除自身 body / 层掩码透传
// ---------------------------------------------------------------------------

#[test]
fn character_query_for_excludes_own_body() {
    let mut adapter = flat_adapter();
    let player = CombatantId::new(1);
    // 做一个挡住去路的静态盒，并把它注册为 player 的 body：
    // 通用查询会撞上它；character_query_for(player) 应把它排除。
    let blocker = adapter.spawn_static_box(Vec3::new(0.0, 1.0, -2.5), Vec3::new(2.0, 2.0, 0.5));
    adapter.register_combatant(player, blocker);

    let plain = adapter.character_query().move_capsule(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -3.0),
        0.35,
        1.8,
    );
    assert!(plain.is_blocked(), "通用查询应被墙体挡住");

    let excluded = adapter.character_query_for(player).move_capsule(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -3.0),
        0.35,
        1.8,
    );
    assert!(
        !excluded.is_blocked(),
        "排除自身 body 后不应再被该盒挡住，contacts={:?}",
        excluded.contacts
    );
    assert!(
        (excluded.position.z + 3.0).abs() < 1e-3,
        "排除后应可以移动 3 米，实际 {}",
        excluded.position.z
    );
}

#[test]
fn layer_mask_is_applied_to_hit_queries() {
    let mut adapter = PhysicsCombatAdapter::new();
    // 敌人：ENEMY 层球体；用层/掩码创建，避免依赖 physics 内部 collider id。
    let enemy_body = adapter.spawn_static_box_with_layers(
        Vec3::new(0.0, 1.0, -0.5),
        Vec3::new(0.6, 1.8, 0.6),
        layers::ENEMY,
        layers::ALL,
    );
    let enemy_combatant = CombatantId::new(77);
    adapter.register_combatant(enemy_combatant, enemy_body);

    let hitbox = Hitbox::new(HitboxShape::sphere(1.0));
    let pose = HitboxPose::new(Vec3::ZERO, Vec3::FORWARD);

    // 只查 ENEMY 层：命中并映射到 combatant。
    adapter.set_hit_filter(QueryFilter::new().with_layer_mask(layers::ENEMY));
    let contacts = adapter.combat_hit_query().query_hitbox(&hitbox, &pose);
    assert_eq!(contacts.len(), 1, "ENEMY 层应命中 1 个目标");
    assert_eq!(contacts[0].target, enemy_combatant);

    // 只查 WORLD 层：不应命中。
    adapter.set_hit_filter(QueryFilter::new().with_layer_mask(layers::WORLD));
    assert!(
        adapter
            .combat_hit_query()
            .query_hitbox(&hitbox, &pose)
            .is_empty(),
        "WORLD 层不应命中 ENEMY 目标"
    );
}

// ---------------------------------------------------------------------------
// 3. 相机扫掠 / 视线
// ---------------------------------------------------------------------------

#[test]
fn camera_sweep_and_line_of_sight_avoid_walls() {
    let mut adapter = flat_adapter();
    wall(&mut adapter);
    let query = adapter.camera_query();

    let from = Vec3::new(0.0, 1.5, 0.0);
    let to = Vec3::new(0.0, 1.5, -5.0);
    let sweep = query.sweep_camera(from, to, 0.25);
    assert!(sweep.hit, "相机向墙扫掠应遮挡");
    assert!(
        sweep.position.z > -2.3 && sweep.position.z < 0.0,
        "安全位置应在墙前，实际 {:?}",
        sweep.position
    );
    assert!(!query.has_line_of_sight(from, to), "墙后目标应不可见");
    assert!(
        query.has_line_of_sight(from, Vec3::new(5.0, 1.5, 0.0)),
        "侧向目标应可见"
    );
}

// ---------------------------------------------------------------------------
// 4. 命中盒映射 / 去重 / runtime
// ---------------------------------------------------------------------------

#[test]
fn combat_hit_query_remaps_body_ids_and_dedupes() {
    // 不放地板，避免地面也落入重叠查询、干扰"去重为 1"的断言。
    let mut adapter = PhysicsCombatAdapter::new();
    // 两个 body 注册到同一个 combatant（多部位命中盒场景）。
    let first = adapter.spawn_static_box(Vec3::new(0.0, 1.0, -0.5), Vec3::new(0.2, 1.8, 0.2));
    let second = adapter.spawn_static_box(Vec3::new(0.15, 1.0, -0.5), Vec3::new(0.2, 1.8, 0.2));
    let enemy = CombatantId::new(42);
    adapter.register_combatant(enemy, first);
    adapter.register_combatant(enemy, second);

    let hitbox = Hitbox::new(HitboxShape::sphere(1.2));
    let pose = HitboxPose::new(Vec3::ZERO, Vec3::FORWARD);
    let contacts = adapter.combat_hit_query().query_hitbox(&hitbox, &pose);
    assert_eq!(
        contacts.len(),
        1,
        "同一 combatant 的多个 body 应去重为 1 个目标"
    );
    assert_eq!(contacts[0].target, enemy);

    // 未注册 body 回退为同值 CombatantId（与 gameplay 适配层一致）。
    let other = adapter.spawn_static_box(Vec3::new(3.0, 1.0, -0.5), Vec3::new(0.2, 1.8, 0.2));
    let wide = Hitbox::new(HitboxShape::sphere(6.0));
    let contacts = adapter.combat_hit_query().query_hitbox(&wide, &pose);
    assert!(contacts
        .iter()
        .any(|contact| contact.target == CombatantId::from(other)));
}

#[test]
fn hitbox_runtime_filters_repeated_hits() {
    let mut adapter = PhysicsCombatAdapter::new();
    let enemy_body = adapter.spawn_static_box(Vec3::new(0.0, 1.0, -0.5), Vec3::new(0.6, 1.8, 0.6));
    let enemy = CombatantId::new(9);
    adapter.register_combatant(enemy, enemy_body);

    let hitbox = Hitbox::new(HitboxShape::sphere(1.2));
    let pose = HitboxPose::new(Vec3::ZERO, Vec3::FORWARD);
    let query = adapter.combat_hit_query();
    let mut runtime = HitboxRuntime::new();
    runtime.begin();

    let first = runtime.detect(&query, &hitbox, &pose);
    assert_eq!(first.len(), 1, "首次查询应命中");
    assert_eq!(first[0].target, enemy);

    let second = runtime.detect(&query, &hitbox, &pose);
    assert!(second.is_empty(), "同一窗口内同一目标只命中一次");

    // 新窗口（下一段攻击）重新允许命中。
    runtime.begin();
    assert_eq!(runtime.detect(&query, &hitbox, &pose).len(), 1);
}

// ---------------------------------------------------------------------------
// 5. 固定步 / 60·144fps 一致性 / 单一积分来源
// ---------------------------------------------------------------------------

#[test]
fn fixed_timestep_is_frame_rate_independent() {
    // 同样的固定步数、同样的固定步长 → 角色轨迹应一致。
    fn run(total_seconds: f32, fps: f32) -> (Vec3, u32, Vec3) {
        let mut adapter = PhysicsCombatAdapter::with_fps(60.0);
        adapter.spawn_static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(60.0, 1.0, 60.0));
        let player = CombatantId::new(1);
        adapter.spawn_character(player, Vec3::new(0.0, 1.5, 0.0), 0.35, 1.8);
        // 一个参与动态积分的道具，用来验证 physics 固定步同样确定性。
        let prop = adapter.spawn_dynamic_box(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.5, 0.5, 0.5));
        let mut motor =
            CharacterMotor::with_position(MotorConfig::default(), Vec3::new(0.0, 1.5, 0.0));
        let input = MotorInput::with_direction(Vec3::new(1.0, 0.0, 0.0));
        let fixed_dt = adapter.timestep().step();
        let frame_dt = 1.0 / fps;
        let frames = (total_seconds * fps).round() as u32;
        let mut steps_total = 0;
        for _ in 0..frames {
            let result = {
                // 固定步数由 FixedTimestep 决定；每步跑一次 gameplay motor。
                let steps = adapter.advance(frame_dt).steps;
                for _ in 0..steps {
                    let output = {
                        let query = adapter.character_query_for(player);
                        motor.step(fixed_dt, &input, &query)
                    };
                    adapter.sync_character_transform(player, output.position);
                }
                steps
            };
            steps_total += result;
        }
        let prop_position = hezhou_gameplay::from_core_vec3(
            *adapter
                .world()
                .get_body(prop)
                .expect("道具 body")
                .position(),
        );
        (motor.position(), steps_total, prop_position)
    }

    let (pos_60, steps_60, prop_60) = run(1.0, 60.0);
    let (pos_144, steps_144, prop_144) = run(1.0, 144.0);

    // 固定步数与轨迹由 FixedTimestep 对齐（允许浮点累加差 1 步）。
    assert!(
        (steps_60 as i64 - steps_144 as i64).abs() <= 1,
        "60/144fps 固定步数应一致：{steps_60} vs {steps_144}"
    );
    if steps_60 == steps_144 {
        assert!(
            (pos_60 - pos_144).magnitude() < 1e-4,
            "同固定步数下角色轨迹应一致：{pos_60:?} vs {pos_144:?}"
        );
        assert!(
            (prop_60 - prop_144).magnitude() < 1e-4,
            "动态道具轨迹也应一致：{prop_60:?} vs {prop_144:?}"
        );
    }
}

#[test]
fn only_gameplay_motor_integrates_character() {
    let mut adapter = flat_adapter();
    let player = CombatantId::new(1);
    let start = Vec3::new(0.0, 2.0, 0.0);
    adapter.spawn_character(player, start, 0.35, 1.8);
    // 动态道具：physics 的 advance 会推进它（说明 advance 并非空操作）。
    let prop = adapter.spawn_dynamic_box(Vec3::new(3.0, 8.0, 0.0), Vec3::new(0.5, 0.5, 0.5));

    // 不调用 gameplay motor，只跑 physics 固定步 1 秒。
    for _ in 0..60 {
        adapter.advance(1.0 / 60.0);
    }

    // 角色是 Kinematic：physics step 不会积分它，位置保持原样。
    let feet = adapter.character_feet(player).expect("角色 body");
    assert!(
        (feet - start).magnitude() < 1e-5,
        "physics step 不应积分角色（单一积分来源），实际 {feet:?}"
    );

    // 动态道具下落了（说明固定步确实在推进动态刚体）。
    let prop_y = hezhou_gameplay::from_core_vec3(
        *adapter
            .world()
            .get_body(prop)
            .expect("道具 body")
            .position(),
    )
    .y;
    assert!(prop_y < 8.0 - 1.0, "动态道具应受重力下落，实际 y={prop_y}");
}

// ---------------------------------------------------------------------------
// 6. 命中结算闭环（命中盒 → DamageEvent → resolve_damage）
// ---------------------------------------------------------------------------

#[test]
fn hit_detection_feeds_damage_resolution() {
    let mut adapter = flat_adapter();
    // 敌人身体：注册为 combatant。
    let enemy_body = adapter.spawn_static_box(Vec3::new(0.0, 1.0, -0.6), Vec3::new(0.6, 1.8, 0.6));
    let enemy = CombatantId::new(2);
    let attacker = CombatantId::new(1);
    adapter.register_combatant(enemy, enemy_body);

    // 攻击者在原点朝 -Z 出拳：命中盒在身前（offset.z 沿 forward 方向）。
    let hitbox = Hitbox::new(HitboxShape::sphere(0.6)).with_offset(Vec3::new(0.0, 1.0, 0.7));
    let pose = HitboxPose::new(Vec3::ZERO, Vec3::FORWARD);

    // 命中查询自动排除攻击者自身 body（这里攻击者未注册 body，行为同通用查询）。
    let query = adapter.combat_hit_query_for(attacker);
    let mut runtime = HitboxRuntime::new();
    runtime.begin();
    let contacts = runtime.detect(&query, &hitbox, &pose);
    assert_eq!(contacts.len(), 1, "应命中敌人");
    assert_eq!(contacts[0].target, enemy);

    // 命中结算：命中帧 → DamageEvent → resolve_damage。
    let mut health = Health::new(100.0);
    let event = DamageEvent::from_contact(attacker, &contacts[0], 35.0, DamageKind::Heavy);
    let outcome = resolve_damage(&mut health, None, None, Vec3::FORWARD, &event);
    assert_eq!(outcome.result, DamageResult::Applied);
    assert_eq!(health.current(), 65.0);
    assert!(!outcome.killed);
}

// ---------------------------------------------------------------------------
// 7. 映射表 / 查询默认过滤器
// ---------------------------------------------------------------------------

#[test]
fn combatant_body_mapping_roundtrip_and_filters() {
    let mut adapter = PhysicsCombatAdapter::new();
    let combatant = CombatantId::new(100);
    let body = adapter.spawn_static_box(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
    let weapon_body = adapter.spawn_static_box(Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.2, 0.2, 1.0));

    assert!(adapter.register_combatant(combatant, body));
    // 一个 combatant 可注册多个 body（多部位命中盒）。
    assert!(adapter.register_combatant(combatant, weapon_body));
    assert!(
        !adapter.register_combatant(combatant, body),
        "重复注册返回 false"
    );
    assert_eq!(adapter.body_of_combatant(combatant), Some(body));
    assert_eq!(adapter.bodies_of_combatant(combatant), &[body, weapon_body]);
    assert_eq!(adapter.combatant_of_body(weapon_body), combatant);

    assert_eq!(
        adapter.unregister_combatant(combatant),
        vec![body, weapon_body]
    );
    assert_eq!(adapter.body_of_combatant(combatant), None);
    // 未注册 body 回退为同值 CombatantId。
    assert_eq!(adapter.combatant_of_body(body), CombatantId::from(body));

    // 默认过滤器：所有层、忽略触发器。
    assert_eq!(
        adapter.character_filter().layer_mask(),
        QueryFilter::new().layer_mask()
    );
    assert!(!adapter.character_filter().includes_sensors());
    adapter.set_character_layer_mask(layers::WORLD);
    assert_eq!(adapter.character_filter().layer_mask(), layers::WORLD);
}
