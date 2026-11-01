//! AnimationClip / Channel / Keyframe：关键帧数据、采样与插值、事件区间。
//!
//! 数据模型对齐 glTF：
//! - 一个 clip 由多条 `Channel` 组成，每条 channel 只驱动**一个关节的一个属性**（T/R/S）；
//! - 关键帧按时间升序保存，采样时对 T/S 做线性插值、对 R 做球面插值（slerp）；
//! - `Interpolation::Step` 支持逐帧/硬切动画；`CubicSpline` 仅保留枚举占位（0.2 回退线性，TODO 0.3）。
//!
//! 时间语义：
//! - 所有时间单位为秒；
//! - 循环（`looping`）用 `rem_euclid` 取模；非循环时把时间 clamp 到 `[0, duration]`；
//! - 事件由 [`AnimationClip::events_in_range`] 按区间触发，支持跨越循环边界。

use crate::math::{Quat, Transform, Vec3, EPSILON};
use crate::pose::Pose;
use crate::skeleton::{JointIndex, Skeleton};

/// 通道驱动的属性类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelKind {
    /// 平移（Vec3）。
    Translation,
    /// 旋转（Quat）。
    Rotation,
    /// 缩放（Vec3）。
    Scale,
}

impl ChannelKind {
    /// 与 `core::ecs::AnimationKeyframe` 对齐的属性名。
    pub fn name(self) -> &'static str {
        match self {
            Self::Translation => "translation",
            Self::Rotation => "rotation",
            Self::Scale => "scale",
        }
    }
}

/// 关键帧插值方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
    /// 线性 / 球面插值（默认）。
    Linear,
    /// 阶梯：保持前一关键帧的值到下一帧。
    Step,
    /// 三次样条（glTF CUBICSPLINE）。0.2 未实现切线数据，采样回退为线性。
    CubicSpline,
}

/// 关键帧值（只允许与通道类型匹配）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyValue {
    Translation(Vec3),
    Rotation(Quat),
    Scale(Vec3),
}

impl KeyValue {
    pub fn kind(self) -> ChannelKind {
        match self {
            Self::Translation(_) => ChannelKind::Translation,
            Self::Rotation(_) => ChannelKind::Rotation,
            Self::Scale(_) => ChannelKind::Scale,
        }
    }

    pub fn as_translation(self) -> Option<Vec3> {
        match self {
            Self::Translation(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_rotation(self) -> Option<Quat> {
        match self {
            Self::Rotation(q) => Some(q),
            _ => None,
        }
    }

    pub fn as_scale(self) -> Option<Vec3> {
        match self {
            Self::Scale(v) => Some(v),
            _ => None,
        }
    }

    /// 按同类型插值；类型不匹配返回 `None`。
    pub fn lerp(a: Self, b: Self, t: f32) -> Option<Self> {
        match (a, b) {
            (Self::Translation(a), Self::Translation(b)) => Some(Self::Translation(a.lerp(b, t))),
            (Self::Rotation(a), Self::Rotation(b)) => Some(Self::Rotation(Quat::slerp(a, b, t))),
            (Self::Scale(a), Self::Scale(b)) => Some(Self::Scale(a.lerp(b, t))),
            _ => None,
        }
    }
}

/// 一个关键帧。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyframe {
    /// 时间（秒），必须有限且 >= 0。
    pub time: f32,
    pub value: KeyValue,
}

impl Keyframe {
    pub fn new(time: f32, value: KeyValue) -> Self {
        Self { time, value }
    }

    pub fn translation(time: f32, value: Vec3) -> Self {
        Self::new(time, KeyValue::Translation(value))
    }

    pub fn rotation(time: f32, value: Quat) -> Self {
        Self::new(time, KeyValue::Rotation(value))
    }

    pub fn scale(time: f32, value: Vec3) -> Self {
        Self::new(time, KeyValue::Scale(value))
    }
}

/// 驱动单个关节单个属性的关键帧序列。
#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    /// 目标关节索引（`Skeleton::joints` 下标）。
    pub target_joint: JointIndex,
    /// 该通道驱动的属性。
    pub kind: ChannelKind,
    /// 插值方式。
    pub interpolation: Interpolation,
    /// 关键帧（按时间升序，`add_key` 自动维护）。
    pub keys: Vec<Keyframe>,
}

impl Channel {
    /// 创建空通道（默认线性插值）。
    pub fn new(target_joint: JointIndex, kind: ChannelKind) -> Self {
        Self {
            target_joint,
            kind,
            interpolation: Interpolation::Linear,
            keys: Vec::new(),
        }
    }

    pub fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// 平移通道快捷构造。
    pub fn translation(target_joint: JointIndex) -> Self {
        Self::new(target_joint, ChannelKind::Translation)
    }

