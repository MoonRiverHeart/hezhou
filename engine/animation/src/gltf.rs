//! glTF 导入数据契约：本 crate **不依赖** `gltf` crate，也不负责解析文件。
//!
//! 职责划分（与 `engine/asset` 的约定）：
//! - `engine/asset` 负责用第三方库解析 `.gltf/.glb`，把结果填充成本模块的
//!   [`GltfAsset`] / [`GltfSkeleton`] / [`GltfSkin`] / [`GltfAnimation`]；
//! - `hezhou-animation` 负责把这些**内存数据**转换成运行时结构
//!   （[`Skeleton`] / [`Skin`] / [`AnimationClip`]）并做校验；
//! - 0.2 不反向序列化，也不处理纹理/材质——那些仍归 asset 模块。
//!
//! 索引约定（重要）：
//! - `GltfNode.parent` / `GltfChannel.target_node` 使用 **glTF 节点索引**（整个节点数组的下标）；
//! - `GltfSkeleton.joint_nodes` 是 skin.joints 顺序，转换后其**位置**就是 `JointIndex`；
//! - 顶点权重、动画通道都通过节点索引映射到 `JointIndex`，再由本模块转换。
//!
//! 0.2 支持范围：
//! - 平移/旋转/缩放通道，LINEAR / STEP 插值；
//! - CUBICSPLINE 数据可以在契约中表达（`Interpolation::CubicSpline`），但采样回退线性（TODO 0.3）。

use std::collections::HashMap;

use crate::clip::{
    AnimationClip, AnimationEvent, Channel, ChannelKind, Interpolation, KeyValue, Keyframe,
};
use crate::math::{Mat4, Quat, Transform, Vec3};
use crate::skeleton::{Joint, JointIndex, Skeleton, SkeletonError};
use crate::skin::{Skin, SkinError};

/// glTF 节点（只保留骨骼动画需要的最小字段）。
#[derive(Clone, Debug, PartialEq)]
pub struct GltfNode {
    /// 节点名（glTF `node.name`，可能为空）。
    pub name: String,
    /// 父节点索引（glTF 节点数组下标）；根节点为 `None`。
    pub parent: Option<usize>,
    /// 局部平移。
    pub translation: Vec3,
    /// 局部旋转（四元数 xyzw）。
    pub rotation: Quat,
    /// 局部缩放。
    pub scale: Vec3,
}

impl GltfNode {
    pub fn new(name: impl Into<String>, parent: Option<usize>) -> Self {
        Self {
            name: name.into(),
            parent,
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// 该节点的局部 TRS。
    pub fn local_transform(&self) -> Transform {
        Transform::new(self.translation, self.rotation, self.scale)
    }
}

/// glTF skin 的骨骼数据（对应 glTF `skin` + 被引用的节点）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GltfSkeleton {
    /// 骨架名（可用 glTF skin.name 或文件名）。
    pub name: String,
    /// 关节节点列表（顺序 = glTF `skin.joints` 顺序）。
    pub joint_nodes: Vec<usize>,
    /// 与 `joint_nodes` 一一对应的逆绑定矩阵（glTF `inverseBindMatrices`）。
    pub inverse_bind_matrices: Vec<Mat4>,
    /// 骨骼节点索引 → 节点（可为空；为空时按 `joint_nodes` 顺序生成占位节点）。
    pub nodes: Vec<GltfNode>,
}

impl GltfSkeleton {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            joint_nodes: Vec::new(),
            inverse_bind_matrices: Vec::new(),
            nodes: Vec::new(),
        }
    }

    /// 节点索引 → 关节索引映射（关节顺序即 `joint_nodes` 顺序）。
    pub fn node_to_joint_map(&self) -> HashMap<usize, JointIndex> {
        self.joint_nodes
            .iter()
            .enumerate()
            .map(|(joint_index, node_index)| (*node_index, joint_index))
            .collect()
    }

    /// 转换为运行时光骨架。
    ///
    /// 逆绑定矩阵缺失的关节先用单位矩阵占位，之后可调用
    /// [`Skeleton::rebuild_inverse_bind_matrices`] 统一补全。
    pub fn to_skeleton(&self) -> Result<Skeleton, AnimationImportError> {
        if self.joint_nodes.is_empty() {
            return Err(AnimationImportError::EmptySkeleton);
        }
        if !self.inverse_bind_matrices.is_empty()
            && self.inverse_bind_matrices.len() != self.joint_nodes.len()
        {
            return Err(AnimationImportError::InverseBindCountMismatch {
                joints: self.joint_nodes.len(),
                inverse_bind: self.inverse_bind_matrices.len(),
            });
        }
        let node_to_joint = self.node_to_joint_map();
        let mut joints = Vec::with_capacity(self.joint_nodes.len());
        for (joint_index, node_index) in self.joint_nodes.iter().enumerate() {
            let node = self
                .nodes
                .get(*node_index)
                .ok_or(AnimationImportError::UnknownNode { node: *node_index })?;
            let parent = match node.parent {
                Some(parent_node) => node_to_joint.get(&parent_node).copied(),
                None => None,
            };
            let inverse_bind = self
                .inverse_bind_matrices
                .get(joint_index)
                .copied()
                .unwrap_or(Mat4::IDENTITY);
            joints.push(Joint::new(
                node.name.clone(),
                parent,
                node.local_transform(),
                inverse_bind,
            ));
        }
        let skeleton = Skeleton::with_joints(self.name.clone(), joints);
        skeleton
            .validate()
            .map_err(AnimationImportError::Skeleton)?;
        Ok(skeleton)
    }
}

