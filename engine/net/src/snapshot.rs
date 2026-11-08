//! 世界快照：实体状态、二进制全量/增量编码、客户端插值缓冲。
//!
//! 同步模型见 `README.md`：服务器权威、UDP 不可靠、无预测/回滚；快照是"全量 + 变化实体
//! delta"的基线确认模型：
//!
//! ```text
//! ReplicationServer (authoritative world)
//!   ├─ tick() 推进 + 记录历史 WorldSnapshot
//!   ├─ build_packet(player) -> SnapshotPacket::Full / Delta
//!   └─ ack(player, tick) 确认 baseline
//!
//! SnapshotPacket bytes -> Message::Snapshot.payload -> UDP
//!
//! ReplicationClient
//!   ├─ apply_packet()（旧 tick 拒绝、delta 需要匹配 baseline）
//!   └─ SnapshotBuffer 保存最近 2 帧，按 render time 插值 position/rotation
//! ```
//!
//! ## 二进制格式（little-endian，长度前缀 + 版本 + 上限）
//!
//! ```text
//! [u32 total_len LE][u8 version=1][u8 kind(0=full,1=delta)]
//! [u64 base_tick][u64 tick][u64 server_time_ms]
//! [u16 entity_count][u16 deleted_count]
//! entity_count × [u32 id][f32 x3 position][f32 x4 rotation][u32 flags]  // 36 bytes
//! deleted_count × [u32 id]
//! ```
//!
//! - 全量：`base_tick=0`、`deleted_count=0`；
//! - delta：`base_tick < tick`，`entity_count` 只包含变化实体，`deleted_count` 为删除列表；
//! - 上限：默认 [`MAX_SNAPSHOT_BYTES`]（64 KiB）与 [`MAX_SNAPSHOT_ENTITIES`]；UDP 实际
//!   单包还受 [`crate::message::MAX_MESSAGE_BYTES`] 限制，复制层默认 1024 字节并做背压；
//! - 解码严格：长度/版本/类型/重复 ID/截断/尾部多余字节均报错。

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::error::SnapshotError;

/// 快照格式版本。
pub const SNAPSHOT_VERSION: u8 = 1;
/// 默认快照最大字节数（编码层保护；UDP 复制层另有更小上限）。
pub const MAX_SNAPSHOT_BYTES: usize = 64 * 1024;
/// 默认最大实体数。
pub const MAX_SNAPSHOT_ENTITIES: usize = 512;
/// 长度前缀字节数（u32 LE）。
pub const SNAPSHOT_LENGTH_PREFIX_BYTES: usize = 4;
/// 头部字节数（含长度前缀）。
pub const SNAPSHOT_HEADER_BYTES: usize = 34;
/// 单个实体状态字节数：id(4)+position(12)+rotation(16)+flags(4)。
pub const SNAPSHOT_ENTITY_BYTES: usize = 36;
/// 默认客户端快照缓冲帧数（最近 2 帧用于插值）。
pub const DEFAULT_SNAPSHOT_BUFFER_CAPACITY: usize = 2;

const KIND_FULL: u8 = 0;
const KIND_DELTA: u8 = 1;

/// 单个实体在网络上的状态。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityState {
    /// 实体 ID（服务器分配，进程内唯一）。
    pub id: u32,
    /// 世界坐标位置。
    pub position: [f32; 3],
    /// 四元数旋转 `[x, y, z, w]`。
    pub rotation: [f32; 4],
    /// 位标志（上层自定义，例如可见/动画状态）。
    pub flags: u32,
}

impl EntityState {
    /// 构造。
    pub const fn new(id: u32, position: [f32; 3], rotation: [f32; 4], flags: u32) -> Self {
        Self {
            id,
            position,
            rotation,
            flags,
        }
    }

    /// 默认旋转（单位四元数）。
    pub const IDENTITY_ROTATION: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

    /// 按 `t` 插值位置（lerp）与旋转（slerp）；flags 取较新的 `other`。
    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            id: self.id,
            position: [
                lerp(self.position[0], other.position[0], t),
                lerp(self.position[1], other.position[1], t),
                lerp(self.position[2], other.position[2], t),
            ],
            rotation: quat_slerp(self.rotation, other.rotation, t),
            flags: other.flags,
        }
    }
}

