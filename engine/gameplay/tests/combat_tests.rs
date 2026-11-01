//! 战斗能力：命中盒、命中/受击/死亡、闪避无敌帧、格挡减伤、重攻击蓄力。

mod support;

use hezhou_gameplay::{
    resolve_damage, Block, BlockConfig, CombatantId, DamageEvent, DamageKind, Dodge, DodgeConfig,
    DodgePhase, Health, HeavyAttack, HeavyAttackConfig, HitContact, Hitbox, HitboxPose,
    HitboxRuntime, HitboxShape, Vec3,
};
use support::MockHitQuery;

fn attacker() -> CombatantId {
    CombatantId(1)
}
fn target() -> CombatantId {
    CombatantId(2)
}

#[test]
fn hitbox_runtime_detects_and_deduplicates_targets() {
    let query = MockHitQuery::new(vec![
        HitContact::new(target(), Vec3::ZERO, Vec3::FORWARD),
        HitContact::new(target(), Vec3::ZERO, Vec3::FORWARD),
        HitContact::new(CombatantId(3), Vec3::ZERO, Vec3::FORWARD),
    ]);
    let hitbox = Hitbox::default();
    let pose = HitboxPose::new(Vec3::ZERO, Vec3::FORWARD);
    let mut runtime = HitboxRuntime::new();

    assert!(
        runtime.detect(&query, &hitbox, &pose).is_empty(),
        "未激活时不应命中"
    );

    runtime.begin();
    let first = runtime.detect(&query, &hitbox, &pose);
    assert_eq!(first.len(), 2, "同一目标一次攻击只应命中一次");
    assert!(runtime.was_hit(target()));
    assert_eq!(runtime.hit_targets().len(), 2);

    assert!(
        runtime.detect(&query, &hitbox, &pose).is_empty(),
        "同一窗口内不应重复命中"
    );

    // 下一段攻击重新激活后可再次命中。
    runtime.begin();
    assert_eq!(runtime.detect(&query, &hitbox, &pose).len(), 2);
}

#[test]
fn hitbox_pose_rotates_local_offset() {
    let hitbox = Hitbox::new(HitboxShape::sphere(0.5)).with_offset(Vec3::new(0.0, 1.0, 0.6));

    let forward_pose = HitboxPose::new(Vec3::ZERO, Vec3::FORWARD);
    let center = hitbox.world_center(&forward_pose);
    assert!(
        (center - Vec3::new(0.0, 1.0, -0.6)).magnitude() < 1e-4,
        "实际 {center:?}"
    );

    let left_pose = HitboxPose::new(Vec3::ZERO, Vec3::LEFT);
    let center = hitbox.world_center(&left_pose);
    assert!(
        (center - Vec3::new(-0.6, 1.0, 0.0)).magnitude() < 1e-4,
        "实际 {center:?}"
    );
}

#[test]
fn damage_reduces_health_and_kills() {
    let mut health = Health::new(100.0);
    let event = DamageEvent::new(attacker(), target(), 30.0);

    let outcome = resolve_damage(&mut health, None, None, Vec3::FORWARD, &event);
    assert_eq!(outcome.result, hezhou_gameplay::DamageResult::Applied);
    assert!((outcome.damage - 30.0).abs() < 1e-4);
    assert!((outcome.remaining_health - 70.0).abs() < 1e-4);
    assert!(!outcome.killed);
    assert!(!health.is_dead());

    let lethal = DamageEvent::new(attacker(), target(), 80.0);
    let outcome = resolve_damage(&mut health, None, None, Vec3::FORWARD, &lethal);
    assert!((outcome.damage - 70.0).abs() < 1e-4, "扣血不应超过剩余生命");
    assert!(outcome.killed);
    assert!(health.is_dead());
    assert!((health.current() - 0.0).abs() < 1e-4);

    // 死亡后不再结算。
    let after_death = resolve_damage(&mut health, None, None, Vec3::FORWARD, &event);
    assert_eq!(
        after_death.result,
        hezhou_gameplay::DamageResult::AlreadyDead
    );
    assert!((after_death.damage).abs() < 1e-4);
}

