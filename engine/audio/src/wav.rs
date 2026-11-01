//! 纯 Rust WAV（RIFF/WAVE）解码与编码。
//!
//! 支持范围（0.2 第一版）：
//! - 容器：RIFF/WAVE，按 chunk 遍历，跳过 `LIST`/`fact` 等无关块；
//! - 编码：PCM 8/16/24/32 位整型、IEEE float 32/64 位；
//! - `WAVE_FORMAT_EXTENSIBLE`（0xFFFE）通过 sub-format 识别为 PCM 或 float；
//! - 多声道：保留原始声道数，混音时取前两路（见 [`crate::sound::SoundData`]）。
//!
//! 不依赖外部 crate，便于离线测试；OGG 见 [`crate::decoder`]。

use crate::error::{AudioError, AudioResult};
use crate::sound::SoundData;

/// 解析 WAV 字节流并解码为 [`SoundData`]。
///
/// `name` 是资源名，仅用于调试与日志。
pub fn decode_wav(name: impl Into<String>, bytes: &[u8]) -> AudioResult<SoundData> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::unsupported(
            "wav",
            "不是 RIFF/WAVE 文件（缺少 RIFF/WAVE 魔数）",
        ));
    }

    let mut format: Option<WavFormat> = None;
    let mut data: Option<&[u8]> = None;
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size =
            u32_at(bytes, pos + 4).ok_or_else(|| AudioError::invalid("chunk 头不完整"))? as usize;
        let start = pos + 8;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| {
                AudioError::invalid(format!(
                    "chunk 长度越界（size={size}, 剩余={})",
                    bytes.len() - start
                ))
            })?;
        match id {
            [b'f', b'm', b't', b' '] => format = Some(parse_fmt(&bytes[start..end])?),
            [b'd', b'a', b't', b'a'] => {
                // 规范只允许一个 data 块；若出现多个则使用第一个。
                data.get_or_insert(&bytes[start..end]);
            }
            _ => {}
        }
        // RIFF chunk 按偶数字节对齐。
        pos = end + (size & 1);
    }

    let format = format.ok_or_else(|| AudioError::invalid("WAV 缺少 fmt chunk"))?;
    let data = data.ok_or_else(|| AudioError::invalid("WAV 缺少 data chunk"))?;
    let channels = format.channels as usize;
    let bytes_per_sample = (format.bits / 8) as usize;
    if bytes_per_sample == 0 {
        return Err(AudioError::invalid("WAV 位深为 0"));
    }
    let frame_bytes = channels * bytes_per_sample;
    if data.len() < frame_bytes {
        return Err(AudioError::invalid(format!(
            "WAV data 长度 {} 不足一帧（{frame_bytes} 字节）",
            data.len()
        )));
    }
    let frames = data.len() / frame_bytes;
    let mut samples = Vec::with_capacity(frames * channels);
    for frame in 0..frames {
        let frame_start = frame * frame_bytes;
        for channel in 0..channels {
            let offset = frame_start + channel * bytes_per_sample;
            let sample = decode_sample(
                &data[offset..offset + bytes_per_sample],
                format.bits,
                format.encoding,
            )?;
            samples.push(sample);
        }
    }

    SoundData::new(name, format.sample_rate, format.channels, samples)
}

/// 把 [`SoundData`] 编码为 16 位 PCM WAV 字节流。
///
/// 用途：测试资源生成、编辑器内预览导出、C ABI 演示。
pub fn encode_pcm16_wav(sound: &SoundData) -> AudioResult<Vec<u8>> {
    let channels = sound.channels();
    if channels == 0 || channels > crate::sound::MAX_CHANNELS {
        return Err(AudioError::invalid("声道数非法，无法编码 WAV"));
    }
    let sample_rate = sound.sample_rate();
    if sample_rate == 0 {
        return Err(AudioError::invalid("采样率非法，无法编码 WAV"));
    }
    let bits = 16u16;
    let block_align = channels * (bits / 8);
    let byte_rate = sample_rate * block_align as u32;
    let data_len = (sound.samples().len() * 2) as u32;
    let total_len = 36u32
        .checked_add(data_len)
        .ok_or_else(|| AudioError::invalid("WAV 数据过大"))?;

    let mut out = Vec::with_capacity(total_len as usize + 8);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&total_len.to_le_bytes());
    out.extend_from_slice(b"WAVE");

    // fmt chunk（16 字节标准 PCM 头）
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());

    // data chunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in sound.samples() {
        let clamped = sample.clamp(-1.0, 1.0);
        let quantized = (clamped * 32767.0).round() as i16;
        out.extend_from_slice(&quantized.to_le_bytes());
    }
    Ok(out)
}