impl fmt::Display for EntityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Entity#{}(pos=({:.2},{:.2},{:.2}), flags={})",
            self.id, self.position[0], self.position[1], self.position[2], self.flags
        )
    }
}

/// 某一 tick 的完整世界状态。
#[derive(Clone, Debug, PartialEq)]
pub struct WorldSnapshot {
    /// 服务器 tick。
    pub tick: u64,
    /// 服务器时间（毫秒），用于客户端插值。
    pub server_time_ms: u64,
    /// 实体列表（`new` 会按 id 排序）。
    pub entities: Vec<EntityState>,
}

impl WorldSnapshot {
    /// 构造并按实体 id 排序（不自动去重；重复 id 由编码/应用校验拒绝）。
    pub fn new(tick: u64, server_time_ms: u64, mut entities: Vec<EntityState>) -> Self {
        entities.sort_by_key(|entity| entity.id);
        Self {
            tick,
            server_time_ms,
            entities,
        }
    }

    /// 实体数量。
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// 按 id 查询实体。
    pub fn entity(&self, id: u32) -> Option<&EntityState> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    /// 内部映射（id → 实体）。
    fn entity_map(&self) -> BTreeMap<u32, EntityState> {
        self.entities
            .iter()
            .map(|entity| (entity.id, *entity))
            .collect()
    }

    fn validate_unique(&self) -> Result<(), SnapshotError> {
        let mut seen = BTreeSet::new();
        for entity in &self.entities {
            if !seen.insert(entity.id) {
                return Err(SnapshotError::DuplicateEntity(entity.id));
            }
        }
        Ok(())
    }
}

/// 一个快照包：全量或基于 baseline 的 delta。
#[derive(Clone, Debug, PartialEq)]
pub enum SnapshotPacket {
    /// 全量世界快照。
    Full(WorldSnapshot),
    /// 相对 `base_tick` 基线：只传变化实体 + 删除列表。
    Delta {
        /// baseline tick。
        base_tick: u64,
        /// 新 tick。
        tick: u64,
        /// 服务器时间（毫秒）。
        server_time_ms: u64,
        /// 变化/新增实体。
        updated: Vec<EntityState>,
        /// 删除的实体 id。
        deleted: Vec<u32>,
    },
}

impl SnapshotPacket {
    /// 快照 tick。
    pub fn tick(&self) -> u64 {
        match self {
            Self::Full(snapshot) => snapshot.tick,
            Self::Delta { tick, .. } => *tick,
        }
    }

    /// 是否全量。
    pub fn is_full(&self) -> bool {
        matches!(self, Self::Full(_))
    }

    /// 由 baseline 与当前快照构造 delta（自动计算变化实体与删除列表）。
    pub fn delta_from(baseline: &WorldSnapshot, current: &WorldSnapshot) -> Self {
        let base_map = baseline.entity_map();
        let current_map = current.entity_map();
        let updated: Vec<EntityState> = current
            .entities
            .iter()
            .copied()
            .filter(|entity| base_map.get(&entity.id) != Some(entity))
            .collect();
        let deleted: Vec<u32> = base_map
            .keys()
            .copied()
            .filter(|id| !current_map.contains_key(id))
            .collect();
        Self::Delta {
            base_tick: baseline.tick,
            tick: current.tick,
            server_time_ms: current.server_time_ms,
            updated,
            deleted,
        }
    }

    /// 编码为默认上限内的字节。
    pub fn encode(&self) -> Result<Vec<u8>, SnapshotError> {
        self.encode_with_limit(MAX_SNAPSHOT_BYTES)
    }

