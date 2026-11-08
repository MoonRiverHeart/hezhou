//! 会话层：握手、序列号、心跳、超时断开。
//!
//! 每个网络对端（[`PeerId`]）对应一个 [`Session`]：
//!
//! - **服务器**：收到未知对端合法的 [`Message::ClientHello`] 后分配 session/player id，
//!   回 [`Message::ServerWelcome`]，会话立即进入 `Connected`；
//! - **客户端**：`connect` 进入 `Connecting` 并发送 Hello；收到 Welcome 后进入 `Connected`；
//! - **序列号**：每个方向独立递增（从 1 开始）；重复/回退包被丢弃并计数，
//!   跳号包被接收但计入 `out_of_order_packets`（UDP 乱序容忍）；
//! - **心跳**：双方按 `heartbeat_interval` 发送 `Heartbeat{pong:false}`，收到后自动回 pong；
//! - **超时**：`now - last_seen > timeout`（服务器）或 > `handshake_timeout`（Connecting）
//!   会移除会话并产生 [`SessionEvent::TimedOut`]。
//!
//! 时间由调用方传入 `Instant::now()`（或测试中的合成时间），便于确定性测试。

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::error::{MessageError, SessionError};
use crate::ids::{PeerId, PlayerId, SessionId};
use crate::message::{DisconnectReason, Message, PROTOCOL_VERSION};

/// 默认心跳间隔。
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
/// 默认会话超时。
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(10);
/// 默认握手超时。
pub const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// 数据报内 Packet 头（sequence u32 LE）。
pub const PACKET_HEADER_BYTES: usize = 4;

/// 会话配置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionConfig {
    /// 协议版本（握手校验）。
    pub protocol_version: u16,
    /// 心跳间隔。
    pub heartbeat_interval: Duration,
    /// 会话超时（任意有效包都会刷新 last_seen）。
    pub timeout: Duration,
    /// 握手超时。
    pub handshake_timeout: Duration,
    /// 期望 tick rate（写入 ServerWelcome）。
    pub tick_rate: u16,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            timeout: DEFAULT_SESSION_TIMEOUT,
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            tick_rate: 60,
        }
    }
}

/// 会话角色。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionRole {
    /// 服务器：接受 Hello、分配 ID。
    Server,
    /// 客户端：主动 connect、等待 Welcome。
    Client,
}

/// 会话状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// 客户端已发 Hello，等待 Welcome。
    Connecting,
    /// 已握手，可收发消息。
    Connected,
    /// 已因超时移除（仅用于事件/记录）。
    TimedOut,
    /// 已主动关闭。
    Closed,
}

/// 会话统计。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionStats {
    /// 发送包数。
    pub packets_sent: u64,
    /// 接收包数（含握手包）。
    pub packets_received: u64,
    /// 发送字节数。
    pub bytes_sent: u64,
    /// 接收字节数。
    pub bytes_received: u64,
    /// 重复/回退序列号被丢弃的次数。
    pub duplicate_packets: u64,
    /// 跳号（乱序）接收次数。
    pub out_of_order_packets: u64,
    /// 发送的心跳请求数。
    pub heartbeats_sent: u64,
    /// 收到的心跳请求数。
    pub heartbeats_received: u64,
    /// 收到的心跳响应数。
    pub pongs_received: u64,
    /// 无法解码的包数量。
    pub protocol_errors: u64,
}

/// 带序列号的网络包（数据报 payload）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    /// 每个方向独立递增的序列号（从 1 开始）。
    pub sequence: u32,
    /// 消息。
    pub message: Message,
}

impl Packet {
    /// 编码为 `[seq u32 LE][message]`，总大小不超过 1200 字节。
    pub fn encode(&self) -> Result<Vec<u8>, MessageError> {
        let message = self
            .message
            .encode_with_limit(crate::message::MAX_MESSAGE_BYTES - PACKET_HEADER_BYTES)?;
        let mut bytes = Vec::with_capacity(PACKET_HEADER_BYTES + message.len());
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes.extend_from_slice(&message);
        Ok(bytes)
    }

