//! 房间注册表：加入/离开/成员查询/广播目标。
//!
//! 本模块只管理"谁在哪个房间"，不关心消息如何发送；[`crate::server::NetServer`]
//! 负责把房间事件转成网络消息（RoomJoined/PlayerJoined/PlayerLeft/Chat）。
//!
//! - 房间按需创建（`join` 时不存在则创建）；
//! - 一个玩家同时只能在一个房间（`AlreadyInRoom`）；
//! - 空房间自动删除，避免长期运行后房间表膨胀；
//! - 容量来自 [`RoomConfig::max_players`]。

use std::collections::BTreeMap;

use crate::error::RoomError;
use crate::ids::{PlayerId, RoomId};

/// 房间配置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomConfig {
    /// 单房间最大人数（至少 1）。
    pub max_players: usize,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self { max_players: 16 }
    }
}

impl RoomConfig {
    /// 构造并钳制容量。
    pub fn new(max_players: usize) -> Self {
        Self {
            max_players: max_players.max(1),
        }
    }
}

/// 房间成员。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomMember {
    /// 玩家 ID。
    pub player: PlayerId,
    /// 显示名。
    pub name: String,
}

/// 一个房间。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    /// 房间 ID。
    pub id: RoomId,
    members: BTreeMap<PlayerId, RoomMember>,
}

impl Room {
    fn new(id: RoomId) -> Self {
        Self {
            id,
            members: BTreeMap::new(),
        }
    }

    /// 当前人数。
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// 玩家是否在房间。
    pub fn contains(&self, player: PlayerId) -> bool {
        self.members.contains_key(&player)
    }

    /// 成员信息。
    pub fn member(&self, player: PlayerId) -> Option<&RoomMember> {
        self.members.get(&player)
    }

    /// 成员列表（按玩家 ID 排序）。
    pub fn members(&self) -> Vec<PlayerId> {
        self.members.keys().copied().collect()
    }

    /// 成员名列表（按玩家 ID 排序）。
    pub fn member_names(&self) -> Vec<&str> {
        self.members
            .values()
            .map(|member| member.name.as_str())
            .collect()
    }
}

/// 加入成功的结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinOutcome {
    /// 房间。
    pub room: RoomId,
    /// 加入的玩家。
    pub player: PlayerId,
    /// 加入前已在房间的成员（用于给新玩家发 RoomJoined）。
    pub existing_members: Vec<PlayerId>,
}

/// 离开的结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaveOutcome {
    /// 房间。
    pub room: RoomId,
    /// 离开的玩家。
    pub player: PlayerId,
    /// 离开后剩余成员（用于广播 PlayerLeft）。
    pub remaining: Vec<PlayerId>,
    /// 房间是否因空而删除。
    pub room_removed: bool,
}

/// 加入房间错误（别名，便于调用方按语义命名）。
pub type JoinError = RoomError;

/// 房间注册表。
#[derive(Clone, Debug)]
pub struct RoomManager {
    config: RoomConfig,
    rooms: BTreeMap<RoomId, Room>,
    membership: BTreeMap<PlayerId, RoomId>,
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new(RoomConfig::default())
    }
}

impl RoomManager {
    /// 创建房间注册表。
    pub fn new(config: RoomConfig) -> Self {
        Self {
            config,
            rooms: BTreeMap::new(),
            membership: BTreeMap::new(),
        }
    }

    /// 配置。
    pub fn config(&self) -> RoomConfig {
        self.config
    }

    /// 加入房间（不存在则创建）。返回加入前的成员列表。
    pub fn join(
        &mut self,
        room: RoomId,
        player: PlayerId,
        name: impl Into<String>,
    ) -> Result<JoinOutcome, JoinError> {
        if room.is_none() {
            return Err(JoinError::NotFound(room));
        }
        if let Some(existing) = self.membership.get(&player) {
            return Err(JoinError::AlreadyInRoom { room: *existing });
        }
        let room_ref = self.rooms.entry(room).or_insert_with(|| Room::new(room));
        if room_ref.len() >= self.config.max_players {
            return Err(JoinError::Full {
                room,
                max: self.config.max_players,
            });
        }
        let existing_members = room_ref.members();
        room_ref.members.insert(
            player,
            RoomMember {
                player,
                name: name.into(),
            },
        );
        self.membership.insert(player, room);
        Ok(JoinOutcome {
            room,
            player,
            existing_members,
        })
    }