    /// 编码并检查总大小上限。
    pub fn encode_with_limit(&self, max_bytes: usize) -> Result<Vec<u8>, SnapshotError> {
        let (kind, base_tick, tick, server_time_ms, entities, deleted): (
            u8,
            u64,
            u64,
            u64,
            &[EntityState],
            &[u32],
        ) = match self {
            Self::Full(snapshot) => {
                snapshot.validate_unique()?;
                (
                    KIND_FULL,
                    0,
                    snapshot.tick,
                    snapshot.server_time_ms,
                    &snapshot.entities,
                    &[],
                )
            }
            Self::Delta {
                base_tick,
                tick,
                server_time_ms,
                updated,
                deleted,
            } => {
                if *base_tick >= *tick {
                    return Err(SnapshotError::TickNotNewer {
                        base: *base_tick,
                        tick: *tick,
                    });
                }
                let mut seen = BTreeSet::new();
                for entity in updated {
                    if !seen.insert(entity.id) {
                        return Err(SnapshotError::DuplicateEntity(entity.id));
                    }
                }
                (
                    KIND_DELTA,
                    *base_tick,
                    *tick,
                    *server_time_ms,
                    updated,
                    deleted,
                )
            }
        };
        if entities.len() > MAX_SNAPSHOT_ENTITIES {
            return Err(SnapshotError::EntityLimit {
                count: entities.len(),
                max: MAX_SNAPSHOT_ENTITIES,
            });
        }
        let size =
            SNAPSHOT_HEADER_BYTES + entities.len() * SNAPSHOT_ENTITY_BYTES + deleted.len() * 4;
        if size > max_bytes {
            return Err(SnapshotError::TooLarge {
                size,
                max: max_bytes,
            });
        }
        if entities.len() > u16::MAX as usize || deleted.len() > u16::MAX as usize {
            return Err(SnapshotError::EntityLimit {
                count: entities.len(),
                max: u16::MAX as usize,
            });
        }

        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(&(size as u32).to_le_bytes());
        bytes.push(SNAPSHOT_VERSION);
        bytes.push(kind);
        bytes.extend_from_slice(&base_tick.to_le_bytes());
        bytes.extend_from_slice(&tick.to_le_bytes());
        bytes.extend_from_slice(&server_time_ms.to_le_bytes());
        bytes.extend_from_slice(&(entities.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(deleted.len() as u16).to_le_bytes());
        for entity in entities {
            bytes.extend_from_slice(&entity.id.to_le_bytes());
            for value in entity.position {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            for value in entity.rotation {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes.extend_from_slice(&entity.flags.to_le_bytes());
        }
        for id in deleted {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        Ok(bytes)
    }

    /// 解码默认上限内的字节。
    pub fn decode(bytes: &[u8]) -> Result<Self, SnapshotError> {
        Self::decode_with_limit(bytes, MAX_SNAPSHOT_BYTES)
    }

    /// 解码并检查总大小上限。
    pub fn decode_with_limit(bytes: &[u8], max_bytes: usize) -> Result<Self, SnapshotError> {
        if bytes.len() > max_bytes {
            return Err(SnapshotError::TooLarge {
                size: bytes.len(),
                max: max_bytes,
            });
        }
        if bytes.len() < SNAPSHOT_HEADER_BYTES {
            return Err(SnapshotError::Truncated);
        }
        let declared = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if declared != bytes.len() {
            return Err(SnapshotError::InvalidLength {
                declared,
                actual: bytes.len(),
            });
        }
        let version = bytes[4];
        if version != SNAPSHOT_VERSION {
            return Err(SnapshotError::UnsupportedVersion(version));
        }
        let kind = bytes[5];
        let base_tick = read_u64(bytes, 6)?;
        let tick = read_u64(bytes, 14)?;
        let server_time_ms = read_u64(bytes, 22)?;
        let entity_count = read_u16(bytes, 30)? as usize;
        let deleted_count = read_u16(bytes, 32)? as usize;
        if entity_count > MAX_SNAPSHOT_ENTITIES {
            return Err(SnapshotError::EntityLimit {
                count: entity_count,
                max: MAX_SNAPSHOT_ENTITIES,
            });
        }
        let expected =
            SNAPSHOT_HEADER_BYTES + entity_count * SNAPSHOT_ENTITY_BYTES + deleted_count * 4;
        if bytes.len() != expected {
            return Err(SnapshotError::InvalidLength {
                declared,
                actual: bytes.len(),
            });
        }

        let mut offset = SNAPSHOT_HEADER_BYTES;
        let mut entities = Vec::with_capacity(entity_count);
        let mut seen = BTreeSet::new();
        for _ in 0..entity_count {
            let id = read_u32(bytes, offset)?;
            if !seen.insert(id) {
                return Err(SnapshotError::DuplicateEntity(id));
            }
            offset += 4;
            let position = [
                read_f32(bytes, offset)?,
                read_f32(bytes, offset + 4)?,
                read_f32(bytes, offset + 8)?,
            ];
            offset += 12;
            let rotation = [
                read_f32(bytes, offset)?,
                read_f32(bytes, offset + 4)?,
                read_f32(bytes, offset + 8)?,
                read_f32(bytes, offset + 12)?,
            ];
            offset += 16;
            let flags = read_u32(bytes, offset)?;
            offset += 4;
            entities.push(EntityState {
                id,
                position,
                rotation,
                flags,
            });
        }
        let mut deleted = Vec::with_capacity(deleted_count);
        for _ in 0..deleted_count {
            deleted.push(read_u32(bytes, offset)?);
            offset += 4;
        }

        match kind {
            KIND_FULL => Ok(Self::Full(WorldSnapshot::new(
                tick,
                server_time_ms,
                entities,
            ))),
            KIND_DELTA => {
                if base_tick >= tick {
                    return Err(SnapshotError::TickNotNewer {
                        base: base_tick,
                        tick,
                    });
                }
                entities.sort_by_key(|entity| entity.id);
                deleted.sort_unstable();
                Ok(Self::Delta {
                    base_tick,
                    tick,
                    server_time_ms,
                    updated: entities,
                    deleted,
                })
            }
            other => Err(SnapshotError::UnknownKind(other)),
        }
    }

    /// 应用快照：全量忽略 baseline；delta 必须匹配 baseline tick。
    pub fn apply(&self, baseline: Option<&WorldSnapshot>) -> Result<WorldSnapshot, SnapshotError> {
        match self {
            Self::Full(snapshot) => {
                snapshot.validate_unique()?;
                Ok(snapshot.clone())
            }
            Self::Delta {
                base_tick,
                tick,
                server_time_ms,
                updated,
                deleted,
            } => {
                let baseline = baseline.ok_or(SnapshotError::MissingBaseline)?;
                if baseline.tick != *base_tick {
                    return Err(SnapshotError::BaseTickMismatch {
                        expected: *base_tick,
                        actual: baseline.tick,
                    });
                }
                if *base_tick >= *tick {
                    return Err(SnapshotError::TickNotNewer {
                        base: *base_tick,
                        tick: *tick,
                    });
                }
                let mut map = baseline.entity_map();
                for id in deleted {
                    map.remove(id);
                }
                let mut seen = BTreeSet::new();
                for entity in updated {
                    if !seen.insert(entity.id) {
                        return Err(SnapshotError::DuplicateEntity(entity.id));
                    }
                    map.insert(entity.id, *entity);
                }
                let mut entities: Vec<EntityState> = map.into_values().collect();
                entities.sort_by_key(|entity| entity.id);
                Ok(WorldSnapshot::new(*tick, *server_time_ms, entities))
            }
        }
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, SnapshotError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(SnapshotError::Truncated)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, SnapshotError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(SnapshotError::Truncated)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, SnapshotError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(SnapshotError::Truncated)?;
    let mut value = [0u8; 8];
    value.copy_from_slice(raw);
    Ok(u64::from_le_bytes(value))
}

fn read_f32(bytes: &[u8], offset: usize) -> Result<f32, SnapshotError> {
    Ok(f32::from_bits(read_u32(bytes, offset)?))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 四元数球面插值（slerp），输入/输出为 `[x,y,z,w]`。
///
/// - 自动选取最短路径（dot < 0 时翻转）；
/// - 近平行时退化为归一化 lerp；
/// - 非法/零四元数时回退到 `a` 的归一化值（无法归一化则返回 `a`）。
pub fn quat_slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    let Some(a) = normalize_quat(a) else {
        return a;
    };
    let Some(mut b) = normalize_quat(b) else {
        return a;
    };
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }
    if dot > 0.999_5 {
        return normalize_quat([
            lerp(a[0], b[0], t),
            lerp(a[1], b[1], t),
            lerp(a[2], b[2], t),
            lerp(a[3], b[3], t),
        ])
        .unwrap_or(a);
    }
    let theta_0 = dot.clamp(-1.0, 1.0).acos();
    let theta = theta_0 * t;
    let sin_theta = theta.sin();
    let sin_theta_0 = theta_0.sin();
    if sin_theta_0.abs() < f32::EPSILON {
        return a;
    }
    let s0 = ((1.0 - t) * theta_0).sin() / sin_theta_0;
    let s1 = sin_theta / sin_theta_0;
    normalize_quat([
        a[0] * s0 + b[0] * s1,
        a[1] * s0 + b[1] * s1,
        a[2] * s0 + b[2] * s1,
        a[3] * s0 + b[3] * s1,
    ])
    .unwrap_or(a)
}

fn normalize_quat(value: [f32; 4]) -> Option<[f32; 4]> {
    let length_squared =
        value[0] * value[0] + value[1] * value[1] + value[2] * value[2] + value[3] * value[3];
    if !length_squared.is_finite() || length_squared <= f32::EPSILON {
        return None;
    }
    let inverse = length_squared.sqrt().recip();
    Some([
        value[0] * inverse,
        value[1] * inverse,
        value[2] * inverse,
        value[3] * inverse,
    ])
}

/// 客户端最近快照缓冲：保存最近 N 帧，按渲染时间插值。
#[derive(Clone, Debug)]
pub struct SnapshotBuffer {
    snapshots: VecDeque<WorldSnapshot>,
    capacity: usize,
}

impl Default for SnapshotBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_SNAPSHOT_BUFFER_CAPACITY)
    }
}

impl SnapshotBuffer {
    /// 创建缓冲（至少 1 帧）。
    pub fn new(capacity: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    /// 容量。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 当前帧数。
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// 最新帧。
    pub fn latest(&self) -> Option<&WorldSnapshot> {
        self.snapshots.back()
    }

    /// 最旧帧。
    pub fn oldest(&self) -> Option<&WorldSnapshot> {
        self.snapshots.front()
    }

    /// 按 tick 取帧。
    pub fn snapshot(&self, tick: u64) -> Option<&WorldSnapshot> {
        self.snapshots.iter().find(|snapshot| snapshot.tick == tick)
    }

    /// 是否包含某 tick。
    pub fn contains(&self, tick: u64) -> bool {
        self.snapshot(tick).is_some()
    }

    /// 清空。
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }

