//! BVH（Bounding Volume Hierarchy）结构定义
//!
//! GPU-friendly的BVH节点和树结构，用于光线追踪加速。
//! BvhNode布局严格匹配GLSL std430对齐规则，32 bytes per节点。
//!
//! std430布局说明:
//! - vec3在std430中base alignment = 16 bytes（与vec4相同）
//! - 但vec3后面紧跟uint时，uint可以填充vec3→vec4的空隙
//! - 因此BvhNode的布局为:
//!   offset 0:  boundsMin[3] (vec3, 12 bytes)
//!   offset 12: leftChild (uint, 4 bytes) — 填充vec3→vec4空隙
//!   offset 16: boundsMax[3] (vec3, 12 bytes)
//!   offset 28: rightChild (uint, 4 bytes) — 填充vec3→vec4空隙
//!   总计: 32 bytes, struct alignment = 16 (vec3最大成员), 32是16的倍数 ✓
//!
//! 对应GLSL声明:
//! ```glsl
//! struct BvhNode {
//!     vec3 boundsMin;   // offset 0
//!     uint leftChild;   // offset 12
//!     vec3 boundsMax;   // offset 16
//!     uint rightChild;  // offset 28
//! };
//! ```
//!
//! isLeaf编码: leftChild == LEAF_MARKER (0xFFFFFFFF) 表示叶子节点，
//! 此时rightChild存储三角形起始索引。
//! 这是GPU BVH的标准做法，避免额外的isLeaf字段破坏32字节对齐。

use bytemuck::{Pod, Zeroable};
use hezhou_geometry::{BoundingBox, MeshData};

/// 叶子节点标记值 — leftChild == LEAF_MARKER时表示叶子节点
pub const LEAF_MARKER: u32 = 0xFFFFFFFF;

/// BVH节点 — GPU-friendly布局，32 bytes per节点
///
/// #[repr(C)]确保Rust内存布局与C/GLSL一致。
/// Pod + Zeroable支持bytemuck直接cast为字节切片上传到GPU。
///
/// **std430对齐验证**:
/// - boundsMin (vec3): offset 0, size 12, alignment 16 ✓
/// - leftChild (uint): offset 12, size 4, alignment 4 ✓ (填充vec3空隙)
/// - boundsMax (vec3): offset 16, size 12, alignment 16 ✓
/// - rightChild (uint): offset 28, size 4, alignment 4 ✓ (填充vec3空隙)
/// - struct size: 32, struct alignment: 16, 32 % 16 == 0 ✓
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct BvhNode {
    /// AABB最小边界 [x, y, z] — GLSL vec3 boundsMin
    pub bounds_min: [f32; 3],
    /// 左子节点索引 — 叶子节点时为LEAF_MARKER(0xFFFFFFFF)
    /// 填充vec3→vec4对齐空隙，同时存储有用数据
    pub left_child: u32,
    /// AABB最大边界 [x, y, z] — GLSL vec3 boundsMax
    pub bounds_max: [f32; 3],
    /// 右子节点索引 — 叶子节点时存储三角形起始索引
    /// 填充vec3→vec4对齐空隙，同时存储有用数据
    pub right_child: u32,
}

impl BvhNode {
    /// 创建内部节点
    pub fn interior(
        bounds_min: [f32; 3],
        bounds_max: [f32; 3],
        left_child: u32,
        right_child: u32,
    ) -> Self {
        Self {
            bounds_min,
            left_child,
            bounds_max,
            right_child,
        }
    }

    /// 创建叶子节点
    ///
    /// left_child设为LEAF_MARKER，right_child存储三角形起始索引。
    pub fn leaf(bounds_min: [f32; 3], bounds_max: [f32; 3], triangle_start: u32) -> Self {
        Self {
            bounds_min,
            left_child: LEAF_MARKER,
            bounds_max,
            right_child: triangle_start,
        }
    }

    /// 是否为叶子节点
    pub fn is_leaf(&self) -> bool {
        self.left_child == LEAF_MARKER
    }

