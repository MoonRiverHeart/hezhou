# hezhou-net — 0.4 网络传输层与会话（std-only）

> 状态：0.4 网络第一批，**原型级**传输/会话基础，为后续多人同步层提供底座。
> 零第三方依赖，仅 `std::net`；Windows / Linux 行为一致（见 §7）。

## 1. 能力边界（先看这里）

本 crate **只做**：

- 数据报传输抽象（[`Transport`]）+ UDP 实现；
- TCP 非阻塞连接 + `u32 LE` 长度前缀 framing；
- 消息枚举（ClientHello / ServerWelcome / Snapshot / Rpc / Heartbeat / Disconnect + 房间消息）与自定义二进制序列化；
- 会话握手/序列号/心跳/超时；
- 房间注册表（加入/离开/广播目标/定向发送）；
- 开箱 `NetServer`/`NetClient`（UDP loopback 可用）。

**不做**（留给后续版本/上层）：

- 状态同步、插值、预测、回滚、权威帧循环；
- 可靠 UDP（重传/拥塞控制/有序交付）——UDP 只有序列号与重复/乱序计数；
- 加密、认证、防作弊、NAT 穿透、中继、房间匹配；
- 完整玩法逻辑；`Snapshot` 的 payload 由上层解释，本 crate 只保证消息结构。

> ⚠️ 安全：明文传输、无认证。只适合本机/局域网/受信网络原型；公网使用前必须增加
> TLS/DTLS、身份认证与输入校验。

## 2. 模块

| 模块 | 内容 |
|---|---|
| `transport` | `Transport` trait（`local_addr`/`send_to`/`recv_from`/`close`/`stats`/`max_datagram_size`）、`TransportStats` |
| `udp` | `UdpTransport`：非阻塞、单数据报上限（默认 **1200**）、可选 `connect(peer)`、收发统计 |
| `framing` | `FrameCodec`/`FrameDecoder`：`u32 LE 长度 + payload`，分片/粘包安全，超限拒绝 |
| `tcp` | `TcpListener`/`TcpConnection`：非阻塞 accept、帧收发、对端关闭检测、`TcpStats` |
| `message` | `Message` 枚举、`MessageKind`、`DisconnectReason`、`RoomRejectReason`、严格编解码 |
| `session` | `SessionManager`/`Session`/`SessionEvent`：服务器/客户端握手、序列号、心跳、超时 |
| `room` | `RoomManager`/`Room`：容量限制、加入/离开、成员、广播目标 |
| `config` | `NetConfig`：bind addr、max clients、timeout、heartbeat、max message、tick rate、room capacity |
| `server` | `NetServer`：UDP + 会话 + 房间 + 消息路由，返回 `ServerEvent` |
| `client` | `NetClient`：连接、心跳、房间操作、Snapshot/Rpc 发送，返回 `ClientEvent` |

## 3. 快速开始

### 3.1 服务器 + 客户端（loopback）

```no_run
use std::time::Instant;
use hezhou_net::{NetConfig, NetServer, NetClient, RoomId};

let config = NetConfig::loopback(); // bind "127.0.0.1:0"
let mut server = NetServer::bind(&config)?;
let addr = server.local_addr()?;

let mut client = NetClient::connect(addr, "alice", &config)?;
let now = Instant::now();

// 驱动循环：每帧 poll + tick。
for _ in 0..120 {
    let now = Instant::now();
    let server_events = server.poll(now, 64);
    server.tick(now);
    let client_events = client.poll(now, 64);
    client.tick(now);
    // 处理事件：ClientConnected、RoomJoined、Chat、ClientDisconnected...
    let _ = (server_events, client_events);
    std::thread::sleep(std::time::Duration::from_millis(8));
}

client.join_room(RoomId::new(1))?;
client.send_chat("hello")?;
# Ok::<(), hezhou_net::TransportError>(())
```

### 3.2 服务器主循环

```text
loop {
    now = Instant::now();
    events = server.poll(now, 64);   // 收包 + 握手 + 房间路由 + 发响应
    events += server.tick(now);      // 心跳 + 超时 + 房间清理
    for event in events { handle(event) }
    sleep(8ms);
}
```

客户端同理：`poll` 收包、`tick` 发心跳/检测超时；`ClientEvent::Connected` 后
`session_id()/player_id()/room()` 可用。

## 4. 配置与限制

`NetConfig` 默认值：

| 字段 | 默认 | 说明 |
|---|---|---|
| `bind_addr` | `0.0.0.0:0` | 服务器监听地址；测试用 `NetConfig::loopback()` → `127.0.0.1:0` |
| `max_clients` | 32 | 服务器最大会话数（超限回 Disconnect） |
| `timeout` | 10s | 任意有效包刷新 `last_seen`；超时移除会话 |
| `heartbeat_interval` | 2s | 双方发 Heartbeat；收到自动回 pong |
| `handshake_timeout` | 5s | 客户端 Connecting 超时 |
| `max_message_bytes` | 1200 | 单消息上限，同时钳制到 1200（避免 UDP 分片） |
| `tick_rate` | 60 | 写入 ServerWelcome，供上层同步层参考 |
| `room_capacity` | 16 | 单房间人数上限 |
| `default_room` | `RoomId(1)` | 默认房间 ID（本 crate 不自动加入） |

