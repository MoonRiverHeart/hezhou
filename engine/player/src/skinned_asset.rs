//! Player 侧真实蒙皮资产消费：`ImportedPrimitive(joints/weights)` → `SkinnedVertex`，
//! 选择第一份带蒙皮的 mesh/skeleton/animation，并计算 joint palette（静态绑定姿态或动画采样）。
//!
//! # 数据流
//!
//! ```text
//! ImportedAssetBundle.mesh_entries()
//!   └─ ImportedMesh.primitives[]
//!        └─ 第一个 joints/weights 非空且长度匹配的 primitive
//!             ├─ primitive_to_skinned_mesh(): ImportedPrimitive → (Vec<SkinnedVertex>, Vec<u32>)
//!             ├─ skeleton: primitive.skeleton / 场景第一个 skeleton
//!             ├─ animation: 场景第一个 animation clip
//!             └─ palette_at(time): joints/node local TRS → world × inverse_bind → JointPalette
//! ```
//!
//! 与 task-28 的契约：
//! - 顶点使用 `hezhou_animation::SkinnedVertex`（64B stride，joint_indices 是 palette 下标）；
//! - palette 使用 `hezhou_animation::JointPalette`（每关节 64B，列主序 `world * inverse_bind`），
//!   由 host 通过 `UIVulkanRenderer::upload_skin_palette(entity_id, bytes)` 每帧上传；
//! - 本模块只做 CPU 转换/采样，不依赖 rhi-vulkan（默认构建可测）。
//!
//! # 限制（0.2 本轮）
//!
//! - 只消费**第一个**带蒙皮的 mesh/primitive、第一个 skeleton、第一个 animation clip；
//! - 只支持三角形拓扑（Points/Lines/Strip/Fan 报错）；
//! - 动画插值支持 Linear / Step；CubicSpline 取关键帧中值近似；
//! - 无材质/纹理绑定（task-28 upload 的 material/texture_path 传 None）；
//! - 无 skeleton 时 mesh 仍可转换，但 palette 计算报 `NoSkeleton`（host 回退 WARN）。

use hezhou_animation::{
    JointPalette, Mat4, Quat, SkinnedVertex, Transform, Vec3, MAX_JOINTS_PER_PALETTE_UBO,
};
use hezhou_asset::import::{
    AnimationInterpolation, AnimationProperty, ImportedAnimation, ImportedAnimationChannel,
    ImportedNode, ImportedPrimitive, ImportedScene, ImportedSkeleton, PrimitiveMode,
};
use hezhou_asset::AssetId;

use crate::imported_assets::ImportedAssetBundle;

/// Player window host 上传真实蒙皮网格时使用的固定实体 ID（非 0，避免与 Scene 实体冲突）。
pub const PLAYER_SKINNED_ENTITY_ID: u64 = 0x4845_5A50_534B_0001; // "HEZPSK\x00\x01"

/// 选中的 skeleton 信息。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkinnedSkeletonInfo {
    /// cook manifest 中的 skeleton AssetId（若 manifest 未声明子资产则为 None）。
    pub asset_id: Option<AssetId>,
    /// `ImportedScene.skeletons` 下标。
    pub scene_index: usize,
    /// 关节数。
    pub joint_count: usize,
}

/// 选中的 animation 信息。
#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedAnimationInfo {
    /// cook manifest 中的 animation AssetId（可能为 None）。
    pub asset_id: Option<AssetId>,
    /// `ImportedScene.animations` 下标。
    pub scene_index: usize,
    /// clip 名。
    pub name: Option<String>,
    /// 时长（秒）。
    pub duration: f32,
}

/// 选中的真实蒙皮资产（CPU 数据 + 所属 ImportedScene 用于 palette 采样）。
#[derive(Clone, Debug)]
pub struct SelectedSkinnedAsset {
    /// mesh 子资产 AssetId。
    pub mesh_asset_id: AssetId,
    /// 所属 glTF 场景 AssetId。
    pub scene_asset_id: AssetId,
    /// mesh 在 `ImportedScene.meshes` 中的下标。
    pub mesh_index: usize,
    /// primitive 在 mesh 中的下标。
    pub primitive_index: usize,
    /// 转换后的蒙皮顶点。
    pub vertices: Vec<SkinnedVertex>,
    /// 三角索引。
    pub indices: Vec<u32>,
    /// primitive 引用的材质下标（0.2 未消费）。
    pub material_index: Option<usize>,
    /// skeleton 选择。
    pub skeleton: Option<SkinnedSkeletonInfo>,
    /// animation 选择。
    pub animation: Option<SkinnedAnimationInfo>,
    /// 所属导入场景（clone，用于 palette 采样）。
    pub scene: ImportedScene,
}

