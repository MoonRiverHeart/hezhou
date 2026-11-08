//! hezhou-physics —— 自研动作物理（不引入任何第三方物理引擎，ADR-002）。
//!
//! ## 0.2 能力（保持兼容）
//! - 胶囊角色 vs 静态 Box/Sphere/Plane 的扫掠与穿透解决（`CharacterMotor`）；
//! - 固定时间步（`FixedTimestep`）；
//! - 碰撞层/掩码、传感器/触发器、命中盒（`Collider`/`layers`/`TriggerEvent`/`Hitbox`）；
//! - raycast / sphere cast / overlap / sweep 查询（`PhysicsWorld` + `query` 模块）；
//! - 调试描述数据（`debug` 模块）；
//! - 保留原有 AABB 刚体、射线、`CharacterController` API。
//!
//! ## 0.3 第一批能力
//! - 动态刚体：重力、线速度积分、质量/逆质量、线性/角阻尼、可选自动睡眠；
//! - 动态 vs 静态/动态接触响应：法向冲量 + 库仑摩擦 + 位置修正（线性）；
//! - CCD 基础：动态球/胶囊对静态目标的扫掠检测（默认关闭，可配置）；
//! - 距离关节（`DistanceJoint`）。
//!
//! ## 约定与限制（详见 README）
//! - `Box` 为轴对齐盒，碰撞用 AABB 近似（忽略刚体旋转）；
//! - `Capsule` 轴固定世界 Y 轴；`Plane` 法线由刚体旋转作用于 +Y；
//! - 接触响应只含线性冲量，无角冲量/惯性张量；
//! - 完整关节/约束求解、布娃娃、动态-动态 CCD、任意朝向碰撞体仍留后续版本。
//!
//! 坐标约定：`Box` 轴对齐；`Capsule` 轴固定为世界 Y 轴。

mod ccd;
pub mod character;
pub mod collider;
pub mod debug;
pub mod dynamics;
pub mod ffi;
pub mod fixed;
pub mod future;
pub mod hitbox;
pub(crate) mod intersect;
pub mod joint;
pub mod motor;
pub mod query;
pub mod raycast;
pub mod rigid_body;
pub mod world;

pub use character::CharacterController;
pub use collider::{layers, layers_interact, Collider, ColliderShape};
pub use debug::{
    collider_debug_color, DebugColor, DebugShape, COLOR_DYNAMIC, COLOR_HITBOX, COLOR_KINEMATIC,
    COLOR_SENSOR, COLOR_STATIC,
};
pub use dynamics::SolverConfig;
pub use fixed::FixedTimestep;
pub use hitbox::Hitbox;
pub use joint::DistanceJoint;
pub use motor::{CharacterMotor, MoveResult};
pub use query::{Hit, OverlapHit, QueryFilter};
pub use raycast::{Ray, RaycastResult};
pub use rigid_body::{RigidBody, RigidBodyType};
pub use world::{CollisionEvent, PhysicsWorld, TriggerEvent, TriggerEventKind};
