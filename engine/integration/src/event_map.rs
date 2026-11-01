//! animation `StateEvent` → gameplay `AnimationEvent` 的事件映射。
//!
//! animation 侧只负责"状态机发生了什么"，gameplay 侧只消费"命中帧 / 连招窗口 /
//! 脚步 / 受击"等战斗事件。本模块把两者对齐：
//!
//! | animation `StateEvent` | gameplay `AnimationEvent` |
//! |---|---|
//! | `Enter(state)` | `ActionStarted(id)`；进入 `Hit` 时额外 `HitReactionStart(id)` |
//! | `Exit(state)` | `ActionEnded(id)` |
//! | `TransitionStarted { from, to }` | `ActionEnded(from_id)` + `ActionStarted(to_id)`，与同批 `Exit`/`Enter` 去重 |
//! | `TransitionFinished { state }` | gameplay 0.2 无对应枚举，保留在原始事件流（`drain_state_events`） |
//! | `ClipEvent { state, event }` | 按事件名映射为 `HitFrame` / `Footstep` / `CancelWindowOpen` / `CancelWindowClose` / `HitReactionStart` |
//!
//! 事件名约定（与资产命名一致，可用 [`AnimationEventMapper::register_rule`] 覆盖）：
//! - `attack_hit`、`hit_frame`、`hit_2`：命中帧，末尾数字为 `hit_index`；
//! - `footstep`、`footstep_l`、`footstep_r`：脚步，`l/r` 或末尾数字为 `foot`；
//! - `cancel_window_open` / `cancel_open` / `combo_window_open`：连招窗口打开；
//! - `cancel_window_close` / `cancel_close`：连招窗口关闭；
//! - `hit_reaction`：受击动作开始；
//! - 其它未识别事件会被计数忽略（`ignored_clip_event_count`），不会 panic。

use std::collections::HashMap;

use hezhou_animation::{AnimationEvent as ClipEvent, AnimationState, StateEvent};
use hezhou_gameplay::{ActionId, AnimationEvent as GameplayEvent};

use crate::mapping::{action_id_for_animation_state, reserved_action_ids};

/// clip 事件名 → gameplay 事件的映射目标。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipEventTarget {
    /// 命中帧（攻击生效帧），`hit_index` 用于多段攻击。
    HitFrame { hit_index: u8 },
    /// 脚步。
    Footstep { foot: u8 },
    /// 连招/取消窗口打开。
    CancelWindowOpen,
    /// 连招/取消窗口关闭。
    CancelWindowClose,
    /// 受击动作开始。
    HitReactionStart,
    /// 明确忽略（例如特效/音效专用事件）。
    Ignore,
}

/// 有状态的动画事件映射器。
///
/// 之所以需要状态：animation 的 `Attack` 状态被所有招式共用，事件里必须补上
/// "最近一次请求的逐招式 ActionId"才能让 gameplay 区分招式。
#[derive(Clone, Debug)]
pub struct AnimationEventMapper {
    last_attack_action: ActionId,
    rules: HashMap<String, ClipEventTarget>,
    ignored_clip_events: u32,
}

impl Default for AnimationEventMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationEventMapper {
    /// 创建映射器（未请求过攻击时事件里的攻击 id 为保留的 `UNKNOWN`）。
    pub fn new() -> Self {
        Self {
            last_attack_action: reserved_action_ids::UNKNOWN,
            rules: HashMap::new(),
            ignored_clip_events: 0,
        }
    }

    /// 记录最近一次请求的攻击招式 id（bridge 在切换状态前调用）。
    pub fn note_attack_action(&mut self, action: ActionId) {
        self.last_attack_action = action;
    }

    /// 最近一次请求的攻击招式 id。
    pub fn last_attack_action(&self) -> ActionId {
        self.last_attack_action
    }

    /// 注册/覆盖某个 clip 事件名的映射规则（优先于内置名称约定）。
    pub fn register_rule(&mut self, name: impl Into<String>, target: ClipEventTarget) {
        self.rules.insert(name.into(), target);
    }

    /// 被忽略的未识别 clip 事件数量（用于资产校验/日志）。
    pub fn ignored_clip_event_count(&self) -> u32 {
        self.ignored_clip_events
    }

