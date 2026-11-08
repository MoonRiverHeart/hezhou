//! TCP 传输：非阻塞 listener + 长度前缀 framing 的流连接。
//!
//! TCP 是字节流，应用消息通过 [`FrameCodec`]/[`FrameDecoder`] 切分：
//!
//! ```text
//! TcpListener::bind("127.0.0.1:0")?.accept()? -> TcpConnection
//! connection.send_frame(payload)  // [u32 LE len][payload]
//! connection.recv_frames()        // 返回当前可完成的完整 payload 列表（非阻塞）
//! ```
//!
//! - listener/connection 创建后都是非阻塞；`accept`/`recv_frames` 无数据返回 `Ok(None)`/空 Vec；
//! - 对端关闭时 `recv_frames` 返回 [`TransportError::Closed`]；
//! - 帧长度超过 [`TcpConnection::max_frame_bytes`] 会在收发两侧被拒绝（分片保护）。

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener as StdTcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::error::{FrameError, TransportError};
use crate::framing::{FrameCodec, FrameDecoder, DEFAULT_MAX_FRAME_BYTES};
use crate::transport::PeerAddr;

/// TCP 帧收发统计。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TcpStats {
    /// 发送的帧数量。
    pub frames_sent: u64,
    /// 接收的帧数量。
    pub frames_received: u64,
    /// 发送字节数（含 4 字节帧头）。
    pub bytes_sent: u64,
    /// 接收字节数（含帧头）。
    pub bytes_received: u64,
    /// 因 WouldBlock 空转的读次数（非阻塞正常现象）。
    pub would_block_reads: u64,
}

/// 非阻塞 TCP listener。
#[derive(Debug)]
pub struct TcpListener {
    listener: StdTcpListener,
    max_frame_bytes: usize,
}

impl TcpListener {
    /// 绑定并设置非阻塞。
    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self, TransportError> {
        Self::bind_with_limit(addr, DEFAULT_MAX_FRAME_BYTES)
    }

    /// 绑定并指定单帧上限。
    pub fn bind_with_limit(
        addr: impl ToSocketAddrs,
        max_frame_bytes: usize,
    ) -> Result<Self, TransportError> {
        let listener = StdTcpListener::bind(addr).map_err(TransportError::io)?;
        listener.set_nonblocking(true).map_err(TransportError::io)?;
        Ok(Self {
            listener,
            max_frame_bytes: max_frame_bytes.max(1),
        })
    }

    /// 本地地址。
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.listener.local_addr().map_err(TransportError::io)
    }

    /// 单帧上限（后缀连接继承）。
    pub fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    /// 非阻塞 accept：无连接返回 `Ok(None)`。
    pub fn accept(&self) -> Result<Option<TcpConnection>, TransportError> {
        match self.listener.accept() {
            Ok((stream, peer)) => Ok(Some(TcpConnection::from_stream_with_limit(
                stream,
                peer,
                self.max_frame_bytes,
            )?)),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(TransportError::io(error)),
        }
    }
}

/// 非阻塞 TCP 连接（长度前缀 framing）。
#[derive(Debug)]
pub struct TcpConnection {
    stream: TcpStream,
    peer: PeerAddr,
    codec: FrameCodec,
    decoder: FrameDecoder,
    stats: TcpStats,
    closed: bool,
}

impl TcpConnection {
    /// 主动连接（阻塞 connect 成功后切非阻塞）。
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self, TransportError> {
        Self::connect_with_limit(addr, DEFAULT_MAX_FRAME_BYTES)
    }

    /// 主动连接并指定单帧上限。
    pub fn connect_with_limit(
        addr: impl ToSocketAddrs,
        max_frame_bytes: usize,
    ) -> Result<Self, TransportError> {
        let stream = TcpStream::connect(addr).map_err(TransportError::io)?;
        let peer = stream.peer_addr().map_err(TransportError::io)?;
        Self::from_stream_with_limit(stream, peer, max_frame_bytes)
    }

    fn from_stream_with_limit(
        stream: TcpStream,
        peer: PeerAddr,
        max_frame_bytes: usize,
    ) -> Result<Self, TransportError> {
        let max_frame_bytes = max_frame_bytes.max(1);
        stream.set_nonblocking(true).map_err(TransportError::io)?;
        // 小消息低延迟；失败不影响主流程。
        let _ = stream.set_nodelay(true);
        Ok(Self {
            stream,
            peer,
            codec: FrameCodec::new(max_frame_bytes),
            decoder: FrameDecoder::new(max_frame_bytes),
            stats: TcpStats::default(),
            closed: false,
        })
    }

    /// 对端地址。
    pub fn peer_addr(&self) -> PeerAddr {
        self.peer
    }

