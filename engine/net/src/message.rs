//! 网络消息与自定义二进制序列化（无第三方依赖）。
//!
//! 消息集合面向"会话/房间"层，后续同步层可在此基础上增加 Snapshot 内容：
//!
//! | 消息 | 方向 | 说明 |
//! |---|---|---|
//! | [`Message::ClientHello`] | client → server | 握手：协议版本 + 客户端名 |
//! | [`Message::ServerWelcome`] | server → client | 握手：分配 session_id / tick rate / 服务器 tick |
//! | [`Message::Snapshot`] | server → client | 状态快照占位（tick/seq/payload），本 crate 不解释内容 |
//! | [`Message::Rpc`] | any | RPC：target（`PlayerId(0)`=服务器）/ method / payload |
//! | [`Message::Heartbeat`] | any | 心跳/心跳响应（nonce + pong 标记） |
//! | [`Message::Disconnect`] | any | 主动断开 + 原因 |
//! | [`Message::JoinRoom`]/[`Message::LeaveRoom`] | client → server | 房间操作 |
//! | [`Message::RoomJoined`]/[`Message::PlayerJoined`]/[`Message::PlayerLeft`]/[`Message::JoinRejected`] | server → client | 房间事件 |
//! | [`Message::Chat`] | client ↔ server | 房间文本消息 |
//!
//! 线上格式（little-endian）：
//!
//! ```text
//! [u8 format_version][u16 message_kind][variant payload...]
//! ```
//!
//! 限制：
//! - 单条消息默认 ≤ [`MAX_MESSAGE_BYTES`]（1200 字节，UDP 单包不分片）；
//! - 字符串/字节数组长度用 u16 前缀，并有字段级上限（名字 64、RPC 方法 64、聊天 256）;
//! - 解码严格：未知类型/版本/截断/尾部多余字节都返回 [`MessageError`]。

use crate::error::MessageError;
use crate::ids::{PlayerId, RoomId, SessionId};

/// 消息格式版本。
pub const MESSAGE_VERSION: u8 = 1;
/// 协议版本（握手协商用）。
pub const PROTOCOL_VERSION: u16 = 1;
/// 单条消息最大字节数（UDP 单包保护；不分片）。
pub const MAX_MESSAGE_BYTES: usize = 1200;
/// 客户端名最大字节数。
pub const MAX_NAME_BYTES: usize = 64;
/// RPC 方法名最大字节数。
pub const MAX_RPC_METHOD_BYTES: usize = 64;
/// 聊天文本最大字节数。
pub const MAX_CHAT_BYTES: usize = 256;
/// 房间成员列表最大数量。
pub const MAX_ROOM_MEMBERS: usize = 64;

/// 消息类型 id（线上 u16）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    /// 握手请求。
    ClientHello = 1,
    /// 握手响应。
    ServerWelcome = 2,
    /// 状态快照（payload 由上层解释）。
    Snapshot = 3,
    /// RPC 调用。
    Rpc = 4,
    /// 心跳/心跳响应。
    Heartbeat = 5,
    /// 主动断开。
    Disconnect = 6,
    /// 加入房间。
    JoinRoom = 7,
    /// 离开房间。
    LeaveRoom = 8,
    /// 加入成功。
    RoomJoined = 9,
    /// 其他玩家加入。
    PlayerJoined = 10,
    /// 其他玩家离开。
    PlayerLeft = 11,
    /// 房间聊天。
    Chat = 12,
    /// 加入被拒绝。
    JoinRejected = 13,
    /// 离开房间成功。
    RoomLeft = 14,
}

impl MessageKind {
    /// 线上 id。
    pub fn id(self) -> u16 {
        self as u16
    }

