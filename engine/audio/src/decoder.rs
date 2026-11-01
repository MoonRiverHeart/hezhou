//! 音频解码器接口与注册表。
//!
//! 0.2 第一版内置纯 Rust WAV 解码器（[`WavDecoder`]）。
//! OGG/Vorbis、Opus 等需要第三方解码库（lewton / symphonia），因此设计为可注册的
//! [`AudioDecoder`]：
//!
//! - 接入方实现 [`AudioDecoder`]，在引擎初始化时调用
//!   [`crate::engine::AudioEngine::register_decoder`]；
//! - 未注册对应解码器时，[`DecoderRegistry::decode`] 返回带具体格式/编解码器名称的
//!   [`crate::error::AudioError::UnsupportedFormat`]，绝不 panic。
//!
//! README 提供了基于 lewton 的 OGG 解码器接入示例。

use crate::error::{AudioError, AudioResult};
use crate::sound::SoundData;
use crate::wav;

/// 音频容器格式（按魔数探测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// RIFF/WAVE。
    Wav,
    /// Ogg 容器（Vorbis/Opus/FLAC）。
    Ogg,
    /// 未知格式。
    Unknown,
}

/// 按文件头魔数探测容器格式。
pub fn detect_format(bytes: &[u8]) -> AudioFormat {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        AudioFormat::Wav
    } else if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
        AudioFormat::Ogg
    } else {
        AudioFormat::Unknown
    }
}

/// 音频解码器 trait：把容器字节流解码为 [`SoundData`]。
///
/// 实现方需要保证：
/// - `supports` 只做轻量魔数/格式探测，不做完整解码；
/// - `decode` 失败时返回描述清晰的 [`AudioError`]，不 panic；
/// - 线程安全（`Send + Sync`），因为可能被多个线程/编辑器预览调用。
pub trait AudioDecoder: Send + Sync {
    /// 解码器名称（日志/调试）。
    fn name(&self) -> &'static str;

    /// 该解码器是否能处理这段字节流。
    fn supports(&self, bytes: &[u8]) -> bool;

    /// 解码为内存音频数据。
    fn decode(&self, name: &str, bytes: &[u8]) -> AudioResult<SoundData>;
}

/// 内置 WAV 解码器（纯 Rust）。
#[derive(Debug, Default, Clone, Copy)]
pub struct WavDecoder;

impl AudioDecoder for WavDecoder {
    fn name(&self) -> &'static str {
        "wav"
    }

    fn supports(&self, bytes: &[u8]) -> bool {
        detect_format(bytes) == AudioFormat::Wav
    }

    fn decode(&self, name: &str, bytes: &[u8]) -> AudioResult<SoundData> {
        wav::decode_wav(name, bytes)
    }
}

/// 解码器注册表：按注册顺序探测并解码。
///
/// 默认通过 [`DecoderRegistry::with_builtin`] 创建，内置 WAV 解码器。
#[derive(Default)]
pub struct DecoderRegistry {
    decoders: Vec<Box<dyn AudioDecoder>>,
}

impl std::fmt::Debug for DecoderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecoderRegistry")
            .field("decoders", &self.decoder_names())
            .finish()
    }
}

