//! 骨骼蒙皮 pipeline 接入计划（0.2）
//!
//! 数据契约由 `hezhou-animation` 提供（`SkinnedVertex` / `SKINNED_VERTEX_ATTRIBUTES` /
//! `JointPalette`），本模块把契约翻译成 Vulkan 顶点输入与描述符绑定描述，
//! **本轮不创建 GPU pipeline**（避免改动现有非蒙皮路径），只提供可测试的接入计划与
//! shader 产物校验，供下一轮在 `UIVulkanRenderer` 中创建蒙皮 pipeline 时直接使用。
//!
//! 契约要点（与 `hezhou_animation::gpu` 一致，测试会交叉校验）:
//! - 顶点 stride = 64；前 32 字节与现有非蒙皮顶点布局一致
//! - location 3 = `R32G32B32A32_UINT`（joint indices），offset 32
//! - location 4 = `R32G32B32A32_SFLOAT`（joint weights），offset 48
//! - joint palette: set=0 binding=1（binding 0 是 LightData），std430 `mat4 joints[]`
//! - 每帧 `AnimationStateMachine::sample` → `JointPalette::from_pose` → `to_le_bytes()` 上传
//! - push constant 复用 raster 现有 144 bytes 布局（`skinning.vert` 与 `rotation.vert` 一致）
//!
//! 渲染侧接入步骤（下一轮 TODO）:
//! 1. 新增蒙皮 pipeline（vertex = `skinning.vert.spv`，fragment 可复用 `rotation.frag.spv`）
//! 2. 顶点输入使用 [`SkinnedVertexInput::create_info`]（stride 64 + 5 个 attribute）
//! 3. 角色创建 joint palette buffer + descriptor set 0 binding 1（≤256 关节用 UBO）
//! 4. draw 前上传 `JointPalette::to_le_bytes()`（绑定到 vertex stage）
//! 5. 非蒙皮实体继续走原 pipeline（不回归）

use ash::vk;
use hezhou_animation::gpu::{
    JOINT_MATRIX_BYTES, JOINT_PALETTE_BINDING, JOINT_PALETTE_SET, MAX_JOINTS_PER_PALETTE_UBO,
    SKINNED_VERTEX_ATTRIBUTES, SKINNED_VERTEX_STRIDE,
};

/// 蒙皮顶点 stride（字节，= 64）
pub const SKINNED_STRIDE_BYTES: u32 = SKINNED_VERTEX_STRIDE as u32;
/// joint indices 顶点属性 location（= 3）
pub const JOINT_INDEX_ATTRIBUTE_LOCATION: u32 = 3;
/// joint weights 顶点属性 location（= 4）
pub const JOINT_WEIGHT_ATTRIBUTE_LOCATION: u32 = 4;
/// 单个 joint 矩阵字节数（mat4 = 64）
pub const JOINT_BYTES: u32 = JOINT_MATRIX_BYTES as u32;
/// SSBO 承载 palette 时的保守关节数上限（256 * 64B = 16 KiB，与 UBO 建议值一致）
pub const MAX_JOINTS_PER_PALETTE_STORAGE: usize = MAX_JOINTS_PER_PALETTE_UBO;

/// joint palette 承载方式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointPaletteMode {
    /// std430 readonly storage buffer（`mat4 joints[]`，无 256 关节限制，推荐）
    StorageBuffer,
    /// uniform buffer（≤256 关节，兼容性更好）
    UniformBuffer,
}

impl JointPaletteMode {
    /// 对应的 Vulkan descriptor 类型
    pub fn descriptor_type(self) -> vk::DescriptorType {
        match self {
            JointPaletteMode::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
            JointPaletteMode::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        }
    }

    /// 关节数是否在该模式容量内
    pub fn supports_joint_count(self, joint_count: usize) -> bool {
        match self {
            JointPaletteMode::StorageBuffer => joint_count <= MAX_JOINTS_PER_PALETTE_STORAGE,
            JointPaletteMode::UniformBuffer => joint_count <= MAX_JOINTS_PER_PALETTE_UBO,
        }
    }
}

/// palette 上传字节数
pub const fn joint_palette_bytes(joint_count: usize) -> usize {
    joint_count * JOINT_MATRIX_BYTES
}

/// joint palette 的 descriptor set layout binding
///
/// set = 0（`JOINT_PALETTE_SET`），binding = 1（LightData 占 binding 0），
/// 只被顶点着色器读取（GPU 蒙皮在 vertex shader）。
pub fn joint_palette_layout_binding(
    mode: JointPaletteMode,
) -> vk::DescriptorSetLayoutBinding<'static> {
    vk::DescriptorSetLayoutBinding {
        binding: JOINT_PALETTE_BINDING,
        descriptor_type: mode.descriptor_type(),
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::VERTEX,
        p_immutable_samplers: std::ptr::null(),
        ..Default::default()
    }
}

