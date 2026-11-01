//! 骨骼层级：Skeleton / Joint / Bone 与逆绑定矩阵。
//!
//! 术语说明（glTF 对齐）：
//! - **Joint**（关节）：骨架树上的节点，携带绑定姿态局部 TRS 与逆绑定矩阵（IBM）。
//! - **Bone**（骨骼）：与 Joint 同义；glTF 中 skin.joints 引用的是节点索引，
//!   引擎内部统一用 `Joint`，`Bone` 仅作为别名保留，方便美术/玩法代码理解。
//! - 关节索引 `JointIndex` 就是 `Skeleton::joints` 的下标，0-based，稳定不变。
//!
//! 约定：
//! - `parent: Option<JointIndex>`，根关节为 `None`；父索引必须指向**已存在**的关节，
//!   因此按拓扑顺序（父先子后）构建时天然无环；`validate()` 仍会做完整校验。
//! - `bind_local` 是该关节在绑定姿态下的**局部** TRS（相对父关节）。
//! - `inverse_bind` 是**蒙皮空间 → 关节局部空间**的逆绑定矩阵（glTF `inverseBindMatrices`）。
//! - 蒙皮矩阵 = `joint_world_matrix * inverse_bind`，见 `skin::Skin::skin_matrices`。

use crate::math::{Mat4, Transform, Vec3};
use std::collections::HashMap;

/// 关节索引（`Skeleton::joints` 的下标）。
pub type JointIndex = usize;

/// 骨架节点：名称 + 父索引 + 绑定姿态 + 逆绑定矩阵。
#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    /// 关节名（glTF 节点名；可为空字符串）。
    pub name: String,
    /// 父关节索引；根关节为 `None`。
    pub parent: Option<JointIndex>,
    /// 绑定姿态下的局部 TRS。
    pub bind_local: Transform,
    /// 逆绑定矩阵（蒙皮空间 → 该关节空间）。
    pub inverse_bind: Mat4,
}

impl Joint {
    pub fn new(
        name: impl Into<String>,
        parent: Option<JointIndex>,
        bind_local: Transform,
        inverse_bind: Mat4,
    ) -> Self {
        Self {
            name: name.into(),
            parent,
            bind_local,
            inverse_bind,
        }
    }

    /// 仅用名称和父索引构造（绑定姿态为单位 TRS，IBM 为单位矩阵）。
    pub fn simple(name: impl Into<String>, parent: Option<JointIndex>) -> Self {
        Self::new(name, parent, Transform::IDENTITY, Mat4::IDENTITY)
    }
}

/// Bone 与 Joint 同义（glTF 术语），保留别名便于业务代码表达。
pub type Bone = Joint;

/// 骨架错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkeletonError {
    /// 父关节索引越界。
    InvalidParent {
        joint: JointIndex,
        parent: JointIndex,
    },
    /// 关节层级中存在环。
    Cycle { joint: JointIndex },
    /// 重复的关节名（`try_add_joint` 会拒绝）。
    DuplicateJointName(String),
    /// 空骨架。
    EmptySkeleton,
    /// 该关节没有可用父节点（内部错误兜底）。
    InvalidHierarchy,
}

impl std::fmt::Display for SkeletonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidParent { joint, parent } => {
                write!(f, "关节 {joint} 的父索引 {parent} 越界")
            }
            Self::Cycle { joint } => write!(f, "关节 {joint} 的父链存在环"),
            Self::DuplicateJointName(name) => write!(f, "重复的关节名: {name}"),
            Self::EmptySkeleton => write!(f, "骨架为空"),
            Self::InvalidHierarchy => write!(f, "骨架层级非法"),
        }
    }
}

impl std::error::Error for SkeletonError {}

/// 骨架：一组按拓扑/导入顺序存放的关节。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Skeleton {
    /// 骨架名（通常是 glTF skin 名或角色名）。
    pub name: String,
    /// 关节列表；下标即 `JointIndex`。
    pub joints: Vec<Joint>,
}

