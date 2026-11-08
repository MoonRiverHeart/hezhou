//! 快照同步 / 插值 / RPC / NetServer+NetClient 集成测试（loopback + 内存）。

use std::time::{Duration, Instant};

use hezhou_net::{
    ClientEvent, EntityState, Message, NetClient, NetConfig, NetServer, PlayerId,
    ReplicationClient, ReplicationConfig, ReplicationServer, RpcCall, RpcRegistry, RpcResult,
    ServerEvent, SnapshotBuffer, SnapshotError, SnapshotPacket, WorldSnapshot,
};

fn entity(id: u32, x: f32, rotation: [f32; 4]) -> EntityState {
    EntityState::new(id, [x, 0.0, 0.0], rotation, 0)
}

fn snapshot(tick: u64, time_ms: u64, entities: Vec<EntityState>) -> WorldSnapshot {
    WorldSnapshot::new(tick, time_ms, entities)
}

fn net_config() -> NetConfig {
    NetConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        max_clients: 4,
        timeout: Duration::from_secs(5),
        heartbeat_interval: Duration::from_millis(50),
        handshake_timeout: Duration::from_millis(500),
        max_message_bytes: 1200,
        tick_rate: 60,
        room_capacity: 4,
        default_room: hezhou_net::RoomId::new(1),
    }
}

