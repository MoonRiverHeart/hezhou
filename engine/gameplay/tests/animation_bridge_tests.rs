//! 动画状态机桥接：状态映射、去重、一次性动作重启、事件拉取。

mod support;

use hezhou_gameplay::{
    locomotion_state, ActionAnimator, ActionId, ActionState, AnimationEvent, AnimationEventSource,
    AttackDefinition, AttackId, ComboConfig, ComboGraph, ComboInputResult, ComboState, MotorOutput,
    TransitionSpec, Vec3,
};
use support::{grounded_output, MockAnimationBridge, MockAnimationEvents};

fn airborne_output(vertical_speed: f32) -> MotorOutput {
    MotorOutput {
        position: Vec3::ZERO,
        velocity: Vec3::new(0.0, vertical_speed, 0.0),
        grounded: false,
        just_jumped: vertical_speed > 0.0,
        just_landed: false,
        speed: 0.0,
        move_intensity: 0.0,
    }
}

#[test]
fn locomotion_state_maps_motor_output() {
    assert_eq!(
        locomotion_state(&grounded_output(0.0), 5.0),
        ActionState::Idle
    );
    assert_eq!(
        locomotion_state(&grounded_output(2.0), 5.0),
        ActionState::Move
    );
    assert_eq!(
        locomotion_state(&grounded_output(6.5), 5.0),
        ActionState::Run
    );
    assert_eq!(
        locomotion_state(&airborne_output(3.0), 5.0),
        ActionState::Jump
    );
    assert_eq!(
        locomotion_state(&airborne_output(-3.0), 5.0),
        ActionState::Fall
    );

    let landed = MotorOutput {
        just_landed: true,
        grounded: true,
        ..MotorOutput::default()
    };
    assert_eq!(locomotion_state(&landed, 5.0), ActionState::Land);
}

#[test]
fn sync_does_not_repeat_same_state() {
    let mut animator = ActionAnimator::new();
    let mut bridge = MockAnimationBridge::default();

    assert!(animator.sync(ActionState::Idle, &mut bridge));
    assert!(
        !animator.sync(ActionState::Idle, &mut bridge),
        "相同状态不应重复请求"
    );
    assert_eq!(bridge.transitions.len(), 1);

    assert!(animator.sync(ActionState::Run, &mut bridge));
    assert_eq!(bridge.transitions.len(), 2);
    assert_eq!(bridge.transitions[1].from, Some(ActionState::Idle));
    assert_eq!(bridge.transitions[1].to, ActionState::Run);
    assert_eq!(animator.current(), Some(ActionState::Run));
}

#[test]
fn one_shot_action_restarts_same_state() {
    let mut animator = ActionAnimator::new();
    let mut bridge = MockAnimationBridge::default();
    let attack = ActionState::Attack(ActionId(1));

    assert!(animator.play_one_shot(attack, &mut bridge));
    assert!(
        animator.play_one_shot(attack, &mut bridge),
        "连续攻击应重启同一动作"
    );
    assert_eq!(bridge.transitions.len(), 2);
    assert!(
        bridge.transitions[1].spec.restart,
        "一次性动作应带 restart 标记"
    );
}

#[test]
fn rejected_transition_keeps_current_state() {
    let mut animator = ActionAnimator::new();
    let mut bridge = MockAnimationBridge {
        reject: true,
        ..MockAnimationBridge::default()
    };

    assert!(!animator.sync(ActionState::Run, &mut bridge));
    assert_eq!(animator.current(), None, "动画层拒绝时逻辑状态不应更新");
}

#[test]
fn custom_transition_spec_is_forwarded() {
    let mut animator = ActionAnimator::new();
    let mut bridge = MockAnimationBridge::default();
    let spec = TransitionSpec {
        fade_seconds: 0.3,
        restart: false,
    };

    animator.sync_with(ActionState::Block, &mut bridge, spec);
    assert!((bridge.transitions[0].spec.fade_seconds - 0.3).abs() < 1e-4);
}

#[test]
fn combo_attack_state_drives_animation_bridge() {
    let mut graph = ComboGraph::new();
    graph.add_attack(
        AttackDefinition::new(AttackId(1), hezhou_gameplay::AttackKind::Light)
            .with_timing(0.1, 0.1, 0.2)
            .with_action(ActionId(7)),
    );
    let mut combo = ComboState::new();
    let mut animator = ActionAnimator::new();
    let mut bridge = MockAnimationBridge::default();

    assert_eq!(
        combo.press(AttackId(1), &graph, &ComboConfig::default()),
        ComboInputResult::Started(AttackId(1))
    );

    // 逻辑侧攻击状态 → 动画桥接。
    let attack_state = combo.action_state(&graph).expect("应有攻击动画状态");
    assert_eq!(attack_state, ActionState::Attack(ActionId(7)));
    assert!(animator.sync(attack_state, &mut bridge));
    assert_eq!(bridge.last_state(), Some(ActionState::Attack(ActionId(7))));

    // 攻击结束 → 回到待机。
    combo.update(1.0, &graph);
    assert!(!combo.is_attacking());
    assert!(animator.sync(ActionState::Idle, &mut bridge));
    assert_eq!(bridge.transitions.len(), 2);
    assert_eq!(bridge.transitions[1].to, ActionState::Idle);
}

#[test]
fn animation_events_can_be_drained() {
    let mut source = MockAnimationEvents {
        events: vec![
            AnimationEvent::ActionStarted(ActionId(3)),
            AnimationEvent::HitFrame {
                action: ActionId(3),
                hit_index: 1,
            },
            AnimationEvent::Footstep { foot: 0 },
        ],
    };
    let mut events = Vec::new();
    source.drain_events(&mut events);
    assert_eq!(events.len(), 3);
    assert_eq!(
        events[1],
        AnimationEvent::HitFrame {
            action: ActionId(3),
            hit_index: 1,
        }
    );

    // 再次拉取应为空。
    source.drain_events(&mut events);
    assert_eq!(events.len(), 3);
}