    /// 解码。
    pub fn decode(bytes: &[u8]) -> Result<Self, MessageError> {
        if bytes.len() > crate::message::MAX_MESSAGE_BYTES {
            return Err(MessageError::TooLarge {
                size: bytes.len(),
                max: crate::message::MAX_MESSAGE_BYTES,
            });
        }
        if bytes.len() < PACKET_HEADER_BYTES {
            return Err(MessageError::Truncated);
        }
        let sequence = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let message = Message::decode(&bytes[PACKET_HEADER_BYTES..])?;
        Ok(Self { sequence, message })
    }
}

/// 一个网络对端的会话状态。
#[derive(Clone, Debug)]
pub struct Session {
    role: SessionRole,
    peer: PeerId,
    state: SessionState,
    session_id: SessionId,
    player_id: PlayerId,
    remote_name: String,
    created_at: Instant,
    last_seen: Instant,
    last_heartbeat_sent: Instant,
    next_sequence: u32,
    last_received_sequence: Option<u32>,
    heartbeat_nonce: u64,
    stats: SessionStats,
}

impl Session {
    fn new(role: SessionRole, peer: PeerId, now: Instant) -> Self {
        Self {
            role,
            peer,
            state: SessionState::Connecting,
            session_id: SessionId::NONE,
            player_id: PlayerId::NONE,
            remote_name: String::new(),
            created_at: now,
            last_seen: now,
            last_heartbeat_sent: now,
            next_sequence: 1,
            last_received_sequence: None,
            heartbeat_nonce: 0,
            stats: SessionStats::default(),
        }
    }

    /// 会话角色。
    pub fn role(&self) -> SessionRole {
        self.role
    }

    /// 对端地址。
    pub fn peer(&self) -> PeerId {
        self.peer
    }

    /// 当前状态。
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// 服务器分配的 session id（客户端握手完成前为 0）。
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// 玩家 id（当前实现与 session id 同值）。
    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    /// 对端显示名（服务器侧来自 ClientHello；客户端侧为 "server"）。
    pub fn remote_name(&self) -> &str {
        &self.remote_name
    }

    /// 是否已连接。
    pub fn is_connected(&self) -> bool {
        self.state == SessionState::Connected
    }

    /// 创建时间。
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// 最近一次收到有效包的时间。
    pub fn last_seen(&self) -> Instant {
        self.last_seen
    }

    /// 统计快照。
    pub fn stats(&self) -> SessionStats {
        self.stats
    }

    /// 下一个待发送序列号（调试用）。
    pub fn next_sequence(&self) -> u32 {
        self.next_sequence
    }
}

/// 需要发送的数据报（peer + 已编码 Packet 字节）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboundDatagram {
    /// 目标对端。
    pub peer: PeerId,
    /// 已编码的 `[seq][message]` 字节。
    pub bytes: Vec<u8>,
}

/// 会话事件（服务器/客户端共用的底层事件）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    /// 握手完成。
    Connected {
        /// 对端。
        peer: PeerId,
        /// 会话 ID。
        session_id: SessionId,
        /// 玩家 ID。
        player_id: PlayerId,
        /// 对端显示名。
        remote_name: String,
    },
    /// 收到业务消息（握手/心跳/断开等内部消息不会出现在这里）。
    Message {
        /// 对端。
        peer: PeerId,
        /// 会话 ID。
        session_id: SessionId,
        /// 消息。
        message: Message,
    },
    /// 超时（握手或会话）。
    TimedOut {
        /// 对端。
        peer: PeerId,
        /// 会话 ID（握手前为 0）。
        session_id: SessionId,
        /// 玩家 ID（握手前为 0）。
        player_id: PlayerId,
    },
    /// 主动断开（收到 Disconnect 或本地 disconnect）。
    Closed {
        /// 对端。
        peer: PeerId,
        /// 会话 ID。
        session_id: SessionId,
        /// 玩家 ID。
        player_id: PlayerId,
        /// 原因。
        reason: DisconnectReason,
    },
}

