//! ComboGraph：攻击定义、连招图与连招窗口状态机。
//!
//! 设计目标：招式数据完全由调用方提供（攻击定义里只有时间轴/命中盒/下一段列表），
//! 引擎不写死任何游戏招式或数值。时间轴以"动作开始后经过的秒数"为准，
//! 因此不依赖具体动画实现：
//!
//! ```text
//! |<-- startup -->|<-- active -->|<---- recovery ---->|
//!                               |<-- combo window -->|   （可被下一段取消/衔接）
//! ```
//!
//! 连招输入有两种合法时机：
//! 1. 处于 combo window 内：直接排队，到窗口起点后衔接；
//! 2. 略早于窗口（在 `ComboConfig::input_buffer_time` 内）：预输入缓冲。

use std::collections::HashMap;

use crate::animation_bridge::{ActionId, ActionState};
use crate::combat::damage::{DamageEvent, DamageKind};
use crate::combat::hitbox::{HitContact, Hitbox};
use crate::ids::CombatantId;
use crate::math::Vec3;

/// 攻击标识（由调用方定义，例如轻击 1、轻击 2、重击）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttackId(pub u32);

impl AttackId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// 攻击类型（只影响动画状态分类与默认伤害类型，不含游戏数值）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AttackKind {
    /// 普通/轻攻击。
    #[default]
    Light,
    /// 重攻击。
    Heavy,
    /// 特殊/技能攻击。
    Special,
}

/// 时间窗口（相对动作开始，秒）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackWindow {
    /// 窗口开始时间。
    pub start: f32,
    /// 窗口结束时间。
    pub end: f32,
}

impl AttackWindow {
    pub fn new(start: f32, end: f32) -> Self {
        Self { start, end }
    }

    /// 窗口长度。
    pub fn duration(&self) -> f32 {
        (self.end - self.start).max(0.0)
    }

    /// 时间 `t` 是否落在窗口内（含边界）。
    pub fn contains(&self, t: f32) -> bool {
        t >= self.start && t <= self.end
    }

    /// 是否为空窗口。
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

/// 攻击阶段。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackPhase {
    /// 前摇。
    Startup,
    /// 判定生效。
    Active,
    /// 后摇。
    Recovery,
    /// 已结束。
    Finished,
}

/// 单个招式的定义。
#[derive(Clone, Debug, PartialEq)]
pub struct AttackDefinition {
    /// 招式 ID。
    pub id: AttackId,
    /// 攻击类型。
    pub kind: AttackKind,
    /// 对应动画动作。
    pub action: ActionId,
    /// 前摇时长（秒）。
    pub startup: f32,
    /// 判定持续时间（秒）。
    pub active: f32,
    /// 后摇时长（秒）。
    pub recovery: f32,
    /// 可衔接下一段的窗口（相对动作开始）。
    pub combo_window: AttackWindow,
    /// 可衔接的后续招式。
    pub next_attacks: Vec<AttackId>,
    /// 命中盒。
    pub hitbox: Hitbox,
    /// 伤害值。
    pub damage: f32,
    /// 伤害类型。
    pub damage_kind: DamageKind,
    /// 击退强度。
    pub knockback: f32,
    /// 是否无视格挡。
    pub unblockable: bool,
    /// 是否可以作为连招起手。
    pub is_root: bool,
    /// 攻击期间向前位移速度（米/秒），0 表示不位移。
    pub forward_push: f32,
}

impl AttackDefinition {
    /// 用通用默认值构造一个招式；时间轴为 0.12s 前摇 + 0.1s 判定 + 0.25s 后摇。
    pub fn new(id: AttackId, kind: AttackKind) -> Self {
        let startup = 0.12;
        let active = 0.1;
        let recovery = 0.25;
        Self {
            id,
            kind,
            action: ActionId(id.0),
            startup,
            active,
            recovery,
            combo_window: AttackWindow::new(startup + active * 0.5, startup + active + recovery),
            next_attacks: Vec::new(),
            hitbox: Hitbox::default(),
            damage: 10.0,
            damage_kind: match kind {
                AttackKind::Heavy => DamageKind::Heavy,
                AttackKind::Special => DamageKind::Special,
                AttackKind::Light => DamageKind::Light,
            },
            knockback: 2.0,
            unblockable: false,
            is_root: true,
            forward_push: 0.0,
        }
    }

