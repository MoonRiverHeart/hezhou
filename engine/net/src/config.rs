//! `NetConfig`：网络层的统一配置。
//!
//! 覆盖任务要求的 bind 地址、最大客户端数、超时、最大消息大小、tick rate；
//! 另外提供心跳/握手超时与房间容量默认值。

use std::time::Duration;

use crate::ids::RoomId;
use crate::room::RoomConfig;
use crate::session::SessionConfig;
use crate::transport::DEFAULT_MAX_DATAGRAM_BYTES;

/// 默认 bind 地址（服务器监听所有 IPv4 接口，随机端口由调用方处理）。
pub const DEFAULT_BIND_ADDR: &str = "0.0.0.0:0";
/// 默认最大客户端数。
pub const DEFAULT_MAX_CLIENTS: usize = 32;
/// 默认会话超时（秒）。
pub const DEFAULT_TIMEOUT_SECS: u64 = 10;
/// 默认心跳间隔（秒）。
pub const DEFAULT_HEARTBEAT_SECS: u64 = 2;
/// 默认握手超时（秒）。
pub const DEFAULT_HANDSHAKE_TIMEOUT_SECS: u64 = 5;
/// 默认 tick rate。
pub const DEFAULT_TICK_RATE: u16 = 60;
/// 默认房间容量。
pub const DEFAULT_ROOM_CAPACITY: usize = 16;

/// 网络配置。
#[derive(Clone, Debug, PartialEq)]
pub struct NetConfig {
    /// 服务器 bind 地址（`"127.0.0.1:0"` = 本机随机端口，便于测试）。
    pub bind_addr: String,
    /// 最大客户端（会话）数。
    pub max_clients: usize,
    /// 会话超时（超过即断开）。
    pub timeout: Duration,
    /// 心跳间隔。
    pub heartbeat_interval: Duration,
    /// 握手超时。
    pub handshake_timeout: Duration,
    /// 单条消息最大字节数（必须 ≤ 数据报/帧上限；默认 1200）。
    pub max_message_bytes: usize,
    /// 服务器 tick rate（写入 ServerWelcome）。
    pub tick_rate: u16,
    /// 单房间人数上限。
    pub room_capacity: usize,
    /// 默认房间 ID（客户端未显式 join 时可用；本 crate 不自动加入房间）。
    pub default_room: RoomId,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            bind_addr: DEFAULT_BIND_ADDR.to_string(),
            max_clients: DEFAULT_MAX_CLIENTS,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            heartbeat_interval: Duration::from_secs(DEFAULT_HEARTBEAT_SECS),
            handshake_timeout: Duration::from_secs(DEFAULT_HANDSHAKE_TIMEOUT_SECS),
            max_message_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
            tick_rate: DEFAULT_TICK_RATE,
            room_capacity: DEFAULT_ROOM_CAPACITY,
            default_room: RoomId::new(1),
        }
    }
}

impl NetConfig {
    /// 绑定到 `127.0.0.1:0` 的测试配置（随机端口）。
    pub fn loopback() -> Self {
        Self {
            bind_addr: "127.0.0.1:0".to_string(),
            ..Self::default()
        }
    }

    /// 设置 bind 地址。
    pub fn with_bind_addr(mut self, addr: impl Into<String>) -> Self {
        self.bind_addr = addr.into();
        self
    }

    /// 设置最大消息大小（同时钳制到数据报默认上限，避免 UDP 分片）。
    pub fn with_max_message_bytes(mut self, max: usize) -> Self {
        self.max_message_bytes = max.clamp(1, DEFAULT_MAX_DATAGRAM_BYTES);
        self
    }

    /// 设置超时（秒）。
    pub fn with_timeout_secs(mut self, seconds: u64) -> Self {
        self.timeout = Duration::from_secs(seconds);
        self
    }

    /// 设置心跳间隔（毫秒；测试用小值）。
    pub fn with_heartbeat_millis(mut self, millis: u64) -> Self {
        self.heartbeat_interval = Duration::from_millis(millis);
        self
    }

    /// 生成会话配置。
    pub fn session_config(&self) -> SessionConfig {
        SessionConfig {
            heartbeat_interval: self.heartbeat_interval,
            timeout: self.timeout,
            handshake_timeout: self.handshake_timeout,
            tick_rate: self.tick_rate,
            ..SessionConfig::default()
        }
    }

    /// 生成房间配置。
    pub fn room_config(&self) -> RoomConfig {
        RoomConfig {
            max_players: self.room_capacity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible_and_clamped() {
        let config = NetConfig::default();
        assert_eq!(config.max_message_bytes, DEFAULT_MAX_DATAGRAM_BYTES);
        let clamped = NetConfig::default().with_max_message_bytes(usize::MAX);
        assert_eq!(clamped.max_message_bytes, DEFAULT_MAX_DATAGRAM_BYTES);
        let loopback = NetConfig::loopback();
        assert_eq!(loopback.bind_addr, "127.0.0.1:0");
        assert_eq!(config.session_config().timeout, config.timeout);
        assert_eq!(config.room_config().max_players, config.room_capacity);
    }
}