    /// 离开当前房间；不在房间返回 `None`。
    pub fn leave(&mut self, player: PlayerId) -> Option<LeaveOutcome> {
        let room = self.membership.remove(&player)?;
        let room_entry = self.rooms.get_mut(&room)?;
        room_entry.members.remove(&player);
        let remaining = room_entry.members();
        let room_removed = room_entry.is_empty();
        if room_removed {
            self.rooms.remove(&room);
        }
        Some(LeaveOutcome {
            room,
            player,
            remaining,
            room_removed,
        })
    }

    /// 玩家当前所在房间。
    pub fn room_of(&self, player: PlayerId) -> Option<RoomId> {
        self.membership.get(&player).copied()
    }

    /// 房间信息。
    pub fn room(&self, id: RoomId) -> Option<&Room> {
        self.rooms.get(&id)
    }

    /// 房间成员列表（房间不存在返回空）。
    pub fn members(&self, room: RoomId) -> Vec<PlayerId> {
        self.rooms.get(&room).map(Room::members).unwrap_or_default()
    }

    /// 广播目标（房间成员；房间不存在返回空）。
    pub fn broadcast_targets(&self, room: RoomId) -> Vec<PlayerId> {
        self.members(room)
    }

    /// 房间数量。
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// 已加入房间的玩家数量。
    pub fn player_count(&self) -> usize {
        self.membership.len()
    }

    /// 是否没有任何房间/玩家。
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty() && self.membership.is_empty()
    }

    /// 房间迭代（按 RoomId 排序）。
    pub fn rooms(&self) -> impl Iterator<Item = &Room> {
        self.rooms.values()
    }

    /// 强制移除玩家（会话超时/断开时使用），等价于 `leave`。
    pub fn remove_player(&mut self, player: PlayerId) -> Option<LeaveOutcome> {
        self.leave(player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_leave_broadcast_targets() {
        let mut manager = RoomManager::new(RoomConfig::new(4));
        let room = RoomId::new(7);
        let outcome = manager.join(room, PlayerId::new(1), "alice").unwrap();
        assert!(outcome.existing_members.is_empty());
        let outcome = manager.join(room, PlayerId::new(2), "bob").unwrap();
        assert_eq!(outcome.existing_members, vec![PlayerId::new(1)]);
        assert_eq!(
            manager.broadcast_targets(room),
            vec![PlayerId::new(1), PlayerId::new(2)]
        );
        assert_eq!(manager.player_count(), 2);
        assert_eq!(manager.room_count(), 1);

        let leave = manager.leave(PlayerId::new(1)).unwrap();
        assert_eq!(leave.remaining, vec![PlayerId::new(2)]);
        assert!(!leave.room_removed);
        let leave = manager.leave(PlayerId::new(2)).unwrap();
        assert!(leave.room_removed);
        assert!(manager.is_empty());
        assert!(manager.leave(PlayerId::new(2)).is_none());
    }

    #[test]
    fn duplicate_join_and_full_room_rejected() {
        let mut manager = RoomManager::new(RoomConfig::new(1));
        let room = RoomId::new(3);
        manager.join(room, PlayerId::new(1), "a").unwrap();
        assert!(matches!(
            manager.join(room, PlayerId::new(1), "a"),
            Err(RoomError::AlreadyInRoom { room: existing }) if existing == room
        ));
        assert!(matches!(
            manager.join(room, PlayerId::new(2), "b"),
            Err(RoomError::Full { room: full, max: 1 }) if full == room
        ));
        // 无效房间 0。
        assert!(matches!(
            manager.join(RoomId::NONE, PlayerId::new(3), "c"),
            Err(RoomError::NotFound(_))
        ));
    }

    #[test]
    fn room_isolated_and_leave_other_room() {
        let mut manager = RoomManager::new(RoomConfig::new(8));
        manager.join(RoomId::new(1), PlayerId::new(1), "a").unwrap();
        manager.join(RoomId::new(2), PlayerId::new(2), "b").unwrap();
        assert_eq!(
            manager.broadcast_targets(RoomId::new(1)),
            vec![PlayerId::new(1)]
        );
        assert_eq!(
            manager.broadcast_targets(RoomId::new(2)),
            vec![PlayerId::new(2)]
        );
        assert_eq!(manager.room_of(PlayerId::new(1)), Some(RoomId::new(1)));
        manager.leave(PlayerId::new(1)).unwrap();
        assert_eq!(manager.room_count(), 1);
        assert_eq!(
            manager.broadcast_targets(RoomId::new(1)),
            Vec::<PlayerId>::new()
        );
    }
}
