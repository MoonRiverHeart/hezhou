//! 通用实体标识。
//!
//! gameplay 组件之间通过 `CombatantId` 引用"战斗参与者"，
//! 集成层负责把它映射到 ECS Entity / 物理 RigidBody 等底层 ID。

use std::fmt;

/// 战斗参与者 ID（可对应 ECS Entity 或物理 body）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CombatantId(pub u64);

impl CombatantId {
    /// 用原始 u64 构造。
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// 取出原始 u64。
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl From<u64> for CombatantId {
    fn from(raw: u64) -> Self {
        Self(raw)
    }
}

impl From<CombatantId> for u64 {
    fn from(id: CombatantId) -> Self {
        id.0
    }
}

impl fmt::Display for CombatantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Combatant#{}", self.0)
    }
}