/// WAV 内部格式描述。
#[derive(Debug, Clone, Copy)]
struct WavFormat {
    encoding: WavEncoding,
    channels: u16,
    sample_rate: u32,
    bits: u16,
}

/// WAV 采样编码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WavEncoding {
    /// 整型 PCM（8 位无符号，16/24/32 位有符号）。
    Pcm,
    /// IEEE 浮点（32/64 位）。
    Float,
}

fn parse_fmt(bytes: &[u8]) -> AudioResult<WavFormat> {
    if bytes.len() < 16 {
        return Err(AudioError::invalid("WAV fmt chunk 长度不足 16 字节"));
    }
    let mut audio_format = u16_at(bytes, 0).unwrap();
    let channels = u16_at(bytes, 2).unwrap();
    let sample_rate = u32_at(bytes, 4).unwrap();
    let bits = u16_at(bytes, 14).unwrap();

    // WAVE_FORMAT_EXTENSIBLE：真实编码格式在 sub-format GUID 的前两个字节。
    if audio_format == 0xFFFE {
        if bytes.len() < 40 {
            return Err(AudioError::invalid(
                "WAV extensible fmt chunk 长度不足 40 字节",
            ));
        }
        audio_format = u16_at(bytes, 24).unwrap();
    }

    let encoding = match audio_format {
        1 => WavEncoding::Pcm,
        3 => WavEncoding::Float,
        other => {
            return Err(AudioError::unsupported(
                "wav",
                format!("WAV 编码格式 {other} 不受支持（支持 PCM 8/16/24/32 与 IEEE float 32/64）"),
            ));
        }
    };

    match (encoding, bits) {
        (WavEncoding::Pcm, 8 | 16 | 24 | 32) => {}
        (WavEncoding::Float, 32 | 64) => {}
        _ => {
            return Err(AudioError::unsupported(
                "wav",
                format!("WAV 位深 {bits} 不受支持（PCM 8/16/24/32，float 32/64）"),
            ));
        }
    }
    if channels == 0 || channels > crate::sound::MAX_CHANNELS {
        return Err(AudioError::unsupported(
            "wav",
            format!(
                "WAV 声道数 {channels} 不受支持（1..={}）",
                crate::sound::MAX_CHANNELS
            ),
        ));
    }
    if sample_rate == 0 {
        return Err(AudioError::invalid("WAV 采样率为 0"));
    }
    if bits == 8 && encoding == WavEncoding::Float {
        return Err(AudioError::invalid("8 位浮点 WAV 非法"));
    }
    Ok(WavFormat {
        encoding,
        channels,
        sample_rate,
        bits,
    })
}

