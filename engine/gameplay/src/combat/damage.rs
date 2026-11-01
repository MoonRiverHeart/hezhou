//! DamageEvent：伤害事件数据与结算管线。
//!
//! [`resolve_damage`] 是纯函数：输入生命值、格挡、闪避与伤害事件，
//! 输出是否命中/格挡/闪避、实际扣血、是否死亡、击退强度。
//! 它不关心伤害来自哪个攻击、也不绑定渲染，便于组合与测试。

use crate::combat::block::Block;
use crate::combat::dodge::Dodge;
use crate::combat::health::Health;
use crate::combat::hitbox::HitContact;
use crate::ids::CombatantId;
use crate::math::Vec3;

/// 伤害类型。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DamageKind {
    /// 轻攻击/普通伤害。
    #[default]
    Light,
    /// 重攻击。
    Heavy,
    /// 特殊/技能伤害。
    Special,
    /// 环境伤害（坠落等）。
    Environmental,
}

/// 一次伤害事件。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageEvent {
    /// 伤害来源（攻击者 ID）。
    pub source: CombatantId,
    /// 受击目标。
    pub target: CombatantId,
    /// 伤害数值。
    pub amount: f32,
    /// 伤害类型。
    pub kind: DamageKind,
    /// 命中点（世界空间）。
    pub hit_point: Vec3,
    /// 命中方向：从攻击者指向目标（单位向量，格挡判定用）。
    pub hit_direction: Vec3,
    /// 击退强度（距离/速度，由物理/控制器决定如何使用）。
    pub knockback: f32,
    /// 是否无视格挡。
    pub unblockable: bool,
}

impl DamageEvent {
    /// 构造最小伤害事件（默认轻击、无击退、可格挡）。
    pub fn new(source: CombatantId, target: CombatantId, amount: f32) -> Self {
        Self {
            source,
            target,
            amount,
            kind: DamageKind::Light,
            hit_point: Vec3::ZERO,
            hit_direction: Vec3::FORWARD,
            knockback: 0.0,
            unblockable: false,
        }
    }

    /// 由命中盒接触点构造伤害事件。
    ///
    /// `hit_direction` 取接触面法线的反方向（法线指向攻击者）。
    pub fn from_contact(
        source: CombatantId,
        contact: &HitContact,
        amount: f32,
        kind: DamageKind,
    ) -> Self {
        let mut event = Self::new(source, contact.target, amount);
        event.kind = kind;
        event.hit_point = contact.point;
        event.hit_direction = (-contact.normal).normalized_or(Vec3::FORWARD);
        event
    }

    pub fn with_kind(mut self, kind: DamageKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_hit_point(mut self, point: Vec3) -> Self {
        self.hit_point = point;
        self
    }

    pub fn with_hit_direction(mut self, direction: Vec3) -> Self {
        self.hit_direction = direction.normalized_or(Vec3::FORWARD);
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
}

/// 结算结果分类。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DamageResult {
    /// 正常命中。
    #[default]
    Applied,
    /// 被格挡。
    Blocked,
    /// 被闪避无敌帧躲开。
    Dodged,
    /// 目标已经死亡。
    AlreadyDead,
}

/// 一次伤害结算的完整结果。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DamageOutcome {
    /// 结算分类。
    pub result: DamageResult,
    /// 实际扣除的生命值。
    pub damage: f32,
    /// 被格挡吸收的伤害量。
    pub guard_damage: f32,
    /// 本次结算是否导致破防。
    pub guard_broken: bool,
    /// 目标是否死亡。
    pub killed: bool,
    /// 目标剩余生命值。
    pub remaining_health: f32,
    /// 实际击退强度（格挡会削减、闪避为 0）。
    pub knockback: f32,
}

/// 结算一次伤害。
///
/// 顺序：已死亡 → 闪避无敌帧 → 格挡（方向/是否可格挡）→ 生命值。
/// `target_forward` 是目标面向（水平单位向量），用于格挡方向判定。
pub fn resolve_damage(
    health: &mut Health,
    block: Option<&mut Block>,
    dodge: Option<&Dodge>,
    target_forward: Vec3,
    event: &DamageEvent,
) -> DamageOutcome {
    if health.is_dead() {
        return DamageOutcome {
            result: DamageResult::AlreadyDead,
            remaining_health: health.current(),
            ..DamageOutcome::default()
        };
    }

    if dodge.map(|dodge| dodge.is_invincible()).unwrap_or(false) {
        return DamageOutcome {
            result: DamageResult::Dodged,
            knockback: 0.0,
            remaining_health: health.current(),
            ..DamageOutcome::default()
        };
    }

    if let Some(block) = block {
        if !event.unblockable
            && block.is_guarding()
            && !block.is_guard_broken()
            && block.can_block(target_forward, event.hit_direction)
        {
            let absorb = block.absorb(event.amount);
            let damage = health.apply_damage(absorb.damage);
            return DamageOutcome {
                result: DamageResult::Blocked,
                damage,
                guard_damage: absorb.guard_damage,
                guard_broken: absorb.broken,
                killed: health.is_dead(),
                remaining_health: health.current(),
                knockback: event.knockback * 0.25,
            };
        }
    }

    let damage = health.apply_damage(event.amount);
    DamageOutcome {
        result: DamageResult::Applied,
        damage,
        guard_damage: 0.0,
        guard_broken: false,
        killed: health.is_dead(),
        remaining_health: health.current(),
        knockback: event.knockback,
    }
}