其他限制：

- **UDP 单包 ≤ 1200 字节**（含 Packet 头 4 字节，业务消息 ≤ 1196 字节），不做 IP 分片；
- **TCP 单帧默认 ≤ 64 KiB**（`DEFAULT_MAX_FRAME_BYTES`，可通过 `with_limit` 调整）；
- 单条消息内字符串/字节数组有字段上限：名字 64、RPC 方法 64、聊天 256、房间成员 64 个；
- 序列号每方向从 1 递增；重复/回退包丢弃并计数，跳号包接收并计数（容忍 UDP 乱序）；
- 房间空时自动删除；玩家同一时间只能在一个房间；
- 所有发送/接受函数都是非阻塞语义（`recv_from`/`accept`/`recv_frames` 无数据返回 `None`/空）。

## 5. 协议格式

消息（UDP 数据报 payload / TCP 帧 payload）：

```text
[u8 format_version=1][u16 kind LE][variant payload...]
```

数据报外层再包一层 Packet：

```text
[u32 sequence LE][message...]    # 总长度 ≤ 1200
```

关键消息：

| 消息 | kind | 字段 |
|---|---|---|
| `ClientHello` | 1 | `protocol_version: u16`, `client_name: str` |
| `ServerWelcome` | 2 | `session_id: u32`, `tick_rate: u16`, `server_tick: u64` |
| `Snapshot` | 3 | `tick: u64`, `seq: u32`, `payload: bytes` |
| `Rpc` | 4 | `target: PlayerId`（0=服务器）, `method: str`, `payload: bytes` |
| `Heartbeat` | 5 | `nonce: u64`, `pong: bool` |
| `Disconnect` | 6 | `reason: enum` |
| `JoinRoom`/`LeaveRoom`/`RoomJoined`/`RoomLeft`/`PlayerJoined`/`PlayerLeft`/`Chat`/`JoinRejected` | 7–14 | 房间操作与事件 |

TCP framing：`[u32 payload_len LE][payload]`，`FrameCodec::encode` / `FrameDecoder::push`。

## 6. 测试

```bash
cd engine
cargo test -p hezhou-net --offline
cargo clippy -p hezhou-net --all-targets --offline -- -D warnings
cargo fmt -p hezhou-net -- --check
```

覆盖：

- framing 编解码、分片/粘包、超大帧拒绝；
- UDP loopback 收发、UDP 单包超限、`connect` 约束；
- TCP loopback 帧收发、分片/粘包、超大帧拒绝；
- 消息全变体 roundtrip、字段/大小超限、坏输入、kind 稳定性；
- 会话：握手分配 ID、心跳→pong、重复/乱序序列号、超时、主动断开、协议版本拒绝；
- 房间：加入/离开/容量/隔离/广播目标；
- loopback 集成：3 客户端并发握手、房间广播聊天顺序、多客户端 Join/Leave、超时清理、房间满拒绝。

时间由调用方传入 `Instant`，循环测试用真实时间 + 短暂 sleep；超时测试用合成
`Instant + Duration`，不依赖真实等待。

## 7. Windows / Linux 差异

- 两者都用 `std::net` + 非阻塞 socket；本 crate 没有平台特定 socket option（不设
  `SO_REUSEADDR`/`SO_REUSEPORT`），行为一致。
- 绑定：Windows 默认 IPv4 `0.0.0.0:0`；Linux 同样。客户端按服务器地址族选择
  `0.0.0.0:0` / `[::]:0`。
- 临时端口/防火墙：首次监听可能触发 Windows Defender 防火墙提示；Linux 无提示但
  受 iptables/nftables 影响。测试使用 `127.0.0.1` 不受影响。
- UDP 缓冲区/丢包率：不同内核默认值不同；loopback 测试不受影响。生产环境建议上层
  增加可靠性/重传。
- 关闭行为：TCP 对端关闭时 Windows/Linux 都表现为 `read()==0` → 本 crate 返回
  `TransportError::Closed`；UDP `send_to` 到未监听端口在 Windows 可能延迟报错/不报错，
  Linux 通常不报错（UDP 特性）。
- 时间：`Instant` 单调时钟，跨平台；测试中合成时间仅用于会话逻辑，不涉及系统时钟。

## 8. 后续接入（同步层）

- `NetServer::broadcast(room, message)` / `send_to_player(player, message)` 可直接用于
  快照广播与定向 RPC；
- `ServerEvent::Message` 把 Snapshot/Rpc 交给上层同步层；
- `SessionStats`（RTT 未来扩展、丢包计数）可用于网络质量 UI；
- 可靠层可在 UDP 序列号基础上增加 ack/重传，而 `Snapshot` 的 payload 已按上层自定义；
- `RoomManager` 可替换为匹配器/大厅服务。

## 9. 已知限制

- 单线程、无 `SO_REUSE*`、无连接迁移/重连策略；
- UDP 明文、无认证/加密/anti-cheat；
- 无分片：超过 1200 字节的消息必须上层拆分；
- TCP 只有单连接 framing，无连接池/TLS；
- 消息格式版本单一（1），不向后兼容；变更 kind/字段需同步升级所有端；
- 本 crate 不持久化任何会话/房间状态，进程重启即丢失。