/// 会话管理器（一个服务器可管理多个客户端；一个客户端只连一个服务器）。
#[derive(Clone, Debug)]
pub struct SessionManager {
    role: SessionRole,
    config: SessionConfig,
    sessions: BTreeMap<PeerId, Session>,
    events: Vec<SessionEvent>,
    outbox: Vec<OutboundDatagram>,
    next_session_id: u32,
    max_sessions: Option<usize>,
}

impl SessionManager {
    /// 创建服务器端管理器。
    pub fn new_server(config: SessionConfig) -> Self {
        Self::new(SessionRole::Server, config)
    }

    /// 创建客户端管理器。
    pub fn new_client(config: SessionConfig) -> Self {
        Self::new(SessionRole::Client, config)
    }

    fn new(role: SessionRole, config: SessionConfig) -> Self {
        Self {
            role,
            config,
            sessions: BTreeMap::new(),
            events: Vec::new(),
            outbox: Vec::new(),
            next_session_id: 1,
            max_sessions: None,
        }
    }

    /// 设置服务器最大会话数（仅服务器角色生效）。
    pub fn with_max_sessions(mut self, max_sessions: usize) -> Self {
        self.max_sessions = Some(max_sessions.max(1));
        self
    }

    /// 角色。
    pub fn role(&self) -> SessionRole {
        self.role
    }

    /// 配置。
    pub fn config(&self) -> SessionConfig {
        self.config
    }

    /// 会话数量。
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 指定对端的会话。
    pub fn session(&self, peer: PeerId) -> Option<&Session> {
        self.sessions.get(&peer)
    }

    /// 指定对端的会话（可变）。
    pub fn session_mut(&mut self, peer: PeerId) -> Option<&mut Session> {
        self.sessions.get_mut(&peer)
    }

    /// 所有会话。
    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    /// 由 session id 查会话。
    pub fn session_by_id(&self, session_id: SessionId) -> Option<&Session> {
        self.sessions
            .values()
            .find(|session| session.session_id == session_id)
    }

    /// 客户端：连接服务器（发送 ClientHello）。
    pub fn connect(
        &mut self,
        server: PeerId,
        client_name: &str,
        now: Instant,
    ) -> Result<(), SessionError> {
        if self.role != SessionRole::Client {
            return Err(SessionError::WrongRole);
        }
        if self.sessions.contains_key(&server) {
            return Ok(());
        }
        let mut session = Session::new(SessionRole::Client, server, now);
        session.remote_name = "server".to_string();
        let hello = Message::ClientHello {
            protocol_version: self.config.protocol_version,
            client_name: client_name.to_string(),
        };
        queue_packet(&mut session, hello, &mut self.outbox)?;
        self.sessions.insert(server, session);
        Ok(())
    }

    /// 主动断开：发 Disconnect（尽力）并移除会话。
    pub fn disconnect(&mut self, peer: PeerId, reason: DisconnectReason) -> bool {
        let Some(mut session) = self.sessions.remove(&peer) else {
            return false;
        };
        if session.state == SessionState::Connected {
            let _ = queue_packet(
                &mut session,
                Message::Disconnect { reason },
                &mut self.outbox,
            );
        }
        self.events.push(SessionEvent::Closed {
            peer,
            session_id: session.session_id,
            player_id: session.player_id,
            reason,
        });
        true
    }

    /// 发送业务消息（要求会话已连接）。
    pub fn send(&mut self, peer: PeerId, message: Message) -> Result<(), SessionError> {
        let Some(session) = self.sessions.get_mut(&peer) else {
            return Err(SessionError::UnknownPeer);
        };
        if !session.is_connected() {
            return Err(SessionError::NotConnected);
        }
        queue_packet(session, message, &mut self.outbox)?;
        Ok(())
    }

