//! `hezhou-integration` 公开 API 验收测试。
//!
//! 覆盖任务要求的：状态映射、逐招式 clip、事件透传、循环/一次性、Death 终止、
//! restart、过渡中同目标，以及 gameplay `ActionAnimator` / `AnimationEventSource`
//! 两个 trait 的真实接入方式。

use hezhou_animation::{
    AnimationClip, AnimationEvent as ClipEvent, AnimationState, Channel, Keyframe, Skeleton,
    Transform, Vec3,
};
use hezhou_gameplay::{
    locomotion_state, ActionAnimator, ActionId, ActionState, ActionStateBridge, ActionTransition,
    AnimationEvent, AnimationEventSource, MotorOutput, TransitionSpec,
};
use hezhou_integration::{
    action_id, map_action_state, reserved_action_ids, AnimationActionAdapter,
    AnimationActionBridge, ClipBinding,
};

// ---------------------------------------------------------------------------
// 测试辅助
// ---------------------------------------------------------------------------

fn skeleton() -> Skeleton {
    let mut skeleton = Skeleton::new("test");
    skeleton.add_root_joint("root", Transform::IDENTITY);
    skeleton
}

/// 构造一个 0 → 1 的平移片段，便于用姿态数值验证"当前播放的是哪个 clip"。
fn translation_clip(name: &str, duration: f32) -> AnimationClip {
    let mut clip = AnimationClip::new(name, duration);
    let mut channel = Channel::translation(0);
    channel
        .add_key(Keyframe::translation(0.0, Vec3::ZERO))
        .unwrap();
    channel
        .add_key(Keyframe::translation(duration, Vec3::new(1.0, 0.0, 0.0)))
        .unwrap();
    clip.add_channel(channel);
    clip
}

/// 带事件的平移片段：`events = [(时间, 事件名)]`。
fn clip_with_events(name: &str, duration: f32, events: &[(f32, &str)]) -> AnimationClip {
    let mut clip = translation_clip(name, duration);
    for (time, event_name) in events {
        clip.add_event(ClipEvent::new(*time, *event_name));
    }
    clip
}

fn attack_clip(action: u32, duration: f32) -> AnimationClip {
    translation_clip(&format!("attack_{action}"), duration)
}

// ---------------------------------------------------------------------------
// 1. 状态映射
// ---------------------------------------------------------------------------

#[test]
fn maps_all_gameplay_action_states() {
    // 0.2 animation 的 6 状态映射（与扩展是否合入无关的部分）。
    let cases = [
        (ActionState::Idle, "Idle"),
        (ActionState::Move, "Run"),
        (ActionState::Run, "Run"),
        (ActionState::Jump, "Jump"),
        (ActionState::Fall, "Jump"),
        (ActionState::Attack(ActionId::new(7)), "Attack"),
        (ActionState::HeavyAttack(ActionId::new(8)), "Attack"),
        (ActionState::Hit, "Hit"),
        (ActionState::Death, "Death"),
    ];
    for (action, expected) in cases {
        assert_eq!(
            map_action_state(action).name(),
            expected,
            "ActionState {action:?} 映射错误"
        );
    }

    // Land/Dodge/Block：依赖 animation task-11 是否合入。
    if hezhou_integration::EXTENDED_ANIMATION_STATES {
        assert_eq!(map_action_state(ActionState::Land).name(), "Land");
        assert_eq!(map_action_state(ActionState::Dodge).name(), "Dodge");
        assert_eq!(map_action_state(ActionState::Block).name(), "Block");
    } else {
        // 兼容映射：Land→Jump、Dodge/Block→Attack（专用 clip 区分动作）。
        assert_eq!(map_action_state(ActionState::Land).name(), "Jump");
        assert_eq!(map_action_state(ActionState::Dodge).name(), "Attack");
        assert_eq!(map_action_state(ActionState::Block).name(), "Attack");
    }
}

// ---------------------------------------------------------------------------
// 2. 逐招式 clip
// ---------------------------------------------------------------------------

#[test]
fn state_slot_clip_allows_attack_action() {
    // 任务要求的"先 set_clip(AnimationState::Attack, clip_for_action) 再切换"。
    let mut adapter = AnimationActionAdapter::new();
    adapter.set_clip(
        AnimationState::Attack,
        translation_clip("custom_attack", 0.5),
    );
    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(42))));
    assert_eq!(adapter.current_state(), AnimationState::Attack);
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(AnimationState::Attack)
            .unwrap()
            .name,
        "custom_attack"
    );
}