/// glTF skin（一个 mesh 可以引用一个 skin）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GltfSkin {
    /// 蒙皮名（glTF `skin.name`）。
    pub name: String,
    /// 参与蒙皮的关节节点索引（glTF `skin.joints`，这里用节点索引表达）。
    pub joint_nodes: Vec<usize>,
    /// 与 `joint_nodes` 一一对应的逆绑定矩阵。
    pub inverse_bind_matrices: Vec<Mat4>,
}

impl GltfSkin {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            joint_nodes: Vec::new(),
            inverse_bind_matrices: Vec::new(),
        }
    }

    /// 通过节点→关节映射转换为运行时蒙皮。
    pub fn to_skin(
        &self,
        node_to_joint: &HashMap<usize, JointIndex>,
    ) -> Result<Skin, AnimationImportError> {
        if self.joint_nodes.len() != self.inverse_bind_matrices.len() {
            return Err(AnimationImportError::InverseBindCountMismatch {
                joints: self.joint_nodes.len(),
                inverse_bind: self.inverse_bind_matrices.len(),
            });
        }
        let mut joints = Vec::with_capacity(self.joint_nodes.len());
        for node_index in &self.joint_nodes {
            let joint = node_to_joint
                .get(node_index)
                .copied()
                .ok_or(AnimationImportError::UnknownJointNode { node: *node_index })?;
            joints.push(joint);
        }
        Ok(Skin::with_joint_indices(
            self.name.clone(),
            joints,
            self.inverse_bind_matrices.clone(),
        ))
    }
}

/// glTF 动画通道（对应 glTF `animation.channels` + `samplers`）。
///
/// `values` 是扁平数组：
/// - Translation / Scale：每个关键帧 3 个 f32（xyz）；
/// - Rotation：每个关键帧 4 个 f32（xyzw）；
/// - CUBICSPLINE：每个关键帧 9/12 个 f32（in-tangent + value + out-tangent），0.2 不支持，会报错。
#[derive(Clone, Debug, PartialEq)]
pub struct GltfChannel {
    /// 目标节点索引（glTF 节点数组下标，需要映射到关节）。
    pub target_node: usize,
    /// 驱动属性。
    pub kind: ChannelKind,
    /// 插值方式。
    pub interpolation: Interpolation,
    /// 关键帧时间（秒，升序）。
    pub times: Vec<f32>,
    /// 关键帧值（扁平数组，见类型文档）。
    pub values: Vec<f32>,
}