    /// 由线上 id 解析。
    pub fn from_id(id: u16) -> Option<Self> {
        Some(match id {
            1 => Self::ClientHello,
            2 => Self::ServerWelcome,
            3 => Self::Snapshot,
            4 => Self::Rpc,
            5 => Self::Heartbeat,
            6 => Self::Disconnect,
            7 => Self::JoinRoom,
            8 => Self::LeaveRoom,
            9 => Self::RoomJoined,
            10 => Self::PlayerJoined,
            11 => Self::PlayerLeft,
            12 => Self::Chat,
            13 => Self::JoinRejected,
            14 => Self::RoomLeft,
            _ => return None,
        })
    }
}

/// 断开原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// 正常退出。
    Normal = 0,
    /// 心跳超时。
    Timeout = 1,
    /// 协议错误。
    ProtocolError = 2,
    /// 服务器关闭。
    ServerShutdown = 3,
    /// 被踢出。
    Kicked = 4,
}

impl DisconnectReason {
    /// 线上 id。
    pub fn id(self) -> u8 {
        self as u8
    }

    /// 由线上 id 解析。
    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => Self::Normal,
            1 => Self::Timeout,
            2 => Self::ProtocolError,
            3 => Self::ServerShutdown,
            4 => Self::Kicked,
            _ => return None,
        })
    }
}

/// 加入房间被拒绝的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomRejectReason {
    /// 房间已满。
    Full = 0,
    /// 已在其它房间。
    AlreadyInRoom = 1,
    /// 房间无效/不存在。
    NotFound = 2,
}

impl RoomRejectReason {
    /// 线上 id。
    pub fn id(self) -> u8 {
        self as u8
    }

    /// 由线上 id 解析。
    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => Self::Full,
            1 => Self::AlreadyInRoom,
            2 => Self::NotFound,
            _ => return None,
        })
    }
}

/// 网络消息。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// 客户端握手：协议版本 + 客户端显示名。
    ClientHello {
        /// 客户端支持的协议版本。
        protocol_version: u16,
        /// 客户端名（最长 [`MAX_NAME_BYTES`]）。
        client_name: String,
    },
    /// 服务器握手响应：分配会话/玩家 ID。
    ServerWelcome {
        /// 分配的会话 ID。
        session_id: SessionId,
        /// 服务器期望的 tick rate。
        tick_rate: u16,
        /// 服务器当前 tick（可选参考）。
        server_tick: u64,
    },
    /// 状态快照占位：本 crate 不解释 payload，由同步层定义。
    Snapshot {
        /// 快照所属 tick。
        tick: u64,
        /// 快照序列号。
        seq: u32,
        /// 快照数据（上层序列化）。
        payload: Vec<u8>,
    },
    /// RPC 调用：target + method + payload。
    Rpc {
        /// 目标玩家；[`PlayerId::NONE`]（0）表示服务器。
        target: PlayerId,
        /// 方法名（最长 [`MAX_RPC_METHOD_BYTES`]）。
        method: String,
        /// 调用参数（上层序列化）。
        payload: Vec<u8>,
    },
    /// 心跳或心跳响应。
    Heartbeat {
        /// 随机/递增 nonce，响应原样带回。
        nonce: u64,
        /// `false` = 请求（ping），`true` = 响应（pong）。
        pong: bool,
    },
    /// 主动断开。
    Disconnect {
        /// 断开原因。
        reason: DisconnectReason,
    },
    /// 请求加入房间。
    JoinRoom {
        /// 房间 ID。
        room: RoomId,
    },
    /// 请求离开当前房间。
    LeaveRoom,
    /// 离开房间成功（服务器回给该客户端）。
    RoomLeft {
        /// 房间 ID。
        room: RoomId,
    },
    /// 加入成功（服务器回给该客户端）。
    RoomJoined {
        /// 房间 ID。
        room: RoomId,
        /// 加入前的现有成员。
        members: Vec<PlayerId>,
    },
    /// 房间内其他玩家加入（服务器广播）。
    PlayerJoined {
        /// 新玩家。
        player: PlayerId,
        /// 房间 ID。
        room: RoomId,
        /// 显示名。
        name: String,
    },
    /// 房间内玩家离开（服务器广播）。
    PlayerLeft {
        /// 离开的玩家。
        player: PlayerId,
        /// 房间 ID。
        room: RoomId,
    },
    /// 房间聊天（服务器广播）。
    Chat {
        /// 房间 ID。
        room: RoomId,
        /// 发送者。
        player: PlayerId,
        /// 文本（最长 [`MAX_CHAT_BYTES`]）。
        text: String,
    },
    /// 加入房间被拒绝。
    JoinRejected {
        /// 房间 ID。
        room: RoomId,
        /// 拒绝原因。
        reason: RoomRejectReason,
    },
}