fn decode_sample(bytes: &[u8], bits: u16, encoding: WavEncoding) -> AudioResult<f32> {
    let value = match (encoding, bits) {
        // 8 位 WAV 是无符号偏移编码。
        (WavEncoding::Pcm, 8) => (f32::from(bytes[0]) - 128.0) / 128.0,
        (WavEncoding::Pcm, 16) => f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) / 32768.0,
        (WavEncoding::Pcm, 24) => {
            let raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
            // 24 位有符号：符号扩展。
            let signed = (raw << 8) >> 8;
            signed as f32 / 8_388_608.0
        }
        (WavEncoding::Pcm, 32) => {
            let raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            raw as f32 / 2_147_483_648.0
        }
        (WavEncoding::Float, 32) => {
            let raw = u32_from_le(bytes)?;
            f32::from_bits(raw)
        }
        (WavEncoding::Float, 64) => {
            let raw = u64_from_le(bytes)?;
            f64::from_bits(raw) as f32
        }
        _ => {
            return Err(AudioError::unsupported(
                "wav",
                format!("不支持的采样编码（bits={bits}, encoding={encoding:?}）"),
            ));
        }
    };
    if !value.is_finite() {
        return Err(AudioError::invalid("WAV 包含 NaN/Inf 采样值"));
    }
    Ok(value)
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes
        .get(offset..offset + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u32_from_le(bytes: &[u8]) -> AudioResult<u32> {
    let array: [u8; 4] = bytes
        .get(..4)
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| AudioError::invalid("WAV 浮点样本长度不足 4 字节"))?;
    Ok(u32::from_le_bytes(array))
}

fn u64_from_le(bytes: &[u8]) -> AudioResult<u64> {
    let array: [u8; 8] = bytes
        .get(..8)
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| AudioError::invalid("WAV 浮点样本长度不足 8 字节"))?;
    Ok(u64::from_le_bytes(array))
}