    /// 映射一批 animation 状态机事件。
    ///
    /// 返回顺序与输入顺序基本一致；`TransitionStarted` 会合并其配对的状态机
    /// `Exit`/`Enter`，保证一次过渡只产生一组 `ActionEnded` + `ActionStarted`。
    pub fn map_events(&mut self, events: Vec<StateEvent>) -> Vec<GameplayEvent> {
        // 先扫描本批过渡：状态机在过渡时会连续派发
        // `Exit(from)` → `TransitionStarted{from,to}` → `Enter(to)`，
        // 这里把 from/to 拆成两个待匹配队列，避免把同一件事派发两次。
        let mut pending_from: Vec<AnimationState> = Vec::new();
        let mut pending_to: Vec<AnimationState> = Vec::new();
        for event in &events {
            if let StateEvent::TransitionStarted { from, to, .. } = event {
                pending_from.push(*from);
                pending_to.push(*to);
            }
        }

        let mut out = Vec::with_capacity(events.len());
        for event in events {
            match event {
                StateEvent::Exit(state) => {
                    if let Some(position) = pending_from.iter().position(|from| *from == state) {
                        // 由 TransitionStarted 统一派发，避免重复 ActionEnded。
                        pending_from.remove(position);
                    } else {
                        let id = action_id_for_animation_state(state, self.last_attack_action);
                        out.push(GameplayEvent::ActionEnded(id));
                    }
                }
                StateEvent::Enter(state) => {
                    if let Some(position) = pending_to.iter().position(|to| *to == state) {
                        // 由 TransitionStarted 统一派发，避免重复 ActionStarted。
                        pending_to.remove(position);
                    } else {
                        self.push_started(state, &mut out);
                    }
                }
                StateEvent::TransitionStarted { from, to, .. } => {
                    let from_id = action_id_for_animation_state(from, self.last_attack_action);
                    out.push(GameplayEvent::ActionEnded(from_id));
                    self.push_started(to, &mut out);
                }
                // gameplay 0.2 的 AnimationEvent 没有"过渡完成"概念：
                // 保留在原始事件流里供编辑器/调试使用，不伪造 gameplay 事件。
                StateEvent::TransitionFinished { .. } => {}
                StateEvent::ClipEvent { state, event } => {
                    if let Some(mapped) = self.map_clip_event(state, &event) {
                        out.push(mapped);
                    }
                }
            }
        }
        out
    }

    /// 把单个 clip 事件映射为 gameplay 事件。
    pub fn map_clip_event(
        &mut self,
        state: AnimationState,
        event: &ClipEvent,
    ) -> Option<GameplayEvent> {
        let target = self
            .rules
            .get(&event.name)
            .copied()
            .or_else(|| classify_clip_event(&event.name));
        let id = action_id_for_animation_state(state, self.last_attack_action);
        match target {
            Some(ClipEventTarget::HitFrame { hit_index }) => Some(GameplayEvent::HitFrame {
                action: id,
                hit_index,
            }),
            Some(ClipEventTarget::Footstep { foot }) => Some(GameplayEvent::Footstep { foot }),
            Some(ClipEventTarget::CancelWindowOpen) => Some(GameplayEvent::CancelWindowOpen(id)),
            Some(ClipEventTarget::CancelWindowClose) => Some(GameplayEvent::CancelWindowClose(id)),
            Some(ClipEventTarget::HitReactionStart) => Some(GameplayEvent::HitReactionStart(id)),
            Some(ClipEventTarget::Ignore) => None,
            None => {
                self.ignored_clip_events = self.ignored_clip_events.saturating_add(1);
                None
            }
        }
    }

    /// `ActionStarted` 的统一派发点（受击状态额外补 `HitReactionStart`）。
    fn push_started(&self, state: AnimationState, out: &mut Vec<GameplayEvent>) {
        let id = action_id_for_animation_state(state, self.last_attack_action);
        if state.name() == "Hit" {
            out.push(GameplayEvent::HitReactionStart(id));
        }
        out.push(GameplayEvent::ActionStarted(id));
    }
}

