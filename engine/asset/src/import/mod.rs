//! 导入数据契约：与具体 glTF 库解耦的引擎级导入结果。
//!
//! 这是 asset crate 提供给 **animation / render / player / editor** 的公开契约：
//! 导入器（目前是 glTF/GLB）把文件解析成这些类型，其它模块只依赖本模块，
//! 不需要链接 `gltf` crate。
//!
//! 结构对应关系：
//!
//! - 网格：`ImportedScene.meshes[i].primitives[j]`
//! - 材质：`ImportedScene.materials[i]`，primitive 通过 `material: Option<usize>` 引用
//! - 纹理：`ImportedScene.textures[i]`，材质通过 [`ImportedTextureRef`] 引用
//! - 骨骼：`ImportedScene.skeletons[i]`，节点通过 `skin: Option<usize>` 引用
//! - 动画：`ImportedScene.animations[i]`，通道按 glTF node 索引 / skeleton joint 索引定位
//!
//! 数据保证：
//!
//! - 所有 `Vec` 顺序与源文件（glTF 索引）一致，跨平台/跨机器确定性；
//! - 关节索引已转换为“skeleton 内的 joint 索引”（`ImportedAnimationChannel::target_joint`），
//!   GPU 蒙皮可以直接按 joint palette 查找；
//! - 骨骼层级 `parent` 也是 joint 索引（`None` 表示根关节）。
//!
//! 已知限制见 [`crate::import::gltf`] 模块文档（不支持 morph target、多套 UV、稀疏 accessor 等）。

use serde::{Deserialize, Serialize};

use crate::error::{AssetError, AssetResult};
use crate::math::{Aabb, Mat4, Trs, Vec2, Vec3, Vec4};

#[cfg(feature = "gltf")]
pub mod gltf;
#[cfg(not(feature = "gltf"))]
#[path = "gltf_stub.rs"]
pub mod gltf;

pub use gltf::{import_gltf_bytes, import_gltf_file};

/// 源文件格式。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportSourceFormat {
    /// 文本 glTF（`.gltf`）。
    Gltf,
    /// 二进制 glTF（`.glb`）。
    Glb,
}

/// 导入结果：一份 glTF/GLB 文件解析出的全部子资产。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedScene {
    /// 源文件格式。
    pub format: ImportSourceFormat,
    /// 源文件 `asset.generator` 字符串（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    /// 节点层级（索引对应 glTF node 索引）。
    pub nodes: Vec<ImportedNode>,
    /// 网格列表。
    pub meshes: Vec<ImportedMesh>,
    /// 材质列表。
    pub materials: Vec<ImportedMaterial>,
    /// 纹理列表。
    pub textures: Vec<ImportedTexture>,
    /// 骨骼 / 蒙皮列表。
    pub skeletons: Vec<ImportedSkeleton>,
    /// 动画列表。
    pub animations: Vec<ImportedAnimation>,
    /// 引用的外部文件（相对源文件的 URI 原样记录，供数据库登记依赖）。
    #[serde(default)]
    pub external_files: Vec<String>,
    /// 导入过程中的非致命问题（不支持的扩展、降级处理等）。
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl ImportedScene {
    /// 查找 primitive 使用的材质。
    pub fn material_of(&self, primitive: &ImportedPrimitive) -> Option<&ImportedMaterial> {
        primitive
            .material
            .and_then(|index| self.materials.get(index))
    }

    /// 查找 primitive 使用的骨骼。
    pub fn skeleton_of(&self, primitive: &ImportedPrimitive) -> Option<&ImportedSkeleton> {
        primitive
            .skeleton
            .and_then(|index| self.skeletons.get(index))
    }

    /// 按名称查找动画（编辑器/状态机绑定用）。
    pub fn animation_by_name(&self, name: &str) -> Option<&ImportedAnimation> {
        self.animations
            .iter()
            .find(|animation| animation.name.as_deref() == Some(name))
    }
}

/// 场景节点。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedNode {
    /// 节点名（glTF 中可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 父节点索引（根节点为 `None`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    /// 子节点索引。
    #[serde(default)]
    pub children: Vec<usize>,
    /// 局部 TRS 变换。
    pub local_transform: Trs,
    /// 引用的网格索引。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<usize>,
    /// 引用的骨骼索引。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin: Option<usize>,
}

/// primitive 拓扑类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveMode {
    /// 点。
    Points,
    /// 线段。
    Lines,
    /// 闭合折线。
    LineLoop,
    /// 折线。
    LineStrip,
    /// 三角形（默认）。
    Triangles,
    /// 三角形带。
    TriangleStrip,
    /// 三角形扇。
    TriangleFan,
}

impl PrimitiveMode {
    /// 是否为三角形类拓扑（渲染器主路径）。
    pub const fn is_triangles(self) -> bool {
        matches!(
            self,
            Self::Triangles | Self::TriangleStrip | Self::TriangleFan
        )
    }
}

/// 网格（一个 glTF mesh 可含多个 primitive）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedMesh {
    /// 网格名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 子网格。
    pub primitives: Vec<ImportedPrimitive>,
}

