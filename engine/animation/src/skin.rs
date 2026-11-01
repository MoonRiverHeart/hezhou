//! 蒙皮：Skin / 顶点权重 / 逆绑定矩阵 / CPU 蒙皮求值。
//!
//! 蒙皮矩阵（joint palette）定义：
//! ```text
//! skin_matrix[i] = world_joint_matrix[skin.joints[i]] * inverse_bind_matrices[i]
//! ```
//! 顶点在**模型空间**完成蒙皮，随后再乘角色实体自身的 model matrix。
//!
//! 顶点最多受 4 根骨骼影响（与 glTF 常见导出、GPU 端点一致）：
//! - `VertexWeights` 保存 `[u16; 4] + [f32; 4]`，权重和为 1；
//! - GPU 顶点布局里 joint index 会扩展为 `[u32; 4]`，见 [`crate::gpu::SkinnedVertex`]。

use crate::math::{Mat4, Vec3};
use crate::skeleton::{JointIndex, Skeleton};

/// 每个顶点最多受多少根骨骼影响。
pub const MAX_JOINT_INFLUENCES: usize = 4;

/// 一个顶点的骨骼权重（最多 4 个影响）。
///
/// 索引语义：`joint_indices[i]` 是 **palette 索引**，即 `Skin::joints` 的下标
/// （0..skin.joint_count()），而不是 `Skeleton::joints` 的全局索引。
/// glTF 的 `JOINTS_0` 顶点属性同样以 skin.joints 下标编码，因此可直接对应。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexWeights {
    /// palette 索引（`Skin::joints` 下标；超出 palette 长度的影响会被忽略）。
    pub joint_indices: [u16; MAX_JOINT_INFLUENCES],
    /// 对应权重；应满足和为 1，否则采样前会自动归一化。
    pub weights: [f32; MAX_JOINT_INFLUENCES],
}

impl Default for VertexWeights {
    fn default() -> Self {
        Self::zero()
    }
}

impl VertexWeights {
    /// 无影响（顶点保持原位，相当于刚性绑定到模型空间）。
    pub const fn zero() -> Self {
        Self {
            joint_indices: [0; MAX_JOINT_INFLUENCES],
            weights: [0.0; MAX_JOINT_INFLUENCES],
        }
    }

    /// 单骨骼刚性绑定（参数为 palette 索引）。
    pub fn single(joint: JointIndex) -> Self {
        let mut weights = Self::zero();
        weights.joint_indices[0] = joint as u16;
        weights.weights[0] = 1.0;
        weights
    }

    /// 由 `(palette 索引, 权重)` 列表构造：取权重最大的 4 个并归一化。
    pub fn from_pairs(pairs: &[(JointIndex, f32)]) -> Self {
        let mut sorted: Vec<(JointIndex, f32)> =
            pairs.iter().copied().filter(|(_, w)| *w > 0.0).collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut out = Self::zero();
        for (slot, (joint, weight)) in sorted.iter().take(MAX_JOINT_INFLUENCES).enumerate() {
            out.joint_indices[slot] = *joint as u16;
            out.weights[slot] = *weight;
        }
        out.normalized()
    }

    /// 归一化权重；总和过小时返回全零权重。
    pub fn normalized(self) -> Self {
        let total: f32 = self.weights.iter().sum();
        let mut out = self;
        if total > 1e-6 {
            for w in &mut out.weights {
                *w /= total;
            }
        } else {
            out.weights = [0.0; MAX_JOINT_INFLUENCES];
        }
        out
    }

    /// 权重总和。
    pub fn total_weight(&self) -> f32 {
        self.weights.iter().sum()
    }

    /// 是否有效（存在正权重）。
    pub fn is_valid(&self) -> bool {
        self.total_weight() > 1e-6
    }

    /// 转换为 GPU 顶点属性所需的 `[u32; 4]` 联合索引。
    pub fn to_u32_joint_indices(&self) -> [u32; MAX_JOINT_INFLUENCES] {
        [
            self.joint_indices[0] as u32,
            self.joint_indices[1] as u32,
            self.joint_indices[2] as u32,
            self.joint_indices[3] as u32,
        ]
    }
}