    /// 推入新快照；tick 不新于最新帧时拒绝并返回 false。
    pub fn push(&mut self, snapshot: WorldSnapshot) -> bool {
        if let Some(latest) = self.snapshots.back() {
            if snapshot.tick <= latest.tick {
                return false;
            }
        }
        self.snapshots.push_back(snapshot);
        while self.snapshots.len() > self.capacity {
            self.snapshots.pop_front();
        }
        true
    }

    /// 按渲染时间（服务器毫秒）插值。
    ///
    /// - 空缓冲返回 `None`；
    /// - 早于最旧帧 → 返回最旧帧（不反向外推）；
    /// - 晚于最新帧 → 返回最新帧（不预测，丢包/追赶回退到最新）；
    /// - 位于两帧之间 → position lerp + rotation slerp；只在一侧存在的实体取存在的一侧。
    pub fn sample(&self, render_time_ms: f64) -> Option<WorldSnapshot> {
        if self.snapshots.is_empty() {
            return None;
        }
        let first = self.snapshots.front().expect("非空");
        let last = self.snapshots.back().expect("非空");
        if !render_time_ms.is_finite()
            || render_time_ms <= first.server_time_ms as f64
            || self.snapshots.len() == 1
        {
            return Some(first.clone());
        }
        if render_time_ms >= last.server_time_ms as f64 {
            return Some(last.clone());
        }
        for index in 0..self.snapshots.len().saturating_sub(1) {
            let older = self.snapshots.get(index).expect("索引有效");
            let newer = self.snapshots.get(index + 1).expect("索引有效");
            let start = older.server_time_ms as f64;
            let end = newer.server_time_ms as f64;
            if render_time_ms >= start && render_time_ms <= end {
                let t = if end > start {
                    ((render_time_ms - start) / (end - start)) as f32
                } else {
                    0.0
                };
                return Some(interpolate_snapshots(older, newer, t));
            }
        }
        Some(last.clone())
    }
}

fn interpolate_snapshots(older: &WorldSnapshot, newer: &WorldSnapshot, t: f32) -> WorldSnapshot {
    let newer_map = newer.entity_map();
    let older_map = older.entity_map();
    let mut entities = Vec::with_capacity(older_map.len().max(newer_map.len()));
    for (id, old) in &older_map {
        match newer_map.get(id) {
            Some(new) => entities.push(old.interpolate(new, t)),
            None => entities.push(*old),
        }
    }
    for (id, new) in &newer_map {
        if !older_map.contains_key(id) {
            entities.push(*new);
        }
    }
    WorldSnapshot::new(newer.tick, newer.server_time_ms, entities)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: u32, x: f32, rotation: [f32; 4]) -> EntityState {
        EntityState::new(id, [x, 0.0, 0.0], rotation, 1)
    }

