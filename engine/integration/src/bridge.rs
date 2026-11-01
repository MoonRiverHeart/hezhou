//! `AnimationActionBridge`：gameplay `ActionStateBridge` / `AnimationEventSource` 的
//! animation 侧实现，内部持有 `AnimationStateMachine`。
//!
//! 职责边界：
//! - **gameplay → animation**：把 [`ActionState`] 映射为 [`AnimationState`]，
//!   在切换前按"逐招式 / 虚拟槽位 / 状态槽位"重绑 clip（这就是"逐招式 clip"的实现）；
//! - **animation → gameplay**：把状态机事件映射为 gameplay `AnimationEvent`，
//!   通过 `update` 返回并缓冲，供 `drain_events` / `AnimationEventSource` 取走；
//! - **特殊规则**：同状态重播（restart）、过渡中请求同一目标、Death 终止。
//!
//! 需要采样姿态/生成 GPU palette 时用 [`AnimationActionAdapter`](crate::AnimationActionAdapter)，
//! 它在本类型之上提供每帧 update/sample 的便捷接口。

use hezhou_animation::{
    AnimationClip, AnimationState, AnimationStateMachine, Pose, Skeleton, StateEvent,
    TransitionCurve,
};
use hezhou_gameplay::{
    ActionId, ActionState, ActionStateBridge, ActionTransition, AnimationEvent,
    AnimationEventSource, TransitionSpec,
};

use crate::clip_registry::{ActionClipRegistry, ClipBinding, VirtualSlot};
use crate::event_map::AnimationEventMapper;
use crate::mapping::{action_id, map_action_state, reserved_action_ids};

/// animation 侧的动作状态桥接器。
#[derive(Clone, Debug)]
pub struct AnimationActionBridge {
    machine: AnimationStateMachine,
    clips: ActionClipRegistry,
    mapper: AnimationEventMapper,
    /// 便捷 `request` 使用的默认过渡参数（gameplay 的 `ActionAnimator` 会自带 spec）。
    default_spec: TransitionSpec,
    /// 最近一次同步的逻辑动作状态（用于识别"同一动作重启/不同招式切换"）。
    current_action: Option<ActionState>,
    /// 尚未被 `update` / `drain` 取走的 gameplay 事件。
    pending_events: Vec<AnimationEvent>,
    /// 原始状态机事件（含 TransitionFinished 等 gameplay 无对应枚举的事件）。
    raw_events: Vec<StateEvent>,
}

impl Default for AnimationActionBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationActionBridge {
    /// 创建桥接器：使用 animation 状态机默认过渡（0.2s smoothstep）。
    pub fn new() -> Self {
        Self {
            machine: AnimationStateMachine::new(),
            clips: ActionClipRegistry::new(),
            mapper: AnimationEventMapper::new(),
            default_spec: TransitionSpec::default(),
            current_action: None,
            pending_events: Vec::new(),
            raw_events: Vec::new(),
        }
    }

    /// 创建桥接器并指定默认过渡时长（秒）。
    pub fn with_default_transition(mut self, duration: f32) -> Self {
        let duration = duration.max(0.0);
        self.machine = AnimationStateMachine::new()
            .with_default_transition(duration, TransitionCurve::SmoothStep);
        self.default_spec = TransitionSpec {
            fade_seconds: duration,
            restart: false,
        };
        self
    }

    // ---- 访问器 -------------------------------------------------------

    /// 内部状态机（只读），用于采样、调试或绑定额外 clip。
    pub fn machine(&self) -> &AnimationStateMachine {
        &self.machine
    }

    /// 内部状态机（可变），用于高级用法（例如直接 `set_clip_with`）。
    pub fn machine_mut(&mut self) -> &mut AnimationStateMachine {
        &mut self.machine
    }

    /// clip 注册表（只读）。
    pub fn clips(&self) -> &ActionClipRegistry {
        &self.clips
    }

    /// clip 注册表（可变）。
    pub fn clips_mut(&mut self) -> &mut ActionClipRegistry {
        &mut self.clips
    }

    /// 事件映射器（只读）。
    pub fn mapper(&self) -> &AnimationEventMapper {
        &self.mapper
    }

    /// 事件映射器（可变），用于注册自定义 clip 事件规则。
    pub fn mapper_mut(&mut self) -> &mut AnimationEventMapper {
        &mut self.mapper
    }

    /// 最近一次同步的逻辑动作状态（尚未同步时为 `None`）。
    pub fn current_action(&self) -> Option<ActionState> {
        self.current_action
    }