/// 蒙皮数据：参与蒙皮的关节列表 + 对应逆绑定矩阵。
///
/// 一个 mesh 可以绑定多个 skin（不同部件），0.2 先支持“一个 skin 一套关节”。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Skin {
    /// 蒙皮名（通常来自 glTF skin 名）。
    pub name: String,
    /// 参与蒙皮的关节索引（全局 `Skeleton::joints` 下标）。
    pub joints: Vec<JointIndex>,
    /// 与 `joints` 一一对应的逆绑定矩阵。
    pub inverse_bind_matrices: Vec<Mat4>,
}

impl Skin {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            joints: Vec::new(),
            inverse_bind_matrices: Vec::new(),
        }
    }

    /// 由骨架生成 1:1 蒙皮；逆绑定矩阵取自 `Skeleton::world_bind_matrices()` 的逆。
    ///
    /// 若骨架已由 glTF 导入并带有 IBM，建议直接用 `with_joint_indices` 传入，
    /// 避免重复求逆带来的精度差异。
    pub fn from_skeleton(skeleton: &Skeleton) -> Self {
        let world = skeleton.world_bind_matrices();
        let inverse_bind_matrices = world
            .iter()
            .map(|m| m.inverse().unwrap_or(Mat4::IDENTITY))
            .collect();
        Self {
            name: skeleton.name.clone(),
            joints: (0..skeleton.joint_count()).collect(),
            inverse_bind_matrices,
        }
    }

    /// 用任意关节子集构造蒙皮。
    pub fn with_joint_indices(
        name: impl Into<String>,
        joints: Vec<JointIndex>,
        inverse_bind_matrices: Vec<Mat4>,
    ) -> Self {
        Self {
            name: name.into(),
            joints,
            inverse_bind_matrices,
        }
    }

    #[inline]
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// 校验：关节数一致、索引不越界（`skeleton` 为骨架定义）。
    pub fn validate(&self, skeleton: &Skeleton) -> Result<(), SkinError> {
        if self.joints.len() != self.inverse_bind_matrices.len() {
            return Err(SkinError::JointCountMismatch {
                joints: self.joints.len(),
                inverse_bind: self.inverse_bind_matrices.len(),
            });
        }
        for (slot, joint) in self.joints.iter().enumerate() {
            if *joint >= skeleton.joint_count() {
                return Err(SkinError::InvalidJointIndex {
                    slot,
                    joint: *joint,
                    skeleton_joint_count: skeleton.joint_count(),
                });
            }
        }
        Ok(())
    }

    /// 计算 GPU joint palette：`world_joint * inverse_bind`。
    ///
    /// `world_joint_matrices` 长度应等于骨架关节数；缺失的关节按单位矩阵处理。
    pub fn skin_matrices(&self, world_joint_matrices: &[Mat4]) -> Vec<Mat4> {
        self.joints
            .iter()
            .zip(self.inverse_bind_matrices.iter())
            .map(|(joint, inverse_bind)| {
                let world = world_joint_matrices
                    .get(*joint)
                    .copied()
                    .unwrap_or(Mat4::IDENTITY);
                world * *inverse_bind
            })
            .collect()
    }

    /// 绑定姿态下的蒙皮矩阵（应接近单位矩阵，用于自检/测试）。
    pub fn bind_skin_matrices(&self, skeleton: &Skeleton) -> Vec<Mat4> {
        self.skin_matrices(&skeleton.world_bind_matrices())
    }
}

/// 蒙皮错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkinError {
    /// `joints` 与 `inverse_bind_matrices` 数量不一致。
    JointCountMismatch { joints: usize, inverse_bind: usize },
    /// 关节索引超出骨架范围。
    InvalidJointIndex {
        slot: usize,
        joint: JointIndex,
        skeleton_joint_count: usize,
    },
}

impl std::fmt::Display for SkinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JointCountMismatch {
                joints,
                inverse_bind,
            } => write!(
                f,
                "skin 关节数({joints})与逆绑定矩阵数({inverse_bind})不一致"
            ),
            Self::InvalidJointIndex {
                slot,
                joint,
                skeleton_joint_count,
            } => write!(
                f,
                "skin 第 {slot} 个关节索引 {joint} 超出骨架关节数 {skeleton_joint_count}"
            ),
        }
    }
}

impl std::error::Error for SkinError {}