/// 子网格（一组顶点属性 + 索引 + 可选蒙皮/材质）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedPrimitive {
    /// 顶点位置。
    pub positions: Vec<Vec3>,
    /// 顶点法线。
    #[serde(default)]
    pub normals: Vec<Vec3>,
    /// 顶点切线（w 为切线空间手性）。
    #[serde(default)]
    pub tangents: Vec<Vec4>,
    /// 第一套 UV。
    #[serde(default)]
    pub uvs: Vec<Vec2>,
    /// 蒙皮关节索引（每个顶点 4 个，skeleton 内 joint 索引）。
    #[serde(default)]
    pub joints: Vec<[u16; 4]>,
    /// 蒙皮权重（每个顶点 4 个）。
    #[serde(default)]
    pub weights: Vec<[f32; 4]>,
    /// 三角形索引；空表示非索引绘制。
    #[serde(default)]
    pub indices: Vec<u32>,
    /// 材质索引。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<usize>,
    /// 骨骼索引（由引用该网格的节点决定）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skeleton: Option<usize>,
    /// 拓扑类型。
    pub mode: PrimitiveMode,
    /// 顶点位置包围盒（导入期计算，可用于剔除/碰撞代理）。
    pub bounds: Aabb,
}

/// 材质 alpha 模式。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlphaMode {
    /// 完全不透明。
    Opaque,
    /// Alpha 裁剪。
    Mask,
    /// Alpha 混合。
    Blend,
}

/// 纹理引用（纹理索引 + UV 通道）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedTextureRef {
    /// 纹理索引（指向 [`ImportedScene::textures`]）。
    pub texture: usize,
    /// UV 通道（0 = TEXCOORD_0）。
    pub tex_coord: u32,
}

/// 纹理过滤模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureFilterMode {
    /// 最近邻。
    Nearest,
    /// 线性。
    #[default]
    Linear,
}

/// 纹理环绕模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureWrapMode {
    /// 边缘拉伸。
    ClampToEdge,
    /// 镜像重复。
    MirroredRepeat,
    /// 重复。
    #[default]
    Repeat,
}

/// 采样器设置。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSampler {
    /// 放大过滤。
    pub mag_filter: TextureFilterMode,
    /// 缩小过滤。
    pub min_filter: TextureFilterMode,
    /// S 方向环绕。
    pub wrap_s: TextureWrapMode,
    /// T 方向环绕。
    pub wrap_t: TextureWrapMode,
}

impl Default for ImportedSampler {
    fn default() -> Self {
        Self {
            mag_filter: TextureFilterMode::Linear,
            min_filter: TextureFilterMode::Linear,
            wrap_s: TextureWrapMode::Repeat,
            wrap_t: TextureWrapMode::Repeat,
        }
    }
}

/// 图片数据来源。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ImportedImage {
    /// 外部图片文件（URI 相对源文件）。
    ExternalFile {
        /// 相对 URI（原样，未做路径拼接）。
        uri: String,
        /// MIME 类型（如 `image/png`）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// 内嵌图片（GLB bufferView 或 data URI），保存原始编码字节（PNG/JPEG）。
    Embedded {
        /// MIME 类型。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        /// 原始图片编码数据。
        data: Vec<u8>,
    },
}

/// 纹理（采样器 + 图片）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedTexture {
    /// 纹理名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 采样器设置。
    pub sampler: ImportedSampler,
    /// 图片数据。
    pub image: ImportedImage,
}

/// PBR 材质（glTF metallic-roughness 子集）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedMaterial {
    /// 材质名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 基础色因子（线性空间 RGBA）。
    pub base_color_factor: Vec4,
    /// 基础色纹理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color_texture: Option<ImportedTextureRef>,
    /// 金属度因子。
    pub metallic_factor: f32,
    /// 粗糙度因子。
    pub roughness_factor: f32,
    /// 金属度-粗糙度纹理（G = roughness, B = metallic）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic_roughness_texture: Option<ImportedTextureRef>,
    /// 法线纹理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_texture: Option<ImportedTextureRef>,
    /// 法线强度。
    pub normal_scale: f32,
    /// 环境光遮蔽纹理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occlusion_texture: Option<ImportedTextureRef>,
    /// 环境光遮蔽强度。
    pub occlusion_strength: f32,
    /// 自发光因子。
    pub emissive_factor: Vec3,
    /// 自发光纹理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive_texture: Option<ImportedTextureRef>,
    /// Alpha 模式。
    pub alpha_mode: AlphaMode,
    /// Alpha 裁剪阈值（仅 Mask 模式使用）。
    pub alpha_cutoff: f32,
    /// 是否双面渲染。
    pub double_sided: bool,
}

impl Default for ImportedMaterial {
    fn default() -> Self {
        Self {
            name: None,
            base_color_factor: Vec4::new(1.0, 1.0, 1.0, 1.0),
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            occlusion_texture: None,
            occlusion_strength: 1.0,
            emissive_factor: Vec3::ZERO,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
        }
    }
}

