//! physics 适配层（`feature = "physics"`）。
//!
//! 把 gameplay 的查询 trait 接到 `hezhou-physics` 的定稿 API 上：
//!
//! | gameplay trait | physics API |
//! |---|---|
//! | [`CharacterCollisionQuery::ground_probe`] | `PhysicsWorld::ground_probe` |
//! | [`CharacterCollisionQuery::move_capsule`] | `PhysicsWorld::move_capsule`（无重力/无台阶/无吸附，由 gameplay 负责手感） |
//! | [`CameraCollisionQuery::sweep_camera`] | `PhysicsWorld::sphere_cast` |
//! | [`VisibilityQuery::has_line_of_sight`] | `PhysicsWorld::raycast_filtered` |
//! | [`HitDetectionQuery::query_hitbox`] | `physics::Hitbox::overlaps` |
//!
//! 使用示例：
//!
//! ```ignore
//! use hezhou_gameplay::physics_adapter::{PhysicsHitQuery, PhysicsQuery};
//!
//! let physics_query = PhysicsQuery::new(&physics_world)
//!     .with_layer_mask(hezhou_physics::layers::WORLD | hezhou_physics::layers::ENEMY)
//!     .exclude_body(player_body_id);
//! let output = motor.step(dt, &input, &physics_query);
//!
//! let hit_query = PhysicsHitQuery::new(&physics_world)
//!     .with_layer_mask(hezhou_physics::layers::ENEMY);
//! ```
//!
//! 已知限制（physics 0.2 均为轴对齐形状）：
//! - 旋转后的 Box/胶囊命中盒只能退化为"世界中心 + 轴对齐尺寸"，斜向命中盒会偏保守；
//! - 胶囊轴固定世界 Y 轴，适用于直立角色；
//! - 命中盒的 `HitboxPose::forward` 只用于计算偏移，不参与形状旋转。
//!
//! 本模块只在启用 `physics` feature 时编译，gameplay 默认仍不依赖 physics。

use hezhou_core::math::Vec3 as CoreVec3;
use hezhou_physics::{Hitbox as PhysicsHitbox, PhysicsWorld, QueryFilter, Ray};

use crate::combat::hitbox::{HitContact, HitDetectionQuery, Hitbox, HitboxPose};
use crate::ids::CombatantId;
use crate::math::Vec3;
use crate::query::{
    CameraCollisionQuery, CameraSweep, CapsuleMoveResult, CharacterCollisionQuery, GroundHit,
    MoveContact, VisibilityQuery,
};

/// gameplay `Vec3` → 引擎/core `Vec3`。
pub fn to_core_vec3(value: Vec3) -> CoreVec3 {
    CoreVec3::new(value.x, value.y, value.z)
}

/// 引擎/core `Vec3` → gameplay `Vec3`。
pub fn from_core_vec3(value: CoreVec3) -> Vec3 {
    Vec3::new(value.x, value.y, value.z)
}

impl From<Vec3> for CoreVec3 {
    fn from(value: Vec3) -> Self {
        CoreVec3::new(value.x, value.y, value.z)
    }
}

impl From<CoreVec3> for Vec3 {
    fn from(value: CoreVec3) -> Self {
        Vec3::new(value.x, value.y, value.z)
    }
}

/// 由总高与半径求 physics 胶囊圆柱段半高（physics 约定：总高 = 2 * (half_height + radius)）。
pub fn capsule_half_height(total_height: f32, radius: f32) -> f32 {
    (total_height * 0.5 - radius).max(0.0)
}

/// 角色/相机/视线查询的 physics 适配器。
#[derive(Clone, Copy)]
pub struct PhysicsQuery<'a> {
    /// 被查询的物理世界。
    pub world: &'a PhysicsWorld,
    /// 查询过滤器（层掩码/是否含触发器/排除自身）。
    pub filter: QueryFilter,
}

impl<'a> PhysicsQuery<'a> {
    /// 使用默认过滤器（命中所有普通碰撞体、忽略触发器）构造。
    pub fn new(world: &'a PhysicsWorld) -> Self {
        Self {
            world,
            filter: QueryFilter::new(),
        }
    }

    /// 使用自定义过滤器构造。
    pub fn with_filter(world: &'a PhysicsWorld, filter: QueryFilter) -> Self {
        Self { world, filter }
    }

    /// 替换过滤器。
    pub fn filter(mut self, filter: QueryFilter) -> Self {
        self.filter = filter;
        self
    }

    /// 只查询这些层。
    pub fn with_layer_mask(mut self, layer_mask: u32) -> Self {
        self.filter = self.filter.with_layer_mask(layer_mask);
        self
    }

    /// 是否把触发器纳入查询。
    pub fn with_sensors(mut self, include_sensors: bool) -> Self {
        self.filter = self.filter.with_sensors(include_sensors);
        self
    }

    /// 排除某个刚体（通常是角色自身）。
    pub fn exclude_body(mut self, body_id: u64) -> Self {
        self.filter = self.filter.exclude_body(body_id);
        self
    }
}

