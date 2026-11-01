//! HeavyAttack：重攻击蓄力。
//!
//! 组件只负责蓄力时序与伤害倍率；具体招式（伤害、命中盒、连招分支）
//! 由 `ComboGraph` 中的 `AttackDefinition` 描述，两者通过集成层组合。

/// 重攻击蓄力参数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeavyAttackConfig {
    /// 最短蓄力时间（秒），低于该时间释放不构成重攻击。
    pub min_charge: f32,
    /// 满蓄力时间（秒）。
    pub max_charge: f32,
    /// 最短蓄力对应的伤害倍率。
    pub min_damage_multiplier: f32,
    /// 满蓄力对应的伤害倍率。
    pub max_damage_multiplier: f32,
    /// 蓄力期间移动速度倍率（供 `MotorInput::speed_scale` 使用）。
    pub movement_scale_while_charging: f32,
}

impl Default for HeavyAttackConfig {
    fn default() -> Self {
        Self {
            min_charge: 0.2,
            max_charge: 1.0,
            min_damage_multiplier: 1.0,
            max_damage_multiplier: 2.0,
            movement_scale_while_charging: 0.4,
        }
    }
}

/// 释放结果。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeavyAttackRelease {
    /// 蓄力比例（0..=1）。
    pub charge_ratio: f32,
    /// 伤害倍率（介于 min/max 之间）。
    pub damage_multiplier: f32,
}

/// 重攻击蓄力组件。
#[derive(Clone, Copy, Debug)]
pub struct HeavyAttack {
    config: HeavyAttackConfig,
    charging: bool,
    elapsed: f32,
    released: Option<HeavyAttackRelease>,
}

impl HeavyAttack {
    pub fn new(config: HeavyAttackConfig) -> Self {
        Self {
            config,
            charging: false,
            elapsed: 0.0,
            released: None,
        }
    }

    pub fn config(&self) -> &HeavyAttackConfig {
        &self.config
    }

    /// 是否正在蓄力。
    pub fn is_charging(&self) -> bool {
        self.charging
    }

    /// 本次蓄力已累计时间。
    pub fn charge_elapsed(&self) -> f32 {
        self.elapsed
    }

    /// 蓄力比例（0..=1）。
    pub fn charge_ratio(&self) -> f32 {
        if self.config.max_charge <= f32::EPSILON {
            if self.charging {
                1.0
            } else {
                0.0
            }
        } else {
            (self.elapsed / self.config.max_charge).clamp(0.0, 1.0)
        }
    }

    /// 是否已达到满蓄力。
    pub fn is_fully_charged(&self) -> bool {
        self.charging && self.elapsed >= self.config.max_charge
    }

    /// 蓄力期间的移动速度倍率；未蓄力时为 1.0。
    pub fn movement_scale(&self) -> f32 {
        if self.charging {
            self.config.movement_scale_while_charging
        } else {
            1.0
        }
    }

    /// 最近一次释放结果（用于表现层/测试读取）。
    pub fn last_release(&self) -> Option<HeavyAttackRelease> {
        self.released
    }

    /// 开始蓄力；已在蓄力中返回 `false`。
    pub fn begin_charge(&mut self) -> bool {
        if self.charging {
            return false;
        }
        self.charging = true;
        self.elapsed = 0.0;
        self.released = None;
        true
    }

    /// 推进蓄力计时。
    pub fn update(&mut self, dt: f32) {
        if self.charging {
            self.elapsed += dt.max(0.0);
        }
    }

    /// 取消蓄力（不产生释放结果）。
    pub fn cancel(&mut self) {
        self.charging = false;
        self.elapsed = 0.0;
        self.released = None;
    }

    /// 释放蓄力。
    ///
    /// 蓄力时间不足 `min_charge` 时返回 `None`（视为普通攻击输入）；
    /// 否则返回蓄力比例与伤害倍率，并结束蓄力状态。
    pub fn release(&mut self) -> Option<HeavyAttackRelease> {
        if !self.charging {
            return None;
        }
        let elapsed = self.elapsed;
        self.charging = false;
        self.elapsed = 0.0;
        if elapsed < self.config.min_charge {
            self.released = None;
            return None;
        }
        let ratio = if self.config.max_charge <= f32::EPSILON {
            1.0
        } else {
            (elapsed / self.config.max_charge).clamp(0.0, 1.0)
        };
        let multiplier = self.config.min_damage_multiplier
            + (self.config.max_damage_multiplier - self.config.min_damage_multiplier) * ratio;
        let result = HeavyAttackRelease {
            charge_ratio: ratio,
            damage_multiplier: multiplier,
        };
        self.released = Some(result);
        Some(result)
    }
}

impl Default for HeavyAttack {
    fn default() -> Self {
        Self::new(HeavyAttackConfig::default())
    }
}
