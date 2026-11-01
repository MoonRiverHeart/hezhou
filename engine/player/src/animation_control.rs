//! Player 蒙皮动画运行时状态（task-34）：窗口 host 维护当前 clip 列表/索引/时间/速度/
//! 播放状态，joint palette 由该状态驱动（不再固定 first clip + elapsed）。
//!
//! C# 通过 `PlayerCSharpContext` v3 的 animation 函数指针查询/控制本状态；Rust 每帧
//! 调用 [`PlayerAnimationState::advance`] 推进时间，再用 [`SelectedSkinnedAsset::palette_for_clip`]
//! 采样当前 clip/time 生成 palette。
//!
//! 限制（0.2）：单 clip 切换，无混合/状态机/事件；时间循环 0..duration；
//! speed 限制在 [-8, 8]；无 clip 时状态为空（current=-1）。

use crate::skinned_asset::SelectedSkinnedAsset;

/// 单个动画 clip 的运行时信息。
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClipEntry {
    /// clip 名（glTF 中可选）。
    pub name: Option<String>,
    /// 时长（秒，>0）。
    pub duration: f32,
}

/// 蒙皮动画运行时状态（窗口 host 持有）。
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerAnimationState {
    clips: Vec<AnimationClipEntry>,
    current: i32,
    time: f32,
    speed: f32,
    playing: bool,
}

/// 动画运行时操作错误（C# ABI 返回 0/-1 前用于日志/边界）。
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationControlError {
    /// 没有 clip（无蒙皮/无动画）。
    NoClips,
    /// clip 下标越界。
    ClipOutOfRange { index: usize, count: usize },
    /// speed 非有限值。
    InvalidSpeed { speed: f32 },
}

impl std::fmt::Display for AnimationControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoClips => write!(f, "没有可播放的动画 clip"),
            Self::ClipOutOfRange { index, count } => {
                write!(f, "clip 下标 {index} 越界（共 {count} 个）")
            }
            Self::InvalidSpeed { speed } => write!(f, "非法播放速度 {speed}"),
        }
    }
}

impl std::error::Error for AnimationControlError {}

/// 无动画 clip 的默认速度。
pub const DEFAULT_ANIMATION_SPEED: f32 = 1.0;
/// speed 允许范围。
pub const MAX_ANIMATION_SPEED: f32 = 8.0;

impl PlayerAnimationState {
    /// 空状态（无蒙皮/无动画）。
    pub fn empty() -> Self {
        Self {
            clips: Vec::new(),
            current: -1,
            time: 0.0,
            speed: DEFAULT_ANIMATION_SPEED,
            playing: false,
        }
    }

    /// 从 clip 列表构造；默认选中第一个 clip、time=0、speed=1、playing=true。
    pub fn new(clips: Vec<AnimationClipEntry>) -> Self {
        let has_clips = !clips.is_empty();
        let current = if has_clips { 0 } else { -1 };
        Self {
            clips,
            current,
            time: 0.0,
            speed: DEFAULT_ANIMATION_SPEED,
            playing: has_clips,
        }
    }

    /// 从选中的蒙皮资产构造：clip 列表来自 `ImportedScene.animations`
    /// （无蒙皮/无动画 → 空状态）。
    pub fn from_selected_asset(selected: Option<&SelectedSkinnedAsset>) -> Self {
        let Some(asset) = selected else {
            return Self::empty();
        };
        let clips = asset
            .scene
            .animations
            .iter()
            .map(|animation| AnimationClipEntry {
                name: animation.name.clone(),
                duration: animation.duration.max(0.0),
            })
            .collect();
        Self::new(clips)
    }

    /// clip 数量。
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    /// clip 列表。
    pub fn clips(&self) -> &[AnimationClipEntry] {
        &self.clips
    }

    /// 按 index 查询 clip。
    pub fn clip(&self, index: usize) -> Option<&AnimationClipEntry> {
        self.clips.get(index)
    }

    /// 按 index 查询 clip 名（越界 None）。
    pub fn clip_name(&self, index: usize) -> Option<Option<&str>> {
        self.clips.get(index).map(|clip| clip.name.as_deref())
    }

    /// 按 index 查询时长（越界 None）。
    pub fn clip_duration(&self, index: usize) -> Option<f32> {
        self.clips.get(index).map(|clip| clip.duration)
    }

    /// 当前 clip 下标（-1 = 无）。
    pub fn current(&self) -> i32 {
        self.current
    }

    /// 当前 clip 的 usize 下标（无 clip → None）。
    pub fn current_index(&self) -> Option<usize> {
        if self.current < 0 {
            None
        } else {
            Some(self.current as usize)
        }
    }

    /// 当前时间（秒）。
    pub fn time(&self) -> f32 {
        self.time
    }

    /// 当前播放速度。
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// 是否在播放。
    pub fn is_playing(&self) -> bool {
        self.playing && self.current_index().is_some()
    }

    /// 播放指定 clip（重置 time=0；越界返回错误）。
    pub fn play(&mut self, index: usize) -> Result<(), AnimationControlError> {
        if index >= self.clips.len() {
            return Err(AnimationControlError::ClipOutOfRange {
                index,
                count: self.clips.len(),
            });
        }
        self.current = index as i32;
        self.time = 0.0;
        self.playing = true;
        Ok(())
    }

