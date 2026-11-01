//! `AnimationActionAdapter`：面向 Editor/Player 每帧循环的便捷封装。
//!
//! 它把 [`AnimationActionBridge`](crate::AnimationActionBridge)（状态机 + 事件映射）
//! 和"采样姿态 / 查询剩余时间 / 处理 Death"等每帧接口放在一起，避免调用方
//! 直接操作状态机内部结构。
//!
//! 典型每帧顺序（详见 crate README）：
//!
//! ```text
//! motor.step(dt) -> MotorOutput
//!   -> locomotion_state / 战斗层决定 desired: ActionState
//!   -> ActionAnimator::sync / play_one_shot(desired, adapter.bridge_mut())
//!   -> let events = adapter.update(dt);
//!   -> 消费 events（命中帧/连招窗口/脚步/受击）
//!   -> let pose = adapter.sample(&skeleton);
//!   -> JointPalette::from_pose(...) -> render 上传
//! ```

use hezhou_animation::{AnimationClip, AnimationState, Pose, Skeleton, StateEvent};
use hezhou_gameplay::{
    ActionId, ActionState, ActionStateBridge, ActionTransition, AnimationEvent, TransitionSpec,
};

use crate::bridge::AnimationActionBridge;
use crate::clip_registry::{ClipBinding, VirtualSlot};
use crate::mapping::map_action_state;

/// animation + gameplay 的每帧适配器。
#[derive(Clone, Debug)]
pub struct AnimationActionAdapter {
    bridge: AnimationActionBridge,
}

impl Default for AnimationActionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationActionAdapter {
    /// 创建适配器（animation 状态机默认过渡 0.2s）。
    pub fn new() -> Self {
        Self {
            bridge: AnimationActionBridge::new(),
        }
    }

    /// 创建适配器并指定默认过渡时长（秒）。
    pub fn with_default_transition(duration: f32) -> Self {
        Self {
            bridge: AnimationActionBridge::new().with_default_transition(duration),
        }
    }

    /// 用已有 bridge 构造（便于复用/测试）。
    pub fn from_bridge(bridge: AnimationActionBridge) -> Self {
        Self { bridge }
    }

    /// 内部 bridge（只读）。
    pub fn bridge(&self) -> &AnimationActionBridge {
        &self.bridge
    }

    /// 内部 bridge（可变）；交给 gameplay `ActionAnimator` 时使用。
    pub fn bridge_mut(&mut self) -> &mut AnimationActionBridge {
        &mut self.bridge
    }

    // ---- clip 绑定 ----------------------------------------------------

    /// 直接绑定 animation 状态 clip（"先 set_clip 再切换"的兼容用法）。
    pub fn set_clip(&mut self, state: AnimationState, clip: AnimationClip) -> &mut Self {
        self.bridge.set_clip(state, clip);
        self
    }

    /// 直接绑定 animation 状态 clip 并指定循环/速度。
    pub fn set_clip_with(
        &mut self,
        state: AnimationState,
        clip: AnimationClip,
        looping: bool,
        speed: f32,
    ) -> &mut Self {
        self.bridge.set_clip_with(state, clip, looping, speed);
        self
    }

    /// 注册状态槽位 clip 绑定。
    pub fn set_state_clip(&mut self, state: AnimationState, binding: ClipBinding) -> &mut Self {
        self.bridge.set_state_clip(state, binding);
        self
    }

    /// 注册逐招式攻击 clip（Attack / HeavyAttack 共用），一次性播放。
    pub fn set_action_clip(&mut self, action: ActionId, clip: AnimationClip) -> &mut Self {
        self.bridge
            .set_action_clip(action, ClipBinding::one_shot(clip));
        self
    }

    /// 注册逐招式攻击 clip 并指定播放速度。
    pub fn set_action_clip_with_speed(
        &mut self,
        action: ActionId,
        clip: AnimationClip,
        speed: f32,
    ) -> &mut Self {
        self.bridge
            .set_action_clip(action, ClipBinding::one_shot(clip).with_speed(speed));
        self
    }