#[test]
fn per_action_clips_switch_attack_state() {
    let mut adapter = AnimationActionAdapter::new();
    adapter
        .set_action_clip(ActionId::new(1), attack_clip(1, 0.5))
        .set_action_clip(ActionId::new(2), attack_clip(2, 0.5));

    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(1))));
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(AnimationState::Attack)
            .unwrap()
            .name,
        "attack_1"
    );

    // 第二段连招：动画状态仍是 Attack，但 clip 必须是 attack_2。
    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(2))));
    assert_eq!(adapter.current_state(), AnimationState::Attack);
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(AnimationState::Attack)
            .unwrap()
            .name,
        "attack_2"
    );
}

#[test]
fn heavy_attack_uses_action_clip() {
    let mut adapter = AnimationActionAdapter::new();
    adapter.set_action_clip(ActionId::new(9), translation_clip("heavy_attack_9", 1.2));
    assert!(adapter.request_one_shot(ActionState::HeavyAttack(ActionId::new(9))));
    assert_eq!(adapter.current_state(), AnimationState::Attack);
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(AnimationState::Attack)
            .unwrap()
            .name,
        "heavy_attack_9"
    );
}

#[test]
fn missing_action_clip_is_rejected() {
    // 攻击类没有注册任何 clip = 资产缺失，gameplay 会收到 false 并降级。
    let mut bridge = AnimationActionBridge::new();
    assert!(
        !bridge.request_one_shot(ActionState::Attack(ActionId::new(404))),
        "缺少招式 clip 时应拒绝并要求降级"
    );
    // 但基础动作不要求 clip（采样绑定姿态即可）。
    assert!(bridge.request(ActionState::Idle));
}

// ---------------------------------------------------------------------------
// 3. 事件映射 / 透传
// ---------------------------------------------------------------------------

#[test]
fn clip_events_map_to_gameplay_events() {
    let mut adapter = AnimationActionAdapter::new();
    adapter.set_action_clip(
        ActionId::new(5),
        clip_with_events(
            "attack_5",
            1.0,
            &[
                (0.1, "attack_hit"),
                (0.2, "cancel_window_open"),
                (0.25, "footstep_r"),
                (0.3, "vfx_glow"),
            ],
        ),
    );
    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(5))));

    let events = adapter.update(0.35);
    assert!(events.contains(&AnimationEvent::HitFrame {
        action: ActionId::new(5),
        hit_index: 0,
    }));
    assert!(events.contains(&AnimationEvent::CancelWindowOpen(ActionId::new(5))));
    assert!(events.contains(&AnimationEvent::Footstep { foot: 1 }));
    // 未识别的资产事件被计数忽略，不会 panic 也不会误派发。
    assert_eq!(
        adapter.bridge().mapper().ignored_clip_event_count(),
        1,
        "vfx_glow 应被忽略并计数"
    );
}

#[test]
fn enter_exit_and_transition_events_are_not_duplicated() {
    let mut bridge = AnimationActionBridge::new().with_default_transition(0.2);
    bridge.set_state_clip(
        AnimationState::Idle,
        ClipBinding::looping(translation_clip("idle", 1.0)),
    );
    bridge.set_action_clip(
        ActionId::new(1),
        ClipBinding::one_shot(translation_clip("attack", 1.0)),
    );

    // 首次同步到 Idle：状态机本来就在 Idle，不产生事件。
    assert!(bridge.request(ActionState::Idle));
    assert!(bridge.update(0.0).is_empty());

    // Idle -> Attack（带过渡）：Exit/TransitionStarted/Enter 只应产生一组事件。
    assert!(bridge.request_one_shot(ActionState::Attack(ActionId::new(1))));
    let events = bridge.update(0.05);

    let started: Vec<&AnimationEvent> = events
        .iter()
        .filter(|event| matches!(event, AnimationEvent::ActionStarted(_)))
        .collect();
    let ended: Vec<&AnimationEvent> = events
        .iter()
        .filter(|event| matches!(event, AnimationEvent::ActionEnded(_)))
        .collect();
    assert_eq!(
        started.len(),
        1,
        "过渡不应重复派发 ActionStarted: {events:?}"
    );
    assert_eq!(ended.len(), 1, "过渡不应重复派发 ActionEnded: {events:?}");
    assert_eq!(*started[0], AnimationEvent::ActionStarted(ActionId::new(1)));
    assert_eq!(
        *ended[0],
        AnimationEvent::ActionEnded(reserved_action_ids::IDLE)
    );

    // TransitionFinished 在 gameplay 0.2 没有对应枚举，保留在原始事件流。
    let _ = bridge.update(0.3);
    let raw = bridge.drain_state_events();
    assert!(raw.iter().any(|event| matches!(
        event,
        hezhou_animation::StateEvent::TransitionFinished {
            state: AnimationState::Attack
        }
    )));
    assert!(
        bridge.drain_events().is_empty(),
        "过渡完成不伪造 gameplay 事件"
    );
}

