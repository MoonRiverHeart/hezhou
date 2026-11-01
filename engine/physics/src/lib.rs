//! hezhou-physics —— 自研动作物理（不引入任何第三方物理引擎，ADR-002）。
//!
//! 0.2 能力范围：
//! - 胶囊角色 vs 静态 Box/Sphere/Plane 的扫掠与穿透解决（`CharacterMotor`）；
//! - 固定时间步（`FixedTimestep`）；
//! - 碰撞层/掩码、传感器/触发器、命中盒（`Collider`/`layers`/`TriggerEvent`/`Hitbox`）；
//! - raycast / sphere cast / overlap / sweep 查询（`PhysicsWorld` + `query` 模块）；
//! - 调试描述数据（`debug` 模块）；
//! - 保留原有 AABB 刚体、射线、`CharacterController` API。
//!
//! 0.3 预留（本版本不实现）：关节/约束、CCD、布娃娃、任意朝向碰撞体、
//! 完整动态刚体堆叠，见 [`future`] 与各模块 `TODO(0.3)` 注释。
//!
//! 坐标约定：`Box` 轴对齐；`Capsule` 轴固定为世界 Y 轴；`Plane` 法线由刚体旋转作用于 +Y 得到。

pub mod character;
pub mod collider;
pub mod debug;
pub mod ffi;
pub mod fixed;
pub mod future;
pub mod hitbox;
pub(crate) mod intersect;
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
pub use fixed::FixedTimestep;
pub use hitbox::Hitbox;
pub use motor::{CharacterMotor, MoveResult};
pub use query::{Hit, OverlapHit, QueryFilter};
pub use raycast::{Ray, RaycastResult};
pub use rigid_body::{RigidBody, RigidBodyType};
pub use world::{CollisionEvent, PhysicsWorld, TriggerEvent, TriggerEventKind};
