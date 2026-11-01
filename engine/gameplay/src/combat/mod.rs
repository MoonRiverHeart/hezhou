//! 战斗能力组件。
//!
//! 全部为纯数据/状态机，不绑定渲染、物理或具体动画实现：
//!
//! | 模块 | 能力 |
//! |---|---|
//! | [`combo`] | 攻击定义、连招图、连招窗口状态机 |
//! | [`hitbox`] | 命中盒数据、命中查询 trait、命中去重 |
//! | [`damage`] | 伤害事件、结算管线（格挡/闪避/死亡） |
//! | [`health`] | 生命值与死亡判定 |
//! | [`dodge`] | 闪避与无敌帧 |
//! | [`block`] | 格挡减伤、护盾消耗与破防 |
//! | [`heavy_attack`] | 重攻击蓄力与伤害倍率 |
//!
//! 典型组合顺序：
//! `ComboState` 推进时间轴 → 命中窗口生效时用 `HitboxRuntime` + `HitDetectionQuery`
//! 查询命中 → `AttackDefinition::damage_events` 生成 `DamageEvent` →
//! `resolve_damage` 结合 `Health` / `Block` / `Dodge` 结算。

pub mod block;
pub mod combo;
pub mod damage;
pub mod dodge;
pub mod health;
pub mod heavy_attack;
pub mod hitbox;

pub use block::{Block, BlockAbsorb, BlockConfig};
pub use combo::{
    AttackDefinition, AttackId, AttackKind, AttackPhase, AttackWindow, ComboAction, ComboConfig,
    ComboGraph, ComboInputResult, ComboState, ComboUpdate,
};
pub use damage::{resolve_damage, DamageEvent, DamageKind, DamageOutcome, DamageResult};
pub use dodge::{Dodge, DodgeConfig, DodgePhase};
pub use health::Health;
pub use heavy_attack::{HeavyAttack, HeavyAttackConfig, HeavyAttackRelease};
pub use hitbox::{HitContact, HitDetectionQuery, Hitbox, HitboxPose, HitboxRuntime, HitboxShape};
