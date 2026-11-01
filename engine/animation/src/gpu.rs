//! GPU 蒙皮数据契约：顶点布局、joint palette 数据布局、shader 接口常量。
//!
//! 本模块**只产出数据/接口约定**，不创建 Vulkan 资源；GPU 资源由 render/rhi 侧负责。
//! 配套 shader 源码见 `engine/shaders/skinning.vert` / `skinning.frag`（手动 glslc 编译）。
//!
//! ## 1. 顶点布局（SkinnedVertex，stride = 64 bytes）
//!
//! | location | 属性 | 格式 | offset |
//! |---|---|---|---|
//! | 0 | position | R32G32B32_SFLOAT | 0 |
//! | 1 | normal | R32G32B32_SFLOAT | 12 |
//! | 2 | uv | R32G32_SFLOAT | 24 |
//! | 3 | joint_indices | R32G32B32A32_UINT | 32 |
//! | 4 | joint_weights | R32G32B32A32_SFLOAT | 48 |
//!
//! 前 32 字节与现有 `rhi-vulkan` 非蒙皮顶点布局完全一致，渲染侧可以共用缓冲区布局，
//! 只是把 stride 从 32 扩展到 64、增加 2 个顶点属性。
//!
//! ## 2. joint palette 布局
//!
//! ```glsl
//! layout(std430, set = 0, binding = 1) readonly buffer JointPalette {
//!     mat4 joints[];   // 每个元素 64 bytes，列主序
//! };
//! ```
//! - 元素个数 = `Skin::joint_count()`，元素 i 对应 `Skin::joints[i]`；
//! - 元素内容 = `world_joint_matrix * inverse_bind_matrix`（模型空间蒙皮）；
//! - 顶点着色器先做 palette 蒙皮，再乘角色实体 model matrix（push constant `model`）；
//! - 小骨骼（<= 256 关节）可以用 UBO（16 KB 上限）；更大骨骼建议 storage buffer。
//!
//! ## 3. shader 端蒙皮公式
//!
//! ```glsl
//! mat4 skin = inJointWeights.x * joints[inJointIndices.x]
//!           + inJointWeights.y * joints[inJointIndices.y]
//!           + inJointWeights.z * joints[inJointIndices.z]
//!           + inJointWeights.w * joints[inJointIndices.w];
//! vec4 skinned = skin * vec4(inPosition, 1.0);
//! ```

use crate::math::Mat4;
use crate::pose::Pose;
use crate::skeleton::Skeleton;
use crate::skin::Skin;

/// 单个 joint 矩阵的字节数（`mat4` = 64 bytes）。
pub const JOINT_MATRIX_BYTES: usize = 64;

/// joint_indices 顶点属性 location（承接现有 0/1/2）。
pub const JOINT_INDEX_ATTRIBUTE_LOCATION: u32 = 3;

/// joint_weights 顶点属性 location。
pub const JOINT_WEIGHT_ATTRIBUTE_LOCATION: u32 = 4;

/// joint palette 的 descriptor set（与 LightData 同在 set 0）。
pub const JOINT_PALETTE_SET: u32 = 0;

/// joint palette 的 binding（LightData 占用 binding 0）。
pub const JOINT_PALETTE_BINDING: u32 = 1;

/// 用 UBO 承载 palette 时建议的最大关节数（256 * 64B = 16 KB）。
pub const MAX_JOINTS_PER_PALETTE_UBO: usize = 256;

/// 蒙皮顶点总字节数（stride）。
pub const SKINNED_VERTEX_STRIDE: usize = 64;

/// 蒙皮顶点：现有 position/normal/uv + 4 骨骼索引 + 4 权重。
///
/// `joint_indices` 是 **palette 索引**（`Skin::joints` 下标），不是骨架全局关节索引；
/// 每个角色/蒙皮实例上传自己的 palette，shader 里直接 `joints[inJointIndices.x]`。
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// palette 索引（0..Skin::joint_count()）。
    pub joint_indices: [u32; 4],
    /// 对应权重（CPU 侧已归一化）。
    pub joint_weights: [f32; 4],
}

