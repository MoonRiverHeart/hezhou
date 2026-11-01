//! AnimationStateMachine：9 动作状态（六个基础状态 + Land/Dodge/Block）+ 过渡 + 事件。
//!
//! 定位：这是**引擎级通用状态机**，只负责“状态 → 片段 → 姿态”和过渡/事件派发，
//! 不包含任何具体游戏规则（连招判定、伤害、AI 决策属于 gameplay 模块）。
//!
//! 关键行为：
//! - 基础状态（0.2 原有）：Idle/Run/Jump/Attack/Hit/Death；
//! - 动作扩展状态（task-11 加法式新增）：Land/Dodge/Block，与旧状态完全兼容；
//! - 每个状态可绑定 0 或 1 个 clip；状态切换带交叉淡入淡出（`Pose::blend`），过渡时长可配；
//! - `Death` 是终止状态：可以进入，但不能从 Death 切换到其它状态；
//! - 事件分两类：状态机事件（Enter/Exit/TransitionStarted/TransitionFinished）与
//!   clip 事件（脚步/命中帧等，原样透传给玩法层）；
//! - 过渡期间旧状态的时间继续推进（保证交叉淡出的旧动画也在动）。

use crate::clip::{AnimationClip, AnimationEvent};
use crate::pose::Pose;
use crate::skeleton::Skeleton;

/// 0.2 动作状态集：六个基础状态 + Land/Dodge/Block 三个动作扩展状态。
///
/// 兼容性说明：新增的三个变体追加在枚举末尾，`ALL` 的前 6 项顺序保持不变，
/// 旧调用方按名称/顺序使用基础 6 状态的行为不变。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimationState {
    Idle,
    Run,
    Jump,
    Attack,
    Hit,
    Death,
    /// 落地（一次性动作，非循环）。
    Land,
    /// 闪避（一次性动作，非循环）。
    Dodge,
    /// 格挡（一次性/持续由集成层选择，默认非循环）。
    Block,
}

impl AnimationState {
    /// 旧 0.2 基础状态（顺序与 task-6 完全一致，供兼容性判断/UI 使用）。
    pub const BASE_STATES: [AnimationState; 6] = [
        AnimationState::Idle,
        AnimationState::Run,
        AnimationState::Jump,
        AnimationState::Attack,
        AnimationState::Hit,
        AnimationState::Death,
    ];

    /// 全部状态：前 6 项为旧基础状态，后 3 项为 Land/Dodge/Block 扩展。
    ///
    /// 顺序固定，便于 UI/测试遍历与集成层按索引映射。
    pub const ALL: [AnimationState; 9] = [
        AnimationState::Idle,
        AnimationState::Run,
        AnimationState::Jump,
        AnimationState::Attack,
        AnimationState::Hit,
        AnimationState::Death,
        AnimationState::Land,
        AnimationState::Dodge,
        AnimationState::Block,
    ];

    /// 稳定名称（用于日志/C# FFI/资源映射）。
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Run => "Run",
            Self::Jump => "Jump",
            Self::Attack => "Attack",
            Self::Hit => "Hit",
            Self::Death => "Death",
            Self::Land => "Land",
            Self::Dodge => "Dodge",
            Self::Block => "Block",
        }
    }

    /// 由名称解析（大小写不敏感）；供 C#/配置字符串使用。
    ///
    /// `Land`/`Dodge`/`Block` 同样可解析，例如集成层把 gameplay 的
    /// `ActionState::Land/Dodge/Block` 直接映射过来。
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|state| state.name().eq_ignore_ascii_case(name))
    }

    /// 是否为终止状态（进入后不再切出）；仅 `Death` 终止。
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Death)
    }

    /// 该状态的默认循环策略：Idle/Run 循环，其余（含 Land/Dodge/Block）一次性动作不循环。
    pub fn default_looping(self) -> bool {
        matches!(self, Self::Idle | Self::Run)
    }
}

/// 过渡曲线。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransitionCurve {
    /// 线性交叉淡入淡出。
    Linear,
    /// 平滑（smoothstep）交叉淡入淡出（默认，动作切换更自然）。
    #[default]
    SmoothStep,
}

