//! UDP 客户端：连接服务器、维持会话、发消息与房间操作。
//!
//! 用法：
//!
//! ```no_run
//! use hezhou_net::{NetClient, NetConfig};
//! use std::time::Instant;
//!
//! let config = NetConfig::loopback();
//! let mut client = NetClient::connect("127.0.0.1:27015".parse().unwrap(), "alice", &config)?;
//! let now = Instant::now();
//! client.tick(now);
//! for event in client.poll(now, 32) {
//!     // 处理 Connected/Message/TimedOut...
//! }
//! # Ok::<(), hezhou_net::TransportError>(())
//! ```
//!
//! 客户端只连一个服务器；房间状态由 `RoomJoined`/`RoomLeft`/`JoinRejected` 消息驱动。

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Instant;

use crate::config::NetConfig;
use crate::error::{SessionError, TransportError};
use crate::ids::{PeerId, PlayerId, RoomId, SessionId};
use crate::message::{DisconnectReason, Message};
use crate::session::{SessionEvent, SessionManager, SessionStats};
use crate::transport::Transport;
use crate::udp::UdpTransport;

/// 客户端事件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientEvent {
    /// 握手完成，拿到 session/player id。
    Connected {
        /// 会话 ID。
        session_id: SessionId,
        /// 玩家 ID。
        player_id: PlayerId,
    },
    /// 断开（主动或服务器通知）。
    Disconnected {
        /// 原因。
        reason: DisconnectReason,
    },
    /// 心跳/握手超时。
    TimedOut,
    /// 业务消息（包含 RoomJoined/PlayerJoined/Chat/Snapshot/...）。
    Message(Message),
}

/// UDP 网络客户端。
#[derive(Debug)]
pub struct NetClient {
    transport: UdpTransport,
    sessions: SessionManager,
    server: PeerId,
    config: NetConfig,
    session_id: SessionId,
    player_id: PlayerId,
    room: Option<RoomId>,
}

impl NetClient {
    /// 连接服务器并发送 ClientHello。
    pub fn connect(
        server_addr: SocketAddr,
        client_name: &str,
        config: &NetConfig,
    ) -> Result<Self, TransportError> {
        let bind_addr: SocketAddr = if server_addr.is_ipv6() {
            (Ipv6Addr::UNSPECIFIED, 0).into()
        } else {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        };
        let transport = UdpTransport::bind_with_limit(bind_addr, config.max_message_bytes)?;
        let mut sessions = SessionManager::new_client(config.session_config());
        let server = PeerId(server_addr);
        sessions
            .connect(server, client_name, Instant::now())
            .map_err(|error| TransportError::Io(error.to_string()))?;
        let mut client = Self {
            transport,
            sessions,
            server,
            config: config.clone(),
            session_id: SessionId::NONE,
            player_id: PlayerId::NONE,
            room: None,
        };
        client.flush_outbox();
        Ok(client)
    }