/// 按事件名约定分类 clip 事件。
fn classify_clip_event(name: &str) -> Option<ClipEventTarget> {
    let lower = name.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }
    // 受击动作要先于通用 hit 判断，否则 "hit_reaction" 会被当成命中帧。
    if lower.starts_with("hit_reaction") {
        return Some(ClipEventTarget::HitReactionStart);
    }
    if lower.starts_with("attack_hit")
        || lower.starts_with("hit_frame")
        || lower == "hit"
        || lower.starts_with("hit_")
    {
        return Some(ClipEventTarget::HitFrame {
            hit_index: trailing_index(&lower).unwrap_or(0),
        });
    }
    if lower.starts_with("footstep") || lower == "foot" || lower.starts_with("foot_") {
        let foot = if lower.ends_with("_l") || lower.ends_with("left") {
            0
        } else if lower.ends_with("_r") || lower.ends_with("right") {
            1
        } else {
            trailing_index(&lower).unwrap_or(0)
        };
        return Some(ClipEventTarget::Footstep { foot });
    }
    if lower.contains("cancel") || lower.contains("combo_window") {
        return Some(if lower.contains("close") {
            ClipEventTarget::CancelWindowClose
        } else {
            ClipEventTarget::CancelWindowOpen
        });
    }
    None
}

/// 取名称末尾的连续数字（`attack_hit_2` → `2`）。
fn trailing_index(name: &str) -> Option<u8> {
    let digits: String = name
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hezhou_animation::AnimationEvent as ClipEvent;
    use hezhou_gameplay::ActionId;

    #[test]
    fn clip_event_names_are_classified() {
        assert_eq!(
            classify_clip_event("attack_hit_2"),
            Some(ClipEventTarget::HitFrame { hit_index: 2 })
        );
        assert_eq!(
            classify_clip_event("footstep_r"),
            Some(ClipEventTarget::Footstep { foot: 1 })
        );
        assert_eq!(
            classify_clip_event("cancel_window_open"),
            Some(ClipEventTarget::CancelWindowOpen)
        );
        assert_eq!(
            classify_clip_event("combo_window_close"),
            Some(ClipEventTarget::CancelWindowClose)
        );
        assert_eq!(
            classify_clip_event("hit_reaction"),
            Some(ClipEventTarget::HitReactionStart)
        );
        assert_eq!(classify_clip_event("vfx_glow"), None);
    }

    #[test]
    fn transition_and_state_events_are_not_duplicated() {
        let mut mapper = AnimationEventMapper::new();
        mapper.note_attack_action(ActionId::new(9));
        let events = vec![
            StateEvent::Exit(AnimationState::Idle),
            StateEvent::TransitionStarted {
                from: AnimationState::Idle,
                to: AnimationState::Attack,
                duration: 0.1,
            },
            StateEvent::Enter(AnimationState::Attack),
            StateEvent::TransitionFinished {
                state: AnimationState::Attack,
            },
            StateEvent::ClipEvent {
                state: AnimationState::Attack,
                event: ClipEvent::new(0.1, "attack_hit"),
            },
        ];
        let mapped = mapper.map_events(events);
        assert_eq!(
            mapped
                .iter()
                .filter(|event| matches!(event, GameplayEvent::ActionEnded(_)))
                .count(),
            1
        );
        assert_eq!(
            mapped
                .iter()
                .filter(|event| matches!(event, GameplayEvent::ActionStarted(_)))
                .count(),
            1
        );
        assert!(mapped.contains(&GameplayEvent::ActionEnded(
            crate::mapping::reserved_action_ids::IDLE
        )));
        assert!(mapped.contains(&GameplayEvent::HitFrame {
            action: ActionId::new(9),
            hit_index: 0
        }));
    }

    #[test]
    fn unknown_clip_events_are_counted_not_emitted() {
        let mut mapper = AnimationEventMapper::new();
        let mapped = mapper.map_events(vec![StateEvent::ClipEvent {
            state: AnimationState::Attack,
            event: ClipEvent::new(0.0, "vfx_glow"),
        }]);
        assert!(mapped.is_empty());
        assert_eq!(mapper.ignored_clip_event_count(), 1);
    }
}