    fn full(tick: u64, entities: Vec<EntityState>) -> WorldSnapshot {
        WorldSnapshot::new(tick, tick * 50, entities)
    }

    #[test]
    fn full_snapshot_roundtrip() {
        let snapshot = full(
            1,
            vec![
                entity(1, 1.0, EntityState::IDENTITY_ROTATION),
                entity(2, 2.0, EntityState::IDENTITY_ROTATION),
            ],
        );
        let packet = SnapshotPacket::Full(snapshot.clone());
        let bytes = packet.encode().unwrap();
        assert_eq!(&bytes[..4], &(bytes.len() as u32).to_le_bytes());
        assert_eq!(SnapshotPacket::decode(&bytes).unwrap(), packet);
        let applied = packet.apply(None).unwrap();
        assert_eq!(applied, snapshot);
    }

    #[test]
    fn delta_only_contains_changes_and_deletions() {
        let baseline = full(
            10,
            vec![
                entity(1, 1.0, EntityState::IDENTITY_ROTATION),
                entity(2, 2.0, EntityState::IDENTITY_ROTATION),
                entity(3, 3.0, EntityState::IDENTITY_ROTATION),
            ],
        );
        let current = full(
            11,
            vec![
                entity(1, 1.5, EntityState::IDENTITY_ROTATION),
                entity(3, 3.0, EntityState::IDENTITY_ROTATION),
                entity(4, 4.0, EntityState::IDENTITY_ROTATION),
            ],
        );
        let delta = SnapshotPacket::delta_from(&baseline, &current);
        let SnapshotPacket::Delta {
            base_tick,
            updated,
            deleted,
            ..
        } = &delta
        else {
            panic!("应为 delta");
        };
        assert_eq!(*base_tick, 10);
        assert_eq!(updated.len(), 2, "只传变化/新增实体");
        assert_eq!(deleted, &vec![2], "实体 2 被删除");
        let bytes = delta.encode().unwrap();
        let decoded = SnapshotPacket::decode(&bytes).unwrap();
        assert_eq!(decoded, delta);
        let applied = decoded.apply(Some(&baseline)).unwrap();
        assert_eq!(applied, current);
    }

