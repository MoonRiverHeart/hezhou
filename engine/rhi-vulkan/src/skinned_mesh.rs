//! 真实蒙皮网格上传数据（0.2 task-28，按 Lead 定稿契约）
//!
//! 公开契约：
//! ```ignore
//! pub struct SkinnedMeshUpload {
//!     pub entity_id: u64,
//!     pub vertices: Vec<hezhou_animation::SkinnedVertex>,
//!     pub indices: Vec<u32>,
//!     pub material: Option<hezhou_render_pipeline::pbr::PbrMaterial>,
//!     pub texture_path: Option<String>,
//! }
//! ```
//! joint palette **不放在 upload 里**，由 `UIVulkanRenderer::upload_skin_palette(entity_id, bytes)`
//! 按实体单独上传（每关节 64B，mat4 列主序，最多 256 个）。
//!
//! 数据契约（与 `hezhou-animation` / `shaders/skinning.vert` 一致）:
//! - 顶点 stride = 64：position(12) + normal(12) + uv(8) + joint_indices(16) + joint_weights(16)
//! - 索引为 u32，必须 3 的倍数且 < 顶点数
//! - palette 为 N × 64B（N ≤ [`MAX_PALETTE_JOINTS`]）
//!
//! **0.2 限制（TODO）**:
//! - 蒙皮着色当前复用 `rotation.frag`（Blinn-Phong 光照回退）；`material` 字段先保留给后续
//!   PBR 蒙皮着色，`texture_path` 暂未用于蒙皮 draw
//! - 不支持 LOD / 骨骼压缩 / 动画状态机直接驱动（palette 由调用方每帧上传）

use std::collections::HashMap;

use hezhou_animation::gpu::{SkinnedVertex, JOINT_MATRIX_BYTES, SKINNED_VERTEX_STRIDE};
use hezhou_core::ecs::EntityId;
use hezhou_render_pipeline::PbrMaterial;

/// 单实体 palette 最大关节数（256 × 64B = 16 KiB）
pub const MAX_PALETTE_JOINTS: usize = 256;
/// 单实体 palette 最大字节数
pub const MAX_PALETTE_BYTES: usize = MAX_PALETTE_JOINTS * JOINT_MATRIX_BYTES;
/// 单个蒙皮网格最大顶点数（防止异常上传打爆显存）
pub const MAX_SKINNED_VERTICES: usize = 1_048_576;
/// 单个蒙皮网格最大索引数
pub const MAX_SKINNED_INDICES: usize = 4_194_304;

/// 顶点 stride（= 64B，供测试/布局断言）
pub const fn skinned_vertex_stride() -> usize {
    SKINNED_VERTEX_STRIDE
}