    /// 处理收到的数据报字节（解码失败会被计数并忽略）。
    pub fn handle_datagram(&mut self, peer: PeerId, bytes: &[u8], now: Instant) {
        let packet = match Packet::decode(bytes) {
            Ok(packet) => packet,
            Err(_) => {
                if let Some(session) = self.sessions.get_mut(&peer) {
                    session.stats.protocol_errors = session.stats.protocol_errors.saturating_add(1);
                }
                return;
            }
        };

        // 服务器：未知对端必须是 ClientHello。
        if self.role == SessionRole::Server && !self.sessions.contains_key(&peer) {
            let Message::ClientHello {
                protocol_version,
                client_name,
            } = &packet.message
            else {
                return;
            };
            if *protocol_version != self.config.protocol_version {
                // 协议版本不匹配：尽力回 Disconnect（不进会话表）。
                if let Ok(bytes) = (Packet {
                    sequence: 1,
                    message: Message::Disconnect {
                        reason: DisconnectReason::ProtocolError,
                    },
                })
                .encode()
                {
                    self.outbox.push(OutboundDatagram { peer, bytes });
                }
                return;
            }
            if let Some(max) = self.max_sessions {
                if self.sessions.len() >= max {
                    if let Ok(bytes) = (Packet {
                        sequence: 1,
                        message: Message::Disconnect {
                            reason: DisconnectReason::ServerShutdown,
                        },
                    })
                    .encode()
                    {
                        self.outbox.push(OutboundDatagram { peer, bytes });
                    }
                    return;
                }
            }
            let session_id = SessionId::new(self.next_session_id);
            self.next_session_id = self.next_session_id.wrapping_add(1).max(1);
            let mut session = Session::new(SessionRole::Server, peer, now);
            session.state = SessionState::Connected;
            session.session_id = session_id;
            session.player_id = PlayerId::new(session_id.raw());
            session.remote_name = client_name.clone();
            session.last_seen = now;
            session.last_heartbeat_sent = now;
            session.last_received_sequence = Some(packet.sequence);
            session.stats.packets_received = 1;
            session.stats.bytes_received = bytes.len() as u64;
            let welcome = Message::ServerWelcome {
                session_id,
                tick_rate: self.config.tick_rate,
                server_tick: 0,
            };
            let _ = queue_packet(&mut session, welcome, &mut self.outbox);
            self.events.push(SessionEvent::Connected {
                peer,
                session_id,
                player_id: session.player_id,
                remote_name: session.remote_name.clone(),
            });
            self.sessions.insert(peer, session);
            return;
        }

        // 已知对端：拆开字段借用，便于同时修改会话/事件/发送队列。
        let SessionManager {
            sessions,
            events,
            outbox,
            ..
        } = self;
        let Some(session) = sessions.get_mut(&peer) else {
            return;
        };
        session.stats.packets_received = session.stats.packets_received.saturating_add(1);
        session.stats.bytes_received = session
            .stats
            .bytes_received
            .saturating_add(bytes.len() as u64);
        if let Some(last) = session.last_received_sequence {
            if packet.sequence <= last {
                session.stats.duplicate_packets = session.stats.duplicate_packets.saturating_add(1);
                return;
            }
            if packet.sequence > last.saturating_add(1) {
                session.stats.out_of_order_packets =
                    session.stats.out_of_order_packets.saturating_add(1);
            }
        }
        session.last_received_sequence = Some(packet.sequence);
        session.last_seen = now;

        // 握手阶段：客户端等待 Welcome；服务器拒绝（Disconnect）也要能感知。
        if session.state == SessionState::Connecting {
            let mut closed_reason = None;
            match &packet.message {
                Message::ServerWelcome { session_id, .. } => {
                    session.session_id = *session_id;
                    session.player_id = PlayerId::new(session_id.raw());
                    session.state = SessionState::Connected;
                    events.push(SessionEvent::Connected {
                        peer,
                        session_id: *session_id,
                        player_id: PlayerId::new(session_id.raw()),
                        remote_name: session.remote_name.clone(),
                    });
                }
                Message::Disconnect { reason } => closed_reason = Some(*reason),
                _ => {}
            }
            if let Some(reason) = closed_reason {
                let session_id = session.session_id;
                let player_id = session.player_id;
                sessions.remove(&peer);
                events.push(SessionEvent::Closed {
                    peer,
                    session_id,
                    player_id,
                    reason,
                });
            }
            return;
        }

        let mut disconnect: Option<(DisconnectReason, SessionId, PlayerId)> = None;
        match packet.message {
            Message::Heartbeat { nonce, pong } => {
                if pong {
                    session.stats.pongs_received = session.stats.pongs_received.saturating_add(1);
                } else {
                    session.stats.heartbeats_received =
                        session.stats.heartbeats_received.saturating_add(1);
                    let response = Message::Heartbeat { nonce, pong: true };
                    let _ = queue_packet(session, response, outbox);
                }
            }
            Message::Disconnect { reason } => {
                disconnect = Some((reason, session.session_id, session.player_id));
            }
            // 已连接后重复的握手消息直接忽略。
            Message::ClientHello { .. } | Message::ServerWelcome { .. } => {}
            message => {
                events.push(SessionEvent::Message {
                    peer,
                    session_id: session.session_id,
                    message,
                });
            }
        }
        if let Some((reason, session_id, player_id)) = disconnect {
            sessions.remove(&peer);
            events.push(SessionEvent::Closed {
                peer,
                session_id,
                player_id,
                reason,
            });
        }
    }