    #[test]
    fn delta_requires_matching_baseline() {
        let baseline = full(5, vec![entity(1, 1.0, EntityState::IDENTITY_ROTATION)]);
        let current = full(6, vec![entity(1, 2.0, EntityState::IDENTITY_ROTATION)]);
        let delta = SnapshotPacket::delta_from(&baseline, &current);
        assert_eq!(delta.apply(None), Err(SnapshotError::MissingBaseline));
        let wrong = full(4, vec![entity(1, 1.0, EntityState::IDENTITY_ROTATION)]);
        assert!(matches!(
            delta.apply(Some(&wrong)),
            Err(SnapshotError::BaseTickMismatch {
                expected: 5,
                actual: 4
            })
        ));
    }

    #[test]
    fn encode_limits_and_invalid_delta_are_rejected() {
        let snapshot = full(1, vec![entity(1, 1.0, EntityState::IDENTITY_ROTATION)]);
        assert!(matches!(
            SnapshotPacket::Full(snapshot.clone()).encode_with_limit(10),
            Err(SnapshotError::TooLarge { .. })
        ));
        let invalid = SnapshotPacket::Delta {
            base_tick: 5,
            tick: 5,
            server_time_ms: 0,
            updated: vec![],
            deleted: vec![],
        };
        assert!(matches!(
            invalid.encode(),
            Err(SnapshotError::TickNotNewer { base: 5, tick: 5 })
        ));
    }