/// 把 `SkinnedVertex` 序列化为 GPU 64B stride 字节流（64B/顶点）
///
/// `SkinnedVertex` 来自零依赖的 animation crate，未实现 `bytemuck::Pod`，
/// 因此手动按字段顺序序列化。
pub fn skinned_vertex_bytes(vertices: &[SkinnedVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * SKINNED_VERTEX_STRIDE);
    for vertex in vertices {
        for value in vertex
            .position
            .iter()
            .chain(vertex.normal.iter())
            .chain(vertex.uv.iter())
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in &vertex.joint_indices {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in &vertex.joint_weights {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

/// 单位矩阵 palette（1 关节，64B）— 未上传 palette 时的安全默认值
pub fn identity_palette_bytes() -> Vec<u8> {
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::with_capacity(JOINT_MATRIX_BYTES);
    for column in identity {
        for value in column {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

/// 蒙皮网格上传/校验错误
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkinnedMeshUploadError {
    /// entity id 为 0
    InvalidEntityId,
    /// 顶点为空
    EmptyVertices,
    /// 索引为空
    EmptyIndices,
    /// 顶点数超过上限
    TooManyVertices(usize),
    /// 索引数超过上限
    TooManyIndices(usize),
    /// 索引数不是 3 的倍数
    InvalidIndexCount(usize),
    /// 索引越界
    IndexOutOfRange {
        /// 越界索引值
        index: u32,
        /// 顶点数量
        vertex_count: usize,
    },
    /// palette 字节数不是 64 的倍数
    PaletteNotMatrixAligned(usize),
    /// palette 超过每实体上限
    PaletteTooLarge(usize),
}

impl std::fmt::Display for SkinnedMeshUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkinnedMeshUploadError::InvalidEntityId => write!(f, "entity_id 不能为 0"),
            SkinnedMeshUploadError::EmptyVertices => write!(f, "蒙皮网格顶点为空"),
            SkinnedMeshUploadError::EmptyIndices => write!(f, "蒙皮网格索引为空"),
            SkinnedMeshUploadError::TooManyVertices(count) => {
                write!(f, "顶点数 {count} 超过上限 {MAX_SKINNED_VERTICES}")
            }
            SkinnedMeshUploadError::TooManyIndices(count) => {
                write!(f, "索引数 {count} 超过上限 {MAX_SKINNED_INDICES}")
            }
            SkinnedMeshUploadError::InvalidIndexCount(count) => {
                write!(f, "索引数 {count} 不是 3 的倍数")
            }
            SkinnedMeshUploadError::IndexOutOfRange {
                index,
                vertex_count,
            } => write!(f, "索引 {index} 越界（顶点数 {vertex_count}）"),
            SkinnedMeshUploadError::PaletteNotMatrixAligned(len) => {
                write!(f, "palette 字节数 {len} 不是 {JOINT_MATRIX_BYTES} 的倍数")
            }
            SkinnedMeshUploadError::PaletteTooLarge(len) => {
                write!(f, "palette 字节数 {len} 超过上限 {MAX_PALETTE_BYTES}")
            }
        }
    }
}

impl std::error::Error for SkinnedMeshUploadError {}

/// 校验 palette 字节（每关节 64B、mat4 列主序、最多 256 个）
pub fn validate_palette_bytes(bytes: &[u8]) -> Result<(), SkinnedMeshUploadError> {
    if !bytes.len().is_multiple_of(JOINT_MATRIX_BYTES) {
        return Err(SkinnedMeshUploadError::PaletteNotMatrixAligned(bytes.len()));
    }
    if bytes.len() > MAX_PALETTE_BYTES {
        return Err(SkinnedMeshUploadError::PaletteTooLarge(bytes.len()));
    }
    Ok(())
}

/// 真实蒙皮网格上传数据（Lead 定稿契约字段，joint palette 单独上传）
#[derive(Clone, Debug, Default)]
pub struct SkinnedMeshUpload {
    /// 绑定实体
    pub entity_id: EntityId,
    /// 蒙皮顶点（64B stride）
    pub vertices: Vec<SkinnedVertex>,
    /// 三角索引（u32）
    pub indices: Vec<u32>,
    /// 可选 PBR 材质（蒙皮着色复用 rotation.frag，当前作为上传数据保留）
    pub material: Option<PbrMaterial>,
    /// 可选 albedo 纹理路径（0.2 暂未用于蒙皮 draw）
    pub texture_path: Option<String>,
}

impl SkinnedMeshUpload {
    /// 创建空上传（需要再填 vertices/indices）
    pub fn new(entity_id: EntityId) -> Self {
        Self {
            entity_id,
            vertices: Vec::new(),
            indices: Vec::new(),
            material: None,
            texture_path: None,
        }
    }

    /// 从顶点/索引构造
    pub fn from_parts(entity_id: EntityId, vertices: Vec<SkinnedVertex>, indices: Vec<u32>) -> Self {
        Self {
            entity_id,
            vertices,
            indices,
            material: None,
            texture_path: None,
        }
    }

    /// 设置 PBR 材质
    pub fn with_material(mut self, material: PbrMaterial) -> Self {
        self.material = Some(material);
        self
    }

    /// 设置 albedo 纹理路径
    pub fn with_texture_path(mut self, path: impl Into<String>) -> Self {
        self.texture_path = Some(path.into());
        self
    }

    /// 顶点数
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// 索引数
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// 三角面数
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// 顶点数据字节（64B/顶点）
    pub fn vertex_bytes(&self) -> Vec<u8> {
        skinned_vertex_bytes(&self.vertices)
    }

    /// 索引数据字节（u32 小端）
    pub fn index_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.indices)
    }

    /// 校验布局/索引（palette 由 `upload_skin_palette` 单独校验）
    pub fn validate(&self) -> Result<(), SkinnedMeshUploadError> {
        if self.entity_id == 0 {
            return Err(SkinnedMeshUploadError::InvalidEntityId);
        }
        if self.vertices.is_empty() {
            return Err(SkinnedMeshUploadError::EmptyVertices);
        }
        if self.indices.is_empty() {
            return Err(SkinnedMeshUploadError::EmptyIndices);
        }
        if self.vertices.len() > MAX_SKINNED_VERTICES {
            return Err(SkinnedMeshUploadError::TooManyVertices(self.vertices.len()));
        }
        if self.indices.len() > MAX_SKINNED_INDICES {
            return Err(SkinnedMeshUploadError::TooManyIndices(self.indices.len()));
        }
        if self.indices.len() % 3 != 0 {
            return Err(SkinnedMeshUploadError::InvalidIndexCount(self.indices.len()));
        }
        for index in &self.indices {
            if *index as usize >= self.vertices.len() {
                return Err(SkinnedMeshUploadError::IndexOutOfRange {
                    index: *index,
                    vertex_count: self.vertices.len(),
                });
            }
        }
        Ok(())
    }
}

/// EntityId → [`SkinnedMeshUpload`] 的 CPU 侧注册表
///
/// 渲染器在其上叠加 GPU buffer（每实体 vertex/index/palette buffer）；palette 由
/// renderer 的 `upload_skin_palette` 单独维护。
#[derive(Clone, Debug, Default)]
pub struct SkinnedMeshRegistry {
    meshes: HashMap<EntityId, SkinnedMeshUpload>,
}

impl SkinnedMeshRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入/替换实体网格（会先校验；返回被替换的旧数据）
    pub fn insert(
        &mut self,
        upload: SkinnedMeshUpload,
    ) -> Result<Option<SkinnedMeshUpload>, SkinnedMeshUploadError> {
        upload.validate()?;
        Ok(self.meshes.insert(upload.entity_id, upload))
    }

    /// 删除实体网格
    pub fn remove(&mut self, entity_id: EntityId) -> Option<SkinnedMeshUpload> {
        self.meshes.remove(&entity_id)
    }

    /// 查询实体网格
    pub fn get(&self, entity_id: EntityId) -> Option<&SkinnedMeshUpload> {
        self.meshes.get(&entity_id)
    }

    /// 是否包含实体
    pub fn contains(&self, entity_id: EntityId) -> bool {
        self.meshes.contains_key(&entity_id)
    }

    /// 网格数量
    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    /// 是否为空（空 → 渲染器使用合成 cube fallback）
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// 实体列表（稳定排序便于调试/测试）
    pub fn entity_ids(&self) -> Vec<EntityId> {
        let mut ids: Vec<EntityId> = self.meshes.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// 清空
    pub fn clear(&mut self) {
        self.meshes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_vertex(position: [f32; 3], joint: u32) -> SkinnedVertex {
        SkinnedVertex {
            position,
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            joint_indices: [joint, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    fn triangle_upload(entity_id: EntityId) -> SkinnedMeshUpload {
        SkinnedMeshUpload::from_parts(
            entity_id,
            vec![
                cube_vertex([0.0, 0.0, 0.0], 0),
                cube_vertex([1.0, 0.0, 0.0], 0),
                cube_vertex([0.0, 1.0, 0.0], 0),
            ],
            vec![0, 1, 2],
        )
    }

    #[test]
    fn vertex_bytes_are_64_per_vertex() {
        let upload = triangle_upload(1);
        assert_eq!(upload.vertex_count(), 3);
        assert_eq!(upload.index_count(), 3);
        assert_eq!(upload.triangle_count(), 1);
        assert_eq!(skinned_vertex_stride(), 64);
        assert_eq!(upload.vertex_bytes().len(), 3 * 64);
        assert_eq!(upload.index_bytes().len(), 3 * 4);
        assert!(upload.validate().is_ok());
        // 字段契约：entity_id / vertices / indices / material / texture_path
        let with_meta = triangle_upload(2)
            .with_material(PbrMaterial::default())
            .with_texture_path("tex/a.png");
        assert!(with_meta.material.is_some());
        assert_eq!(with_meta.texture_path.as_deref(), Some("tex/a.png"));
    }

    #[test]
    fn palette_validation() {
        assert_eq!(identity_palette_bytes().len(), 64);
        assert!(validate_palette_bytes(&identity_palette_bytes()).is_ok());
        assert!(validate_palette_bytes(&[]).is_ok());
        assert_eq!(
            validate_palette_bytes(&[0u8; 40]),
            Err(SkinnedMeshUploadError::PaletteNotMatrixAligned(40))
        );
        assert_eq!(
            validate_palette_bytes(&vec![0u8; MAX_PALETTE_BYTES + 64]),
            Err(SkinnedMeshUploadError::PaletteTooLarge(MAX_PALETTE_BYTES + 64))
        );
        // 256 关节（16KB）恰好允许
        assert!(validate_palette_bytes(&vec![0u8; MAX_PALETTE_BYTES]).is_ok());
    }

    #[test]
    fn validate_rejects_bad_data() {
        assert_eq!(
            triangle_upload(0).validate(),
            Err(SkinnedMeshUploadError::InvalidEntityId)
        );
        let mut empty_vertices = triangle_upload(1);
        empty_vertices.vertices.clear();
        assert_eq!(
            empty_vertices.validate(),
            Err(SkinnedMeshUploadError::EmptyVertices)
        );
        let mut empty_indices = triangle_upload(1);
        empty_indices.indices.clear();
        assert_eq!(
            empty_indices.validate(),
            Err(SkinnedMeshUploadError::EmptyIndices)
        );
        let mut non_tri = triangle_upload(1);
        non_tri.indices = vec![0, 1];
        assert_eq!(
            non_tri.validate(),
            Err(SkinnedMeshUploadError::InvalidIndexCount(2))
        );
        let mut out_of_range = triangle_upload(1);
        out_of_range.indices = vec![0, 1, 99];
        assert!(matches!(
            out_of_range.validate(),
            Err(SkinnedMeshUploadError::IndexOutOfRange { index: 99, .. })
        ));
    }

    #[test]
    fn registry_insert_get_remove_replace() {
        let mut registry = SkinnedMeshRegistry::new();
        assert!(registry.is_empty());
        assert!(registry.insert(triangle_upload(1)).unwrap().is_none());
        assert!(registry.insert(triangle_upload(2)).unwrap().is_none());
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.entity_ids(), vec![1, 2]);
        assert!(registry.contains(1));
        // 替换返回旧数据
        let replacement = triangle_upload(1).with_texture_path("tex/a.png");
        let old = registry.insert(replacement).unwrap();
        assert!(old.is_some());
        assert_eq!(
            registry.get(1).unwrap().texture_path.as_deref(),
            Some("tex/a.png")
        );
        // 移除
        assert!(registry.remove(1).is_some());
        assert!(!registry.remove(1).is_some());
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.entity_ids(), vec![2]);
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn registry_rejects_invalid_upload() {
        let mut registry = SkinnedMeshRegistry::new();
        let mut upload = triangle_upload(1);
        upload.indices = vec![0, 1, 42];
        assert!(registry.insert(upload).is_err());
        assert!(registry.is_empty());
    }
}
