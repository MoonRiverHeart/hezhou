//! 物理调试描述数据。
//!
//! 渲染层（或编辑器的物理调试视图）拿到 `DebugShape` 后可以：
//! - 用 `line_segments()` 直接画线段；
//! - 或按形状类型用自己的渲染方式绘制。
//!
//! 每个碰撞体/命中盒都会生成一个带颜色的描述，颜色按刚体类型与传感器区分。

use crate::collider::{Collider, ColliderShape};
use crate::hitbox::Hitbox;
use crate::rigid_body::RigidBody;
use crate::world::{ColliderEntry, PhysicsWorld};
use hezhou_core::math::Vec3;

/// 调试颜色（RGBA，0..1）。
pub type DebugColor = [f32; 4];

/// 静态刚体颜色（灰）。
pub const COLOR_STATIC: DebugColor = [0.62, 0.64, 0.68, 1.0];
/// 动态刚体颜色（绿）。
pub const COLOR_DYNAMIC: DebugColor = [0.25, 0.90, 0.45, 1.0];
/// 运动学刚体颜色（蓝）。
pub const COLOR_KINEMATIC: DebugColor = [0.30, 0.62, 1.00, 1.0];
/// 触发器颜色（黄）。
pub const COLOR_SENSOR: DebugColor = [1.00, 0.85, 0.20, 1.0];
/// 默认命中盒颜色（红）。
pub const COLOR_HITBOX: DebugColor = [1.00, 0.30, 0.30, 1.0];

/// 一个物理调试形状描述（世界空间）。
#[derive(Clone, Copy, Debug)]
pub enum DebugShape {
    Box {
        center: Vec3,
        half_extents: Vec3,
        color: DebugColor,
    },
    Sphere {
        center: Vec3,
        radius: f32,
        color: DebugColor,
    },
    /// 胶囊：`a` / `b` 为圆柱段两端点（世界空间），`radius` 为半径。
    Capsule {
        a: Vec3,
        b: Vec3,
        radius: f32,
        color: DebugColor,
    },
    Cylinder {
        center: Vec3,
        half_height: f32,
        radius: f32,
        color: DebugColor,
    },
    Plane {
        point: Vec3,
        normal: Vec3,
        color: DebugColor,
    },
}

impl DebugShape {
    /// 调试颜色。
    pub fn color(&self) -> DebugColor {
        match self {
            Self::Box { color, .. }
            | Self::Sphere { color, .. }
            | Self::Capsule { color, .. }
            | Self::Cylinder { color, .. }
            | Self::Plane { color, .. } => *color,
        }
    }

    /// 转成线段列表（世界空间），供渲染层直接画线框。
    pub fn line_segments(&self) -> Vec<(Vec3, Vec3)> {
        match self {
            Self::Box {
                center,
                half_extents,
                ..
            } => box_lines(*center, *half_extents),
            Self::Sphere { center, radius, .. } => {
                let mut lines = Vec::new();
                push_circle(&mut lines, *center, Vec3::up(), *radius, 24);
                push_circle(&mut lines, *center, Vec3::new(1.0, 0.0, 0.0), *radius, 24);
                push_circle(&mut lines, *center, Vec3::new(0.0, 0.0, 1.0), *radius, 24);
                lines
            }
            Self::Capsule { a, b, radius, .. } => {
                let mut lines = Vec::new();
                push_circle(&mut lines, *a, Vec3::up(), *radius, 16);
                push_circle(&mut lines, *b, Vec3::up(), *radius, 16);
                for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    let offset = Vec3::new(dx * radius, 0.0, dz * radius);
                    lines.push((*a + offset, *b + offset));
                }
                lines.push((*a, *b));
                lines
            }
            Self::Cylinder {
                center,
                half_height,
                radius,
                ..
            } => {
                let half = Vec3::up() * *half_height;
                let a = *center - half;
                let b = *center + half;
                let mut lines = Vec::new();
                push_circle(&mut lines, a, Vec3::up(), *radius, 16);
                push_circle(&mut lines, b, Vec3::up(), *radius, 16);
                for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    let offset = Vec3::new(dx * radius, 0.0, dz * radius);
                    lines.push((a + offset, b + offset));
                }
                lines
            }
            Self::Plane { point, normal, .. } => {
                let (t1, t2) = tangent_basis(*normal);
                let half = 2.0f32;
                let p00 = *point - t1 * half - t2 * half;
                let p10 = *point + t1 * half - t2 * half;
                let p11 = *point + t1 * half + t2 * half;
                let p01 = *point - t1 * half + t2 * half;
                vec![
                    (p00, p10),
                    (p10, p11),
                    (p11, p01),
                    (p01, p00),
                    (*point, *point + *normal),
                ]
            }
        }
    }
}