    /// 设置时间轴；`combo_window` 会同步为默认窗口（可随后用 `with_combo_window` 覆盖）。
    pub fn with_timing(mut self, startup: f32, active: f32, recovery: f32) -> Self {
        self.startup = startup.max(0.0);
        self.active = active.max(0.0);
        self.recovery = recovery.max(0.0);
        self.combo_window = AttackWindow::new(
            self.startup + self.active * 0.5,
            self.startup + self.active + self.recovery,
        );
        self
    }

    pub fn with_combo_window(mut self, window: AttackWindow) -> Self {
        self.combo_window = window;
        self
    }

    pub fn with_next(mut self, next: impl IntoIterator<Item = AttackId>) -> Self {
        self.next_attacks = next.into_iter().collect();
        self
    }

    pub fn with_hitbox(mut self, hitbox: Hitbox) -> Self {
        self.hitbox = hitbox;
        self
    }

    pub fn with_damage(mut self, damage: f32, kind: DamageKind) -> Self {
        self.damage = damage;
        self.damage_kind = kind;
        self
    }

    pub fn with_knockback(mut self, knockback: f32) -> Self {
        self.knockback = knockback;
        self
    }

    pub fn with_unblockable(mut self, unblockable: bool) -> Self {
        self.unblockable = unblockable;
        self
    }

    pub fn with_action(mut self, action: ActionId) -> Self {
        self.action = action;
        self
    }

    pub fn with_forward_push(mut self, forward_push: f32) -> Self {
        self.forward_push = forward_push;
        self
    }

    /// 仅作为连招后续段（不能起手）。
    pub fn as_combo_only(mut self) -> Self {
        self.is_root = false;
        self
    }

    /// 仅作为起手（默认即 true）。
    pub fn as_root(mut self) -> Self {
        self.is_root = true;
        self
    }

    /// 动作总时长。
    pub fn total_duration(&self) -> f32 {
        self.startup + self.active + self.recovery
    }

    /// 判定生效窗口。
    pub fn hitbox_window(&self) -> AttackWindow {
        AttackWindow::new(self.startup, self.startup + self.active)
    }

    /// 根据命中结果生成伤害事件（接入 `resolve_damage` 前的中间数据）。
    pub fn damage_events(
        &self,
        source: CombatantId,
        attacker_forward: Vec3,
        contacts: &[HitContact],
    ) -> Vec<DamageEvent> {
        contacts
            .iter()
            .map(|contact| {
                DamageEvent::from_contact(source, contact, self.damage, self.damage_kind)
                    .with_hit_direction(attacker_forward)
                    .with_knockback(self.knockback)
                    .with_unblockable(self.unblockable)
            })
            .collect()
    }
}

/// 连招图：招式集合与"谁能接谁"的关系。
#[derive(Clone, Debug, Default)]
pub struct ComboGraph {
    attacks: HashMap<AttackId, AttackDefinition>,
}

impl ComboGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加/替换招式。
    pub fn add_attack(&mut self, definition: AttackDefinition) -> &mut Self {
        self.attacks.insert(definition.id, definition);
        self
    }

    /// 查询招式。
    pub fn attack(&self, id: AttackId) -> Option<&AttackDefinition> {
        self.attacks.get(&id)
    }

    /// 是否可以从 `from` 衔接到 `to`。
    pub fn can_chain(&self, from: AttackId, to: AttackId) -> bool {
        self.attack(from)
            .map(|definition| definition.next_attacks.contains(&to))
            .unwrap_or(false)
    }

    /// 是否可以作为起手招式。
    pub fn can_start(&self, id: AttackId) -> bool {
        self.attack(id)
            .map(|definition| definition.is_root)
            .unwrap_or(false)
    }

    /// 所有起手招式（按 ID 排序，结果稳定）。
    pub fn root_attacks(&self) -> Vec<AttackId> {
        let mut roots: Vec<AttackId> = self
            .attacks
            .values()
            .filter(|definition| definition.is_root)
            .map(|definition| definition.id)
            .collect();
        roots.sort();
        roots
    }

    /// 招式数量。
    pub fn len(&self) -> usize {
        self.attacks.len()
    }

    /// 是否为空图。
    pub fn is_empty(&self) -> bool {
        self.attacks.is_empty()
    }
}