    /// 获取叶子节点的三角形起始索引
    ///
    /// 仅在is_leaf()为true时有效。
    pub fn triangle_start_index(&self) -> u32 {
        self.right_child
    }

    /// 获取内部节点的左子节点索引
    ///
    /// 仅在is_leaf()为false时有效。
    pub fn left_index(&self) -> u32 {
        self.left_child
    }

    /// 获取内部节点的右子节点索引
    ///
    /// 仅在is_leaf()为false时有效。
    pub fn right_index(&self) -> u32 {
        self.right_child
    }

    /// 从BoundingBox创建节点bounds数组
    pub fn bounds_from_bbox(bbox: &BoundingBox) -> ([f32; 3], [f32; 3]) {
        (
            [bbox.min.x, bbox.min.y, bbox.min.z],
            [bbox.max.x, bbox.max.y, bbox.max.z],
        )
    }
}

/// BVH树结构
///
/// 包含节点数组（GPU上传用）和三角形索引映射。
/// nodes数组可直接通过bytemuck cast为字节切片上传到GPU SSBO。
pub struct Bvh {
    /// BVH节点数组 — 可直接上传到GPU
    pub nodes: Vec<BvhNode>,
    /// 三角形索引映射 — 叶子节点引用的原始三角形索引
    /// nodes[i].right_child（叶子时）指向此数组的起始位置
    pub triangle_indices: Vec<u32>,
}