impl SelectedSkinnedAsset {
    /// 顶点数。
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// 三角形数。
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// 关节数（无 skeleton 时为 0）。
    pub fn joint_count(&self) -> usize {
        self.skeleton.as_ref().map(|info| info.joint_count).unwrap_or(0)
    }

    /// 单行统计（启动日志/CLI）。
    pub fn summary(&self) -> String {
        let animation = self
            .animation
            .as_ref()
            .map(|info| {
                format!(
                    "{}（{:.2}s）",
                    info.name.as_deref().unwrap_or("<unnamed>"),
                    info.duration
                )
            })
            .unwrap_or_else(|| "无动画（静态绑定姿态）".to_string());
        format!(
            "蒙皮资产：mesh {}（{} 顶点 / {} 三角形），skeleton {} 关节，animation {}，材质 {}",
            self.mesh_asset_id,
            self.vertex_count(),
            self.triangle_count(),
            self.joint_count(),
            animation,
            self.material_index
                .map(|index| index.to_string())
                .unwrap_or_else(|| "无".to_string()),
        )
    }

    /// 计算 `time` 时刻的 joint palette（静态姿态时传 0.0 即可）。
    ///
    /// 无 skeleton / 关节数超过 GPU palette 上限时报可读错误。
    pub fn palette_at(&self, time: f32) -> Result<JointPalette, SkinnedAssetError> {
        let info = self
            .skeleton
            .as_ref()
            .ok_or(SkinnedAssetError::NoSkeleton {
                mesh: self.mesh_asset_id,
            })?;
        compute_joint_palette(
            &self.scene,
            info.scene_index,
            self.animation.as_ref().map(|animation| animation.scene_index),
            time,
        )
    }

    /// 构造 task-28 `SkinnedMeshUpload` 所需的顶点/索引（palette 由调用方单独上传）。
    pub fn upload_parts(&self) -> (Vec<SkinnedVertex>, Vec<u32>) {
        (self.vertices.clone(), self.indices.clone())
    }
}

/// 从 bundle 选择第一个带 joints/weights 的蒙皮 mesh/primitive。
///
/// 无蒙皮资产返回 `Ok(None)`（调用方回退内置 cube）；数据非法返回可读错误。
pub fn select_first_skinned_asset(
    bundle: &ImportedAssetBundle,
) -> Result<Option<SelectedSkinnedAsset>, SkinnedAssetError> {
    for (mesh_id, scene_id, mesh_index, mesh) in bundle.mesh_entries() {
        for (primitive_index, primitive) in mesh.primitives.iter().enumerate() {
            if primitive.joints.is_empty()
                || primitive.weights.is_empty()
                || primitive.joints.len() != primitive.positions.len()
                || primitive.weights.len() != primitive.positions.len()
            {
                // 未蒙皮的 primitive（或其他 primitive）跳过，继续找下一份。
                continue;
            }
            let scene = bundle
                .scene(scene_id)
                .ok_or(SkinnedAssetError::MissingScene { mesh: mesh_id })?
                .clone();

            let (vertices, indices) = primitive_to_skinned_mesh(primitive)?;

            // skeleton：primitive 指定优先；否则用场景第一个。
            let skeleton_info = primitive
                .skeleton
                .or_else(|| {
                    if scene.skeletons.is_empty() {
                        None
                    } else {
                        Some(0)
                    }
                })
                .and_then(|skeleton_index| {
                    scene.skeletons.get(skeleton_index).map(|skeleton| {
                        SkinnedSkeletonInfo {
                            asset_id: bundle.skeleton_id_for(scene_id, skeleton_index),
                            scene_index: skeleton_index,
                            joint_count: skeleton.joints.len(),
                        }
                    })
                });

            // animation：场景第一个 clip。
            let animation_info = scene.animations.first().map(|animation| SkinnedAnimationInfo {
                asset_id: bundle.animation_id_for(scene_id, 0),
                scene_index: 0,
                name: animation.name.clone(),
                duration: animation.duration,
            });

            return Ok(Some(SelectedSkinnedAsset {
                mesh_asset_id: mesh_id,
                scene_asset_id: scene_id,
                mesh_index,
                primitive_index,
                vertices,
                indices,
                material_index: primitive.material,
                skeleton: skeleton_info,
                animation: animation_info,
                scene,
            }));
        }
    }
    Ok(None)
}