    /// 当前 animation 状态。
    pub fn current_animation_state(&self) -> AnimationState {
        self.machine.current_state()
    }

    /// 当前 animation 状态内时间（秒）。
    pub fn current_time(&self) -> f32 {
        self.machine.current_time()
    }

    /// 当前状态是否处于过渡中。
    pub fn is_transitioning(&self) -> bool {
        self.machine.is_transitioning()
    }

    /// 当前过渡权重（0..1；无过渡为 1）。
    pub fn transition_alpha(&self) -> f32 {
        self.machine.transition_alpha()
    }

    /// 是否已进入终止状态（`Death`）。
    pub fn is_terminal(&self) -> bool {
        self.machine.current_state().is_terminal()
    }

    // ---- clip 绑定（逐招式 / 状态 / 虚拟槽位） -----------------------

    /// 直接绑定 animation 状态 clip（兼容玩法：先 `set_clip` 再切换）。
    pub fn set_clip(&mut self, state: AnimationState, clip: AnimationClip) {
        self.machine.set_clip(state, clip);
    }

    /// 直接绑定 animation 状态 clip 并指定循环/速度。
    pub fn set_clip_with(
        &mut self,
        state: AnimationState,
        clip: AnimationClip,
        looping: bool,
        speed: f32,
    ) {
        self.machine.set_clip_with(state, clip, looping, speed);
    }

    /// 注册状态槽位 clip 绑定。
    pub fn set_state_clip(&mut self, state: AnimationState, binding: ClipBinding) {
        self.clips.set_state_clip(state, binding);
    }

    /// 注册逐招式攻击 clip 绑定（Attack / HeavyAttack 共用）。
    pub fn set_action_clip(&mut self, action: ActionId, binding: ClipBinding) {
        self.clips.set_action_clip(action, binding);
    }

    /// 注册 Land/Dodge/Block 虚拟槽位 clip 绑定。
    pub fn set_virtual_clip(&mut self, slot: VirtualSlot, binding: ClipBinding) {
        self.clips.set_virtual_clip(slot, binding);
    }

    // ---- 请求 / 推进 / 事件 -------------------------------------------

    /// 按默认过渡参数请求动作状态（一次性动作用 [`Self::request_one_shot`]）。
    pub fn request(&mut self, action: ActionState) -> bool {
        self.request_transition(ActionTransition {
            from: self.current_action,
            to: action,
            spec: self.default_spec,
        })
    }

    /// 请求一次性动作（短淡化 + 同动作重启）。
    pub fn request_one_shot(&mut self, action: ActionState) -> bool {
        self.request_transition(ActionTransition {
            from: self.current_action,
            to: action,
            spec: TransitionSpec::one_shot(),
        })
    }

    /// 推进状态机 `dt` 秒并返回本帧新映射出的 gameplay 事件。
    ///
    /// 返回的事件同时会从内部缓冲中取走；如需按 gameplay 的
    /// [`AnimationEventSource`] 接口拉取，可用 `update` 返回的 `Vec` 或 `drain_events`。
    pub fn update(&mut self, dt: f32) -> Vec<AnimationEvent> {
        let state_events = self.machine.update(dt);
        self.raw_events.extend(state_events.iter().cloned());
        let mapped = self.mapper.map_events(state_events);
        self.pending_events.extend(mapped);
        std::mem::take(&mut self.pending_events)
    }

    /// 取走自上次 `drain` 以来缓冲的 gameplay 事件。
    pub fn drain_events(&mut self) -> Vec<AnimationEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// 取走原始状态机事件（含 `TransitionStarted` / `TransitionFinished`）。
    pub fn drain_state_events(&mut self) -> Vec<StateEvent> {
        std::mem::take(&mut self.raw_events)
    }

    /// 重播当前 animation 状态（时间归零，不影响进行中的过渡）。
    ///
    /// 与 gameplay 的"同动作重启"不同：这里不产生 `ActionStarted` 事件；
    /// 需要事件语义请走 [`AnimationActionBridge::request_one_shot`]。
    pub fn restart(&mut self) {
        self.machine.restart();
    }

