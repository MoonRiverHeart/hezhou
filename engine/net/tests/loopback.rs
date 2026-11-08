//! UDP loopback 集成测试：多客户端、房间广播/定向、心跳、超时、断开。
//!
//! 这些测试使用真实 `127.0.0.1` UDP socket；时间用调用方传入的 `Instant` 控制，
//! 因此超时测试不需要真实等待（合成时间即可）。

use std::time::{Duration, Instant};

use hezhou_net::{
    ClientEvent, DisconnectReason, Message, NetClient, NetConfig, NetServer, RoomId,
    RoomRejectReason, ServerEvent,
};

fn test_config() -> NetConfig {
    NetConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        max_clients: 8,
        timeout: Duration::from_secs(5),
        heartbeat_interval: Duration::from_millis(20),
        handshake_timeout: Duration::from_millis(500),
        max_message_bytes: 1200,
        tick_rate: 60,
        room_capacity: 4,
        default_room: RoomId::new(1),
    }
}

/// 建服务器 + n 个客户端；返回服务器与客户端。
fn setup(n: usize) -> (NetServer, Vec<NetClient>) {
    let config = test_config();
    let server = NetServer::bind(&config).expect("server bind");
    let addr = server.local_addr().expect("server addr");
    let clients = (0..n)
        .map(|index| {
            NetClient::connect(addr, &format!("player-{index}"), &config).expect("client connect")
        })
        .collect();
    // 第一条 Hello 由 connect 时发送；余下靠 pump。
    (server, clients)
}

/// 轮询服务器与客户端（真实时间 now），直到 `done` 为真或超时。
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