/// 蒙皮顶点输入描述（拥有 Vulkan 结构所需的数组，便于安全传递）
pub struct SkinnedVertexInput {
    /// 顶点绑定（stride = 64）
    pub binding: vk::VertexInputBindingDescription,
    /// 5 个顶点属性（0..=4）
    pub attributes: [vk::VertexInputAttributeDescription; 5],
}

impl SkinnedVertexInput {
    /// 按 `hezhou_animation` 契约构造
    pub fn new() -> Self {
        let mut attributes = [vk::VertexInputAttributeDescription::default(); 5];
        for (index, attribute) in SKINNED_VERTEX_ATTRIBUTES.iter().enumerate() {
            attributes[index] = vk::VertexInputAttributeDescription {
                location: attribute.location,
                binding: 0,
                format: format_from_contract(attribute.format),
                offset: attribute.offset as u32,
            };
        }
        Self {
            binding: vk::VertexInputBindingDescription {
                binding: 0,
                stride: SKINNED_STRIDE_BYTES,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            attributes,
        }
    }

    /// 生成 pipeline 创建用的 vertex input state
    pub fn create_info(&self) -> vk::PipelineVertexInputStateCreateInfo<'_> {
        vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &self.binding,
            vertex_attribute_description_count: self.attributes.len() as u32,
            p_vertex_attribute_descriptions: self.attributes.as_ptr(),
            ..Default::default()
        }
    }
}

impl Default for SkinnedVertexInput {
    fn default() -> Self {
        Self::new()
    }
}

/// 把 animation 契约里的 Vulkan 格式名转成 ash 枚举
///
/// 未知格式直接 panic（契约新增格式时必须同步本函数）。
pub fn format_from_contract(name: &str) -> vk::Format {
    match name {
        "VK_FORMAT_R32G32B32_SFLOAT" => vk::Format::R32G32B32_SFLOAT,
        "VK_FORMAT_R32G32_SFLOAT" => vk::Format::R32G32_SFLOAT,
        "VK_FORMAT_R32G32B32A32_SFLOAT" => vk::Format::R32G32B32A32_SFLOAT,
        "VK_FORMAT_R32G32B32A32_UINT" => vk::Format::R32G32B32A32_UINT,
        other => panic!("未支持的蒙皮顶点格式契约: {other}"),
    }
}

/// 蒙皮顶点着色器 SPIR-V（`skinning.vert.spv`）
pub const SKINNING_VERT_SPV: &[u8] = include_bytes!("../../shaders/skinning.vert.spv");
/// 蒙皮片元着色器 SPIR-V（`skinning.frag.spv`，可选；优先复用 rotation.frag.spv）
pub const SKINNING_FRAG_SPV: &[u8] = include_bytes!("../../shaders/skinning.frag.spv");

/// 取蒙皮顶点着色器 SPIR-V
pub fn skinning_vert_spv() -> &'static [u8] {
    SKINNING_VERT_SPV
}

/// 取蒙皮片元着色器 SPIR-V
pub fn skinning_frag_spv() -> &'static [u8] {
    SKINNING_FRAG_SPV
}

