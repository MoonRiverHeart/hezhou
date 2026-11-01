//! Health：生命值与死亡判定。
//!
//! 纯数据组件；受击/格挡/闪避的结算由 [`crate::combat::damage::resolve_damage`] 完成。

/// 生命值组件。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Health {
    max: f32,
    current: f32,
}

impl Health {
    /// 构造满血生命值；`max` 为负时按 0 处理。
    pub fn new(max: f32) -> Self {
        let max = max.max(0.0);
        Self { max, current: max }
    }

    /// 最大生命值。
    pub fn max(&self) -> f32 {
        self.max
    }

    /// 当前生命值。
    pub fn current(&self) -> f32 {
        self.current
    }

    /// 生命值比例（0..=1）。
    pub fn ratio(&self) -> f32 {
        if self.max <= f32::EPSILON {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    /// 是否已死亡。
    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    /// 扣血，返回实际扣除量；死亡后不再扣血。
    pub fn apply_damage(&mut self, amount: f32) -> f32 {
        if amount <= 0.0 || self.is_dead() {
            return 0.0;
        }
        let dealt = amount.min(self.current);
        self.current -= dealt;
        if self.current <= 0.0 {
            self.current = 0.0;
        }
        dealt
    }

    /// 回血，返回实际恢复量。
    pub fn heal(&mut self, amount: f32) -> f32 {
        if amount <= 0.0 || self.is_dead() {
            return 0.0;
        }
        let healed = amount.min(self.max - self.current).max(0.0);
        self.current += healed;
        healed
    }

    /// 直接设置当前生命值（复活/存档恢复用），会被夹到 `[0, max]`。
    pub fn set_current(&mut self, value: f32) {
        self.current = value.clamp(0.0, self.max);
    }

    /// 修改最大生命值：`current` 会同步夹紧；提高上限不会自动回血。
    pub fn set_max(&mut self, max: f32) {
        self.max = max.max(0.0);
        self.current = self.current.clamp(0.0, self.max);
    }

    /// 重置为满血（复活用）。
    pub fn reset(&mut self) {
        self.current = self.max;
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::new(100.0)
    }
}
