//! UDP 服务器：会话管理 + 房间 + 消息路由。
//!
//! `NetServer` 把 [`crate::udp::UdpTransport`]、[`crate::session::SessionManager`] 与
//! [`crate::room::RoomManager`] 组合成一个可直接用的服务器：
//!
//! 1. `poll(now, max_datagrams)`：非阻塞收包 → 会话/握手/心跳 → 房间路由 → 返回事件；
//! 2. `tick(now)`：推进会话（发送心跳、检测超时）→ 房间清理 → 返回事件；
//! 3. 房间事件自动产生消息：`RoomJoined`/`PlayerJoined`/`PlayerLeft`/`JackRejected`/`Chat`；
//! 4. `Snapshot`/`Rpc` 等业务消息：`Rpc` 按 target 定向转发（`PlayerId::NONE`=服务器），
//!    其余通过 [`ServerEvent::Message`] 交给上层同步层。
//!
//! 这是**原型级**服务器：单线程、无权威帧循环、无状态同步/预测/回滚，仅验证传输与会话链路。

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::Instant;

use crate::config::NetConfig;
use crate::error::TransportError;
use crate::ids::{PeerId, PlayerId, RoomId, SessionId};
use crate::message::{DisconnectReason, Message, RoomRejectReason};
use crate::room::RoomManager;
use crate::session::{SessionEvent, SessionManager};
use crate::transport::Transport;
use crate::udp::UdpTransport;

/// 服务器事件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerEvent {
    /// 新客户端握手完成。
    ClientConnected {
        /// 对端。
        peer: PeerId,
        /// 会话 ID。
        session_id: SessionId,
        /// 玩家 ID。
        player_id: PlayerId,
        /// 客户端名。
        name: String,
    },
    /// 客户端断开（主动/超时）。
    ClientDisconnected {
        /// 对端。
        peer: PeerId,
        /// 会话 ID。
        session_id: SessionId,
        /// 玩家 ID。
        player_id: PlayerId,
        /// 原因。
        reason: DisconnectReason,
        /// 断开前所在房间。
        room: Option<RoomId>,
    },
    /// 业务消息（Snapshot/Rpc 到服务器/其他）。
    Message {
        /// 对端。
        peer: PeerId,
        /// 玩家 ID。
        player_id: PlayerId,
        /// 消息。
        message: Message,
    },
    /// 玩家加入房间成功。
    RoomJoined {
        /// 对端。
        peer: PeerId,
        /// 玩家。
        player: PlayerId,
        /// 房间。
        room: RoomId,
        /// 加入前的现有成员。
        existing_members: Vec<PlayerId>,
    },
    /// 玩家离开房间。
    RoomLeft {
        /// 对端。
        peer: PeerId,
        /// 玩家。
        player: PlayerId,
        /// 房间。
        room: RoomId,
    },
    /// 加入房间被拒绝。
    JoinRejected {
        /// 对端。
        peer: PeerId,
        /// 玩家。
        player: PlayerId,
        /// 房间。
        room: RoomId,
        /// 原因。
        reason: RoomRejectReason,
    },
    /// 房间聊天（已广播给房间成员）。
    Chat {
        /// 房间。
        room: RoomId,
        /// 发送者。
        from: PlayerId,
        /// 文本。
        text: String,
    },
}

/// UDP 网络服务器。
#[derive(Debug)]
pub struct NetServer {
    transport: UdpTransport,
    sessions: SessionManager,
    rooms: RoomManager,
    config: NetConfig,
    player_names: BTreeMap<PlayerId, String>,
    player_peers: BTreeMap<PlayerId, PeerId>,
    session_tick: u64,
}

impl NetServer {
    /// 按配置绑定并创建服务器（`config.bind_addr` 通常为 `"127.0.0.1:0"` 便于测试）。
    pub fn bind(config: &NetConfig) -> Result<Self, TransportError> {
        let transport = UdpTransport::bind_with_limit(&config.bind_addr, config.max_message_bytes)?;
        let sessions = SessionManager::new_server(config.session_config())
            .with_max_sessions(config.max_clients);
        Ok(Self {
            transport,
            sessions,
            rooms: RoomManager::new(config.room_config()),
            config: config.clone(),
            player_names: BTreeMap::new(),
            player_peers: BTreeMap::new(),
            session_tick: 0,
        })
    }

