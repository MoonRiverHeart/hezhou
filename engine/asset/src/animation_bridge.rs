//! 与 `hezhou-animation` 的 glTF 契约桥接（feature = `animation-bridge`）。
//!
//! 职责边界：
//!
//! - `hezhou-asset` 负责用 `gltf` 库解析文件，得到与第三方库解耦的 [`ImportedScene`]；
//! - `hezhou-animation` 负责把内存数据转成运行时 `Skeleton` / `Skin` / `AnimationClip`；
//! - 本模块是两者之间**唯一的适配层**：把 [`ImportedScene`] 填充成
//!   `hezhou_animation::gltf::GltfAsset`，再交给 `GltfAsset::into_runtime()`。
//!
//! 索引约定与 animation crate 保持一致：
//!
//! - `GltfNode.parent` / `GltfChannel.target_node` 使用 **glTF 节点索引**（整个节点数组下标）；
//! - `joint_nodes` 顺序 = `skin.joints` 顺序 = 运行时 `JointIndex`；
//! - `values` 扁平存储：Translation/Scale 每帧 3 个 f32，Rotation 每帧 4 个 f32（xyzw）。
//!
//! 已知限制（0.2）：
//!
//! - morph target 权重通道没有对应运行时通道，桥接时跳过并记录 warning；
//! - `CUBICSPLINE` 通道会原样保留，但 animation 0.2 运行时转换会报不支持；
//! - 动画事件暂未解析（glTF 标准没有事件；后续从自定义扩展/命名约定生成），`events` 为空。

use hezhou_animation::clip::{ChannelKind, Interpolation};
use hezhou_animation::gltf::{
    AnimationAssetData, AnimationImportError, GltfAnimation, GltfAsset, GltfChannel, GltfNode,
    GltfSkeleton, GltfSkin,
};
use hezhou_animation::math::{Mat4 as AnimMat4, Quat as AnimQuat, Vec3 as AnimVec3};

use crate::import::{
    AnimationInterpolation, AnimationProperty, ImportedAnimationChannel, ImportedScene,
};

/// 桥接结果：animation crate 的 glTF 契约 + 适配过程中的告警。
#[derive(Clone, Debug)]
pub struct GltfAssetBridge {
    /// 可直接交给 `GltfAsset::into_runtime()` 的数据。
    pub asset: GltfAsset,
    /// 适配过程中的非致命告警（跳过的通道等）。
    pub warnings: Vec<String>,
}

impl GltfAssetBridge {
    /// 转为运行时动画数据（`Skeleton` / `Skin` / `AnimationClip`）。
    pub fn into_runtime(self) -> Result<AnimationAssetData, AnimationImportError> {
        self.asset.into_runtime()
    }

    /// 是否有骨架（`GltfAsset::into_runtime` 要求骨架存在）。
    pub fn has_skeleton(&self) -> bool {
        self.asset.skeleton.is_some()
    }
}

/// 把 asset 的导入数据适配成 animation crate 的 glTF 契约。
pub fn imported_scene_to_gltf_asset(scene: &ImportedScene) -> GltfAssetBridge {
    let mut warnings = Vec::new();

    // 节点表按 glTF 节点索引填充（GltfSkeleton.nodes 也按节点索引访问）。
    let nodes: Vec<GltfNode> = scene
        .nodes
        .iter()
        .map(|node| GltfNode {
            name: node.name.clone().unwrap_or_default(),
            parent: node.parent,
            translation: AnimVec3::new(
                node.local_transform.translation.x,
                node.local_transform.translation.y,
                node.local_transform.translation.z,
            ),
            rotation: AnimQuat::new(
                node.local_transform.rotation.x,
                node.local_transform.rotation.y,
                node.local_transform.rotation.z,
                node.local_transform.rotation.w,
            ),
            scale: AnimVec3::new(
                node.local_transform.scale.x,
                node.local_transform.scale.y,
                node.local_transform.scale.z,
            ),
        })
        .collect();

    let skeleton = scene.skeletons.first().map(|skeleton| GltfSkeleton {
        name: skeleton.name.clone().unwrap_or_default(),
        joint_nodes: skeleton.joints.iter().map(|joint| joint.node).collect(),
        inverse_bind_matrices: skeleton
            .joints
            .iter()
            .map(|joint| AnimMat4 {
                cols: joint.inverse_bind_matrix.cols,
            })
            .collect(),
        nodes: nodes.clone(),
    });
    if skeleton.is_none() {
        warnings.push("场景没有 skin/skeleton，animation 无法生成运行时骨架".to_string());
    }

    let skins: Vec<GltfSkin> = scene
        .skeletons
        .iter()
        .map(|skeleton| GltfSkin {
            name: skeleton.name.clone().unwrap_or_default(),
            joint_nodes: skeleton.joints.iter().map(|joint| joint.node).collect(),
            inverse_bind_matrices: skeleton
                .joints
                .iter()
                .map(|joint| AnimMat4 {
                    cols: joint.inverse_bind_matrix.cols,
                })
                .collect(),
        })
        .collect();

    let animations: Vec<GltfAnimation> = scene
        .animations
        .iter()
        .map(|animation| {
            let mut channels = Vec::new();
            for channel in &animation.channels {
                match convert_channel(channel, &mut warnings) {
                    Some(converted) => channels.push(converted),
                    None => continue,
                }
            }
            GltfAnimation {
                name: animation.name.clone().unwrap_or_default(),
                channels,
                // 0.2：glTF 无标准事件，TODO 从扩展/命名约定解析。
                events: Vec::new(),
            }
        })
        .collect();

    GltfAssetBridge {
        asset: GltfAsset {
            skeleton,
            skins,
            animations,
        },
        warnings,
    }
}

fn convert_channel(
    channel: &ImportedAnimationChannel,
    warnings: &mut Vec<String>,
) -> Option<GltfChannel> {
    let kind = match channel.property {
        AnimationProperty::Translation => ChannelKind::Translation,
        AnimationProperty::Rotation => ChannelKind::Rotation,
        AnimationProperty::Scale => ChannelKind::Scale,
        AnimationProperty::MorphTargetWeights => {
            warnings.push(format!(
                "节点 {} 的 morph target 权重动画 0.2 不支持，已跳过",
                channel.target_node
            ));
            return None;
        }
    };
    let interpolation = match channel.interpolation {
        AnimationInterpolation::Linear => Interpolation::Linear,
        AnimationInterpolation::Step => Interpolation::Step,
        AnimationInterpolation::CubicSpline => {
            warnings.push(format!(
                "节点 {} 的 CUBICSPLINE 通道 0.2 运行时暂不支持（数据已保留）",
                channel.target_node
            ));
            Interpolation::CubicSpline
        }
    };
    Some(GltfChannel {
        target_node: channel.target_node,
        kind,
        interpolation,
        times: channel.times.clone(),
        values: channel.values.clone(),
    })
}
