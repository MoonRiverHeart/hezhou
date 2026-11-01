//! 动作 clip 注册表：按招式 / 按动画状态 / 按虚拟槽位解析要播放的片段。
//!
//! 0.2 的 animation 状态机每个状态只能绑定 **一个** clip，而 gameplay 需要
//! "Attack 状态 + 逐招式 clip" 的两层结构，以及兼容模式下的 Land/Dodge/Block
//! 虚拟槽位。集成层因此维护一张注册表：
//!
//! - [`ActionClipRegistry::set_action_clip`]：逐招式（`ActionId`）指定攻击 clip；
//! - [`ActionClipRegistry::set_state_clip`]：直接绑定 animation 状态槽位；
//! - [`ActionClipRegistry::set_virtual_clip`]：绑定 Land/Dodge/Block 虚拟槽位
//!   （animation 扩展未合入时它们复用 Jump/Attack 状态）。
//!
//! 解析优先级：逐招式 → 虚拟槽位 → 状态槽位。解析结果由 bridge 在切换状态前
//! `set_clip_with` 写入状态机，因此同一 animation 状态可以在不同招式间切换 clip。

use std::collections::HashMap;

use hezhou_animation::{AnimationClip, AnimationState};
use hezhou_gameplay::{ActionId, ActionState};

use crate::mapping::map_action_state;

/// 一个状态/招式的 clip 绑定。
#[derive(Clone, Debug, PartialEq)]
pub struct ClipBinding {
    /// 动画片段。
    pub clip: AnimationClip,
    /// 是否循环播放；一次性动作（攻击/闪避/受击/死亡等）会被强制设为 `false`。
    pub looping: bool,
    /// 播放速度倍率。
    pub speed: f32,
}

impl ClipBinding {
    /// 通用构造。
    pub fn new(clip: AnimationClip, looping: bool) -> Self {
        Self {
            clip,
            looping,
            speed: 1.0,
        }
    }

    /// 一次性动作绑定（攻击/闪避/受击/死亡）。
    pub fn one_shot(clip: AnimationClip) -> Self {
        Self::new(clip, false)
    }

    /// 循环动作绑定（待机/奔跑/格挡保持）。
    pub fn looping(clip: AnimationClip) -> Self {
        Self::new(clip, true)
    }

    /// 设置播放速度。
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.0);
        self
    }
}

/// animation 扩展状态未合入时的虚拟动作槽位。
///
/// - animation 已有同名状态时（`extended-states` feature），虚拟 clip 会写入同名状态；
/// - 否则 `Land` 写入 `Jump` 状态槽、`Dodge`/`Block` 写入 `Attack` 状态槽，
///   通过"每次切换前重绑 clip"保证不同动作不会串片。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VirtualSlot {
    /// 落地。
    Land,
    /// 闪避。
    Dodge,
    /// 格挡。
    Block,
}

impl VirtualSlot {
    /// 对应的 gameplay 动作状态（无 payload）。
    pub fn action_state(self) -> ActionState {
        match self {
            Self::Land => ActionState::Land,
            Self::Dodge => ActionState::Dodge,
            Self::Block => ActionState::Block,
        }
    }
}

/// 动作 clip 注册表。
#[derive(Clone, Debug, Default)]
pub struct ActionClipRegistry {
    action_clips: HashMap<ActionId, ClipBinding>,
    state_clips: HashMap<AnimationState, ClipBinding>,
    virtual_clips: HashMap<VirtualSlot, ClipBinding>,
}

impl ActionClipRegistry {
    /// 空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册逐招式攻击 clip（Attack / HeavyAttack 共用）。
    pub fn set_action_clip(&mut self, action: ActionId, binding: ClipBinding) {
        self.action_clips.insert(action, binding);
    }

    /// 直接注册 animation 状态槽位 clip。
    pub fn set_state_clip(&mut self, state: AnimationState, binding: ClipBinding) {
        self.state_clips.insert(state, binding);
    }