// ---------------------------------------------------------------------------
// 4. 循环 / 一次性动作
// ---------------------------------------------------------------------------

#[test]
fn looping_idle_and_one_shot_attack_timing() {
    let mut adapter = AnimationActionAdapter::new();
    adapter.set_clip(AnimationState::Idle, translation_clip("idle", 1.0));
    adapter.set_action_clip(ActionId::new(1), attack_clip(1, 1.0));

    assert!(adapter.request(ActionState::Idle));
    adapter.update(1.25);
    assert!(
        (adapter.current_time() - 0.25).abs() < 1e-4,
        "Idle 应循环回绕"
    );
    assert_eq!(
        adapter.state_remaining(),
        None,
        "循环动作由 gameplay 时间驱动"
    );

    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(1))));
    adapter.update(0.5);
    assert!((adapter.current_time() - 0.5).abs() < 1e-4);
    let remaining = adapter.state_remaining().expect("一次性动作应有剩余时间");
    assert!((remaining - 0.5).abs() < 1e-3, "remaining={remaining}");

    adapter.update(2.0);
    assert!(
        (adapter.current_time() - 1.0).abs() < 1e-4,
        "一次性动作应钳制在 duration"
    );
    assert!(
        adapter.state_remaining().unwrap() < 1e-4,
        "一次性动作播完剩余时间为 0"
    );
}

// ---------------------------------------------------------------------------
// 5. Death 终止
// ---------------------------------------------------------------------------

#[test]
fn death_terminates_and_rejects_later_requests() {
    let mut bridge = AnimationActionBridge::new();
    bridge.set_clip(AnimationState::Idle, translation_clip("idle", 1.0));
    bridge.set_state_clip(
        AnimationState::Death,
        ClipBinding::one_shot(translation_clip("death", 1.0)),
    );
    bridge.set_action_clip(
        ActionId::new(1),
        ClipBinding::one_shot(translation_clip("attack", 1.0)),
    );

    assert!(bridge.request_one_shot(ActionState::Attack(ActionId::new(1))));
    assert!(bridge.request_one_shot(ActionState::Death));
    assert!(bridge.is_terminal());
    assert_eq!(bridge.current_animation_state(), AnimationState::Death);

    // 进入 Death 后一切切换都被拒绝（包括再进 Death / 回 Idle / 攻击）。
    assert!(!bridge.request(ActionState::Idle));
    assert!(!bridge.request_one_shot(ActionState::Attack(ActionId::new(1))));
    assert!(!bridge.request_one_shot(ActionState::Death));

    // 时间继续推进，但状态不会再变化。
    bridge.update(10.0);
    assert_eq!(bridge.current_animation_state(), AnimationState::Death);
    assert_eq!(
        bridge.current_action(),
        Some(ActionState::Death),
        "终态仍报告死亡动作"
    );
}

// ---------------------------------------------------------------------------
// 6. restart
// ---------------------------------------------------------------------------