    /// 当前动作的剩余时间（秒）。
    ///
    /// - 一次性动作（攻击/受击/闪避/死亡等）：`Some(duration - time)`；
    /// - 循环动作（待机/奔跑/格挡保持）或未绑定 clip：`None`，表示由 gameplay
    ///   时间驱动（符合 gameplay `ActionStateBridge::state_remaining` 的默认约定）。
    pub fn state_remaining(&self) -> Option<f32> {
        let state = self.machine.current_state();
        let entry = self.machine.entry(state)?;
        if entry.looping || entry.duration() <= 0.0 {
            return None;
        }
        Some((entry.duration() - self.machine.current_time()).max(0.0))
    }

    // ---- 内部实现 -----------------------------------------------------

    /// 解析动作 clip 并写入状态机；返回目标 animation 状态。
    ///
    /// 返回 `None` 表示拒绝该动作（攻击类没有可用 clip 视为资产缺失）。
    fn prepare_action(&mut self, action: ActionState) -> Option<AnimationState> {
        let target_state = map_action_state(action);
        if let Some(binding) = self.clips.resolve(action).cloned() {
            // 一次性动作强制不循环（避免攻击/受击循环刷事件）。
            let looping = if action.is_one_shot() {
                false
            } else {
                binding.looping
            };
            self.machine
                .set_clip_with(target_state, binding.clip, looping, binding.speed);
            return Some(target_state);
        }

        // 没有注册绑定：攻击类必须有可用 clip（逐招式或此前 set_clip 的状态槽位），
        // 否则 gameplay 应当收到 `false`（资产缺失）并自行降级。
        if matches!(action, ActionState::Attack(_) | ActionState::HeavyAttack(_))
            && self.machine.clip(target_state).is_none()
        {
            return None;
        }
        Some(target_state)
    }

    /// 记录当前动作（攻击类同时更新事件映射器里的逐招式 id）。
    fn note_action(&mut self, action: ActionState) {
        self.current_action = Some(action);
        if let ActionState::Attack(id) | ActionState::HeavyAttack(id) = action {
            self.mapper.note_attack_action(id);
        }
    }

    /// 同状态重启时补发"动作开始"事件（gameplay 才能感知到重播）。
    fn emit_restart_event(&mut self, action: ActionState) {
        let id = action_id(action);
        if matches!(action, ActionState::Hit) {
            self.pending_events
                .push(AnimationEvent::HitReactionStart(id));
        }
        self.pending_events.push(AnimationEvent::ActionStarted(id));
    }
}

impl ActionStateBridge for AnimationActionBridge {
    fn request_transition(&mut self, transition: ActionTransition) -> bool {
        // 1) Death 是终止状态：进入后拒绝一切切换（含再次进入 Death）。
        if self.machine.current_state().is_terminal() {
            return false;
        }

        let action = transition.to;
        // 2) 解析逐招式 clip 并绑定到目标状态（拒绝 = 资产缺失）。
        let Some(target_state) = self.prepare_action(action) else {
            return false;
        };

        // 3) 目标动画状态与当前相同：同状态不能走 transition（状态机拒绝同状态切换），
        //    这里处理"同一动画状态下的不同招式/重播"，以及"过渡中请求同一目标"。
        if self.machine.current_state() == target_state {
            let action_changed = self.current_action != Some(action);
            let should_restart =
                transition.spec.restart || (action_changed && action.is_one_shot());
            if should_restart {
                // 只重置当前片段时间；进行中的过渡（淡入）继续，不重新开始过渡。
                self.machine.restart();
                self.emit_restart_event(action);
            }
            self.note_action(action);
            return true;
        }

        // 4) 跨状态过渡：允许打断进行中的过渡（animation 0.2 的行为），
        //    过渡中请求同一目标在上面的同状态分支处理。
        let accepted = self
            .machine
            .transition_to(target_state, transition.spec.fade_seconds.max(0.0));
        if accepted {
            self.note_action(action);
        }
        accepted
    }

    fn state_remaining(&self) -> Option<f32> {
        AnimationActionBridge::state_remaining(self)
    }
}

impl AnimationEventSource for AnimationActionBridge {
    fn drain_events(&mut self, out: &mut Vec<AnimationEvent>) {
        out.append(&mut self.pending_events);
    }
}

/// 返回"尚未同步过任何动作"的保留 id（便于调用方初始化事件过滤器）。
pub fn unknown_action_id() -> ActionId {
    reserved_action_ids::UNKNOWN
}

/// 采样当前姿态（便捷函数，等价 `bridge.machine().sample(skeleton)`）。
pub fn sample_pose(bridge: &AnimationActionBridge, skeleton: &Skeleton) -> Pose {
    bridge.machine.sample(skeleton)
}
