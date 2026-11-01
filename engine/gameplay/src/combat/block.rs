//! Block：格挡（减伤 + 护盾消耗 + 破防）。

use crate::math::{angle_between, Vec3};

/// 格挡参数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockConfig {
    /// 减伤比例（0..=1）：1 表示完全吸收，0 表示不吸收。
    pub damage_reduction: f32,
    /// 护盾上限。
    pub guard_max: f32,
    /// 破防后的硬直时间（秒）。
    pub guard_break_stun: f32,
    /// 可格挡的正面夹角（度，总角度；面向方向两侧各一半）。
    pub block_angle_deg: f32,
    /// 未格挡时护盾恢复速度（点/秒）。
    pub guard_regen_per_second: f32,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            damage_reduction: 0.8,
            guard_max: 100.0,
            guard_break_stun: 0.8,
            block_angle_deg: 120.0,
            guard_regen_per_second: 8.0,
        }
    }
}

/// 单次格挡吸收结果。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BlockAbsorb {
    /// 穿透到生命值的伤害。
    pub damage: f32,
    /// 本次消耗的护盾。
    pub guard_damage: f32,
    /// 本次是否破防。
    pub broken: bool,
}

/// 格挡组件。
#[derive(Clone, Copy, Debug)]
pub struct Block {
    config: BlockConfig,
    guarding: bool,
    guard: f32,
    break_timer: f32,
}

impl Block {
    pub fn new(config: BlockConfig) -> Self {
        Self {
            config,
            guarding: false,
            guard: config.guard_max.max(0.0),
            break_timer: 0.0,
        }
    }

    pub fn config(&self) -> &BlockConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: BlockConfig) {
        self.config = config;
        self.guard = self.guard.min(config.guard_max.max(0.0));
    }

    /// 当前护盾值。
    pub fn guard(&self) -> f32 {
        self.guard
    }

    /// 护盾比例（0..=1）。
    pub fn guard_ratio(&self) -> f32 {
        if self.config.guard_max <= f32::EPSILON {
            0.0
        } else {
            (self.guard / self.config.guard_max).clamp(0.0, 1.0)
        }
    }

    /// 是否处于格挡姿态。
    pub fn is_guarding(&self) -> bool {
        self.guarding
    }

    /// 是否破防硬直中。
    pub fn is_guard_broken(&self) -> bool {
        self.break_timer > 0.0
    }

    /// 破防剩余硬直时间。
    pub fn guard_break_remaining(&self) -> f32 {
        self.break_timer
    }

    /// 进入格挡姿态；破防中无法格挡。
    pub fn begin_guard(&mut self) -> bool {
        if self.is_guard_broken() {
            return false;
        }
        self.guarding = true;
        true
    }

    /// 结束格挡姿态。
    pub fn end_guard(&mut self) {
        self.guarding = false;
    }

    /// 判断是否能挡住来自 `hit_direction`（攻击者→目标）方向的攻击。
    ///
    /// 目标是"面向"攻击者：`forward` 与 `-hit_direction` 的夹角需小于半个格挡角。
    pub fn can_block(&self, forward: Vec3, hit_direction: Vec3) -> bool {
        if !self.guarding || self.is_guard_broken() {
            return false;
        }
        let half_angle = (self.config.block_angle_deg * 0.5).to_radians();
        angle_between(forward.horizontal(), (-hit_direction).horizontal()) <= half_angle
    }

    /// 吸收一次伤害。
    ///
    /// 返回穿透伤害、护盾消耗与是否破防；破防会立即结束格挡姿态。
    pub fn absorb(&mut self, incoming: f32) -> BlockAbsorb {
        if incoming <= 0.0 || !self.guarding || self.is_guard_broken() {
            return BlockAbsorb {
                damage: incoming.max(0.0),
                guard_damage: 0.0,
                broken: false,
            };
        }
        let reduction = self.config.damage_reduction.clamp(0.0, 1.0);
        let guard_damage = incoming * reduction;
        let damage = incoming - guard_damage;
        self.guard = (self.guard - guard_damage).max(0.0);
        let broken = self.guard <= 0.0;
        if broken {
            self.guard = 0.0;
            self.guarding = false;
            self.break_timer = self.config.guard_break_stun.max(0.0);
        }
        BlockAbsorb {
            damage,
            guard_damage,
            broken,
        }
    }

    /// 每帧更新：破防硬直倒计时与护盾恢复。
    pub fn update(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        if self.break_timer > 0.0 {
            self.break_timer -= dt;
            if self.break_timer <= 0.0 {
                self.break_timer = 0.0;
                // 破防恢复后先恢复一半护盾，避免立刻再次破防。
                self.guard = self.config.guard_max.max(0.0) * 0.5;
            }
        } else if !self.guarding {
            self.guard = (self.guard + self.config.guard_regen_per_second * dt)
                .min(self.config.guard_max.max(0.0));
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new(BlockConfig::default())
    }
}