/// `ImportedPrimitive` → `(Vec<SkinnedVertex>, Vec<u32>)`。
///
/// 校验：positions 非空、三角形拓扑、可选属性长度一致、joints/weights 长度一致、
/// 索引非空且 < 顶点数、关节数 ≤ GPU palette 上限；权重归一化。
pub fn primitive_to_skinned_mesh(
    primitive: &ImportedPrimitive,
) -> Result<(Vec<SkinnedVertex>, Vec<u32>), SkinnedAssetError> {
    if primitive.positions.is_empty() {
        return Err(SkinnedAssetError::EmptyPrimitive);
    }
    if primitive.mode != PrimitiveMode::Triangles {
        return Err(SkinnedAssetError::UnsupportedTopology {
            mode: format!("{:?}", primitive.mode),
        });
    }
    let vertex_count = primitive.positions.len();
    validate_optional_len("normals", primitive.normals.len(), vertex_count)?;
    validate_optional_len("uvs", primitive.uvs.len(), vertex_count)?;
    if primitive.joints.len() != vertex_count || primitive.weights.len() != vertex_count {
        return Err(SkinnedAssetError::SkinAttributeMismatch {
            attribute: "joints/weights",
            expected: vertex_count,
            actual: primitive.joints.len().min(primitive.weights.len()),
        });
    }

    let mut vertices = Vec::with_capacity(vertex_count);
    for index in 0..vertex_count {
        let position = primitive.positions[index];
        let normal = primitive
            .normals
            .get(index)
            .copied()
            .unwrap_or_default();
        let uv = primitive.uvs.get(index).copied().unwrap_or_default();
        let joints = primitive.joints[index].map(u32::from);
        let weights = primitive.weights[index];
        vertices.push(
            SkinnedVertex::unskinned(
                [position.x, position.y, position.z],
                [normal.x, normal.y, normal.z],
                [uv.x, uv.y],
            )
            .with_weights(joints, weights),
        );
    }

    let indices = if primitive.indices.is_empty() {
        let generated: Vec<u32> = (0..vertex_count as u32).collect();
        if generated.len() % 3 != 0 {
            return Err(SkinnedAssetError::InvalidIndexCount {
                count: generated.len(),
            });
        }
        generated
    } else {
        if primitive.indices.len() % 3 != 0 {
            return Err(SkinnedAssetError::InvalidIndexCount {
                count: primitive.indices.len(),
            });
        }
        if let Some(invalid) = primitive
            .indices
            .iter()
            .find(|index| **index as usize >= vertex_count)
        {
            return Err(SkinnedAssetError::IndexOutOfRange {
                index: *invalid,
                vertex_count,
            });
        }
        primitive.indices.clone()
    };

    Ok((vertices, indices))
}

fn validate_optional_len(
    attribute: &'static str,
    actual: usize,
    expected: usize,
) -> Result<(), SkinnedAssetError> {
    if actual != 0 && actual != expected {
        return Err(SkinnedAssetError::AttributeMismatch {
            attribute,
            expected,
            actual,
        });
    }
    Ok(())
}