impl DecoderRegistry {
    /// 创建空注册表（不含任何解码器，主要给高级用法）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建含内置解码器的注册表：WAV（始终）+ OGG/Vorbis（启用 `ogg` feature 时）。
    pub fn with_builtin() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(WavDecoder));
        #[cfg(feature = "ogg")]
        registry.register(Box::new(crate::ogg_vorbis::LewtonOggDecoder));
        registry
    }

    /// 注册自定义解码器（例如基于 lewton 的 OGG 解码器）。
    pub fn register(&mut self, decoder: Box<dyn AudioDecoder>) {
        self.decoders.push(decoder);
    }

    /// 已注册解码器名称列表。
    pub fn decoder_names(&self) -> Vec<&'static str> {
        self.decoders.iter().map(|decoder| decoder.name()).collect()
    }

    /// 解码一段字节流。
    ///
    /// 依次尝试注册的解码器；全部不支持时根据魔数给出可读错误：
    /// - OGG：报告容器内编解码器（Vorbis/Opus/FLAC）与接入建议；
    /// - 其他：报告未知格式。
    pub fn decode(&self, name: &str, bytes: &[u8]) -> AudioResult<SoundData> {
        for decoder in &self.decoders {
            if decoder.supports(bytes) {
                return decoder.decode(name, bytes);
            }
        }
        match detect_format(bytes) {
            AudioFormat::Wav => Err(AudioError::unsupported(
                "wav",
                "WAV 文件存在，但没有注册支持它的解码器（内置 WavDecoder 缺失？）",
            )),
            AudioFormat::Ogg => {
                let detail = match ogg::probe(bytes) {
                    Ok(info) => format!(
                        "检测到 OGG/{} 音频：当前版本未内置对应解码器，请通过 \
                         DecoderRegistry::register 注册基于 lewton/symphonia 的 AudioDecoder\
                         （见 crate README「OGG 接入点」）",
                        info.codec_name()
                    ),
                    Err(err) => format!(
                        "检测到 OGG 容器但无法解析首页：{err}；当前版本未内置 OGG 解码器，\
                         请注册基于 lewton/symphonia 的 AudioDecoder"
                    ),
                };
                Err(AudioError::unsupported("ogg", detail))
            }
            AudioFormat::Unknown => Err(AudioError::unsupported(
                "unknown",
                "无法识别的音频格式：内置仅支持 WAV；OGG 需要注册解码器",
            )),
        }
    }
}

/// Ogg 容器的最小探测（纯 Rust，不解码音频）。
///
/// 目的：在缺少 OGG 解码器时给出“到底是什么编码”的准确错误信息，
/// 也便于资源导入流程做格式校验。
pub mod ogg {
    use super::*;

