//! 音频模块统一错误类型。
//!
//! 设计原则：
//! - 所有对外 API 都返回 [`AudioResult`]，不 panic；
//! - 错误信息包含足够上下文（路径、格式、原因），便于编辑器/Console 展示；
//! - 不依赖外部错误库，保持 crate 可离线编译。

use std::fmt;

/// 音频模块统一返回类型。
pub type AudioResult<T> = Result<T, AudioError>;

/// 音频模块错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// 文件/流读写失败。
    Io {
        /// 出错路径（可能为空字符串，表示内存流）。
        path: String,
        /// 底层错误描述。
        message: String,
    },
    /// 格式不受支持（例如未注册 OGG 解码器、WAV 编码格式不认识）。
    UnsupportedFormat {
        /// 格式名，如 `"wav"`、`"ogg"`。
        format: String,
        /// 详细原因与修复建议。
        detail: String,
    },
    /// 数据内容非法（文件损坏、参数越界等）。
    InvalidData(String),
    /// 未注册对应解码器。
    DecoderNotFound(String),
    /// 音频后端（设备）错误。
    Backend(String),
    /// 调用参数不合法。
    InvalidArgument(String),
}

impl AudioError {
    /// 构造 IO 错误。
    pub fn io(path: impl Into<String>, err: &std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            message: err.to_string(),
        }
    }

    /// 构造数据非法错误。
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidData(message.into())
    }

    /// 构造格式不支持错误。
    pub fn unsupported(format: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
            detail: detail.into(),
        }
    }

    /// 构造参数非法错误。
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                if path.is_empty() {
                    write!(f, "音频 IO 错误: {message}")
                } else {
                    write!(f, "音频 IO 错误 [{path}]: {message}")
                }
            }
            Self::UnsupportedFormat { format, detail } => {
                write!(f, "不支持的音频格式 [{format}]: {detail}")
            }
            Self::InvalidData(message) => write!(f, "音频数据非法: {message}"),
            Self::DecoderNotFound(name) => write!(f, "未注册音频解码器: {name}"),
            Self::Backend(message) => write!(f, "音频后端错误: {message}"),
            Self::InvalidArgument(message) => write!(f, "音频参数非法: {message}"),
        }
    }
}

impl std::error::Error for AudioError {}