fn client_messages(events: &[ClientEvent]) -> Vec<Message> {
    events
        .iter()
        .filter_map(|event| match event {
            ClientEvent::Message(message) => Some(message.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn multi_client_handshake_assigns_unique_sessions() {
    let (mut server, mut clients) = setup(3);
    let (server_events, client_events) =
        pump_until(&mut server, &mut clients, |server, clients| {
            server.session_count() == 3 && clients.iter().all(NetClient::is_connected)
        });

    assert_eq!(server.session_count(), 3);
    assert_eq!(server.player_count(), 3);
    assert!(clients.iter().all(NetClient::is_connected));
    assert!(clients.iter().all(|client| client.session_id().raw() > 0));
    let mut ids: Vec<u32> = clients
        .iter()
        .map(|client| client.session_id().raw())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 3, "三个客户端 session id 应唯一");

    // 每个客户端都应收到 Connected 事件；服务器应有 3 个 ClientConnected。
    assert_eq!(
        client_events
            .iter()
            .filter(|events| events
                .iter()
                .any(|event| matches!(event, ClientEvent::Connected { .. })))
            .count(),
        3
    );
    assert_eq!(
        server_events
            .iter()
            .filter(|event| matches!(event, ServerEvent::ClientConnected { .. }))
            .count(),
        3
    );
}

#[test]
fn room_join_broadcast_chat_and_leave() {
    let (mut server, mut clients) = setup(2);
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 2 && clients.iter().all(NetClient::is_connected)
    });

    let room = RoomId::new(7);
    clients[0].join_room(room).unwrap();
    clients[1].join_room(room).unwrap();
    let (server_events, client_events) =
        pump_until(&mut server, &mut clients, |server, clients| {
            server.room_of(clients[0].player_id()) == Some(room)
                && server.room_of(clients[1].player_id()) == Some(room)
                && clients.iter().all(|client| client.room() == Some(room))
        });
    assert_eq!(
        server_events
            .iter()
            .filter(|event| matches!(event, ServerEvent::RoomJoined { .. }))
            .count(),
        2
    );
    assert!(client_events.iter().any(|events| client_messages(events)
        .iter()
        .any(|message| matches!(message, Message::PlayerJoined { .. }))));

    // 第一个客户端发聊天，两个客户端都应收到广播。
    clients[0].send_chat("hello-room").unwrap();
    let (server_events, client_events) = pump_for(&mut server, &mut clients, 30);
    assert!(server_events
        .iter()
        .any(|event| matches!(event, ServerEvent::Chat { text, .. } if text == "hello-room")));
    assert!(client_events.iter().all(|events| client_messages(events)
        .iter()
        .any(|message| matches!(message, Message::Chat { text, .. } if text == "hello-room"))));

    // 离开房间：一个客户端离开，另一个收到 PlayerLeft。
    clients[1].leave_room().unwrap();
    let (server_events, client_events) = pump_for(&mut server, &mut clients, 20);
    assert!(server_events
        .iter()
        .any(|event| matches!(event, ServerEvent::RoomLeft { room: left, .. } if *left == room)));
    assert_eq!(server.room_of(clients[0].player_id()), Some(room));
    assert_eq!(server.room_of(clients[1].player_id()), None);
    assert!(client_messages(&client_events[0])
        .iter()
        .any(|message| matches!(message, Message::PlayerLeft { room: left, .. } if *left == room)));
    assert!(client_messages(&client_events[1])
        .iter()
        .any(|message| matches!(message, Message::RoomLeft { room: left } if *left == room)));
}

#[test]
fn chat_broadcast_reaches_all_members() {
    let (mut server, mut clients) = setup(2);
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 2 && clients.iter().all(NetClient::is_connected)
    });
    let room = RoomId::new(9);
    clients[0].join_room(room).unwrap();
    clients[1].join_room(room).unwrap();
    let _ = pump_until(&mut server, &mut clients, |_, clients| {
        clients.iter().all(|client| client.room() == Some(room))
    });

    clients[0].send_chat("first").unwrap();
    clients[0].send_chat("second").unwrap();
    let (_, client_events) = pump_for(&mut server, &mut clients, 40);
    let first_messages = client_messages(&client_events[0]);
    let chats: Vec<&str> = first_messages
        .iter()
        .filter_map(|message| match message {
            Message::Chat { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(chats, vec!["first", "second"], "同一客户端的聊天顺序应保持");
    let second_messages = client_messages(&client_events[1]);
    assert!(second_messages
        .iter()
        .any(|message| matches!(message, Message::Chat { text, .. } if text == "first")));
    assert!(second_messages
        .iter()
        .any(|message| matches!(message, Message::Chat { text, .. } if text == "second")));
}

#[test]
fn heartbeat_keeps_session_alive_and_timeout_removes_it() {
    let (mut server, mut clients) = setup(1);
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 1 && clients[0].is_connected()
    });
    let player = clients[0].player_id();
    let heartbeat_interval = test_config().heartbeat_interval;

    // 服务器推进到心跳时间：发出 Heartbeat(ping)，客户端回 pong。
    let now = Instant::now();
    let server_events = server.tick(now + heartbeat_interval);
    let _ = pump_for(&mut server, &mut clients, 10);
    assert!(server_events
        .iter()
        .all(|event| !matches!(event, ServerEvent::ClientDisconnected { .. })));
    assert!(clients[0].session_stats().heartbeats_received >= 1);
    let _ = server.tick(Instant::now());
    assert_eq!(server.session_count(), 1);
    assert!(server.session_stats_for(player).is_some());

    // 合成时间超过 timeout → 服务器移除会话并广播断开。
    let timeout = test_config().timeout;
    let events = server.tick(Instant::now() + timeout + Duration::from_secs(1));
    assert_eq!(server.session_count(), 0);
    assert!(events.iter().any(|event| matches!(
        event,
        ServerEvent::ClientDisconnected { player_id, reason: DisconnectReason::Timeout, .. }
            if *player_id == player
    )));
}