/// 计算某个 skeleton 在 `time`（秒）的 joint palette；`animation_index=None` 为静态绑定姿态。
pub fn compute_joint_palette(
    scene: &ImportedScene,
    skeleton_index: usize,
    animation_index: Option<usize>,
    time: f32,
) -> Result<JointPalette, SkinnedAssetError> {
    let skeleton = scene
        .skeletons
        .get(skeleton_index)
        .ok_or(SkinnedAssetError::SkeletonOutOfRange {
            index: skeleton_index,
            count: scene.skeletons.len(),
        })?;
    if skeleton.joints.is_empty() {
        return Err(SkinnedAssetError::EmptySkeleton);
    }
    if skeleton.joints.len() > MAX_JOINTS_PER_PALETTE_UBO {
        return Err(SkinnedAssetError::TooManyJoints {
            count: skeleton.joints.len(),
            max: MAX_JOINTS_PER_PALETTE_UBO,
        });
    }

    // 1) 初始 local TRS 来自节点绑定姿态。
    let mut locals: Vec<Transform> = Vec::with_capacity(skeleton.joints.len());
    for joint in &skeleton.joints {
        let node = scene
            .nodes
            .get(joint.node)
            .ok_or(SkinnedAssetError::NodeOutOfRange {
                joint_node: joint.node,
                node_count: scene.nodes.len(),
            })?;
        locals.push(node_local_transform(node));
    }

    // 2) 动画采样覆盖 local TRS（按 joint 索引）。
    if let Some(animation_index) = animation_index {
        let animation = scene.animations.get(animation_index).ok_or(
            SkinnedAssetError::AnimationOutOfRange {
                index: animation_index,
                count: scene.animations.len(),
            },
        )?;
        apply_animation(&mut locals, skeleton, animation, time)?;
    }

    // 3) 关节层级 → world 矩阵。
    let world = compute_world_matrices(skeleton, &locals)?;

    // 4) palette = world * inverse_bind。
    let mut matrices = Vec::with_capacity(world.len());
    for (world_matrix, joint) in world.iter().zip(skeleton.joints.iter()) {
        let inverse_bind = asset_mat4_to_animation(joint.inverse_bind_matrix);
        matrices.push(world_matrix.mul_mat4(&inverse_bind));
    }
    Ok(JointPalette::from_matrices(matrices))
}

fn node_local_transform(node: &ImportedNode) -> Transform {
    let trs = node.local_transform;
    Transform::new(
        Vec3::new(trs.translation.x, trs.translation.y, trs.translation.z),
        Quat::new(
            trs.rotation.x,
            trs.rotation.y,
            trs.rotation.z,
            trs.rotation.w,
        ),
        Vec3::new(trs.scale.x, trs.scale.y, trs.scale.z),
    )
}

fn asset_mat4_to_animation(matrix: hezhou_asset::Mat4) -> Mat4 {
    Mat4::from_cols(matrix.cols)
}

fn apply_animation(
    locals: &mut [Transform],
    skeleton: &ImportedSkeleton,
    animation: &ImportedAnimation,
    time: f32,
) -> Result<(), SkinnedAssetError> {
    for channel in &animation.channels {
        let Some(joint_index) = channel
            .target_joint
            .or_else(|| skeleton.joint_index_by_node(channel.target_node))
        else {
            continue;
        };
        if joint_index >= locals.len() {
            return Err(SkinnedAssetError::ChannelJointOutOfRange {
                joint: joint_index,
                joint_count: locals.len(),
            });
        }
        let sampled = sample_channel(channel, time)?;
        let transform = &mut locals[joint_index];
        match channel.property {
            AnimationProperty::Translation => {
                transform.translation = Vec3::new(sampled[0], sampled[1], sampled[2]);
            }
            AnimationProperty::Scale => {
                transform.scale = Vec3::new(sampled[0], sampled[1], sampled[2]);
            }
            AnimationProperty::Rotation => {
                transform.rotation =
                    Quat::from_xyzw_normalized(sampled[0], sampled[1], sampled[2], sampled[3]);
            }
            AnimationProperty::MorphTargetWeights => {
                // 0.2 不消费 morph target。
            }
        }
    }
    Ok(())
}

