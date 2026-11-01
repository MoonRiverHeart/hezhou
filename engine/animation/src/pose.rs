//! 姿态（Pose）与姿态混合，以及 0.3 的 Blend Tree / IK 预留接口。
//!
//! 姿态就是“每个关节的局部 TRS 数组”，长度与骨架关节数一致。
//! 0.2 提供的混合能力：两段姿态按 alpha 交叉淡入淡出（`Pose::blend`）。
//! 0.2 **不做** Blend Tree / IK，仅保留 [`PoseNode`] / [`IkSolver`] 接口和 TODO。

use crate::math::{Transform, Vec3};
use crate::skeleton::{JointIndex, Skeleton};

/// 采样得到的骨架姿态（局部 TRS）。
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    /// 每个关节的局部变换；下标与 `Skeleton::joints` 对齐。
    pub locals: Vec<Transform>,
}

impl Pose {
    /// 生成指定关节数的单位姿态。
    pub fn identity(joint_count: usize) -> Self {
        Self {
            locals: vec![Transform::IDENTITY; joint_count],
        }
    }

    pub fn from_locals(locals: Vec<Transform>) -> Self {
        Self { locals }
    }

    #[inline]
    pub fn joint_count(&self) -> usize {
        self.locals.len()
    }

    #[inline]
    pub fn joint(&self, index: JointIndex) -> Option<&Transform> {
        self.locals.get(index)
    }

    #[inline]
    pub fn joint_mut(&mut self, index: JointIndex) -> Option<&mut Transform> {
        self.locals.get_mut(index)
    }

    /// 两段姿态按 `alpha` 交叉淡入淡出：T/S 线性，R 球面插值。
    ///
    /// 长度不一致时以较长者为准，缺失项按单位变换处理（避免越界 panic）。
    pub fn blend(a: &Pose, b: &Pose, alpha: f32) -> Pose {
        let alpha = alpha.clamp(0.0, 1.0);
        let count = a.locals.len().max(b.locals.len());
        let mut locals = Vec::with_capacity(count);
        for i in 0..count {
            let ta = a.locals.get(i).copied().unwrap_or(Transform::IDENTITY);
            let tb = b.locals.get(i).copied().unwrap_or(Transform::IDENTITY);
            locals.push(Transform::lerp(&ta, &tb, alpha));
        }
        Pose { locals }
    }

    /// 多段姿态加权混合（权重会自动归一化）。
    ///
    /// 0.2 只在内部/测试中使用；真正的 Blend Tree 在 0.3 通过 [`PoseNode`] 实现。
    pub fn blend_many(poses: &[(&Pose, f32)]) -> Pose {
        let count = poses
            .iter()
            .map(|(pose, _)| pose.joint_count())
            .max()
            .unwrap_or(0);
        let weight_sum: f32 = poses.iter().map(|(_, w)| w.max(0.0)).sum();
        let mut locals = vec![Transform::IDENTITY; count];
        if weight_sum <= 1e-6 {
            return Pose { locals };
        }
        for (pose, weight) in poses {
            let w = weight.max(0.0) / weight_sum;
            if w <= 0.0 {
                continue;
            }
            for (i, slot) in locals.iter_mut().enumerate() {
                let src = pose.locals.get(i).copied().unwrap_or(Transform::IDENTITY);
                // 用逐段 slerp 累加的简化实现：正确性依赖权重归一化，够 0.2 用。
                *slot = Transform::lerp(slot, &src, w);
            }
        }
        Pose { locals }
    }

    /// 便于调试：返回某关节的世界位置。
    pub fn world_position(&self, skeleton: &Skeleton, joint: JointIndex) -> Option<Vec3> {
        let world = skeleton.compute_world_transforms(&self.locals);
        world.get(joint).map(|m| m.transform_point(Vec3::ZERO))
    }
}

/// 动画求值上下文（为 0.3 的动画图/Blend Tree 预留）。
pub struct AnimationContext<'a> {
    pub skeleton: &'a Skeleton,
    /// 本次更新的时间步长（秒）。
    pub delta_time: f32,
}