impl Message {
    /// 消息类型。
    pub fn kind(&self) -> MessageKind {
        match self {
            Self::ClientHello { .. } => MessageKind::ClientHello,
            Self::ServerWelcome { .. } => MessageKind::ServerWelcome,
            Self::Snapshot { .. } => MessageKind::Snapshot,
            Self::Rpc { .. } => MessageKind::Rpc,
            Self::Heartbeat { .. } => MessageKind::Heartbeat,
            Self::Disconnect { .. } => MessageKind::Disconnect,
            Self::JoinRoom { .. } => MessageKind::JoinRoom,
            Self::LeaveRoom => MessageKind::LeaveRoom,
            Self::RoomLeft { .. } => MessageKind::RoomLeft,
            Self::RoomJoined { .. } => MessageKind::RoomJoined,
            Self::PlayerJoined { .. } => MessageKind::PlayerJoined,
            Self::PlayerLeft { .. } => MessageKind::PlayerLeft,
            Self::Chat { .. } => MessageKind::Chat,
            Self::JoinRejected { .. } => MessageKind::JoinRejected,
        }
    }

    /// 编码为默认上限的字节。
    pub fn encode(&self) -> Result<Vec<u8>, MessageError> {
        self.encode_with_limit(MAX_MESSAGE_BYTES)
    }

    /// 编码并校验总大小不超过 `max_bytes`。
    pub fn encode_with_limit(&self, max_bytes: usize) -> Result<Vec<u8>, MessageError> {
        let mut encoder = Encoder::new();
        encoder.u8(MESSAGE_VERSION);
        encoder.u16(self.kind().id());
        match self {
            Self::ClientHello {
                protocol_version,
                client_name,
            } => {
                encoder.u16(*protocol_version);
                encoder.string("client_name", client_name, MAX_NAME_BYTES)?;
            }
            Self::ServerWelcome {
                session_id,
                tick_rate,
                server_tick,
            } => {
                encoder.u32(session_id.raw());
                encoder.u16(*tick_rate);
                encoder.u64(*server_tick);
            }
            Self::Snapshot { tick, seq, payload } => {
                encoder.u64(*tick);
                encoder.u32(*seq);
                encoder.bytes("payload", payload, u16::MAX as usize)?;
            }
            Self::Rpc {
                target,
                method,
                payload,
            } => {
                encoder.u32(target.raw());
                encoder.string("method", method, MAX_RPC_METHOD_BYTES)?;
                encoder.bytes("payload", payload, u16::MAX as usize)?;
            }
            Self::Heartbeat { nonce, pong } => {
                encoder.u64(*nonce);
                encoder.u8(u8::from(*pong));
            }
            Self::Disconnect { reason } => {
                encoder.u8(reason.id());
            }
            Self::JoinRoom { room } => {
                encoder.u32(room.raw());
            }
            Self::LeaveRoom => {}
            Self::RoomLeft { room } => {
                encoder.u32(room.raw());
            }
            Self::RoomJoined { room, members } => {
                encoder.u32(room.raw());
                if members.len() > MAX_ROOM_MEMBERS {
                    return Err(MessageError::FieldTooLong {
                        field: "members",
                        size: members.len(),
                        max: MAX_ROOM_MEMBERS,
                    });
                }
                encoder.u16(members.len() as u16);
                for member in members {
                    encoder.u32(member.raw());
                }
            }
            Self::PlayerJoined { player, room, name } => {
                encoder.u32(player.raw());
                encoder.u32(room.raw());
                encoder.string("name", name, MAX_NAME_BYTES)?;
            }
            Self::PlayerLeft { player, room } => {
                encoder.u32(player.raw());
                encoder.u32(room.raw());
            }
            Self::Chat { room, player, text } => {
                encoder.u32(room.raw());
                encoder.u32(player.raw());
                encoder.string("text", text, MAX_CHAT_BYTES)?;
            }
            Self::JoinRejected { room, reason } => {
                encoder.u32(room.raw());
                encoder.u8(reason.id());
            }
        }
        let bytes = encoder.finish();
        if bytes.len() > max_bytes {
            return Err(MessageError::TooLarge {
                size: bytes.len(),
                max: max_bytes,
            });
        }
        Ok(bytes)
    }