impl GltfChannel {
    pub fn new(target_node: usize, kind: ChannelKind) -> Self {
        Self {
            target_node,
            kind,
            interpolation: Interpolation::Linear,
            times: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn translation(target_node: usize) -> Self {
        Self::new(target_node, ChannelKind::Translation)
    }

    pub fn rotation(target_node: usize) -> Self {
        Self::new(target_node, ChannelKind::Rotation)
    }

    pub fn scale(target_node: usize) -> Self {
        Self::new(target_node, ChannelKind::Scale)
    }

    pub fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// 每关键帧 f32 数量。
    fn value_stride(&self) -> usize {
        match self.kind {
            ChannelKind::Translation | ChannelKind::Scale => 3,
            ChannelKind::Rotation => 4,
        }
    }

    /// 转换为运行时通道。
    pub fn to_channel(
        &self,
        node_to_joint: &HashMap<usize, JointIndex>,
    ) -> Result<Channel, AnimationImportError> {
        let target_joint = node_to_joint.get(&self.target_node).copied().ok_or(
            AnimationImportError::UnknownJointNode {
                node: self.target_node,
            },
        )?;
        if self.interpolation == Interpolation::CubicSpline {
            return Err(AnimationImportError::UnsupportedInterpolation {
                interpolation: self.interpolation,
            });
        }
        let stride = self.value_stride();
        if stride == 0 || self.values.len() != self.times.len() * stride {
            return Err(AnimationImportError::ChannelValueCountMismatch {
                target_node: self.target_node,
                times: self.times.len(),
                values: self.values.len(),
                expected_stride: stride,
            });
        }
        let mut channel =
            Channel::new(target_joint, self.kind).with_interpolation(self.interpolation);
        for (key_index, time) in self.times.iter().enumerate() {
            let base = key_index * stride;
            let value = match self.kind {
                ChannelKind::Translation => KeyValue::Translation(Vec3::new(
                    self.values[base],
                    self.values[base + 1],
                    self.values[base + 2],
                )),
                ChannelKind::Rotation => KeyValue::Rotation(Quat::from_xyzw_normalized(
                    self.values[base],
                    self.values[base + 1],
                    self.values[base + 2],
                    self.values[base + 3],
                )),
                ChannelKind::Scale => KeyValue::Scale(Vec3::new(
                    self.values[base],
                    self.values[base + 1],
                    self.values[base + 2],
                )),
            };
            channel
                .add_key(Keyframe::new(*time, value))
                .map_err(AnimationImportError::Clip)?;
        }
        Ok(channel)
    }
}

/// glTF 动画片段。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GltfAnimation {
    pub name: String,
    pub channels: Vec<GltfChannel>,
    /// 可选事件（glTF 本身没有事件；asset 层可从自定义扩展或命名约定生成）。
    pub events: Vec<AnimationEvent>,
}

impl GltfAnimation {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            channels: Vec::new(),
            events: Vec::new(),
        }
    }

    /// 转换为运行时 clip。
    pub fn to_clip(
        &self,
        node_to_joint: &HashMap<usize, JointIndex>,
        skeleton: &Skeleton,
    ) -> Result<AnimationClip, AnimationImportError> {
        let mut clip = AnimationClip::new(self.name.clone(), 0.0);
        for channel in &self.channels {
            clip.add_channel(channel.to_channel(node_to_joint)?);
        }
        for event in &self.events {
            clip.add_event(event.clone());
        }
        clip.recompute_duration();
        clip.validate(skeleton)
            .map_err(AnimationImportError::Clip)?;
        Ok(clip)
    }
}

/// 一个 glTF 角色/资产的动画子集（asset 模块的交付物）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GltfAsset {
    /// 骨架（必需）。
    pub skeleton: Option<GltfSkeleton>,
    /// 蒙皮列表（可为空：只有动画没有 mesh）。
    pub skins: Vec<GltfSkin>,
    /// 动画列表。
    pub animations: Vec<GltfAnimation>,
}

impl GltfAsset {
    pub fn new(skeleton: GltfSkeleton) -> Self {
        Self {
            skeleton: Some(skeleton),
            skins: Vec::new(),
            animations: Vec::new(),
        }
    }