    /// 注册 Land/Dodge/Block 虚拟槽位 clip。
    pub fn set_virtual_clip(&mut self, slot: VirtualSlot, binding: ClipBinding) {
        self.virtual_clips.insert(slot, binding);
    }

    /// 移除逐招式 clip。
    pub fn remove_action_clip(&mut self, action: ActionId) -> Option<ClipBinding> {
        self.action_clips.remove(&action)
    }

    /// 查询逐招式 clip。
    pub fn action_clip(&self, action: ActionId) -> Option<&ClipBinding> {
        self.action_clips.get(&action)
    }

    /// 查询状态槽位 clip。
    pub fn state_clip(&self, state: AnimationState) -> Option<&ClipBinding> {
        self.state_clips.get(&state)
    }

    /// 查询虚拟槽位 clip。
    pub fn virtual_clip(&self, slot: VirtualSlot) -> Option<&ClipBinding> {
        self.virtual_clips.get(&slot)
    }

    /// 已注册的逐招式 clip 数量。
    pub fn action_clip_count(&self) -> usize {
        self.action_clips.len()
    }

    /// 解析某个逻辑动作应使用的 clip 绑定（不含状态映射）。
    ///
    /// 返回 `None` 表示没有注册任何可用 clip，由调用方决定降级或拒绝。
    pub fn resolve(&self, action: ActionState) -> Option<&ClipBinding> {
        match action {
            ActionState::Idle => self.state_clip(AnimationState::Idle),
            ActionState::Move | ActionState::Run => self.state_clip(AnimationState::Run),
            ActionState::Jump | ActionState::Fall => self.state_clip(AnimationState::Jump),
            ActionState::Land => self
                .virtual_clip(VirtualSlot::Land)
                .or_else(|| self.state_clip(map_action_state(action))),
            ActionState::Attack(id) | ActionState::HeavyAttack(id) => self
                .action_clip(id)
                .or_else(|| self.state_clip(AnimationState::Attack)),
            ActionState::Dodge => self
                .virtual_clip(VirtualSlot::Dodge)
                .or_else(|| self.state_clip(map_action_state(action))),
            ActionState::Block => self
                .virtual_clip(VirtualSlot::Block)
                .or_else(|| self.state_clip(map_action_state(action))),
            ActionState::Hit => self.state_clip(AnimationState::Hit),
            ActionState::Death => self.state_clip(AnimationState::Death),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hezhou_animation::AnimationClip;

    fn clip(name: &str) -> AnimationClip {
        AnimationClip::new(name, 1.0)
    }

    #[test]
    fn per_action_clip_has_priority_over_state_slot() {
        let mut registry = ActionClipRegistry::new();
        registry.set_state_clip(
            AnimationState::Attack,
            ClipBinding::one_shot(clip("shared")),
        );
        registry.set_action_clip(ActionId::new(7), ClipBinding::one_shot(clip("attack_7")));

        let resolved = registry
            .resolve(ActionState::Attack(ActionId::new(7)))
            .unwrap();
        assert_eq!(resolved.clip.name, "attack_7");
        // 未注册的招式回退到状态槽位。
        let fallback = registry
            .resolve(ActionState::Attack(ActionId::new(8)))
            .unwrap();
        assert_eq!(fallback.clip.name, "shared");
    }

    #[test]
    fn virtual_slot_has_priority_over_compat_state_slot() {
        let mut registry = ActionClipRegistry::new();
        registry.set_state_clip(AnimationState::Jump, ClipBinding::one_shot(clip("jump")));
        registry.set_virtual_clip(VirtualSlot::Land, ClipBinding::one_shot(clip("land")));
        assert_eq!(
            registry.resolve(ActionState::Land).unwrap().clip.name,
            "land"
        );
        assert_eq!(
            registry.resolve(ActionState::Jump).unwrap().clip.name,
            "jump"
        );
    }
}