    /// 旋转通道快捷构造。
    pub fn rotation(target_joint: JointIndex) -> Self {
        Self::new(target_joint, ChannelKind::Rotation)
    }

    /// 缩放通道快捷构造。
    pub fn scale(target_joint: JointIndex) -> Self {
        Self::new(target_joint, ChannelKind::Scale)
    }

    /// 追加关键帧：校验类型匹配和时间有限，并保持升序。
    pub fn add_key(&mut self, key: Keyframe) -> Result<(), ClipError> {
        if !key.time.is_finite() || key.time < 0.0 {
            return Err(ClipError::InvalidTime { time: key.time });
        }
        let value_kind = key.value.kind();
        if value_kind != self.kind {
            return Err(ClipError::KindMismatch {
                expected: self.kind,
                actual: value_kind,
            });
        }
        let index = self
            .keys
            .partition_point(|existing| existing.time <= key.time);
        self.keys.insert(index, key);
        Ok(())
    }

    /// 通道覆盖的时间范围（最后一个关键帧时间；空通道为 0）。
    pub fn duration(&self) -> f32 {
        self.keys.last().map(|k| k.time).unwrap_or(0.0)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// 在时间 `time` 采样该通道（调用方负责循环/钳制时间）。
    pub fn sample(&self, time: f32) -> Option<KeyValue> {
        match self.keys.len() {
            0 => None,
            1 => Some(self.keys[0].value),
            _ => {
                let first = self.keys.first()?;
                let last = self.keys.last()?;
                if time <= first.time {
                    return Some(first.value);
                }
                if time >= last.time {
                    return Some(last.value);
                }
                let index = self
                    .keys
                    .partition_point(|key| key.time <= time)
                    .saturating_sub(1);
                let a = self.keys.get(index)?;
                let b = self.keys.get(index + 1)?;
                let span = b.time - a.time;
                if span <= EPSILON {
                    return Some(b.value);
                }
                let alpha = ((time - a.time) / span).clamp(0.0, 1.0);
                match self.interpolation {
                    Interpolation::Step => Some(a.value),
                    Interpolation::Linear => {
                        KeyValue::lerp(a.value, b.value, alpha).or(Some(a.value))
                    }
                    // TODO(0.3)：CubicSpline 需要 in/out tangent 三倍关键帧布局，
                    // 当前数据契约未携带切线，先按线性回退以保证行为可预期。
                    Interpolation::CubicSpline => {
                        KeyValue::lerp(a.value, b.value, alpha).or(Some(a.value))
                    }
                }
            }
        }
    }
}

/// 动画事件（脚步、攻击命中帧、受击帧等）。
///
/// 事件由状态机/玩家在时间推进时派发，动画模块不执行具体游戏逻辑。
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationEvent {
    /// 触发时间（秒）。
    pub time: f32,
    /// 事件名（例如 `"footstep"`、`"attack_hit"`）。
    pub name: String,
}

impl AnimationEvent {
    pub fn new(time: f32, name: impl Into<String>) -> Self {
        Self {
            time,
            name: name.into(),
        }
    }
}

/// 动画片段：多个通道 + 时长 + 事件。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationClip {
    /// 片段名（通常来自 glTF animation 名，例如 `Idle`/`Run`）。
    pub name: String,
    /// 片段时长（秒）；为 0 时由关键帧自动推算。
    pub duration: f32,
    /// 通道列表。
    pub channels: Vec<Channel>,
    /// 事件列表。
    pub events: Vec<AnimationEvent>,
}

impl AnimationClip {
    pub fn new(name: impl Into<String>, duration: f32) -> Self {
        Self {
            name: name.into(),
            duration,
            channels: Vec::new(),
            events: Vec::new(),
        }
    }

    /// 由通道推导时长构造。
    pub fn from_channels(name: impl Into<String>, channels: Vec<Channel>) -> Self {
        let mut clip = Self {
            name: name.into(),
            duration: 0.0,
            channels,
            events: Vec::new(),
        };
        clip.duration = clip.recompute_duration();
        clip
    }

    /// 重新计算时长（取所有通道/事件的最大时间）。
    pub fn recompute_duration(&mut self) -> f32 {
        let channel_max = self
            .channels
            .iter()
            .map(|c| c.duration())
            .fold(0.0f32, f32::max);
        let event_max = self.events.iter().map(|e| e.time).fold(0.0f32, f32::max);
        let duration = channel_max.max(event_max);
        if duration > 0.0 {
            self.duration = duration;
        }
        self.duration
    }

    pub fn add_channel(&mut self, channel: Channel) {
        self.channels.push(channel);
        self.recompute_duration();
    }

    pub fn add_event(&mut self, event: AnimationEvent) {
        self.events.push(event);
        self.events.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.recompute_duration();
    }

    #[inline]
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty() && self.events.is_empty()
    }

