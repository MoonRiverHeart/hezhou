//! hezhou-animation — Hezhou 引擎 0.2 骨骼/蒙皮动画能力。
//!
//! 本 crate 只做**引擎能力**，不绑定任何具体角色/游戏规则：
//!
//! | 能力 | 入口 |
//! |---|---|
//! | 骨骼层级 / 绑定姿态 / 逆绑定矩阵 | [`Skeleton`]、[`Joint`]、[`Bone`] |
//! | 蒙皮 / 权重 / 蒙皮矩阵 | [`Skin`]、[`VertexWeights`] |
//! | 关键帧 / 通道 / 采样（T/R/S） | [`AnimationClip`]、[`Channel`]、[`Keyframe`] |
//! | 单片段播放（循环/速度/事件） | [`AnimationPlayer`] |
//! | 9 动作状态机（基础 6 + Land/Dodge/Block）+ 过渡 + 事件 | [`AnimationStateMachine`]、[`AnimationState`] |
//! | CPU 蒙皮 / 姿态混合 | [`Pose`]、[`skin_position`] |
//! | GPU joint palette / 顶点布局 / shader 契约 | [`JointPalette`]、[`SkinnedVertex`] |
//! | glTF 导入数据契约（由 asset 模块填充） | [`GltfAsset`]、[`AnimationAssetData`] |
//!
//! ## 快速上手
//!
//! ```
//! use hezhou_animation::{
//!     AnimationClip, AnimationState, AnimationStateMachine, Channel, Joint, Keyframe,
//!     Mat4, Quat, Skeleton, Skin, Transform, Vec3,
//! };
//!
//! // 1) 搭一个 2 关节骨架（父先子后构建）
//! let mut skeleton = Skeleton::new("hero");
//! let root = skeleton.add_root_joint("root", Transform::IDENTITY);
//! skeleton
//!     .add_joint(
//!         "hand",
//!         root,
//!         Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
//!     )
//!     .unwrap();
//! // 逆绑定矩阵由绑定姿态自动补全，并生成 1:1 蒙皮
//! skeleton.rebuild_inverse_bind_matrices().unwrap();
//! let skin = Skin::from_skeleton(&skeleton);
//!
//! // 2) 做一个抬手片段
//! let mut clip = AnimationClip::new("Attack", 1.0);
//! let mut channel = Channel::translation(1);
//! channel.add_key(Keyframe::translation(0.0, Vec3::new(0.0, 1.0, 0.0))).unwrap();
//! channel.add_key(Keyframe::translation(1.0, Vec3::new(0.0, 1.0, 2.0))).unwrap();
//! clip.add_channel(channel);
//!
//! // 3) 状态机绑定片段，切换到 Attack 并采样
//! let mut machine = AnimationStateMachine::new();
//! machine.set_clip(AnimationState::Attack, clip);
//! machine.transition_to(AnimationState::Attack, 0.0);
//! machine.update(0.5);
//! let pose = machine.sample(&skeleton);
//!
//! // 4) 计算 GPU palette（CPU 端数据，渲染侧负责上传）
//! let palette = hezhou_animation::JointPalette::from_pose(&skeleton, &skin, &pose);
//! assert_eq!(palette.len(), skeleton.joint_count());
//! # let _ = (Mat4::IDENTITY, Joint::simple("unused", None));
//! ```
//!
//! ## 功能边界（0.2）
//! - **包含**：骨骼/蒙皮、关键帧采样（LINEAR/STEP）、循环/速度、9 状态机（基础 6 状态 +
//!   Land/Dodge/Block 动作扩展）与交叉淡入淡出、动画事件、蒙皮矩阵/palette、CPU 蒙皮、glTF 数据转换。
//! - **不包含**（0.3+，接口已预留）：Blend Tree / 混合空间、IK、根运动提取、动画层/遮罩、
//!   CUBICSPLINE 切线采样、GPU 资源创建（Vulkan buffer/descriptor 归 render/rhi）。
//!
//! ## 集成约定
//! - **资源**：asset 模块把 glTF 解析为 [`GltfAsset`]，调用 [`GltfAsset::into_runtime`]
//!   得到 [`AnimationAssetData`]（骨架/蒙皮/片段）。
//! - **渲染**：每帧用 `AnimationStateMachine::sample` 得到 [`Pose`]，用
//!   [`JointPalette::from_pose`] 生成 palette；render 侧按 [`gpu`] 模块的布局上传 buffer，
//!   并用 `engine/shaders/skinning.vert` 的接口契约做 GPU 蒙皮。
//! - **玩法**：通过 [`AnimationStateDriver`] trait 驱动状态切换，事件用
//!   [`AnimationStateMachine::update`] 返回值消费（脚步/命中帧等）。
//!
//! 除 shader/GPU 上传外，本 crate 全部为纯 Rust 逻辑，可离线单测。

#![forbid(unsafe_code)]

pub mod clip;
pub mod gltf;
pub mod gpu;
pub mod math;
pub mod player;
pub mod pose;
pub mod skeleton;
pub mod skin;
pub mod state_machine;

pub use clip::{
    AnimationClip, AnimationEvent, Channel, ChannelKind, ClipError, Interpolation, KeyValue,
    Keyframe,
};
pub use gltf::{
    AnimationAssetData, AnimationImportError, GltfAnimation, GltfAsset, GltfChannel, GltfNode,
    GltfSkeleton, GltfSkin,
};
pub use gpu::{
    GpuVertexAttribute, JointPalette, SkinnedVertex, JOINT_INDEX_ATTRIBUTE_LOCATION,
    JOINT_MATRIX_BYTES, JOINT_PALETTE_BINDING, JOINT_PALETTE_SET, JOINT_WEIGHT_ATTRIBUTE_LOCATION,
    MAX_JOINTS_PER_PALETTE_UBO, SKINNED_VERTEX_ATTRIBUTES, SKINNED_VERTEX_STRIDE,
    SKINNING_FUNCTION_GLSL, SKINNING_PALETTE_GLSL, SKINNING_VERTEX_INPUT_GLSL,
};
pub use math::{Mat4, Quat, Transform, Vec3, EPSILON};
pub use player::AnimationPlayer;
pub use pose::{
    AnimationContext, IkSolver, IkTarget, Pose, PoseNode, TwoBoneIk, UnsupportedFeature,
};
pub use skeleton::{Bone, Joint, JointIndex, Skeleton, SkeletonError};
pub use skin::{
    skin_normal, skin_position, skin_vertex, Skin, SkinError, VertexWeights, MAX_JOINT_INFLUENCES,
};
pub use state_machine::{
    AnimationState, AnimationStateDriver, AnimationStateMachine, StateEntry, StateEvent,
    Transition, TransitionCurve,
};