    /// 推进时间：发送心跳、检查超时。
    pub fn tick(&mut self, now: Instant) {
        let mut timed_out: Vec<PeerId> = Vec::new();
        for session in self.sessions.values_mut() {
            let elapsed = now.saturating_duration_since(session.last_seen);
            match session.state {
                SessionState::Connecting => {
                    if elapsed >= self.config.handshake_timeout {
                        timed_out.push(session.peer);
                    }
                }
                SessionState::Connected => {
                    if elapsed >= self.config.timeout {
                        timed_out.push(session.peer);
                    } else if now.saturating_duration_since(session.last_heartbeat_sent)
                        >= self.config.heartbeat_interval
                    {
                        session.heartbeat_nonce = session.heartbeat_nonce.wrapping_add(1);
                        session.stats.heartbeats_sent =
                            session.stats.heartbeats_sent.saturating_add(1);
                        session.last_heartbeat_sent = now;
                        let heartbeat = Message::Heartbeat {
                            nonce: session.heartbeat_nonce,
                            pong: false,
                        };
                        let _ = queue_packet(session, heartbeat, &mut self.outbox);
                    }
                }
                SessionState::TimedOut | SessionState::Closed => {
                    timed_out.push(session.peer);
                }
            }
        }
        for peer in timed_out {
            if let Some(mut session) = self.sessions.remove(&peer) {
                session.state = SessionState::TimedOut;
                self.events.push(SessionEvent::TimedOut {
                    peer,
                    session_id: session.session_id,
                    player_id: session.player_id,
                });
            }
        }
    }

    /// 取走缓冲事件。
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    /// 取走待发送数据报。
    pub fn take_outbound(&mut self) -> Vec<OutboundDatagram> {
        std::mem::take(&mut self.outbox)
    }

    /// 是否还有待发送数据报。
    pub fn has_outbound(&self) -> bool {
        !self.outbox.is_empty()
    }
}

