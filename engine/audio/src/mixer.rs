//! 软件混音器：音效声部（voice）管理、增益合成、采样率转换与输出渲染。
//!
//! 纯 Rust 实现，不依赖音频设备，因此可完全离线单元测试。
//! 真实后端（cpal/kira）只需在设备回调里调用 [`Mixer::render_interleaved`]，
//! 或通过 [`crate::backend::MixerSink`] 拉取数据。
//!
//! 信号流：
//! ```text
//! SoundData(源采样率) --重采样--> voice 音量 --3D 衰减/声像--> 总线增益 --> 主音量 --> 限幅输出
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::bus::{clamp_volume, AudioBus, DEFAULT_BUS};
use crate::sound::{SoundData, OUTPUT_CHANNELS};
use crate::spatial::{evaluate, pan_to_gains, Listener, SpatialParams};

/// 声部（voice）唯一 id。
pub type VoiceId = u64;

/// 播放状态机。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    /// 已停止（初始状态；声部从混音器移除后查询也返回该状态）。
    #[default]
    Stopped,
    /// 播放中。
    Playing,
    /// 暂停（位置冻结）。
    Paused,
}

impl PlaybackState {
    /// 是否为活动状态（播放中或暂停）。
    pub fn is_active(self) -> bool {
        matches!(self, Self::Playing | Self::Paused)
    }

    /// 转成 C ABI / C# 使用的整数（0=Playing, 1=Paused, 2=Stopped）。
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Playing => 0,
            Self::Paused => 1,
            Self::Stopped => 2,
        }
    }
}

/// 一个正在播放（或暂停）的音频声部。
///
/// 由 [`Mixer`] 拥有；对外通过 [`crate::engine::SoundHandle`] 操作。
#[derive(Debug)]
pub struct Voice {
    pub(crate) id: VoiceId,
    pub(crate) sound: Arc<SoundData>,
    pub(crate) state: PlaybackState,
    /// 源帧位置（f64 用于线性重采样，避免逐帧取整误差）。
    pub(crate) position: f64,
    pub(crate) volume: f32,
    pub(crate) looping: bool,
    pub(crate) bus: String,
    pub(crate) spatial: Option<SpatialParams>,
}

impl Voice {
    /// 创建一个播放中的声部（引擎内部使用）。
    pub(crate) fn new(
        id: VoiceId,
        sound: Arc<SoundData>,
        volume: f32,
        looping: bool,
        bus: String,
        spatial: Option<SpatialParams>,
    ) -> Self {
        Self {
            id,
            sound,
            state: PlaybackState::Playing,
            position: 0.0,
            volume: clamp_volume(volume),
            looping,
            bus,
            spatial,
        }
    }

    /// 声部 id。
    pub fn id(&self) -> VoiceId {
        self.id
    }

    /// 当前状态。
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// 播放位置（秒，基于源采样率）。
    pub fn position_secs(&self) -> f32 {
        let rate = self.sound.sample_rate() as f64;
        if rate <= 0.0 {
            0.0
        } else {
            (self.position / rate) as f32
        }
    }

    /// 音效总时长（秒）。
    pub fn duration_secs(&self) -> f32 {
        self.sound.duration_secs()
    }

    /// 声部音量。
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// 是否循环。
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// 所属总线名。
    pub fn bus(&self) -> &str {
        &self.bus
    }

    /// 是否为 3D 声源。
    pub fn is_3d(&self) -> bool {
        self.spatial.is_some()
    }

    /// 3D 参数（2D 声部返回 `None`）。
    pub fn spatial(&self) -> Option<&SpatialParams> {
        self.spatial.as_ref()
    }

    /// 关联音频数据（只读）。
    pub fn sound(&self) -> &Arc<SoundData> {
        &self.sound
    }
}

/// 软件混音器。
///
/// 线程安全由调用方（[`crate::engine::AudioEngine`] 内的 `Mutex`）保证。
#[derive(Debug)]
pub struct Mixer {
    sample_rate: u32,
    voices: BTreeMap<VoiceId, Voice>,
    buses: BTreeMap<String, AudioBus>,
    master_volume: f32,
    muted: bool,
    listener: Listener,
    rendered_frames: u64,
}