    /// 设置当前时间（秒；clamp 到 [0, duration]；无 clip 时返回错误）。
    pub fn set_time(&mut self, seconds: f32) -> Result<(), AnimationControlError> {
        let Some(index) = self.current_index() else {
            return Err(AnimationControlError::NoClips);
        };
        let duration = self.clips[index].duration;
        if !seconds.is_finite() {
            return Err(AnimationControlError::NoClips);
        }
        self.time = if duration > 0.0 {
            seconds.clamp(0.0, duration)
        } else {
            0.0
        };
        Ok(())
    }

    /// 设置播放速度（有限值，clamp 到 [-8, 8]）。
    pub fn set_speed(&mut self, speed: f32) -> Result<(), AnimationControlError> {
        if !speed.is_finite() {
            return Err(AnimationControlError::InvalidSpeed { speed });
        }
        self.speed = speed.clamp(-MAX_ANIMATION_SPEED, MAX_ANIMATION_SPEED);
        Ok(())
    }

    /// 设置是否播放。
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }

    /// 每帧推进时间（仅在 playing 且当前 clip 时长 > 0 时）。
    pub fn advance(&mut self, dt_seconds: f32) {
        if !self.playing {
            return;
        }
        let Some(index) = self.current_index() else {
            return;
        };
        let duration = self.clips[index].duration;
        if duration <= 0.0 || !dt_seconds.is_finite() || dt_seconds <= 0.0 {
            return;
        }
        let next = self.time + dt_seconds * self.speed;
        self.time = next.rem_euclid(duration);
    }

    /// 单行状态摘要（日志/诊断）。
    pub fn summary(&self) -> String {
        if self.clips.is_empty() {
            return "无动画 clip".to_string();
        }
        let clip = &self.clips[self.current.max(0) as usize];
        format!(
            "clip {}/{} `{}` t={:.3}/{:.3}s speed={:.2} playing={}",
            self.current,
            self.clips.len(),
            clip.name.as_deref().unwrap_or("<unnamed>"),
            self.time,
            clip.duration,
            self.speed,
            self.playing
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clips() -> Vec<AnimationClipEntry> {
        vec![
            AnimationClipEntry {
                name: Some("Idle".to_string()),
                duration: 1.0,
            },
            AnimationClipEntry {
                name: Some("Run".to_string()),
                duration: 0.5,
            },
        ]
    }

    #[test]
    fn default_state_selects_first_clip() {
        let state = PlayerAnimationState::new(clips());
        assert_eq!(state.clip_count(), 2);
        assert_eq!(state.current(), 0);
        assert_eq!(state.time(), 0.0);
        assert_eq!(state.speed(), 1.0);
        assert!(state.is_playing());
        assert_eq!(state.clip_name(0), Some(Some("Idle")));
        assert_eq!(state.clip_duration(1), Some(0.5));
        assert_eq!(state.clip_name(2), None);
    }

    #[test]
    fn empty_state_is_safe() {
        let mut state = PlayerAnimationState::empty();
        assert_eq!(state.clip_count(), 0);
        assert_eq!(state.current(), -1);
        assert!(state.current_index().is_none());
        assert!(!state.is_playing());
        state.advance(1.0);
        assert!(state.play(0).is_err());
        assert!(state.set_time(0.5).is_err());
        assert!(state.set_speed(2.0).is_ok());
        assert!(state.summary().contains("无动画"));
    }

    #[test]
    fn play_sets_clip_and_resets_time() {
        let mut state = PlayerAnimationState::new(clips());
        state.set_time(0.75).unwrap();
        assert!((state.time() - 0.75).abs() < 1e-6);
        state.play(1).unwrap();
        assert_eq!(state.current(), 1);
        assert_eq!(state.time(), 0.0);
        assert_eq!(state.clip_name(1), Some(Some("Run")));
        assert!(state.play(2).is_err());
    }

    #[test]
    fn time_and_speed_are_clamped() {
        let mut state = PlayerAnimationState::new(clips());
        state.set_time(99.0).unwrap();
        assert!((state.time() - 1.0).abs() < 1e-6);
        state.set_time(-1.0).unwrap();
        assert_eq!(state.time(), 0.0);
        state.set_speed(100.0).unwrap();
        assert!((state.speed() - MAX_ANIMATION_SPEED).abs() < 1e-6);
        state.set_speed(-100.0).unwrap();
        assert!((state.speed() + MAX_ANIMATION_SPEED).abs() < 1e-6);
        assert!(state.set_speed(f32::NAN).is_err());
    }

    #[test]
    fn advance_loops_with_speed_and_pauses() {
        let mut state = PlayerAnimationState::new(clips());
        state.set_speed(2.0).unwrap();
        state.advance(0.2);
        assert!((state.time() - 0.4).abs() < 1e-6);
        state.advance(0.4);
        assert!((state.time() - 0.2).abs() < 1e-6, "应循环: {}", state.time());
        state.set_playing(false);
        state.advance(0.5);
        assert!((state.time() - 0.2).abs() < 1e-6);
        assert!(!state.is_playing());
    }

    #[test]
    fn summary_contains_clip_and_time() {
        let state = PlayerAnimationState::new(clips());
        let summary = state.summary();
        assert!(summary.contains("clip 0/2"));
        assert!(summary.contains("Idle"));
        assert!(summary.contains("speed=1.00"));
    }
}
