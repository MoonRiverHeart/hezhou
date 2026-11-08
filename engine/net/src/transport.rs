//! 传输抽象：数据报 [`Transport`] trait 与统计。
//!
//! UDP 实现于 [`crate::udp::UdpTransport`]；TCP 是字节流，使用
//! [`crate::tcp::TcpConnection`] + [`crate::framing`] 的帧接口，不实现本 trait。
//!
//! `recv_from` 约定：非阻塞 socket 暂时无数据返回 `Ok(None)`，而不是错误。

use std::net::SocketAddr;

use crate::error::TransportError;

/// 对端地址（本 crate 直接使用 `SocketAddr`，不做 NAT 穿透）。
pub type PeerAddr = SocketAddr;

/// 默认单数据报上限（UDP 不分片；与消息层 1200 字节保持一致）。
pub const DEFAULT_MAX_DATAGRAM_BYTES: usize = 1200;

/// 传输统计。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransportStats {
    /// 成功发送的数据报数量。
    pub datagrams_sent: u64,
    /// 成功接收的数据报数量。
    pub datagrams_received: u64,
    /// 发送字节数。
    pub bytes_sent: u64,
    /// 接收字节数。
    pub bytes_received: u64,
    /// 发送失败次数。
    pub send_errors: u64,
    /// 接收失败次数（不含 WouldBlock）。
    pub recv_errors: u64,
}

impl TransportStats {
    pub(crate) fn record_sent(&mut self, bytes: usize) {
        self.datagrams_sent = self.datagrams_sent.saturating_add(1);
        self.bytes_sent = self.bytes_sent.saturating_add(bytes as u64);
    }

    pub(crate) fn record_received(&mut self, bytes: usize) {
        self.datagrams_received = self.datagrams_received.saturating_add(1);
        self.bytes_received = self.bytes_received.saturating_add(bytes as u64);
    }
}

/// 数据报传输抽象。
pub trait Transport {
    /// 本地绑定地址。
    fn local_addr(&self) -> Result<PeerAddr, TransportError>;
    /// 发送一个数据报；超过 [`Transport::max_datagram_size`] 返回
    /// [`TransportError::MessageTooLarge`]。
    fn send_to(&mut self, peer: PeerAddr, payload: &[u8]) -> Result<usize, TransportError>;
    /// 非阻塞接收；无数据返回 `Ok(None)`。
    fn recv_from(&mut self, buffer: &mut [u8])
        -> Result<Option<(usize, PeerAddr)>, TransportError>;
    /// 关闭传输；之后 send/recv 返回 [`TransportError::Closed`]。
    fn close(&mut self);
    /// 统计快照。
    fn stats(&self) -> TransportStats;
    /// 单数据报发送上限。
    fn max_datagram_size(&self) -> usize;
    /// 发送别名（与 [`Transport::send_to`] 等价，便于按任务接口命名使用）。
    fn send(&mut self, peer: PeerAddr, payload: &[u8]) -> Result<usize, TransportError> {
        self.send_to(peer, payload)
    }
    /// 接收别名（与 [`Transport::recv_from`] 等价，便于按任务接口命名使用）。
    fn recv(&mut self, buffer: &mut [u8]) -> Result<Option<(usize, PeerAddr)>, TransportError> {
        self.recv_from(buffer)
    }
    /// 配置超时（数据报实现可选；TCP 见 `TcpConnection`）。
    fn set_read_timeout(
        &mut self,
        _timeout: Option<std::time::Duration>,
    ) -> Result<(), TransportError> {
        Err(TransportError::Io("该传输不支持读超时".to_string()))
    }
    /// "连接"到固定对端（UDP connect 语义，可选）。
    fn connect(&mut self, _peer: PeerAddr) -> Result<(), TransportError> {
        Err(TransportError::Io("该传输不支持 connect".to_string()))
    }
    /// 已连接对端（未连接返回 `Ok(None)`）。
    fn peer_addr(&self) -> Result<Option<PeerAddr>, TransportError> {
        Ok(None)
    }
}
