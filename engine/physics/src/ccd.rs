//! CCD 基础：高速球/胶囊的 swept（扫掠）检测。
//!
//! 0.3 第一版只处理“动态球/胶囊 vs 静态/运动学目标”；第二批扩展为
//! **动态-动态 CCD**：对动态目标用“相对位移”扫掠，避免两个高速刚体互穿。
//!
//! 启用条件：
//! - `PhysicsWorld::set_ccd_enabled(true)` 全局开关；
//! - 运动刚体 `RigidBody::set_ccd_enabled(true)`；
//! - 相对位移足够大（每个目标单独判断），否则交给离散检测。
//!
//! 限制：
//! - 只支持球/胶囊作为运动形状，盒/圆柱由离散检测处理；
//! - 动态-动态建议**双方都启用 CCD**，否则只有启用方会被约束；
//! - 顺序求解（按刚体遍历）为近似，极端多体高速场景可能有轻微偏差；
//! - 命中不额外产生碰撞事件（由下一帧离散接触补上）。

use crate::collider::layers_interact;
use crate::intersect::{self, TargetShape};
use crate::world::PhysicsWorld;
use hezhou_core::math::Vec3;

/// CCD 扫掠命中（内部）。
#[derive(Clone, Copy, Debug)]
pub(crate) struct CcdHit {
    /// 命中发生在相对位移的比例（0..=1）。
    pub t: f32,
    /// 从目标指向运动刚体的推出法线。
    pub normal: Vec3,
    /// 命中的目标刚体/碰撞体。
    pub target_body: u64,
    /// 目标碰撞体 id（保留给后续 CCD 事件上报）。
    #[allow(dead_code)]
    pub target_collider: u64,
    /// 目标的逆质量（静态/运动学为 0，用于动量分配）。
    pub target_inv_mass: f32,
    /// 目标本步开始时的速度（用于相对速度计算）。
    pub target_velocity: Vec3,
}

impl PhysicsWorld {
    /// 对指定刚体做一次 CCD 扫掠，返回最近命中。
    ///
    /// `dt` 用于估算动态目标本步的位移（`target_velocity * dt`）；
    /// 扫掠在相对位移空间进行，命中后由调用方按 `displacement * t` 前进运动刚体。
    pub(crate) fn ccd_sweep(&self, body_id: u64, displacement: Vec3, dt: f32) -> Option<CcdHit> {
        if displacement.magnitude() <= 1e-6 {
            return None;
        }

        // 自身形状参数（只支持球/胶囊；其它形状由离散检测处理）。
        let mut own: Option<(Vec3, f32, f32, u32, u32)> = None;
        self.each_collider(|entry| {
            if entry.body_id != body_id {
                return;
            }
            let (radius, half_height) = match entry.collider.shape() {
                crate::ColliderShape::Sphere { radius } => (*radius, 0.0),
                crate::ColliderShape::Capsule {
                    half_height,
                    radius,
                } => (*radius, *half_height),
                _ => return,
            };
            own = Some((
                entry.position,
                radius,
                half_height,
                entry.collider.layer(),
                entry.collider.mask(),
            ));
        });
        let (center, radius, half_height, layer, mask) = own?;

        let a = center - Vec3::up() * half_height;
        let b = center + Vec3::up() * half_height;
        let mut best: Option<CcdHit> = None;

        self.each_collider(|entry| {
            if entry.body_id == body_id {
                return;
            }
            if entry.collider.is_sensor() {
                return;
            }
            if !layers_interact(layer, mask, entry.collider.layer(), entry.collider.mask()) {
                return;
            }
            let Some(target) = (match entry.collider.shape() {
                // 旋转盒：CCD 用保守 AABB（与接触检测一致）。
                crate::ColliderShape::Box { half_extents } => Some(TargetShape::Box {
                    center: entry.position,
                    half_extents: crate::intersect::rotated_aabb_half_extents(
                        entry.body.rotation(),
                        *half_extents,
                    ),
                }),
                shape => TargetShape::from_collider(shape, entry.position, entry.plane_normal),
            }) else {
                return;
            };

            // 动态目标：用自身速度估算本步位移，做相对扫掠；静态/运动学按静止。
            let (target_displacement, target_velocity, target_inv_mass) = if entry.body.is_dynamic()
            {
                let velocity = *entry.body.velocity();
                let inv_mass = if entry.body.is_movable() {
                    entry.body.inverse_mass()
                } else {
                    0.0
                };
                (velocity * dt, velocity, inv_mass)
            } else {
                (Vec3::zero(), Vec3::zero(), 0.0)
            };
            let relative_displacement = displacement - target_displacement;
            // 相对位移太小：交给离散检测，避免静止时每帧扫掠。
            if relative_displacement.magnitude() <= radius * 0.5 {
                return;
            }

            let Some(hit) =
                intersect::sweep_segment_shape(a, b, radius, relative_displacement, &target)
            else {
                return;
            };
            if best.is_none_or(|current| hit.t < current.t) {
                best = Some(CcdHit {
                    t: hit.t,
                    normal: hit.normal,
                    target_body: entry.body_id,
                    target_collider: entry.collider_id,
                    target_inv_mass,
                    target_velocity,
                });
            }
        });

        best
    }
}