/// 校验 SPIR-V 魔数（0x07230203），供启动自检/测试使用
pub fn is_valid_spirv(bytes: &[u8]) -> bool {
    bytes.len() >= 20 && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == 0x0723_0203
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skinned_stride_matches_animation_contract() {
        assert_eq!(SKINNED_STRIDE_BYTES, 64);
        assert_eq!(SKINNED_VERTEX_STRIDE, 64);
        assert_eq!(SKINNED_STRIDE_BYTES as usize, SKINNED_VERTEX_STRIDE);
    }

    #[test]
    fn skinned_vertex_input_matches_animation_contract() {
        let input = SkinnedVertexInput::new();
        assert_eq!(input.binding.stride, 64);
        assert_eq!(input.binding.input_rate, vk::VertexInputRate::VERTEX);
        assert_eq!(input.attributes.len(), 5);
        assert_eq!(input.attributes.len(), SKINNED_VERTEX_ATTRIBUTES.len());

        // 与 animation crate 的属性表逐项对齐
        for (index, attribute) in SKINNED_VERTEX_ATTRIBUTES.iter().enumerate() {
            assert_eq!(input.attributes[index].location, attribute.location);
            assert_eq!(input.attributes[index].format, format_from_contract(attribute.format));
            assert_eq!(input.attributes[index].offset, attribute.offset as u32);
            assert_eq!(input.attributes[index].binding, 0);
        }
    }

    #[test]
    fn first_32_bytes_match_existing_vertex_layout() {
        // position/normal/uv 与现有 rotation 顶点（stride 32）完全一致
        let input = SkinnedVertexInput::new();
        assert_eq!(input.attributes[0].location, 0);
        assert_eq!(input.attributes[0].offset, 0);
        assert_eq!(input.attributes[0].format, vk::Format::R32G32B32_SFLOAT);
        assert_eq!(input.attributes[1].location, 1);
        assert_eq!(input.attributes[1].offset, 12);
        assert_eq!(input.attributes[1].format, vk::Format::R32G32B32_SFLOAT);
        assert_eq!(input.attributes[2].location, 2);
        assert_eq!(input.attributes[2].offset, 24);
        assert_eq!(input.attributes[2].format, vk::Format::R32G32_SFLOAT);
    }

    #[test]
    fn joint_attributes_use_required_formats_and_offsets() {
        let input = SkinnedVertexInput::new();
        let indices = input.attributes[3];
        assert_eq!(indices.location, JOINT_INDEX_ATTRIBUTE_LOCATION);
        assert_eq!(indices.binding, 0);
        assert_eq!(indices.format, vk::Format::R32G32B32A32_UINT);
        assert_eq!(indices.offset, 32);
        let weights = input.attributes[4];
        assert_eq!(weights.location, JOINT_WEIGHT_ATTRIBUTE_LOCATION);
        assert_eq!(weights.binding, 0);
        assert_eq!(weights.format, vk::Format::R32G32B32A32_SFLOAT);
        assert_eq!(weights.offset, 48);
    }

    #[test]
    fn vertex_input_create_info_wires_arrays() {
        let input = SkinnedVertexInput::new();
        let info = input.create_info();
        assert_eq!(info.vertex_binding_description_count, 1);
        assert_eq!(info.vertex_attribute_description_count, 5);
        assert_eq!(
            info.p_vertex_binding_descriptions as usize,
            &input.binding as *const vk::VertexInputBindingDescription as usize
        );
        assert_eq!(info.p_vertex_attribute_descriptions, input.attributes.as_ptr());
        // 默认字段安全
        assert_eq!(info.p_next, std::ptr::null());
        assert_eq!(info.flags, vk::PipelineVertexInputStateCreateFlags::empty());
    }

    #[test]
    fn joint_palette_binding_uses_set0_binding1() {
        for mode in [JointPaletteMode::StorageBuffer, JointPaletteMode::UniformBuffer] {
            let binding = joint_palette_layout_binding(mode);
            assert_eq!(binding.binding, 1);
            assert_eq!(JOINT_PALETTE_SET, 0);
            assert_eq!(JOINT_PALETTE_BINDING, 1);
            assert_eq!(binding.descriptor_count, 1);
            assert_eq!(binding.stage_flags, vk::ShaderStageFlags::VERTEX);
            assert_eq!(binding.descriptor_type, mode.descriptor_type());
        }
        assert_eq!(
            JointPaletteMode::StorageBuffer.descriptor_type(),
            vk::DescriptorType::STORAGE_BUFFER
        );
        assert_eq!(
            JointPaletteMode::UniformBuffer.descriptor_type(),
            vk::DescriptorType::UNIFORM_BUFFER
        );
    }

    #[test]
    fn joint_palette_capacity_and_bytes() {
        assert_eq!(JOINT_BYTES, 64);
        assert_eq!(joint_palette_bytes(1), 64);
        assert_eq!(joint_palette_bytes(256), 16 * 1024);
        assert!(JointPaletteMode::UniformBuffer.supports_joint_count(256));
        assert!(!JointPaletteMode::UniformBuffer.supports_joint_count(257));
        assert!(JointPaletteMode::StorageBuffer.supports_joint_count(256));
        assert!(!JointPaletteMode::StorageBuffer.supports_joint_count(MAX_JOINTS_PER_PALETTE_STORAGE + 1));
    }

    #[test]
    fn joint_palette_to_le_bytes_matches_vulkan_matrix_size() {
        // animation 侧 palette 序列化结果 = 关节数 * 64 字节，可直接上传 UBO/SSBO
        let palette = hezhou_animation::JointPalette::identity(3);
        assert_eq!(palette.to_le_bytes().len(), joint_palette_bytes(3));
        assert_eq!(palette.to_le_bytes().len(), 3 * JOINT_BYTES as usize);
    }

    #[test]
    fn skinning_spirv_products_are_valid() {
        assert!(is_valid_spirv(SKINNING_VERT_SPV), "skinning.vert.spv 无效");
        assert!(is_valid_spirv(SKINNING_FRAG_SPV), "skinning.frag.spv 无效");
        assert_eq!(skinning_vert_spv(), SKINNING_VERT_SPV);
        assert_eq!(skinning_frag_spv(), SKINNING_FRAG_SPV);
        assert!(!is_valid_spirv(&[]));
        assert!(!is_valid_spirv(&[0u8; 32]));
    }

    #[test]
    fn format_contract_mapping_is_complete() {
        for attribute in SKINNED_VERTEX_ATTRIBUTES {
            // 未知格式会 panic，这里逐个确认都能映射
            let format = format_from_contract(attribute.format);
            assert_ne!(format, vk::Format::UNDEFINED);
        }
        assert_eq!(
            format_from_contract("VK_FORMAT_R32G32B32A32_UINT"),
            vk::Format::R32G32B32A32_UINT
        );
    }
}
