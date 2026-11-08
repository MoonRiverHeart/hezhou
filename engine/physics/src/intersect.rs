//! 内部几何求交：点 / 线段 / 射线 / 扫掠与 Sphere / Box / Capsule / Plane 的
//! 距离、接触与命中计算。
//!
//! 约定（与现有 AABB 能力一致）：
//! - 所有静态形状使用“目标形状中心 + 形状参数”的世界空间表示；
//! - `Box` 为轴对齐盒（忽略刚体旋转）；
//! - `Capsule` 的轴固定为世界 Y 轴（角色始终直立）；
//! - `Plane` 为无限平面，法线由刚体旋转作用于 +Y 得到。
//!
//! 距离函数使用有符号距离（SDF）：负值表示穿透。SDF 沿直线段是凸函数，
//! 因此胶囊/球的重叠与扫掠可以通过三分搜索 + 保守推进稳定求解。
//! TODO(0.3)：支持任意朝向的盒/胶囊，并为 Cylinder 提供精确求交。

use crate::collider::ColliderShape;
use hezhou_core::math::Vec3;

/// 数值容差。
pub(crate) const EPS: f32 = 1e-5;

/// 旋转盒的**保守 AABB 半尺寸**：把局部半尺寸按旋转矩阵的绝对值扩张。
///
/// 这是 OBB 的包围盒近似：结果永远不会小于真实 OBB 的投影范围，
/// 因此接触检测是保守的（可能产生少量假阳性，但不会漏检）。
/// 完整 OBB/分离轴精确接触留后续版本。
pub(crate) fn rotated_aabb_half_extents(
    rotation: &nalgebra::UnitQuaternion<f32>,
    half_extents: Vec3,
) -> Vec3 {
    let matrix = rotation.to_rotation_matrix();
    let m = matrix.matrix();
    Vec3::new(
        m[(0, 0)].abs() * half_extents.x
            + m[(0, 1)].abs() * half_extents.y
            + m[(0, 2)].abs() * half_extents.z,
        m[(1, 0)].abs() * half_extents.x
            + m[(1, 1)].abs() * half_extents.y
            + m[(1, 2)].abs() * half_extents.z,
        m[(2, 0)].abs() * half_extents.x
            + m[(2, 1)].abs() * half_extents.y
            + m[(2, 2)].abs() * half_extents.z,
    )
}

/// 目标静态形状（世界空间）。
#[derive(Clone, Copy, Debug)]
pub(crate) enum TargetShape {
    Sphere {
        center: Vec3,
        radius: f32,
    },
    Box {
        center: Vec3,
        half_extents: Vec3,
    },
    Capsule {
        center: Vec3,
        half_height: f32,
        radius: f32,
    },
    Plane {
        point: Vec3,
        normal: Vec3,
    },
}

/// 一次形状接触（重叠/穿透）结果。
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShapeContact {
    /// 表面间距，负值表示穿透深度。
    pub distance: f32,
    /// 从目标形状指向查询形状的单位法线（把查询形状推出去的方向）。
    pub normal: Vec3,
    /// 查询形状表面上的接触点。
    pub query_point: Vec3,
    /// 目标形状表面上的接触点。
    pub target_point: Vec3,
}

/// 扫掠命中。
#[derive(Clone, Copy, Debug)]
pub(crate) struct SweepContact {
    /// 归一化时间参数 `0..=1`。
    pub t: f32,
    /// 从目标指向查询形状的单位法线。
    pub normal: Vec3,
    /// 查询形状表面上的接触点。
    pub point: Vec3,
}

/// 射线命中。
#[derive(Clone, Copy, Debug)]
pub(crate) struct RayContact {
    /// 沿射线方向的距离（`>= 0`）。
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

impl TargetShape {
    /// 由碰撞体形状 + 世界位置构造目标形状；`Cylinder` 暂不支持精确求交。
    pub(crate) fn from_collider(
        shape: &ColliderShape,
        position: Vec3,
        plane_normal: Vec3,
    ) -> Option<Self> {
        Some(match shape {
            ColliderShape::Sphere { radius } => Self::Sphere {
                center: position,
                radius: *radius,
            },
            ColliderShape::Box { half_extents } => Self::Box {
                center: position,
                half_extents: *half_extents,
            },
            ColliderShape::Capsule {
                half_height,
                radius,
            } => Self::Capsule {
                center: position,
                half_height: *half_height,
                radius: *radius,
            },
            ColliderShape::Plane => {
                let normal = plane_normal.normalized();
                Self::Plane {
                    point: position,
                    normal: if normal.magnitude() > EPS {
                        normal
                    } else {
                        Vec3::up()
                    },
                }
            }
            // TODO(0.3)：Cylinder 精确求交；当前查询会跳过圆柱碰撞体。
            ColliderShape::Cylinder { .. } => return None,
        })
    }