impl Bvh {
    /// 创建空BVH
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            triangle_indices: Vec::new(),
        }
    }

    /// 从MeshData构建BVH
    ///
    /// 使用MeshData的顶点和索引构建BVH树，
    /// 每个叶子节点包含一个三角形。
    ///
    /// 构建算法: 自顶向下递归分割，使用中点分割策略。
    /// 1. 计算所有三角形的bounding box
    /// 2. 选择最长轴的中点作为分割点
    /// 3. 递归构建左右子树
    /// 4. 叶子节点包含单个三角形
    pub fn build(mesh: &MeshData) -> Self {
        if mesh.indices.is_empty() {
            return Self::empty();
        }

        let triangle_count = mesh.triangle_count();
        let mut triangle_bboxes = Vec::with_capacity(triangle_count);
        let mut triangle_indices = Vec::with_capacity(triangle_count);

        // 计算每个三角形的bounding box
        for chunk in mesh.indices.chunks(3) {
            if chunk.len() == 3 {
                let i0 = chunk[0] as usize;
                let i1 = chunk[1] as usize;
                let i2 = chunk[2] as usize;

                if i0 < mesh.vertices.len() && i1 < mesh.vertices.len() && i2 < mesh.vertices.len() {
                    let p0 = mesh.vertices[i0].position;
                    let p1 = mesh.vertices[i1].position;
                    let p2 = mesh.vertices[i2].position;

                    let min = [
                        p0[0].min(p1[0]).min(p2[0]),
                        p0[1].min(p1[1]).min(p2[1]),
                        p0[2].min(p1[2]).min(p2[2]),
                    ];
                    let max = [
                        p0[0].max(p1[0]).max(p2[0]),
                        p0[1].max(p1[1]).max(p2[1]),
                        p0[2].max(p1[2]).max(p2[2]),
                    ];

                    triangle_bboxes.push((min, max));
                    triangle_indices.push(chunk[0]);
                    triangle_indices.push(chunk[1]);
                    triangle_indices.push(chunk[2]);
                }
            }
        }

        if triangle_bboxes.is_empty() {
            return Self::empty();
        }

        // 构建索引列表（0到triangle_count-1）
        let mut object_indices: Vec<u32> = (0..triangle_bboxes.len() as u32).collect();

        // 递归构建BVH节点
        let mut nodes = Vec::new();
        Self::build_recursive(
            &triangle_bboxes,
            &mut object_indices,
            &mut nodes,
            &triangle_indices,
        );

        Self {
            nodes,
            triangle_indices,
        }
    }

    /// 递归构建BVH子树
    ///
    /// 返回子树根节点的索引。
    fn build_recursive(
        triangle_bboxes: &[( [f32; 3], [f32; 3] )],
        object_indices: &mut Vec<u32>,
        nodes: &mut Vec<BvhNode>,
        triangle_indices: &[u32],
    ) -> u32 {
        if object_indices.len() == 1 {
            // 叶子节点 — 单个三角形
            let tri_idx = object_indices[0] as usize;
            let (min, max) = triangle_bboxes[tri_idx];
            let start_index = tri_idx * 3; // triangle_indices中的起始位置

            let node = BvhNode::leaf(min, max, start_index as u32);
            let node_index = nodes.len() as u32;
            nodes.push(node);
            return node_index;
        }

        // 计算所有对象的合并bounding box
        let (parent_min, parent_max) = Self::compute_combined_bounds(triangle_bboxes, object_indices);

        // 选择最长轴
        let extent = [
            parent_max[0] - parent_min[0],
            parent_max[1] - parent_min[1],
            parent_max[2] - parent_min[2],
        ];
        let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] { 0 }
                   else if extent[1] >= extent[2] { 1 }
                   else { 2 };

        // 中点分割
        let midpoint = parent_min[axis] + extent[axis] * 0.5;

        // 按中点分割对象到左右两组
        let mut left_indices = Vec::new();
        let mut right_indices = Vec::new();

        for &idx in object_indices.iter() {
            let tri_idx = idx as usize;
            let (bbox_min, _bbox_max) = triangle_bboxes[tri_idx];
            let center = (bbox_min[axis] + _bbox_max[axis]) * 0.5;

            if center < midpoint {
                left_indices.push(idx);
            } else {
                right_indices.push(idx);
            }
        }

        // 防止空分割 — 如果某侧为空，强制分配一个对象
        if left_indices.is_empty() {
            left_indices.push(object_indices[0]);
            right_indices.retain(|&idx| idx != object_indices[0]);
        }
        if right_indices.is_empty() {
            right_indices.push(object_indices[object_indices.len() - 1]);
            left_indices.retain(|&idx| idx != object_indices[object_indices.len() - 1]);
        }

        // 先预留父节点位置（占位），然后递归构建子树
        let parent_node_index = nodes.len() as u32;
        nodes.push(BvhNode::default()); // 占位节点

        // 递归构建左子树
        let left_child = Self::build_recursive(
            triangle_bboxes,
            &mut left_indices,
            nodes,
            triangle_indices,
        );

        // 递归构建右子树
        let right_child = Self::build_recursive(
            triangle_bboxes,
            &mut right_indices,
            nodes,
            triangle_indices,
        );

        // 更新父节点
        nodes[parent_node_index as usize] = BvhNode::interior(
            parent_min,
            parent_max,
            left_child,
            right_child,
        );

        parent_node_index
    }

    /// 计算一组对象的合并bounding box
    fn compute_combined_bounds(
        triangle_bboxes: &[( [f32; 3], [f32; 3] )],
        object_indices: &[u32],
    ) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::MAX, f32::MAX, f32::MAX];
        let mut max = [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];

        for &idx in object_indices.iter() {
            let tri_idx = idx as usize;
            let (bbox_min, bbox_max) = triangle_bboxes[tri_idx];
            for i in 0..3 {
                min[i] = min[i].min(bbox_min[i]);
                max[i] = max[i].max(bbox_max[i]);
            }
        }

        (min, max)
    }

    /// 获取节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取节点数据的字节大小（用于GPU SSBO上传）
    pub fn nodes_byte_size(&self) -> usize {
        self.nodes.len() * std::mem::size_of::<BvhNode>()
    }

    /// 获取节点数据作为字节切片（用于GPU上传）
    ///
    /// 使用bytemuck安全转换，无需unsafe。
    pub fn nodes_as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.nodes)
    }
}