    /// Ogg 内封装的编解码器。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OggCodec {
        /// Vorbis（OGG 中最常见）。
        Vorbis,
        /// Opus。
        Opus,
        /// FLAC（Ogg 映射）。
        Flac,
        /// 无法识别。
        Unknown,
    }

    impl OggCodec {
        /// 编解码器名称（错误信息/日志用）。
        pub fn name(self) -> &'static str {
            match self {
                Self::Vorbis => "Vorbis",
                Self::Opus => "Opus",
                Self::Flac => "FLAC",
                Self::Unknown => "Unknown",
            }
        }
    }

    /// Ogg 首页探测结果。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OggInfo {
        /// 容器内编解码器。
        pub codec: OggCodec,
        /// 流序号（serial number）。
        pub serial: u32,
        /// 声道数（若编码器信息可解析）。
        pub channels: Option<u8>,
        /// 采样率（若编码器信息可解析）。
        pub sample_rate: Option<u32>,
    }

    impl OggInfo {
        /// 编解码器名称。
        pub fn codec_name(&self) -> &'static str {
            self.codec.name()
        }
    }

    /// 解析 Ogg 第一页并识别编解码器。
    ///
    /// 只依赖 Ogg 页结构与 Vorbis/Opus/FLAC 的 identification header，
    /// 不做音频解码。
    pub fn probe(bytes: &[u8]) -> AudioResult<OggInfo> {
        if bytes.len() < 27 || &bytes[0..4] != b"OggS" {
            return Err(AudioError::invalid("不是 Ogg 容器（缺少 OggS 魔数）"));
        }
        if bytes[4] != 0 {
            return Err(AudioError::invalid(format!(
                "不支持的 Ogg 版本 {}",
                bytes[4]
            )));
        }
        let header_type = bytes[5];
        let serial = u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]);
        let page_segments = bytes[26] as usize;
        let segment_table_start = 27usize;
        let segment_table_end = segment_table_start + page_segments;
        if bytes.len() < segment_table_end {
            return Err(AudioError::invalid("Ogg 页段表被截断"));
        }
        let mut packet_len = 0usize;
        for index in 0..page_segments {
            packet_len += bytes[segment_table_start + index] as usize;
        }
        let packet_start = segment_table_end;
        let packet_end = packet_start + packet_len;
        if bytes.len() < packet_end {
            return Err(AudioError::invalid("Ogg 首页包数据被截断"));
        }
        let packet = &bytes[packet_start..packet_end];

        // Vorbis identification header: 0x01 "vorbis" version(4) channels(1) rate(4) ...
        if packet.first() == Some(&0x01) && packet.len() >= 16 && &packet[1..7] == b"vorbis" {
            let channels = packet[11];
            let sample_rate = u32::from_le_bytes([packet[12], packet[13], packet[14], packet[15]]);
            return Ok(OggInfo {
                codec: OggCodec::Vorbis,
                serial,
                channels: Some(channels),
                sample_rate: Some(sample_rate),
            });
        }
        // OpusHead: "OpusHead" version(1) channels(1) pre_skip(2) input_rate(4)
        if packet.len() >= 19 && &packet[0..8] == b"OpusHead" {
            let channels = packet[9];
            let sample_rate = u32::from_le_bytes([packet[12], packet[13], packet[14], packet[15]]);
            return Ok(OggInfo {
                codec: OggCodec::Opus,
                serial,
                channels: Some(channels),
                sample_rate: Some(sample_rate),
            });
        }
        // Ogg FLAC 映射：0x7F "FLAC"
        if packet.len() >= 5 && packet[0] == 0x7F && &packet[1..5] == b"FLAC" {
            return Ok(OggInfo {
                codec: OggCodec::Flac,
                serial,
                channels: None,
                sample_rate: None,
            });
        }
        let _ = header_type;
        Ok(OggInfo {
            codec: OggCodec::Unknown,
            serial,
            channels: None,
            sample_rate: None,
        })
    }

    /// 构造一个最小 Vorbis Ogg 首页（测试/文档用）。
    ///
    /// 仅包含页头与 identification header，不含真实音频数据。
    #[cfg(test)]
    pub(crate) fn synthetic_vorbis_page(channels: u8, sample_rate: u32) -> Vec<u8> {
        let mut identification = Vec::new();
        identification.push(0x01);
        identification.extend_from_slice(b"vorbis");
        identification.extend_from_slice(&0u32.to_le_bytes()); // version
        identification.push(channels);
        identification.extend_from_slice(&sample_rate.to_le_bytes());
        identification.extend_from_slice(&0u32.to_le_bytes()); // bitrate max
        identification.extend_from_slice(&0u32.to_le_bytes()); // bitrate nominal
        identification.extend_from_slice(&0u32.to_le_bytes()); // bitrate min
        identification.push(0xB8); // blocksize
        identification.push(0x01); // framing

        let mut page = Vec::new();
        page.extend_from_slice(b"OggS");
        page.push(0); // version
        page.push(0x02); // header_type: BOS
        page.extend_from_slice(&0u64.to_le_bytes()); // granule position
        page.extend_from_slice(&1u32.to_le_bytes()); // serial
        page.extend_from_slice(&0u32.to_le_bytes()); // sequence
        page.extend_from_slice(&0u32.to_le_bytes()); // crc（探测不校验）
        page.push(1); // page_segments
        page.push(identification.len() as u8);
        page.extend_from_slice(&identification);
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_by_magic() {
        assert_eq!(detect_format(b"RIFFxxxxWAVE"), AudioFormat::Wav);
        assert_eq!(detect_format(b"OggSxxxx"), AudioFormat::Ogg);
        assert_eq!(detect_format(b"ID3..."), AudioFormat::Unknown);
        assert_eq!(detect_format(b""), AudioFormat::Unknown);
    }

    #[test]
    fn builtin_registry_decodes_wav() {
        let registry = DecoderRegistry::with_builtin();
        // 默认（无 feature）只有 WAV；启用 `ogg` 后会追加 OGG/Vorbis 解码器。
        assert!(registry.decoder_names().contains(&"wav"));
        let wav_bytes =
            wav::encode_pcm16_wav(&SoundData::sine("tone", 440.0, 0.01, 48000).unwrap()).unwrap();
        let sound = registry.decode("tone", &wav_bytes).unwrap();
        assert_eq!(sound.frames(), 480);
    }

    #[test]
    #[cfg(not(feature = "ogg"))]
    fn ogg_without_decoder_reports_codec_and_suggestion() {
        let registry = DecoderRegistry::with_builtin();
        let page = ogg::synthetic_vorbis_page(2, 44100);
        let info = ogg::probe(&page).unwrap();
        assert_eq!(info.codec, ogg::OggCodec::Vorbis);
        assert_eq!(info.channels, Some(2));
        assert_eq!(info.sample_rate, Some(44100));

        let err = registry.decode("bgm", &page).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("Vorbis"),
            "错误信息应包含编解码器: {message}"
        );
        assert!(message.contains("OGG"), "错误信息应包含容器: {message}");
        assert!(matches!(err, AudioError::UnsupportedFormat { .. }));
    }

    #[test]
    fn ogg_probe_reports_opus_and_flac() {
        let mut packet = Vec::new();
        packet.extend_from_slice(b"OpusHead");
        packet.push(1); // version
        packet.push(2); // channels
        packet.extend_from_slice(&0u16.to_le_bytes()); // pre-skip
        packet.extend_from_slice(&48000u32.to_le_bytes()); // input rate
        packet.extend_from_slice(&0i16.to_le_bytes()); // output gain
        packet.push(0); // mapping family
        let mut page = Vec::new();
        page.extend_from_slice(b"OggS");
        page.push(0);
        page.push(0x02);
        page.extend_from_slice(&0u64.to_le_bytes());
        page.extend_from_slice(&7u32.to_le_bytes());
        page.extend_from_slice(&0u32.to_le_bytes());
        page.extend_from_slice(&0u32.to_le_bytes());
        page.push(1);
        page.push(packet.len() as u8);
        page.extend_from_slice(&packet);
        let info = ogg::probe(&page).unwrap();
        assert_eq!(info.codec, ogg::OggCodec::Opus);
        assert_eq!(info.channels, Some(2));
        assert_eq!(info.sample_rate, Some(48000));

        let mut flac_packet = vec![0x7F];
        flac_packet.extend_from_slice(b"FLAC");
        flac_packet.extend_from_slice(&[0u8; 4]);
        let mut page = Vec::new();
        page.extend_from_slice(b"OggS");
        page.push(0);
        page.push(0x02);
        page.extend_from_slice(&0u64.to_le_bytes());
        page.extend_from_slice(&9u32.to_le_bytes());
        page.extend_from_slice(&0u32.to_le_bytes());
        page.extend_from_slice(&0u32.to_le_bytes());
        page.push(1);
        page.push(flac_packet.len() as u8);
        page.extend_from_slice(&flac_packet);
        let info = ogg::probe(&page).unwrap();
        assert_eq!(info.codec, ogg::OggCodec::Flac);
    }

    #[test]
    fn probe_rejects_broken_pages() {
        assert!(ogg::probe(b"OggS").is_err());
        assert!(ogg::probe(b"not-an-ogg-page-at-all").is_err());
    }

    #[test]
    fn custom_decoder_can_be_registered() {
        struct ToyDecoder;
        impl AudioDecoder for ToyDecoder {
            fn name(&self) -> &'static str {
                "toy"
            }
            fn supports(&self, bytes: &[u8]) -> bool {
                bytes.starts_with(b"TOY1")
            }
            fn decode(&self, name: &str, bytes: &[u8]) -> AudioResult<SoundData> {
                if bytes.len() < 5 {
                    return Err(AudioError::invalid("TOY 数据过短"));
                }
                let frames = bytes[4] as usize;
                SoundData::silent(name, 8000, 1, frames)
            }
        }
        let mut registry = DecoderRegistry::with_builtin();
        registry.register(Box::new(ToyDecoder));
        assert!(registry.decoder_names().contains(&"wav"));
        assert!(registry.decoder_names().contains(&"toy"));
        let sound = registry.decode("toy", b"TOY1\x05").unwrap();
        assert_eq!(sound.frames(), 5);
    }

    #[test]
    fn unknown_format_message_is_readable() {
        let registry = DecoderRegistry::with_builtin();
        let err = registry.decode("x", b"RIFFnotwave!!").unwrap_err();
        assert!(matches!(err, AudioError::UnsupportedFormat { .. }));
        let err = registry.decode("x", b"random bytes here").unwrap_err();
        assert!(err.to_string().contains("无法识别"));
    }
}
