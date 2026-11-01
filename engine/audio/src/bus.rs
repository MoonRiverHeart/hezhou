//! 音频总线（Bus）与音量规则。
//!
//! 总线是“成组音量控制”的最小抽象：音效挂到某条总线上，调整总线音量/静音
//! 会同时影响该总线上的全部音效。0.2 第一版总线只做增益（不含 EQ/压缩等效果），
//! 后续可在混音器里扩展每总线效果链。
//!
//! 音量规则统一为：
//! - 取值范围 `[0.0, 1.0]`；
//! - `NaN` 按 `0.0`（静音）处理，避免污染混音结果；
//! - 主音量（master）独立于总线，最终增益 = master × bus × voice × 3D 衰减。

/// 默认总线名：所有未指定总线的音效都挂在这里。
pub const DEFAULT_BUS: &str = "default";

/// 把任意音量值规范到 `[0.0, 1.0]`；NaN 视为 0（静音），保证混音输出安全。
pub fn clamp_volume(volume: f32) -> f32 {
    if volume.is_nan() {
        0.0
    } else {
        volume.clamp(0.0, 1.0)
    }
}

/// 一条音频总线。
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBus {
    name: String,
    volume: f32,
    muted: bool,
}

impl AudioBus {
    /// 以给定名称创建总线（默认音量 1.0，未静音）。
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            volume: 1.0,
            muted: false,
        }
    }

    /// 总线名称。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 设置音量（自动 clamp 到 `[0,1]`，NaN → 0）。
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = clamp_volume(volume);
    }

    /// 当前音量。
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// 设置静音。
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// 是否静音。
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// 总线有效增益（静音时为 0，否则为音量）。
    pub fn effective_gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }
}

impl Default for AudioBus {
    fn default() -> Self {
        Self::new(DEFAULT_BUS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_volume_is_clamped_and_nan_safe() {
        let mut bus = AudioBus::new("sfx");
        assert_eq!(bus.name(), "sfx");
        assert_eq!(bus.volume(), 1.0);

        bus.set_volume(0.5);
        assert_eq!(bus.volume(), 0.5);
        bus.set_volume(1.5);
        assert_eq!(bus.volume(), 1.0);
        bus.set_volume(-0.5);
        assert_eq!(bus.volume(), 0.0);
        bus.set_volume(f32::NAN);
        assert_eq!(bus.volume(), 0.0);
        assert_eq!(bus.effective_gain(), 0.0);
    }

    #[test]
    fn mute_zeroes_effective_gain_and_restores() {
        let mut bus = AudioBus::new("bgm");
        bus.set_volume(0.8);
        bus.set_muted(true);
        assert!(bus.muted());
        assert_eq!(bus.effective_gain(), 0.0);

        bus.set_muted(false);
        assert_eq!(bus.effective_gain(), 0.8);
    }

    #[test]
    fn default_bus_name() {
        let bus = AudioBus::default();
        assert_eq!(bus.name(), DEFAULT_BUS);
        assert_eq!(bus.effective_gain(), 1.0);
    }
}
