pub mod device;
pub mod context;
pub mod swapchain;
pub mod pipeline;
pub mod buffer;
pub mod command_buffer;
pub mod demo;
pub mod triangle_demo;
pub mod renderer;
pub mod rotation_renderer;
pub mod ui_renderer;
pub mod ui_vulkan_renderer;
pub mod primitive_meshes;
pub mod mesh_loader;

// 0.2 渲染能力（PBR / Shadow Map / 后处理）— 资源规划与 GPU 资源创建
pub mod render_quality;
pub mod shadow_map;
// 实际 PBR/Shadow/Post 绘制路径（接入 UIVulkanRenderer 的 draw path）
pub mod pbr_forward;
// 0.2 骨骼蒙皮 pipeline 接入计划（契约由 hezhou-animation 提供，本轮不创建 pipeline）
pub mod skinning_plan;

#[cfg(feature = "mono")]
pub mod mono_rotation_renderer;

pub use device::VulkanDevice;
pub use demo::VulkanDemo;
pub use triangle_demo::TriangleDemo;
pub use renderer::VulkanRenderer;
pub use rotation_renderer::RotationRenderer;
pub use ui_renderer::VulkanUIRenderer;
pub use ui_vulkan_renderer::UIVulkanRenderer;
pub use command_buffer::VulkanCommandBuffer;
pub use pipeline::VulkanComputePipeline;

// 0.2 渲染能力导出
pub use render_quality::{
    RenderPlan, RenderPassKind, ShadowPassDesc, BloomPlan,
    build_render_plan, pass_order, shadow_map_bytes, shadow_pass_desc,
    quality_summary, settings_with_preset,
    SHADOW_DEPTH_FORMAT, HDR_COLOR_FORMAT, MAX_BLOOM_MIPS,
};
pub use shadow_map::{ShadowMapResources, ShadowMapError};
pub use pbr_forward::{
    PbrForwardPass, SkinnedPipelineState, MeshDraw, LegacyPushBase, SkyPushConstant,
    PbrDrawItem, PbrTextureBinding,
    camera_view_matrix, camera_projection_matrix, format_is_srgb,
    create_render_target, destroy_render_target, create_host_visible_buffer, GpuTarget,
    MAX_SKIN_JOINTS, JOINT_PALETTE_BUFFER_BYTES, MAX_ENTITY_TEXTURE_SETS,
    PBR_PUSH_CONSTANT_BYTES, PBR_FRAME_UBO_BYTES, SHADOW_PUSH_CONSTANT_BYTES,
    TONEMAP_PUSH_CONSTANT_BYTES, BLOOM_PUSH_CONSTANT_BYTES,
    LEGACY_PUSH_CONSTANT_BYTES, SKY_PUSH_CONSTANT_BYTES,
    GIZMO_AXIS_COUNT, GIZMO_VERTICES_PER_AXIS, GIZMO_VERTEX_BUFFER_BYTES,
    LEGACY_FOV_DEGREES, LEGACY_NEAR, LEGACY_FAR,
};
pub use skinning_plan::{
    SkinnedVertexInput, JointPaletteMode,
    joint_palette_layout_binding, joint_palette_bytes, format_from_contract,
    skinning_vert_spv, skinning_frag_spv,
    SKINNED_STRIDE_BYTES, JOINT_BYTES,
};
