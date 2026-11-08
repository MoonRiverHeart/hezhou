//! 长度前缀 framing（u32 LE + payload）与分片保护。
//!
//! 用于 TCP 流：TCP 是字节流，需要长度前缀切分应用消息；
//! UDP 数据报天然有边界，但仍用 [`FrameCodec`] 做消息大小的统一校验/编码便于协议一致。
//!
//! 规则：
//! - 帧格式：`[u32 LE payload_len][payload bytes]`；
//! - `payload_len` 超过 [`FrameCodec::max_frame_bytes`] → [`FrameError::TooLarge`]；
//! - [`FrameDecoder`] 支持任意分片/粘包：把收到的字节 push 进去，返回当前可完成的所有帧；
//! - 空 payload 合法（长度为 0 的帧）。

use crate::error::FrameError;

/// 帧头长度（u32 LE）。
pub const FRAME_HEADER_SIZE: usize = 4;

/// 默认最大帧长（64 KiB，适合 TCP；UDP 单包由消息层 1200 字节限制）。
pub const DEFAULT_MAX_FRAME_BYTES: usize = 64 * 1024;

/// 长度前缀帧编解码器。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCodec {
    max_frame_bytes: usize,
}

impl FrameCodec {
    /// 创建 codec（`max_frame_bytes` 至少为 1）。
    pub fn new(max_frame_bytes: usize) -> Self {
        Self {
            max_frame_bytes: max_frame_bytes.max(1),
        }
    }

    /// 最大 payload 长度。
    pub fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    /// 编码一帧（含 4 字节长度前缀）。
    pub fn encode(&self, payload: &[u8]) -> Result<Vec<u8>, FrameError> {
        if payload.len() > self.max_frame_bytes {
            return Err(FrameError::TooLarge {
                size: payload.len(),
                max: self.max_frame_bytes,
            });
        }
        let mut frame = Vec::with_capacity(FRAME_HEADER_SIZE + payload.len());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);
        Ok(frame)
    }

    /// 创建配套的增量解码器。
    pub fn decoder(&self) -> FrameDecoder {
        FrameDecoder::new(self.max_frame_bytes)
    }
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME_BYTES)
    }
}

/// 增量帧解码器：处理 TCP 分片/粘包。
#[derive(Clone, Debug)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    max_frame_bytes: usize,
}

impl FrameDecoder {
    /// 创建解码器。
    pub fn new(max_frame_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_bytes: max_frame_bytes.max(1),
        }
    }

    /// 最大 payload 长度。
    pub fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    /// 当前缓存但尚未组成完整帧的字节数。
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }

    /// 是否有半包（用于调试/统计）。
    pub fn has_partial_frame(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// 清空缓存（连接关闭/协议错误时调用）。
    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    /// 追加收到的字节，返回本次可完成的完整帧（按到达顺序）。
    ///
    /// 返回值中的 `Vec<u8>` 只包含 payload，不含长度前缀。
    /// 错误时 decoder 会清空缓存（避免后续字节继续解析错位）。
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            if self.buffer.len() < FRAME_HEADER_SIZE {
                break;
            }
            let length = u32::from_le_bytes([
                self.buffer[0],
                self.buffer[1],
                self.buffer[2],
                self.buffer[3],
            ]) as usize;
            if length > self.max_frame_bytes {
                self.buffer.clear();
                return Err(FrameError::TooLarge {
                    size: length,
                    max: self.max_frame_bytes,
                });
            }
            let total = FRAME_HEADER_SIZE + length;
            if self.buffer.len() < total {
                break;
            }
            let payload = self.buffer[FRAME_HEADER_SIZE..total].to_vec();
            self.buffer.drain(..total);
            frames.push(payload);
        }
        Ok(frames)
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let codec = FrameCodec::new(128);
        let mut decoder = codec.decoder();
        let frame = codec.encode(b"hello").unwrap();
        assert_eq!(frame.len(), FRAME_HEADER_SIZE + 5);
        let frames = decoder.push(&frame).unwrap();
        assert_eq!(frames, vec![b"hello".to_vec()]);
        assert!(!decoder.has_partial_frame());
    }

    #[test]
    fn partial_reads_are_buffered() {
        let codec = FrameCodec::new(128);
        let mut decoder = codec.decoder();
        let frame = codec.encode(b"fragmented").unwrap();
        let mut frames = Vec::new();
        for byte in &frame[..frame.len() - 1] {
            frames.extend(decoder.push(&[*byte]).unwrap());
        }
        assert!(frames.is_empty());
        assert!(decoder.has_partial_frame());
        frames.extend(decoder.push(&frame[frame.len() - 1..]).unwrap());
        assert_eq!(frames, vec![b"fragmented".to_vec()]);
    }

    #[test]
    fn sticky_packets_return_multiple_frames() {
        let codec = FrameCodec::new(64);
        let mut decoder = codec.decoder();
        let mut bytes = Vec::new();
        bytes.extend(codec.encode(b"a").unwrap());
        bytes.extend(codec.encode(b"bc").unwrap());
        bytes.extend(codec.encode(b"").unwrap());
        let frames = decoder.push(&bytes).unwrap();
        assert_eq!(frames, vec![b"a".to_vec(), b"bc".to_vec(), Vec::new()]);
    }

    #[test]
    fn oversized_frame_is_rejected_on_encode_and_decode() {
        let codec = FrameCodec::new(4);
        assert!(matches!(
            codec.encode(b"12345"),
            Err(FrameError::TooLarge { size: 5, max: 4 })
        ));

        let mut decoder = FrameDecoder::new(4);
        let mut forged = 5u32.to_le_bytes().to_vec();
        forged.extend_from_slice(b"12345");
        assert!(matches!(
            decoder.push(&forged),
            Err(FrameError::TooLarge { size: 5, max: 4 })
        ));
        assert_eq!(decoder.buffered_len(), 0);
    }
}