impl Mixer {
    /// 创建混音器（默认 48 kHz 输出）。
    pub fn new(sample_rate: u32) -> Self {
        let mut buses = BTreeMap::new();
        buses.insert(DEFAULT_BUS.to_string(), AudioBus::new(DEFAULT_BUS));
        Self {
            sample_rate: sample_rate.max(1),
            voices: BTreeMap::new(),
            buses,
            master_volume: 1.0,
            muted: false,
            listener: Listener::default(),
            rendered_frames: 0,
        }
    }

    /// 输出采样率（Hz）。
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 设置所有声部的音量（0=静音，1=原始音量）。
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = clamp_volume(volume);
    }

    /// 当前主音量。
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// 主静音开关。
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// 是否主静音。
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// 主输出有效增益（静音时为 0）。
    pub fn master_gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.master_volume
        }
    }

    /// 更新监听者（相机）状态。
    pub fn set_listener(&mut self, listener: Listener) {
        self.listener = listener;
    }

    /// 当前监听者。
    pub fn listener(&self) -> Listener {
        self.listener
    }

    /// 确保指定总线存在（不存在则创建）。
    pub fn ensure_bus(&mut self, name: &str) {
        if !self.buses.contains_key(name) {
            self.buses.insert(name.to_string(), AudioBus::new(name));
        }
    }

    /// 设置总线音量（总线不存在时自动创建）。
    pub fn set_bus_volume(&mut self, name: &str, volume: f32) {
        self.ensure_bus(name);
        if let Some(bus) = self.buses.get_mut(name) {
            bus.set_volume(volume);
        }
    }

    /// 总线音量（不存在时返回 1.0）。
    pub fn bus_volume(&self, name: &str) -> f32 {
        self.buses.get(name).map(AudioBus::volume).unwrap_or(1.0)
    }

    /// 设置总线静音（总线不存在时自动创建）。
    pub fn set_bus_muted(&mut self, name: &str, muted: bool) {
        self.ensure_bus(name);
        if let Some(bus) = self.buses.get_mut(name) {
            bus.set_muted(muted);
        }
    }

    /// 总线是否静音（不存在时返回 false）。
    pub fn bus_muted(&self, name: &str) -> bool {
        self.buses.get(name).map(AudioBus::muted).unwrap_or(false)
    }

    /// 所有总线名称（排序）。
    pub fn bus_names(&self) -> Vec<String> {
        self.buses.keys().cloned().collect()
    }

    /// 加入一个声部。
    pub(crate) fn add_voice(&mut self, voice: Voice) {
        self.voices.insert(voice.id, voice);
    }

    /// 查询声部。
    pub(crate) fn voice(&self, id: VoiceId) -> Option<&Voice> {
        self.voices.get(&id)
    }

    /// 所有声部（只读迭代）。
    pub fn voices(&self) -> impl Iterator<Item = &Voice> {
        self.voices.values()
    }

    /// 活动声部数量（播放中 + 暂停）。
    pub fn active_voice_count(&self) -> usize {
        self.voices
            .values()
            .filter(|voice| voice.state.is_active())
            .count()
    }

    /// 声部总数（含刚结束但尚未清理的声部，通常是 0 个）。
    pub fn total_voice_count(&self) -> usize {
        self.voices.len()
    }

    /// 停止并移除全部声部，返回移除数量。
    pub fn stop_all(&mut self) -> usize {
        let count = self.voices.len();
        self.voices.clear();
        count
    }

    /// 暂停声部；仅当声部正在播放时返回 `true`。
    pub(crate) fn pause_voice(&mut self, id: VoiceId) -> bool {
        match self.voices.get_mut(&id) {
            Some(voice) if voice.state == PlaybackState::Playing => {
                voice.state = PlaybackState::Paused;
                true
            }
            _ => false,
        }
    }

    /// 继续播放；仅当声部暂停时返回 `true`。
    pub(crate) fn resume_voice(&mut self, id: VoiceId) -> bool {
        match self.voices.get_mut(&id) {
            Some(voice) if voice.state == PlaybackState::Paused => {
                voice.state = PlaybackState::Playing;
                true
            }
            _ => false,
        }
    }

    /// 停止并移除声部；声部不存在返回 `false`。
    pub(crate) fn stop_voice(&mut self, id: VoiceId) -> bool {
        self.voices.remove(&id).is_some()
    }

    /// 设置声部音量；声部不存在返回 `false`。
    pub(crate) fn set_voice_volume(&mut self, id: VoiceId, volume: f32) -> bool {
        match self.voices.get_mut(&id) {
            Some(voice) => {
                voice.volume = clamp_volume(volume);
                true
            }
            None => false,
        }
    }

    /// 设置声部循环标记；声部不存在返回 `false`。
    pub(crate) fn set_voice_looping(&mut self, id: VoiceId, looping: bool) -> bool {
        match self.voices.get_mut(&id) {
            Some(voice) => {
                voice.looping = looping;
                true
            }
            None => false,
        }
    }

    /// 更新 3D 声源位置；2D 声部返回 `false`。
    pub(crate) fn set_voice_position(
        &mut self,
        id: VoiceId,
        position: crate::spatial::Vec3,
    ) -> bool {
        match self.voices.get_mut(&id) {
            Some(voice) => match voice.spatial.as_mut() {
                Some(spatial) => {
                    spatial.position = position;
                    true
                }
                None => false,
            },
            None => false,
        }
    }

    /// 设置声部所属总线（不存在时自动创建）；声部不存在返回 `false`。
    pub(crate) fn set_voice_bus(&mut self, id: VoiceId, bus: &str) -> bool {
        if !self.buses.contains_key(bus) {
            self.ensure_bus(bus);
        }
        match self.voices.get_mut(&id) {
            Some(voice) => {
                voice.bus = bus.to_string();
                true
            }
            None => false,
        }
    }

    /// 已渲染总帧数（性能/调试统计）。
    pub fn rendered_frames(&self) -> u64 {
        self.rendered_frames
    }

    /// 渲染一块交错立体声样本（`out.len() / 2` 帧）。
    ///
    /// - 自动处理重采样、循环、播放结束（结束的声部会被移除，状态视为 `Stopped`）；
    /// - 输出会被限幅到 `[-1, 1]`，NaN/Inf 归零；
    /// - 返回实际渲染帧数（不足一帧的样本被忽略）。
    pub fn render_interleaved(&mut self, out: &mut [f32]) -> usize {
        let frames = out.len() / OUTPUT_CHANNELS;
        if frames == 0 {
            return 0;
        }
        out.fill(0.0);

        let master_gain = self.master_gain();
        if self.voices.is_empty() {
            self.rendered_frames = self.rendered_frames.saturating_add(frames as u64);
            return frames;
        }

        let listener = self.listener;
        // 预取总线增益，避免在遍历声部时借用冲突。
        let bus_gains: BTreeMap<String, f32> = self
            .buses
            .iter()
            .map(|(name, bus)| (name.clone(), bus.effective_gain()))
            .collect();

        let mut finished: Vec<VoiceId> = Vec::new();
        for (voice_id, voice) in self.voices.iter_mut() {
            if voice.state != PlaybackState::Playing {
                continue;
            }
            if voice.sound.is_empty() {
                voice.state = PlaybackState::Stopped;
                finished.push(*voice_id);
                continue;
            }

            let bus_gain = bus_gains.get(&voice.bus).copied().unwrap_or(1.0);
            let (spatial_gain, pan_gain) = match voice.spatial.as_ref() {
                Some(spatial) => {
                    let output = evaluate(&listener, spatial);
                    let (left, right) = pan_to_gains(output.pan);
                    (output.gain, (left, right))
                }
                None => (1.0, (1.0, 1.0)),
            };
            let gain = master_gain * bus_gain * voice.volume * spatial_gain;
            if gain <= 0.0 {
                // 静音/衰减为 0 时仍需推进位置，保证暂停/结束时机正确。
                let step = voice.sound.sample_rate() as f64 / self.sample_rate as f64;
                advance_silent(voice, step, frames, &mut finished);
                continue;
            }
            let (left_pan, right_pan) = pan_gain;
            let left_gain = gain * left_pan;
            let right_gain = gain * right_pan;
            let sound = Arc::clone(&voice.sound);
            let step = sound.sample_rate() as f64 / self.sample_rate as f64;
            let sound_frames = sound.frames() as f64;
            for frame in 0..frames {
                let (left_sample, right_sample) = sample_linear(&sound, voice.position);
                out[frame * OUTPUT_CHANNELS] += left_sample * left_gain;
                out[frame * OUTPUT_CHANNELS + 1] += right_sample * right_gain;
                voice.position += step;
                if voice.position >= sound_frames {
                    if voice.looping {
                        voice.position %= sound_frames;
                    } else {
                        voice.state = PlaybackState::Stopped;
                        finished.push(*voice_id);
                        break;
                    }
                }
            }
        }

        for id in finished {
            self.voices.remove(&id);
        }
        for sample in out.iter_mut() {
            *sample = if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }
        self.rendered_frames = self.rendered_frames.saturating_add(frames as u64);
        frames
    }

    /// 推进指定帧数并把输出丢弃（无声卡降级/离线测试用）。
    pub fn advance_frames(&mut self, mut frames: usize) {
        const CHUNK_FRAMES: usize = 1024;
        let mut buffer = [0.0f32; CHUNK_FRAMES * OUTPUT_CHANNELS];
        while frames > 0 {
            let chunk = frames.min(CHUNK_FRAMES);
            self.render_interleaved(&mut buffer[..chunk * OUTPUT_CHANNELS]);
            frames -= chunk;
        }
    }
}