impl SkinnedVertex {
    /// 结构体字节数（应为 64）。
    pub const SIZE: usize = std::mem::size_of::<SkinnedVertex>();

    /// 非蒙皮顶点（权重全 0，shader 会退化为原位置；也可用于静态 mesh 共用布局）。
    pub fn unskinned(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            uv,
            joint_indices: [0; 4],
            joint_weights: [0.0; 4],
        }
    }

    /// 带一个骨骼影响（刚性绑定）的顶点。
    pub fn rigid(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], joint: u32) -> Self {
        Self {
            position,
            normal,
            uv,
            joint_indices: [joint, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    /// 写入 4 骨骼权重（自动归一化；权重和为 0 时保持非蒙皮语义）。
    pub fn with_weights(mut self, joint_indices: [u32; 4], weights: [f32; 4]) -> Self {
        let sum: f32 = weights.iter().sum();
        self.joint_indices = joint_indices;
        if sum > 1e-6 {
            self.joint_weights = [
                weights[0] / sum,
                weights[1] / sum,
                weights[2] / sum,
                weights[3] / sum,
            ];
        } else {
            self.joint_weights = [0.0; 4];
        }
        self
    }

    /// 是否带有效蒙皮权重。
    pub fn is_skinned(&self) -> bool {
        self.joint_weights.iter().sum::<f32>() > 1e-6
    }
}

/// 顶点属性描述（供 render 侧创建 `VkVertexInputAttributeDescription`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuVertexAttribute {
    pub location: u32,
    /// Vulkan 格式名（字符串便于跨 crate 传契约且不依赖 ash）。
    pub format: &'static str,
    pub offset: usize,
}

/// 蒙皮顶点属性表（顺序即 location 顺序）。
pub const SKINNED_VERTEX_ATTRIBUTES: [GpuVertexAttribute; 5] = [
    GpuVertexAttribute {
        location: 0,
        format: "VK_FORMAT_R32G32B32_SFLOAT",
        offset: 0,
    },
    GpuVertexAttribute {
        location: 1,
        format: "VK_FORMAT_R32G32B32_SFLOAT",
        offset: 12,
    },
    GpuVertexAttribute {
        location: 2,
        format: "VK_FORMAT_R32G32_SFLOAT",
        offset: 24,
    },
    GpuVertexAttribute {
        location: JOINT_INDEX_ATTRIBUTE_LOCATION,
        format: "VK_FORMAT_R32G32B32A32_UINT",
        offset: 32,
    },
    GpuVertexAttribute {
        location: JOINT_WEIGHT_ATTRIBUTE_LOCATION,
        format: "VK_FORMAT_R32G32B32A32_SFLOAT",
        offset: 48,
    },
];

/// 顶点着色器输入声明（可直接粘贴到 shader，location 与 [`SKINNED_VERTEX_ATTRIBUTES`] 对应）。
pub const SKINNING_VERTEX_INPUT_GLSL: &str = "\
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;
layout(location = 3) in uvec4 inJointIndices;
layout(location = 4) in vec4 inJointWeights;
";

/// joint palette 的 GLSL 声明（storage buffer 版本）。
pub const SKINNING_PALETTE_GLSL: &str = "\
layout(std430, set = 0, binding = 1) readonly buffer JointPalette {
    mat4 joints[];
};
";

/// shader 端蒙皮计算函数（GLSL）。
pub const SKINNING_FUNCTION_GLSL: &str = "\
mat4 buildSkinMatrix() {
    return inJointWeights.x * joints[inJointIndices.x]
         + inJointWeights.y * joints[inJointIndices.y]
         + inJointWeights.z * joints[inJointIndices.z]
         + inJointWeights.w * joints[inJointIndices.w];
}
";

/// CPU 侧 joint palette：连续存放的蒙皮矩阵，可直接上传 GPU。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JointPalette {
    /// 蒙皮矩阵（每个 = `world_joint * inverse_bind`）。
    pub matrices: Vec<Mat4>,
}

impl JointPalette {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_matrices(matrices: Vec<Mat4>) -> Self {
        Self { matrices }
    }