    #[test]
    fn decode_rejects_corrupt_or_duplicate_data() {
        let packet = SnapshotPacket::Full(full(
            1,
            vec![entity(1, 1.0, EntityState::IDENTITY_ROTATION)],
        ));
        let bytes = packet.encode().unwrap();
        // 长度前缀错误。
        let mut bad_length = bytes.clone();
        bad_length[..4].copy_from_slice(&(bytes.len() as u32 + 1).to_le_bytes());
        assert!(matches!(
            SnapshotPacket::decode(&bad_length),
            Err(SnapshotError::InvalidLength { .. })
        ));
        // 版本错误。
        let mut bad_version = bytes.clone();
        bad_version[4] = 99;
        assert!(matches!(
            SnapshotPacket::decode(&bad_version),
            Err(SnapshotError::UnsupportedVersion(99))
        ));
        // 重复实体 id。
        let duplicated = SnapshotPacket::Full(WorldSnapshot::new(
            2,
            0,
            vec![
                entity(1, 1.0, EntityState::IDENTITY_ROTATION),
                entity(1, 2.0, EntityState::IDENTITY_ROTATION),
            ],
        ));
        assert!(matches!(
            duplicated.encode(),
            Err(SnapshotError::DuplicateEntity(1))
        ));
        // 截断。
        assert!(matches!(
            SnapshotPacket::decode(&bytes[..bytes.len() - 1]),
            Err(SnapshotError::InvalidLength { .. })
        ));
    }

    #[test]
    fn interpolation_lerps_position_and_slerps_rotation() {
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let older = full(
            1,
            vec![EntityState::new(
                1,
                [0.0, 0.0, 0.0],
                EntityState::IDENTITY_ROTATION,
                0,
            )],
        );
        let newer = full(
            2,
            vec![EntityState::new(
                1,
                [10.0, 20.0, -30.0],
                [0.0, half, 0.0, half],
                0,
            )],
        );
        let mut buffer = SnapshotBuffer::new(2);
        buffer.push(older.clone());
        buffer.push(newer.clone());
        let sampled = buffer.sample(older.server_time_ms as f64 + 25.0).unwrap();
        let entity = sampled.entity(1).unwrap();
        assert!((entity.position[0] - 5.0).abs() < 1e-5);
        assert!((entity.position[1] - 10.0).abs() < 1e-5);
        assert!((entity.position[2] - -15.0).abs() < 1e-5);
        // 45° 旋转的 y/w 分量。
        assert!((entity.rotation[1] - 0.382_683_43).abs() < 1e-4);
        assert!((entity.rotation[3] - 0.923_879_5).abs() < 1e-4);
        let length: f32 = entity.rotation.iter().map(|value| value * value).sum();
        assert!((length - 1.0).abs() < 1e-4);
    }

    #[test]
    fn buffer_rejects_old_frames_and_falls_back_on_gaps() {
        let mut buffer = SnapshotBuffer::new(2);
        assert!(buffer.sample(0.0).is_none());
        assert!(buffer.push(full(
            1,
            vec![entity(1, 1.0, EntityState::IDENTITY_ROTATION)]
        )));
        assert!(!buffer.push(full(
            1,
            vec![entity(1, 2.0, EntityState::IDENTITY_ROTATION)]
        )));
        // 丢包：直接推入 tick 3，缓冲只保留 tick 3（容量 2，tick1 + tick3）。
        assert!(buffer.push(full(
            3,
            vec![entity(1, 3.0, EntityState::IDENTITY_ROTATION)]
        )));
        assert_eq!(buffer.len(), 2);
        // 渲染时间在 tick1/3 之间：仍可插值。
        let sampled = buffer.sample(100.0).unwrap();
        assert!(sampled.entity(1).is_some());
        // 早于最旧/晚于最新 → 钳制到端点（无外推预测）。
        assert_eq!(buffer.sample(0.0).unwrap().tick, 1);
        assert_eq!(buffer.sample(10_000.0).unwrap().tick, 3);
        // 只保留最新帧时（新缓冲）也回退到最新。
        let mut latest_only = SnapshotBuffer::new(2);
        latest_only.push(full(
            9,
            vec![entity(1, 9.0, EntityState::IDENTITY_ROTATION)],
        ));
        assert_eq!(latest_only.sample(0.0).unwrap().tick, 9);
    }
}
