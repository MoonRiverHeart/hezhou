//! Dodge：闪避与无敌帧。
//!
//! 组件只维护闪避时序与位移速度，实际位移由 `CharacterMotor` 的
//! `external_velocity` 通道驱动，避免和移动逻辑重复积分。

use crate::math::Vec3;

/// 闪避阶段。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DodgePhase {
    /// 可以闪避。
    #[default]
    Ready,
    /// 闪避动作进行中。
    Active,
    /// 冷却中。
    Cooldown,
}

/// 闪避参数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DodgeConfig {
    /// 闪避动作总时长（秒）。
    pub duration: f32,
    /// 无敌帧开始时间（相对闪避开始，秒）。
    pub invincible_start: f32,
    /// 无敌帧结束时间（相对闪避开始，秒）。
    pub invincible_end: f32,
    /// 冷却时间（秒），从闪避结束开始计算。
    pub cooldown: f32,
    /// 闪避位移速度（米/秒）。
    pub speed: f32,
}

impl Default for DodgeConfig {
    fn default() -> Self {
        Self {
            duration: 0.4,
            invincible_start: 0.05,
            invincible_end: 0.3,
            cooldown: 0.6,
            speed: 9.0,
        }
    }
}

/// 闪避组件。
#[derive(Clone, Copy, Debug)]
pub struct Dodge {
    config: DodgeConfig,
    phase: DodgePhase,
    elapsed: f32,
    cooldown_remaining: f32,
    direction: Vec3,
}

impl Dodge {
    pub fn new(config: DodgeConfig) -> Self {
        Self {
            config,
            phase: DodgePhase::Ready,
            elapsed: 0.0,
            cooldown_remaining: 0.0,
            direction: Vec3::FORWARD,
        }
    }

    pub fn config(&self) -> &DodgeConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: DodgeConfig) {
        self.config = config;
    }

    /// 当前阶段。
    pub fn phase(&self) -> DodgePhase {
        self.phase
    }

    /// 当前闪避已进行时间。
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// 冷却剩余时间。
    pub fn cooldown_remaining(&self) -> f32 {
        self.cooldown_remaining
    }

    /// 当前闪避方向（水平单位向量）。
    pub fn direction(&self) -> Vec3 {
        self.direction
    }

    /// 是否处于闪避动作中。
    pub fn is_active(&self) -> bool {
        self.phase == DodgePhase::Active
    }

    /// 是否处于无敌帧。
    pub fn is_invincible(&self) -> bool {
        self.phase == DodgePhase::Active
            && self.elapsed >= self.config.invincible_start
            && self.elapsed < self.config.invincible_end
    }

    /// 尝试开始闪避；`direction` 为零时使用前向。
    ///
    /// 冷却中或已在闪避中返回 `false`。
    pub fn try_start(&mut self, direction: Vec3) -> bool {
        if self.phase != DodgePhase::Ready {
            return false;
        }
        self.direction = direction.horizontal().normalized_or(Vec3::FORWARD);
        self.phase = DodgePhase::Active;
        self.elapsed = 0.0;
        true
    }

    /// 推进闪避状态机。
    pub fn update(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        match self.phase {
            DodgePhase::Ready => {}
            DodgePhase::Active => {
                self.elapsed += dt;
                if self.elapsed >= self.config.duration.max(0.0) {
                    self.elapsed = self.config.duration.max(0.0);
                    self.phase = DodgePhase::Cooldown;
                    self.cooldown_remaining = self.config.cooldown.max(0.0);
                }
            }
            DodgePhase::Cooldown => {
                self.cooldown_remaining -= dt;
                if self.cooldown_remaining <= 0.0 {
                    self.cooldown_remaining = 0.0;
                    self.phase = DodgePhase::Ready;
                }
            }
        }
    }

    /// 闪避位移速度；未在闪避中时为零向量。
    ///
    /// 集成层通常把它放进 `MotorInput::external_velocity`。
    pub fn velocity(&self) -> Vec3 {
        if self.phase == DodgePhase::Active {
            self.direction * self.config.speed
        } else {
            Vec3::ZERO
        }
    }

    /// 强制取消（受击打断/调试用），回到可闪避状态。
    pub fn cancel(&mut self) {
        self.phase = DodgePhase::Ready;
        self.elapsed = 0.0;
        self.cooldown_remaining = 0.0;
    }
}

impl Default for Dodge {
    fn default() -> Self {
        Self::new(DodgeConfig::default())
    }
}
