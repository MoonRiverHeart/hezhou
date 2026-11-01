//! hezhou-gameplay — 3D 动作游戏通用能力组件。
//!
//! 本 crate 只提供"引擎能力"，不包含任何具体游戏规则、关卡或数值：
//!
//! | 模块 | 能力 |
//! |---|---|
//! | [`motor`] | `CharacterMotor`：移动/奔跑/跳跃/重力/地面吸附 |
//! | [`camera`] | `ThirdPersonCamera`、`LockOnState`：第三人称跟随、锁定目标 |
//! | [`combat`] | `ComboGraph`、`Hitbox`、`DamageEvent`、`Health`、`Dodge`、`Block`、`HeavyAttack` |
//! | [`animation_bridge`] | 与动画状态机的桥接 trait（不依赖具体动画实现） |
//! | [`query`] | 物理查询 trait（角色胶囊扫掠、相机扫掠、视线）+ 平地参考实现 |
//! | [`physics_adapter`] | 可选 `physics` feature：把查询 trait 接到 `hezhou-physics` |
//! | [`math`] | 能力组件共用的轻量向量数学 |
//!
//! 依赖方向：默认只依赖标准库；启用 `physics` feature 时才引入
//! `hezhou-core` / `hezhou-physics`。物理查询与动画状态通过 trait 注入，
//! 因此可以在没有渲染/物理/动画 crate 的情况下独立做确定性测试。
//!
//! 详细 API 与组合方式见 crate 根目录 `README.md`，可运行示例见
//! `examples/compose.rs`。

pub mod animation_bridge;
pub mod camera;
pub mod combat;
pub mod ids;
pub mod math;
pub mod motor;
pub mod query;

/// physics 适配层（启用 `physics` feature 时编译）。
#[cfg(feature = "physics")]
pub mod physics_adapter;

#[cfg(feature = "physics")]
pub use physics_adapter::{
    capsule_half_height, from_core_vec3, to_core_vec3, to_physics_hitbox, PhysicsHitQuery,
    PhysicsQuery,
};

pub use animation_bridge::{
    locomotion_state, ActionAnimator, ActionId, ActionState, ActionStateBridge, ActionTransition,
    AnimationEvent, AnimationEventSource, TransitionSpec,
};
pub use camera::{
    CameraLookInput, CameraOutput, LockOnConfig, LockOnState, LockOnTarget, LockOnUpdate,
    LockTargetCandidate, ThirdPersonCamera, ThirdPersonCameraConfig,
};
pub use combat::{
    resolve_damage, AttackDefinition, AttackId, AttackKind, AttackPhase, AttackWindow, Block,
    BlockAbsorb, BlockConfig, ComboAction, ComboConfig, ComboGraph, ComboInputResult, ComboState,
    ComboUpdate, DamageEvent, DamageKind, DamageOutcome, DamageResult, Dodge, DodgeConfig,
    DodgePhase, Health, HeavyAttack, HeavyAttackConfig, HeavyAttackRelease, HitContact,
    HitDetectionQuery, Hitbox, HitboxPose, HitboxRuntime, HitboxShape,
};
pub use ids::CombatantId;
pub use math::{
    angle_between, move_towards, rotate_towards_angle, smoothing_alpha, wrap_angle, Vec3,
};
pub use motor::{CharacterMotor, MotorConfig, MotorInput, MotorOutput};
pub use query::{
    CameraCollisionQuery, CameraSweep, CapsuleMoveResult, CharacterCollisionQuery, FlatGroundQuery,
    GroundHit, MoveContact, VisibilityQuery,
};

/// 常用类型的一次性导入。
pub mod prelude {
    pub use crate::animation_bridge::{
        locomotion_state, ActionAnimator, ActionId, ActionState, ActionStateBridge, TransitionSpec,
    };
    pub use crate::camera::{
        CameraLookInput, LockOnConfig, LockOnState, LockOnTarget, LockTargetCandidate,
        ThirdPersonCamera, ThirdPersonCameraConfig,
    };
    pub use crate::combat::{
        resolve_damage, AttackDefinition, AttackId, AttackKind, Block, BlockConfig, ComboConfig,
        ComboGraph, ComboInputResult, ComboState, DamageEvent, DamageKind, Dodge, DodgeConfig,
        Health, HeavyAttack, HeavyAttackConfig, HitContact, HitDetectionQuery, Hitbox, HitboxPose,
        HitboxRuntime, HitboxShape,
    };
    pub use crate::ids::CombatantId;
    pub use crate::math::Vec3;
    pub use crate::motor::{CharacterMotor, MotorConfig, MotorInput};
    pub use crate::query::{
        CameraCollisionQuery, CameraSweep, CharacterCollisionQuery, FlatGroundQuery,
        VisibilityQuery,
    };
}