    /// 转换为运行时数据（骨架 + 蒙皮 + 片段）。
    ///
    /// 这是 asset 模块与 animation 模块之间的**唯一转换入口**：
    /// asset 侧只负责把 glTF 解析成 [`GltfAsset`]，不接触运行时结构。
    pub fn into_runtime(self) -> Result<AnimationAssetData, AnimationImportError> {
        let gltf_skeleton = self.skeleton.ok_or(AnimationImportError::MissingSkeleton)?;
        let mut skeleton = gltf_skeleton.to_skeleton()?;
        let node_to_joint = gltf_skeleton.node_to_joint_map();
        // glTF 的 inverseBindMatrices 是可选属性；缺失时用绑定姿态反算补全。
        if gltf_skeleton.inverse_bind_matrices.is_empty() {
            skeleton
                .rebuild_inverse_bind_matrices()
                .map_err(AnimationImportError::Skeleton)?;
        }
        let mut skins = Vec::with_capacity(self.skins.len());
        for gltf_skin in &self.skins {
            let skin = if gltf_skin.inverse_bind_matrices.is_empty() {
                // skin 未提供 IBM：复用骨架已补全的逆绑定矩阵。
                let mut joints = Vec::with_capacity(gltf_skin.joint_nodes.len());
                let mut inverse_bind_matrices = Vec::with_capacity(gltf_skin.joint_nodes.len());
                for node_index in &gltf_skin.joint_nodes {
                    let joint = node_to_joint
                        .get(node_index)
                        .copied()
                        .ok_or(AnimationImportError::UnknownJointNode { node: *node_index })?;
                    joints.push(joint);
                    inverse_bind_matrices.push(
                        skeleton
                            .joints
                            .get(joint)
                            .map(|j| j.inverse_bind)
                            .unwrap_or(Mat4::IDENTITY),
                    );
                }
                Skin::with_joint_indices(gltf_skin.name.clone(), joints, inverse_bind_matrices)
            } else {
                gltf_skin.to_skin(&node_to_joint)?
            };
            skin.validate(&skeleton)
                .map_err(AnimationImportError::Skin)?;
            skins.push(skin);
        }
        let mut clips = Vec::with_capacity(self.animations.len());
        for animation in &self.animations {
            clips.push(animation.to_clip(&node_to_joint, &skeleton)?);
        }
        Ok(AnimationAssetData {
            skeleton,
            skins,
            clips,
        })
    }
}

/// 运行时动画资产：asset 导入的最终产出，可直接交给 gameplay/render。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationAssetData {
    pub skeleton: Skeleton,
    pub skins: Vec<Skin>,
    pub clips: Vec<AnimationClip>,
}

impl AnimationAssetData {
    /// 按名称查找片段（asset 层通常用动画名映射状态）。
    pub fn clip(&self, name: &str) -> Option<&AnimationClip> {
        self.clips.iter().find(|clip| clip.name == name)
    }

    /// 第一个 skin（0.2 单 skin 角色足够）。
    pub fn primary_skin(&self) -> Option<&Skin> {
        self.skins.first()
    }

    /// 把片段按名称绑定到状态机（找不到的片段跳过）。
    pub fn bind_to_state_machine(
        &self,
        machine: &mut crate::state_machine::AnimationStateMachine,
        mapping: &[(crate::state_machine::AnimationState, &str)],
    ) {
        for (state, clip_name) in mapping {
            if let Some(clip) = self.clip(clip_name) {
                machine.set_clip(*state, clip.clone());
            }
        }
    }
}

/// 导入/转换错误。
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationImportError {
    /// 没有提供骨架。
    MissingSkeleton,
    /// 骨架为空（joint_nodes 为空）。
    EmptySkeleton,
    /// 骨架层级错误。
    Skeleton(SkeletonError),
    /// 蒙皮数据错误。
    Skin(SkinError),
    /// 节点索引在节点数组中不存在。
    UnknownNode { node: usize },
    /// 蒙皮/动画引用了非骨骼节点。
    UnknownJointNode { node: usize },
    /// `joint_nodes` 与逆绑定矩阵数量不一致。
    InverseBindCountMismatch { joints: usize, inverse_bind: usize },
    /// 通道值数量与关键帧时间数量不匹配。
    ChannelValueCountMismatch {
        target_node: usize,
        times: usize,
        values: usize,
        expected_stride: usize,
    },
    /// 0.2 不支持的插值方式（CUBICSPLINE）。
    UnsupportedInterpolation { interpolation: Interpolation },
    /// 片段数据错误。
    Clip(crate::clip::ClipError),
}