impl TransitionCurve {
    /// 把线性进度映射为混合权重。
    pub fn evaluate(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::SmoothStep => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// 一次进行中的过渡。
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    /// 来源状态。
    pub from: AnimationState,
    /// 目标状态。
    pub to: AnimationState,
    /// 过渡总时长（秒）。
    pub duration: f32,
    /// 已进行时间（秒）。
    pub elapsed: f32,
    /// 过渡曲线。
    pub curve: TransitionCurve,
}

impl Transition {
    pub fn new(
        from: AnimationState,
        to: AnimationState,
        duration: f32,
        curve: TransitionCurve,
    ) -> Self {
        Self {
            from,
            to,
            duration: duration.max(0.0),
            elapsed: 0.0,
            curve,
        }
    }

    /// 混合权重：0 = 完全旧状态，1 = 完全新状态。
    pub fn alpha(&self) -> f32 {
        if self.duration <= f32::EPSILON {
            return 1.0;
        }
        self.curve.evaluate(self.elapsed / self.duration)
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration - f32::EPSILON
    }
}

/// 单个状态的配置。
#[derive(Clone, Debug, PartialEq)]
pub struct StateEntry {
    pub state: AnimationState,
    /// 绑定的动画片段；为 `None` 时采样骨架绑定姿态。
    pub clip: Option<AnimationClip>,
    /// 该片段是否循环。
    pub looping: bool,
    /// 播放速度倍率。
    pub speed: f32,
}

impl StateEntry {
    pub fn new(state: AnimationState) -> Self {
        Self {
            state,
            clip: None,
            looping: state.default_looping(),
            speed: 1.0,
        }
    }

    pub fn duration(&self) -> f32 {
        self.clip.as_ref().map(|c| c.duration).unwrap_or(0.0)
    }
}

/// 状态机派发的事件。
#[derive(Clone, Debug, PartialEq)]
pub enum StateEvent {
    /// 进入某状态（带过渡时在过渡开始时派发，保证玩法能立刻响应）。
    Enter(AnimationState),
    /// 离开某状态。
    Exit(AnimationState),
    /// 开始一次过渡。
    TransitionStarted {
        from: AnimationState,
        to: AnimationState,
        duration: f32,
    },
    /// 过渡完成。
    TransitionFinished { state: AnimationState },
    /// clip 内事件（脚步/命中帧等），原样透传。
    ClipEvent {
        state: AnimationState,
        event: AnimationEvent,
    },
}

/// 9 状态骨骼动画状态机（基础 6 状态 + Land/Dodge/Block）。
#[derive(Clone, Debug)]
pub struct AnimationStateMachine {
    states: Vec<StateEntry>,
    current: AnimationState,
    /// 当前状态内时间（秒）。
    time: f32,
    /// 过渡中旧状态的时间（秒）。
    previous_time: f32,
    transition: Option<Transition>,
    /// 默认过渡时长（秒）。
    default_transition_duration: f32,
    /// 默认过渡曲线。
    default_curve: TransitionCurve,
    /// 未被 `update`/`drain_events` 取走的事件。
    pending_events: Vec<StateEvent>,
}

impl Default for AnimationStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationStateMachine {
    /// 创建状态机：全部 9 个状态均已注册（基础 6 + Land/Dodge/Block），
    /// 但未绑定 clip（采样时回退绑定姿态）。
    pub fn new() -> Self {
        Self {
            states: AnimationState::ALL
                .into_iter()
                .map(StateEntry::new)
                .collect(),
            current: AnimationState::Idle,
            time: 0.0,
            previous_time: 0.0,
            transition: None,
            default_transition_duration: 0.2,
            default_curve: TransitionCurve::SmoothStep,
            pending_events: Vec::new(),
        }
    }

    /// 设置默认过渡时长（秒）与曲线。
    pub fn with_default_transition(mut self, duration: f32, curve: TransitionCurve) -> Self {
        self.default_transition_duration = duration.max(0.0);
        self.default_curve = curve;
        self
    }

    /// 绑定状态片段（使用状态默认循环策略与 1.0 速度）。
    pub fn set_clip(&mut self, state: AnimationState, clip: AnimationClip) {
        let looping = state.default_looping();
        self.set_clip_with(state, clip, looping, 1.0);
    }

    /// 绑定状态片段并指定循环/速度。
    pub fn set_clip_with(
        &mut self,
        state: AnimationState,
        clip: AnimationClip,
        looping: bool,
        speed: f32,
    ) {
        if let Some(entry) = self.states.iter_mut().find(|e| e.state == state) {
            entry.clip = Some(clip);
            entry.looping = looping;
            entry.speed = speed.max(0.0);
        }
    }