#[test]
fn client_disconnect_cleans_room_and_notifies_others() {
    let (mut server, mut clients) = setup(2);
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 2 && clients.iter().all(NetClient::is_connected)
    });
    let room = RoomId::new(11);
    clients[0].join_room(room).unwrap();
    clients[1].join_room(room).unwrap();
    let _ = pump_until(&mut server, &mut clients, |_, clients| {
        clients.iter().all(|client| client.room() == Some(room))
    });
    let leaving = clients[0].player_id();
    clients[0].disconnect(DisconnectReason::Normal);
    let (server_events, client_events) = pump_for(&mut server, &mut clients, 30);

    assert_eq!(server.session_count(), 1);
    assert_eq!(server.room_of(leaving), None);
    assert!(server_events.iter().any(|event| matches!(
        event,
        ServerEvent::ClientDisconnected { player_id, reason: DisconnectReason::Normal, room: Some(disconnected_room), .. }
            if *player_id == leaving && *disconnected_room == room
    )));
    assert!(client_messages(&client_events[1])
        .iter()
        .any(|message| matches!(
            message,
            Message::PlayerLeft { player, room: left } if *player == leaving && *left == room
        )));
}

#[test]
fn join_rejected_when_room_full() {
    let mut config = test_config();
    config.room_capacity = 1;
    let mut server = NetServer::bind(&config).unwrap();
    let addr = server.local_addr().unwrap();
    let mut clients = vec![
        NetClient::connect(addr, "a", &config).unwrap(),
        NetClient::connect(addr, "b", &config).unwrap(),
    ];
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 2 && clients.iter().all(NetClient::is_connected)
    });
    let room = RoomId::new(3);
    clients[0].join_room(room).unwrap();
    let _ = pump_until(&mut server, &mut clients, |server, _| {
        server.room_manager().members(room).len() == 1
    });
    clients[1].join_room(room).unwrap();
    let (server_events, client_events) = pump_for(&mut server, &mut clients, 30);
    assert!(server_events.iter().any(|event| matches!(
        event,
        ServerEvent::JoinRejected { room: rejected, reason: RoomRejectReason::Full, .. } if *rejected == room
    )));
    assert!(client_messages(&client_events[1]).iter().any(|message| matches!(
        message,
        Message::JoinRejected { room: rejected, reason: RoomRejectReason::Full } if *rejected == room
    )));
}

#[test]
fn directed_send_reaches_only_target_player() {
    let (mut server, mut clients) = setup(2);
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 2 && clients.iter().all(NetClient::is_connected)
    });
    let room = RoomId::new(13);
    clients[0].join_room(room).unwrap();
    clients[1].join_room(room).unwrap();
    let _ = pump_until(&mut server, &mut clients, |_, clients| {
        clients.iter().all(|client| client.room() == Some(room))
    });

    let target = clients[0].player_id();
    let direct = Message::Chat {
        room,
        player: target,
        text: "direct".to_string(),
    };
    assert!(server.send_to_player(target, direct));
    let (_, client_events) = pump_for(&mut server, &mut clients, 30);
    assert!(client_messages(&client_events[0])
        .iter()
        .any(|message| matches!(message, Message::Chat { text, .. } if text == "direct")));
    assert!(!client_messages(&client_events[1])
        .iter()
        .any(|message| matches!(message, Message::Chat { text, .. } if text == "direct")));
}

#[test]
fn timeout_removes_all_silent_sessions() {
    let (mut server, mut clients) = setup(2);
    let _ = pump_until(&mut server, &mut clients, |server, clients| {
        server.session_count() == 2 && clients.iter().all(NetClient::is_connected)
    });
    let player_a = clients[0].player_id();
    let player_b = clients[1].player_id();

    // 只让客户端 A 发一次消息刷新 last_seen（B 保持旧时间），再用超时时间 tick。
    let timeout = test_config().timeout;
    let events = server.tick(Instant::now() + timeout + Duration::from_secs(1));
    assert_eq!(server.session_count(), 0, "两个会话都应超时");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                ServerEvent::ClientDisconnected {
                    reason: DisconnectReason::Timeout,
                    ..
                }
            ))
            .count(),
        2
    );
    assert_ne!(player_a, player_b);
}