#[test]
fn health_heal_and_reset_are_clamped() {
    let mut health = Health::new(100.0);
    health.apply_damage(30.0);
    assert!((health.ratio() - 0.7).abs() < 1e-4);

    assert!((health.heal(50.0) - 30.0).abs() < 1e-4, "回血不应超过上限");
    assert!((health.current() - 100.0).abs() < 1e-4);
    assert!((health.heal(10.0)).abs() < 1e-4, "满血时不应继续回复");

    health.set_current(0.0);
    assert!(health.is_dead());
    health.reset();
    assert!(!health.is_dead(), "reset 应复活为满血");
}

#[test]
fn dodge_invincibility_frames_block_damage() {
    let mut dodge = Dodge::new(DodgeConfig::default());
    let mut health = Health::new(100.0);
    let event = DamageEvent::new(attacker(), target(), 50.0);

    assert!(dodge.try_start(Vec3::FORWARD));
    assert!(!dodge.try_start(Vec3::FORWARD), "闪避中不能再次闪避");

    dodge.update(0.1);
    assert!(dodge.is_active());
    assert!(dodge.is_invincible(), "0.1s 应处于无敌帧内");
    let outcome = resolve_damage(&mut health, None, Some(&dodge), Vec3::FORWARD, &event);
    assert_eq!(outcome.result, hezhou_gameplay::DamageResult::Dodged);
    assert!((outcome.damage).abs() < 1e-4, "无敌帧内不应受伤");
    assert!((health.current() - 100.0).abs() < 1e-4);

    // 越过无敌帧窗口后恢复正常受伤。
    dodge.update(0.3);
    assert!(!dodge.is_invincible());
    assert_eq!(dodge.phase(), DodgePhase::Cooldown);
    let outcome = resolve_damage(&mut health, None, Some(&dodge), Vec3::FORWARD, &event);
    assert_eq!(outcome.result, hezhou_gameplay::DamageResult::Applied);
    assert!((outcome.damage - 50.0).abs() < 1e-4);
}

#[test]
fn dodge_velocity_and_cooldown_sequence() {
    let config = DodgeConfig::default();
    let mut dodge = Dodge::new(config);
    assert!(dodge.try_start(Vec3::new(0.0, 0.0, -1.0)));
    let velocity = dodge.velocity();
    assert!((velocity - Vec3::new(0.0, 0.0, -config.speed)).magnitude() < 1e-4);

    dodge.update(config.duration);
    assert_eq!(dodge.phase(), DodgePhase::Cooldown);
    assert!(
        dodge.velocity().magnitude() < 1e-4,
        "冷却中不应再有闪避位移"
    );

    dodge.update(config.cooldown);
    assert_eq!(dodge.phase(), DodgePhase::Ready);
    assert!(dodge.try_start(Vec3::FORWARD), "冷却结束后可再次闪避");
}

#[test]
fn block_reduces_damage_and_consumes_guard() {
    let mut block = Block::new(BlockConfig::default());
    assert!(block.begin_guard());
    let mut health = Health::new(100.0);
    let event = DamageEvent::new(attacker(), target(), 50.0)
        .with_hit_direction(Vec3::BACK)
        .with_knockback(8.0);

    // 目标面向 -Z，攻击来自正前方（攻击者→目标方向为 +Z）。
    let outcome = resolve_damage(&mut health, Some(&mut block), None, Vec3::FORWARD, &event);
    assert_eq!(outcome.result, hezhou_gameplay::DamageResult::Blocked);
    assert!(
        (outcome.damage - 10.0).abs() < 1e-4,
        "默认减伤 80% 后应受 10 点伤害"
    );
    assert!((outcome.guard_damage - 40.0).abs() < 1e-4);
    assert!((outcome.remaining_health - 90.0).abs() < 1e-4);
    assert!((outcome.knockback - 2.0).abs() < 1e-4, "格挡应削减击退");
    assert!(!outcome.killed);
    assert!(block.is_guarding());
}

#[test]
fn block_requires_facing_attacker() {
    let mut block = Block::new(BlockConfig::default());
    block.begin_guard();
    let mut health = Health::new(100.0);
    let event = DamageEvent::new(attacker(), target(), 50.0).with_hit_direction(Vec3::BACK);

    // 目标背对攻击者（面向 +Z）。
    let outcome = resolve_damage(&mut health, Some(&mut block), None, Vec3::BACK, &event);
    assert_eq!(outcome.result, hezhou_gameplay::DamageResult::Applied);
    assert!((outcome.damage - 50.0).abs() < 1e-4, "背对时不应减伤");
    assert!((block.guard() - block.config().guard_max).abs() < 1e-4);
}

