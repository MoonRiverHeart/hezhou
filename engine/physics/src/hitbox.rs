//! 命中盒（Hitbox）：攻击判定用的查询形状。
//!
//! 命中盒不是世界里的常驻碰撞体，而是一个“按需查询”的形状：
//! 攻击帧到来时，用宿主实体原点 + `offset` 在世界里做一次重叠查询，
//! 命中的目标即可用于伤害/受击判定。
//!
//! 支持 Sphere / Box / Capsule 三种形状（0.2 动作游戏需求）。

use crate::collider::{layers, ColliderShape};
use crate::debug::{DebugColor, COLOR_HITBOX};
use crate::query::{OverlapHit, QueryFilter};
use crate::world::PhysicsWorld;
use hezhou_core::math::Vec3;

/// 命中盒：查询形状 + 相对宿主实体的偏移 + 层/掩码 + 调试颜色。
#[derive(Clone, Debug)]
pub struct Hitbox {
    /// 命中盒形状（Sphere / Box / Capsule）。
    pub shape: ColliderShape,
    /// 相对宿主实体原点的局部偏移。
    pub offset: Vec3,
    /// 命中盒所在层。
    pub layer: u32,
    /// 命中盒愿意命中的层掩码。
    pub mask: u32,
    /// 调试绘制颜色。
    pub color: DebugColor,
}

impl Hitbox {
    fn with_shape(shape: ColliderShape) -> Self {
        Self {
            shape,
            offset: Vec3::zero(),
            layer: layers::PLAYER,
            mask: layers::ALL,
            color: COLOR_HITBOX,
        }
    }

    /// 球形命中盒。
    pub fn sphere(radius: f32) -> Self {
        Self::with_shape(ColliderShape::sphere(radius))
    }

    /// 盒形命中盒（轴对齐半尺寸）。
    pub fn box_half_extents(half_extents: Vec3) -> Self {
        Self::with_shape(ColliderShape::box_half_extents(half_extents))
    }

    /// 胶囊命中盒（`half_height` 为圆柱段半高）。
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::with_shape(ColliderShape::capsule(half_height, radius))
    }

    /// 设置相对宿主实体的局部偏移。
    pub fn with_offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }

    /// 设置层与掩码。
    pub fn with_layers(mut self, layer: u32, mask: u32) -> Self {
        self.layer = layer;
        self.mask = mask;
        self
    }

    /// 设置调试颜色。
    pub fn with_color(mut self, color: DebugColor) -> Self {
        self.color = color;
        self
    }

    /// 命中盒世界空间中心。
    pub fn center(&self, origin: Vec3) -> Vec3 {
        origin + self.offset
    }

    /// 在世界中查询命中盒重叠目标。
    ///
    /// `filter` 与命中盒自身掩码取交集（两者都要允许才会命中）；
    /// 传入 `origin` 为宿主实体原点。
    pub fn overlaps(
        &self,
        world: &PhysicsWorld,
        origin: Vec3,
        filter: QueryFilter,
    ) -> Vec<OverlapHit> {
        let filter = filter.with_layer_mask(filter.layer_mask() & self.mask);
        world.overlap_shape(self.center(origin), &self.shape, filter)
    }
}
