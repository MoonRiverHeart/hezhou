//! Hitbox：命中盒数据、姿态与命中查询抽象。
//!
//! 命中盒本身是纯数据；与场景的重叠检测由 [`HitDetectionQuery`] 注入
//! （physics crate 或集成层实现），因此命中判定可脱离物理实现测试。
//!
//! 命中盒局部坐标约定：`offset.x` 向右、`offset.y` 向上、`offset.z` 向前，
//! 绕 Y 轴按 [`HitboxPose::forward`] 的 yaw 旋转。

use crate::ids::CombatantId;
use crate::math::Vec3;

/// 命中盒形状。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitboxShape {
    /// 球。
    Sphere {
        /// 半径。
        radius: f32,
    },
    /// 胶囊（沿 Y 轴）。
    Capsule {
        /// 圆柱段半高。
        half_height: f32,
        /// 半径。
        radius: f32,
    },
    /// 盒（轴对齐于攻击者朝向）。
    Box {
        /// 半尺寸。
        half_extents: Vec3,
    },
}

impl HitboxShape {
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::Capsule {
            half_height,
            radius,
        }
    }

    pub fn box_half_extents(half_extents: Vec3) -> Self {
        Self::Box { half_extents }
    }

    /// 形状的包围半径（用于粗筛/调试）。
    pub fn bounding_radius(&self) -> f32 {
        match self {
            Self::Sphere { radius } => *radius,
            Self::Capsule {
                half_height,
                radius,
            } => (*half_height + *radius).abs(),
            Self::Box { half_extents } => half_extents.magnitude(),
        }
    }
}

impl Default for HitboxShape {
    fn default() -> Self {
        Self::Sphere { radius: 0.7 }
    }
}

/// 命中盒定义（相对攻击者）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hitbox {
    /// 形状。
    pub shape: HitboxShape,
    /// 相对攻击者脚底的位置偏移（局部坐标）。
    pub offset: Vec3,
}

impl Hitbox {
    pub fn new(shape: HitboxShape) -> Self {
        Self {
            shape,
            offset: Vec3::ZERO,
        }
    }

    pub fn with_offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }

    /// 命中盒中心的世界坐标。
    pub fn world_center(&self, pose: &HitboxPose) -> Vec3 {
        let yaw = pose.forward.horizontal().yaw();
        pose.origin
            + Vec3::right_from_yaw(yaw) * self.offset.x
            + Vec3::UP * self.offset.y
            + Vec3::from_yaw(yaw) * self.offset.z
    }
}

impl Default for Hitbox {
    fn default() -> Self {
        Self {
            shape: HitboxShape::default(),
            offset: Vec3::new(0.0, 1.0, 0.6),
        }
    }
}

/// 攻击者在命中时刻的世界姿态。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitboxPose {
    /// 攻击者脚底位置。
    pub origin: Vec3,
    /// 攻击者水平朝向（会归一化；零向量回退为 -Z）。
    pub forward: Vec3,
}

impl HitboxPose {
    pub fn new(origin: Vec3, forward: Vec3) -> Self {
        Self {
            origin,
            forward: forward.horizontal().normalized_or(Vec3::FORWARD),
        }
    }

    /// 攻击者右向。
    pub fn right(&self) -> Vec3 {
        Vec3::right_from_yaw(self.forward.yaw())
    }
}

impl Default for HitboxPose {
    fn default() -> Self {
        Self {
            origin: Vec3::ZERO,
            forward: Vec3::FORWARD,
        }
    }
}

/// 一次命中接触。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitContact {
    /// 被命中的目标。
    pub target: CombatantId,
    /// 命中点（世界空间）。
    pub point: Vec3,
    /// 接触面法线（世界空间，指向攻击者一侧）。
    pub normal: Vec3,
}

impl HitContact {
    pub fn new(target: CombatantId, point: Vec3, normal: Vec3) -> Self {
        Self {
            target,
            point,
            normal: normal.normalized(),
        }
    }
}

/// 命中盒重叠查询（物理实现注入）。
pub trait HitDetectionQuery {
    /// 查询与命中盒重叠的目标；同一目标只应返回一次。
    fn query_hitbox(&self, hitbox: &Hitbox, pose: &HitboxPose) -> Vec<HitContact>;
}

/// 一次命中窗口的运行时状态：去重"一次攻击只命中同一目标一次"。
#[derive(Clone, Debug, Default)]
pub struct HitboxRuntime {
    active: bool,
    hit_targets: Vec<CombatantId>,
}

impl HitboxRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// 激活命中窗口并清空已命中记录（每次攻击/每段命中盒开始时调用）。
    pub fn begin(&mut self) {
        self.active = true;
        self.hit_targets.clear();
    }

    /// 关闭命中窗口。
    pub fn end(&mut self) {
        self.active = false;
    }

    /// 是否处于激活状态。
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// 本次窗口已命中的目标。
    pub fn hit_targets(&self) -> &[CombatantId] {
        &self.hit_targets
    }

    /// 目标本窗口是否已被命中。
    pub fn was_hit(&self, target: CombatantId) -> bool {
        self.hit_targets.contains(&target)
    }

    /// 重置（结束窗口并清空记录）。
    pub fn reset(&mut self) {
        self.active = false;
        self.hit_targets.clear();
    }

    /// 查询命中并过滤本窗口已命中的目标。
    ///
    /// 未激活时返回空列表。
    pub fn detect(
        &mut self,
        query: &dyn HitDetectionQuery,
        hitbox: &Hitbox,
        pose: &HitboxPose,
    ) -> Vec<HitContact> {
        if !self.active {
            return Vec::new();
        }
        let mut fresh = Vec::new();
        for contact in query.query_hitbox(hitbox, pose) {
            if self.was_hit(contact.target) {
                continue;
            }
            self.hit_targets.push(contact.target);
            fresh.push(contact);
        }
        fresh
    }
}