    /// 目标形状的世界空间轴对齐包围盒（`Plane` 返回 `None`）。
    pub(crate) fn aabb(&self) -> Option<(Vec3, Vec3)> {
        match self {
            Self::Sphere { center, radius } => {
                let r = Vec3::new(*radius, *radius, *radius);
                Some((*center - r, *center + r))
            }
            Self::Box {
                center,
                half_extents,
            } => Some((*center - *half_extents, *center + *half_extents)),
            Self::Capsule {
                center,
                half_height,
                radius,
            } => {
                let half = Vec3::new(*radius, half_height + radius, *radius);
                Some((*center - half, *center + half))
            }
            Self::Plane { .. } => None,
        }
    }

    /// 有符号距离：负值表示点在形状内部。
    pub(crate) fn sdf(&self, p: Vec3) -> f32 {
        match self {
            Self::Sphere { center, radius } => (p - *center).magnitude() - *radius,
            Self::Box {
                center,
                half_extents,
            } => point_box_sdf(p, *center, *half_extents),
            Self::Capsule {
                center,
                half_height,
                radius,
            } => {
                let a = *center - Vec3::up() * *half_height;
                let b = *center + Vec3::up() * *half_height;
                point_capsule_sdf(p, a, b, *radius)
            }
            Self::Plane { point, normal } => Vec3::dot(&(p - *point), normal),
        }
    }

    /// 形状表面指向点 `p` 的单位法线（退化时回退为 +Y）。
    pub(crate) fn normal_at(&self, p: Vec3) -> Vec3 {
        match self {
            Self::Sphere { center, .. } => {
                let d = p - *center;
                non_zero_normal(d)
            }
            Self::Box {
                center,
                half_extents,
            } => box_normal_at(p, *center, *half_extents),
            Self::Capsule {
                center,
                half_height,
                radius,
            } => {
                let a = *center - Vec3::up() * *half_height;
                let b = *center + Vec3::up() * *half_height;
                let _ = radius;
                let q = closest_point_on_segment(p, a, b);
                non_zero_normal(p - q)
            }
            Self::Plane { normal, .. } => {
                let sd = self.sdf(p);
                if sd >= 0.0 {
                    *normal
                } else {
                    -*normal
                }
            }
        }
    }
}

/// 非零向量归一化；退化时回退为 +Y。
fn non_zero_normal(v: Vec3) -> Vec3 {
    if v.magnitude() > EPS {
        v.normalized()
    } else {
        Vec3::up()
    }
}

/// 点与线段的最近点。
pub(crate) fn closest_point_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let denom = Vec3::dot(&ab, &ab);
    if denom <= EPS {
        return a;
    }
    let t = (Vec3::dot(&(p - a), &ab) / denom).clamp(0.0, 1.0);
    a + ab * t
}

/// 点到胶囊（线段 + 半径）的有符号距离。
pub(crate) fn point_capsule_sdf(p: Vec3, a: Vec3, b: Vec3, radius: f32) -> f32 {
    let q = closest_point_on_segment(p, a, b);
    (p - q).magnitude() - radius
}

/// 点到轴对齐盒的有符号距离（精确 SDF）。
pub(crate) fn point_box_sdf(p: Vec3, center: Vec3, half_extents: Vec3) -> f32 {
    let d = p - center;
    let q = Vec3::new(
        d.x.abs() - half_extents.x,
        d.y.abs() - half_extents.y,
        d.z.abs() - half_extents.z,
    );
    let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).magnitude();
    let inside = q.x.max(q.y.max(q.z)).min(0.0);
    outside + inside
}

