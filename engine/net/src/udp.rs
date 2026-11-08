//! UDP 传输实现（`std::net::UdpSocket`）。
//!
//! - 非阻塞：`bind` 后立即 `set_nonblocking(true)`，`recv_from` 无数据返回 `Ok(None)`；
//! - 单数据报上限默认 [`DEFAULT_MAX_DATAGRAM_BYTES`]（1200），发送超限直接拒绝
//!   （不分片，避免 IP 分片与 MTU 问题）；
//! - 可选 `connect(peer)` 固定对端：之后 `send_to` 只允许该对端（由 OS 过滤）；
//! - 统计所有成功发送/接收的数据报与字节数。

use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

use crate::error::TransportError;
use crate::transport::{PeerAddr, Transport, TransportStats, DEFAULT_MAX_DATAGRAM_BYTES};

/// 非阻塞 UDP 传输。
#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
    max_datagram_size: usize,
    stats: TransportStats,
    closed: bool,
}

impl UdpTransport {
    /// 绑定地址（默认 1200 字节单包上限）。
    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self, TransportError> {
        Self::bind_with_limit(addr, DEFAULT_MAX_DATAGRAM_BYTES)
    }

    /// 绑定地址并指定单数据报上限。
    pub fn bind_with_limit(
        addr: impl ToSocketAddrs,
        max_datagram_size: usize,
    ) -> Result<Self, TransportError> {
        let max_datagram_size = max_datagram_size.clamp(1, u16::MAX as usize);
        let socket = UdpSocket::bind(addr).map_err(TransportError::io)?;
        socket.set_nonblocking(true).map_err(TransportError::io)?;
        Ok(Self {
            socket,
            max_datagram_size,
            stats: TransportStats::default(),
            closed: false,
        })
    }

    /// 绑定 `0.0.0.0:0`/`[::]:0` 之类的任意端口（按地址族）。
    pub fn bind_any(ipv4: bool) -> Result<Self, TransportError> {
        if ipv4 {
            Self::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
        } else {
            Self::bind((std::net::Ipv6Addr::UNSPECIFIED, 0))
        }
    }

    /// 配置读超时（非阻塞模式下一般不需要；`Some` 时 `recv_from` 超时返回 `Ok(None)`）。
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.socket
            .set_read_timeout(timeout)
            .map_err(TransportError::io)
    }

    /// 配置写超时。
    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.socket
            .set_write_timeout(timeout)
            .map_err(TransportError::io)
    }

    /// 是否已关闭。
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// 底层 socket 的本地地址。
    pub fn socket_local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.socket.local_addr().map_err(TransportError::io)
    }
}

impl Transport for UdpTransport {
    fn local_addr(&self) -> Result<PeerAddr, TransportError> {
        self.socket.local_addr().map_err(TransportError::io)
    }

    fn send_to(&mut self, peer: PeerAddr, payload: &[u8]) -> Result<usize, TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        if payload.len() > self.max_datagram_size {
            self.stats.send_errors = self.stats.send_errors.saturating_add(1);
            return Err(TransportError::MessageTooLarge {
                size: payload.len(),
                max: self.max_datagram_size,
            });
        }
        match self.socket.send_to(payload, peer) {
            Ok(sent) => {
                self.stats.record_sent(sent);
                Ok(sent)
            }
            Err(error) => {
                self.stats.send_errors = self.stats.send_errors.saturating_add(1);
                Err(TransportError::io(error))
            }
        }
    }

    fn recv_from(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<Option<(usize, PeerAddr)>, TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        match self.socket.recv_from(buffer) {
            Ok((size, peer)) => {
                self.stats.record_received(size);
                Ok(Some((size, peer)))
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(error) => {
                self.stats.recv_errors = self.stats.recv_errors.saturating_add(1);
                Err(TransportError::io(error))
            }
        }
    }

    fn close(&mut self) {
        self.closed = true;
    }

    fn stats(&self) -> TransportStats {
        self.stats
    }

    fn max_datagram_size(&self) -> usize {
        self.max_datagram_size
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.socket
            .set_read_timeout(timeout)
            .map_err(TransportError::io)
    }

    fn connect(&mut self, peer: PeerAddr) -> Result<(), TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        self.socket.connect(peer).map_err(TransportError::io)
    }

    fn peer_addr(&self) -> Result<Option<PeerAddr>, TransportError> {
        match self.socket.peer_addr() {
            Ok(peer) => Ok(Some(peer)),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_reports_local_addr_and_stats() {
        let mut transport = UdpTransport::bind("127.0.0.1:0").unwrap();
        let addr = transport.local_addr().unwrap();
        assert_ne!(addr.port(), 0);
        assert_eq!(transport.stats(), TransportStats::default());
        assert_eq!(transport.max_datagram_size(), DEFAULT_MAX_DATAGRAM_BYTES);
        transport.close();
        assert!(matches!(
            transport.send_to(addr, b"x"),
            Err(TransportError::Closed)
        ));
    }

    #[test]
    fn udp_loopback_send_receive() {
        let mut server = UdpTransport::bind("127.0.0.1:0").unwrap();
        let server_addr = server.local_addr().unwrap();
        let mut client = UdpTransport::bind("127.0.0.1:0").unwrap();
        let client_addr = client.local_addr().unwrap();

        let sent = client.send_to(server_addr, b"ping").unwrap();
        assert_eq!(sent, 4);

        let mut buffer = [0u8; 64];
        let mut received = None;
        for _ in 0..100 {
            if let Some(value) = server.recv_from(&mut buffer).unwrap() {
                received = Some(value);
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let (size, peer) = received.expect("服务器应收到 UDP 数据报");
        assert_eq!(&buffer[..size], b"ping");
        assert_eq!(peer, client_addr);
        assert_eq!(client.stats().datagrams_sent, 1);
        assert_eq!(server.stats().datagrams_received, 1);
    }

    #[test]
    fn oversized_datagram_is_rejected() {
        let mut transport = UdpTransport::bind_with_limit("127.0.0.1:0", 8).unwrap();
        let peer = transport.local_addr().unwrap();
        let payload = [0u8; 9];
        assert!(matches!(
            transport.send_to(peer, &payload),
            Err(TransportError::MessageTooLarge { size: 9, max: 8 })
        ));
        assert_eq!(transport.stats().send_errors, 1);
    }

    #[test]
    fn connect_restricts_peer() {
        let server = UdpTransport::bind("127.0.0.1:0").unwrap();
        let server_addr = server.local_addr().unwrap();
        let mut client = UdpTransport::bind("127.0.0.1:0").unwrap();
        client.connect(server_addr).unwrap();
        assert_eq!(client.peer_addr().unwrap(), Some(server_addr));
        client.send_to(server_addr, b"x").unwrap();
    }
}