    /// 时间解析：循环时取模，非循环时钳制到 `[0, duration]`。
    pub fn resolve_time(&self, time: f32, looping: bool) -> f32 {
        let duration = self.duration;
        if duration <= 0.0 {
            return 0.0;
        }
        if looping {
            time.rem_euclid(duration)
        } else {
            time.clamp(0.0, duration)
        }
    }

    /// 以 `base` 为底（未驱动关节保持底姿态）采样整个 clip。
    ///
    /// `base` 通常是 `skeleton.default_pose()`（绑定姿态）；长度决定输出姿态长度。
    pub fn sample_into(&self, time: f32, looping: bool, base: &[Transform]) -> Pose {
        let mut locals = base.to_vec();
        let resolved = self.resolve_time(time, looping);
        self.apply_channels(resolved, &mut locals);
        Pose { locals }
    }

    /// 用骨架绑定姿态作为底采样整个 clip。
    pub fn sample(&self, time: f32, looping: bool, skeleton: &Skeleton) -> Pose {
        self.sample_into(time, looping, &skeleton.default_pose())
    }

    /// 把通道写到已有姿态上（不改变未驱动关节）。
    pub fn apply_channels(&self, resolved_time: f32, locals: &mut [Transform]) {
        for channel in &self.channels {
            let Some(value) = channel.sample(resolved_time) else {
                continue;
            };
            let Some(joint) = locals.get_mut(channel.target_joint) else {
                // 越界通道跳过（`validate` 可以提前发现；这里保证采样不 panic）。
                continue;
            };
            match value {
                KeyValue::Translation(v) => joint.translation = v,
                KeyValue::Rotation(q) => joint.rotation = q,
                KeyValue::Scale(v) => joint.scale = v,
            }
        }
    }

    /// 判断事件是否在 `[start, end)` 内触发；循环时考虑跨边界重复出现。
    ///
    /// 语义：
    /// - 半开区间 `[start, end)`，因此相邻两次推进不会重复派发同一事件；
    /// - 循环片段中事件时间按 `duration` 取模，跨越循环边界时仍会触发一次；
    /// - 一次推进跨越**多个完整循环**时每个事件只派发一次（简化，避免卡顿帧刷屏）。
    ///
    /// 该函数是纯函数，便于单测与状态机复用。
    pub fn event_fires(
        start: f32,
        end: f32,
        event_time: f32,
        looping: bool,
        duration: f32,
    ) -> bool {
        if end <= start + EPSILON {
            return false;
        }
        if duration <= 0.0 {
            return event_time >= start && event_time < end;
        }
        if !looping {
            if event_time < start {
                return false;
            }
            // 到达片段末尾时把该片段内的事件（含恰好在 duration 的事件）一次派发。
            if end >= duration - EPSILON {
                return event_time <= duration + EPSILON;
            }
            return event_time < end;
        }
        let t = event_time.rem_euclid(duration);
        if end - start >= duration {
            return true;
        }
        let start_floor = (start / duration).floor();
        let end_floor = (end / duration).floor();
        let local_start = start - start_floor * duration;
        let local_end = end - end_floor * duration;
        if end_floor > start_floor {
            // 跨过循环边界：命中 [local_start, duration) ∪ [0, local_end)。
            t >= local_start - EPSILON || t < local_end
        } else {
            t >= local_start - EPSILON && t < local_end
        }
    }

    /// 返回 `[start, end)` 区间内触发的事件（按时间升序）。
    pub fn events_in_range(&self, start: f32, end: f32, looping: bool) -> Vec<AnimationEvent> {
        if end <= start + EPSILON {
            return Vec::new();
        }
        self.events
            .iter()
            .filter(|event| Self::event_fires(start, end, event.time, looping, self.duration))
            .cloned()
            .collect()
    }

    /// 校验：通道目标关节在骨架范围内、关键帧升序、类型匹配。
    pub fn validate(&self, skeleton: &Skeleton) -> Result<(), ClipError> {
        for (channel_index, channel) in self.channels.iter().enumerate() {
            if channel.target_joint >= skeleton.joint_count() {
                return Err(ClipError::JointOutOfRange {
                    channel: channel_index,
                    joint: channel.target_joint,
                    skeleton_joint_count: skeleton.joint_count(),
                });
            }
            let mut previous_time = f32::NEG_INFINITY;
            for (key_index, key) in channel.keys.iter().enumerate() {
                if !key.time.is_finite() || key.time < 0.0 {
                    return Err(ClipError::InvalidTime { time: key.time });
                }
                if key.time < previous_time {
                    return Err(ClipError::KeysNotSorted {
                        channel: channel_index,
                        key: key_index,
                    });
                }
                if key.value.kind() != channel.kind {
                    return Err(ClipError::KindMismatch {
                        expected: channel.kind,
                        actual: key.value.kind(),
                    });
                }
                previous_time = key.time;
            }
        }
        Ok(())
    }
}

