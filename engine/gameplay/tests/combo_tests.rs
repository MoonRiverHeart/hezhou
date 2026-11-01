//! ComboGraph / ComboState：时间轴、连招窗口、预输入、结束与动画状态映射。

mod support;

use hezhou_gameplay::{
    ActionId, ActionState, AttackDefinition, AttackId, AttackKind, AttackPhase, AttackWindow,
    ComboConfig, ComboGraph, ComboInputResult, ComboState, DamageKind, Vec3,
};

const LIGHT_1: AttackId = AttackId(1);
const LIGHT_2: AttackId = AttackId(2);
const HEAVY: AttackId = AttackId(3);

/// 测试用连招图：
/// - LIGHT_1（可起手）→ LIGHT_2；
/// - LIGHT_2（仅连招）→ HEAVY；
/// - HEAVY 为独立起手重击。
fn test_graph() -> ComboGraph {
    let mut graph = ComboGraph::new();
    graph.add_attack(
        AttackDefinition::new(LIGHT_1, AttackKind::Light)
            .with_timing(0.12, 0.1, 0.25)
            .with_next([LIGHT_2])
            .with_damage(10.0, DamageKind::Light)
            .with_forward_push(2.0),
    );
    graph.add_attack(
        AttackDefinition::new(LIGHT_2, AttackKind::Light)
            .with_timing(0.1, 0.08, 0.2)
            .with_next([HEAVY])
            .with_damage(12.0, DamageKind::Light)
            .as_combo_only(),
    );
    graph.add_attack(
        AttackDefinition::new(HEAVY, AttackKind::Heavy)
            .with_timing(0.3, 0.15, 0.4)
            .with_damage(30.0, DamageKind::Heavy)
            .with_unblockable(true),
    );
    graph
}

fn start_light_1(state: &mut ComboState, graph: &ComboGraph) {
    let result = state.press(LIGHT_1, graph, &ComboConfig::default());
    assert_eq!(result, ComboInputResult::Started(LIGHT_1));
}

#[test]
fn attack_phases_follow_timeline() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);

    let early = state.update(0.05, &graph);
    assert_eq!(early.phase, Some(AttackPhase::Startup));
    assert!(!early.hitbox_active);

    let active = state.update(0.1, &graph); // 累计 0.15，处于 0.12..0.22
    assert_eq!(active.phase, Some(AttackPhase::Active));
    assert!(active.hitbox_active, "判定窗口内应生效");
    assert!(active.hitbox.is_some());

    let recovery = state.update(0.15, &graph); // 累计 0.30，处于 0.22..0.47
    assert_eq!(recovery.phase, Some(AttackPhase::Recovery));
    assert!(!recovery.hitbox_active);

    let finished = state.update(0.3, &graph); // 累计 0.60 > 0.47
    assert_eq!(finished.finished, Some(LIGHT_1));
    assert_eq!(finished.action, None);
    assert!(!state.is_attacking());
}

#[test]
fn hitbox_active_only_during_active_window() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);

    let definition = graph.attack(LIGHT_1).unwrap();
    let window = definition.hitbox_window();
    assert_eq!(window, AttackWindow::new(0.12, 0.22));

    let mut elapsed = 0.0;
    for _ in 0..40 {
        let update = state.update(0.01, &graph);
        elapsed += 0.01;
        let expected = window.contains(elapsed) && elapsed < window.end;
        assert_eq!(update.hitbox_active, expected, "t={elapsed} 时判定状态不符");
        if update.finished.is_some() {
            break;
        }
    }
}

#[test]
fn combo_input_inside_window_chains() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);

    // 累计到 0.20s（combo window 0.17..0.47 内）。
    state.update(0.2, &graph);
    let result = state.press(LIGHT_2, &graph, &ComboConfig::default());
    assert_eq!(result, ComboInputResult::Queued(LIGHT_2));

    let chained = state.update(0.0, &graph);
    assert_eq!(chained.chained, Some(LIGHT_2));
    assert_eq!(chained.action, Some(LIGHT_2));
    assert_eq!(state.current_attack(), Some(LIGHT_2));
}

#[test]
fn early_input_is_buffered_and_chains_at_window_start() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);
    state.update(0.13, &graph); // 窗口从 0.17 开始

    let result = state.press(LIGHT_2, &graph, &ComboConfig::default());
    assert_eq!(result, ComboInputResult::Queued(LIGHT_2), "预输入应被缓冲");

    let update = state.update(0.0, &graph);
    assert_eq!(update.chained, None, "窗口尚未开始，不应立刻衔接");
    assert_eq!(state.current_attack(), Some(LIGHT_1));

    let update = state.update(0.1, &graph); // 累计 0.23 >= 0.17
    assert_eq!(update.chained, Some(LIGHT_2));
}

#[test]
fn combo_input_outside_window_is_rejected() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);

    // 刚起手，距离窗口开始（0.17）很远，且关闭预输入缓冲。
    let strict = ComboConfig {
        input_buffer_time: 0.0,
    };
    assert_eq!(
        state.press(LIGHT_2, &graph, &strict),
        ComboInputResult::Rejected
    );
    assert_eq!(state.current_attack(), Some(LIGHT_1));
}

#[test]
fn cannot_chain_to_unlisted_attack() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);
    state.update(0.2, &graph);

    // LIGHT_1 的 next_attacks 只有 LIGHT_2，不能直接接 HEAVY。
    assert_eq!(
        state.press(HEAVY, &graph, &ComboConfig::default()),
        ComboInputResult::Rejected
    );
    assert_eq!(state.current_attack(), Some(LIGHT_1));
}

#[test]
fn combo_only_attack_cannot_start_from_idle() {
    let graph = test_graph();
    let mut state = ComboState::new();
    assert_eq!(
        state.press(LIGHT_2, &graph, &ComboConfig::default()),
        ComboInputResult::Rejected,
        "非起手招式不能从待机直接起手"
    );
    assert!(!state.is_attacking());
}

#[test]
fn attack_can_restart_after_finishing() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);
    let update = state.update(0.6, &graph);
    assert_eq!(update.finished, Some(LIGHT_1));
    assert!(!state.is_attacking());

    assert_eq!(
        state.press(LIGHT_1, &graph, &ComboConfig::default()),
        ComboInputResult::Started(LIGHT_1)
    );
}

#[test]
fn heavy_attack_maps_to_heavy_animation_state() {
    let graph = test_graph();
    let mut state = ComboState::new();
    assert_eq!(
        state.press(HEAVY, &graph, &ComboConfig::default()),
        ComboInputResult::Started(HEAVY)
    );
    assert_eq!(
        state.action_state(&graph),
        Some(ActionState::HeavyAttack(ActionId(3)))
    );

    let light_graph = test_graph();
    let mut light_state = ComboState::new();
    start_light_1(&mut light_state, &light_graph);
    assert_eq!(
        light_state.action_state(&light_graph),
        Some(ActionState::Attack(ActionId(1)))
    );
}

#[test]
fn forward_push_provides_motor_external_velocity() {
    let graph = test_graph();
    let mut state = ComboState::new();
    start_light_1(&mut state, &graph);

    let velocity = state.forward_push_velocity(&graph, Vec3::FORWARD);
    assert!(
        (velocity - Vec3::new(0.0, 0.0, -2.0)).magnitude() < 1e-4,
        "前冲速度应按面朝方向生成，实际 {velocity:?}"
    );

    // 没有攻击时不应有位移。
    let mut idle = ComboState::new();
    assert!(
        idle.forward_push_velocity(&graph, Vec3::FORWARD)
            .magnitude()
            < 1e-4
    );
    idle.cancel();
}
