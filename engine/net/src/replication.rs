//! 服务器权威复制：全量/增量快照构建、per-client baseline ack、上限/背压；客户端应用。
//!
//! ## 同步模型
//!
//! - 服务器是**唯一权威**：世界状态由游戏逻辑写入 [`ReplicationServer`]；
//! - 服务器每 tick 记录一份 [`WorldSnapshot`] 历史；客户端通过 [`ReplicationServer::ack`]
//!   确认已应用的 tick，服务器以该历史快照作为下一次 delta 的 baseline；
//! - 首次/间隔到期/ack 历史缺失/backpressure 时发**全量**，其余发**增量**（变化实体+删除列表）；
//! - 无预测、无回滚、无插值外推；客户端 [`ReplicationClient`] 只按顺序应用快照，
//!   [`SnapshotBuffer`] 负责渲染时间插值（丢包时回退到最新帧）；
//! - 每客户端独立构建，互不影响；`max_snapshot_bytes` 控制 UDP 单包/带宽上限，
//!   超出时本 tick 丢弃并计入 `backpressure_drops`。
//!
//! 本模块是 0.4 原型：不含权威帧循环、输入预测、回滚、可靠重传、加密。

use std::collections::{BTreeMap, VecDeque};

use crate::error::{ReplicationError, SnapshotError};
use crate::ids::PlayerId;
use crate::snapshot::{EntityState, SnapshotBuffer, SnapshotPacket, WorldSnapshot};

/// 默认全量快照间隔（tick）。
pub const DEFAULT_FULL_SNAPSHOT_INTERVAL_TICKS: u64 = 60;
/// 默认服务器快照历史长度（用于 ack baseline 查找）。
pub const DEFAULT_REPLICATION_HISTORY: usize = 64;
/// 默认单快照字节上限（适配 1200 字节 UDP 单包，留出 Packet/Message 头开销）。
pub const DEFAULT_REPLICATION_MAX_SNAPSHOT_BYTES: usize = 1024;
/// 默认世界实体上限。
pub const DEFAULT_REPLICATION_MAX_ENTITIES: usize = 256;

/// 复制层配置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationConfig {
    /// 单个快照编码后最大字节数；超过则背压丢弃。
    pub max_snapshot_bytes: usize,
    /// 每隔多少 tick 强制一次全量快照（至少 1）。
    pub full_snapshot_interval_ticks: u64,
    /// 世界实体上限。
    pub max_entities: usize,
    /// 服务器保留的历史快照数量（用于 ack baseline）。
    pub history_limit: usize,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            max_snapshot_bytes: DEFAULT_REPLICATION_MAX_SNAPSHOT_BYTES,
            full_snapshot_interval_ticks: DEFAULT_FULL_SNAPSHOT_INTERVAL_TICKS,
            max_entities: DEFAULT_REPLICATION_MAX_ENTITIES,
            history_limit: DEFAULT_REPLICATION_HISTORY,
        }
    }
}

/// 单个客户端的复制统计。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClientReplicationStats {
    /// 最近 ack 的 tick。
    pub acked_tick: u64,
    /// 发出的全量快照数。
    pub full_snapshots: u64,
    /// 发出的 delta 快照数。
    pub delta_snapshots: u64,
    /// 因超过大小上限被丢弃的次数（背压）。
    pub backpressure_drops: u64,
    /// 最近构建的快照字节数。
    pub last_snapshot_bytes: usize,
}

#[derive(Debug)]
struct ClientReplication {
    acked_baseline: Option<WorldSnapshot>,
    acked_tick: u64,
    needs_full: bool,
    full_snapshots: u64,
    delta_snapshots: u64,
    backpressure_drops: u64,
    last_snapshot_bytes: usize,
}

impl ClientReplication {
    fn new() -> Self {
        Self {
            acked_baseline: None,
            acked_tick: 0,
            needs_full: true,
            full_snapshots: 0,
            delta_snapshots: 0,
            backpressure_drops: 0,
            last_snapshot_bytes: 0,
        }
    }