    /// 注册落地 clip（兼容模式下写入 Jump 状态槽）。
    pub fn set_land_clip(&mut self, clip: AnimationClip) -> &mut Self {
        self.bridge
            .set_virtual_clip(VirtualSlot::Land, ClipBinding::one_shot(clip));
        self
    }

    /// 注册闪避 clip（兼容模式下写入 Attack 状态槽）。
    pub fn set_dodge_clip(&mut self, clip: AnimationClip) -> &mut Self {
        self.bridge
            .set_virtual_clip(VirtualSlot::Dodge, ClipBinding::one_shot(clip));
        self
    }

    /// 注册格挡 clip（格挡是保持型动作，默认循环）。
    pub fn set_block_clip(&mut self, clip: AnimationClip) -> &mut Self {
        self.bridge
            .set_virtual_clip(VirtualSlot::Block, ClipBinding::looping(clip));
        self
    }

    // ---- 每帧接口 -----------------------------------------------------

    /// 请求基础/一次性动作（一次性动作自动使用短淡化 + 重启）。
    pub fn request(&mut self, action: ActionState) -> bool {
        self.bridge.request(action)
    }

    /// 请求一次性动作（攻击/重攻击/闪避/受击/死亡/落地）。
    pub fn request_one_shot(&mut self, action: ActionState) -> bool {
        self.bridge.request_one_shot(action)
    }

    /// 以自定义过渡参数请求动作。
    pub fn request_with(&mut self, action: ActionState, spec: TransitionSpec) -> bool {
        self.bridge.request_transition(ActionTransition {
            from: self.bridge.current_action(),
            to: action,
            spec,
        })
    }

    /// 推进状态机（`dt` 秒）并返回本帧映射出的 gameplay 事件。
    pub fn update(&mut self, dt: f32) -> Vec<AnimationEvent> {
        self.bridge.update(dt)
    }

    /// 取走缓冲中的 gameplay 事件（与 `update` 二选一，避免重复消费）。
    pub fn drain_events(&mut self) -> Vec<AnimationEvent> {
        self.bridge.drain_events()
    }

    /// 取走原始状态机事件（含 `TransitionStarted` / `TransitionFinished`）。
    pub fn drain_state_events(&mut self) -> Vec<StateEvent> {
        self.bridge.drain_state_events()
    }

    /// 采样当前姿态（过渡中为交叉淡入淡出结果）。
    pub fn sample(&self, skeleton: &Skeleton) -> Pose {
        self.bridge.machine().sample(skeleton)
    }

    /// 当前 animation 状态。
    pub fn current_state(&self) -> AnimationState {
        self.bridge.current_animation_state()
    }

    /// 当前逻辑动作状态（`None` = 尚未同步）。
    pub fn current_action(&self) -> Option<ActionState> {
        self.bridge.current_action()
    }

    /// 当前 animation 状态内时间（秒）。
    pub fn current_time(&self) -> f32 {
        self.bridge.current_time()
    }

    /// 当前状态归一化进度（0..1）。
    pub fn normalized_time(&self) -> f32 {
        self.bridge.machine().normalized_time()
    }

    /// 当前动作剩余时间（一次性动作 `Some`，循环动作 `None`）。
    pub fn state_remaining(&self) -> Option<f32> {
        self.bridge.state_remaining()
    }

    /// 是否处于过渡中。
    pub fn is_transitioning(&self) -> bool {
        self.bridge.is_transitioning()
    }

    /// 当前过渡权重（0..1）。
    pub fn transition_alpha(&self) -> f32 {
        self.bridge.transition_alpha()
    }

    /// 是否已进入死亡等终止状态（进入后调色板应停在最后一帧）。
    pub fn is_terminal(&self) -> bool {
        self.bridge.is_terminal()
    }

    /// 重播当前 animation 状态（编辑器预览/受击重播）。
    pub fn restart(&mut self) {
        self.bridge.restart();
    }

    /// 便捷：当前逻辑动作对应的 animation 状态（尚未同步时按 `Idle` 返回）。
    pub fn mapped_state_for(&self, action: ActionState) -> AnimationState {
        map_action_state(action)
    }
}