/// 动画片段/通道错误。
#[derive(Clone, Debug, PartialEq)]
pub enum ClipError {
    /// 关键帧时间非法（NaN/Inf/负数）。
    InvalidTime { time: f32 },
    /// 关键帧值与通道类型不匹配。
    KindMismatch {
        expected: ChannelKind,
        actual: ChannelKind,
    },
    /// 关键帧未按时间升序（手工构造 `Channel.keys` 时可能违反）。
    KeysNotSorted { channel: usize, key: usize },
    /// 通道目标关节越界。
    JointOutOfRange {
        channel: usize,
        joint: JointIndex,
        skeleton_joint_count: usize,
    },
}

impl std::fmt::Display for ClipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTime { time } => write!(f, "关键帧时间非法: {time}"),
            Self::KindMismatch { expected, actual } => {
                write!(f, "关键帧类型不匹配: 期望 {expected:?}，实际 {actual:?}")
            }
            Self::KeysNotSorted { channel, key } => {
                write!(f, "通道 {channel} 的关键帧 {key} 顺序错误")
            }
            Self::JointOutOfRange {
                channel,
                joint,
                skeleton_joint_count,
            } => write!(
                f,
                "通道 {channel} 的目标关节 {joint} 超出骨架关节数 {skeleton_joint_count}"
            ),
        }
    }
}

impl std::error::Error for ClipError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::Skeleton;

    fn two_key_translation_channel() -> Channel {
        let mut channel = Channel::translation(0);
        channel
            .add_key(Keyframe::translation(0.0, Vec3::ZERO))
            .unwrap();
        channel
            .add_key(Keyframe::translation(1.0, Vec3::new(10.0, 0.0, 0.0)))
            .unwrap();
        channel
    }

    #[test]
    fn channel_translation_linear_sample() {
        let channel = two_key_translation_channel();
        let mid = channel.sample(0.5).unwrap().as_translation().unwrap();
        assert!(mid.approx_eq(Vec3::new(5.0, 0.0, 0.0), 1e-5));
        let before = channel.sample(-1.0).unwrap().as_translation().unwrap();
        assert!(before.approx_eq(Vec3::ZERO, 1e-5));
    }

    #[test]
    fn channel_rotation_uses_slerp_and_step_holds_value() {
        let mut channel = Channel::rotation(0);
        channel
            .add_key(Keyframe::rotation(0.0, Quat::IDENTITY))
            .unwrap();
        channel
            .add_key(Keyframe::rotation(
                1.0,
                Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_2),
            ))
            .unwrap();
        let mid = channel.sample(0.5).unwrap().as_rotation().unwrap();
        let expected = Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_4);
        assert!(mid.approx_eq(expected, 1e-4));
        channel.interpolation = Interpolation::Step;
        let stepped = channel.sample(0.9).unwrap().as_rotation().unwrap();
        assert!(stepped.approx_eq(Quat::IDENTITY, 1e-6));
    }

    #[test]
    fn add_key_rejects_wrong_kind_and_keeps_sorted() {
        let mut channel = Channel::translation(0);
        assert!(matches!(
            channel.add_key(Keyframe::rotation(0.0, Quat::IDENTITY)),
            Err(ClipError::KindMismatch { .. })
        ));
        channel
            .add_key(Keyframe::translation(2.0, Vec3::ONE))
            .unwrap();
        channel
            .add_key(Keyframe::translation(1.0, Vec3::ZERO))
            .unwrap();
        assert_eq!(channel.keys[0].time, 1.0);
        assert_eq!(channel.keys[1].time, 2.0);
    }

    #[test]
    fn clip_sample_keeps_bind_pose_for_undriven_joints() {
        let mut skeleton = Skeleton::new("s");
        skeleton.add_root_joint(
            "root",
            Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
        );
        skeleton
            .add_joint(
                "child",
                0,
                Transform::from_trs(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY),
            )
            .unwrap();
        let mut clip = AnimationClip::new("move", 1.0);
        let mut channel = Channel::translation(1);
        channel
            .add_key(Keyframe::translation(0.0, Vec3::ZERO))
            .unwrap();
        channel
            .add_key(Keyframe::translation(1.0, Vec3::new(3.0, 0.0, 0.0)))
            .unwrap();
        clip.add_channel(channel);
        clip.validate(&skeleton).unwrap();
        let pose = clip.sample(0.5, false, &skeleton);
        assert!(pose.locals[0]
            .translation
            .approx_eq(Vec3::new(0.0, 1.0, 0.0), 1e-5));
        assert!(pose.locals[1]
            .translation
            .approx_eq(Vec3::new(1.5, 0.0, 0.0), 1e-5));
    }
}