impl Skeleton {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            joints: Vec::new(),
        }
    }

    /// 用完整关节列表构造（不做校验，导入器可用；运行时请调用 `validate`）。
    pub fn with_joints(name: impl Into<String>, joints: Vec<Joint>) -> Self {
        Self {
            name: name.into(),
            joints,
        }
    }

    #[inline]
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    /// 追加一个根关节，返回其索引。
    pub fn add_root_joint(&mut self, name: impl Into<String>, bind_local: Transform) -> JointIndex {
        self.joints
            .push(Joint::new(name, None, bind_local, Mat4::IDENTITY));
        self.joints.len() - 1
    }

    /// 追加一个子关节；父索引非法或重名时返回错误。
    pub fn add_joint(
        &mut self,
        name: impl Into<String>,
        parent: JointIndex,
        bind_local: Transform,
    ) -> Result<JointIndex, SkeletonError> {
        let name = name.into();
        if parent >= self.joints.len() {
            return Err(SkeletonError::InvalidParent {
                joint: self.joints.len(),
                parent,
            });
        }
        if self.joints.iter().any(|j| j.name == name) {
            return Err(SkeletonError::DuplicateJointName(name));
        }
        self.joints
            .push(Joint::new(name, Some(parent), bind_local, Mat4::IDENTITY));
        Ok(self.joints.len() - 1)
    }

    #[inline]
    pub fn joint(&self, index: JointIndex) -> Option<&Joint> {
        self.joints.get(index)
    }

    #[inline]
    pub fn joint_mut(&mut self, index: JointIndex) -> Option<&mut Joint> {
        self.joints.get_mut(index)
    }

    /// 按名称查找关节索引（重名时返回第一个）。
    pub fn find(&self, name: &str) -> Option<JointIndex> {
        self.joints.iter().position(|j| j.name == name)
    }

    /// 名称 → 索引映射（供 glTF/asset 导入转换用）。
    pub fn name_index_map(&self) -> HashMap<&str, JointIndex> {
        self.joints
            .iter()
            .enumerate()
            .map(|(i, j)| (j.name.as_str(), i))
            .collect()
    }

    /// 直接子关节索引列表。
    pub fn children(&self, index: JointIndex) -> Vec<JointIndex> {
        self.joints
            .iter()
            .enumerate()
            .filter_map(|(i, j)| (j.parent == Some(index)).then_some(i))
            .collect()
    }

    /// 根关节索引列表。
    pub fn roots(&self) -> Vec<JointIndex> {
        self.joints
            .iter()
            .enumerate()
            .filter_map(|(i, j)| j.parent.is_none().then_some(i))
            .collect()
    }

    /// 关节深度（根为 0）；父链非法时返回 `None`。
    pub fn depth(&self, index: JointIndex) -> Option<usize> {
        let mut depth = 0;
        let mut cursor = index;
        let mut steps = 0;
        while let Some(parent) = self.joints.get(cursor)?.parent {
            depth += 1;
            cursor = parent;
            steps += 1;
            if steps > self.joints.len() {
                return None;
            }
        }
        Some(depth)
    }

    /// 校验层级与父索引：父索引合法、无环、非空骨架。
    pub fn validate(&self) -> Result<(), SkeletonError> {
        if self.joints.is_empty() {
            return Err(SkeletonError::EmptySkeleton);
        }
        let count = self.joints.len();
        for (i, joint) in self.joints.iter().enumerate() {
            if let Some(parent) = joint.parent {
                if parent >= count {
                    return Err(SkeletonError::InvalidParent { joint: i, parent });
                }
            }
            // 沿父链向上走，最多 count 步；超过说明有环。
            let mut cursor = i;
            for _ in 0..=count {
                match self.joints[cursor].parent {
                    Some(parent) => cursor = parent,
                    None => break,
                }
            }
            if self.joints[cursor].parent.is_some() {
                return Err(SkeletonError::Cycle { joint: i });
            }
        }
        Ok(())
    }

    /// 拓扑顺序（父先子后）。层级非法时把剩余关节按原顺序追加，保证长度不变。
    // 说明：需要按下标访问 `self.joints` 与 `placed` 两个数组，用索引循环最直观。
    #[allow(clippy::needless_range_loop)]
    pub fn topological_order(&self) -> Vec<JointIndex> {
        let count = self.joints.len();
        let mut order = Vec::with_capacity(count);
        let mut placed = vec![false; count];
        for _ in 0..count {
            let mut progressed = false;
            for i in 0..count {
                if placed[i] {
                    continue;
                }
                let ready = !matches!(
                    self.joints[i].parent,
                    Some(parent) if parent < count && !placed[parent]
                );
                if ready {
                    placed[i] = true;
                    order.push(i);
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
        for i in 0..count {
            if !placed[i] {
                order.push(i);
            }
        }
        order
    }

    /// 绑定姿态局部 TRS（每个关节的 `bind_local`）。
    pub fn default_pose(&self) -> Vec<Transform> {
        self.joints.iter().map(|j| j.bind_local).collect()
    }

    /// 绑定姿态的世界矩阵（由 `bind_local` 逐层累乘，根关节以单位矩阵为父）。
    pub fn world_bind_matrices(&self) -> Vec<Mat4> {
        self.compute_world_transforms(&self.default_pose())
    }

    /// 由局部姿态计算世界矩阵数组；长度按关节数，缺失项回退到绑定姿态。
    ///
    /// 内部按拓扑顺序累乘，因此本地姿态顺序无关。
    pub fn compute_world_transforms(&self, local_pose: &[Transform]) -> Vec<Mat4> {
        let count = self.joints.len();
        let mut world = vec![Mat4::IDENTITY; count];
        let mut done = vec![false; count];
        for index in self.topological_order() {
            let joint = &self.joints[index];
            let local = local_pose.get(index).copied().unwrap_or(joint.bind_local);
            let local_matrix = local.to_matrix();
            let parent_world = match joint.parent {
                Some(parent) if parent < count && done[parent] => world[parent],
                _ => Mat4::IDENTITY,
            };
            world[index] = parent_world * local_matrix;
            done[index] = true;
        }
        world
    }

    /// 由绑定姿态反算每个关节的逆绑定矩阵（覆盖 `Joint::inverse_bind`）。
    ///
    /// 导入器没提供 IBM 时用这个补全；奇异矩阵回退为单位矩阵。
    pub fn rebuild_inverse_bind_matrices(&mut self) -> Result<(), SkeletonError> {
        self.validate()?;
        let world = self.world_bind_matrices();
        for (joint, world_matrix) in self.joints.iter_mut().zip(world) {
            joint.inverse_bind = world_matrix.inverse().unwrap_or(Mat4::IDENTITY);
        }
        Ok(())
    }

    /// 绑定姿态下的蒙皮矩阵 `world_bind * inverse_bind`（理论上为单位矩阵）。
    pub fn bind_skin_matrices(&self) -> Vec<Mat4> {
        let world = self.world_bind_matrices();
        self.joints
            .iter()
            .zip(world)
            .map(|(joint, w)| w * joint.inverse_bind)
            .collect()
    }

    /// 关节的绑定世界位置（便于调试/命中盒挂点）。
    pub fn bind_world_position(&self, index: JointIndex) -> Option<Vec3> {
        let world = self.world_bind_matrices();
        world.get(index).map(|m| m.transform_point(Vec3::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Quat;

    fn chain_skeleton() -> Skeleton {
        let mut skeleton = Skeleton::new("test_chain");
        let root = skeleton.add_root_joint("root", Transform::IDENTITY);
        let spine = skeleton
            .add_joint(
                "spine",
                root,
                Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
            )
            .unwrap();
        skeleton
            .add_joint(
                "head",
                spine,
                Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
            )
            .unwrap();
        skeleton
    }

    #[test]
    fn world_transforms_multiply_parent_chain() {
        let mut skeleton = chain_skeleton();
        skeleton.joints[2].bind_local.translation = Vec3::new(0.0, 1.5, 0.0);
        let world = skeleton.world_bind_matrices();
        assert_eq!(world.len(), 3);
        let head_pos = world[2].transform_point(Vec3::ZERO);
        assert!(
            head_pos.approx_eq(Vec3::new(0.0, 2.5, 0.0), 1e-5),
            "got {head_pos:?}"
        );
    }

    #[test]
    fn validate_rejects_cycle_and_bad_parent() {
        let mut skeleton = chain_skeleton();
        // 人为制造环：root 的父指向自己。
        skeleton.joints[0].parent = Some(0);
        assert!(matches!(
            skeleton.validate(),
            Err(SkeletonError::Cycle { .. })
        ));

        let mut skeleton = chain_skeleton();
        skeleton.joints[0].parent = Some(99);
        assert!(matches!(
            skeleton.validate(),
            Err(SkeletonError::InvalidParent { .. })
        ));
    }

    #[test]
    fn topological_order_parent_before_child() {
        // 子先加入数组，父后加入，仍应得到父先子后的顺序。
        let mut skeleton = Skeleton::new("reordered");
        skeleton.joints.push(Joint::simple("child", Some(1)));
        skeleton.joints.push(Joint::simple("parent", None));
        let order = skeleton.topological_order();
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn bind_skin_matrices_are_identity_at_rest_pose() {
        let mut skeleton = chain_skeleton();
        skeleton.rebuild_inverse_bind_matrices().unwrap();
        for m in skeleton.bind_skin_matrices() {
            assert!(m.approx_eq(&Mat4::IDENTITY, 1e-4));
        }
    }
}
