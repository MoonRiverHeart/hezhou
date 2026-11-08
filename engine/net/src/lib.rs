//! `hezhou-net` — Hezhou 0.4 网络传输层与会话（std-only）。
//!
//! 本 crate 是后续多人/联机能力的第一层：只提供**传输、消息、会话、房间**，
//! 不做状态同步、预测、回滚、加密或具体玩法。零第三方依赖，纯 `std::net`。
//!
//! | 能力 | 入口 |
//! |---|---|
//! | 数据报传输抽象 | [`Transport`] trait、[`TransportStats`] |
//! | UDP | [`UdpTransport`]（非阻塞、最大数据报限制、收发统计） |
//! | TCP + 长度前缀 framing | [`TcpListener`]、[`TcpConnection`]、[`FrameCodec`]、[`FrameDecoder`] |
//! | 消息协议 | [`Message`]（ClientHello/ServerWelcome/Snapshot/Rpc/Heartbeat/Disconnect/Room 等）+ 自定义二进制 |
//! | 会话 | [`SessionManager`]、[`Session`]、[`SessionEvent`]：握手、序列号、心跳、超时断开 |
//! | 房间 | [`RoomManager`]、[`Room`]：加入/离开/广播/定向发送 |
//! | 快照同步 | [`EntityState`]、[`WorldSnapshot`]、[`SnapshotPacket`]：全量/delta 二进制；[`ReplicationServer`]、[`ReplicationClient`]、[`SnapshotBuffer`] |
//! | RPC | [`RpcRegistry`]、[`RpcCall`]、[`RpcResult`]：方法注册/调用 + 二进制结果 |
//! | 开箱 server/client | [`NetServer`]、[`NetClient`]、[`NetConfig`] |
//!
//! ## 快速开始（UDP loopback）
//!
//! ```no_run
//! use hezhou_net::{NetClient, NetConfig, NetServer};
//! use std::time::Instant;
//!
//! let server = NetServer::bind(&NetConfig::default())?;
//! let server_addr = server.local_addr()?;
//! let mut client = NetClient::connect(server_addr, "player-1", &NetConfig::default())?;
//! let now = Instant::now();
//! client.tick(now);
//! # Ok::<(), hezhou_net::TransportError>(())
//! ```
//!
//! ## 能力边界（0.4 第一批）
//!
//! - **仅传输/会话**：无状态同步、插值、预测、回滚、权威服务器逻辑；
//! - **不安全**：无加密/认证/anti-cheat，只适合本机/局域网/受信网络原型；
//! - **UDP 不可靠**：有序列号与重复/乱序计数，但不重传；可靠性用 TCP 或后续可靠层；
//! - **TCP** 只提供长度前缀 framing 与非阻塞帧收发，不做连接池/重连策略；
//! - **无 NAT 穿透/打洞/中继**；地址为直连 `SocketAddr`；
//! - 消息上限默认 1200 字节（UDP 单包保护），快照/RPC 需自行控制 payload 大小；
//! - 跨平台：Windows/Linux 均使用 `std::net`，行为一致；差异主要在防火墙/临时端口
//!   与 UDP 缓冲区大小（见 `README.md`）。
//!
//! 详细协议、配置、测试与限制见 crate 根目录 `README.md`。

#![forbid(unsafe_code)]

pub mod client;
pub mod config;
pub mod error;
pub mod framing;
pub mod ids;
pub mod message;
pub mod replication;
pub mod room;
pub mod rpc;
pub mod server;
pub mod session;
pub mod snapshot;
pub mod tcp;
pub mod transport;
pub mod udp;

pub use client::{ClientEvent, NetClient};
pub use config::NetConfig;
pub use error::{
    FrameError, MessageError, ReplicationError, RoomError, RpcError, SessionError, SnapshotError,
    TransportError,
};
pub use framing::{FrameCodec, FrameDecoder, DEFAULT_MAX_FRAME_BYTES, FRAME_HEADER_SIZE};
pub use ids::{PeerId, PlayerId, RoomId, SessionId};
pub use message::{
    DisconnectReason, Message, MessageKind, RoomRejectReason, MAX_MESSAGE_BYTES, MESSAGE_VERSION,
    PROTOCOL_VERSION,
};
pub use replication::{
    ClientReplicationStats, ClientSnapshot, ReceiveOutcome, ReplicationClient,
    ReplicationClientStats, ReplicationConfig, ReplicationServer,
    DEFAULT_FULL_SNAPSHOT_INTERVAL_TICKS, DEFAULT_REPLICATION_HISTORY,
    DEFAULT_REPLICATION_MAX_ENTITIES, DEFAULT_REPLICATION_MAX_SNAPSHOT_BYTES,
};
pub use room::{JoinError, JoinOutcome, LeaveOutcome, Room, RoomConfig, RoomManager, RoomMember};
pub use rpc::{
    RpcCall, RpcRegistry, RpcResult, MAX_RPC_BYTES, MAX_RPC_ERROR_BYTES, MAX_RPC_METHOD_BYTES,
    MAX_RPC_PAYLOAD_BYTES, RPC_VERSION,
};
pub use server::{NetServer, ServerEvent};
pub use session::{
    OutboundDatagram, Packet, Session, SessionConfig, SessionEvent, SessionManager, SessionRole,
    SessionState, SessionStats,
};
pub use snapshot::{
    EntityState, SnapshotBuffer, SnapshotPacket, WorldSnapshot, DEFAULT_SNAPSHOT_BUFFER_CAPACITY,
    MAX_SNAPSHOT_BYTES, MAX_SNAPSHOT_ENTITIES, SNAPSHOT_ENTITY_BYTES, SNAPSHOT_HEADER_BYTES,
    SNAPSHOT_LENGTH_PREFIX_BYTES, SNAPSHOT_VERSION,
};
pub use tcp::{TcpConnection, TcpListener, TcpStats};
pub use transport::{PeerAddr, Transport, TransportStats, DEFAULT_MAX_DATAGRAM_BYTES};
pub use udp::UdpTransport;