    /// 本地监听地址。
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.transport.local_addr()
    }

    /// 配置。
    pub fn config(&self) -> &NetConfig {
        &self.config
    }

    /// 当前会话数。
    pub fn session_count(&self) -> usize {
        self.sessions.session_count()
    }

    /// 已连接玩家数。
    pub fn player_count(&self) -> usize {
        self.player_peers.len()
    }

    /// 房间管理器（只读）。
    pub fn room_manager(&self) -> &RoomManager {
        &self.rooms
    }

    /// 玩家所在房间。
    pub fn room_of(&self, player: PlayerId) -> Option<RoomId> {
        self.rooms.room_of(player)
    }

    /// 玩家会话统计。
    pub fn session_stats_for(&self, player: PlayerId) -> Option<crate::session::SessionStats> {
        let peer = self.player_peers.get(&player)?;
        self.sessions.session(*peer).map(|session| session.stats())
    }

    /// 玩家对端地址。
    pub fn peer_of(&self, player: PlayerId) -> Option<PeerId> {
        self.player_peers.get(&player).copied()
    }

    /// 收到并处理最多 `max_datagrams` 个数据报，返回本次产生的事件。
    pub fn poll(&mut self, now: Instant, max_datagrams: usize) -> Vec<ServerEvent> {
        let mut buffer = [0u8; 2048];
        let mut received = 0usize;
        while received < max_datagrams.max(1) {
            match self.transport.recv_from(&mut buffer) {
                Ok(Some((size, peer))) => {
                    received += 1;
                    self.sessions
                        .handle_datagram(PeerId(peer), &buffer[..size], now);
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
        let mut events = Vec::new();
        self.drain_session_events(&mut events);
        self.flush_outbox();
        events
    }

    /// 推进时间（心跳/超时），返回本次产生的事件。
    pub fn tick(&mut self, now: Instant) -> Vec<ServerEvent> {
        self.session_tick = self.session_tick.wrapping_add(1);
        self.sessions.tick(now);
        let mut events = Vec::new();
        self.drain_session_events(&mut events);
        self.flush_outbox();
        events
    }

    /// 向房间广播（房间不存在返回 0）。
    pub fn broadcast(&mut self, room: RoomId, message: Message) -> usize {
        let targets = self.rooms.broadcast_targets(room);
        self.send_to_players(&targets, message)
    }

    /// 定向发送给玩家（未连接返回 false）。
    pub fn send_to_player(&mut self, player: PlayerId, message: Message) -> bool {
        let Some(peer) = self.player_peers.get(&player).copied() else {
            return false;
        };
        self.sessions.send(peer, message).is_ok()
    }

    /// 处理内部会话事件并路由消息/房间。
    fn drain_session_events(&mut self, events: &mut Vec<ServerEvent>) {
        for event in self.sessions.take_events() {
            match event {
                SessionEvent::Connected {
                    peer,
                    session_id,
                    player_id,
                    remote_name,
                } => {
                    self.player_names.insert(player_id, remote_name.clone());
                    self.player_peers.insert(player_id, peer);
                    events.push(ServerEvent::ClientConnected {
                        peer,
                        session_id,
                        player_id,
                        name: remote_name,
                    });
                }
                SessionEvent::Message {
                    peer,
                    session_id,
                    message,
                } => {
                    let player = PlayerId::new(session_id.raw());
                    self.handle_message(peer, player, message, events);
                }
                SessionEvent::TimedOut {
                    peer,
                    session_id,
                    player_id,
                } => {
                    self.remove_player(
                        peer,
                        session_id,
                        player_id,
                        DisconnectReason::Timeout,
                        events,
                    );
                }
                SessionEvent::Closed {
                    peer,
                    session_id,
                    player_id,
                    reason,
                } => {
                    self.remove_player(peer, session_id, player_id, reason, events);
                }
            }
        }
    }

    fn handle_message(
        &mut self,
        peer: PeerId,
        player: PlayerId,
        message: Message,
        events: &mut Vec<ServerEvent>,
    ) {
        match message {
            Message::JoinRoom { room } => {
                let name = self.player_names.get(&player).cloned().unwrap_or_default();
                match self.rooms.join(room, player, name.clone()) {
                    Ok(outcome) => {
                        // 给新玩家回 RoomJoined（列出加入前成员）。
                        let _ = self.sessions.send(
                            peer,
                            Message::RoomJoined {
                                room,
                                members: outcome.existing_members.clone(),
                            },
                        );
                        // 给已有成员广播 PlayerJoined。
                        let joined = Message::PlayerJoined {
                            player,
                            room,
                            name: name.clone(),
                        };
                        self.send_to_players(&outcome.existing_members, joined);
                        events.push(ServerEvent::RoomJoined {
                            peer,
                            player,
                            room,
                            existing_members: outcome.existing_members,
                        });
                    }
                    Err(error) => {
                        let reason = match error {
                            crate::error::RoomError::Full { .. } => RoomRejectReason::Full,
                            crate::error::RoomError::AlreadyInRoom { .. } => {
                                RoomRejectReason::AlreadyInRoom
                            }
                            crate::error::RoomError::NotFound(_) => RoomRejectReason::NotFound,
                            crate::error::RoomError::NotInRoom => RoomRejectReason::NotFound,
                        };
                        let _ = self
                            .sessions
                            .send(peer, Message::JoinRejected { room, reason });
                        events.push(ServerEvent::JoinRejected {
                            peer,
                            player,
                            room,
                            reason,
                        });
                    }
                }
            }
            Message::LeaveRoom => {
                if let Some(leave) = self.rooms.leave(player) {
                    let _ = self
                        .sessions
                        .send(peer, Message::RoomLeft { room: leave.room });
                    let left = Message::PlayerLeft {
                        player,
                        room: leave.room,
                    };
                    self.send_to_players(&leave.remaining, left);
                    events.push(ServerEvent::RoomLeft {
                        peer,
                        player,
                        room: leave.room,
                    });
                }
            }
            Message::Chat {
                room,
                player: sender,
                text,
            } => {
                // 校验发送者/房间一致，防止伪造。
                if sender == player && self.rooms.room_of(player) == Some(room) {
                    let broadcast = Message::Chat {
                        room,
                        player,
                        text: text.clone(),
                    };
                    let targets = self.rooms.broadcast_targets(room);
                    self.send_to_players(&targets, broadcast);
                    events.push(ServerEvent::Chat {
                        room,
                        from: player,
                        text,
                    });
                }
            }
            Message::Rpc {
                target,
                method,
                payload,
            } => {
                if target.is_none() {
                    events.push(ServerEvent::Message {
                        peer,
                        player_id: player,
                        message: Message::Rpc {
                            target,
                            method,
                            payload,
                        },
                    });
                } else if let Some(target_peer) = self.player_peers.get(&target).copied() {
                    let _ = self.sessions.send(
                        target_peer,
                        Message::Rpc {
                            target,
                            method,
                            payload,
                        },
                    );
                } else {
                    events.push(ServerEvent::Message {
                        peer,
                        player_id: player,
                        message: Message::Rpc {
                            target,
                            method,
                            payload,
                        },
                    });
                }
            }
            message => {
                events.push(ServerEvent::Message {
                    peer,
                    player_id: player,
                    message,
                });
            }
        }
    }

    fn remove_player(
        &mut self,
        peer: PeerId,
        session_id: SessionId,
        player_id: PlayerId,
        reason: DisconnectReason,
        events: &mut Vec<ServerEvent>,
    ) {
        let room = self.rooms.room_of(player_id);
        if let Some(leave) = self.rooms.leave(player_id) {
            let left = Message::PlayerLeft {
                player: player_id,
                room: leave.room,
            };
            self.send_to_players(&leave.remaining, left);
        }
        self.player_names.remove(&player_id);
        self.player_peers.remove(&player_id);
        events.push(ServerEvent::ClientDisconnected {
            peer,
            session_id,
            player_id,
            reason,
            room,
        });
    }

    fn send_to_players(&mut self, players: &[PlayerId], message: Message) -> usize {
        let mut sent = 0usize;
        for player in players {
            if let Some(peer) = self.player_peers.get(player).copied() {
                if self.sessions.send(peer, message.clone()).is_ok() {
                    sent += 1;
                }
            }
        }
        sent
    }

    fn flush_outbox(&mut self) {
        for datagram in self.sessions.take_outbound() {
            let _ = self
                .transport
                .send_to(datagram.peer.addr(), &datagram.bytes);
        }
    }
}