/// 关节。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedJoint {
    /// 对应的 glTF node 索引。
    pub node: usize,
    /// 关节名（来自 node 名）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 父关节索引（根关节为 `None`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    /// 逆绑定矩阵（joint -> model 空间）。
    pub inverse_bind_matrix: Mat4,
}

/// 骨骼 / 蒙皮。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedSkeleton {
    /// 骨骼名（glTF skin 名）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 关节列表（顺序与 glTF skin.joints 一致，供 GPU joint palette 使用）。
    pub joints: Vec<ImportedJoint>,
    /// 根关节索引。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_joint: Option<usize>,
}

impl ImportedSkeleton {
    /// 按 node 索引查找关节索引。
    pub fn joint_index_by_node(&self, node: usize) -> Option<usize> {
        self.joints.iter().position(|joint| joint.node == node)
    }
}

/// 动画属性通道。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationProperty {
    /// 平移（3 分量）。
    Translation,
    /// 旋转四元数（4 分量，xyzw）。
    Rotation,
    /// 缩放（3 分量）。
    Scale,
    /// Morph target 权重（分量数 = values.len() / times.len()）。
    MorphTargetWeights,
}

impl AnimationProperty {
    /// 每个关键帧的分量数；`MorphTargetWeights` 返回 `None`（需从数据推断）。
    pub const fn component_count(self) -> Option<usize> {
        match self {
            Self::Translation | Self::Scale => Some(3),
            Self::Rotation => Some(4),
            Self::MorphTargetWeights => None,
        }
    }
}

/// 关键帧插值方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationInterpolation {
    /// 线性（旋转使用 slerp）。
    Linear,
    /// 阶跃。
    Step,
    /// 三次样条（每个关键帧输出 in-tangent / value / out-tangent 三个值）。
    CubicSpline,
}

/// 动画通道。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedAnimationChannel {
    /// 目标 glTF node 索引。
    pub target_node: usize,
    /// 目标关节索引（若该 node 属于某个 skeleton）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_joint: Option<usize>,
    /// 动画属性。
    pub property: AnimationProperty,
    /// 插值方式。
    pub interpolation: AnimationInterpolation,
    /// 关键帧时间（秒，升序）。
    pub times: Vec<f32>,
    /// 关键帧值（扁平 f32；按 `property.component_count()` 分组）。
    ///
    /// - `Linear`/`Step`：长度为 `times.len() * component_count`；
    /// - `CubicSpline`：每个关键帧存 `in-tangent`、`value`、`out-tangent`，
    ///   长度为 `times.len() * component_count * 3`。
    pub values: Vec<f32>,
}

/// 动画片段。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedAnimation {
    /// 动画名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 通道列表。
    pub channels: Vec<ImportedAnimationChannel>,
    /// 时长（秒）。
    pub duration: f32,
}

/// 校验导入数据的基础一致性（供数据库/测试使用）。
///
/// 检查：索引不越界、关节索引有效、动画通道时间升序。
pub fn validate_imported_scene(scene: &ImportedScene) -> AssetResult<()> {
    for (mesh_index, mesh) in scene.meshes.iter().enumerate() {
        for (prim_index, primitive) in mesh.primitives.iter().enumerate() {
            if (!primitive.normals.is_empty()
                && primitive.normals.len() != primitive.positions.len())
                || (!primitive.uvs.is_empty() && primitive.uvs.len() != primitive.positions.len())
                || (!primitive.joints.is_empty()
                    && primitive.joints.len() != primitive.positions.len())
                || (!primitive.weights.is_empty()
                    && primitive.weights.len() != primitive.positions.len())
            {
                return Err(AssetError::Import(format!(
                    "mesh {mesh_index} primitive {prim_index} 顶点属性数量不一致"
                )));
            }
            if let Some(material) = primitive.material {
                if material >= scene.materials.len() {
                    return Err(AssetError::Import(format!(
                        "mesh {mesh_index} primitive {prim_index} 材质索引越界: {material}"
                    )));
                }
            }
            if let Some(skeleton) = primitive.skeleton {
                if skeleton >= scene.skeletons.len() {
                    return Err(AssetError::Import(format!(
                        "mesh {mesh_index} primitive {prim_index} 骨骼索引越界: {skeleton}"
                    )));
                }
            }
        }
    }
    for (skeleton_index, skeleton) in scene.skeletons.iter().enumerate() {
        for (joint_index, joint) in skeleton.joints.iter().enumerate() {
            if let Some(parent) = joint.parent {
                if parent >= skeleton.joints.len() {
                    return Err(AssetError::Import(format!(
                        "skeleton {skeleton_index} joint {joint_index} 父关节越界: {parent}"
                    )));
                }
            }
        }
    }
    for animation in &scene.animations {
        for channel in &animation.channels {
            if !channel.times.windows(2).all(|pair| pair[0] <= pair[1]) {
                return Err(AssetError::Import(format!(
                    "动画 {:?} 的通道时间不是升序",
                    animation.name
                )));
            }
        }
    }
    Ok(())
}