    fn stats(&self) -> ClientReplicationStats {
        ClientReplicationStats {
            acked_tick: self.acked_tick,
            full_snapshots: self.full_snapshots,
            delta_snapshots: self.delta_snapshots,
            backpressure_drops: self.backpressure_drops,
            last_snapshot_bytes: self.last_snapshot_bytes,
        }
    }
}

/// 一个为客户端构建好的快照（可直接放入 `Message::Snapshot.payload`）。
#[derive(Clone, Debug, PartialEq)]
pub struct ClientSnapshot {
    /// 目标客户端。
    pub player: PlayerId,
    /// 快照包（全量/delta）。
    pub packet: SnapshotPacket,
    /// 编码后的字节（长度前缀/版本/上限已校验）。
    pub bytes: Vec<u8>,
    /// 是否全量。
    pub is_full: bool,
}

impl ClientSnapshot {
    /// 编码字节数。
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// 快照 tick。
    pub fn tick(&self) -> u64 {
        self.packet.tick()
    }
}

/// 服务器权威复制层（独立于 `NetServer`，可内存测试；由游戏循环驱动）。
#[derive(Debug)]
pub struct ReplicationServer {
    config: ReplicationConfig,
    tick: u64,
    server_time_ms: u64,
    entities: BTreeMap<u32, EntityState>,
    history: VecDeque<WorldSnapshot>,
    clients: BTreeMap<PlayerId, ClientReplication>,
}

impl ReplicationServer {
    /// 创建服务器。
    pub fn new(config: ReplicationConfig) -> Self {
        Self {
            config,
            tick: 0,
            server_time_ms: 0,
            entities: BTreeMap::new(),
            history: VecDeque::new(),
            clients: BTreeMap::new(),
        }
    }

    /// 配置。
    pub fn config(&self) -> ReplicationConfig {
        self.config
    }

    /// 当前 tick。
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// 服务器时间（最近一次 `tick` 的毫秒）。
    pub fn server_time_ms(&self) -> u64 {
        self.server_time_ms
    }

    /// 推进一个 tick：记录当前世界快照到历史。
    pub fn tick(&mut self, server_time_ms: u64) -> u64 {
        self.server_time_ms = server_time_ms;
        self.tick = self.tick.saturating_add(1);
        let snapshot = self.snapshot();
        self.history.push_back(snapshot);
        while self.history.len() > self.config.history_limit.max(1) {
            self.history.pop_front();
        }
        self.tick
    }