impl CharacterCollisionQuery for PhysicsQuery<'_> {
    fn ground_probe(&self, feet: Vec3, radius: f32, max_distance: f32) -> Option<GroundHit> {
        // 半高传 0 时为球探针；physics 的 distance 语义与胶囊一致（脚底到地面距离）。
        let hit =
            self.world
                .ground_probe(to_core_vec3(feet), radius, 0.0, max_distance, self.filter)?;
        Some(GroundHit::new(
            feet - Vec3::UP * hit.distance(),
            from_core_vec3(*hit.normal()),
            hit.distance(),
        ))
    }

    fn move_capsule(&self, feet: Vec3, delta: Vec3, radius: f32, height: f32) -> CapsuleMoveResult {
        let half_height = capsule_half_height(height, radius);
        let result = self.world.move_capsule(
            to_core_vec3(feet),
            to_core_vec3(delta),
            radius,
            half_height,
            self.filter,
        );
        let contacts = result
            .hits
            .iter()
            .map(|hit| {
                MoveContact::new(
                    from_core_vec3(*hit.point()),
                    from_core_vec3(*hit.normal()),
                    hit.distance(),
                )
            })
            .collect();
        CapsuleMoveResult::with_contacts(from_core_vec3(result.position), contacts)
    }
}

impl CameraCollisionQuery for PhysicsQuery<'_> {
    fn sweep_camera(&self, from: Vec3, to: Vec3, radius: f32) -> CameraSweep {
        let delta = to - from;
        let distance = delta.magnitude();
        if distance <= f32::EPSILON {
            return CameraSweep::clear(to);
        }
        let direction = delta / distance;
        match self.world.sphere_cast(
            to_core_vec3(from),
            radius,
            to_core_vec3(direction),
            distance,
            self.filter,
        ) {
            Some(hit) => CameraSweep::blocked(
                from + direction * hit.distance(),
                from_core_vec3(*hit.normal()),
            ),
            None => CameraSweep::clear(to),
        }
    }
}

impl VisibilityQuery for PhysicsQuery<'_> {
    fn has_line_of_sight(&self, from: Vec3, to: Vec3) -> bool {
        let delta = to - from;
        let distance = delta.magnitude();
        if distance <= f32::EPSILON {
            return true;
        }
        let ray = Ray::with_max_distance(to_core_vec3(from), to_core_vec3(delta), distance);
        self.world.raycast_filtered(&ray, self.filter).is_none()
    }
}

/// 把 gameplay 命中盒转换为 physics 查询形状。
///
/// 命中盒中心 = `pose.origin + 旋转后的局部 offset`，
/// 因此 conversion 后 `origin` 传 [`CoreVec3::zero()`] 即可（offset 已是世界空间）。
pub fn to_physics_hitbox(hitbox: &Hitbox, pose: &HitboxPose) -> PhysicsHitbox {
    use crate::combat::hitbox::HitboxShape;

    let shape = match hitbox.shape {
        HitboxShape::Sphere { radius } => PhysicsHitbox::sphere(radius),
        HitboxShape::Capsule {
            half_height,
            radius,
        } => PhysicsHitbox::capsule(half_height, radius),
        HitboxShape::Box { half_extents } => {
            PhysicsHitbox::box_half_extents(to_core_vec3(half_extents))
        }
    };
    shape.with_offset(to_core_vec3(hitbox.world_center(pose)))
}

/// 攻击命中查询的 physics 适配器。
#[derive(Clone, Copy)]
pub struct PhysicsHitQuery<'a> {
    /// 被查询的物理世界。
    pub world: &'a PhysicsWorld,
    /// 查询过滤器。
    pub filter: QueryFilter,
}

impl<'a> PhysicsHitQuery<'a> {
    pub fn new(world: &'a PhysicsWorld) -> Self {
        Self {
            world,
            filter: QueryFilter::new(),
        }
    }

    pub fn with_filter(world: &'a PhysicsWorld, filter: QueryFilter) -> Self {
        Self { world, filter }
    }

    pub fn filter(mut self, filter: QueryFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_layer_mask(mut self, layer_mask: u32) -> Self {
        self.filter = self.filter.with_layer_mask(layer_mask);
        self
    }

    pub fn with_sensors(mut self, include_sensors: bool) -> Self {
        self.filter = self.filter.with_sensors(include_sensors);
        self
    }

    pub fn exclude_body(mut self, body_id: u64) -> Self {
        self.filter = self.filter.exclude_body(body_id);
        self
    }
}

impl HitDetectionQuery for PhysicsHitQuery<'_> {
    fn query_hitbox(&self, hitbox: &Hitbox, pose: &HitboxPose) -> Vec<HitContact> {
        let physics_hitbox = to_physics_hitbox(hitbox, pose);
        // offset 已含世界中心，所以宿主原点传零。
        let overlaps = physics_hitbox.overlaps(self.world, CoreVec3::zero(), self.filter);
        let mut contacts: Vec<HitContact> = Vec::with_capacity(overlaps.len());
        for overlap in overlaps {
            let target = CombatantId::from(overlap.body_id());
            // 0.2 physics 每个刚体只挂一个碰撞体；仍按 body_id 去重，防重复命中。
            if contacts.iter().any(|contact| contact.target == target) {
                continue;
            }
            contacts.push(HitContact::new(
                target,
                from_core_vec3(*overlap.point()),
                from_core_vec3(*overlap.normal()),
            ));
        }
        contacts
    }
}