#[test]
fn one_shot_restart_resets_time_and_emits_started() {
    let mut bridge = AnimationActionBridge::new();
    bridge.set_action_clip(
        ActionId::new(1),
        ClipBinding::one_shot(translation_clip("attack", 1.0)),
    );

    assert!(bridge.request_one_shot(ActionState::Attack(ActionId::new(1))));
    let first_events = bridge.update(0.4);
    assert!(bridge.current_time() > 0.3);
    assert!(
        first_events.contains(&AnimationEvent::ActionStarted(ActionId::new(1))),
        "首次请求应有 ActionStarted，事件={first_events:?}"
    );

    // 同一动作重播（gameplay `play_one_shot` 的 restart 语义）。
    assert!(bridge.request_one_shot(ActionState::Attack(ActionId::new(1))));
    assert_eq!(bridge.current_time(), 0.0, "restart 应把片段时间归零");
    let events = bridge.update(0.0);
    assert!(
        events.contains(&AnimationEvent::ActionStarted(ActionId::new(1))),
        "restart 应补发 ActionStarted，事件={events:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. 过渡中请求同一目标
// ---------------------------------------------------------------------------

#[test]
fn same_target_during_transition_restarts_without_new_transition() {
    let mut adapter = AnimationActionAdapter::new();
    adapter
        .set_action_clip(ActionId::new(1), attack_clip(1, 1.0))
        .set_action_clip(ActionId::new(2), attack_clip(2, 1.0));

    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(1))));
    adapter.update(0.02);
    assert!(adapter.is_transitioning(), "0.08s 淡入中");

    // 淡入还没结束就请求同一 animation 状态（不同招式）：接受、换 clip、重启时间，
    // 但不重新开始过渡。
    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(2))));
    assert_eq!(adapter.current_state(), AnimationState::Attack);
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(AnimationState::Attack)
            .unwrap()
            .name,
        "attack_2"
    );
    assert_eq!(adapter.current_time(), 0.0);
    assert!(adapter.is_transitioning(), "进行中的淡入不应被取消");

    adapter.update(0.1);
    assert!(!adapter.is_transitioning(), "过渡应正常结束");
}

// ---------------------------------------------------------------------------
// 8. gameplay 两个 trait 的真实接入
// ---------------------------------------------------------------------------

#[test]
fn gameplay_action_animator_drives_bridge() {
    let mut adapter = AnimationActionAdapter::with_default_transition(0.1);
    adapter.set_clip(AnimationState::Idle, translation_clip("idle", 1.0));
    adapter.set_clip(AnimationState::Run, translation_clip("run", 1.0));
    adapter.set_action_clip(ActionId::new(1), attack_clip(1, 1.0));

    let mut animator = ActionAnimator::new();
    assert!(animator.sync(ActionState::Idle, adapter.bridge_mut()));
    assert_eq!(animator.current(), Some(ActionState::Idle));

    // 移动输出 → 基础状态（模拟每帧第一步）。
    let output = MotorOutput {
        grounded: true,
        speed: 5.0,
        ..Default::default()
    };
    let desired = locomotion_state(&output, 4.0);
    assert_eq!(desired, ActionState::Run);
    assert!(animator.sync(desired, adapter.bridge_mut()));
    assert_eq!(adapter.current_state(), AnimationState::Run);

    let events = adapter.update(1.0 / 60.0);
    assert!(events.contains(&AnimationEvent::ActionEnded(reserved_action_ids::IDLE)));
    assert!(events.contains(&AnimationEvent::ActionStarted(reserved_action_ids::RUN)));

    // 战斗层覆盖一次性动作。
    assert!(animator.play_one_shot(ActionState::Attack(ActionId::new(1)), adapter.bridge_mut()));
    adapter.update(0.25);
    assert_eq!(adapter.current_state(), AnimationState::Attack);
    assert_eq!(
        animator.current(),
        Some(ActionState::Attack(ActionId::new(1)))
    );

    let pose = adapter.sample(&skeleton());
    assert!((pose.locals[0].translation.x - 0.25).abs() < 1e-3);
}

#[test]
fn animation_event_source_drains_mapped_events() {
    let mut bridge = AnimationActionBridge::new();
    bridge.set_action_clip(
        ActionId::new(1),
        ClipBinding::one_shot(translation_clip("attack", 1.0)),
    );
    assert!(bridge.request_one_shot(ActionState::Attack(ActionId::new(1))));
    bridge.update(0.1);
    // 重播产生合成事件，进入缓冲。
    assert!(bridge.request_one_shot(ActionState::Attack(ActionId::new(1))));

    let mut out = Vec::new();
    AnimationEventSource::drain_events(&mut bridge, &mut out);
    assert!(out.contains(&AnimationEvent::ActionStarted(ActionId::new(1))));

    let mut again = Vec::new();
    AnimationEventSource::drain_events(&mut bridge, &mut again);
    assert!(again.is_empty(), "drain 后缓冲应清空");
}