    /// 当前世界的快照（按实体 id 排序）。
    pub fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot::new(
            self.tick,
            self.server_time_ms,
            self.entities.values().copied().collect(),
        )
    }

    /// 世界实体迭代。
    pub fn entities(&self) -> impl Iterator<Item = &EntityState> {
        self.entities.values()
    }

    /// 实体数量。
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// 按 id 查实体。
    pub fn entity(&self, id: u32) -> Option<&EntityState> {
        self.entities.get(&id)
    }

    /// 新增/更新实体；超过上限返回 [`ReplicationError::EntityLimit`]。
    pub fn upsert_entity(&mut self, entity: EntityState) -> Result<(), ReplicationError> {
        if !self.entities.contains_key(&entity.id)
            && self.entities.len() >= self.config.max_entities
        {
            return Err(ReplicationError::EntityLimit {
                count: self.entities.len() + 1,
                max: self.config.max_entities,
            });
        }
        self.entities.insert(entity.id, entity);
        Ok(())
    }

    /// 删除实体，返回是否存在。
    pub fn remove_entity(&mut self, id: u32) -> bool {
        self.entities.remove(&id).is_some()
    }

    /// 清空世界。
    pub fn clear_entities(&mut self) {
        self.entities.clear();
    }

    /// 历史快照（用于 ack 查找）。
    pub fn history(&self) -> impl Iterator<Item = &WorldSnapshot> {
        self.history.iter()
    }

    /// 注册客户端（重复注册保留现有 ack 状态）。
    pub fn register_client(&mut self, player: PlayerId) {
        self.clients
            .entry(player)
            .or_insert_with(ClientReplication::new);
    }

    /// 注销客户端。
    pub fn unregister_client(&mut self, player: PlayerId) -> bool {
        self.clients.remove(&player).is_some()
    }

    /// 客户端是否已注册。
    pub fn is_registered(&self, player: PlayerId) -> bool {
        self.clients.contains_key(&player)
    }

    /// 客户端数量。
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// 客户端统计。
    pub fn client_stats(&self, player: PlayerId) -> Option<ClientReplicationStats> {
        self.clients.get(&player).map(ClientReplication::stats)
    }

    /// 客户端确认已应用 tick：若历史中存在该 tick 快照，作为下一次 delta 的 baseline。
    /// 返回 `true` 表示 baseline 已建立；`false` 表示下一步需回退全量。
    pub fn ack(&mut self, player: PlayerId, tick: u64) -> Result<bool, ReplicationError> {
        let Some(client) = self.clients.get_mut(&player) else {
            return Err(ReplicationError::UnknownClient(player));
        };
        client.acked_tick = tick;
        if let Some(snapshot) = self
            .history
            .iter()
            .find(|snapshot| snapshot.tick == tick)
            .cloned()
        {
            client.acked_baseline = Some(snapshot);
            client.needs_full = false;
            Ok(true)
        } else {
            client.acked_baseline = None;
            client.needs_full = true;
            Ok(false)
        }
    }

    /// 强制下一次为全量。
    pub fn force_full(&mut self, player: PlayerId) -> Result<(), ReplicationError> {
        let Some(client) = self.clients.get_mut(&player) else {
            return Err(ReplicationError::UnknownClient(player));
        };
        client.needs_full = true;
        client.acked_baseline = None;
        Ok(())
    }

    /// 为单个客户端构建快照：全量或 delta，并做大小上限/背压检查。
    pub fn build_packet(&mut self, player: PlayerId) -> Result<ClientSnapshot, ReplicationError> {
        if !self.clients.contains_key(&player) {
            return Err(ReplicationError::UnknownClient(player));
        }
        let current = self.snapshot();
        let client = self.clients.get(&player).expect("已检查");
        let baseline = client.acked_baseline.as_ref();
        let force_full = client.needs_full || baseline.is_none();
        let interval_due = baseline
            .map(|baseline| {
                current.tick.saturating_sub(baseline.tick)
                    >= self.config.full_snapshot_interval_ticks.max(1)
            })
            .unwrap_or(true);
        let use_full = force_full || interval_due;

        let packet = if use_full {
            SnapshotPacket::Full(current)
        } else {
            SnapshotPacket::delta_from(baseline.expect("非 full 时已有 baseline"), &current)
        };
        let bytes = match packet.encode_with_limit(self.config.max_snapshot_bytes) {
            Ok(bytes) => bytes,
            Err(SnapshotError::TooLarge { size, .. }) => {
                let max = self.config.max_snapshot_bytes;
                if let Some(client) = self.clients.get_mut(&player) {
                    client.backpressure_drops = client.backpressure_drops.saturating_add(1);
                    client.last_snapshot_bytes = size;
                }
                return Err(ReplicationError::Backpressure { size, max });
            }
            Err(error) => return Err(ReplicationError::Snapshot(error)),
        };
        let size = bytes.len();
        let is_full = use_full;
        if let Some(client) = self.clients.get_mut(&player) {
            if is_full {
                client.full_snapshots = client.full_snapshots.saturating_add(1);
            } else {
                client.delta_snapshots = client.delta_snapshots.saturating_add(1);
            }
            client.last_snapshot_bytes = size;
        }
        Ok(ClientSnapshot {
            player,
            packet,
            bytes,
            is_full,
        })
    }

    /// 为所有已注册客户端构建快照；单个失败不影响其他客户端。
    pub fn build_packets(&mut self) -> Vec<(PlayerId, Result<ClientSnapshot, ReplicationError>)> {
        let players: Vec<PlayerId> = self.clients.keys().copied().collect();
        players
            .into_iter()
            .map(|player| {
                let result = self.build_packet(player);
                (player, result)
            })
            .collect()
    }
}