/// 测试/工具用：构造最小 WAV 字节流的辅助函数。
#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个标准 PCM WAV（未压缩，无额外 chunk）。
    fn pcm_wav(sample_rate: u32, channels: u16, bits: u16, data: &[u8]) -> Vec<u8> {
        let block_align = channels * (bits / 8);
        let byte_rate = sample_rate * block_align as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    fn f32_wav(sample_rate: u32, channels: u16, bits: u16, data: &[u8]) -> Vec<u8> {
        let block_align = channels * (bits / 8);
        let byte_rate = sample_rate * block_align as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&3u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn decode_pcm16_mono_and_stereo() {
        let mono_data: Vec<u8> = [16384i16, -16384i16, 0i16]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let sound = decode_wav("mono", &pcm_wav(48000, 1, 16, &mono_data)).unwrap();
        assert_eq!(sound.channels(), 1);
        assert_eq!(sound.frames(), 3);
        assert!((sound.samples()[0] - 0.5).abs() < 1e-6);

        let stereo_data: Vec<u8> = [32767i16, -32768i16, 0i16, 0i16]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let sound = decode_wav("stereo", &pcm_wav(44100, 2, 16, &stereo_data)).unwrap();
        assert_eq!(sound.channels(), 2);
        assert_eq!(sound.frames(), 2);
        assert_eq!(sound.sample_rate(), 44100);
        let (l, r) = sound.stereo_frame(0);
        assert!((l - 0.999_969_7).abs() < 1e-6);
        assert!((r + 1.0).abs() < 1e-9);
    }

    #[test]
    fn decode_pcm8_unsigned() {
        // 8 位 WAV：0 表示负满幅，128 表示 0，255 表示接近正满幅。
        let sound = decode_wav("u8", &pcm_wav(8000, 1, 8, &[128, 255, 0])).unwrap();
        assert_eq!(sound.frames(), 3);
        assert!((sound.samples()[0]).abs() < 1e-6);
        assert!((sound.samples()[1] - 127.0 / 128.0).abs() < 1e-6);
        assert!((sound.samples()[2] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn decode_pcm24_and_pcm32() {
        // 24 位：最大的正数 0x7FFFFF ≈ 1.0
        let data24 = [0xFFu8, 0xFF, 0x7F];
        let sound = decode_wav("i24", &pcm_wav(48000, 1, 24, &data24)).unwrap();
        assert!((sound.samples()[0] - (8_388_607.0 / 8_388_608.0)).abs() < 1e-6);

        let data32 = i32::MAX.to_le_bytes();
        let sound = decode_wav("i32", &pcm_wav(48000, 1, 32, &data32)).unwrap();
        assert!((sound.samples()[0] - (2_147_483_647.0 / 2_147_483_648.0)).abs() < 1e-6);
    }

    #[test]
    fn decode_ieee_float32_and_64() {
        let data32 = 0.5f32.to_bits().to_le_bytes();
        let sound = decode_wav("f32", &f32_wav(48000, 1, 32, &data32)).unwrap();
        assert!((sound.samples()[0] - 0.5).abs() < 1e-6);

        let data64 = 0.25f64.to_bits().to_le_bytes();
        let sound = decode_wav("f64", &f32_wav(48000, 1, 64, &data64)).unwrap();
        assert!((sound.samples()[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn skips_unknown_chunks_and_handles_odd_padding() {
        // 在 fmt 与 data 之间插入一个奇数长度的 LIST chunk（需要 1 字节填充）。
        let pcm: Vec<u8> = [1000i16, -1000i16]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let mut wav = pcm_wav(48000, 1, 16, &pcm);
        // 在 data chunk 之前插入 LIST chunk。简化：重构整个文件。
        let mut fmt_part = Vec::new();
        fmt_part.extend_from_slice(&wav[0..36]); // RIFF 头 + fmt chunk（36 字节）
        let mut rest = Vec::new();
        rest.extend_from_slice(b"LIST");
        rest.extend_from_slice(&3u32.to_le_bytes());
        rest.extend_from_slice(b"abc"); // 3 字节 -> 需要 1 字节填充
        rest.push(0);
        rest.extend_from_slice(&wav[36..]); // data chunk
        let mut merged = fmt_part;
        merged.extend_from_slice(&rest);
        // 修正 RIFF size
        let total = merged.len() as u32 - 8;
        merged[4..8].copy_from_slice(&total.to_le_bytes());
        wav = merged;
        let sound = decode_wav("odd", &wav).unwrap();
        assert_eq!(sound.frames(), 2);
    }

    #[test]
    fn rejects_invalid_wav() {
        assert!(matches!(
            decode_wav("bad", b"not a wav file"),
            Err(AudioError::UnsupportedFormat { .. })
        ));

        // 缺少 fmt
        let mut only_data = Vec::new();
        only_data.extend_from_slice(b"RIFF");
        only_data.extend_from_slice(&100u32.to_le_bytes());
        only_data.extend_from_slice(b"WAVE");
        only_data.extend_from_slice(b"data");
        only_data.extend_from_slice(&4u32.to_le_bytes());
        only_data.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            decode_wav("bad", &only_data),
            Err(AudioError::InvalidData(_))
        ));

        // chunk 长度越界
        let mut truncated = pcm_wav(48000, 1, 16, &[0, 0]);
        let len = truncated.len();
        truncated[len - 1] = 0xFF;
        truncated.truncate(len - 1);
        assert!(decode_wav("bad", &truncated).is_err());
    }

    #[test]
    fn rejects_unsupported_encoding_and_bits() {
        // format = 2（ADPCM）
        let mut wav = pcm_wav(48000, 1, 16, &[0, 0]);
        wav[20] = 2;
        assert!(matches!(
            decode_wav("adpcm", &wav),
            Err(AudioError::UnsupportedFormat { .. })
        ));

        // 位深 12 不支持
        let mut wav = pcm_wav(48000, 1, 16, &[0, 0, 0]);
        wav[34] = 12;
        assert!(matches!(
            decode_wav("bits12", &wav),
            Err(AudioError::UnsupportedFormat { .. })
        ));
    }

    #[test]
    fn roundtrip_encode_decode_pcm16() {
        let tone = SoundData::sine("tone", 220.0, 0.05, 48000).unwrap();
        let bytes = encode_pcm16_wav(&tone).unwrap();
        let decoded = decode_wav("tone", &bytes).unwrap();
        assert_eq!(decoded.sample_rate(), tone.sample_rate());
        assert_eq!(decoded.channels(), tone.channels());
        assert_eq!(decoded.frames(), tone.frames());
        for (original, roundtrip) in tone.samples().iter().zip(decoded.samples()) {
            assert!((original - roundtrip).abs() < 1.0 / 32767.0 + 1e-6);
        }
    }
}
