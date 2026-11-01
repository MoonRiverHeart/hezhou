//! 解码后的内存音频数据。
//!
//! [`SoundData`] 是引擎内部唯一的音频数据表示：f32 交错 PCM。
//! 解码器（WAV/未来 OGG）输出 [`SoundData`]，混音器按源采样率重采样后输出到设备。

use crate::error::{AudioError, AudioResult};

/// 输出声道数：引擎固定输出立体声（交错 L/R）。
pub const OUTPUT_CHANNELS: usize = 2;

/// 单个音效允许的最大声道数（构造/解码上限，超过报错）。
pub const MAX_CHANNELS: u16 = 8;

/// 已解码的内存音频数据（f32 交错 PCM）。
///
/// - 单声道在混音时复制到左右声道；
/// - 2 声道直接使用；
/// - 大于 2 声道时取前两路（0.2 只做基础能力，多声道下混后续按需扩展）。
#[derive(Debug, Clone)]
pub struct SoundData {
    name: String,
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

impl SoundData {
    /// 构造音频数据并做合法性校验。
    ///
    /// 校验项：采样率 > 0、声道数在 `1..=MAX_CHANNELS`、样本数能被声道数整除、
    /// 样本全部为有限值（不允许 NaN/Inf）。
    pub fn new(
        name: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
    ) -> AudioResult<Self> {
        if sample_rate == 0 {
            return Err(AudioError::invalid_argument("采样率不能为 0"));
        }
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(AudioError::invalid_argument(format!(
                "声道数 {channels} 非法（支持 1..={MAX_CHANNELS}）"
            )));
        }
        let channel_count = channels as usize;
        if !samples.len().is_multiple_of(channel_count) {
            return Err(AudioError::invalid(format!(
                "样本数 {} 不是声道数 {channel_count} 的整数倍",
                samples.len()
            )));
        }
        if let Some(pos) = samples.iter().position(|s| !s.is_finite()) {
            return Err(AudioError::invalid(format!(
                "样本 #{pos} 不是有限值（NaN/Inf 不允许）"
            )));
        }
        Ok(Self {
            name: name.into(),
            sample_rate,
            channels,
            samples,
        })
    }

    /// 从 i16 PCM 构造（16 位定点按 1/32768 归一到 [-1, 1)）。
    pub fn from_pcm_i16(
        name: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        data: &[i16],
    ) -> AudioResult<Self> {
        let samples = data.iter().map(|v| f32::from(*v) / 32768.0).collect();
        Self::new(name, sample_rate, channels, samples)
    }

    /// 构造一段静音（用于测试或占位资源）。
    pub fn silent(
        name: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        frames: usize,
    ) -> AudioResult<Self> {
        let total = frames
            .checked_mul(channels as usize)
            .ok_or_else(|| AudioError::invalid_argument("静音样本数溢出"))?;
        Self::new(name, sample_rate, channels, vec![0.0; total])
    }