    /// 由 skin + 关节世界矩阵计算 palette。
    pub fn from_skin(skin: &Skin, world_joint_matrices: &[Mat4]) -> Self {
        Self {
            matrices: skin.skin_matrices(world_joint_matrices),
        }
    }

    /// 由骨架 + 蒙皮 + 姿态计算 palette（CPU 采样到 GPU 上传的完整链路）。
    pub fn from_pose(skeleton: &Skeleton, skin: &Skin, pose: &Pose) -> Self {
        let world = skeleton.compute_world_transforms(&pose.locals);
        Self::from_skin(skin, &world)
    }

    /// 生成 `count` 个单位矩阵的 palette（无蒙皮的静态 mesh 占位）。
    pub fn identity(count: usize) -> Self {
        Self {
            matrices: vec![Mat4::IDENTITY; count],
        }
    }

    pub fn push(&mut self, matrix: Mat4) {
        self.matrices.push(matrix);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.matrices.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.matrices.is_empty()
    }

    /// f32 数量（= 关节数 * 16）。
    pub fn float_count(&self) -> usize {
        self.matrices.len() * 16
    }

    /// 字节数（= 关节数 * 64）。
    pub fn byte_len(&self) -> usize {
        self.matrices.len() * JOINT_MATRIX_BYTES
    }

    /// 平铺为 f32 数组（列主序，每矩阵 16 个）。
    pub fn to_f32_vec(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.float_count());
        for matrix in &self.matrices {
            out.extend_from_slice(&matrix.to_cols_array());
        }
        out
    }

    /// 平铺为 little-endian 字节（Vulkan `mat4` 直接可用）。
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.byte_len());
        for value in self.to_f32_vec() {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    /// 写入调用方提供的字节缓冲，返回写入字节数；缓冲不足时只写能容纳的部分矩阵。
    pub fn write_le_bytes(&self, out: &mut [u8]) -> usize {
        let mut written = 0;
        for matrix in &self.matrices {
            if written + JOINT_MATRIX_BYTES > out.len() {
                break;
            }
            for value in matrix.to_cols_array() {
                out[written..written + 4].copy_from_slice(&value.to_le_bytes());
                written += 4;
            }
        }
        written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Quat, Transform, Vec3};
    use crate::skeleton::Skeleton;

    #[test]
    fn skinned_vertex_layout_matches_contract() {
        assert_eq!(SkinnedVertex::SIZE, SKINNED_VERTEX_STRIDE);
        assert_eq!(SKINNED_VERTEX_STRIDE, 64);
        // 属性 offset 连续且不重叠。
        let mut expected = 0;
        for attribute in SKINNED_VERTEX_ATTRIBUTES {
            assert_eq!(
                attribute.offset, expected,
                "location {}",
                attribute.location
            );
            let size = match attribute.location {
                0 | 1 => 12,
                2 => 8,
                3 => 16,
                4 => 16,
                _ => unreachable!(),
            };
            expected += size;
        }
        assert_eq!(expected, SKINNED_VERTEX_STRIDE);
    }

    #[test]
    fn palette_bytes_are_64_per_matrix_and_identity_at_rest() {
        let mut skeleton = Skeleton::new("s");
        skeleton.add_root_joint(
            "root",
            Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
        );
        skeleton
            .add_joint(
                "child",
                0,
                Transform::from_trs(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY),
            )
            .unwrap();
        let skin = crate::skin::Skin::from_skeleton(&skeleton);
        let pose = Pose::from_locals(skeleton.default_pose());
        let palette = JointPalette::from_pose(&skeleton, &skin, &pose);
        assert_eq!(palette.len(), 2);
        assert_eq!(palette.byte_len(), 2 * JOINT_MATRIX_BYTES);
        assert_eq!(palette.to_f32_vec().len(), 32);
        for matrix in &palette.matrices {
            assert!(
                matrix.approx_eq(&Mat4::IDENTITY, 1e-4),
                "rest pose palette 应为单位矩阵"
            );
        }
        let bytes = palette.to_le_bytes();
        assert_eq!(bytes.len(), palette.byte_len());
    }
}