    /// 本地地址。
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.stream.local_addr().map_err(TransportError::io)
    }

    /// 单帧上限。
    pub fn max_frame_bytes(&self) -> usize {
        self.codec.max_frame_bytes()
    }

    /// 是否已关闭。
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// 统计。
    pub fn stats(&self) -> TcpStats {
        self.stats
    }

    /// 配置底层读超时（非阻塞模式下一般不使用）。
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.stream
            .set_read_timeout(timeout)
            .map_err(TransportError::io)
    }

    /// 关闭连接。
    pub fn close(&mut self) {
        self.closed = true;
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }

    /// 发送一帧（加长度前缀）。返回写入的字节数（含帧头）。
    pub fn send_frame(&mut self, payload: &[u8]) -> Result<usize, TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        let frame = self.codec.encode(payload).map_err(|error| match error {
            FrameError::TooLarge { size, max } => TransportError::MessageTooLarge { size, max },
        })?;
        match self.stream.write_all(&frame) {
            Ok(()) => {
                self.stats.frames_sent = self.stats.frames_sent.saturating_add(1);
                self.stats.bytes_sent = self.stats.bytes_sent.saturating_add(frame.len() as u64);
                Ok(frame.len())
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => Err(TransportError::Io(
                "TCP 写缓冲已满（WouldBlock）".to_string(),
            )),
            Err(error) => Err(TransportError::io(error)),
        }
    }

    /// 非阻塞读取当前可完成的帧（payload），无数据返回空 Vec。
    ///
    /// 对端正常关闭时返回 [`TransportError::Closed`]；帧超限会清空解码缓存并返回错误。
    pub fn recv_frames(&mut self) -> Result<Vec<Vec<u8>>, TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        let mut frames = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            match self.stream.read(&mut buffer) {
                Ok(0) => {
                    self.closed = true;
                    return Err(TransportError::Closed);
                }
                Ok(size) => {
                    self.stats.bytes_received =
                        self.stats.bytes_received.saturating_add(size as u64);
                    match self.decoder.push(&buffer[..size]) {
                        Ok(mut decoded) => {
                            self.stats.frames_received = self
                                .stats
                                .frames_received
                                .saturating_add(decoded.len() as u64);
                            frames.append(&mut decoded);
                        }
                        Err(error) => {
                            self.decoder.reset();
                            return Err(match error {
                                FrameError::TooLarge { size, max } => {
                                    TransportError::MessageTooLarge { size, max }
                                }
                            });
                        }
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                    ) =>
                {
                    self.stats.would_block_reads = self.stats.would_block_reads.saturating_add(1);
                    break;
                }
                Err(error) => return Err(TransportError::io(error)),
            }
        }
        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pump_accept(listener: &TcpListener) -> TcpConnection {
        for _ in 0..500 {
            if let Some(connection) = listener.accept().unwrap() {
                return connection;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("TCP accept 超时");
    }

    fn pump_frames(connection: &mut TcpConnection, expected: usize) -> Vec<Vec<u8>> {
        let mut all = Vec::new();
        for _ in 0..500 {
            all.extend(connection.recv_frames().unwrap_or_default());
            if all.len() >= expected {
                return all;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("TCP recv_frames 超时，已收到 {} 帧", all.len());
    }

    #[test]
    fn tcp_loopback_send_receive_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let mut client = TcpConnection::connect(addr).unwrap();
        let mut server = pump_accept(&listener);

        client.send_frame(b"hello").unwrap();
        let frames = pump_frames(&mut server, 1);
        assert_eq!(frames, vec![b"hello".to_vec()]);

        server.send_frame(b"world").unwrap();
        let frames = pump_frames(&mut client, 1);
        assert_eq!(frames, vec![b"world".to_vec()]);
        assert_eq!(client.stats().frames_sent, 1);
        assert_eq!(server.stats().frames_received, 1);
    }

    #[test]
    fn tcp_fragmented_and_sticky_frames_are_decoded() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let mut raw = TcpStream::connect(addr).unwrap();
        let mut server = pump_accept(&listener);

        let codec = FrameCodec::new(DEFAULT_MAX_FRAME_BYTES);
        let frame = codec.encode(b"fragmented").unwrap();
        raw.write_all(&frame[..2]).unwrap();
        raw.flush().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        // 半包时不应产出帧（recv_frames 返回空）。
        let partial = server.recv_frames().unwrap();
        assert!(partial.is_empty());
        raw.write_all(&frame[2..]).unwrap();
        raw.flush().unwrap();
        let frames = pump_frames(&mut server, 1);
        assert_eq!(frames, vec![b"fragmented".to_vec()]);

        // 粘包：两帧一次写入。
        let mut two = codec.encode(b"a").unwrap();
        two.extend(codec.encode(b"b").unwrap());
        raw.write_all(&two).unwrap();
        raw.flush().unwrap();
        let frames = pump_frames(&mut server, 2);
        assert_eq!(frames, vec![b"a".to_vec(), b"b".to_vec()]);
    }

    #[test]
    fn tcp_oversized_frame_is_rejected_on_send() {
        let listener = TcpListener::bind_with_limit("127.0.0.1:0", 4).unwrap();
        let addr = listener.local_addr().unwrap();
        let mut client = TcpConnection::connect_with_limit(addr, 4).unwrap();
        assert!(matches!(
            client.send_frame(b"12345"),
            Err(TransportError::MessageTooLarge { size: 5, max: 4 })
        ));
    }
}