    /// 本地绑定地址。
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.transport.local_addr()
    }

    /// 服务器地址。
    pub fn server_addr(&self) -> SocketAddr {
        self.server.addr()
    }

    /// 配置。
    pub fn config(&self) -> &NetConfig {
        &self.config
    }

    /// 是否已握手。
    pub fn is_connected(&self) -> bool {
        self.sessions
            .session(self.server)
            .map(|session| session.is_connected())
            .unwrap_or(false)
    }

    /// 会话 ID（握手前为 0）。
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// 玩家 ID（握手前为 0）。
    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    /// 当前所在房间（由服务器消息驱动）。
    pub fn room(&self) -> Option<RoomId> {
        self.room
    }

    /// 会话统计。
    pub fn session_stats(&self) -> SessionStats {
        self.sessions
            .session(self.server)
            .map(|session| session.stats())
            .unwrap_or_default()
    }

    /// 非阻塞收包并处理，返回本次事件。
    pub fn poll(&mut self, now: Instant, max_datagrams: usize) -> Vec<ClientEvent> {
        let mut buffer = [0u8; 2048];
        let mut received = 0usize;
        while received < max_datagrams.max(1) {
            match self.transport.recv_from(&mut buffer) {
                Ok(Some((size, _peer))) => {
                    received += 1;
                    self.sessions
                        .handle_datagram(self.server, &buffer[..size], now);
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
        let mut events = Vec::new();
        self.drain_events(&mut events);
        self.flush_outbox();
        events
    }

    /// 推进时间（心跳/超时），返回本次事件。
    pub fn tick(&mut self, now: Instant) -> Vec<ClientEvent> {
        self.sessions.tick(now);
        let mut events = Vec::new();
        self.drain_events(&mut events);
        self.flush_outbox();
        events
    }

    /// 加入房间（结果通过 `RoomJoined`/`JoinRejected` 消息返回）。
    pub fn join_room(&mut self, room: RoomId) -> Result<(), SessionError> {
        self.send(Message::JoinRoom { room })
    }

    /// 离开当前房间。
    pub fn leave_room(&mut self) -> Result<(), SessionError> {
        self.send(Message::LeaveRoom)
    }

    /// 在当前房间发送聊天。
    pub fn send_chat(&mut self, text: impl Into<String>) -> Result<(), SessionError> {
        let room = self.room.ok_or(SessionError::NoRoom)?;
        self.send(Message::Chat {
            room,
            player: self.player_id,
            text: text.into(),
        })
    }

    /// 发送状态快照（服务端/客户端均可使用；本 crate 不解释 payload）。
    pub fn send_snapshot(
        &mut self,
        tick: u64,
        seq: u32,
        payload: Vec<u8>,
    ) -> Result<(), SessionError> {
        self.send(Message::Snapshot { tick, seq, payload })
    }

    /// 发送 RPC（`PlayerId::NONE`=服务器）。
    pub fn send_rpc(
        &mut self,
        target: PlayerId,
        method: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<(), SessionError> {
        self.send(Message::Rpc {
            target,
            method: method.into(),
            payload,
        })
    }

    /// 发送任意业务消息（要求已连接）。
    pub fn send(&mut self, message: Message) -> Result<(), SessionError> {
        self.sessions.send(self.server, message)?;
        self.flush_outbox();
        Ok(())
    }

    /// 主动断开（尽力发送 Disconnect 后本地移除会话）。
    pub fn disconnect(&mut self, reason: DisconnectReason) -> bool {
        let disconnected = self.sessions.disconnect(self.server, reason);
        self.room = None;
        self.flush_outbox();
        disconnected
    }

    fn drain_events(&mut self, events: &mut Vec<ClientEvent>) {
        for event in self.sessions.take_events() {
            match event {
                SessionEvent::Connected {
                    session_id,
                    player_id,
                    ..
                } => {
                    self.session_id = session_id;
                    self.player_id = player_id;
                    events.push(ClientEvent::Connected {
                        session_id,
                        player_id,
                    });
                }
                SessionEvent::Message { message, .. } => {
                    match &message {
                        Message::RoomJoined { room, .. } => self.room = Some(*room),
                        // 离开成功或被拒绝后，本地不再认为在房间内。
                        Message::RoomLeft { .. } | Message::JoinRejected { .. } => {
                            self.room = None;
                        }
                        _ => {}
                    }
                    events.push(ClientEvent::Message(message));
                }
                SessionEvent::TimedOut { .. } => {
                    self.room = None;
                    events.push(ClientEvent::TimedOut);
                }
                SessionEvent::Closed { reason, .. } => {
                    self.room = None;
                    events.push(ClientEvent::Disconnected { reason });
                }
            }
        }
    }

    fn flush_outbox(&mut self) {
        for datagram in self.sessions.take_outbound() {
            let _ = self
                .transport
                .send_to(datagram.peer.addr(), &datagram.bytes);
        }
    }
}
