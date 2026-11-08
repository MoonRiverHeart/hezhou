//! 可读的错误类型：传输 / framing / 消息 / 会话 / 房间。
//!
//! 所有错误都实现 `Display`/`std::error::Error`，便于上层日志/回显。
//! `std::io::Error` 不实现 `Clone`，所以网络层统一转为字符串保存（本 crate 只做原型级诊断）。

use std::fmt;
use std::io;

use crate::ids::RoomId;

/// 传输层错误（UDP/TCP socket 操作）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// 底层 IO 错误（保留可读文本，避免 `io::Error` 不能 Clone）。
    Io(String),
    /// 已关闭。
    Closed,
    /// 单次发送超过最大数据报/消息限制（分片保护）。
    MessageTooLarge {
        /// 实际大小（字节）。
        size: usize,
        /// 上限（字节）。
        max: usize,
    },
}

impl TransportError {
    /// 由 `io::Error` 构造。
    pub fn io(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }

    /// `WouldBlock`/`TimedOut` 是否属于"暂时没有数据"。
    pub fn is_would_block(&self) -> bool {
        matches!(self, Self::Io(message) if message.contains("would block") || message.contains("timed out") || message.contains("10035"))
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "网络 IO 错误: {message}"),
            Self::Closed => write!(f, "传输已关闭"),
            Self::MessageTooLarge { size, max } => {
                write!(f, "消息过大: {size} 字节 > 上限 {max} 字节")
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// 长度前缀 framing 错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// payload 超过最大帧长（发送或解码时）。
    TooLarge {
        /// 实际/声明的帧长（字节）。
        size: usize,
        /// 上限（字节）。
        max: usize,
    },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { size, max } => {
                write!(f, "帧过大: {size} 字节 > 上限 {max} 字节（分片保护）")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// 消息编解码错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageError {
    /// 编码/解码后的消息超过限制。
    TooLarge {
        /// 实际大小（字节）。
        size: usize,
        /// 上限（字节）。
        max: usize,
    },
    /// 输入字节不足（被截断）。
    Truncated,
    /// 未知消息类型 id。
    UnknownKind(u16),
    /// 不支持的消息格式版本。
    UnsupportedVersion(u8),
    /// 字符串不是合法 UTF-8。
    InvalidUtf8,
    /// 字符串/字节数组长度超过字段上限。
    FieldTooLong {
        /// 字段名。
        field: &'static str,
        /// 实际大小。
        size: usize,
        /// 上限。
        max: usize,
    },
    /// 解码后还有多余字节。
    TrailingBytes,
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { size, max } => write!(f, "消息过大: {size} > {max} 字节"),
            Self::Truncated => write!(f, "消息被截断"),
            Self::UnknownKind(kind) => write!(f, "未知消息类型: {kind}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "不支持的消息格式版本: {version}")
            }
            Self::InvalidUtf8 => write!(f, "消息中的字符串不是合法 UTF-8"),
            Self::FieldTooLong { field, size, max } => {
                write!(f, "字段 {field} 过长: {size} > {max} 字节")
            }
            Self::TrailingBytes => write!(f, "消息尾部存在多余字节"),
        }
    }
}

impl std::error::Error for MessageError {}

/// 会话层错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// 对端没有会话。
    UnknownPeer,
    /// 会话尚未完成握手。
    NotConnected,
    /// 消息编码失败。
    Message(MessageError),
    /// 角色不允许该操作（例如服务端不允许主动 connect）。
    WrongRole,
    /// 客户端不在房间。
    NoRoom,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPeer => write!(f, "未知网络对端"),
            Self::NotConnected => write!(f, "会话尚未连接"),
            Self::Message(error) => write!(f, "消息错误: {error}"),
            Self::WrongRole => write!(f, "会话角色不允许该操作"),
            Self::NoRoom => write!(f, "客户端不在任何房间"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<MessageError> for SessionError {
    fn from(error: MessageError) -> Self {
        Self::Message(error)
    }
}

/// 房间管理错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    /// 房间不存在。
    NotFound(RoomId),
    /// 房间已满。
    Full {
        /// 房间。
        room: RoomId,
        /// 该房间人数上限。
        max: usize,
    },
    /// 玩家已经在另一个房间。
    AlreadyInRoom {
        /// 玩家当前所在房间。
        room: RoomId,
    },
    /// 玩家不在任何房间。
    NotInRoom,
}

impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(room) => write!(f, "房间不存在: {room}"),
            Self::Full { room, max } => write!(f, "房间已满: {room}（上限 {max} 人）"),
            Self::AlreadyInRoom { room } => write!(f, "玩家已在房间 {room}"),
            Self::NotInRoom => write!(f, "玩家不在任何房间"),
        }
    }
}

impl std::error::Error for RoomError {}
