//! OGG/Vorbis 解码器（feature `ogg`，基于 lewton）。
//!
//! 启用 `ogg` feature 后：
//! - [`crate::decoder::DecoderRegistry::with_builtin`] 自动注册 [`LewtonOggDecoder`]，
//!   因此 [`crate::engine::AudioEngine`] 无需额外配置即可从文件/内存加载 OGG；
//! - 未启用时保留原有行为：OGG 数据会得到“未注册解码器”的可读错误。
//!
//! 与 WAV 解码器一样输出 f32 交错 [`SoundData`]；lewton 输出的 i16 采样按
//! `1/32768` 归一化。

use crate::decoder::{AudioDecoder, AudioFormat, DecoderRegistry};
use crate::error::{AudioError, AudioResult};
use crate::sound::SoundData;

/// 基于 lewton 的 OGG/Vorbis 解码器。
#[derive(Debug, Default, Clone, Copy)]
pub struct LewtonOggDecoder;

impl LewtonOggDecoder {
    /// 创建解码器。
    pub fn new() -> Self {
        Self
    }

    /// 以 `Box<dyn AudioDecoder>` 形式返回（便于传给引擎）。
    pub fn boxed() -> Box<dyn AudioDecoder> {
        Box::new(Self)
    }
}

impl AudioDecoder for LewtonOggDecoder {
    fn name(&self) -> &'static str {
        "ogg-vorbis"
    }

    fn supports(&self, bytes: &[u8]) -> bool {
        crate::decoder::detect_format(bytes) == AudioFormat::Ogg
    }

    fn decode(&self, name: &str, bytes: &[u8]) -> AudioResult<SoundData> {
        let cursor = std::io::Cursor::new(bytes);
        let mut reader = lewton::inside_ogg::OggStreamReader::new(cursor).map_err(|error| {
            AudioError::invalid(format!(
                "OGG/Vorbis 头解析失败（文件损坏或非 Vorbis）: {error}"
            ))
        })?;
        let sample_rate = reader.ident_hdr.audio_sample_rate;
        let channels = reader.ident_hdr.audio_channels;
        let mut samples: Vec<f32> = Vec::new();
        loop {
            match reader.read_dec_packet_itl() {
                Ok(Some(packet)) => {
                    samples.extend(packet.iter().map(|sample| f32::from(*sample) / 32768.0));
                }
                Ok(None) => break,
                Err(error) => {
                    return Err(AudioError::invalid(format!("OGG/Vorbis 解码失败: {error}")));
                }
            }
        }
        if samples.is_empty() {
            return Err(AudioError::invalid("OGG/Vorbis 不含音频采样"));
        }
        SoundData::new(name, sample_rate, u16::from(channels), samples)
    }
}

/// 把 lewton 解码器注册到注册表（同名解码器已存在时跳过，保证幂等）。
pub fn register_into(registry: &mut DecoderRegistry) {
    if !registry.decoder_names().contains(&"ogg-vorbis") {
        registry.register(LewtonOggDecoder::boxed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::AudioEngine;
    use std::path::{Path, PathBuf};

    /// 测试音频：0.1 秒 440Hz 正弦（48kHz 单声道 Vorbis）。
    ///
    /// 由 numpy 正弦波 + libsndfile（soundfile）离线生成，无第三方版权内容；
    /// 见 `tests/assets/README.md`。
    const OGG_TONE: &[u8] = include_bytes!("../tests/assets/tone_440hz_100ms.ogg");

    struct TempFile {
        path: PathBuf,
    }

    impl TempFile {
        fn new(bytes: &[u8]) -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let index = COUNTER.fetch_add(1, Ordering::Relaxed);
            let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/audio-test-tmp");
            std::fs::create_dir_all(&directory).expect("创建测试临时目录失败");
            let path = directory.join(format!("hezhou_audio_{}_{}.ogg", std::process::id(), index));
            std::fs::write(&path, bytes).expect("写入临时 OGG 失败");
            Self { path }
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn decoder_name_and_support_detection() {
        let decoder = LewtonOggDecoder::new();
        assert_eq!(decoder.name(), "ogg-vorbis");
        assert!(decoder.supports(OGG_TONE));
        assert!(!decoder.supports(b"RIFF\x00\x00\x00\x00WAVE"));
        assert!(!decoder.supports(b"not audio"));
    }

    #[test]
    fn decodes_real_ogg_vorbis_tone() {
        let sound = LewtonOggDecoder::new().decode("tone", OGG_TONE).unwrap();
        assert_eq!(sound.sample_rate(), 48_000);
        assert_eq!(sound.channels(), 1);
        // Vorbis 有编码延迟/尾部填充，帧数接近但不必严格等于 4800。
        assert!(
            (4_000..=9_600).contains(&sound.frames()),
            "OGG 解码帧数异常: {}",
            sound.frames()
        );
        let peak = sound
            .samples()
            .iter()
            .fold(0.0f32, |accumulator, sample| accumulator.max(sample.abs()));
        assert!(peak > 0.1, "解码后应有音频信号, peak={peak}");
    }

    #[test]
    fn registry_auto_registers_ogg_when_feature_enabled() {
        let registry = DecoderRegistry::with_builtin();
        assert!(
            registry.decoder_names().contains(&"ogg-vorbis"),
            "ogg feature 下应自动注册 lewton 解码器: {:?}",
            registry.decoder_names()
        );
        let sound = registry.decode("tone", OGG_TONE).unwrap();
        assert!(!sound.is_empty());
    }

    #[test]
    fn register_into_is_idempotent() {
        let mut registry = DecoderRegistry::with_builtin();
        register_into(&mut registry);
        register_into(&mut registry);
        let count = registry
            .decoder_names()
            .iter()
            .filter(|name| **name == "ogg-vorbis")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn engine_loads_ogg_from_memory_and_file() {
        let engine = AudioEngine::null();
        assert!(engine.decoder_names().contains(&"ogg-vorbis"));

        engine.load_from_memory("ogg-memory", OGG_TONE).unwrap();
        let sound = engine.sound("ogg-memory").expect("内存 OGG 应进入缓存");
        assert_eq!(sound.sample_rate(), 48_000);
        assert!(!sound.is_empty());

        let file = TempFile::new(OGG_TONE);
        engine.load_audio_file("ogg-file", &file.path).unwrap();
        assert!(engine.sound("ogg-file").is_some());

        // 播放与状态机在静音引擎下正常推进。
        let handle = engine.play("ogg-memory").unwrap();
        engine.advance_frames(480);
        assert!(!handle.is_valid() || handle.state() == crate::PlaybackState::Playing);
    }

    #[test]
    fn truncated_or_invalid_ogg_returns_error_without_panic() {
        // 截断在头部（文件头解析阶段失败）。
        let error = LewtonOggDecoder::new()
            .decode("bad", &OGG_TONE[..200])
            .unwrap_err();
        assert!(matches!(error, AudioError::InvalidData(_)));
        assert!(error.to_string().contains("OGG"), "错误信息应可读: {error}");

        // 只有标识页、缺少注释/建立头（用容器探测页面构造）。
        let page = crate::decoder::ogg::synthetic_vorbis_page(2, 44_100);
        let error = LewtonOggDecoder::new().decode("bad", &page).unwrap_err();
        assert!(matches!(error, AudioError::InvalidData(_)));
    }
}
