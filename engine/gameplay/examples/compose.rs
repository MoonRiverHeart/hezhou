//! 组合示例：展示 `hezhou-gameplay` 各能力如何拼装使用。
//!
//! 本示例**只展示 API 用法**，不包含具体游戏规则/关卡/数值：
//! 地图是一块平地，攻击只是一段通用时间轴，敌人只是一个 ID。
//! 真实项目应把这里的手写循环换成 ECS 系统，并用 physics/animation 的实现替换
//! `FlatGroundQuery` 与 `DummyHitQuery`。

use hezhou_gameplay::prelude::*;
use hezhou_gameplay::{ActionStateBridge, ActionTransition, DamageEvent, HitDetectionQuery};

/// 把逻辑状态切换打印出来，真实项目里由 animation crate 实现。
struct PrintAnimator;

impl ActionStateBridge for PrintAnimator {
    fn request_transition(&mut self, transition: ActionTransition) -> bool {
        println!(
            "[anim] {:?} -> {:?} (fade {:.2}s)",
            transition.from, transition.to, transition.spec.fade_seconds
        );
        true
    }
}

/// 简化命中查询：固定命中一个目标（真实项目由 physics 提供重叠查询）。
struct DummyHitQuery {
    targets: Vec<CombatantId>,
}

impl HitDetectionQuery for DummyHitQuery {
    fn query_hitbox(&self, _hitbox: &Hitbox, _pose: &HitboxPose) -> Vec<HitContact> {
        self.targets
            .iter()
            .map(|target| HitContact::new(*target, Vec3::new(0.0, 1.0, -1.2), Vec3::FORWARD))
            .collect()
    }
}

fn main() {
    // 1. 物理查询：示例用平地参考实现；正式项目替换为 physics crate 的实现。
    let world = FlatGroundQuery::new(0.0);
    let mut motor = CharacterMotor::new(MotorConfig::default());
    let mut animator = ActionAnimator::new();
    let mut animator_bridge = PrintAnimator;

    // 2. 第三人称相机 + 锁定。
    let mut camera = ThirdPersonCamera::new(ThirdPersonCameraConfig::default());
    camera.set_pitch(0.2);
    let mut lock_on = LockOnState::new();
    let candidates = [LockTargetCandidate::new(
        CombatantId(2),
        Vec3::new(0.0, 0.0, -3.0),
    )];
    if let Some(target_id) = lock_on.select(
        Vec3::new(0.0, 1.5, 4.0),
        Vec3::FORWARD,
        &candidates,
        &LockOnConfig::default(),
        Some(&world),
    ) {
        println!("锁定目标: {target_id}");
    }

    // 3. 连招图：两段通用攻击定义（时间轴 + 命中盒 + 下一段）。
    let mut graph = ComboGraph::new();
    graph.add_attack(
        AttackDefinition::new(AttackId(1), AttackKind::Light)
            .with_timing(0.1, 0.1, 0.2)
            .with_next([AttackId(2)])
            .with_damage(10.0, DamageKind::Light),
    );
    graph.add_attack(
        AttackDefinition::new(AttackId(2), AttackKind::Light)
            .with_timing(0.1, 0.1, 0.25)
            .with_damage(12.0, DamageKind::Light)
            .as_combo_only(),
    );
    let mut combo = ComboState::new();
    let combo_config = ComboConfig::default();
    let _ = combo.press(AttackId(1), &graph, &combo_config);

    // 4. 战斗结算组件。
    let mut health = Health::new(100.0);
    let mut block = Block::new(BlockConfig::default());
    let mut dodge = Dodge::new(DodgeConfig::default());
    let mut hit_runtime = HitboxRuntime::new();
    let hit_query = DummyHitQuery {
        targets: vec![CombatantId(2)],
    };
    let attacker = CombatantId(1);

    // 5. 固定时间步主循环（示例 2 秒）。
    let dt = 1.0 / 60.0;
    let input = MotorInput::with_direction(Vec3::FORWARD);
    let mut was_hitbox_active = false;
    let mut damage_total = 0.0f32;

    for frame in 0..120 {
        // 角色移动 + 动画状态同步（用相机朝向把输入转成世界方向）。
        let motor_output = motor.step(dt, &input, &world);
        let camera_output = camera.update(dt, motor_output.position, &world);
        let _ = camera_output; // 渲染层使用

        // 主动画状态：攻击期间优先使用 ComboState 的动作状态，否则用移动状态。
        let combo_update = combo.update(dt, &graph);
        match combo.action_state(&graph) {
            Some(attack_state) => {
                animator.sync(attack_state, &mut animator_bridge);
            }
            None => {
                let locomotion = locomotion_state(&motor_output, 5.0);
                animator.sync(locomotion, &mut animator_bridge);
            }
        }

        // 攻击进入判定窗口的第一帧：查询命中 → 生成伤害事件 → 结算。
        if combo_update.hitbox_active && !was_hitbox_active {
            if let Some(attack_id) = combo_update.action {
                if let Some(definition) = graph.attack(attack_id) {
                    hit_runtime.begin();
                    let pose = HitboxPose::new(motor_output.position, Vec3::FORWARD);
                    let contacts = hit_runtime.detect(&hit_query, &definition.hitbox, &pose);
                    for event in definition.damage_events(attacker, Vec3::FORWARD, &contacts) {
                        let outcome = resolve_damage(
                            &mut health,
                            Some(&mut block),
                            Some(&dodge),
                            Vec3::FORWARD,
                            &event,
                        );
                        damage_total += outcome.damage;
                    }
                }
            }
        }
        was_hitbox_active = combo_update.hitbox_active;

        // 窗口内尝试第二段连招。
        if frame == 20 {
            let _ = combo.press(AttackId(2), &graph, &combo_config);
        }
        // 闪避与格挡状态推进。
        if frame == 60 {
            let _ = dodge.try_start(Vec3::RIGHT);
        }
        dodge.update(dt);
        if frame == 100 {
            block.begin_guard();
        }
        block.update(dt);
    }

    // 6. 结算一次"格挡中的攻击"作为示例输出。
    block.begin_guard();
    let incoming = DamageEvent::new(attacker, CombatantId(2), 20.0).with_hit_direction(Vec3::BACK);
    let outcome = resolve_damage(
        &mut health,
        Some(&mut block),
        Some(&dodge),
        Vec3::FORWARD,
        &incoming,
    );

    println!(
        "攻击累计伤害={damage_total:.1}；格挡结算={:?} 实际伤害={:.1} 剩余生命={:.1}",
        outcome.result, outcome.damage, outcome.remaining_health
    );
    println!("当前动画状态={:?}", animator.current());
    println!(
        "锁定目标={:?} 瞄准点={:?}",
        lock_on.current(),
        lock_on.aim_point()
    );
    println!("角色位置={:?}", motor.position());
}
