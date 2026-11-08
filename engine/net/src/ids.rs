//! 网络 ID：`SessionId` / `PlayerId` / `RoomId` / `PeerId`。
//!
//! 约定：
//! - `0` 保留：`SessionId(0)`/`PlayerId(0)` 表示"服务器"（[`Message::Rpc`] 的 target 用 `PlayerId(0)`）；
//! - `RoomId(0)` 为无效房间（加入房间时必须 >0）；
//! - ID 只在单次进程会话内有效，不持久化。

use std::fmt;
use std::net::SocketAddr;

macro_rules! define_numeric_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);

        impl $name {
            /// 保留值：0 表示服务器/无效。
            pub const NONE: Self = Self(0);

            /// 构造。
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            /// 原始数值。
            pub const fn raw(self) -> u32 {
                self.0
            }

            /// 是否为保留值 0。
            pub const fn is_none(self) -> bool {
                self.0 == 0
            }
        }

        impl From<u32> for $name {
            fn from(raw: u32) -> Self {
                Self(raw)
            }
        }

        impl From<$name> for u32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "#{}"), self.0)
            }
        }
    };
}

define_numeric_id!(SessionId, "会话 ID（服务器分配，`0` 为保留值）。");
define_numeric_id!(
    PlayerId,
    "玩家 ID（当前实现与 SessionId 同值，`0` 表示服务器）。"
);
define_numeric_id!(RoomId, "房间 ID（`0` 为无效房间）。");

/// 网络对端：由 `SocketAddr` 标识（本 crate 不做 NAT 穿透，对端即 IP:port）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub SocketAddr);

impl PeerId {
    /// 构造。
    pub const fn new(addr: SocketAddr) -> Self {
        Self(addr)
    }

    /// 取出地址。
    pub const fn addr(self) -> SocketAddr {
        self.0
    }
}

impl From<SocketAddr> for PeerId {
    fn from(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

impl From<PeerId> for SocketAddr {
    fn from(peer: PeerId) -> Self {
        peer.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
