//! CCD 基础：高速球/胶囊对静态碰撞体的扫掠检测。
//!
//! 0.3 第一版只处理“动态球/胶囊 vs 静态/运动学目标”，并且需要同时满足：
//! - `PhysicsWorld::set_ccd_enabled(true)` 全局开关；
//! - 刚体 `RigidBody::set_ccd_enabled(true)`；
//! - 单步位移足够大（大于半径的一半），否则交给离散检测。
//!
//! 动态-动态 CCD、旋转盒 CCD、子步进（sub-stepping）留后续版本。

use crate::collider::layers_interact;
use crate::intersect::{self, TargetShape};
use crate::world::PhysicsWorld;
use hezhou_core::math::Vec3;

/// CCD 扫掠命中（内部）。
#[derive(Clone, Copy, Debug)]
pub(crate) struct CcdHit {
    /// 命中发生在位移的比例（0..=1）。
    pub t: f32,
    /// 从目标指向运动刚体的推出法线。
    pub normal: Vec3,
    /// 命中的目标刚体/碰撞体（保留给后续 CCD 事件上报使用）。
    #[allow(dead_code)]
    pub target_body: u64,
    #[allow(dead_code)]
    pub target_collider: u64,
}

impl PhysicsWorld {
    /// 对指定刚体做一次 CCD 扫掠，返回最近命中。
    pub(crate) fn ccd_sweep(&self, body_id: u64, displacement: Vec3) -> Option<CcdHit> {
        let length = displacement.magnitude();
        if length <= 1e-6 {
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

        // 位移太小说明是常规接触/静止，交给离散求解，避免每帧额外扫掠。
        if length <= radius * 0.5 {
            return None;
        }

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
            // 0.3 第一版只对静态/运动学目标做 CCD。
            if entry.body.is_dynamic() {
                return;
            }
            if !layers_interact(layer, mask, entry.collider.layer(), entry.collider.mask()) {
                return;
            }
            let Some(target) = TargetShape::from_collider(
                entry.collider.shape(),
                entry.position,
                entry.plane_normal,
            ) else {
                return;
            };
            let Some(hit) = intersect::sweep_segment_shape(a, b, radius, displacement, &target)
            else {
                return;
            };
            if best.is_none_or(|current| hit.t < current.t) {
                best = Some(CcdHit {
                    t: hit.t,
                    normal: hit.normal,
                    target_body: entry.body_id,
                    target_collider: entry.collider_id,
                });
            }
        });

        best
    }
}