/// 客户端复制统计。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplicationClientStats {
    /// 应用的快照数。
    pub frames_applied: u64,
    /// 全量快照数。
    pub full_snapshots: u64,
    /// delta 快照数。
    pub delta_snapshots: u64,
    /// 旧 tick 被拒绝次数。
    pub stale_rejected: u64,
    /// 检测到的 tick 跳号次数（丢包）。
    pub gaps: u64,
}

/// 应用快照的结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiveOutcome {
    /// 应用后的 tick。
    pub tick: u64,
    /// 是否全量。
    pub is_full: bool,
    /// 与上一应用 tick 之间是否跳号（丢包/服务器追赶）。
    pub gap: bool,
    /// 本次 delta 使用的 baseline tick（全量为 `None`）。
    pub baseline_tick: Option<u64>,
}

/// 客户端复制：接收/应用快照，维护最近 2 帧插值缓冲。
#[derive(Debug)]
pub struct ReplicationClient {
    baseline: Option<WorldSnapshot>,
    latest: Option<WorldSnapshot>,
    buffer: SnapshotBuffer,
    last_applied_tick: Option<u64>,
    stats: ReplicationClientStats,
}

impl Default for ReplicationClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicationClient {
    /// 默认缓冲（最近 2 帧）。
    pub fn new() -> Self {
        Self {
            baseline: None,
            latest: None,
            buffer: SnapshotBuffer::new(2),
            last_applied_tick: None,
            stats: ReplicationClientStats::default(),
        }
    }

    /// 指定缓冲帧数。
    pub fn with_buffer_capacity(capacity: usize) -> Self {
        Self {
            baseline: None,
            latest: None,
            buffer: SnapshotBuffer::new(capacity),
            last_applied_tick: None,
            stats: ReplicationClientStats::default(),
        }
    }

    /// 应用一个快照包；旧 tick/delta 不匹配返回可读错误。
    pub fn apply_packet(
        &mut self,
        packet: &SnapshotPacket,
    ) -> Result<ReceiveOutcome, SnapshotError> {
        let tick = packet.tick();
        if let Some(last) = self.last_applied_tick {
            if tick <= last {
                self.stats.stale_rejected = self.stats.stale_rejected.saturating_add(1);
                return Err(SnapshotError::StaleTick {
                    last,
                    received: tick,
                });
            }
        }
        let gap = self
            .last_applied_tick
            .map(|last| tick > last.saturating_add(1))
            .unwrap_or(false);
        let snapshot = packet.apply(self.baseline.as_ref())?;
        let baseline_tick = match packet {
            SnapshotPacket::Delta { base_tick, .. } => Some(*base_tick),
            SnapshotPacket::Full(_) => None,
        };
        self.baseline = Some(snapshot.clone());
        self.latest = Some(snapshot.clone());
        self.buffer.push(snapshot);
        self.last_applied_tick = Some(tick);
        self.stats.frames_applied = self.stats.frames_applied.saturating_add(1);
        if packet.is_full() {
            self.stats.full_snapshots = self.stats.full_snapshots.saturating_add(1);
        } else {
            self.stats.delta_snapshots = self.stats.delta_snapshots.saturating_add(1);
        }
        if gap {
            self.stats.gaps = self.stats.gaps.saturating_add(1);
        }
        Ok(ReceiveOutcome {
            tick,
            is_full: packet.is_full(),
            gap,
            baseline_tick,
        })
    }