    /// 生成单声道正弦波（纯 Rust 算法；用于离线测试、调试与编辑器预览占位）。
    ///
    /// - 幅度固定为 0.25，避免多个音效叠加时立刻削波；
    /// - `duration_secs` 必须 > 0，否则返回参数错误。
    pub fn sine(
        name: impl Into<String>,
        frequency_hz: f32,
        duration_secs: f32,
        sample_rate: u32,
    ) -> AudioResult<Self> {
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
            return Err(AudioError::invalid_argument(format!(
                "正弦频率 {frequency_hz} 非法"
            )));
        }
        if !duration_secs.is_finite() || duration_secs <= 0.0 {
            return Err(AudioError::invalid_argument(format!(
                "时长 {duration_secs} 非法"
            )));
        }
        if sample_rate == 0 {
            return Err(AudioError::invalid_argument("采样率不能为 0"));
        }
        let frames = (duration_secs * sample_rate as f32).round() as usize;
        if frames == 0 {
            return Err(AudioError::invalid_argument("时长为 0 帧"));
        }
        let phase_step = std::f32::consts::TAU * frequency_hz / sample_rate as f32;
        let samples = (0..frames)
            .map(|i| 0.25 * (phase_step * i as f32).sin())
            .collect();
        Self::new(name, sample_rate, 1, samples)
    }

    /// 资源名（调试/日志用）。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 源采样率（Hz）。
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 源声道数。
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// 是否没有音频帧。
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// 每声道帧数。
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    /// 时长（秒）。
    pub fn duration_secs(&self) -> f32 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.frames() as f32 / self.sample_rate as f32
        }
    }

    /// 原始交错样本（只读）。
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// 内存占用估算（字节），用于缓存统计。
    pub fn memory_bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f32>()
    }

    /// 取第 `frame` 帧映射到立体声后的 (left, right)。
    ///
    /// - 单声道：左右相同；
    /// - 多声道：取前两路；
    /// - 越界：返回静音。
    pub fn stereo_frame(&self, frame: usize) -> (f32, f32) {
        if frame >= self.frames() {
            return (0.0, 0.0);
        }
        let base = frame * self.channels as usize;
        match self.channels {
            0 => (0.0, 0.0),
            1 => (
                self.samples.get(base).copied().unwrap_or(0.0),
                self.samples.get(base).copied().unwrap_or(0.0),
            ),
            _ => (
                self.samples.get(base).copied().unwrap_or(0.0),
                self.samples.get(base + 1).copied().unwrap_or(0.0),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_invalid_arguments() {
        assert!(matches!(
            SoundData::new("bad", 0, 1, vec![0.0]),
            Err(AudioError::InvalidArgument(_))
        ));
        assert!(matches!(
            SoundData::new("bad", 48000, 0, vec![0.0]),
            Err(AudioError::InvalidArgument(_))
        ));
        assert!(matches!(
            SoundData::new("bad", 48000, 9, vec![0.0; 9]),
            Err(AudioError::InvalidArgument(_))
        ));
        assert!(matches!(
            SoundData::new("bad", 48000, 2, vec![0.0; 3]),
            Err(AudioError::InvalidData(_))
        ));
        assert!(matches!(
            SoundData::new("bad", 48000, 1, vec![f32::NAN]),
            Err(AudioError::InvalidData(_))
        ));
    }

    #[test]
    fn frames_duration_and_stereo_mapping() {
        let stereo = SoundData::new("st", 48000, 2, vec![0.1, -0.1, 0.2, -0.2]).unwrap();
        assert_eq!(stereo.frames(), 2);
        assert!((stereo.duration_secs() - 2.0 / 48000.0).abs() < 1e-9);
        assert_eq!(stereo.stereo_frame(0), (0.1, -0.1));
        assert_eq!(stereo.stereo_frame(1), (0.2, -0.2));
        assert_eq!(stereo.stereo_frame(2), (0.0, 0.0));

        let mono = SoundData::from_pcm_i16("mono", 48000, 1, &[16384, -16384]).unwrap();
        let (l, r) = mono.stereo_frame(0);
        assert!((l - 0.5).abs() < 1e-6);
        assert_eq!(l, r);

        let multi = SoundData::new("multi", 48000, 4, vec![0.1; 8]).unwrap();
        assert_eq!(multi.frames(), 2);
        assert_eq!(multi.stereo_frame(0), (0.1, 0.1));
    }

    #[test]
    fn silent_and_sine_generation() {
        let silence = SoundData::silent("s", 48000, 2, 100).unwrap();
        assert_eq!(silence.frames(), 100);
        assert!(silence.samples().iter().all(|s| *s == 0.0));

        let tone = SoundData::sine("tone", 440.0, 0.1, 48000).unwrap();
        assert_eq!(tone.channels(), 1);
        assert_eq!(tone.frames(), 4800);
        assert!(tone
            .samples()
            .iter()
            .all(|s| s.abs() <= 0.25 + f32::EPSILON));
        assert!(tone.samples().iter().any(|s| *s != 0.0));

        assert!(matches!(
            SoundData::sine("bad", 0.0, 0.1, 48000),
            Err(AudioError::InvalidArgument(_))
        ));
        assert!(matches!(
            SoundData::sine("bad", 440.0, 0.0, 48000),
            Err(AudioError::InvalidArgument(_))
        ));
    }

    #[test]
    fn memory_bytes_counts_samples() {
        let data = SoundData::silent("s", 48000, 2, 10).unwrap();
        assert_eq!(data.memory_bytes(), 20 * std::mem::size_of::<f32>());
    }
}