/// 连招配置。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComboConfig {
    /// 预输入缓冲时间（秒），允许在窗口开始前提前按键。
    pub input_buffer_time: f32,
}

impl Default for ComboConfig {
    fn default() -> Self {
        Self {
            input_buffer_time: 0.15,
        }
    }
}

/// 攻击输入处理结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboInputResult {
    /// 新起手了一段攻击。
    Started(AttackId),
    /// 已排队，等待连招窗口/下一段。
    Queued(AttackId),
    /// 输入被拒绝（不在窗口、不能衔接、招式不存在等）。
    Rejected,
}

impl ComboInputResult {
    /// 输入是否被接受（起手或排队）。
    pub fn is_accepted(self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

/// 当前正在进行的连招动作。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComboAction {
    attack: AttackId,
    elapsed: f32,
    queued: Option<AttackId>,
}

impl ComboAction {
    /// 从招式开始一个新的动作。
    pub fn new(attack: AttackId) -> Self {
        Self {
            attack,
            elapsed: 0.0,
            queued: None,
        }
    }

    /// 当前招式 ID。
    pub fn attack(&self) -> AttackId {
        self.attack
    }

    /// 已进行时间。
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// 已排队的下一段。
    pub fn queued(&self) -> Option<AttackId> {
        self.queued
    }

    /// 推进时间。
    pub fn advance(&mut self, dt: f32) {
        self.elapsed += dt.max(0.0);
    }

    /// 当前阶段。
    pub fn phase(&self, definition: &AttackDefinition) -> AttackPhase {
        if self.elapsed < definition.startup {
            AttackPhase::Startup
        } else if self.elapsed < definition.startup + definition.active {
            AttackPhase::Active
        } else if self.elapsed < definition.total_duration() {
            AttackPhase::Recovery
        } else {
            AttackPhase::Finished
        }
    }

    /// 判定是否生效。
    pub fn is_hit_window_open(&self, definition: &AttackDefinition) -> bool {
        self.elapsed >= definition.startup && self.elapsed < definition.startup + definition.active
    }
}

/// 一帧的连招状态输出。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComboUpdate {
    /// 当前招式。
    pub action: Option<AttackId>,
    /// 当前阶段。
    pub phase: Option<AttackPhase>,
    /// 判定是否生效。
    pub hitbox_active: bool,
    /// 生效中的命中盒（未生效为 `None`）。
    pub hitbox: Option<Hitbox>,
    /// 本帧新起手。
    pub started: Option<AttackId>,
    /// 本帧衔接的下一段。
    pub chained: Option<AttackId>,
    /// 本帧结束的招式。
    pub finished: Option<AttackId>,
}

impl ComboUpdate {
    /// 是否正在攻击。
    pub fn is_attacking(&self) -> bool {
        self.action.is_some()
    }
}

/// 连招状态机。
#[derive(Clone, Debug, Default)]
pub struct ComboState {
    current: Option<ComboAction>,
}