    /// 解码默认上限内的消息。
    pub fn decode(bytes: &[u8]) -> Result<Self, MessageError> {
        if bytes.len() > MAX_MESSAGE_BYTES {
            return Err(MessageError::TooLarge {
                size: bytes.len(),
                max: MAX_MESSAGE_BYTES,
            });
        }
        let mut decoder = Decoder::new(bytes);
        let version = decoder.u8()?;
        if version != MESSAGE_VERSION {
            return Err(MessageError::UnsupportedVersion(version));
        }
        let kind_id = decoder.u16()?;
        let kind = MessageKind::from_id(kind_id).ok_or(MessageError::UnknownKind(kind_id))?;
        let message = match kind {
            MessageKind::ClientHello => Self::ClientHello {
                protocol_version: decoder.u16()?,
                client_name: decoder.string("client_name", MAX_NAME_BYTES)?,
            },
            MessageKind::ServerWelcome => Self::ServerWelcome {
                session_id: SessionId::new(decoder.u32()?),
                tick_rate: decoder.u16()?,
                server_tick: decoder.u64()?,
            },
            MessageKind::Snapshot => Self::Snapshot {
                tick: decoder.u64()?,
                seq: decoder.u32()?,
                payload: decoder.bytes("payload", u16::MAX as usize)?,
            },
            MessageKind::Rpc => Self::Rpc {
                target: PlayerId::new(decoder.u32()?),
                method: decoder.string("method", MAX_RPC_METHOD_BYTES)?,
                payload: decoder.bytes("payload", u16::MAX as usize)?,
            },
            MessageKind::Heartbeat => Self::Heartbeat {
                nonce: decoder.u64()?,
                pong: decoder.u8()? != 0,
            },
            MessageKind::Disconnect => Self::Disconnect {
                reason: DisconnectReason::from_id(decoder.u8()?)
                    .ok_or(MessageError::UnknownKind(kind_id))?,
            },
            MessageKind::JoinRoom => Self::JoinRoom {
                room: RoomId::new(decoder.u32()?),
            },
            MessageKind::LeaveRoom => Self::LeaveRoom,
            MessageKind::RoomLeft => Self::RoomLeft {
                room: RoomId::new(decoder.u32()?),
            },
            MessageKind::RoomJoined => {
                let room = RoomId::new(decoder.u32()?);
                let count = decoder.u16()? as usize;
                if count > MAX_ROOM_MEMBERS {
                    return Err(MessageError::FieldTooLong {
                        field: "members",
                        size: count,
                        max: MAX_ROOM_MEMBERS,
                    });
                }
                let mut members = Vec::with_capacity(count);
                for _ in 0..count {
                    members.push(PlayerId::new(decoder.u32()?));
                }
                Self::RoomJoined { room, members }
            }
            MessageKind::PlayerJoined => Self::PlayerJoined {
                player: PlayerId::new(decoder.u32()?),
                room: RoomId::new(decoder.u32()?),
                name: decoder.string("name", MAX_NAME_BYTES)?,
            },
            MessageKind::PlayerLeft => Self::PlayerLeft {
                player: PlayerId::new(decoder.u32()?),
                room: RoomId::new(decoder.u32()?),
            },
            MessageKind::Chat => Self::Chat {
                room: RoomId::new(decoder.u32()?),
                player: PlayerId::new(decoder.u32()?),
                text: decoder.string("text", MAX_CHAT_BYTES)?,
            },
            MessageKind::JoinRejected => Self::JoinRejected {
                room: RoomId::new(decoder.u32()?),
                reason: RoomRejectReason::from_id(decoder.u8()?)
                    .ok_or(MessageError::UnknownKind(kind_id))?,
            },
        };
        decoder.finish()?;
        Ok(message)
    }
}

struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(32),
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn bytes(&mut self, field: &'static str, data: &[u8], max: usize) -> Result<(), MessageError> {
        if data.len() > max {
            return Err(MessageError::FieldTooLong {
                field,
                size: data.len(),
                max,
            });
        }
        self.u16(data.len() as u16);
        self.bytes.extend_from_slice(data);
        Ok(())
    }

    fn string(&mut self, field: &'static str, value: &str, max: usize) -> Result<(), MessageError> {
        self.bytes(field, value.as_bytes(), max)
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn ensure(&self, count: usize) -> Result<(), MessageError> {
        if self.offset + count > self.bytes.len() {
            return Err(MessageError::Truncated);
        }
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, MessageError> {
        self.ensure(1)?;
        let value = self.bytes[self.offset];
        self.offset += 1;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, MessageError> {
        self.ensure(2)?;
        let value = u16::from_le_bytes([self.bytes[self.offset], self.bytes[self.offset + 1]]);
        self.offset += 2;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, MessageError> {
        self.ensure(4)?;
        let mut raw = [0u8; 4];
        raw.copy_from_slice(&self.bytes[self.offset..self.offset + 4]);
        self.offset += 4;
        Ok(u32::from_le_bytes(raw))
    }

    fn u64(&mut self) -> Result<u64, MessageError> {
        self.ensure(8)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&self.bytes[self.offset..self.offset + 8]);
        self.offset += 8;
        Ok(u64::from_le_bytes(raw))
    }

    fn bytes(&mut self, field: &'static str, max: usize) -> Result<Vec<u8>, MessageError> {
        let length = self.u16()? as usize;
        if length > max {
            return Err(MessageError::FieldTooLong {
                field,
                size: length,
                max,
            });
        }
        self.ensure(length)?;
        let data = self.bytes[self.offset..self.offset + length].to_vec();
        self.offset += length;
        Ok(data)
    }

    fn string(&mut self, field: &'static str, max: usize) -> Result<String, MessageError> {
        let bytes = self.bytes(field, max)?;
        String::from_utf8(bytes).map_err(|_| MessageError::InvalidUtf8)
    }

    fn finish(self) -> Result<(), MessageError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(MessageError::TrailingBytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(message: Message) {
        let bytes = message.encode().expect("encode");
        let decoded = Message::decode(&bytes).expect("decode");
        assert_eq!(decoded, message);
    }

    #[test]
    fn all_message_variants_roundtrip() {
        roundtrip(Message::ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "alice".to_string(),
        });
        roundtrip(Message::ServerWelcome {
            session_id: SessionId::new(7),
            tick_rate: 60,
            server_tick: 42,
        });
        roundtrip(Message::Snapshot {
            tick: 100,
            seq: 3,
            payload: vec![1, 2, 3, 4],
        });
        roundtrip(Message::Rpc {
            target: PlayerId::NONE,
            method: "spawn".to_string(),
            payload: b"params".to_vec(),
        });
        roundtrip(Message::Heartbeat {
            nonce: 99,
            pong: false,
        });
        roundtrip(Message::Heartbeat {
            nonce: 99,
            pong: true,
        });
        roundtrip(Message::Disconnect {
            reason: DisconnectReason::Normal,
        });
        roundtrip(Message::JoinRoom {
            room: RoomId::new(3),
        });
        roundtrip(Message::LeaveRoom);
        roundtrip(Message::RoomLeft {
            room: RoomId::new(3),
        });
        roundtrip(Message::RoomJoined {
            room: RoomId::new(3),
            members: vec![PlayerId::new(1), PlayerId::new(2)],
        });
        roundtrip(Message::PlayerJoined {
            player: PlayerId::new(2),
            room: RoomId::new(3),
            name: "bob".to_string(),
        });
        roundtrip(Message::PlayerLeft {
            player: PlayerId::new(2),
            room: RoomId::new(3),
        });
        roundtrip(Message::Chat {
            room: RoomId::new(3),
            player: PlayerId::new(1),
            text: "hello".to_string(),
        });
        roundtrip(Message::JoinRejected {
            room: RoomId::new(3),
            reason: RoomRejectReason::Full,
        });
    }

    #[test]
    fn oversized_message_is_rejected() {
        let message = Message::Snapshot {
            tick: 0,
            seq: 0,
            payload: vec![0u8; MAX_MESSAGE_BYTES],
        };
        assert!(matches!(
            message.encode(),
            Err(MessageError::TooLarge { .. })
        ));
        // 小 limit 下同样拒绝。
        let small = Message::Chat {
            room: RoomId::new(1),
            player: PlayerId::new(1),
            text: "hello".to_string(),
        };
        assert!(matches!(
            small.encode_with_limit(4),
            Err(MessageError::TooLarge { .. })
        ));
    }

    #[test]
    fn field_limits_are_enforced() {
        let long_name = "x".repeat(MAX_NAME_BYTES + 1);
        let message = Message::ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: long_name,
        };
        assert!(matches!(
            message.encode(),
            Err(MessageError::FieldTooLong {
                field: "client_name",
                ..
            })
        ));

        let long_chat = "y".repeat(MAX_CHAT_BYTES + 1);
        let message = Message::Chat {
            room: RoomId::new(1),
            player: PlayerId::new(1),
            text: long_chat,
        };
        assert!(matches!(
            message.encode(),
            Err(MessageError::FieldTooLong { field: "text", .. })
        ));
    }

    #[test]
    fn decode_rejects_bad_input() {
        assert_eq!(Message::decode(&[]), Err(MessageError::Truncated));
        assert_eq!(
            Message::decode(&[99, 0, 0]),
            Err(MessageError::UnsupportedVersion(99))
        );
        assert_eq!(
            Message::decode(&[MESSAGE_VERSION, 0xEE, 0xEE]),
            Err(MessageError::UnknownKind(0xEEEE))
        );
        let bytes = Message::LeaveRoom.encode().unwrap();
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(Message::decode(&trailing), Err(MessageError::TrailingBytes));
        // 截断：去掉最后一个字节。
        let mut truncated = bytes;
        truncated.pop();
        // LeaveRoom 的 payload 为空，截断的是 kind 低位，应视为截断/未知；这里只断言有错误。
        assert!(Message::decode(&truncated).is_err());
    }

    #[test]
    fn message_kind_ids_are_stable() {
        assert_eq!(MessageKind::ClientHello.id(), 1);
        assert_eq!(MessageKind::ServerWelcome.id(), 2);
        assert_eq!(MessageKind::Snapshot.id(), 3);
        assert_eq!(MessageKind::Rpc.id(), 4);
        assert_eq!(MessageKind::Heartbeat.id(), 5);
        assert_eq!(MessageKind::Disconnect.id(), 6);
        assert_eq!(MessageKind::from_id(13), Some(MessageKind::JoinRejected));
        assert_eq!(MessageKind::from_id(14), Some(MessageKind::RoomLeft));
        assert_eq!(MessageKind::from_id(0), None);
    }
}