    /// 解除状态片段绑定。
    pub fn clear_clip(&mut self, state: AnimationState) {
        if let Some(entry) = self.states.iter_mut().find(|e| e.state == state) {
            entry.clip = None;
        }
    }

    pub fn entry(&self, state: AnimationState) -> Option<&StateEntry> {
        self.states.iter().find(|e| e.state == state)
    }

    pub fn clip(&self, state: AnimationState) -> Option<&AnimationClip> {
        self.entry(state).and_then(|e| e.clip.as_ref())
    }

    #[inline]
    pub fn current_state(&self) -> AnimationState {
        self.current
    }

    #[inline]
    pub fn current_time(&self) -> f32 {
        self.time
    }

    /// 当前状态归一化进度（0..1；无 clip 返回 0）。
    pub fn normalized_time(&self) -> f32 {
        let duration = self
            .entry(self.current)
            .map(|e| e.duration())
            .unwrap_or(0.0);
        if duration > 0.0 {
            (self.time / duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    #[inline]
    pub fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }

    #[inline]
    pub fn transition(&self) -> Option<&Transition> {
        self.transition.as_ref()
    }

    /// 当前过渡权重（0..1；无过渡时为 1）。
    pub fn transition_alpha(&self) -> f32 {
        self.transition.as_ref().map(|t| t.alpha()).unwrap_or(1.0)
    }

    /// 判断能否切换到目标状态。
    ///
    /// 规则（0.2）：
    /// - 同状态重复请求返回 `false`（需要重播请调用 `restart`）；
    /// - `Death` 为终止状态，不能切出；
    /// - 过渡进行中允许打断，但目标与当前过渡目标相同则忽略。
    pub fn can_transition_to(&self, state: AnimationState) -> bool {
        if self.current.is_terminal() {
            return false;
        }
        if let Some(transition) = &self.transition {
            if transition.to == state {
                return false;
            }
        } else if state == self.current {
            return false;
        }
        true
    }

    /// 以默认过渡时长切换状态；成功返回 `true`。
    pub fn request_state(&mut self, state: AnimationState) -> bool {
        self.transition_to(state, self.default_transition_duration)
    }

    /// 以指定过渡时长切换状态（`duration <= 0` 为瞬时切换）。
    pub fn transition_to(&mut self, state: AnimationState, duration: f32) -> bool {
        if !self.can_transition_to(state) {
            return false;
        }
        let from = self.current;
        self.pending_events.push(StateEvent::Exit(from));
        self.previous_time = self.time;
        self.time = 0.0;
        self.current = state;
        if duration <= 0.0 {
            self.transition = None;
            self.pending_events.push(StateEvent::Enter(state));
        } else {
            let curve = self.default_curve;
            self.transition = Some(Transition::new(from, state, duration, curve));
            self.pending_events.push(StateEvent::TransitionStarted {
                from,
                to: state,
                duration,
            });
            self.pending_events.push(StateEvent::Enter(state));
        }
        true
    }

    /// 立即重置到指定状态（不走过渡，清空进行中的过渡）。
    pub fn reset_to(&mut self, state: AnimationState) {
        let from = self.current;
        self.pending_events.push(StateEvent::Exit(from));
        self.transition = None;
        self.previous_time = 0.0;
        self.time = 0.0;
        self.current = state;
        self.pending_events.push(StateEvent::Enter(state));
    }

    /// 重播当前状态（时间归零），不影响进行中的过渡。
    pub fn restart(&mut self) {
        self.time = 0.0;
    }

    /// 推进状态机 `dt` 秒，返回自上次调用以来产生的事件。
    ///
    /// 事件顺序：接口调用期间累积的事件（Exit/TransitionStarted/Enter）→
    /// 当前 clip 事件 → 过渡完成事件。
    pub fn update(&mut self, dt: f32) -> Vec<StateEvent> {
        let mut events = std::mem::take(&mut self.pending_events);
        if dt > 0.0 {
            // 1) 推进当前状态时间并派发 clip 事件。
            if let Some(entry) = self.entry(self.current).cloned() {
                let start = self.time;
                let end = start + dt * entry.speed;
                if let Some(clip) = &entry.clip {
                    for event in clip.events_in_range(start, end, entry.looping) {
                        events.push(StateEvent::ClipEvent {
                            state: self.current,
                            event,
                        });
                    }
                }
                self.time = if entry.looping {
                    let duration = entry.duration();
                    if duration > 0.0 {
                        end.rem_euclid(duration)
                    } else {
                        0.0
                    }
                } else {
                    let duration = entry.duration();
                    if duration > 0.0 {
                        end.min(duration)
                    } else {
                        0.0
                    }
                };
            }
            // 2) 推进过渡：旧状态时间同步前进，保证交叉淡出的旧动画也在动。
            if let Some(transition) = self.transition.as_mut() {
                let from_entry = self
                    .states
                    .iter()
                    .find(|e| e.state == transition.from)
                    .cloned();
                if let Some(entry) = from_entry {
                    self.previous_time += dt * entry.speed;
                    let duration = entry.duration();
                    if entry.looping && duration > 0.0 {
                        self.previous_time = self.previous_time.rem_euclid(duration);
                    } else if duration > 0.0 && entry.speed > 0.0 {
                        self.previous_time = self.previous_time.min(duration);
                    }
                }
                transition.elapsed += dt;
                if transition.is_finished() {
                    let finished = transition.to;
                    self.transition = None;
                    events.push(StateEvent::TransitionFinished { state: finished });
                }
            }
        }
        events
    }

    /// 采样当前姿态（过渡期间为两段姿态的交叉淡入淡出结果）。
    pub fn sample(&self, skeleton: &Skeleton) -> Pose {
        let current_pose = self.sample_state(self.current, self.time, skeleton);
        match &self.transition {
            None => current_pose,
            Some(transition) => {
                let previous_pose =
                    self.sample_state(transition.from, self.previous_time, skeleton);
                Pose::blend(&previous_pose, &current_pose, transition.alpha())
            }
        }
    }

    /// 采样指定状态的片段（无 clip 时返回绑定姿态）。
    pub fn sample_state(&self, state: AnimationState, time: f32, skeleton: &Skeleton) -> Pose {
        match self.entry(state).and_then(|e| e.clip.as_ref()) {
            Some(clip) => {
                let looping = self.entry(state).map(|e| e.looping).unwrap_or(false);
                clip.sample(time, looping, skeleton)
            }
            None => Pose::from_locals(skeleton.default_pose()),
        }
    }

    /// 取出尚未消费的事件。
    pub fn drain_events(&mut self) -> Vec<StateEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

/// gameplay → animation 的最小桥接接口。
///
/// gameplay 模块只依赖这个 trait，不依赖状态机内部实现；
/// `AnimationStateMachine` 已经实现了它，可直接作为 `dyn AnimationStateDriver` 使用。
pub trait AnimationStateDriver {
    /// 请求切换到目标状态；返回是否接受。
    fn request_state(&mut self, state: AnimationState) -> bool;
    /// 当前状态。
    fn current_state(&self) -> AnimationState;
    /// 是否正在过渡。
    fn is_transitioning(&self) -> bool;
}

impl AnimationStateDriver for AnimationStateMachine {
    fn request_state(&mut self, state: AnimationState) -> bool {
        AnimationStateMachine::request_state(self, state)
    }

    fn current_state(&self) -> AnimationState {
        self.current
    }

    fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{Channel, Keyframe};
    use crate::math::{Transform, Vec3};
    use crate::skeleton::Skeleton;

    fn skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("s");
        skeleton.add_root_joint("root", Transform::IDENTITY);
        skeleton
    }

    fn translation_clip(name: &str, end_x: f32) -> AnimationClip {
        let mut clip = AnimationClip::new(name, 1.0);
        let mut channel = Channel::translation(0);
        channel
            .add_key(Keyframe::translation(0.0, Vec3::ZERO))
            .unwrap();
        channel
            .add_key(Keyframe::translation(1.0, Vec3::new(end_x, 0.0, 0.0)))
            .unwrap();
        clip.add_channel(channel);
        clip
    }

    /// 数值恒定的平移片段（便于验证交叉淡入淡出的权重）。
    fn constant_translation_clip(name: &str, x: f32) -> AnimationClip {
        let mut clip = AnimationClip::new(name, 1.0);
        let mut channel = Channel::translation(0);
        channel
            .add_key(Keyframe::translation(0.0, Vec3::new(x, 0.0, 0.0)))
            .unwrap();
        channel
            .add_key(Keyframe::translation(1.0, Vec3::new(x, 0.0, 0.0)))
            .unwrap();
        clip.add_channel(channel);
        clip
    }

    #[test]
    fn state_machine_blends_transition_and_finishes() {
        let skeleton = skeleton();
        let mut machine =
            AnimationStateMachine::new().with_default_transition(0.4, TransitionCurve::Linear);
        machine.set_clip(AnimationState::Idle, constant_translation_clip("idle", 0.0));
        machine.set_clip(AnimationState::Run, constant_translation_clip("run", 10.0));
        // 进入 Run：过渡期间 Idle 与 Run 按 alpha 混合。
        assert!(machine.request_state(AnimationState::Run));
        let events = machine.update(0.2);
        assert!(events
            .iter()
            .any(|e| matches!(e, StateEvent::Enter(AnimationState::Run))));
        assert!(events
            .iter()
            .any(|e| matches!(e, StateEvent::TransitionStarted { .. })));
        assert!((machine.transition_alpha() - 0.5).abs() < 1e-5);
        let pose = machine.sample(&skeleton);
        assert!(pose.locals[0]
            .translation
            .approx_eq(Vec3::new(5.0, 0.0, 0.0), 1e-4));
        let events = machine.update(0.25);
        assert!(events.iter().any(|e| matches!(
            e,
            StateEvent::TransitionFinished {
                state: AnimationState::Run
            }
        )));
        assert!(!machine.is_transitioning());
        assert_eq!(machine.current_state(), AnimationState::Run);
    }

    #[test]
    fn state_machine_requires_distinct_state_and_respects_death_terminal() {
        let mut machine = AnimationStateMachine::new();
        assert!(
            !machine.request_state(AnimationState::Idle),
            "同状态重复请求应被拒绝"
        );
        assert!(machine.request_state(AnimationState::Attack));
        machine.update(0.01);
        assert!(machine.request_state(AnimationState::Death));
        machine.update(1.0);
        assert_eq!(machine.current_state(), AnimationState::Death);
        assert!(
            !machine.request_state(AnimationState::Idle),
            "Death 不应切出"
        );
        assert!(!machine.can_transition_to(AnimationState::Run));
    }

    #[test]
    fn state_machine_emits_exit_enter_and_clip_events() {
        let mut machine =
            AnimationStateMachine::new().with_default_transition(0.0, TransitionCurve::Linear);
        let mut attack = translation_clip("attack", 1.0);
        attack.add_event(AnimationEvent::new(0.1, "attack_hit"));
        machine.set_clip_with(AnimationState::Attack, attack, false, 1.0);
        assert!(machine.request_state(AnimationState::Attack));
        let events = machine.update(0.05);
        assert!(events
            .iter()
            .any(|e| matches!(e, StateEvent::Exit(AnimationState::Idle))));
        assert!(events
            .iter()
            .any(|e| matches!(e, StateEvent::Enter(AnimationState::Attack))));
        let events = machine.update(0.1);
        assert!(events.iter().any(|e| matches!(
            e,
            StateEvent::ClipEvent { event, .. } if event.name == "attack_hit"
        )));
    }

    #[test]
    fn state_machine_driver_trait_delegates() {
        let mut machine = AnimationStateMachine::new();
        let driver: &mut dyn AnimationStateDriver = &mut machine;
        assert!(driver.request_state(AnimationState::Jump));
        assert_eq!(driver.current_state(), AnimationState::Jump);
        assert!(driver.is_transitioning());
    }

    #[test]
    fn new_states_are_additive_and_non_looping() {
        // 旧 6 状态顺序与语义保持不变，新状态只追加在后面。
        assert_eq!(AnimationState::ALL.len(), 9);
        assert_eq!(AnimationState::BASE_STATES.len(), 6);
        assert_eq!(AnimationState::ALL[..6], AnimationState::BASE_STATES);
        assert_eq!(
            &AnimationState::ALL[6..],
            &[
                AnimationState::Land,
                AnimationState::Dodge,
                AnimationState::Block
            ]
        );
        for state in AnimationState::ALL {
            // 名称往返稳定（集成层按名称映射）。
            assert_eq!(AnimationState::from_name(state.name()), Some(state));
            // 只有 Death 终止。
            assert_eq!(state.is_terminal(), state == AnimationState::Death);
            // 只有 Idle/Run 循环；Land/Dodge/Block 默认一次性。
            assert_eq!(
                state.default_looping(),
                matches!(state, AnimationState::Idle | AnimationState::Run),
                "state={}",
                state.name()
            );
        }
        assert!(!AnimationState::Land.default_looping());
        assert!(!AnimationState::Dodge.default_looping());
        assert!(!AnimationState::Block.default_looping());
        // 大小写不敏感解析（集成层可能来自 C#/配置字符串）。
        assert_eq!(
            AnimationState::from_name("land"),
            Some(AnimationState::Land)
        );
        assert_eq!(
            AnimationState::from_name("DODGE"),
            Some(AnimationState::Dodge)
        );
        assert_eq!(
            AnimationState::from_name("block"),
            Some(AnimationState::Block)
        );
        assert_eq!(AnimationState::from_name("unknown"), None);
    }

    #[test]
    fn state_machine_supports_land_dodge_block_transitions() {
        let skeleton = skeleton();
        let mut machine =
            AnimationStateMachine::new().with_default_transition(0.0, TransitionCurve::Linear);
        machine.set_clip(AnimationState::Land, constant_translation_clip("land", 1.0));
        machine.set_clip(
            AnimationState::Dodge,
            constant_translation_clip("dodge", 2.0),
        );
        machine.set_clip(
            AnimationState::Block,
            constant_translation_clip("block", 3.0),
        );

        assert!(machine.request_state(AnimationState::Land));
        let events = machine.update(0.01);
        assert!(events
            .iter()
            .any(|event| matches!(event, StateEvent::Enter(AnimationState::Land))));
        assert_eq!(machine.current_state(), AnimationState::Land);
        let pose = machine.sample(&skeleton);
        assert!(pose.locals[0]
            .translation
            .approx_eq(Vec3::new(1.0, 0.0, 0.0), 1e-5));

        assert!(machine.request_state(AnimationState::Dodge));
        machine.update(0.01);
        assert_eq!(machine.current_state(), AnimationState::Dodge);
        let pose = machine.sample(&skeleton);
        assert!(pose.locals[0]
            .translation
            .approx_eq(Vec3::new(2.0, 0.0, 0.0), 1e-5));

        assert!(machine.request_state(AnimationState::Block));
        machine.update(0.01);
        assert_eq!(machine.current_state(), AnimationState::Block);
        let pose = machine.sample(&skeleton);
        assert!(pose.locals[0]
            .translation
            .approx_eq(Vec3::new(3.0, 0.0, 0.0), 1e-5));

        // 新状态的默认循环策略同步到状态表项。
        assert!(!machine.entry(AnimationState::Land).unwrap().looping);
        assert!(!machine.entry(AnimationState::Dodge).unwrap().looping);
        assert!(!machine.entry(AnimationState::Block).unwrap().looping);
    }

    #[test]
    fn death_remains_terminal_after_new_states() {
        let mut machine = AnimationStateMachine::new();
        assert!(machine.request_state(AnimationState::Block));
        machine.update(1.0);
        assert!(machine.request_state(AnimationState::Death));
        machine.update(1.0);
        assert_eq!(machine.current_state(), AnimationState::Death);
        assert!(!machine.request_state(AnimationState::Land));
        assert!(!machine.request_state(AnimationState::Dodge));
        assert!(!machine.can_transition_to(AnimationState::Block));
    }

    #[test]
    fn old_six_states_regression() {
        // 旧状态名称/循环/终止语义保持与 task-6 完全一致。
        let expected = [
            ("Idle", true),
            ("Run", true),
            ("Jump", false),
            ("Attack", false),
            ("Hit", false),
            ("Death", false),
        ];
        for (state, (name, looping)) in AnimationState::BASE_STATES.iter().zip(expected) {
            assert_eq!(state.name(), name);
            assert_eq!(state.default_looping(), looping);
            assert_eq!(AnimationState::from_name(name), Some(*state));
        }
        // 旧 6 状态之间仍可正常切换。
        let mut machine = AnimationStateMachine::new();
        assert!(machine.request_state(AnimationState::Run));
        machine.update(1.0);
        assert!(machine.request_state(AnimationState::Attack));
        machine.update(1.0);
        assert!(machine.request_state(AnimationState::Hit));
        machine.update(1.0);
        assert!(machine.request_state(AnimationState::Death));
    }
}