impl std::fmt::Display for AnimationImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSkeleton => write!(f, "glTF 资产缺少骨架"),
            Self::EmptySkeleton => write!(f, "glTF 骨架没有任何关节"),
            Self::Skeleton(err) => write!(f, "骨架错误: {err}"),
            Self::Skin(err) => write!(f, "蒙皮错误: {err}"),
            Self::UnknownNode { node } => write!(f, "节点索引 {node} 不存在"),
            Self::UnknownJointNode { node } => {
                write!(f, "节点 {node} 不是该 skin 的关节")
            }
            Self::InverseBindCountMismatch { joints, inverse_bind } => write!(
                f,
                "关节数({joints})与逆绑定矩阵数({inverse_bind})不一致"
            ),
            Self::ChannelValueCountMismatch {
                target_node,
                times,
                values,
                expected_stride,
            } => write!(
                f,
                "节点 {target_node} 的动画通道: {times} 个关键帧 × {expected_stride} 分量 ≠ {values} 个值"
            ),
            Self::UnsupportedInterpolation { interpolation } => {
                write!(f, "0.2 不支持插值方式 {interpolation:?}（CUBICSPLINE 计划 0.3）")
            }
            Self::Clip(err) => write!(f, "动画片段错误: {err}"),
        }
    }
}

impl std::error::Error for AnimationImportError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个两关节 glTF 资产：root(节点0) → child(节点1)。
    fn two_joint_asset() -> GltfAsset {
        let mut skeleton = GltfSkeleton::new("hero");
        skeleton.nodes = vec![GltfNode::new("root", None), {
            let mut node = GltfNode::new("child", Some(0));
            node.translation = Vec3::new(0.0, 1.0, 0.0);
            node
        }];
        skeleton.joint_nodes = vec![0, 1];
        skeleton.inverse_bind_matrices = vec![
            Mat4::IDENTITY,
            Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        ];
        let mut asset = GltfAsset::new(skeleton);
        let mut skin = GltfSkin::new("hero_skin");
        skin.joint_nodes = vec![0, 1];
        skin.inverse_bind_matrices = vec![
            Mat4::IDENTITY,
            Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        ];
        asset.skins.push(skin);
        let mut animation = GltfAnimation::new("Run");
        let mut channel = GltfChannel::translation(1);
        channel.times = vec![0.0, 1.0];
        channel.values = vec![0.0, 0.0, 0.0, 0.0, 2.0, 0.0];
        animation.channels.push(channel);
        animation.events.push(AnimationEvent::new(0.5, "footstep"));
        asset.animations.push(animation);
        asset
    }

    #[test]
    fn gltf_asset_converts_to_runtime_structures() {
        let runtime = two_joint_asset().into_runtime().unwrap();
        assert_eq!(runtime.skeleton.joint_count(), 2);
        assert_eq!(runtime.skins.len(), 1);
        assert_eq!(runtime.clips.len(), 1);
        let clip = runtime.clip("Run").unwrap();
        assert_eq!(clip.channel_count(), 1);
        assert!((clip.duration - 1.0).abs() < 1e-5);
        // 引用完整性：skin 的关节都在骨架范围内、clip 通道关节合法。
        runtime.skins[0].validate(&runtime.skeleton).unwrap();
        clip.validate(&runtime.skeleton).unwrap();
    }

    #[test]
    fn gltf_unknown_joint_node_is_rejected() {
        let mut asset = two_joint_asset();
        asset.skins[0].joint_nodes[1] = 99;
        assert!(matches!(
            asset.into_runtime(),
            Err(AnimationImportError::UnknownJointNode { node: 99 })
        ));
    }

    #[test]
    fn gltf_channel_value_count_is_validated() {
        let mut asset = two_joint_asset();
        asset.animations[0].channels[0].values.pop();
        assert!(matches!(
            asset.into_runtime(),
            Err(AnimationImportError::ChannelValueCountMismatch { .. })
        ));
    }

    #[test]
    fn gltf_cubic_spline_is_explicitly_unsupported() {
        let mut asset = two_joint_asset();
        asset.animations[0].channels[0].interpolation = Interpolation::CubicSpline;
        assert!(matches!(
            asset.into_runtime(),
            Err(AnimationImportError::UnsupportedInterpolation { .. })
        ));
    }
}