fn pump_until(
    server: &mut NetServer,
    clients: &mut [NetClient],
    mut done: impl FnMut(&NetServer, &[NetClient]) -> bool,
) -> (Vec<ServerEvent>, Vec<Vec<ClientEvent>>) {
    let mut server_events = Vec::new();
    let mut client_events: Vec<Vec<ClientEvent>> = vec![Vec::new(); clients.len()];
    for _ in 0..2_000 {
        let now = Instant::now();
        server_events.extend(server.poll(now, 64));
        for (index, client) in clients.iter_mut().enumerate() {
            client_events[index].extend(client.poll(now, 64));
        }
        if done(server, clients) {
            return (server_events, client_events);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("loopback pump 超时");
}

fn pump_for(
    server: &mut NetServer,
    clients: &mut [NetClient],
    iterations: usize,
) -> (Vec<ServerEvent>, Vec<Vec<ClientEvent>>) {
    let mut server_events = Vec::new();
    let mut client_events: Vec<Vec<ClientEvent>> = vec![Vec::new(); clients.len()];
    for _ in 0..iterations {
        let now = Instant::now();
        server_events.extend(server.poll(now, 64));
        for (index, client) in clients.iter_mut().enumerate() {
            client_events[index].extend(client.poll(now, 64));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    (server_events, client_events)
}

#[test]
fn snapshot_full_delta_and_stale_tick_rejection() {
    let baseline = snapshot(
        1,
        100,
        vec![
            entity(1, 0.0, EntityState::IDENTITY_ROTATION),
            entity(2, 10.0, EntityState::IDENTITY_ROTATION),
        ],
    );
    let full = SnapshotPacket::Full(baseline.clone());
    let bytes = full.encode().unwrap();
    let decoded = SnapshotPacket::decode(&bytes).unwrap();
    let mut client = ReplicationClient::new();
    let outcome = client.apply_packet(&decoded).unwrap();
    assert!(outcome.is_full);
    assert_eq!(client.latest().unwrap(), &baseline);

    // 旧 tick 拒绝。
    assert!(matches!(
        client.apply_packet(&full),
        Err(SnapshotError::StaleTick {
            last: 1,
            received: 1
        })
    ));
    assert_eq!(client.stats().stale_rejected, 1);

    let current = snapshot(
        2,
        200,
        vec![
            entity(1, 5.0, EntityState::IDENTITY_ROTATION),
            entity(3, 30.0, EntityState::IDENTITY_ROTATION),
        ],
    );
    let delta = SnapshotPacket::delta_from(&baseline, &current);
    let delta_bytes = delta.encode().unwrap();
    let decoded_delta = SnapshotPacket::decode(&delta_bytes).unwrap();
    let outcome = client.apply_packet(&decoded_delta).unwrap();
    assert!(!outcome.is_full);
    assert_eq!(outcome.baseline_tick, Some(1));
    let latest = client.latest().unwrap();
    assert_eq!(latest.tick, 2);
    assert_eq!(latest.entity_count(), 2);
    assert!(latest.entity(2).is_none(), "删除实体应被移除");
    assert!(latest.entity(3).is_some(), "新增实体应被加入");
    assert!((latest.entity(1).unwrap().position[0] - 5.0).abs() < 1e-6);

    // 缺失 baseline 的 delta 被拒绝。
    let mut fresh = ReplicationClient::new();
    assert!(matches!(
        fresh.apply_packet(&decoded_delta),
        Err(SnapshotError::MissingBaseline)
    ));
}

#[test]
fn client_interpolation_position_rotation_and_packet_loss_fallback() {
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let first = snapshot(1, 100, vec![entity(1, 0.0, EntityState::IDENTITY_ROTATION)]);
    let second = snapshot(2, 200, vec![entity(1, 10.0, [0.0, half, 0.0, half])]);
    let mut client = ReplicationClient::new();
    client
        .apply_packet(&SnapshotPacket::Full(first.clone()))
        .unwrap();
    client
        .apply_packet(&SnapshotPacket::Full(second.clone()))
        .unwrap();
    assert_eq!(client.buffer().len(), 2);
    let sampled = client.sample(150.0).unwrap();
    let interpolated = sampled.entity(1).unwrap();
    assert!((interpolated.position[0] - 5.0).abs() < 1e-5);
    assert!((interpolated.rotation[1] - 0.382_683_43).abs() < 1e-4);
    assert!((interpolated.rotation[3] - 0.923_879_5).abs() < 1e-4);

    // 丢包：客户端只收到 tick1 后直接收到 tick3，gap 计数 + 缓冲回退到最新（不外推）。
    let mut lossy = ReplicationClient::new();
    lossy
        .apply_packet(&SnapshotPacket::Full(snapshot(
            1,
            100,
            vec![entity(1, 0.0, EntityState::IDENTITY_ROTATION)],
        )))
        .unwrap();
    let tick3 = snapshot(
        3,
        300,
        vec![entity(1, 30.0, EntityState::IDENTITY_ROTATION)],
    );
    let outcome = lossy
        .apply_packet(&SnapshotPacket::Full(tick3.clone()))
        .unwrap();
    assert!(outcome.gap, "tick 1 → 3 应检测为跳号");
    assert_eq!(lossy.stats().gaps, 1);
    // 渲染时间晚于最新帧：回退到最新帧而非外推预测。
    assert_eq!(lossy.sample(10_000.0).unwrap().tick, 3);
    // 空缓冲没有可插值帧。
    let buffer = SnapshotBuffer::new(2);
    assert!(buffer.sample(0.0).is_none());
    // 只有最新一帧时（旧帧被丢/被清空）也回退到最新。
    let mut latest_only = ReplicationClient::new();
    latest_only
        .apply_packet(&SnapshotPacket::Full(snapshot(
            5,
            500,
            vec![entity(1, 50.0, EntityState::IDENTITY_ROTATION)],
        )))
        .unwrap();
    assert_eq!(latest_only.sample(0.0).unwrap().tick, 5);
}

#[test]
fn replication_server_ack_builds_delta_end_to_end_in_memory() {
    let mut server = ReplicationServer::new(ReplicationConfig {
        full_snapshot_interval_ticks: 100,
        ..ReplicationConfig::default()
    });
    let player = PlayerId::new(1);
    server.register_client(player);
    server
        .upsert_entity(entity(1, 1.0, EntityState::IDENTITY_ROTATION))
        .unwrap();
    server
        .upsert_entity(entity(2, 2.0, EntityState::IDENTITY_ROTATION))
        .unwrap();
    server.tick(1_000);
    let full = server.build_packet(player).unwrap();
    assert!(full.is_full);
    let mut client = ReplicationClient::new();
    client.apply_bytes(&full.bytes).unwrap();
    // 客户端确认 baseline。
    assert!(server.ack(player, full.tick()).unwrap());

    server.tick(2_000);
    server
        .upsert_entity(entity(1, 9.0, EntityState::IDENTITY_ROTATION))
        .unwrap();
    server.remove_entity(2);
    let delta = server.build_packet(player).unwrap();
    assert!(!delta.is_full);
    let outcome = client.apply_bytes(&delta.bytes).unwrap();
    assert_eq!(outcome.baseline_tick, Some(full.tick()));
    assert_eq!(client.last_applied_tick(), Some(delta.tick()));
    assert_eq!(client.latest().unwrap().entity_count(), 1);
    assert_eq!(server.client_stats(player).unwrap().delta_snapshots, 1);
}

#[test]
fn rpc_registry_register_call_unknown_and_handler_error() {
    let mut registry = RpcRegistry::new();
    registry
        .register("echo", |payload| Ok(payload.to_vec()))
        .unwrap();
    registry
        .register("add", |payload| {
            if payload.len() != 8 {
                return Err(hezhou_net::RpcError::Decode(
                    "add 需要 2 个 i32".to_string(),
                ));
            }
            let a = i32::from_le_bytes(payload[..4].try_into().unwrap());
            let b = i32::from_le_bytes(payload[4..].try_into().unwrap());
            Ok((a + b).to_le_bytes().to_vec())
        })
        .unwrap();
    registry
        .register("fail", |_| {
            Err(hezhou_net::RpcError::HandlerError("失败".to_string()))
        })
        .unwrap();

    let echo = RpcCall::new(PlayerId::NONE, "echo", b"hello".to_vec(), 7);
    let bytes = echo.encode().unwrap();
    let result_bytes = registry.call_bytes(&bytes).unwrap();
    let result = RpcResult::decode(&result_bytes).unwrap();
    assert!(result.ok);
    assert_eq!(result.seq, 7);
    assert_eq!(result.payload, b"hello");

    let unknown = RpcCall::new(PlayerId::NONE, "missing", Vec::new(), 1);
    assert!(matches!(
        registry.call(&unknown),
        Err(hezhou_net::RpcError::UnknownMethod(_))
    ));
    let failed = RpcCall::new(PlayerId::NONE, "fail", Vec::new(), 2);
    let result = registry.call(&failed).unwrap();
    assert!(!result.ok);
    assert!(result.error.unwrap().contains("失败"));
}

#[test]
fn udp_loopback_snapshot_transfer_ack_and_rpc_roundtrip() {
    let config = net_config();
    let mut server = NetServer::bind(&config).unwrap();
    let addr = server.local_addr().unwrap();
    let mut clients = vec![NetClient::connect(addr, "replica", &config).unwrap()];
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 1 && clients[0].is_connected()
    });
    let player = clients[0].player_id();

    // 独立的权威复制层：world -> snapshot -> Message::Snapshot -> UDP -> client。
    let mut replication = ReplicationServer::new(ReplicationConfig {
        full_snapshot_interval_ticks: 100,
        ..ReplicationConfig::default()
    });
    replication.register_client(player);
    replication
        .upsert_entity(entity(1, 1.0, EntityState::IDENTITY_ROTATION))
        .unwrap();
    replication.tick(1_000);
    let full = replication.build_packet(player).unwrap();
    assert!(server.send_to_player(
        player,
        Message::Snapshot {
            tick: full.tick(),
            seq: 0,
            payload: full.bytes.clone(),
        }
    ));
    let (_, client_events) = pump_for(&mut server, &mut clients, 30);
    let snapshot_message = client_events[0]
        .iter()
        .find_map(|event| match event {
            ClientEvent::Message(Message::Snapshot { payload, .. }) => Some(payload.clone()),
            _ => None,
        })
        .expect("客户端应收到 Snapshot");
    let mut replica_client = ReplicationClient::new();
    replica_client.apply_bytes(&snapshot_message).unwrap();
    assert_eq!(replica_client.latest().unwrap().tick, full.tick());
    assert!(replication.ack(player, full.tick()).unwrap());

    // 客户端 RPC：echo，服务器注册表分发并将结果发回。
    let mut registry = RpcRegistry::new();
    registry
        .register("echo", |payload| Ok(payload.to_vec()))
        .unwrap();
    clients[0]
        .send_rpc(PlayerId::NONE, "echo", b"ping".to_vec())
        .unwrap();
    let (server_events, _) = pump_for(&mut server, &mut clients, 30);
    let (request_target, request_method, request_payload) = server_events
        .iter()
        .find_map(|event| match event {
            ServerEvent::Message {
                player_id,
                message:
                    Message::Rpc {
                        target,
                        method,
                        payload,
                    },
                ..
            } if *player_id == player => Some((*target, method.clone(), payload.clone())),
            _ => None,
        })
        .expect("服务器应收到 Rpc");
    assert_eq!(request_method, "echo");
    let call = RpcCall::new(request_target, request_method, request_payload, 42);
    let result = registry.call(&call).unwrap();
    assert!(result.ok);
    server.send_to_player(
        player,
        Message::Rpc {
            target: player,
            method: "echo.result".to_string(),
            payload: result.encode().unwrap(),
        },
    );
    let (_, client_events) = pump_for(&mut server, &mut clients, 30);
    let response = client_events[0]
        .iter()
        .find_map(|event| match event {
            ClientEvent::Message(Message::Rpc { payload, .. }) => Some(payload.clone()),
            _ => None,
        })
        .expect("客户端应收到 RPC 结果");
    let result = RpcResult::decode(&response).unwrap();
    assert!(result.ok);
    assert_eq!(result.seq, 42);
    assert_eq!(result.payload, b"ping");
}