    /// 解码并应用字节。
    pub fn apply_bytes(&mut self, bytes: &[u8]) -> Result<ReceiveOutcome, SnapshotError> {
        let packet = SnapshotPacket::decode(bytes)?;
        self.apply_packet(&packet)
    }

    /// 最近应用的快照。
    pub fn latest(&self) -> Option<&WorldSnapshot> {
        self.latest.as_ref()
    }

    /// 当前 baseline（最后一次应用后的完整世界）。
    pub fn baseline(&self) -> Option<&WorldSnapshot> {
        self.baseline.as_ref()
    }

    /// 插值缓冲。
    pub fn buffer(&self) -> &SnapshotBuffer {
        &self.buffer
    }

    /// 最近应用 tick。
    pub fn last_applied_tick(&self) -> Option<u64> {
        self.last_applied_tick
    }

    /// 待确认 tick（用于回传 ack）。
    pub fn pending_ack_tick(&self) -> Option<u64> {
        self.last_applied_tick
    }

    /// 按服务器时间插值。
    pub fn sample(&self, render_time_ms: f64) -> Option<WorldSnapshot> {
        self.buffer.sample(render_time_ms)
    }

    /// 统计。
    pub fn stats(&self) -> ReplicationClientStats {
        self.stats
    }

    /// 重置（服务器重连/强制全量时）。
    pub fn reset(&mut self) {
        self.baseline = None;
        self.latest = None;
        self.buffer.clear();
        self.last_applied_tick = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::EntityState;

    fn entity(id: u32, x: f32) -> EntityState {
        EntityState::new(id, [x, 0.0, 0.0], EntityState::IDENTITY_ROTATION, 0)
    }

    #[test]
    fn delta_after_ack_uses_baseline_and_applies_cleanly() {
        let mut server = ReplicationServer::new(ReplicationConfig {
            full_snapshot_interval_ticks: 100,
            ..ReplicationConfig::default()
        });
        let player = PlayerId::new(1);
        server.register_client(player);
        server.upsert_entity(entity(1, 1.0)).unwrap();
        server.upsert_entity(entity(2, 2.0)).unwrap();
        server.tick(0);

        let full = server.build_packet(player).unwrap();
        assert!(full.is_full);
        let mut client = ReplicationClient::new();
        client.apply_bytes(&full.bytes).unwrap();
        assert_eq!(client.latest().unwrap().tick, 1);
        assert!(server.ack(player, 1).unwrap());

        server.tick(16);
        server.upsert_entity(entity(1, 5.0)).unwrap();
        server.remove_entity(2);
        let delta = server.build_packet(player).unwrap();
        assert!(!delta.is_full);
        let outcome = client.apply_bytes(&delta.bytes).unwrap();
        assert_eq!(outcome.baseline_tick, Some(1));
        assert_eq!(outcome.tick, 2);
        let latest = client.latest().unwrap();
        assert_eq!(latest.entity_count(), 1);
        assert!((latest.entity(1).unwrap().position[0] - 5.0).abs() < 1e-6);
        assert!(latest.entity(2).is_none());
        assert_eq!(client.buffer().len(), 2);
    }

    #[test]
    fn backpressure_and_entity_limit_are_reported() {
        let mut server = ReplicationServer::new(ReplicationConfig {
            max_snapshot_bytes: 40,
            max_entities: 1,
            ..ReplicationConfig::default()
        });
        let player = PlayerId::new(1);
        server.register_client(player);
        assert!(matches!(server.upsert_entity(entity(1, 1.0)), Ok(())));
        assert!(matches!(
            server.upsert_entity(entity(2, 2.0)),
            Err(ReplicationError::EntityLimit { count: 2, max: 1 })
        ));
        server.tick(0);
        assert!(matches!(
            server.build_packet(player),
            Err(ReplicationError::Backpressure { max: 40, .. })
        ));
        assert_eq!(server.client_stats(player).unwrap().backpressure_drops, 1);
    }
}