/// 0.3 预留：动画节点抽象（Clip / BlendTree / Layer 的输出都可视为 Pose）。
///
/// TODO(0.3)：实现 BlendTree 节点、1D/2D 混合空间、动画层与遮罩。
pub trait PoseNode {
    fn evaluate(&self, context: &AnimationContext<'_>) -> Pose;
}

/// IK 目标（0.3 预留）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IkTarget {
    /// 末端关节（例如手/脚）。
    pub joint: JointIndex,
    /// 目标世界位置。
    pub target_position: Vec3,
    /// 权重 0..1，用于 IK/FK 混合。
    pub weight: f32,
}

/// IK 求解器接口（0.3 预留）。
///
/// TODO(0.3)：实现 two-bone IK（腿/手）、look-at、foot placement。
pub trait IkSolver {
    fn solve(
        &self,
        context: &AnimationContext<'_>,
        pose: &mut Pose,
        target: &IkTarget,
    ) -> Result<(), UnsupportedFeature>;
}

/// 0.2 的功能边界错误：调用未实现的 0.3 能力时返回。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedFeature {
    /// 未实现的功能名（例如 `"TwoBoneIk"`）。
    pub feature: &'static str,
}

impl std::fmt::Display for UnsupportedFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "动画功能 `{}` 在 0.2 中未实现（计划 0.3）", self.feature)
    }
}

impl std::error::Error for UnsupportedFeature {}

/// 0.3 预留的 Two-Bone IK 占位实现；0.2 调用会返回 [`UnsupportedFeature`]。
#[derive(Clone, Copy, Debug, Default)]
pub struct TwoBoneIk;

impl IkSolver for TwoBoneIk {
    fn solve(
        &self,
        _context: &AnimationContext<'_>,
        _pose: &mut Pose,
        _target: &IkTarget,
    ) -> Result<(), UnsupportedFeature> {
        Err(UnsupportedFeature {
            feature: "TwoBoneIk",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Quat;
    use crate::skeleton::Skeleton;

    #[test]
    fn pose_blend_interpolates_translation_and_rotation() {
        let a = Pose::from_locals(vec![Transform::from_trs(
            Vec3::new(0.0, 0.0, 0.0),
            Quat::IDENTITY,
        )]);
        let b = Pose::from_locals(vec![Transform::from_trs(
            Vec3::new(10.0, 0.0, 0.0),
            Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_2),
        )]);
        let half = Pose::blend(&a, &b, 0.5);
        assert!(half.locals[0]
            .translation
            .approx_eq(Vec3::new(5.0, 0.0, 0.0), 1e-5));
        let expected_rotation = Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_4);
        assert!(half.locals[0].rotation.approx_eq(expected_rotation, 1e-4));
    }

    #[test]
    fn pose_world_position_follows_hierarchy() {
        let mut skeleton = Skeleton::new("s");
        skeleton.add_root_joint(
            "root",
            Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
        );
        skeleton
            .add_joint(
                "tip",
                0,
                Transform::from_trs(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY),
            )
            .unwrap();
        let pose = skeleton.default_pose();
        let pose = Pose::from_locals(pose);
        let tip = pose.world_position(&skeleton, 1).unwrap();
        assert!(tip.approx_eq(Vec3::new(0.0, 3.0, 0.0), 1e-5), "got {tip:?}");
    }

    #[test]
    fn two_bone_ik_is_explicitly_unsupported_in_0_2() {
        let mut skeleton = Skeleton::new("s");
        skeleton.add_root_joint("root", Transform::IDENTITY);
        let mut pose = Pose::identity(1);
        let ctx = AnimationContext {
            skeleton: &skeleton,
            delta_time: 0.016,
        };
        let target = IkTarget {
            joint: 0,
            target_position: Vec3::ZERO,
            weight: 1.0,
        };
        assert_eq!(
            TwoBoneIk.solve(&ctx, &mut pose, &target),
            Err(UnsupportedFeature {
                feature: "TwoBoneIk"
            })
        );
    }
}