/// 静音声部的“仅推进位置”路径（避免无意义采样计算，同时保持结束时机正确）。
fn advance_silent(voice: &mut Voice, step: f64, frames: usize, finished: &mut Vec<VoiceId>) {
    let sound_frames = voice.sound.frames() as f64;
    if sound_frames <= 0.0 {
        voice.state = PlaybackState::Stopped;
        finished.push(voice.id);
        return;
    }
    let end_position = voice.position + step * frames as f64;
    if end_position >= sound_frames {
        if voice.looping {
            voice.position = end_position % sound_frames;
        } else {
            voice.position = sound_frames;
            voice.state = PlaybackState::Stopped;
            finished.push(voice.id);
        }
    } else {
        voice.position = end_position;
    }
}

/// 以线性插值读取源数据在 `position`（源帧、可含小数）处的立体声样本。
pub(crate) fn sample_linear(sound: &SoundData, position: f64) -> (f32, f32) {
    let frames = sound.frames();
    if frames == 0 {
        return (0.0, 0.0);
    }
    let position = position.max(0.0);
    let base = position.floor() as usize;
    if base >= frames {
        return (0.0, 0.0);
    }
    let next = (base + 1).min(frames - 1);
    let frac = (position - base as f64) as f32;
    let (left0, right0) = sound.stereo_frame(base);
    let (left1, right1) = sound.stereo_frame(next);
    (
        left0 * (1.0 - frac) + left1 * frac,
        right0 * (1.0 - frac) + right1 * frac,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::{Attenuation, AttenuationModel, Vec3};

    fn sine_voice(id: VoiceId) -> Voice {
        let sound = Arc::new(SoundData::sine("tone", 440.0, 1.0, 48000).unwrap());
        Voice::new(id, sound, 1.0, false, DEFAULT_BUS.to_string(), None)
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn playback_state_helpers() {
        assert!(PlaybackState::Playing.is_active());
        assert!(PlaybackState::Paused.is_active());
        assert!(!PlaybackState::Stopped.is_active());
        assert_eq!(PlaybackState::Playing.as_i32(), 0);
        assert_eq!(PlaybackState::Paused.as_i32(), 1);
        assert_eq!(PlaybackState::Stopped.as_i32(), 2);
        assert_eq!(PlaybackState::default(), PlaybackState::Stopped);
    }

    #[test]
    fn render_produces_audio_and_advances() {
        let mut mixer = Mixer::new(48000);
        mixer.add_voice(sine_voice(1));
        let mut buffer = vec![0.0f32; 4800 * 2];
        let frames = mixer.render_interleaved(&mut buffer);
        assert_eq!(frames, 4800);
        assert!(rms(&buffer) > 0.01);
        let voice = mixer.voice(1).unwrap();
        assert!((voice.position_secs() - 0.1).abs() < 1e-6);
        assert_eq!(voice.state(), PlaybackState::Playing);
        assert_eq!(mixer.rendered_frames(), 4800);
    }

    #[test]
    fn pause_freezes_position_and_resume_continues() {
        let mut mixer = Mixer::new(48000);
        mixer.add_voice(sine_voice(1));
        let mut buffer = vec![0.0f32; 480 * 2];
        mixer.render_interleaved(&mut buffer);
        assert!(mixer.pause_voice(1));
        assert!(!mixer.pause_voice(1));
        let paused_position = mixer.voice(1).unwrap().position;
        mixer.render_interleaved(&mut buffer);
        assert_eq!(mixer.voice(1).unwrap().position, paused_position);
        assert_eq!(mixer.voice(1).unwrap().state(), PlaybackState::Paused);

        assert!(mixer.resume_voice(1));
        assert!(!mixer.resume_voice(1));
        mixer.render_interleaved(&mut buffer);
        assert!(mixer.voice(1).unwrap().position > paused_position);
    }

    #[test]
    fn non_looping_voice_ends_and_is_removed() {
        let mut mixer = Mixer::new(48000);
        let sound = Arc::new(SoundData::silent("short", 48000, 1, 480).unwrap());
        mixer.add_voice(Voice::new(
            7,
            sound,
            1.0,
            false,
            DEFAULT_BUS.to_string(),
            None,
        ));
        mixer.advance_frames(479);
        assert_eq!(mixer.voice(7).unwrap().state(), PlaybackState::Playing);
        mixer.advance_frames(1);
        assert!(mixer.voice(7).is_none());
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn looping_voice_wraps_and_keeps_playing() {
        let mut mixer = Mixer::new(48000);
        let sound = Arc::new(SoundData::silent("loop", 48000, 1, 480).unwrap());
        mixer.add_voice(Voice::new(
            3,
            sound,
            1.0,
            true,
            DEFAULT_BUS.to_string(),
            None,
        ));
        mixer.advance_frames(4800);
        let voice = mixer.voice(3).unwrap();
        assert_eq!(voice.state(), PlaybackState::Playing);
        assert!(voice.position < 480.0);
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn master_and_bus_volume_scale_output() {
        // 每次用独立 mixer 比较同一段渲染的 RMS。
        fn render_with(master: f32, bus: f32) -> f32 {
            let mut mixer = Mixer::new(48000);
            mixer.set_master_volume(master);
            mixer.set_bus_volume(DEFAULT_BUS, bus);
            mixer.add_voice(sine_voice(1));
            let mut buffer = vec![0.0f32; 4800 * 2];
            mixer.render_interleaved(&mut buffer);
            rms(&buffer)
        }
        let full = render_with(1.0, 1.0);
        let half_master = render_with(0.5, 1.0);
        let half_bus = render_with(1.0, 0.5);
        let quarter = render_with(0.5, 0.5);
        assert!((half_master / full - 0.5).abs() < 1e-3);
        assert!((half_bus / full - 0.5).abs() < 1e-3);
        assert!((quarter / full - 0.25).abs() < 1e-3);
    }

    #[test]
    fn mute_paths_produce_silence_but_still_advance() {
        let mut mixer = Mixer::new(48000);
        mixer.set_muted(true);
        mixer.add_voice(sine_voice(1));
        mixer.advance_frames(480);
        assert_eq!(mixer.voice(1).unwrap().position, 480.0);
        assert_eq!(mixer.voice(1).unwrap().state(), PlaybackState::Playing);

        mixer.set_muted(false);
        mixer.set_bus_muted(DEFAULT_BUS, true);
        let mut buffer = vec![0.0f32; 480 * 2];
        mixer.render_interleaved(&mut buffer);
        assert!(buffer.iter().all(|s| *s == 0.0));
        assert_eq!(mixer.voice(1).unwrap().position, 960.0);
    }

    #[test]
    fn spatial_source_pans_and_attenuates() {
        // 右方 40 单位（强衰减）
        let mut mixer = Mixer::new(48000);
        let sound = Arc::new(SoundData::sine("tone", 440.0, 1.0, 48000).unwrap());
        mixer.add_voice(Voice::new(
            1,
            sound,
            1.0,
            false,
            DEFAULT_BUS.to_string(),
            Some(SpatialParams::new(Vec3::new(40.0, 0.0, 0.0))),
        ));
        let mut buffer = vec![0.0f32; 4800 * 2];
        mixer.render_interleaved(&mut buffer);
        let left = rms(&buffer.iter().step_by(2).copied().collect::<Vec<_>>());
        let right = rms(&buffer
            .iter()
            .skip(1)
            .step_by(2)
            .copied()
            .collect::<Vec<_>>());
        assert!(right > left * 5.0, "右方声源应主要在右声道");
        assert!(rms(&buffer) < 0.05, "远距离声源应被明显衰减");

        // 监听者位置上的声源：满增益居中
        let mut mixer = Mixer::new(48000);
        let sound = Arc::new(SoundData::sine("tone", 440.0, 1.0, 48000).unwrap());
        mixer.add_voice(Voice::new(
            1,
            sound,
            1.0,
            false,
            DEFAULT_BUS.to_string(),
            Some(SpatialParams::new(Vec3::ZERO)),
        ));
        mixer.render_interleaved(&mut buffer);
        let near = rms(&buffer);
        assert!(near > 0.1);
    }

    #[test]
    fn distance_cutoff_makes_far_source_silent() {
        let mut mixer = Mixer::new(48000);
        let sound = Arc::new(SoundData::silent("s", 48000, 1, 48000).unwrap());
        let attenuation = Attenuation::new(AttenuationModel::Linear, 1.0, 10.0, 1.0);
        mixer.add_voice(Voice::new(
            1,
            sound,
            1.0,
            false,
            DEFAULT_BUS.to_string(),
            Some(SpatialParams::new(Vec3::new(0.0, 0.0, -50.0)).with_attenuation(attenuation)),
        ));
        let mut buffer = vec![0.0f32; 480 * 2];
        mixer.render_interleaved(&mut buffer);
        assert!(buffer.iter().all(|s| *s == 0.0));
        assert_eq!(mixer.voice(1).unwrap().position, 480.0);
    }

    #[test]
    fn sample_rate_conversion_ends_at_correct_time() {
        // 源 24k、1 秒（24000 帧），输出 48k：应在输出 1 秒（48000 帧）后结束。
        let mut mixer = Mixer::new(48000);
        let sound = Arc::new(SoundData::silent("24k", 24000, 1, 24000).unwrap());
        mixer.add_voice(Voice::new(
            1,
            sound,
            1.0,
            false,
            DEFAULT_BUS.to_string(),
            None,
        ));
        mixer.advance_frames(47999);
        assert_eq!(mixer.voice(1).unwrap().state(), PlaybackState::Playing);
        mixer.advance_frames(1);
        assert!(mixer.voice(1).is_none());
    }

    #[test]
    fn voices_are_mixed_and_limited() {
        let mut mixer = Mixer::new(48000);
        // 两个满幅 0.9 的音效叠加 -> 限幅到 1.0，不出现 NaN/Inf。
        for id in 1..=2 {
            let data = vec![0.9f32; 100 * 2];
            let sound = Arc::new(SoundData::new("loud", 48000, 2, data).unwrap());
            mixer.add_voice(Voice::new(
                id,
                sound,
                1.0,
                false,
                DEFAULT_BUS.to_string(),
                None,
            ));
        }
        let mut buffer = vec![0.0f32; 100 * 2];
        mixer.render_interleaved(&mut buffer);
        assert!(buffer
            .iter()
            .all(|s| s.is_finite() && *s <= 1.0 && *s >= -1.0));
        assert!(buffer.iter().any(|s| *s >= 0.99));
    }

    #[test]
    fn voice_controls_and_bus_reassignment() {
        let mut mixer = Mixer::new(48000);
        mixer.add_voice(sine_voice(9));
        assert!(mixer.set_voice_volume(9, 0.5));
        assert_eq!(mixer.voice(9).unwrap().volume(), 0.5);
        assert!(!mixer.set_voice_volume(404, 0.5));

        assert!(mixer.set_voice_looping(9, true));
        assert!(mixer.voice(9).unwrap().is_looping());

        // 2D 声部不能 set_position
        assert!(!mixer.set_voice_position(9, Vec3::new(1.0, 2.0, 3.0)));

        assert!(mixer.set_voice_bus(9, "sfx"));
        assert_eq!(mixer.voice(9).unwrap().bus(), "sfx");
        assert!(mixer.bus_names().contains(&"sfx".to_string()));

        assert_eq!(mixer.stop_all(), 1);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn render_with_no_voices_is_silence() {
        let mut mixer = Mixer::new(48000);
        let mut buffer = vec![0.5f32; 100 * 2];
        let frames = mixer.render_interleaved(&mut buffer);
        assert_eq!(frames, 100);
        assert!(buffer.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn sample_linear_interpolates_and_clamps() {
        let sound = SoundData::new("s", 1000, 1, vec![0.0, 1.0, 0.0]).unwrap();
        let (left, right) = sample_linear(&sound, 0.0);
        assert_eq!((left, right), (0.0, 0.0));
        let (left, _) = sample_linear(&sound, 1.0);
        assert!((left - 1.0).abs() < 1e-6);
        let (left, _) = sample_linear(&sound, 0.5);
        assert!((left - 0.5).abs() < 1e-6);
        let (left, _) = sample_linear(&sound, 10.0);
        assert_eq!(left, 0.0);
    }
}
