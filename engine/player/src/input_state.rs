//! Player 输入快照（task-59）：窗口 host 每帧从 renderer `is_key_down` 生成，
//! C# 通过 `PlayerCSharpContext` v7 的 `input_axis_x/input_axis_z/input_jump_pressed`
//! 读取。
//!
//! 轴语义（C# 侧直接乘速度使用）：
//! - `axis_x`：+1 = 右（D / Right），-1 = 左（A / Left）；
//! - `axis_z`：+1 = 前（W / Up），-1 = 后（S / Down）；
//! - `jump_pressed`：Space 的**按下沿**（每帧只在首次按下时为 1，避免长按连跳）。

/// 一帧输入快照。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerInputState {
    axis_x: f32,
    axis_z: f32,
    jump_pressed: bool,
}

impl PlayerInputState {
    /// 空输入（未按键/无窗口）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置本帧输入快照。
    pub fn set(&mut self, axis_x: f32, axis_z: f32, jump_pressed: bool) {
        self.axis_x = if axis_x.is_finite() {
            axis_x.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.axis_z = if axis_z.is_finite() {
            axis_z.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.jump_pressed = jump_pressed;
    }

    /// 清空输入（窗口失焦/暂停）。
    pub fn clear(&mut self) {
        self.axis_x = 0.0;
        self.axis_z = 0.0;
        self.jump_pressed = false;
    }

    /// 水平 X 轴（右 +）。
    pub fn axis_x(&self) -> f32 {
        self.axis_x
    }

    /// 水平 Z 轴（前 +）。
    pub fn axis_z(&self) -> f32 {
        self.axis_z
    }

    /// 本帧是否有跳跃按下沿。
    pub fn jump_pressed(&self) -> bool {
        self.jump_pressed
    }

    /// 是否无输入（含 jump）。
    pub fn is_idle(&self) -> bool {
        self.axis_x == 0.0 && self.axis_z == 0.0 && !self.jump_pressed
    }

    /// 单行摘要。
    pub fn summary(&self) -> String {
        format!(
            "input x={:.2} z={:.2} jump={}",
            self.axis_x, self.axis_z, self.jump_pressed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_and_jump_are_clamped_and_readable() {
        let mut input = PlayerInputState::new();
        assert!(input.is_idle());
        input.set(2.0, -2.0, true);
        assert_eq!(input.axis_x(), 1.0);
        assert_eq!(input.axis_z(), -1.0);
        assert!(input.jump_pressed());
        input.set(f32::NAN, 0.5, false);
        assert_eq!(input.axis_x(), 0.0);
        assert_eq!(input.axis_z(), 0.5);
        assert!(input.summary().contains("x=0.00"));
        input.clear();
        assert!(input.is_idle());
    }
}