/// CPU 蒙皮：把模型空间顶点变换到蒙皮后的模型空间。
///
/// 注意：`weights.joint_indices` 是 palette 索引（见 [`VertexWeights`]），
/// 传入的 `skin_matrices` 必须是同一蒙皮的 palette（`Skin::skin_matrices` 的输出）。
pub fn skin_position(position: Vec3, weights: &VertexWeights, skin_matrices: &[Mat4]) -> Vec3 {
    let normalized = weights.normalized();
    if !normalized.is_valid() {
        return position;
    }
    let mut out = Vec3::ZERO;
    for slot in 0..MAX_JOINT_INFLUENCES {
        let weight = normalized.weights[slot];
        if weight <= 0.0 {
            continue;
        }
        let index = normalized.joint_indices[slot] as usize;
        if let Some(matrix) = skin_matrices.get(index) {
            out = out + matrix.transform_point(position) * weight;
        }
    }
    out
}

/// CPU 蒙皮：法线（用蒙皮矩阵线性部分变换后归一化，0.2 不做逆转置）。
pub fn skin_normal(normal: Vec3, weights: &VertexWeights, skin_matrices: &[Mat4]) -> Vec3 {
    let normalized = weights.normalized();
    if !normalized.is_valid() {
        return normal;
    }
    let mut out = Vec3::ZERO;
    for slot in 0..MAX_JOINT_INFLUENCES {
        let weight = normalized.weights[slot];
        if weight <= 0.0 {
            continue;
        }
        let index = normalized.joint_indices[slot] as usize;
        if let Some(matrix) = skin_matrices.get(index) {
            out = out + matrix.transform_vector(normal) * weight;
        }
    }
    out.normalize_or_zero()
}

/// CPU 蒙皮：一次求值位置 + 法线（无 GPU 时的回退路径/单元测试用）。
pub fn skin_vertex(
    position: Vec3,
    normal: Vec3,
    weights: &VertexWeights,
    skin_matrices: &[Mat4],
) -> (Vec3, Vec3) {
    (
        skin_position(position, weights, skin_matrices),
        skin_normal(normal, weights, skin_matrices),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Quat, Transform};
    use crate::skeleton::Skeleton;

    fn two_joint_skin() -> (Skeleton, Skin) {
        let mut skeleton = Skeleton::new("two_joints");
        // 两个根关节（互不影响），便于独立验证权重混合。
        skeleton.add_root_joint("a", Transform::IDENTITY);
        skeleton.add_root_joint("b", Transform::IDENTITY);
        let skin = Skin::from_skeleton(&skeleton);
        (skeleton, skin)
    }

    #[test]
    fn skin_matrix_at_rest_pose_is_identity() {
        let (skeleton, skin) = two_joint_skin();
        for m in skin.bind_skin_matrices(&skeleton) {
            assert!(m.approx_eq(&Mat4::IDENTITY, 1e-4));
        }
    }

    #[test]
    fn cpu_skinning_blends_two_joints() {
        let (skeleton, skin) = two_joint_skin();
        // 两个根关节分别向 +X / +Y 平移 10。
        let pose = vec![
            Transform::from_trs(Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY),
            Transform::from_trs(Vec3::new(0.0, 10.0, 0.0), Quat::IDENTITY),
        ];
        let world = skeleton.compute_world_transforms(&pose);
        let palette = skin.skin_matrices(&world);
        let weights = VertexWeights::from_pairs(&[(0, 0.25), (1, 0.75)]);
        let skinned = skin_position(Vec3::ZERO, &weights, &palette);
        // 0.25 * (10,0) + 0.75 * (0,10) = (2.5, 7.5)
        assert!(
            skinned.approx_eq(Vec3::new(2.5, 7.5, 0.0), 1e-4),
            "got {skinned:?}"
        );
    }

    #[test]
    fn weights_are_normalized_and_top4_kept() {
        let weights = VertexWeights::from_pairs(&[(0, 2.0), (1, 1.0), (2, 1.0)]);
        assert!((weights.total_weight() - 1.0).abs() < 1e-6);
        let many = VertexWeights::from_pairs(&[(0, 5.0), (1, 4.0), (2, 3.0), (3, 2.0), (4, 1.0)]);
        assert!((many.total_weight() - 1.0).abs() < 1e-6);
        assert_eq!(many.joint_indices[0], 0);
        assert_eq!(many.joint_indices[3], 3, "权重最小的第 5 个影响应被丢弃");
        assert!(!VertexWeights::zero().is_valid());
    }
}