/// 轴对齐盒表面指向点的法线；点在盒内时沿最浅的面。
fn box_normal_at(p: Vec3, center: Vec3, half: Vec3) -> Vec3 {
    let d = p - center;
    let mut n = Vec3::zero();
    if d.x > half.x {
        n.x = 1.0;
    } else if d.x < -half.x {
        n.x = -1.0;
    }
    if d.y > half.y {
        n.y = 1.0;
    } else if d.y < -half.y {
        n.y = -1.0;
    }
    if d.z > half.z {
        n.z = 1.0;
    } else if d.z < -half.z {
        n.z = -1.0;
    }
    if n.magnitude() > EPS {
        return n.normalized();
    }

    // 点在盒内部：选择距离最近的面方向作为推出法线。
    let candidates = [
        (half.x - d.x, Vec3::new(1.0, 0.0, 0.0)),
        (half.x + d.x, Vec3::new(-1.0, 0.0, 0.0)),
        (half.y - d.y, Vec3::new(0.0, 1.0, 0.0)),
        (half.y + d.y, Vec3::new(0.0, -1.0, 0.0)),
        (half.z - d.z, Vec3::new(0.0, 0.0, 1.0)),
        (half.z + d.z, Vec3::new(0.0, 0.0, -1.0)),
    ];
    let mut best_depth = f32::INFINITY;
    let mut best_normal = Vec3::up();
    for (depth, normal) in candidates {
        if depth >= 0.0 && depth < best_depth {
            best_depth = depth;
            best_normal = normal;
        }
    }
    best_normal
}

/// 线段（胶囊中轴）与目标形状的接触计算。
///
/// 由于目标形状均为凸体且 SDF 精确，`sdf` 限制在直线段上是凸函数，
/// 用三分搜索求最小值即可得到精确的最近特征。
pub(crate) fn segment_shape_contact(
    a: Vec3,
    b: Vec3,
    radius: f32,
    target: &TargetShape,
) -> ShapeContact {
    let seg = b - a;
    let seg_len_sq = Vec3::dot(&seg, &seg);

    let point_at = |t: f32| a + seg * t;
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    if seg_len_sq > EPS {
        for _ in 0..48 {
            let m1 = lo + (hi - lo) / 3.0;
            let m2 = hi - (hi - lo) / 3.0;
            // 平局时向左收缩：盒的 SDF 沿线段可能出现“平台区”，
            // 取平台区靠前的点能拿到更稳定的面法线（例如胶囊贴住台阶顶面时）。
            if target.sdf(point_at(m1)) <= target.sdf(point_at(m2)) {
                hi = m2;
            } else {
                lo = m1;
            }
        }
    }
    let t = ((lo + hi) * 0.5).clamp(0.0, 1.0);
    let p = point_at(t);
    let d = target.sdf(p);
    let normal = target.normal_at(p);
    let target_point = if d.abs() > EPS { p - normal * d } else { p };
    ShapeContact {
        distance: d - radius,
        normal,
        query_point: p - normal * radius,
        target_point,
    }
}

/// 轴对齐盒查询形状与目标形状的重叠计算。
fn box_vs_target(center: Vec3, half_extents: Vec3, target: &TargetShape) -> Option<ShapeContact> {
    match target {
        TargetShape::Sphere { center: sc, radius } => {
            let q = *sc - clamp_vec(*sc, center - half_extents, center + half_extents);
            let dist = q.magnitude();
            if dist >= *radius {
                return None;
            }
            if dist > EPS {
                let normal = -q.normalized();
                let depth = radius - dist;
                let target_point = *sc - q.normalized() * *radius;
                Some(ShapeContact {
                    distance: -depth,
                    normal,
                    query_point: *sc - q,
                    target_point,
                })
            } else {
                // 球心在盒内：沿最近面的反方向把盒推出去，深度 = 半径 + 到该面的距离。
                let face_normal = box_normal_at(*sc, center, half_extents);
                let (axis, sign) = face_axis_direction(face_normal);
                let face_distance =
                    (component(half_extents, axis) - component(*sc - center, axis) * sign).max(0.0);
                let normal = -face_normal;
                let depth = radius + face_distance;
                Some(ShapeContact {
                    distance: -depth,
                    normal,
                    query_point: *sc,
                    target_point: *sc,
                })
            }
        }
        TargetShape::Box {
            center: bc,
            half_extents: bh,
        } => {
            let qmin = center - half_extents;
            let qmax = center + half_extents;
            let tmin = *bc - *bh;
            let tmax = *bc + *bh;
            let mut best_axis = 0usize;
            let mut best_overlap = f32::INFINITY;
            for axis in 0..3 {
                let overlap = component(qmax, axis).min(component(tmax, axis))
                    - component(qmin, axis).max(component(tmin, axis));
                if overlap <= 0.0 {
                    return None;
                }
                if overlap < best_overlap {
                    best_overlap = overlap;
                    best_axis = axis;
                }
            }
            let sign = if component(center, best_axis) >= component(*bc, best_axis) {
                1.0
            } else {
                -1.0
            };
            let normal = axis_vec(best_axis) * sign;
            Some(ShapeContact {
                distance: -best_overlap,
                normal,
                query_point: center,
                target_point: center - normal * best_overlap,
            })
        }
        TargetShape::Capsule {
            center: cc,
            half_height,
            radius,
        } => {
            // 用“胶囊中轴 vs 查询盒”的接触反过来求：法线需取反。
            let ca = *cc - Vec3::up() * *half_height;
            let cb = *cc + Vec3::up() * *half_height;
            let box_target = TargetShape::Box {
                center,
                half_extents,
            };
            let contact = segment_shape_contact(ca, cb, *radius, &box_target);
            if contact.distance >= 0.0 {
                return None;
            }
            Some(ShapeContact {
                distance: contact.distance,
                normal: -contact.normal,
                query_point: contact.target_point,
                target_point: contact.query_point,
            })
        }
        TargetShape::Plane { point, normal } => {
            let sd_center = Vec3::dot(&(center - *point), normal);
            let extent = normal.x.abs() * half_extents.x
                + normal.y.abs() * half_extents.y
                + normal.z.abs() * half_extents.z;
            let min_sd = sd_center - extent;
            let max_sd = sd_center + extent;
            if min_sd > 0.0 || max_sd < 0.0 {
                return None;
            }
            let push_positive = -min_sd;
            let push_negative = max_sd;
            let (n, depth) = if push_positive <= push_negative {
                (*normal, push_positive)
            } else {
                (-*normal, push_negative)
            };
            Some(ShapeContact {
                distance: -depth,
                normal: n,
                query_point: center,
                target_point: center - *normal * sd_center,
            })
        }
    }
}