/// 按刚体类型与传感器标记取颜色。
pub fn collider_debug_color(body: &RigidBody, collider: &Collider) -> DebugColor {
    if collider.is_sensor() {
        return COLOR_SENSOR;
    }
    match body.body_type() {
        crate::RigidBodyType::Dynamic => COLOR_DYNAMIC,
        crate::RigidBodyType::Kinematic => COLOR_KINEMATIC,
        crate::RigidBodyType::Static => COLOR_STATIC,
    }
}

impl PhysicsWorld {
    /// 所有碰撞体的调试描述（供物理调试视图使用）。
    pub fn debug_shapes(&self) -> Vec<DebugShape> {
        let mut out = Vec::new();
        self.each_collider(|entry| {
            out.push(debug_shape_of(&entry));
        });
        out
    }
}

fn debug_shape_of(entry: &ColliderEntry<'_>) -> DebugShape {
    let color = collider_debug_color(entry.body, entry.collider);
    match entry.collider.shape() {
        ColliderShape::Sphere { radius } => DebugShape::Sphere {
            center: entry.position,
            radius: *radius,
            color,
        },
        ColliderShape::Box { half_extents } => DebugShape::Box {
            center: entry.position,
            half_extents: *half_extents,
            color,
        },
        ColliderShape::Capsule {
            half_height,
            radius,
        } => DebugShape::Capsule {
            a: entry.position - Vec3::up() * *half_height,
            b: entry.position + Vec3::up() * *half_height,
            radius: *radius,
            color,
        },
        ColliderShape::Cylinder {
            half_height,
            radius,
        } => DebugShape::Cylinder {
            center: entry.position,
            half_height: *half_height,
            radius: *radius,
            color,
        },
        ColliderShape::Plane => DebugShape::Plane {
            point: entry.position,
            normal: entry.plane_normal,
            color,
        },
    }
}

impl Hitbox {
    /// 命中盒调试描述（`origin` 为宿主实体原点）。
    pub fn debug_shape(&self, origin: Vec3) -> DebugShape {
        let center = origin + self.offset;
        let color = self.color;
        match &self.shape {
            ColliderShape::Sphere { radius } => DebugShape::Sphere {
                center,
                radius: *radius,
                color,
            },
            ColliderShape::Box { half_extents } => DebugShape::Box {
                center,
                half_extents: *half_extents,
                color,
            },
            ColliderShape::Capsule {
                half_height,
                radius,
            } => DebugShape::Capsule {
                a: center - Vec3::up() * *half_height,
                b: center + Vec3::up() * *half_height,
                radius: *radius,
                color,
            },
            ColliderShape::Cylinder {
                half_height,
                radius,
            } => DebugShape::Cylinder {
                center,
                half_height: *half_height,
                radius: *radius,
                color,
            },
            ColliderShape::Plane => DebugShape::Plane {
                point: center,
                normal: Vec3::up(),
                color,
            },
        }
    }
}

fn box_lines(center: Vec3, half: Vec3) -> Vec<(Vec3, Vec3)> {
    let corners = [
        center + Vec3::new(-half.x, -half.y, -half.z),
        center + Vec3::new(half.x, -half.y, -half.z),
        center + Vec3::new(half.x, -half.y, half.z),
        center + Vec3::new(-half.x, -half.y, half.z),
        center + Vec3::new(-half.x, half.y, -half.z),
        center + Vec3::new(half.x, half.y, -half.z),
        center + Vec3::new(half.x, half.y, half.z),
        center + Vec3::new(-half.x, half.y, half.z),
    ];
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    EDGES
        .iter()
        .map(|(a, b)| (corners[*a], corners[*b]))
        .collect()
}

/// 在一个以 `axis` 为法线的圆上生成折线。
fn push_circle(
    out: &mut Vec<(Vec3, Vec3)>,
    center: Vec3,
    axis: Vec3,
    radius: f32,
    segments: usize,
) {
    let (t1, t2) = tangent_basis(axis);
    let segments = segments.max(3);
    let mut prev = center + t1 * radius;
    for i in 1..=segments {
        let angle = std::f32::consts::TAU * (i as f32) / (segments as f32);
        let current = center + t1 * (radius * angle.cos()) + t2 * (radius * angle.sin());
        out.push((prev, current));
        prev = current;
    }
}

/// 由法线构造两个切向基。
fn tangent_basis(normal: Vec3) -> (Vec3, Vec3) {
    let n = if normal.magnitude() > 1e-6 {
        normal.normalized()
    } else {
        Vec3::up()
    };
    let helper = if n.y.abs() > 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::up()
    };
    let t1 = Vec3::cross(&n, &helper).normalized();
    let t2 = Vec3::cross(&n, &t1).normalized();
    (t1, t2)
}