impl ComboState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前连招动作。
    pub fn current(&self) -> Option<ComboAction> {
        self.current
    }

    /// 当前招式 ID。
    pub fn current_attack(&self) -> Option<AttackId> {
        self.current.map(|action| action.attack)
    }

    /// 是否正在攻击。
    pub fn is_attacking(&self) -> bool {
        self.current.is_some()
    }

    /// 立即清空连招状态（受击打断/死亡/重置）。
    pub fn cancel(&mut self) {
        self.current = None;
    }

    /// 处理一次攻击输入。
    ///
    /// - 无当前动作：只有 `is_root` 的招式可以起手；
    /// - 有当前动作：需满足 `can_chain` 且处于连招窗口内（或处于预输入缓冲期）。
    pub fn press(
        &mut self,
        attack: AttackId,
        graph: &ComboGraph,
        config: &ComboConfig,
    ) -> ComboInputResult {
        let Some(definition) = graph.attack(attack) else {
            return ComboInputResult::Rejected;
        };

        if let Some(mut current) = self.current {
            let Some(current_definition) = graph.attack(current.attack) else {
                self.current = None;
                return ComboInputResult::Rejected;
            };
            if !graph.can_chain(current.attack, attack) {
                return ComboInputResult::Rejected;
            }
            let window = current_definition.combo_window;
            let now = current.elapsed;
            let in_window = window.contains(now);
            let early_buffer =
                now < window.start && now + config.input_buffer_time.max(0.0) >= window.start;
            if in_window || early_buffer {
                current.queued = Some(attack);
                self.current = Some(current);
                ComboInputResult::Queued(attack)
            } else {
                ComboInputResult::Rejected
            }
        } else if definition.is_root {
            self.current = Some(ComboAction::new(attack));
            ComboInputResult::Started(attack)
        } else {
            ComboInputResult::Rejected
        }
    }

    /// 推进连招状态机。
    pub fn update(&mut self, dt: f32, graph: &ComboGraph) -> ComboUpdate {
        let Some(mut current) = self.current else {
            return ComboUpdate::default();
        };
        current.advance(dt);
        let Some(definition) = graph.attack(current.attack) else {
            self.current = None;
            return ComboUpdate {
                action: None,
                phase: Some(AttackPhase::Finished),
                finished: Some(current.attack),
                ..ComboUpdate::default()
            };
        };

        let total_duration = definition.total_duration();
        let combo_window = definition.combo_window;
        let hitbox_open = current.is_hit_window_open(definition);
        let hitbox = definition.hitbox;
        let phase = current.phase(definition);

        // 连招衔接：排队输入在窗口开始后立即生效（可取消当前后摇/判定）。
        if let Some(next) = current.queued {
            if current.elapsed >= combo_window.start
                && current.elapsed <= combo_window.end
                && graph.can_chain(current.attack, next)
            {
                let next_action = ComboAction::new(next);
                let next_definition = graph.attack(next);
                let (next_phase, next_hitbox) = match next_definition {
                    Some(next_definition) => (
                        next_action.phase(next_definition),
                        if next_action.is_hit_window_open(next_definition) {
                            Some(next_definition.hitbox)
                        } else {
                            None
                        },
                    ),
                    None => (AttackPhase::Startup, None),
                };
                self.current = Some(next_action);
                return ComboUpdate {
                    action: Some(next),
                    phase: Some(next_phase),
                    hitbox_active: next_hitbox.is_some(),
                    hitbox: next_hitbox,
                    started: Some(next),
                    chained: Some(next),
                    finished: None,
                };
            }
        }

        if current.elapsed >= total_duration {
            self.current = None;
            return ComboUpdate {
                action: None,
                phase: Some(AttackPhase::Finished),
                hitbox_active: false,
                hitbox: None,
                started: None,
                chained: None,
                finished: Some(current.attack),
            };
        }

        self.current = Some(current);
        ComboUpdate {
            action: Some(current.attack),
            phase: Some(phase),
            hitbox_active: hitbox_open,
            hitbox: if hitbox_open { Some(hitbox) } else { None },
            started: None,
            chained: None,
            finished: None,
        }
    }

    /// 当前生效中的命中盒。
    pub fn active_hitbox(&self, graph: &ComboGraph) -> Option<Hitbox> {
        let current = self.current?;
        let definition = graph.attack(current.attack)?;
        if current.is_hit_window_open(definition) {
            Some(definition.hitbox)
        } else {
            None
        }
    }

    /// 当前攻击期间期望的动画状态（供 `animation_bridge` 使用）。
    pub fn action_state(&self, graph: &ComboGraph) -> Option<ActionState> {
        let current = self.current?;
        let definition = graph.attack(current.attack)?;
        Some(match definition.kind {
            AttackKind::Heavy => ActionState::HeavyAttack(definition.action),
            AttackKind::Light | AttackKind::Special => ActionState::Attack(definition.action),
        })
    }

    /// 当前攻击期间自动前冲速度（供 `MotorInput::external_velocity` 使用）。
    pub fn forward_push_velocity(&self, graph: &ComboGraph, forward: Vec3) -> Vec3 {
        let Some(current) = self.current else {
            return Vec3::ZERO;
        };
        let Some(definition) = graph.attack(current.attack) else {
            return Vec3::ZERO;
        };
        if definition.forward_push.abs() <= f32::EPSILON {
            return Vec3::ZERO;
        }
        forward.horizontal().normalized_or(Vec3::FORWARD) * definition.forward_push
    }
}
