//! gameplay `ActionState` → animation `AnimationState` 的映射表。
//!
//! 0.2 的 animation 状态机交付 6 个基础状态（Idle/Run/Jump/Attack/Hit/Death），
//! task-11 又追加了 Land/Dodge/Block 三个扩展状态；而 gameplay 的动作集合更细
//! （Move/Fall/Land/Dodge/Block/HeavyAttack/逐招式攻击）。本模块负责把两者对齐：
//!
//! | gameplay `ActionState` | animation `AnimationState`（默认，扩展已合入） | 兼容模式（`--no-default-features`） |
//! |---|---|---|
//! | `Idle` | `Idle` | `Idle` |
//! | `Move` / `Run` | `Run` | `Run` |
//! | `Jump` / `Fall` | `Jump` | `Jump` |
//! | `Land` | `Land` | `Jump` |
//! | `Attack(_)` / `HeavyAttack(_)` | `Attack`（按招式换 clip） | `Attack` |
//! | `Dodge` | `Dodge` | `Attack`（专用 clip） |
//! | `Block` | `Block` | `Attack`（专用 clip） |
//! | `Hit` | `Hit` | `Hit` |
//! | `Death` | `Death` | `Death` |
//!
//! 默认启用 `extended-states` feature，做一一映射；关闭后使用兼容映射，
//! 保证在 animation task-11 之前的历史版本上也能编译、测试、运行。

use hezhou_animation::AnimationState;
use hezhou_gameplay::{ActionId, ActionState};

/// 集成层为非攻击动作保留的 `ActionId`（约定 `< 32`）。
///
/// gameplay 的 [`ActionId`] 是"动画资产中的 clip/action 标识"；攻击/重攻击自带
/// 逐招式 id，而移动、受击等状态没有 payload。集成层用这些小整数 id 让
/// `AnimationEvent::ActionStarted` / `ActionEnded` 等事件也能表达它们。
/// 游戏自定义招式请从 32 开始分配，避免与保留区冲突。
pub mod reserved_action_ids {
    use hezhou_gameplay::ActionId;

    /// 未知动作（尚未同步过任何动作）。
    pub const UNKNOWN: ActionId = ActionId::new(0);
    /// 待机。
    pub const IDLE: ActionId = ActionId::new(1);
    /// 步行。
    pub const MOVE: ActionId = ActionId::new(2);
    /// 奔跑。
    pub const RUN: ActionId = ActionId::new(3);
    /// 跳跃（上升）。
    pub const JUMP: ActionId = ActionId::new(4);
    /// 下落。
    pub const FALL: ActionId = ActionId::new(5);
    /// 落地。
    pub const LAND: ActionId = ActionId::new(6);
    /// 闪避。
    pub const DODGE: ActionId = ActionId::new(7);
    /// 格挡。
    pub const BLOCK: ActionId = ActionId::new(8);
    /// 受击。
    pub const HIT: ActionId = ActionId::new(9);
    /// 死亡。
    pub const DEATH: ActionId = ActionId::new(10);
}

/// 是否启用了 animation Land/Dodge/Block 扩展状态的一一映射。
pub const EXTENDED_ANIMATION_STATES: bool = cfg!(feature = "extended-states");

#[cfg(feature = "extended-states")]
#[inline]
fn land_state() -> AnimationState {
    AnimationState::Land
}

#[cfg(not(feature = "extended-states"))]
#[inline]
fn land_state() -> AnimationState {
    // 兼容：animation 尚无 Land 状态，落地复用 Jump 状态（一次性）。
    AnimationState::Jump
}

#[cfg(feature = "extended-states")]
#[inline]
fn dodge_state() -> AnimationState {
    AnimationState::Dodge
}

#[cfg(not(feature = "extended-states"))]
#[inline]
fn dodge_state() -> AnimationState {
    // 兼容：Dodge 复用 Attack 状态槽位，具体 clip 由 `set_dodge_clip` 指定。
    AnimationState::Attack
}

#[cfg(feature = "extended-states")]
#[inline]
fn block_state() -> AnimationState {
    AnimationState::Block
}

#[cfg(not(feature = "extended-states"))]
#[inline]
fn block_state() -> AnimationState {
    // 兼容：Block 复用 Attack 状态槽位，具体 clip 由 `set_block_clip` 指定。
    AnimationState::Attack
}