fn queue_packet(
    session: &mut Session,
    message: Message,
    outbox: &mut Vec<OutboundDatagram>,
) -> Result<(), MessageError> {
    let sequence = session.next_sequence;
    session.next_sequence = session.next_sequence.wrapping_add(1).max(1);
    let bytes = Packet { sequence, message }.encode()?;
    session.stats.packets_sent = session.stats.packets_sent.saturating_add(1);
    session.stats.bytes_sent = session.stats.bytes_sent.saturating_add(bytes.len() as u64);
    outbox.push(OutboundDatagram {
        peer: session.peer,
        bytes,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SessionConfig {
        SessionConfig {
            heartbeat_interval: Duration::from_millis(100),
            timeout: Duration::from_millis(500),
            handshake_timeout: Duration::from_millis(200),
            ..SessionConfig::default()
        }
    }

    fn peer(port: u16) -> PeerId {
        PeerId::new(format!("127.0.0.1:{port}").parse().unwrap())
    }

    fn packet(sequence: u32, message: Message) -> Vec<u8> {
        Packet { sequence, message }.encode().unwrap()
    }

    #[test]
    fn packet_roundtrip_and_limits() {
        let packet = Packet {
            sequence: 9,
            message: Message::Heartbeat {
                nonce: 1,
                pong: false,
            },
        };
        let bytes = packet.encode().unwrap();
        assert_eq!(Packet::decode(&bytes).unwrap(), packet);
        assert_eq!(Packet::decode(&[0, 1]), Err(MessageError::Truncated));
        // 超过 1200 字节的包被拒绝。
        let big = Packet {
            sequence: 1,
            message: Message::Snapshot {
                tick: 0,
                seq: 0,
                payload: vec![0u8; crate::message::MAX_MESSAGE_BYTES],
            },
        };
        assert!(big.encode().is_err());
    }

    #[test]
    fn server_handshake_assigns_ids_and_emits_events() {
        let mut server = SessionManager::new_server(config());
        let client = peer(1000);
        let now = Instant::now();
        server.handle_datagram(
            client,
            &packet(
                1,
                Message::ClientHello {
                    protocol_version: PROTOCOL_VERSION,
                    client_name: "alice".to_string(),
                },
            ),
            now,
        );
        assert_eq!(server.session_count(), 1);
        let events = server.take_events();
        assert!(matches!(
            &events[0],
            SessionEvent::Connected { peer, session_id, player_id, remote_name }
                if *peer == client && session_id.raw() == 1 && player_id.raw() == 1 && remote_name == "alice"
        ));
        let outbound = server.take_outbound();
        assert_eq!(outbound.len(), 1);
        assert!(matches!(
            Packet::decode(&outbound[0].bytes).unwrap().message,
            Message::ServerWelcome { session_id, .. } if session_id.raw() == 1
        ));
        // 客户端收到 Welcome 后发送业务消息。
        let session = server.session(client).unwrap();
        let client_seq = session.last_received_sequence.unwrap();
        assert_eq!(client_seq, 1);
    }

    #[test]
    fn heartbeat_roundtrip_updates_stats() {
        let mut server = SessionManager::new_server(config());
        let mut client = SessionManager::new_client(config());
        let server_peer = peer(2000);
        let client_peer = peer(2001);
        let now = Instant::now();
        server.handle_datagram(
            client_peer,
            &packet(
                1,
                Message::ClientHello {
                    protocol_version: PROTOCOL_VERSION,
                    client_name: "bob".to_string(),
                },
            ),
            now,
        );
        client.connect(server_peer, "bob", now).unwrap();
        // 丢弃客户端初始 Hello（本测试直接注入 Welcome，不经过网络）。
        let _ = client.take_outbound();
        let welcome = server.take_outbound();
        client.handle_datagram(server_peer, &welcome[0].bytes, now);
        assert!(client.session(server_peer).unwrap().is_connected());

        // 服务器发心跳。
        server.tick(now + Duration::from_millis(150));
        let heartbeats = server.take_outbound();
        assert_eq!(heartbeats.len(), 1);
        client.handle_datagram(
            server_peer,
            &heartbeats[0].bytes,
            now + Duration::from_millis(160),
        );
        assert_eq!(
            client
                .session(server_peer)
                .unwrap()
                .stats()
                .heartbeats_received,
            1
        );
        let pongs = client.take_outbound();
        assert_eq!(pongs.len(), 1);
        server.handle_datagram(
            client_peer,
            &pongs[0].bytes,
            now + Duration::from_millis(170),
        );
        assert_eq!(
            server.session(client_peer).unwrap().stats().pongs_received,
            1
        );
    }

    #[test]
    fn duplicate_and_out_of_order_sequences_are_counted() {
        let mut server = SessionManager::new_server(config());
        let client = peer(3000);
        let now = Instant::now();
        server.handle_datagram(
            client,
            &packet(
                1,
                Message::ClientHello {
                    protocol_version: PROTOCOL_VERSION,
                    client_name: "c".to_string(),
                },
            ),
            now,
        );
        server.take_events();
        server.take_outbound();
        // 重复 seq=1 → 丢弃。
        server.handle_datagram(
            client,
            &packet(
                1,
                Message::Chat {
                    room: crate::ids::RoomId::new(1),
                    player: PlayerId::new(1),
                    text: "dup".to_string(),
                },
            ),
            now,
        );
        assert_eq!(server.session(client).unwrap().stats().duplicate_packets, 1);
        // 跳号 seq=4 → 接收 + 计数。
        server.handle_datagram(
            client,
            &packet(
                4,
                Message::Chat {
                    room: crate::ids::RoomId::new(1),
                    player: PlayerId::new(1),
                    text: "gap".to_string(),
                },
            ),
            now,
        );
        assert_eq!(
            server.session(client).unwrap().stats().out_of_order_packets,
            1
        );
        let events = server.take_events();
        assert!(events.iter().any(|event| matches!(
            event,
            SessionEvent::Message { message: Message::Chat { text, .. }, .. } if text == "gap"
        )));
    }

    #[test]
    fn timeout_removes_session_and_emits_event() {
        let mut server = SessionManager::new_server(config());
        let client = peer(4000);
        let now = Instant::now();
        server.handle_datagram(
            client,
            &packet(
                1,
                Message::ClientHello {
                    protocol_version: PROTOCOL_VERSION,
                    client_name: "d".to_string(),
                },
            ),
            now,
        );
        server.take_events();
        server.tick(now + Duration::from_millis(600));
        assert_eq!(server.session_count(), 0);
        assert!(matches!(
            server.take_events().as_slice(),
            [SessionEvent::TimedOut { peer, .. }] if *peer == client
        ));
    }

    #[test]
    fn client_handshake_timeout_when_no_welcome() {
        let mut client = SessionManager::new_client(config());
        let server_peer = peer(5000);
        let now = Instant::now();
        client.connect(server_peer, "e", now).unwrap();
        assert_eq!(
            client.session(server_peer).unwrap().session_id(),
            SessionId::NONE
        );
        client.tick(now + Duration::from_millis(250));
        assert_eq!(client.session_count(), 0);
        assert!(matches!(
            client.take_events().as_slice(),
            [SessionEvent::TimedOut { .. }]
        ));
    }

    #[test]
    fn disconnect_and_api_errors() {
        let mut server = SessionManager::new_server(config());
        let client = peer(6000);
        let now = Instant::now();
        server.handle_datagram(
            client,
            &packet(
                1,
                Message::ClientHello {
                    protocol_version: PROTOCOL_VERSION,
                    client_name: "f".to_string(),
                },
            ),
            now,
        );
        server.take_events();
        server.take_outbound();
        server.send(client, Message::LeaveRoom).unwrap();
        assert!(server.has_outbound());
        assert!(matches!(
            server.send(peer(6550), Message::LeaveRoom),
            Err(SessionError::UnknownPeer)
        ));
        assert!(server.disconnect(client, DisconnectReason::Normal));
        assert_eq!(server.session_count(), 0);
        assert!(matches!(
            server.take_events().as_slice(),
            [SessionEvent::Closed {
                reason: DisconnectReason::Normal,
                ..
            }]
        ));
        // 已断开后发送会失败。
        assert!(matches!(
            server.send(client, Message::LeaveRoom),
            Err(SessionError::UnknownPeer)
        ));
    }

    #[test]
    fn server_rejects_unknown_non_hello_packet() {
        let mut server = SessionManager::new_server(config());
        let now = Instant::now();
        server.handle_datagram(
            peer(7000),
            &packet(
                1,
                Message::Heartbeat {
                    nonce: 1,
                    pong: false,
                },
            ),
            now,
        );
        assert_eq!(server.session_count(), 0);
    }

    #[test]
    fn server_enforces_protocol_version() {
        let mut server = SessionManager::new_server(config());
        let client = peer(8000);
        let now = Instant::now();
        server.handle_datagram(
            client,
            &packet(
                1,
                Message::ClientHello {
                    protocol_version: PROTOCOL_VERSION + 1,
                    client_name: "bad".to_string(),
                },
            ),
            now,
        );
        assert_eq!(server.session_count(), 0);
        let outbound = server.take_outbound();
        assert_eq!(outbound.len(), 1);
        assert!(matches!(
            Packet::decode(&outbound[0].bytes).unwrap().message,
            Message::Disconnect {
                reason: DisconnectReason::ProtocolError
            }
        ));
    }
}