#[test]
fn bridge_trait_reports_state_remaining() {
    let mut bridge = AnimationActionBridge::new();
    bridge.set_clip(AnimationState::Idle, translation_clip("idle", 1.0));
    bridge.set_action_clip(
        ActionId::new(1),
        ClipBinding::one_shot(translation_clip("attack", 1.0)),
    );

    {
        let bridge_dyn: &mut dyn ActionStateBridge = &mut bridge;
        assert_eq!(bridge_dyn.state_remaining(), None, "Idle 是循环动作");
        assert!(bridge_dyn.request_transition(ActionTransition {
            from: None,
            to: ActionState::Idle,
            spec: TransitionSpec::default(),
        }));
        assert!(bridge_dyn.request_transition(ActionTransition {
            from: Some(ActionState::Idle),
            to: ActionState::Attack(ActionId::new(1)),
            spec: TransitionSpec::one_shot(),
        }));
    }

    bridge.update(0.25);
    let remaining = bridge.state_remaining().expect("攻击为一次性动作");
    assert!((remaining - 0.75).abs() < 1e-3, "remaining={remaining}");
}

// ---------------------------------------------------------------------------
// 9. Land / Dodge / Block（扩展未合入时的兼容映射）
// ---------------------------------------------------------------------------

#[test]
fn land_dodge_block_clips_apply_through_compat_or_extended_mapping() {
    let mut adapter = AnimationActionAdapter::new();
    adapter
        .set_land_clip(translation_clip("land", 0.3))
        .set_dodge_clip(translation_clip("dodge", 0.4))
        .set_block_clip(translation_clip("block", 1.0));

    // 落地：一次性动作。
    assert!(adapter.request_one_shot(ActionState::Land));
    assert_eq!(adapter.current_state(), map_action_state(ActionState::Land));
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(adapter.current_state())
            .unwrap()
            .name,
        "land"
    );

    // 闪避：一次性动作，写入自己的虚拟槽位。
    assert!(adapter.request_one_shot(ActionState::Dodge));
    assert_eq!(
        adapter.current_state(),
        map_action_state(ActionState::Dodge)
    );
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(adapter.current_state())
            .unwrap()
            .name,
        "dodge"
    );

    // 格挡：保持型动作（不重启），默认循环。
    assert!(adapter.request(ActionState::Block));
    assert_eq!(
        adapter.current_state(),
        map_action_state(ActionState::Block)
    );
    assert_eq!(
        adapter
            .bridge()
            .machine()
            .clip(adapter.current_state())
            .unwrap()
            .name,
        "block"
    );
    assert_eq!(adapter.state_remaining(), None, "格挡是保持型/循环动作");
    assert_eq!(
        action_id(ActionState::Block),
        reserved_action_ids::BLOCK,
        "格挡事件使用保留 ActionId"
    );
}

// ---------------------------------------------------------------------------
// 10. 采样与每帧流程
// ---------------------------------------------------------------------------

#[test]
fn adapter_samples_pose_with_transition_and_events() {
    let skeleton = skeleton();
    let mut adapter = AnimationActionAdapter::with_default_transition(0.2);
    adapter.set_clip(AnimationState::Idle, translation_clip("idle", 1.0));
    adapter.set_action_clip(
        ActionId::new(3),
        clip_with_events("attack_3", 1.0, &[(0.5, "attack_hit")]),
    );

    assert!(adapter.request(ActionState::Idle));
    adapter.update(0.5);
    let pose = adapter.sample(&skeleton);
    assert!((pose.locals[0].translation.x - 0.5).abs() < 1e-4);

    // 过渡中姿态是两段 clip 的混合；过渡完成后进入攻击片段。
    assert!(adapter.request_one_shot(ActionState::Attack(ActionId::new(3))));
    let mid = adapter.update(0.02);
    assert!(adapter.is_transitioning());
    assert!(adapter.transition_alpha() > 0.0 && adapter.transition_alpha() < 1.0);
    assert!(!mid
        .iter()
        .any(|event| matches!(event, AnimationEvent::HitFrame { .. })));

    // 推进到攻击片段 0.62s：命中帧（0.5s）应透传。
    let events = adapter.update(0.6);
    assert!(!adapter.is_transitioning(), "过渡应已结束");
    assert!(events.contains(&AnimationEvent::HitFrame {
        action: ActionId::new(3),
        hit_index: 0,
    }));
    let pose = adapter.sample(&skeleton);
    assert!(
        pose.locals[0].translation.x > 0.3,
        "进入攻击片段后姿态应继续推进"
    );
    assert_eq!(
        action_id(ActionState::Attack(ActionId::new(3))),
        ActionId::new(3)
    );
}