/// 把 gameplay 逻辑动作状态映射为 animation 状态机状态。
///
/// 该函数无副作用、无状态，便于 C#/工具层复用。
pub fn map_action_state(action: ActionState) -> AnimationState {
    match action {
        ActionState::Idle => AnimationState::Idle,
        // 0.2 animation 没有 Walk/单独的 Move 状态：Move 与 Run 共用 Run。
        ActionState::Move | ActionState::Run => AnimationState::Run,
        // 0.2 animation 没有 Fall 状态：上升/下落共用 Jump。
        ActionState::Jump | ActionState::Fall => AnimationState::Jump,
        ActionState::Land => land_state(),
        ActionState::Attack(_) | ActionState::HeavyAttack(_) => AnimationState::Attack,
        ActionState::Dodge => dodge_state(),
        ActionState::Block => block_state(),
        ActionState::Hit => AnimationState::Hit,
        ActionState::Death => AnimationState::Death,
    }
}

/// 把逻辑动作状态映射为事件里使用的 `ActionId`。
///
/// 攻击/重攻击返回逐招式 id；其余动作返回 [`reserved_action_ids`] 保留值。
pub fn action_id(action: ActionState) -> ActionId {
    match action {
        ActionState::Attack(id) | ActionState::HeavyAttack(id) => id,
        ActionState::Idle => reserved_action_ids::IDLE,
        ActionState::Move => reserved_action_ids::MOVE,
        ActionState::Run => reserved_action_ids::RUN,
        ActionState::Jump => reserved_action_ids::JUMP,
        ActionState::Fall => reserved_action_ids::FALL,
        ActionState::Land => reserved_action_ids::LAND,
        ActionState::Dodge => reserved_action_ids::DODGE,
        ActionState::Block => reserved_action_ids::BLOCK,
        ActionState::Hit => reserved_action_ids::HIT,
        ActionState::Death => reserved_action_ids::DEATH,
    }
}

/// 由 animation 状态反查事件里使用的 `ActionId`。
///
/// `last_attack_action` 是最近一次请求的攻击招式 id：animation 的 `Attack`
/// 状态被所有招式共用，事件里需要它来区分是哪个招式触发的。
pub fn action_id_for_animation_state(
    state: AnimationState,
    last_attack_action: ActionId,
) -> ActionId {
    // 用 `name()` 而不是直接 match 枚举变体：这样 animation 未来新增
    // Land/Dodge/Block 等状态时，本 crate 不需要跟着改 match 就能编译。
    match state.name() {
        "Attack" => last_attack_action,
        "Idle" => reserved_action_ids::IDLE,
        "Run" => reserved_action_ids::RUN,
        "Jump" => reserved_action_ids::JUMP,
        "Hit" => reserved_action_ids::HIT,
        "Death" => reserved_action_ids::DEATH,
        "Land" => reserved_action_ids::LAND,
        "Dodge" => reserved_action_ids::DODGE,
        "Block" => reserved_action_ids::BLOCK,
        _ => reserved_action_ids::UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_ids_stay_below_user_range() {
        for id in [
            reserved_action_ids::UNKNOWN,
            reserved_action_ids::IDLE,
            reserved_action_ids::MOVE,
            reserved_action_ids::RUN,
            reserved_action_ids::JUMP,
            reserved_action_ids::FALL,
            reserved_action_ids::LAND,
            reserved_action_ids::DODGE,
            reserved_action_ids::BLOCK,
            reserved_action_ids::HIT,
            reserved_action_ids::DEATH,
        ] {
            assert!(id.raw() < 32, "保留 ActionId 应小于 32");
        }
        // 唯一性：避免事件里无法区分动作。
        let mut ids: Vec<u32> = [
            reserved_action_ids::UNKNOWN,
            reserved_action_ids::IDLE,
            reserved_action_ids::MOVE,
            reserved_action_ids::RUN,
            reserved_action_ids::JUMP,
            reserved_action_ids::FALL,
            reserved_action_ids::LAND,
            reserved_action_ids::DODGE,
            reserved_action_ids::BLOCK,
            reserved_action_ids::HIT,
            reserved_action_ids::DEATH,
        ]
        .iter()
        .map(|id| id.raw())
        .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 11);
    }

    #[test]
    fn attack_events_use_per_move_action_id() {
        let custom = ActionId::new(100);
        assert_eq!(
            action_id_for_animation_state(AnimationState::Attack, custom),
            custom
        );
        assert_eq!(
            action_id(ActionState::HeavyAttack(custom)),
            custom,
            "重攻击也走逐招式 id"
        );
    }
}