#[test]
fn unblockable_damage_ignores_block() {
    let mut block = Block::new(BlockConfig::default());
    block.begin_guard();
    let mut health = Health::new(100.0);
    let event = DamageEvent::new(attacker(), target(), 50.0)
        .with_hit_direction(Vec3::BACK)
        .with_unblockable(true);

    let outcome = resolve_damage(&mut health, Some(&mut block), None, Vec3::FORWARD, &event);
    assert_eq!(outcome.result, hezhou_gameplay::DamageResult::Applied);
    assert!((outcome.damage - 50.0).abs() < 1e-4);
    assert!((block.guard() - block.config().guard_max).abs() < 1e-4);
}

#[test]
fn guard_breaks_after_depleted() {
    let mut block = Block::new(BlockConfig {
        guard_max: 50.0,
        ..BlockConfig::default()
    });
    block.begin_guard();

    let first = block.absorb(40.0);
    assert!(!first.broken);
    assert!((block.guard() - 18.0).abs() < 1e-4);

    let second = block.absorb(40.0);
    assert!(second.broken, "护盾耗尽应破防");
    assert!(block.is_guard_broken());
    assert!(!block.is_guarding(), "破防后应退出格挡姿态");
    assert!(block.guard_break_remaining() > 0.0);
    assert!(
        !block.can_block(Vec3::FORWARD, Vec3::BACK),
        "破防中无法格挡"
    );
}

#[test]
fn guard_regenerates_when_not_guarding() {
    let config = BlockConfig {
        guard_max: 100.0,
        guard_regen_per_second: 50.0,
        ..BlockConfig::default()
    };
    let mut block = Block::new(config);
    block.begin_guard();
    block.absorb(50.0); // 消耗 40 护盾
    block.end_guard();
    assert!((block.guard() - 60.0).abs() < 1e-4);

    block.update(0.5);
    assert!((block.guard() - 85.0).abs() < 1e-4, "非格挡时应恢复护盾");
}

#[test]
fn heavy_attack_charge_scales_damage_multiplier() {
    let config = HeavyAttackConfig::default();
    let mut heavy = HeavyAttack::new(config);

    assert!(heavy.begin_charge());
    assert!((heavy.movement_scale() - config.movement_scale_while_charging).abs() < 1e-4);
    heavy.update(0.5);
    let release = heavy.release().expect("满足最短蓄力后应能释放");
    assert!((release.charge_ratio - 0.5).abs() < 1e-4);
    assert!(
        (release.damage_multiplier - 1.5).abs() < 1e-4,
        "半蓄力伤害倍率应为 1.5，实际 {}",
        release.damage_multiplier
    );
    assert!(!heavy.is_charging());

    // 满蓄力。
    assert!(heavy.begin_charge());
    heavy.update(5.0);
    assert!(heavy.is_fully_charged());
    let release = heavy.release().expect("满蓄力应能释放");
    assert!((release.charge_ratio - 1.0).abs() < 1e-4);
    assert!((release.damage_multiplier - config.max_damage_multiplier).abs() < 1e-4);
}

#[test]
fn heavy_attack_release_too_early_or_cancel() {
    let mut heavy = HeavyAttack::new(HeavyAttackConfig::default());
    assert!(heavy.begin_charge());
    heavy.update(0.05);
    assert!(heavy.release().is_none(), "低于最短蓄力不应构成重攻击");
    assert!(!heavy.is_charging());

    assert!(heavy.begin_charge());
    heavy.update(0.5);
    heavy.cancel();
    assert!(heavy.last_release().is_none(), "取消不应产生释放结果");
    assert!(heavy.release().is_none());
}

#[test]
fn damage_event_from_contact_uses_contact_normal() {
    let contact = HitContact::new(target(), Vec3::new(0.0, 1.0, -0.5), Vec3::FORWARD);
    let event = DamageEvent::from_contact(attacker(), &contact, 20.0, DamageKind::Heavy);
    assert_eq!(event.kind, DamageKind::Heavy);
    assert!(
        (event.hit_direction - Vec3::BACK).magnitude() < 1e-4,
        "命中方向应为接触法线反方向，实际 {:?}",
        event.hit_direction
    );
    assert!((event.hit_point - Vec3::new(0.0, 1.0, -0.5)).magnitude() < 1e-4);
}