fn component(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn axis_vec(axis: usize) -> Vec3 {
    match axis {
        0 => Vec3::new(1.0, 0.0, 0.0),
        1 => Vec3::new(0.0, 1.0, 0.0),
        _ => Vec3::new(0.0, 0.0, 1.0),
    }
}

fn face_axis_direction(normal: Vec3) -> (usize, f32) {
    let ax = normal.x.abs();
    let ay = normal.y.abs();
    let az = normal.z.abs();
    if ax >= ay && ax >= az {
        (0, normal.x.signum())
    } else if ay >= az {
        (1, normal.y.signum())
    } else {
        (2, normal.z.signum())
    }
}

fn clamp_vec(v: Vec3, min: Vec3, max: Vec3) -> Vec3 {
    Vec3::new(
        v.x.clamp(min.x, max.x),
        v.y.clamp(min.y, max.y),
        v.z.clamp(min.z, max.z),
    )
}

/// 查询形状与目标形状的重叠计算。
///
/// 支持查询形状 `Sphere` / `Box` / `Capsule`；`Plane` 只作为静态目标，
/// 不能作为查询形状。返回 `None` 表示未重叠（相切也不算命中）。
pub(crate) fn shape_overlap(
    query_shape: &ColliderShape,
    query_center: Vec3,
    target: &TargetShape,
) -> Option<ShapeContact> {
    match query_shape {
        ColliderShape::Sphere { radius } => {
            let contact = segment_shape_contact(query_center, query_center, *radius, target);
            if contact.distance < 0.0 {
                Some(contact)
            } else {
                None
            }
        }
        ColliderShape::Capsule {
            half_height,
            radius,
        } => {
            let a = query_center - Vec3::up() * *half_height;
            let b = query_center + Vec3::up() * *half_height;
            let contact = segment_shape_contact(a, b, *radius, target);
            if contact.distance < 0.0 {
                Some(contact)
            } else {
                None
            }
        }
        ColliderShape::Box { half_extents } => box_vs_target(query_center, *half_extents, target),
        // 查询形状不支持 Plane / Cylinder（0.2 只用命中盒 Sphere/Box/Capsule）。
        ColliderShape::Plane | ColliderShape::Cylinder { .. } => None,
    }
}

/// 线段（或点）沿位移对目标形状做保守推进扫掠。
///
/// `a`/`b` 为查询形状中轴线段端点，`radius` 为其半径（球可令 `a == b`），
/// `displacement` 为本步位移。返回最早的接触（`t` 为 `0..=1` 的比例）。
pub(crate) fn sweep_segment_shape(
    a: Vec3,
    b: Vec3,
    radius: f32,
    displacement: Vec3,
    target: &TargetShape,
) -> Option<SweepContact> {
    let len = displacement.magnitude();
    if len <= EPS {
        let contact = segment_shape_contact(a, b, radius, target);
        if contact.distance <= 0.0 {
            return Some(SweepContact {
                t: 0.0,
                normal: contact.normal,
                point: contact.query_point,
            });
        }
        return None;
    }

    // 广相：段扫掠包围盒与目标 AABB 是否相交。
    if let Some((tmin, tmax)) = target.aabb() {
        let pa = a - Vec3::new(radius, radius, radius);
        let pb = b + Vec3::new(radius, radius, radius);
        let s_min = Vec3::new(pa.x.min(pb.x), pa.y.min(pb.y), pa.z.min(pb.z));
        let s_max = Vec3::new(pa.x.max(pb.x), pa.y.max(pb.y), pa.z.max(pb.z));
        let d = displacement;
        let e_min = Vec3::new(
            s_min.x.min(s_min.x + d.x),
            s_min.y.min(s_min.y + d.y),
            s_min.z.min(s_min.z + d.z),
        );
        let e_max = Vec3::new(
            s_max.x.max(s_max.x + d.x),
            s_max.y.max(s_max.y + d.y),
            s_max.z.max(s_max.z + d.z),
        );
        if !aabb_overlap(e_min, e_max, tmin, tmax) {
            return None;
        }
    }

    // 保守推进：沿查询位移前进到“表面间距为 0”的线性估计位置。
    // 由于距离函数是凸的，推进不会越过真实接触点；若因法线退化导致轻微越界，
    // 再用二分在 (t_separated, t) 区间内回退到首次接触，保证结果稳定。
    let at = |t: f32| {
        let offset = displacement * t;
        segment_shape_contact(a + offset, b + offset, radius, target)
    };

    let mut t_separated = 0.0f32;
    let mut t = 0.0f32;
    for _ in 0..24 {
        let contact = at(t);
        if contact.distance <= 0.0 {
            let mut lo = t_separated;
            let mut hi = t;
            for _ in 0..24 {
                let mid = (lo + hi) * 0.5;
                if at(mid).distance <= 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            // 用接触前（外侧）的法线，避免穿透后用内部面法线。
            let surface = at(lo);
            return Some(SweepContact {
                t: hi,
                normal: surface.normal,
                point: surface.query_point,
            });
        }
        // 沿法线的接近速度（负值表示正在靠近）。
        let approach = -Vec3::dot(&displacement, &contact.normal);
        if approach <= EPS {
            return None;
        }
        let dt = contact.distance / approach;
        if dt <= EPS {
            return Some(SweepContact {
                t,
                normal: contact.normal,
                point: contact.query_point,
            });
        }
        t_separated = t;
        t += dt;
        if t > 1.0 {
            return None;
        }
    }
    None
}

/// 轴对齐包围盒相交测试。
pub(crate) fn aabb_overlap(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> bool {
    a_min.x <= b_max.x
        && a_max.x >= b_min.x
        && a_min.y <= b_max.y
        && a_max.y >= b_min.y
        && a_min.z <= b_max.z
        && a_max.z >= b_min.z
}

/// 射线与目标形状求交。
pub(crate) fn ray_target(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    target: &TargetShape,
) -> Option<RayContact> {
    let hit = match target {
        TargetShape::Sphere { center, radius } => ray_sphere(origin, direction, *center, *radius),
        TargetShape::Box {
            center,
            half_extents,
        } => ray_box(origin, direction, *center, *half_extents),
        TargetShape::Capsule {
            center,
            half_height,
            radius,
        } => {
            let a = *center - Vec3::up() * *half_height;
            let b = *center + Vec3::up() * *half_height;
            ray_capsule(origin, direction, a, b, *radius)
        }
        TargetShape::Plane { point, normal } => ray_plane(origin, direction, *point, *normal),
    }?;
    if hit.t > max_distance + EPS {
        return None;
    }
    Some(hit)
}

/// 射线与球求交（取最近的正根）。
pub(crate) fn ray_sphere(
    origin: Vec3,
    direction: Vec3,
    center: Vec3,
    radius: f32,
) -> Option<RayContact> {
    let oc = origin - center;
    let b = Vec3::dot(&oc, &direction);
    let c = Vec3::dot(&oc, &oc) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let mut t = -b - sq;
    if t < 0.0 {
        t = -b + sq;
    }
    if t < 0.0 {
        return None;
    }
    let point = origin + direction * t;
    let normal = non_zero_normal(point - center);
    Some(RayContact { t, point, normal })
}

/// 射线与轴对齐盒求交（slab 法）。
pub(crate) fn ray_box(
    origin: Vec3,
    direction: Vec3,
    center: Vec3,
    half_extents: Vec3,
) -> Option<RayContact> {
    let min = center - half_extents;
    let max = center + half_extents;
    let mut tmin = f32::NEG_INFINITY;
    let mut tmax = f32::INFINITY;
    let mut entry_axis = 0usize;
    let mut entry_sign = 1.0f32;

    for axis in 0..3 {
        let o = component(origin, axis);
        let d = component(direction, axis);
        let (lo, hi) = (component(min, axis), component(max, axis));
        if d.abs() <= EPS {
            if o < lo || o > hi {
                return None;
            }
        } else {
            let t1 = (lo - o) / d;
            let t2 = (hi - o) / d;
            let (near, far) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
            if near > tmin {
                tmin = near;
                entry_axis = axis;
                // 从负方向进入时命中较小的面，法线为负轴；反之正轴。
                entry_sign = if d > 0.0 { -1.0 } else { 1.0 };
            }
            if far < tmax {
                tmax = far;
            }
            if tmin > tmax {
                return None;
            }
        }
    }

    let (t, normal) = if tmin >= 0.0 {
        (tmin, axis_vec(entry_axis) * entry_sign)
    } else if tmax >= 0.0 {
        // 射线的起点在盒内部：返回出射面。
        let axis_sign = if component(direction, entry_axis) > 0.0 {
            1.0
        } else {
            -1.0
        };
        (tmax, axis_vec(entry_axis) * axis_sign)
    } else {
        return None;
    };
    let point = origin + direction * t;
    Some(RayContact { t, point, normal })
}

/// 射线与无限平面求交，法线朝向射线来向。
pub(crate) fn ray_plane(
    origin: Vec3,
    direction: Vec3,
    point: Vec3,
    normal: Vec3,
) -> Option<RayContact> {
    let denom = Vec3::dot(&direction, &normal);
    let sd = Vec3::dot(&(origin - point), &normal);
    if denom.abs() <= EPS {
        return None;
    }
    let t = -sd / denom;
    if t < 0.0 {
        return None;
    }
    let hit_point = origin + direction * t;
    let n = if Vec3::dot(&direction, &normal) > 0.0 {
        -normal
    } else {
        normal
    };
    Some(RayContact {
        t,
        point: hit_point,
        normal: n,
    })
}

/// 射线与竖直胶囊求交（侧圆柱 + 两端半球，取最近正根）。
pub(crate) fn ray_capsule(
    origin: Vec3,
    direction: Vec3,
    a: Vec3,
    b: Vec3,
    radius: f32,
) -> Option<RayContact> {
    let mut best: Option<RayContact> = None;

    // 侧面：竖直无限圆柱，再按 y 范围截断。
    let (cx, cz) = (a.x, a.z);
    let ox = origin.x - cx;
    let oz = origin.z - cz;
    let dx = direction.x;
    let dz = direction.z;
    let aa = dx * dx + dz * dz;
    if aa > EPS {
        let bb = ox * dx + oz * dz;
        let cc = ox * ox + oz * oz - radius * radius;
        let disc = bb * bb - aa * cc;
        if disc >= 0.0 {
            let sq = disc.sqrt();
            for t in [(-bb - sq) / aa, (-bb + sq) / aa] {
                if t < 0.0 {
                    continue;
                }
                let p = origin + direction * t;
                let (low, high) = if a.y <= b.y { (a.y, b.y) } else { (b.y, a.y) };
                if p.y >= low - EPS && p.y <= high + EPS {
                    let normal = non_zero_normal(Vec3::new(p.x - cx, 0.0, p.z - cz));
                    let candidate = RayContact {
                        t,
                        point: p,
                        normal,
                    };
                    if best.is_none_or(|h| candidate.t < h.t) {
                        best = Some(candidate);
                    }
                }
            }
        }
    }

    // 两端球盖。
    for cap in [a, b] {
        if let Some(hit) = ray_sphere(origin, direction, cap, radius) {
            if best.is_none_or(|h| hit.t < h.t) {
                best = Some(hit);
            }
        }
    }

    best
}