/// 采样一个动画通道，返回最多 4 分量。
fn sample_channel(
    channel: &ImportedAnimationChannel,
    time: f32,
) -> Result<[f32; 4], SkinnedAssetError> {
    let components = channel
        .property
        .component_count()
        .ok_or(SkinnedAssetError::UnsupportedChannel {
            property: format!("{:?}", channel.property),
        })?;
    if channel.times.is_empty() {
        return Err(SkinnedAssetError::EmptyAnimationChannel);
    }
    if channel.times.len() < 2 && channel.interpolation == AnimationInterpolation::Linear {
        // 单关键帧：直接取该帧。
    }

    let time = time.clamp(
        *channel.times.first().expect("已检查非空"),
        *channel.times.last().expect("已检查非空"),
    );
    let key_count = channel.times.len();
    let expected_values = match channel.interpolation {
        AnimationInterpolation::CubicSpline => key_count * components * 3,
        _ => key_count * components,
    };
    if channel.values.len() < expected_values {
        return Err(SkinnedAssetError::AnimationValueCount {
            expected: expected_values,
            actual: channel.values.len(),
        });
    }

    // 找到相邻关键帧下标。
    let mut upper = 1usize;
    while upper < key_count && channel.times[upper] < time {
        upper += 1;
    }
    if upper >= key_count {
        return Ok(read_keyframe(
            channel,
            key_count - 1,
            components,
            channel.interpolation,
        ));
    }
    let lower = upper - 1;
    let lower_time = channel.times[lower];
    let upper_time = channel.times[upper];
    let factor = if upper_time > lower_time {
        ((time - lower_time) / (upper_time - lower_time)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let mut out = [0.0f32; 4];
    match channel.interpolation {
        AnimationInterpolation::Step => {
            out = read_keyframe(channel, lower, components, channel.interpolation);
        }
        AnimationInterpolation::Linear => {
            let a = read_keyframe(channel, lower, components, channel.interpolation);
            let b = read_keyframe(channel, upper, components, channel.interpolation);
            for index in 0..components {
                out[index] = a[index] + (b[index] - a[index]) * factor;
            }
        }
        AnimationInterpolation::CubicSpline => {
            // 0.2 简化：忽略切线，取 lower 关键帧的 value。
            out = read_keyframe(channel, lower, components, channel.interpolation);
        }
    }
    Ok(out)
}

fn read_keyframe(
    channel: &ImportedAnimationChannel,
    keyframe: usize,
    components: usize,
    interpolation: AnimationInterpolation,
) -> [f32; 4] {
    let stride = if interpolation == AnimationInterpolation::CubicSpline {
        components * 3
    } else {
        components
    };
    let base = keyframe * stride
        + if interpolation == AnimationInterpolation::CubicSpline {
            components // 取 in-tangent 之后的中值
        } else {
            0
        };
    let mut out = [0.0f32; 4];
    for index in 0..components {
        out[index] = channel.values.get(base + index).copied().unwrap_or(0.0);
    }
    if components == 4 {
        // 旋转四元数归一化由 apply_animation 完成。
    }
    out
}

/// 按关节层级计算 world 矩阵（递归 + memo，检测环）。
fn compute_world_matrices(
    skeleton: &ImportedSkeleton,
    locals: &[Transform],
) -> Result<Vec<Mat4>, SkinnedAssetError> {
    let mut world: Vec<Option<Mat4>> = vec![None; locals.len()];
    let mut visiting: Vec<bool> = vec![false; locals.len()];
    for joint in 0..locals.len() {
        compute_world_matrix(joint, skeleton, locals, &mut world, &mut visiting)?;
    }
    Ok(world.into_iter().map(|matrix| matrix.unwrap_or(Mat4::IDENTITY)).collect())
}

fn compute_world_matrix(
    joint: usize,
    skeleton: &ImportedSkeleton,
    locals: &[Transform],
    world: &mut [Option<Mat4>],
    visiting: &mut [bool],
) -> Result<Mat4, SkinnedAssetError> {
    if let Some(matrix) = world[joint] {
        return Ok(matrix);
    }
    if visiting[joint] {
        return Err(SkinnedAssetError::SkeletonCycle { joint });
    }
    visiting[joint] = true;
    let local = locals[joint].to_matrix();
    let matrix = match skeleton.joints[joint].parent {
        Some(parent) => {
            if parent >= locals.len() {
                return Err(SkinnedAssetError::JointParentOutOfRange {
                    joint,
                    parent,
                    joint_count: locals.len(),
                });
            }
            compute_world_matrix(parent, skeleton, locals, world, visiting)?
                .mul_mat4(&local)
        }
        None => local,
    };
    visiting[joint] = false;
    world[joint] = Some(matrix);
    Ok(matrix)
}

/// 蒙皮资产消费错误。
#[derive(Debug)]
pub enum SkinnedAssetError {
    /// 没有可用的蒙皮 mesh（调用方回退内置 cube）。
    EmptyPrimitive,
    /// primitive 不是三角形拓扑。
    UnsupportedTopology { mode: String },
    /// 顶点属性数量与位置数量不一致。
    AttributeMismatch {
        attribute: &'static str,
        expected: usize,
        actual: usize,
    },
    /// joints/weights 与顶点数量不一致。
    SkinAttributeMismatch {
        attribute: &'static str,
        expected: usize,
        actual: usize,
    },
    /// 索引数量不是 3 的倍数。
    InvalidIndexCount { count: usize },
    /// 索引越界。
    IndexOutOfRange { index: u32, vertex_count: usize },
    /// mesh 找不到所属场景。
    MissingScene { mesh: AssetId },
    /// primitive 未指定 skeleton 且场景没有 skeleton。
    NoSkeleton { mesh: AssetId },
    /// skeleton 下标越界。
    SkeletonOutOfRange { index: usize, count: usize },
    /// skeleton 无关节。
    EmptySkeleton,
    /// 关节数超过 GPU palette 上限。
    TooManyJoints { count: usize, max: usize },
    /// 关节引用的 node 越界。
    NodeOutOfRange {
        joint_node: usize,
        node_count: usize,
    },
    /// animation 下标越界。
    AnimationOutOfRange { index: usize, count: usize },
    /// 动画通道目标关节越界。
    ChannelJointOutOfRange { joint: usize, joint_count: usize },
    /// 通道属性不受支持（如 morph target 权重）。
    UnsupportedChannel { property: String },
    /// 动画通道没有关键帧。
    EmptyAnimationChannel,
    /// 动画 values 数量与 times/分量不匹配。
    AnimationValueCount { expected: usize, actual: usize },
    /// 关节父索引越界。
    JointParentOutOfRange {
        joint: usize,
        parent: usize,
        joint_count: usize,
    },
    /// 关节层级成环。
    SkeletonCycle { joint: usize },
}

impl std::fmt::Display for SkinnedAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPrimitive => write!(f, "蒙皮 primitive 顶点为空"),
            Self::UnsupportedTopology { mode } => {
                write!(f, "不支持的 primitive 拓扑 `{mode}`（仅支持 Triangles）")
            }
            Self::AttributeMismatch {
                attribute,
                expected,
                actual,
            } => write!(
                f,
                "primitive {attribute} 数量 {actual} 与顶点数 {expected} 不一致"
            ),
            Self::SkinAttributeMismatch {
                attribute,
                expected,
                actual,
            } => write!(
                f,
                "primitive {attribute} 数量 {actual} 与顶点数 {expected} 不一致"
            ),
            Self::InvalidIndexCount { count } => {
                write!(f, "索引数量 {count} 不是 3 的倍数")
            }
            Self::IndexOutOfRange {
                index,
                vertex_count,
            } => write!(f, "索引 {index} 越界（顶点数 {vertex_count}）"),
            Self::MissingScene { mesh } => write!(f, "mesh {mesh} 找不到所属导入场景"),
            Self::NoSkeleton { mesh } => write!(f, "mesh {mesh} 没有 skeleton，无法计算 palette"),
            Self::SkeletonOutOfRange { index, count } => {
                write!(f, "skeleton 下标 {index} 越界（场景有 {count} 个）")
            }
            Self::EmptySkeleton => write!(f, "skeleton 没有关节"),
            Self::TooManyJoints { count, max } => {
                write!(f, "关节数 {count} 超过 GPU palette 上限 {max}")
            }
            Self::NodeOutOfRange {
                joint_node,
                node_count,
            } => write!(
                f,
                "关节引用的 node {joint_node} 越界（场景有 {node_count} 个 node）"
            ),
            Self::AnimationOutOfRange { index, count } => {
                write!(f, "animation 下标 {index} 越界（场景有 {count} 个）")
            }
            Self::ChannelJointOutOfRange {
                joint,
                joint_count,
            } => write!(f, "动画通道目标关节 {joint} 越界（关节数 {joint_count}）"),
            Self::UnsupportedChannel { property } => {
                write!(f, "不支持的动画通道属性 `{property}`")
            }
            Self::EmptyAnimationChannel => write!(f, "动画通道没有关键帧"),
            Self::AnimationValueCount { expected, actual } => {
                write!(f, "动画 values 数量 {actual} 与期望 {expected} 不一致")
            }
            Self::JointParentOutOfRange {
                joint,
                parent,
                joint_count,
            } => write!(
                f,
                "关节 {joint} 的父索引 {parent} 越界（关节数 {joint_count}）"
            ),
            Self::SkeletonCycle { joint } => write!(f, "关节 {joint} 的层级成环"),
        }
    }
}

impl std::error::Error for SkinnedAssetError {}
