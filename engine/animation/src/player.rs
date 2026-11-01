//! AnimationPlayer：驱动单个 `AnimationClip` 的时间推进、循环、速度与事件派发。
//!
//! 与 `core::ecs::components::AnimationPlayer`（Transform 级旧实现）区别：
//! - 这里输出的是整个骨架的 [`crate::pose::Pose`]，而不是单个 LocalTransform；
//! - 时间推进返回本步触发的事件（脚步/命中帧等），由玩法层消费；
//! - 循环用 `rem_euclid`，非循环到末尾后钳制并置 `finished`。

use crate::clip::{AnimationClip, AnimationEvent};
use crate::pose::Pose;
use crate::skeleton::Skeleton;

/// 单个动画片段的播放器（无状态机时的最简播放路径）。
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationPlayer {
    /// 播放的片段。
    pub clip: AnimationClip,
    /// 当前片段内时间（秒）。
    pub time: f32,
    /// 播放速度倍率（1.0 = 正常速度；<= 0 时暂停推进）。
    pub speed: f32,
    /// 是否循环。
    pub looping: bool,
    /// 是否处于播放状态（paused 时不推进时间）。
    pub playing: bool,
    /// 非循环片段是否已播放到最后。
    pub finished: bool,
    /// 上一次 `advance` 派发的事件（便于调试/上层查询）。
    pub last_events: Vec<AnimationEvent>,
}

impl AnimationPlayer {
    pub fn new(clip: AnimationClip) -> Self {
        Self {
            looping: true,
            speed: 1.0,
            playing: true,
            finished: false,
            time: 0.0,
            clip,
            last_events: Vec::new(),
        }
    }

    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// 当前片段时长（秒）。
    pub fn duration(&self) -> f32 {
        self.clip.duration
    }

    /// 归一化播放进度 0..1（时长为 0 时返回 0）。
    pub fn normalized_time(&self) -> f32 {
        if self.clip.duration > 0.0 {
            (self.time / self.clip.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn play(&mut self) {
        self.playing = true;
        self.finished = false;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn resume(&mut self) {
        self.playing = true;
    }

    /// 停止并回到起点。
    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
        self.finished = false;
    }

    /// 跳转到指定时间（按循环/非循环规则解析），清除上一帧事件。
    pub fn seek(&mut self, time: f32) {
        self.time = self.clip.resolve_time(time, self.looping);
        self.finished = false;
        self.last_events.clear();
    }

    /// 重置到起点并开始播放。
    pub fn reset(&mut self) {
        self.time = 0.0;
        self.finished = false;
        self.playing = true;
        self.last_events.clear();
    }

    /// 推进 `dt` 秒，返回本步 `[start, end)` 内触发的事件。
    ///
    /// - `playing == false`、`dt <= 0`、`speed <= 0` 时不推进并返回空；
    /// - 循环片段用 `rem_euclid` 回绕；非循环片段钳制到 `duration` 并置 `finished = true`；
    /// - 事件在时间推进前按原始 `[start, end)` 计算，因此跨循环边界也能正确派发。
    pub fn advance(&mut self, dt: f32) -> Vec<AnimationEvent> {
        self.last_events.clear();
        if !self.playing || dt <= 0.0 || self.speed <= 0.0 {
            return Vec::new();
        }
        let duration = self.clip.duration;
        if duration <= 0.0 {
            return Vec::new();
        }
        let start = self.time;
        let delta = dt * self.speed;
        let end = start + delta;
        let events = self.clip.events_in_range(start, end, self.looping);
        if self.looping {
            self.time = end.rem_euclid(duration);
            self.finished = false;
        } else if end >= duration {
            self.time = duration;
            self.finished = true;
        } else {
            self.time = end;
        }
        self.last_events = events.clone();
        events
    }

    /// 用骨架绑定姿态作为底采样当前时间。
    pub fn sample(&self, skeleton: &Skeleton) -> Pose {
        self.clip.sample(self.time, self.looping, skeleton)
    }

    /// 用调用方提供的底姿态采样当前时间。
    pub fn sample_into(&self, base: &[crate::math::Transform]) -> Pose {
        self.clip.sample_into(self.time, self.looping, base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{Channel, Keyframe};
    use crate::math::{Quat, Vec3};
    use crate::skeleton::Skeleton;

    fn simple_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("s");
        skeleton.add_root_joint("root", crate::math::Transform::IDENTITY);
        skeleton
    }

    fn one_second_clip() -> AnimationClip {
        let mut clip = AnimationClip::new("clip", 1.0);
        let mut channel = Channel::translation(0);
        channel
            .add_key(Keyframe::translation(0.0, Vec3::ZERO))
            .unwrap();
        channel
            .add_key(Keyframe::translation(1.0, Vec3::new(10.0, 0.0, 0.0)))
            .unwrap();
        clip.add_channel(channel);
        clip.add_event(AnimationEvent::new(0.25, "footstep"));
        clip.add_event(AnimationEvent::new(0.75, "attack_hit"));
        clip
    }

    #[test]
    fn advance_wraps_looping_time() {
        let mut player = AnimationPlayer::new(one_second_clip());
        player.advance(1.25);
        assert!((player.time - 0.25).abs() < 1e-5, "time={}", player.time);
        // 循环后应再次派发 0.25 处的事件。
        player.advance(0.001);
        assert!(player.time < 0.3);
    }

    #[test]
    fn non_looping_player_clamps_and_finishes() {
        let mut player = AnimationPlayer::new(one_second_clip()).with_looping(false);
        player.advance(2.0);
        assert!((player.time - 1.0).abs() < 1e-5);
        assert!(player.finished);
        // 到末尾后再次推进不再改变时间。
        player.advance(0.5);
        assert!((player.time - 1.0).abs() < 1e-5);
    }

    #[test]
    fn speed_scales_advanced_time() {
        let mut player = AnimationPlayer::new(one_second_clip())
            .with_looping(false)
            .with_speed(2.0);
        player.advance(0.25);
        assert!((player.time - 0.5).abs() < 1e-5, "time={}", player.time);
    }

    #[test]
    fn player_emits_events_across_loop_boundary() {
        let mut player = AnimationPlayer::new(one_second_clip());
        // 从 0.9 推进到 1.1（循环后 0.1）：0.25 处事件在下一圈 1.25 之前不触发；
        // 但 0.25 事件在 (0.9, 1.1) 内无等价时刻，0.75 也没有 ⇒ 空。
        player.seek(0.9);
        let events = player.advance(0.2);
        assert!(events.is_empty(), "events={events:?}");
        // 从 0.9 推进到 1.4：跨边界后 0.25 事件对应 1.25，应触发。
        player.seek(0.9);
        let events = player.advance(0.5);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "footstep");
    }

    #[test]
    fn sample_uses_bind_pose_for_undriven_and_curve_for_driven() {
        let skeleton = simple_skeleton();
        let mut player = AnimationPlayer::new(one_second_clip()).with_looping(false);
        player.seek(0.5);
        let pose = player.sample(&skeleton);
        assert!(pose.locals[0]
            .translation
            .approx_eq(Vec3::new(5.0, 0.0, 0.0), 1e-5));
        // 单位四元数未被通道修改。
        assert!(pose.locals[0].rotation.approx_eq(Quat::IDENTITY, 1e-6));
    }
}
