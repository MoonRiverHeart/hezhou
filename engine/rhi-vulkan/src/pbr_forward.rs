//! PBR / Shadow / Post 实际绘制路径（0.2 接入 UIVulkanRenderer）
//!
//! 本模块把 task-4 的 CPU 能力真正落到 Vulkan GPU 资源与 draw path:
//! - **HDR 目标** `R16G16B16A16_SFLOAT`（深度复用 renderer 的 D32 depth view）
//! - **PBR forward pipeline**（`pbr.vert/frag`：set0 = FrameUbo + 阴影 array sampler，
//!   set1 = albedo/normal/MR/AO，96 bytes push constant）
//! - **Shadow pass**（`shadow_depth.vert/frag`：1~3 级 cascade、D32 array 深度图、128B push constant）
//! - **Bloom prefilter pass**（`bloom_prefilter.frag`：半分辨率 HDR 第 0 级）
//! - **Tonemap pass**（`post_tonemap.frag`：HDR → renderer 的 LDR offscreen image，
//!   之后交给既有 FXAA → UI，保证 FXAA/UI 链路不变）
//! - **蒙皮 pipeline**（[`SkinnedPipelineState`]：`skinning.vert` + `rotation.frag`，
//!   stride 64 + location 3/4，joint palette set0 binding1，palette 可上传）
//!
//! **Legacy 零回归**：本模块只由 `UIVulkanRenderer` 在
//! `RenderSettings::shading == ShadingModel::Pbr` 时创建/录制；默认设置仍是
//! Blinn-Phong legacy 路径，不会触碰本模块任何资源。
//!
//! 本轮已知限制（详见 `render-pipeline/docs/RENDER_0_2.md`）:
//! - PBR 模式暂不绘制天空盒、选中 outline、3D gizmo（legacy 模式保留）
//! - PBR 材质固定使用 `RenderSettings.default_material` + 中性默认贴图
//!   （per-entity 材质需要 core/asset + C# 接入）
//! - Bloom 只有单级 prefilter + 合成（无多级模糊上采样链）
//! - 蒙皮 pipeline 已创建且 palette 可上传，但 Scene 侧暂无蒙皮 mesh 数据源，尚未进入 draw 循环

use ash::vk;
use bytemuck::cast_slice;
use hezhou_animation::gpu::{SkinnedVertex, SKINNED_VERTEX_STRIDE};
use hezhou_render_pipeline::post::{BloomPrefilterPushConstant, PostTonemapPushConstant};
use hezhou_render_pipeline::pbr::{PbrFrameUbo, PbrLight, PbrMaterial, PbrPushConstant};
use hezhou_render_pipeline::shadow::{camera_basis, ShadowFrameData, ShadowPushConstant};
use hezhou_render_pipeline::{PushConstantData, RenderSettings, ViewportExtent};
use parking_lot::Mutex;
use std::collections::HashMap;

use crate::render_quality::{shadow_pass_desc, HDR_COLOR_FORMAT};
use crate::shadow_map::ShadowMapResources;
use crate::skinning_plan::{joint_palette_bytes, SkinnedVertexInput};

/// 蒙皮 palette 容量（256 关节 × 64B = 16 KiB）
pub const MAX_SKIN_JOINTS: usize = 256;
/// palette buffer 字节数
pub const JOINT_PALETTE_BUFFER_BYTES: usize = joint_palette_bytes(MAX_SKIN_JOINTS);
/// PBR push constant 字节数（= 96）
pub const PBR_PUSH_CONSTANT_BYTES: u32 = std::mem::size_of::<PbrPushConstant>() as u32;
/// PbrFrameUbo 字节数（= 480）
pub const PBR_FRAME_UBO_BYTES: u64 = std::mem::size_of::<PbrFrameUbo>() as u64;
/// Shadow push constant 字节数（= 128）
pub const SHADOW_PUSH_CONSTANT_BYTES: u32 = std::mem::size_of::<ShadowPushConstant>() as u32;
/// Tonemap push constant 字节数（= 16）
pub const TONEMAP_PUSH_CONSTANT_BYTES: u32 = std::mem::size_of::<PostTonemapPushConstant>() as u32;
/// Bloom push constant 字节数（= 16）
pub const BLOOM_PUSH_CONSTANT_BYTES: u32 =
    std::mem::size_of::<BloomPrefilterPushConstant>() as u32;
/// legacy 着色器固定 FOV（与 `shaders/rotation.vert` 一致，切换 PBR 后取景不变）
pub const LEGACY_FOV_DEGREES: f32 = 60.0;
/// legacy 近平面
pub const LEGACY_NEAR: f32 = 0.1;
/// legacy 远平面
pub const LEGACY_FAR: f32 = 100.0;
/// legacy 静态着色 push constant 字节数（`rotation.vert/frag` 布局，144B）
pub const LEGACY_PUSH_CONSTANT_BYTES: u32 = std::mem::size_of::<PushConstantData>() as u32;
/// 天空盒 push constant 字节数（top/bottom 两个 vec4，32B）
pub const SKY_PUSH_CONSTANT_BYTES: u32 = 32;
/// Gizmo 单轴顶点数（build_gizmo_vertices: 2 平面 × 4 三角形 × 3 顶点 = 24）
pub const GIZMO_VERTICES_PER_AXIS: u32 = 24;
/// Gizmo 轴数（X/Y/Z）
pub const GIZMO_AXIS_COUNT: u32 = 3;
/// Gizmo 顶点缓冲字节数（3 轴 × 24 顶点 × 32B）
pub const GIZMO_VERTEX_BUFFER_BYTES: u64 =
    (GIZMO_AXIS_COUNT * GIZMO_VERTICES_PER_AXIS * 32) as u64;
/// Gizmo 三轴颜色（X 红 / Y 绿 / Z 蓝，与 legacy gizmo 一致）
pub const GIZMO_AXIS_COLORS: [[f32; 3]; 3] = [
    [0.95, 0.25, 0.25],
    [0.3, 0.95, 0.4],
    [0.3, 0.55, 1.0],
];

/// 天空盒 push constant（与 `shaders/sky.vert/frag` 一致）
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyPushConstant {
    /// 顶部颜色（w=保留）
    pub top: [f32; 4],
    /// 底部颜色（w=保留）
    pub bottom: [f32; 4],
}

impl SkyPushConstant {
    /// 从场景天空颜色构造
    pub fn new(top: [f32; 3], bottom: [f32; 3]) -> Self {
        Self {
            top: [top[0], top[1], top[2], 1.0],
            bottom: [bottom[0], bottom[1], bottom[2], 1.0],
        }
    }
}

/// outline / gizmo 共用的 legacy 相机 push 数据（`rotation.vert/frag` 144B 布局）
#[derive(Clone, Copy, Debug)]
pub struct LegacyPushBase {
    /// 视口尺寸（像素）
    pub viewport: [f32; 2],
    /// 相机世界坐标
    pub camera_pos: [f32; 3],
    /// 相机 yaw
    pub camera_yaw: f32,
    /// 相机 pitch
    pub camera_pitch: f32,
}

impl LegacyPushBase {
    /// 构造 outline 的 push constant（选中高亮色 `[1.0,0.5,0.0]`，is_selected=1）
    pub fn outline_push(&self, model: [[f32; 4]; 4]) -> PushConstantData {
        self.build(model, [1.0, 0.5, 0.0], 1.0, 0.0)
    }

    /// 构造 gizmo 轴线的 push constant（选中色作为轴线颜色，无纹理/无高光）
    pub fn gizmo_push(&self, model: [[f32; 4]; 4], color: [f32; 3]) -> PushConstantData {
        self.build(model, color, 1.0, 0.0)
    }

    /// 构造普通 mesh 的 push constant（is_selected=0，非 outline；蒙皮合成 mesh 用）
    pub fn mesh_push(&self, model: [[f32; 4]; 4], has_texture: f32) -> PushConstantData {
        self.build(model, [0.0, 0.0, 0.0], 0.0, has_texture)
    }

    fn build(
        &self,
        model: [[f32; 4]; 4],
        outline_color: [f32; 3],
        is_selected: f32,
        has_texture: f32,
    ) -> PushConstantData {
        PushConstantData {
            model,
            outline_color,
            is_selected,
            viewport_size: self.viewport,
            _pad: [0.0, 0.0],
            camera_pos: self.camera_pos,
            camera_yaw: self.camera_yaw,
            camera_pitch: self.camera_pitch,
            _pad2: 0.0,
            has_texture,
            specular_strength: 0.3,
            ambient_strength: 0.15,
            shininess: 32.0,
            _end_pad: [0.0, 0.0],
        }
    }
}


/// 单个 mesh 的绘制描述（与 mesh 数据来源解耦）
#[derive(Clone, Copy, Debug, Default)]
pub struct MeshDraw {
    /// 顶点缓冲偏移（vertex 单位）
    pub vertex_offset: u32,
    /// 顶点数
    pub vertex_count: u32,
    /// 索引缓冲偏移（index 单位）
    pub index_offset: u32,
    /// 索引数（0 = 非索引绘制）
    pub index_count: u32,
    /// 模型矩阵（column-major，与 GPU 约定一致）
    pub model: [[f32; 4]; 4],
}

impl MeshDraw {
    /// 非索引绘制（primitive mesh）
    pub fn primitive(vertex_offset: u32, vertex_count: u32, model: [[f32; 4]; 4]) -> Self {
        Self {
            vertex_offset,
            vertex_count,
            index_offset: 0,
            index_count: 0,
            model,
        }
    }

    /// 索引绘制（自定义 OBJ mesh）
    pub fn indexed(
        vertex_offset: u32,
        vertex_count: u32,
        index_offset: u32,
        index_count: u32,
        model: [[f32; 4]; 4],
    ) -> Self {
        Self {
            vertex_offset,
            vertex_count,
            index_offset,
            index_count,
            model,
        }
    }

    /// 是否使用索引缓冲
    pub fn is_indexed(&self) -> bool {
        self.index_count > 0
    }
}

/// 每实体纹理绑定（PBR set1 的 albedo/normal/MR/AO 贴图槽）
///
/// `cache_key` 由调用方给出（通常为 image view 的 raw handle），用于在
/// [`PbrForwardPass`] 内缓存 PBR 兼容的 descriptor set，避免每帧重复分配。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PbrTextureBinding {
    /// 缓存 key（同一纹理多次绘制复用 descriptor set）
    pub cache_key: u64,
    /// 贴图 image view
    pub view: vk::ImageView,
    /// 贴图 sampler
    pub sampler: vk::Sampler,
}

/// PBR 每实体绘制项：mesh + 该实体材质 + 可选纹理
#[derive(Clone, Copy, Debug)]
pub struct PbrDrawItem {
    /// mesh 绘制描述（模型矩阵/顶点范围）
    pub mesh: MeshDraw,
    /// 该实体材质（来自 `PbrMaterialOverride` 或默认材质）
    pub material: PbrMaterial,
    /// 可选的每实体纹理（None = 默认中性贴图）
    pub texture: Option<PbrTextureBinding>,
}

impl PbrDrawItem {
    /// 只用材质（默认中性贴图）
    pub fn new(mesh: MeshDraw, material: PbrMaterial) -> Self {
        Self {
            mesh,
            material,
            texture: None,
        }
    }

    /// 带每实体纹理
    pub fn with_texture(mesh: MeshDraw, material: PbrMaterial, texture: PbrTextureBinding) -> Self {
        Self {
            mesh,
            material,
            texture: Some(texture),
        }
    }

    /// 该绘制项对应的 push constant（用于 CPU 契约测试/调试）
    pub fn push_constant(&self) -> PbrPushConstant {
        PbrPushConstant::from_material(self.mesh.model, &self.material)
    }
}

/// 每实体纹理 descriptor set 上限（超出回退默认贴图，避免 pool 爆炸）
pub const MAX_ENTITY_TEXTURE_SETS: usize = 256;
/// 同时可绘制的真实蒙皮实体上限（每实体一个 palette descriptor set）
pub const MAX_SKINNED_ENTITIES: usize = 256;

/// 自有的离屏目标（image + memory + view）
pub struct GpuTarget {
    /// image handle
    pub image: vk::Image,
    /// memory handle
    pub memory: vk::DeviceMemory,
    /// image view
    pub view: vk::ImageView,
}

struct HdrTarget {
    target: GpuTarget,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
}

struct BloomTarget {
    target: GpuTarget,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    extent: vk::Extent2D,
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    set_layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
}

struct ShadowTarget {
    map: ShadowMapResources,
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

/// PBR/Shadow/Post 实际绘制通道
pub struct PbrForwardPass {
    device: ash::Device,
    /// 视口尺寸
    pub extent: ViewportExtent,
    settings: RenderSettings,
    ldr_format: vk::Format,
    depth_view: vk::ImageView,
    ldr_view: vk::ImageView,

    hdr: HdrTarget,
    bloom: Option<BloomTarget>,
    shadow: Option<ShadowTarget>,

    pbr_pipeline: vk::Pipeline,
    pbr_layout: vk::PipelineLayout,
    pbr_set0_layout: vk::DescriptorSetLayout,
    pbr_set1_layout: vk::DescriptorSetLayout,
    pbr_set0_pool: vk::DescriptorPool,
    pbr_set0: vk::DescriptorSet,
    pbr_set1_pool: vk::DescriptorPool,
    pbr_set1: vk::DescriptorSet,
    pbr_frame_ubo: vk::Buffer,
    pbr_frame_ubo_memory: vk::DeviceMemory,

    tonemap_pipeline: vk::Pipeline,
    tonemap_layout: vk::PipelineLayout,
    tonemap_set0_layout: vk::DescriptorSetLayout,
    tonemap_pool: vk::DescriptorPool,
    tonemap_set: vk::DescriptorSet,
    tonemap_render_pass: vk::RenderPass,
    tonemap_framebuffer: vk::Framebuffer,

    dummy_shadow: Option<GpuTarget>,
    dummy_shadow_sampler: Option<vk::Sampler>,

    material_sampler: vk::Sampler,
    material_textures: Vec<GpuTarget>,
    hdr_sampler: vk::Sampler,
    // 每实体纹理 descriptor 缓存（PBR set1：4 个 sampler）
    entity_texture_pool: vk::DescriptorPool,
    entity_texture_cache: Mutex<HashMap<u64, vk::DescriptorSet>>,

    // legacy 视觉补齐：天空盒 / 选中 outline / 3D gizmo
    sky_pipeline: vk::Pipeline,
    sky_layout: vk::PipelineLayout,
    outline_pipeline: vk::Pipeline,
    /// outline/gizmo 用的 legacy 兼容 pipeline layout（set0=light UBO，set1=材质贴图，144B push）
    legacy_draw_layout: vk::PipelineLayout,
    legacy_set0_layout: vk::DescriptorSetLayout,
    legacy_set1_layout: vk::DescriptorSetLayout,
    legacy_pool: vk::DescriptorPool,
    /// 内置 fallback sets（renderer 可传入自己的兼容 sets 覆盖）
    legacy_light_set: vk::DescriptorSet,
    legacy_texture_set: vk::DescriptorSet,
    legacy_light_ubo: vk::Buffer,
    legacy_light_ubo_memory: vk::DeviceMemory,
}

impl PbrForwardPass {
    /// 创建 PBR/Shadow/Post 全部资源与 pipeline
    ///
    /// # Safety
    /// - `device` 必须由 `instance`/`physical_device` 创建且合法
    /// - `command_pool`/`queue` 用于一次性纹理上传与布局转换（内部 `queue_wait_idle`）
    /// - `depth_view`/`ldr_view` 由 renderer 持有，本模块只引用、不销毁
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        extent: ViewportExtent,
        depth_view: vk::ImageView,
        ldr_view: vk::ImageView,
        ldr_format: vk::Format,
        settings: &RenderSettings,
    ) -> Result<Self, String> {
        if extent.width == 0 || extent.height == 0 {
            return Err("PbrForwardPass: 视口尺寸为 0".to_string());
        }
        let device = device.clone();
        let settings = settings.clone().validate();
        let memory_properties = instance.get_physical_device_memory_properties(physical_device);

        // === 1. HDR 主目标 ===
        let hdr_target = create_gpu_target(
            &device,
            &memory_properties,
            extent,
            HDR_COLOR_FORMAT,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
        )?;
        let hdr_render_pass = create_hdr_render_pass(&device)?;
        let hdr_framebuffer =
            create_framebuffer(&device, hdr_render_pass, &[hdr_target.view, depth_view], extent)?;

        // === 2. Tonemap 渲染目标 = renderer 的 LDR offscreen image ===
        let tonemap_render_pass = create_ldr_render_pass(&device, ldr_format)?;
        let tonemap_framebuffer =
            create_framebuffer(&device, tonemap_render_pass, &[ldr_view], extent)?;

        // === 3. 采样器 ===
        let hdr_sampler = create_sampler(&device)?;
        let material_sampler = create_sampler(&device)?;

        // === 4. Frame UBO ===
        let (pbr_frame_ubo, pbr_frame_ubo_memory) = create_host_buffer(
            &device,
            &memory_properties,
            PBR_FRAME_UBO_BYTES,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;

        // === 5. Shadow 资源（按质量档） ===
        let mut shadow = None;
        if shadow_pass_desc(&settings).is_some() {
            let map = ShadowMapResources::create_from_settings(
                instance,
                &device,
                physical_device,
                &settings,
            )
            .ok_or_else(|| "PbrForwardPass: 阴影配置缺失".to_string())?
            .map_err(|e| format!("PbrForwardPass: 创建阴影图失败: {e}"))?;
            let shadow_layout = create_pipeline_layout(
                &device,
                SHADOW_PUSH_CONSTANT_BYTES,
                vk::ShaderStageFlags::VERTEX,
                &[],
            )?;
            let shadow_pipeline = create_pipeline(
                &device,
                map.render_pass,
                shadow_layout,
                PipelineShaders {
                    vert: include_bytes!("../../shaders/shadow_depth.vert.spv"),
                    frag: include_bytes!("../../shaders/shadow_depth.frag.spv"),
                },
                &PipelineOptions {
                    vertex_stride: 32,
                    vertex_attributes: &[
                        (0, vk::Format::R32G32B32_SFLOAT, 0),
                        (1, vk::Format::R32G32B32_SFLOAT, 12),
                        (2, vk::Format::R32G32_SFLOAT, 24),
                    ],
                    depth_test: true,
                    depth_write: true,
                    depth_compare: vk::CompareOp::LESS_OR_EQUAL,
                    color_attachment: false,
                    alpha_blend: false,
                },
            )?;
            shadow = Some(ShadowTarget {
                map,
                pipeline: shadow_pipeline,
                layout: shadow_layout,
            });
        }

        // === 6. 无阴影占位深度图（保证 set0 binding1 合法） ===
        let (dummy_shadow, dummy_shadow_sampler) = if shadow.is_some() {
            (None, None)
        } else {
            let target = create_dummy_shadow_target(&device, &memory_properties, command_pool, queue)?;
            let sampler = create_shadow_compare_sampler(&device)?;
            (Some(target), Some(sampler))
        };

        // === 7. 默认材质贴图（albedo 白 / normal 中性 / MR 中性 / AO 白） ===
        let mut material_textures = Vec::new();
        for rgba in [
            [255u8, 255, 255, 255],
            [128u8, 128, 255, 255],
            [0u8, 255, 255, 255],
            [255u8, 255, 255, 255],
        ] {
            material_textures.push(upload_texture(
                &device,
                &memory_properties,
                command_pool,
                queue,
                &rgba,
            )?);
        }

        // === 8. PBR descriptor set 布局/池/集 ===
        let pbr_set0_layout = create_descriptor_set_layout(
            &device,
            &[
                binding(
                    0,
                    vk::DescriptorType::UNIFORM_BUFFER,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
                binding(
                    1,
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
        )?;
        let pbr_set1_layout = create_descriptor_set_layout(
            &device,
            &[
                binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
                binding(1, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
                binding(2, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
                binding(3, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
            ],
        )?;
        let pbr_set0_pool = create_descriptor_pool(
            &device,
            1,
            &[
                (vk::DescriptorType::UNIFORM_BUFFER, 1),
                (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1),
            ],
        )?;
        let pbr_set0 = allocate_descriptor_set(&device, pbr_set0_pool, pbr_set0_layout)?;
        let pbr_set1_pool = create_descriptor_pool(
            &device,
            1,
            &[(vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 4)],
        )?;
        let pbr_set1 = allocate_descriptor_set(&device, pbr_set1_pool, pbr_set1_layout)?;
        // 每实体纹理 descriptor 池（复用 pbr_set1_layout：4 个 combined sampler）
        let entity_texture_pool = create_descriptor_pool(
            &device,
            MAX_ENTITY_TEXTURE_SETS as u32,
            &[(
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                (MAX_ENTITY_TEXTURE_SETS * 4) as u32,
            )],
        )?;

        let (shadow_image_view, shadow_sampler) = match &shadow {
            Some(shadow) => (shadow.map.array_view, shadow.map.sampler),
            None => (
                dummy_shadow.as_ref().expect("dummy shadow 已创建").view,
                dummy_shadow_sampler.expect("dummy shadow sampler 已创建"),
            ),
        };
        let frame_ubo_info = vk::DescriptorBufferInfo {
            buffer: pbr_frame_ubo,
            offset: 0,
            range: PBR_FRAME_UBO_BYTES,
        };
        let shadow_image_info = vk::DescriptorImageInfo {
            sampler: shadow_sampler,
            image_view: shadow_image_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet {
                    dst_set: pbr_set0,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &frame_ubo_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: pbr_set0,
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &shadow_image_info,
                    ..Default::default()
                },
            ],
            &[],
        );

        let material_infos: Vec<vk::DescriptorImageInfo> = material_textures
            .iter()
            .map(|texture| vk::DescriptorImageInfo {
                sampler: material_sampler,
                image_view: texture.view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            })
            .collect();
        let material_writes: Vec<vk::WriteDescriptorSet> = material_infos
            .iter()
            .enumerate()
            .map(|(index, info)| vk::WriteDescriptorSet {
                dst_set: pbr_set1,
                dst_binding: index as u32,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: info,
                ..Default::default()
            })
            .collect();
        device.update_descriptor_sets(&material_writes, &[]);

        // === 9. PBR pipeline ===
        let pbr_layout = create_pipeline_layout(
            &device,
            PBR_PUSH_CONSTANT_BYTES,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            &[pbr_set0_layout, pbr_set1_layout],
        )?;
        let pbr_pipeline = create_pipeline(
            &device,
            hdr_render_pass,
            pbr_layout,
            PipelineShaders {
                vert: include_bytes!("../../shaders/pbr.vert.spv"),
                frag: include_bytes!("../../shaders/pbr.frag.spv"),
            },
            &PipelineOptions {
                vertex_stride: 32,
                vertex_attributes: &[
                    (0, vk::Format::R32G32B32_SFLOAT, 0),
                    (1, vk::Format::R32G32B32_SFLOAT, 12),
                    (2, vk::Format::R32G32_SFLOAT, 24),
                ],
                depth_test: true,
                depth_write: true,
                depth_compare: vk::CompareOp::LESS,
                color_attachment: true,
                alpha_blend: false,
            },
        )?;

        // === 9.1 天空盒 pipeline（复用 sky.vert/frag，HDR 目标，全屏三角形） ===
        let sky_layout = create_pipeline_layout(
            &device,
            SKY_PUSH_CONSTANT_BYTES,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            &[],
        )?;
        let sky_pipeline = create_pipeline(
            &device,
            hdr_render_pass,
            sky_layout,
            PipelineShaders {
                vert: include_bytes!("../../shaders/sky.vert.spv"),
                frag: include_bytes!("../../shaders/sky.frag.spv"),
            },
            &PipelineOptions {
                vertex_stride: 0,
                vertex_attributes: &[],
                depth_test: false,
                depth_write: false,
                depth_compare: vk::CompareOp::ALWAYS,
                color_attachment: true,
                alpha_blend: false,
            },
        )?;

        // === 9.2 选中 outline / Gizmo pipeline ===
        // legacy 兼容 layout：set0 = light UBO(binding0, FRAGMENT)，
        // set1 = 材质贴图(binding0, FRAGMENT)，push constant = 144B。
        // renderer 自己的 light/texture descriptor sets 与本 layout 兼容，可直接复用；
        // 独立使用（GPU 测试）时使用模块内置的 fallback sets。
        let legacy_set0_layout = create_descriptor_set_layout(
            &device,
            &[binding(
                0,
                vk::DescriptorType::UNIFORM_BUFFER,
                vk::ShaderStageFlags::FRAGMENT,
            )],
        )?;
        let legacy_set1_layout = create_descriptor_set_layout(
            &device,
            &[binding(
                0,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                vk::ShaderStageFlags::FRAGMENT,
            )],
        )?;
        let legacy_pool = create_descriptor_pool(
            &device,
            2,
            &[
                (vk::DescriptorType::UNIFORM_BUFFER, 1),
                (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1),
            ],
        )?;
        let legacy_light_set =
            allocate_descriptor_set(&device, legacy_pool, legacy_set0_layout)?;
        let legacy_texture_set =
            allocate_descriptor_set(&device, legacy_pool, legacy_set1_layout)?;
        let (legacy_light_ubo, legacy_light_ubo_memory) = create_host_buffer(
            &device,
            &memory_properties,
            32,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        // 默认光数据（std430：direction(12)+pad(4)+color(12)+intensity(4) = 32B）
        let default_legacy_light: [f32; 8] = [-0.5, -1.0, -0.5, 0.0, 1.0, 1.0, 1.0, 1.0];
        let legacy_light_ptr = device
            .map_memory(legacy_light_ubo_memory, 0, 32, vk::MemoryMapFlags::empty())
            .map_err(|e| format!("PbrForwardPass: map 内置 light UBO 失败: {e}"))?;
        std::ptr::copy_nonoverlapping(
            default_legacy_light.as_ptr() as *const u8,
            legacy_light_ptr as *mut u8,
            32,
        );
        device.unmap_memory(legacy_light_ubo_memory);
        let legacy_light_info = vk::DescriptorBufferInfo {
            buffer: legacy_light_ubo,
            offset: 0,
            range: 32,
        };
        let legacy_white_info = vk::DescriptorImageInfo {
            sampler: material_sampler,
            image_view: material_textures[0].view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet {
                    dst_set: legacy_light_set,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &legacy_light_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: legacy_texture_set,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &legacy_white_info,
                    ..Default::default()
                },
            ],
            &[],
        );
        let legacy_draw_layout = create_pipeline_layout(
            &device,
            LEGACY_PUSH_CONSTANT_BYTES,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            &[legacy_set0_layout, legacy_set1_layout],
        )?;

        let outline_pipeline = create_pipeline(
            &device,
            hdr_render_pass,
            legacy_draw_layout,
            PipelineShaders {
                vert: include_bytes!("../../shaders/rotation.vert.spv"),
                frag: include_bytes!("../../shaders/rotation.frag.spv"),
            },
            &PipelineOptions {
                vertex_stride: 32,
                vertex_attributes: &[
                    (0, vk::Format::R32G32B32_SFLOAT, 0),
                    (1, vk::Format::R32G32B32_SFLOAT, 12),
                    (2, vk::Format::R32G32_SFLOAT, 24),
                ],
                depth_test: true,
                depth_write: false,
                depth_compare: vk::CompareOp::LESS_OR_EQUAL,
                color_attachment: true,
                alpha_blend: true,
            },
        )?;

        // === 10. Bloom 资源（按质量档） ===
        let mut bloom = None;
        if settings.effective_bloom() {
            let bloom_extent = vk::Extent2D {
                width: (extent.width / 2).max(1),
                height: (extent.height / 2).max(1),
            };
            let target = create_gpu_target(
                &device,
                &memory_properties,
                ViewportExtent::new(bloom_extent.width, bloom_extent.height),
                HDR_COLOR_FORMAT,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                vk::ImageAspectFlags::COLOR,
            )?;
            let render_pass = create_bloom_render_pass(&device)?;
            let framebuffer = create_framebuffer(
                &device,
                render_pass,
                &[target.view],
                ViewportExtent::new(bloom_extent.width, bloom_extent.height),
            )?;
            let set_layout = create_descriptor_set_layout(
                &device,
                &[binding(
                    0,
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    vk::ShaderStageFlags::FRAGMENT,
                )],
            )?;
            let pool = create_descriptor_pool(
                &device,
                1,
                &[(vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1)],
            )?;
            let set = allocate_descriptor_set(&device, pool, set_layout)?;
            let src_info = vk::DescriptorImageInfo {
                sampler: hdr_sampler,
                image_view: hdr_target.view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            };
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: set,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &src_info,
                    ..Default::default()
                }],
                &[],
            );
            let layout = create_pipeline_layout(
                &device,
                BLOOM_PUSH_CONSTANT_BYTES,
                vk::ShaderStageFlags::FRAGMENT,
                &[set_layout],
            )?;
            let pipeline = create_pipeline(
                &device,
                render_pass,
                layout,
                PipelineShaders {
                    vert: include_bytes!("../../shaders/post.vert.spv"),
                    frag: include_bytes!("../../shaders/bloom_prefilter.frag.spv"),
                },
                &PipelineOptions {
                    vertex_stride: 0,
                    vertex_attributes: &[],
                    depth_test: false,
                    depth_write: false,
                    depth_compare: vk::CompareOp::ALWAYS,
                    color_attachment: true,
                    alpha_blend: false,
                },
            )?;
            bloom = Some(BloomTarget {
                target,
                render_pass,
                framebuffer,
                extent: bloom_extent,
                pipeline,
                layout,
                set_layout,
                pool,
                descriptor_set: set,
            });
        }

        // === 11. Tonemap descriptor + pipeline ===
        let tonemap_set0_layout = create_descriptor_set_layout(
            &device,
            &[
                binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
                binding(1, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT),
            ],
        )?;
        let tonemap_pool = create_descriptor_pool(
            &device,
            1,
            &[(vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 2)],
        )?;
        let tonemap_set = allocate_descriptor_set(&device, tonemap_pool, tonemap_set0_layout)?;
        // 无 Bloom 时 binding1 绑定 HDR 自身（intensity 传 0，结果无影响）
        let bloom_image_view = match &bloom {
            Some(bloom) => bloom.target.view,
            None => hdr_target.view,
        };
        let hdr_info = vk::DescriptorImageInfo {
            sampler: hdr_sampler,
            image_view: hdr_target.view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        let bloom_info = vk::DescriptorImageInfo {
            sampler: hdr_sampler,
            image_view: bloom_image_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet {
                    dst_set: tonemap_set,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &hdr_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: tonemap_set,
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &bloom_info,
                    ..Default::default()
                },
            ],
            &[],
        );
        let tonemap_layout = create_pipeline_layout(
            &device,
            TONEMAP_PUSH_CONSTANT_BYTES,
            vk::ShaderStageFlags::FRAGMENT,
            &[tonemap_set0_layout],
        )?;
        let tonemap_pipeline = create_pipeline(
            &device,
            tonemap_render_pass,
            tonemap_layout,
            PipelineShaders {
                vert: include_bytes!("../../shaders/post.vert.spv"),
                frag: include_bytes!("../../shaders/post_tonemap.frag.spv"),
            },
            &PipelineOptions {
                vertex_stride: 0,
                vertex_attributes: &[],
                depth_test: false,
                depth_write: false,
                depth_compare: vk::CompareOp::ALWAYS,
                color_attachment: true,
                alpha_blend: false,
            },
        )?;

        Ok(Self {
            device,
            extent,
            settings,
            ldr_format,
            depth_view,
            ldr_view,
            hdr: HdrTarget {
                target: hdr_target,
                render_pass: hdr_render_pass,
                framebuffer: hdr_framebuffer,
            },
            bloom,
            shadow,
            pbr_pipeline,
            pbr_layout,
            pbr_set0_layout,
            pbr_set1_layout,
            pbr_set0_pool,
            pbr_set0,
            pbr_set1_pool,
            pbr_set1,
            pbr_frame_ubo,
            pbr_frame_ubo_memory,
            tonemap_pipeline,
            tonemap_layout,
            tonemap_set0_layout,
            tonemap_pool,
            tonemap_set,
            tonemap_render_pass,
            tonemap_framebuffer,
            dummy_shadow,
            dummy_shadow_sampler,
            material_sampler,
            material_textures,
            hdr_sampler,
            entity_texture_pool,
            entity_texture_cache: Mutex::new(HashMap::new()),
            sky_pipeline,
            sky_layout,
            outline_pipeline,
            legacy_draw_layout,
            legacy_set0_layout,
            legacy_set1_layout,
            legacy_pool,
            legacy_light_set,
            legacy_texture_set,
            legacy_light_ubo,
            legacy_light_ubo_memory,
        })
    }

    /// 创建时的设置快照
    pub fn settings(&self) -> &RenderSettings {
        &self.settings
    }

    /// HDR 主 render pass（蒙皮 pipeline 需要与它兼容）
    pub fn hdr_render_pass(&self) -> vk::RenderPass {
        self.hdr.render_pass
    }

    /// 是否启用阴影 pass
    pub fn shadow_active(&self) -> bool {
        self.shadow.is_some()
    }

    /// 是否启用 Bloom pass
    pub fn bloom_active(&self) -> bool {
        self.bloom.is_some()
    }

    /// LDR 输出格式是否 SRGB（若是，tonemap 的 gamma 由硬件完成）
    pub fn ldr_is_srgb(&self) -> bool {
        format_is_srgb(self.ldr_format)
    }

    /// 天空盒 pipeline 是否已创建
    pub fn sky_pipeline_ready(&self) -> bool {
        self.sky_pipeline != vk::Pipeline::null()
    }

    /// 选中 outline / Gizmo pipeline 是否已创建
    pub fn outline_pipeline_ready(&self) -> bool {
        self.outline_pipeline != vk::Pipeline::null()
    }

    /// renderer 的 legacy pipeline layout（outline/gizmo descriptor sets 绑定用）
    pub fn legacy_draw_layout(&self) -> vk::PipelineLayout {
        self.legacy_draw_layout
    }

    /// 上传 PbrFrameUbo（含阴影级联数据）
    ///
    /// # Safety
    /// 需要 UBO 内存可映射（本模块创建时使用 HOST_VISIBLE|HOST_COHERENT）
    pub unsafe fn update_frame_ubo(
        &self,
        light: &PbrLight,
        view: [[f32; 4]; 4],
        proj: [[f32; 4]; 4],
        camera_pos: [f32; 3],
        shadow: Option<&ShadowFrameData>,
    ) -> Result<PbrFrameUbo, String> {
        let mut ubo = PbrFrameUbo::from_light(light, view, proj, camera_pos);
        if let (Some(frame), Some(config)) = (shadow, self.settings.shadow_config()) {
            frame.write_into_ubo(&config, &mut ubo);
        }
        let ptr = self
            .device
            .map_memory(
                self.pbr_frame_ubo_memory,
                0,
                PBR_FRAME_UBO_BYTES,
                vk::MemoryMapFlags::empty(),
            )
            .map_err(|e| format!("PbrForwardPass: map frame UBO 失败: {e}"))?;
        std::ptr::copy_nonoverlapping(
            &ubo as *const PbrFrameUbo as *const u8,
            ptr as *mut u8,
            PBR_FRAME_UBO_BYTES as usize,
        );
        self.device.unmap_memory(self.pbr_frame_ubo_memory);
        Ok(ubo)
    }

    /// 录制阴影深度 pass（每个 cascade 一遍）
    ///
    /// # Safety
    /// `cmd` 必须处于录制状态；vertex/index buffer 必须已包含对应 mesh 数据
    pub unsafe fn record_shadow(
        &self,
        cmd: vk::CommandBuffer,
        frame: &ShadowFrameData,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        draws: &[MeshDraw],
    ) {
        let Some(shadow) = &self.shadow else {
            return;
        };
        let extent = shadow.map.extent;
        for cascade in 0..frame.cascade_count {
            let Some(framebuffer) = shadow.map.framebuffer(cascade) else {
                continue;
            };
            let clear_values = [vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            }];
            self.begin_render_pass(
                cmd,
                shadow.map.render_pass,
                framebuffer,
                extent,
                &clear_values,
            );
            self.set_viewport(cmd, extent);
            self.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                shadow.pipeline,
            );
            self.device
                .cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
            for draw in draws {
                let push = ShadowPushConstant {
                    light_view_proj: frame.light_view_proj[cascade as usize],
                    model: draw.model,
                };
                self.device.cmd_push_constants(
                    cmd,
                    shadow.layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    cast_slice(&[push]),
                );
                self.draw_mesh(cmd, index_buffer, draw);
            }
            self.device.cmd_end_render_pass(cmd);
        }
    }

    /// 录制 PBR 主 pass（HDR 目标）— 兼容入口：begin + 实体 + end
    ///
    /// # Safety
    /// 同 [`PbrForwardPass::record_shadow`]
    pub unsafe fn record_scene(
        &self,
        cmd: vk::CommandBuffer,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        draws: &[MeshDraw],
        material: &PbrMaterial,
    ) {
        self.begin_scene(cmd, [0.05, 0.05, 0.1, 1.0]);
        self.record_scene_draws(cmd, vertex_buffer, index_buffer, draws, material);
        self.end_scene(cmd);
    }

    /// 开始 PBR HDR render pass
    ///
    /// `clear_color` 用于无天空盒时区分质量档底色（有天空盒时会被全屏天空覆盖）。
    ///
    /// # Safety
    /// 同 [`PbrForwardPass::record_shadow`]
    pub unsafe fn begin_scene(&self, cmd: vk::CommandBuffer, clear_color: [f32; 4]) {
        let extent = vk::Extent2D {
            width: self.extent.width,
            height: self.extent.height,
        };
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: clear_color,
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        self.begin_render_pass(
            cmd,
            self.hdr.render_pass,
            self.hdr.framebuffer,
            extent,
            &clear_values,
        );
        self.set_viewport(cmd, extent);
    }

    /// 绘制天空盒（全屏三角形；应在实体之前、`begin_scene` 之后调用）
    ///
    /// # Safety
    /// 必须已调用 [`PbrForwardPass::begin_scene`]
    pub unsafe fn record_sky(&self, cmd: vk::CommandBuffer, top: [f32; 3], bottom: [f32; 3]) {
        self.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.sky_pipeline);
        let push = SkyPushConstant::new(top, bottom);
        self.device.cmd_push_constants(
            cmd,
            self.sky_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            cast_slice(&[push]),
        );
        self.device.cmd_draw(cmd, 3, 1, 0, 0);
    }

    /// 绘制 PBR 实体（bind pipeline/sets/vertex + per-draw push constant）
    ///
    /// # Safety
    /// 必须处于 HDR render pass 内
    pub unsafe fn record_scene_draws(
        &self,
        cmd: vk::CommandBuffer,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        draws: &[MeshDraw],
        material: &PbrMaterial,
    ) {
        self.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pbr_pipeline);
        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.pbr_layout,
            0,
            &[self.pbr_set0, self.pbr_set1],
            &[],
        );
        self.device
            .cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
        for draw in draws {
            let push = PbrPushConstant::from_material(draw.model, material);
            self.device.cmd_push_constants(
                cmd,
                self.pbr_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                cast_slice(&[push]),
            );
            self.draw_mesh(cmd, index_buffer, draw);
        }
    }

    /// 获取/创建每实体纹理的 PBR set1 descriptor set（内部缓存）
    ///
    /// set1 4 个 binding 分别为 albedo(每实体) / normal / metallic-roughness / AO，
    /// 后三者使用模块内置中性贴图。超过 [`MAX_ENTITY_TEXTURE_SETS`] 时返回 None，
    /// 调用方回退默认贴图 set。
    unsafe fn entity_texture_set(&self, binding: &PbrTextureBinding) -> Option<vk::DescriptorSet> {
        if let Some(set) = self.entity_texture_cache.lock().get(&binding.cache_key) {
            return Some(*set);
        }
        let mut cache = self.entity_texture_cache.lock();
        // 双重检查（并发 &self 录制场景）
        if let Some(set) = cache.get(&binding.cache_key) {
            return Some(*set);
        }
        if cache.len() >= MAX_ENTITY_TEXTURE_SETS {
            return None;
        }
        let set =
            allocate_descriptor_set(&self.device, self.entity_texture_pool, self.pbr_set1_layout)
                .ok()?;
        let infos = [
            vk::DescriptorImageInfo {
                sampler: binding.sampler,
                image_view: binding.view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            vk::DescriptorImageInfo {
                sampler: self.material_sampler,
                image_view: self.material_textures[1].view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            vk::DescriptorImageInfo {
                sampler: self.material_sampler,
                image_view: self.material_textures[2].view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            vk::DescriptorImageInfo {
                sampler: self.material_sampler,
                image_view: self.material_textures[3].view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        ];
        let writes: Vec<vk::WriteDescriptorSet> = infos
            .iter()
            .enumerate()
            .map(|(index, info)| vk::WriteDescriptorSet {
                dst_set: set,
                dst_binding: index as u32,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: info,
                ..Default::default()
            })
            .collect();
        self.device.update_descriptor_sets(&writes, &[]);
        cache.insert(binding.cache_key, set);
        Some(set)
    }

    /// 已缓存的每实体纹理 descriptor set 数量（调试/测试）
    pub fn entity_texture_set_count(&self) -> usize {
        self.entity_texture_cache.lock().len()
    }

    /// 录制 PBR 实体（每实体材质 + 可选每实体纹理）
    ///
    /// 每个 `PbrDrawItem` 独立写 PushConstant（base_color + metallic/roughness/
    /// normal_scale/ao）；带纹理的 item 绑定其专属 set1 descriptor，否则用默认中性贴图。
    ///
    /// # Safety
    /// 必须处于 HDR render pass 内；vertex/index buffer 必须已包含对应 mesh 数据
    pub unsafe fn record_scene_draws_with_materials(
        &self,
        cmd: vk::CommandBuffer,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        items: &[PbrDrawItem],
    ) {
        self.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pbr_pipeline);
        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.pbr_layout,
            0,
            &[self.pbr_set0, self.pbr_set1],
            &[],
        );
        self.device
            .cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
        let mut current_set1 = self.pbr_set1;
        for item in items {
            let push = item.push_constant();
            self.device.cmd_push_constants(
                cmd,
                self.pbr_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                cast_slice(&[push]),
            );
            let set1 = match item.texture {
                Some(texture) => self.entity_texture_set(&texture).unwrap_or(self.pbr_set1),
                None => self.pbr_set1,
            };
            if set1 != current_set1 {
                self.device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pbr_layout,
                    0,
                    &[self.pbr_set0, set1],
                    &[],
                );
                current_set1 = set1;
            }
            self.draw_mesh(cmd, index_buffer, &item.mesh);
        }
    }

    /// 绘制选中 outline（复用 legacy `rotation.vert/frag` pipeline + 144B push constant）
    ///
    /// `light_set` / `texture_set` 为 renderer 的 legacy descriptor sets
    /// （与 `legacy_draw_layout` 兼容）。
    ///
    /// # Safety
    /// 必须处于 HDR render pass 内；descriptor sets 必须与 `legacy_draw_layout` 兼容
    pub unsafe fn record_outline(
        &self,
        cmd: vk::CommandBuffer,
        light_set: Option<vk::DescriptorSet>,
        texture_set: Option<vk::DescriptorSet>,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        outlines: &[MeshDraw],
        base: &LegacyPushBase,
    ) {
        if outlines.is_empty() {
            return;
        }
        self.device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.outline_pipeline,
        );
        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.legacy_draw_layout,
            0,
            &[
                light_set.unwrap_or(self.legacy_light_set),
                texture_set.unwrap_or(self.legacy_texture_set),
            ],
            &[],
        );
        self.device
            .cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
        for draw in outlines {
            let push = base.outline_push(draw.model);
            self.device.cmd_push_constants(
                cmd,
                self.legacy_draw_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                cast_slice(&[push]),
            );
            self.draw_mesh(cmd, index_buffer, draw);
        }
    }

    /// 绘制 3D Gizmo 三轴（复用 outline pipeline + renderer 的 gizmo 顶点缓冲）
    ///
    /// # Safety
    /// 必须处于 HDR render pass 内；`gizmo_vertex_buffer` 需包含
    /// [`GIZMO_AXIS_COUNT`] × [`GIZMO_VERTICES_PER_AXIS`] 个 32B 顶点
    pub unsafe fn record_gizmo(
        &self,
        cmd: vk::CommandBuffer,
        light_set: Option<vk::DescriptorSet>,
        texture_set: Option<vk::DescriptorSet>,
        gizmo_vertex_buffer: vk::Buffer,
        position: [f32; 3],
        base: &LegacyPushBase,
    ) {
        self.device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.outline_pipeline,
        );
        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.legacy_draw_layout,
            0,
            &[
                light_set.unwrap_or(self.legacy_light_set),
                texture_set.unwrap_or(self.legacy_texture_set),
            ],
            &[],
        );
        self.device
            .cmd_bind_vertex_buffers(cmd, 0, &[gizmo_vertex_buffer], &[0]);
        let model = translation_matrix(position);
        for axis in 0..GIZMO_AXIS_COUNT {
            let push = base.gizmo_push(model, GIZMO_AXIS_COLORS[axis as usize]);
            self.device.cmd_push_constants(
                cmd,
                self.legacy_draw_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                cast_slice(&[push]),
            );
            self.device.cmd_draw(
                cmd,
                GIZMO_VERTICES_PER_AXIS,
                1,
                axis * GIZMO_VERTICES_PER_AXIS,
                0,
            );
        }
    }

    /// 结束 PBR HDR render pass
    ///
    /// # Safety
    /// 必须与 [`PbrForwardPass::begin_scene`] 配对
    pub unsafe fn end_scene(&self, cmd: vk::CommandBuffer) {
        self.device.cmd_end_render_pass(cmd);
    }

    /// 录制后处理：Bloom prefilter（可选）→ Tonemap（写 LDR）
    ///
    /// # Safety
    /// 同 [`PbrForwardPass::record_shadow`]
    pub unsafe fn record_post(&self, cmd: vk::CommandBuffer) {
        // 1. Bloom prefilter
        if let Some(bloom) = &self.bloom {
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];
            self.begin_render_pass(
                cmd,
                bloom.render_pass,
                bloom.framebuffer,
                bloom.extent,
                &clear_values,
            );
            self.set_viewport(cmd, bloom.extent);
            self.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                bloom.pipeline,
            );
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                bloom.layout,
                0,
                &[bloom.descriptor_set],
                &[],
            );
            let push = BloomPrefilterPushConstant::from_settings(
                &self.settings.bloom,
                [self.extent.width, self.extent.height],
            );
            self.device.cmd_push_constants(
                cmd,
                bloom.layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                cast_slice(&[push]),
            );
            self.device.cmd_draw(cmd, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(cmd);
        }

        // 2. Tonemap → LDR
        let extent = vk::Extent2D {
            width: self.extent.width,
            height: self.extent.height,
        };
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];
        self.begin_render_pass(
            cmd,
            self.tonemap_render_pass,
            self.tonemap_framebuffer,
            extent,
            &clear_values,
        );
        self.set_viewport(cmd, extent);
        self.device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.tonemap_pipeline,
        );
        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.tonemap_layout,
            0,
            &[self.tonemap_set],
            &[],
        );
        // SRGB 目标由硬件编码，避免 shader gamma 双重校正
        let mut tonemap = self.settings.tonemap;
        if self.ldr_is_srgb() {
            tonemap.gamma = 1.0;
        }
        let push = tonemap.push_constant(if self.bloom_active() {
            self.settings.bloom.intensity
        } else {
            0.0
        });
        self.device.cmd_push_constants(
            cmd,
            self.tonemap_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            cast_slice(&[push]),
        );
        self.device.cmd_draw(cmd, 3, 1, 0, 0);
        self.device.cmd_end_render_pass(cmd);
    }

    /// 销毁全部资源（调用方必须保证 GPU 已空闲）
    ///
    /// # Safety
    /// 必须在 `device_wait_idle()` 之后调用；`depth_view`/`ldr_view` 由调用方释放
    pub unsafe fn destroy(mut self) {
        let device = self.device.clone();
        let pipelines = [
            self.pbr_pipeline,
            self.tonemap_pipeline,
            self.sky_pipeline,
            self.outline_pipeline,
        ];
        for pipeline in pipelines {
            if pipeline != vk::Pipeline::null() {
                device.destroy_pipeline(pipeline, None);
            }
        }
        if self.sky_layout != vk::PipelineLayout::null() {
            device.destroy_pipeline_layout(self.sky_layout, None);
        }
        // outline/gizmo 的 legacy 兼容 layout + fallback sets
        if self.legacy_draw_layout != vk::PipelineLayout::null() {
            device.destroy_pipeline_layout(self.legacy_draw_layout, None);
        }
        device.destroy_descriptor_pool(self.legacy_pool, None);
        device.destroy_descriptor_set_layout(self.legacy_set0_layout, None);
        device.destroy_descriptor_set_layout(self.legacy_set1_layout, None);
        device.destroy_buffer(self.legacy_light_ubo, None);
        device.free_memory(self.legacy_light_ubo_memory, None);
        if let Some(mut shadow) = self.shadow.take() {
            device.destroy_pipeline(shadow.pipeline, None);
            device.destroy_pipeline_layout(shadow.layout, None);
            shadow.map.destroy(&device);
        }
        if let Some(bloom) = self.bloom.take() {
            device.destroy_pipeline(bloom.pipeline, None);
            device.destroy_pipeline_layout(bloom.layout, None);
            device.destroy_descriptor_pool(bloom.pool, None);
            device.destroy_render_pass(bloom.render_pass, None);
            device.destroy_framebuffer(bloom.framebuffer, None);
            destroy_gpu_target(&device, bloom.target);
        }
        device.destroy_pipeline_layout(self.pbr_layout, None);
        device.destroy_pipeline_layout(self.tonemap_layout, None);
        device.destroy_descriptor_pool(self.pbr_set0_pool, None);
        device.destroy_descriptor_pool(self.pbr_set1_pool, None);
        device.destroy_descriptor_pool(self.entity_texture_pool, None);
        self.entity_texture_cache.lock().clear();
        device.destroy_descriptor_pool(self.tonemap_pool, None);
        device.destroy_descriptor_set_layout(self.pbr_set0_layout, None);
        device.destroy_descriptor_set_layout(self.pbr_set1_layout, None);
        device.destroy_descriptor_set_layout(self.tonemap_set0_layout, None);
        device.destroy_render_pass(self.tonemap_render_pass, None);
        device.destroy_framebuffer(self.tonemap_framebuffer, None);
        device.destroy_render_pass(self.hdr.render_pass, None);
        device.destroy_framebuffer(self.hdr.framebuffer, None);
        device.destroy_sampler(self.hdr_sampler, None);
        device.destroy_sampler(self.material_sampler, None);
        device.destroy_buffer(self.pbr_frame_ubo, None);
        device.free_memory(self.pbr_frame_ubo_memory, None);
        destroy_gpu_target(&device, self.hdr.target);
        if let Some(target) = self.dummy_shadow.take() {
            destroy_gpu_target(&device, target);
        }
        if let Some(sampler) = self.dummy_shadow_sampler.take() {
            device.destroy_sampler(sampler, None);
        }
        for texture in self.material_textures.drain(..) {
            destroy_gpu_target(&device, texture);
        }
        let _ = self.depth_view;
        let _ = self.ldr_view;
    }

    unsafe fn begin_render_pass(
        &self,
        cmd: vk::CommandBuffer,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        extent: vk::Extent2D,
        clear_values: &[vk::ClearValue],
    ) {
        self.device.cmd_begin_render_pass(
            cmd,
            &vk::RenderPassBeginInfo {
                render_pass,
                framebuffer,
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                },
                clear_value_count: clear_values.len() as u32,
                p_clear_values: clear_values.as_ptr(),
                ..Default::default()
            },
            vk::SubpassContents::INLINE,
        );
    }

    unsafe fn set_viewport(&self, cmd: vk::CommandBuffer, extent: vk::Extent2D) {
        self.device.cmd_set_viewport(
            cmd,
            0,
            &[vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        self.device.cmd_set_scissor(
            cmd,
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            }],
        );
    }

    unsafe fn draw_mesh(&self, cmd: vk::CommandBuffer, index_buffer: vk::Buffer, draw: &MeshDraw) {
        if draw.index_count > 0 {
            self.device.cmd_bind_index_buffer(
                cmd,
                index_buffer,
                0,
                vk::IndexType::UINT32,
            );
            self.device.cmd_draw_indexed(
                cmd,
                draw.index_count,
                1,
                draw.index_offset,
                draw.vertex_offset as i32,
                0,
            );
        } else {
            self.device
                .cmd_draw(cmd, draw.vertex_count, 1, draw.vertex_offset, 0);
        }
    }
}

/// 蒙皮 pipeline 状态
///
/// 契约（与 `hezhou-animation` + `skinning.vert` 一致）:
/// - 顶点 stride 64，location 3 = joint indices (uvec4)，location 4 = weights (vec4)
/// - joint palette: set0 binding1，std430 `mat4 joints[]`（STORAGE_BUFFER）
/// - light UBO: set0 binding0，材质贴图 set1 binding0
/// - push constant: 复用 raster 144 bytes 布局
pub struct SkinnedPipelineState {
    device: ash::Device,
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    set0_layout: vk::DescriptorSetLayout,
    set1_layout: vk::DescriptorSetLayout,
    pool: vk::DescriptorPool,
    set0: vk::DescriptorSet,
    set1: vk::DescriptorSet,
    palette_buffer: vk::Buffer,
    palette_memory: vk::DeviceMemory,
    light_ubo: vk::Buffer,
    light_ubo_memory: vk::DeviceMemory,
    white_texture: GpuTarget,
    sampler: vk::Sampler,
    // 合成 skinned mesh（8 顶点 × 64B + 36 索引），用于验证 pipeline 实际进 draw
    skinned_vertex_buffer: vk::Buffer,
    skinned_vertex_memory: vk::DeviceMemory,
    skinned_index_buffer: vk::Buffer,
    skinned_index_memory: vk::DeviceMemory,
    skinned_vertex_count: u32,
    skinned_index_count: u32,
    // 每实体 palette descriptor 池/缓存（set0 = light UBO + palette SSBO）
    entity_pool: vk::DescriptorPool,
    entity_palette_sets: Mutex<HashMap<u64, vk::DescriptorSet>>,
}

impl SkinnedPipelineState {
    /// 创建蒙皮 pipeline 及 palette/light/材质占位资源
    ///
    /// # Safety
    /// 同 [`PbrForwardPass::create`]；`hdr_render_pass` 必须与 `PbrForwardPass` 兼容
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        hdr_render_pass: vk::RenderPass,
        _extent: ViewportExtent,
    ) -> Result<Self, String> {
        let device = device.clone();
        let memory_properties = instance.get_physical_device_memory_properties(physical_device);

        // set0: light UBO(binding0) + joint palette SSBO(binding1)
        let set0_layout = create_descriptor_set_layout(
            &device,
            &[
                binding(0, vk::DescriptorType::UNIFORM_BUFFER, vk::ShaderStageFlags::FRAGMENT),
                binding(
                    1,
                    vk::DescriptorType::STORAGE_BUFFER,
                    vk::ShaderStageFlags::VERTEX,
                ),
            ],
        )?;
        // set1: 材质贴图
        let set1_layout = create_descriptor_set_layout(
            &device,
            &[binding(
                0,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                vk::ShaderStageFlags::FRAGMENT,
            )],
        )?;
        let pool = create_descriptor_pool(
            &device,
            2,
            &[
                (vk::DescriptorType::UNIFORM_BUFFER, 1),
                (vk::DescriptorType::STORAGE_BUFFER, 1),
                (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 1),
            ],
        )?;
        let set0 = allocate_descriptor_set(&device, pool, set0_layout)?;
        let set1 = allocate_descriptor_set(&device, pool, set1_layout)?;

        // 每实体 palette 的 descriptor 池（set0 = light UBO + palette SSBO）
        let entity_pool = create_descriptor_pool(
            &device,
            MAX_SKINNED_ENTITIES as u32,
            &[
                (vk::DescriptorType::UNIFORM_BUFFER, MAX_SKINNED_ENTITIES as u32),
                (vk::DescriptorType::STORAGE_BUFFER, MAX_SKINNED_ENTITIES as u32),
            ],
        )?;

        // light UBO（32 bytes，与 rotation.frag 的 LightData 一致）
        let (light_ubo, light_ubo_memory) = create_host_buffer(
            &device,
            &memory_properties,
            32,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        let default_light: [f32; 8] = [-0.5, -1.0, -0.5, 0.0, 1.0, 1.0, 1.0, 1.0];
        let light_ptr = device
            .map_memory(light_ubo_memory, 0, 32, vk::MemoryMapFlags::empty())
            .map_err(|e| format!("SkinnedPipelineState: map light UBO 失败: {e}"))?;
        std::ptr::copy_nonoverlapping(
            default_light.as_ptr() as *const u8,
            light_ptr as *mut u8,
            32,
        );
        device.unmap_memory(light_ubo_memory);

        // joint palette SSBO
        let (palette_buffer, palette_memory) = create_host_buffer(
            &device,
            &memory_properties,
            JOINT_PALETTE_BUFFER_BYTES as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;

        // 白色占位贴图
        let white_texture =
            upload_texture(&device, &memory_properties, command_pool, queue, &[255, 255, 255, 255])?;
        let sampler = create_sampler(&device)?;
        let light_info = vk::DescriptorBufferInfo {
            buffer: light_ubo,
            offset: 0,
            range: 32,
        };
        let palette_info = vk::DescriptorBufferInfo {
            buffer: palette_buffer,
            offset: 0,
            range: JOINT_PALETTE_BUFFER_BYTES as u64,
        };
        let white_info = vk::DescriptorImageInfo {
            sampler,
            image_view: white_texture.view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet {
                    dst_set: set0,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &light_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: set0,
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    p_buffer_info: &palette_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: set1,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &white_info,
                    ..Default::default()
                },
            ],
            &[],
        );

        // pipeline: skinning.vert + rotation.frag，顶点输入 stride 64
        let vertex_input = SkinnedVertexInput::new();
        let layout = create_pipeline_layout(
            &device,
            144,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            &[set0_layout, set1_layout],
        )?;
        let pipeline = create_pipeline(
            &device,
            hdr_render_pass,
            layout,
            PipelineShaders {
                vert: include_bytes!("../../shaders/skinning.vert.spv"),
                frag: include_bytes!("../../shaders/rotation.frag.spv"),
            },
            &PipelineOptions {
                vertex_stride: vertex_input.binding.stride,
                vertex_attributes: &[
                    (0, vk::Format::R32G32B32_SFLOAT, 0),
                    (1, vk::Format::R32G32B32_SFLOAT, 12),
                    (2, vk::Format::R32G32_SFLOAT, 24),
                    (3, vk::Format::R32G32B32A32_UINT, 32),
                    (4, vk::Format::R32G32B32A32_SFLOAT, 48),
                ],
                depth_test: true,
                depth_write: true,
                depth_compare: vk::CompareOp::LESS,
                color_attachment: true,
                alpha_blend: false,
            },
        )?;

        // 合成 skinned mesh：8 顶点（64B stride）+ 36 索引，用于真实 draw 验证
        let (synthetic_vertices, synthetic_indices) = synthetic_skinned_cube();
        let vertex_bytes = crate::skinned_mesh::skinned_vertex_bytes(&synthetic_vertices);
        let (skinned_vertex_buffer, skinned_vertex_memory) = create_host_buffer(
            &device,
            &memory_properties,
            vertex_bytes.len() as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let vertex_ptr = device
            .map_memory(
                skinned_vertex_memory,
                0,
                vertex_bytes.len() as u64,
                vk::MemoryMapFlags::empty(),
            )
            .map_err(|e| format!("SkinnedPipelineState: map skinned vertex buffer 失败: {e}"))?;
        std::ptr::copy_nonoverlapping(
            vertex_bytes.as_ptr(),
            vertex_ptr as *mut u8,
            vertex_bytes.len(),
        );
        device.unmap_memory(skinned_vertex_memory);
        let index_bytes = cast_slice(&synthetic_indices);
        let (skinned_index_buffer, skinned_index_memory) = create_host_buffer(
            &device,
            &memory_properties,
            index_bytes.len() as u64,
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;
        let index_ptr = device
            .map_memory(
                skinned_index_memory,
                0,
                index_bytes.len() as u64,
                vk::MemoryMapFlags::empty(),
            )
            .map_err(|e| format!("SkinnedPipelineState: map skinned index buffer 失败: {e}"))?;
        std::ptr::copy_nonoverlapping(
            index_bytes.as_ptr(),
            index_ptr as *mut u8,
            index_bytes.len(),
        );
        device.unmap_memory(skinned_index_memory);

        Ok(Self {
            device,
            pipeline,
            layout,
            set0_layout,
            set1_layout,
            pool,
            set0,
            set1,
            palette_buffer,
            palette_memory,
            light_ubo,
            light_ubo_memory,
            white_texture,
            sampler,
            skinned_vertex_buffer,
            skinned_vertex_memory,
            skinned_index_buffer,
            skinned_index_memory,
            skinned_vertex_count: synthetic_vertices.len() as u32,
            skinned_index_count: synthetic_indices.len() as u32,
            entity_pool,
            entity_palette_sets: Mutex::new(HashMap::new()),
        })
    }

    /// 蒙皮 pipeline handle（供下一轮 draw 循环/测试使用）
    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    /// 合成 skinned mesh 顶点数（= 8）
    pub fn skinned_vertex_count(&self) -> u32 {
        self.skinned_vertex_count
    }

    /// 合成 skinned mesh 索引数（= 36）
    pub fn skinned_index_count(&self) -> u32 {
        self.skinned_index_count
    }

    /// 上传动画化 palette（2 关节，绕 Y 轴摆动；用于合成 skinned draw）
    ///
    /// # Safety
    /// palette buffer 为 HOST_VISIBLE|HOST_COHERENT
    pub unsafe fn update_palette_animated(&self, time: f32) -> Result<usize, String> {
        let bytes = synthetic_skin_palette(time);
        self.upload_joint_palette(&bytes)
    }

    /// 绘制合成 skinned mesh（验证 stride64 + location3/4 + palette 的真实 draw）
    ///
    /// `model` 为网格摆放矩阵；`base` 提供 legacy 相机 push 数据。
    ///
    /// # Safety
    /// 必须处于 HDR render pass 内；调用前已通过 `update_palette_animated` 或
    /// `upload_joint_palette` 上传 palette
    pub unsafe fn record_synthetic(
        &self,
        cmd: vk::CommandBuffer,
        model: [[f32; 4]; 4],
        base: &LegacyPushBase,
    ) {
        self.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.layout,
            0,
            &[self.set0, self.set1],
            &[],
        );
        self.device
            .cmd_bind_vertex_buffers(cmd, 0, &[self.skinned_vertex_buffer], &[0]);
        self.device.cmd_bind_index_buffer(
            cmd,
            self.skinned_index_buffer,
            0,
            vk::IndexType::UINT32,
        );
        let push = base.mesh_push(model, 0.0);
        self.device.cmd_push_constants(
            cmd,
            self.layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            cast_slice(&[push]),
        );
        self.device
            .cmd_draw_indexed(cmd, self.skinned_index_count, 1, 0, 0, 0);
    }

    /// 上传 joint palette（`JointPalette::to_le_bytes()` 输出）— 合成 mesh 专用
    ///
    /// # Safety
    /// palette buffer 为 HOST_VISIBLE|HOST_COHERENT
    pub unsafe fn upload_joint_palette(&self, bytes: &[u8]) -> Result<usize, String> {
        if bytes.len() > JOINT_PALETTE_BUFFER_BYTES {
            return Err(format!(
                "joint palette 过大: {} > {} 字节",
                bytes.len(),
                JOINT_PALETTE_BUFFER_BYTES
            ));
        }
        let ptr = self
            .device
            .map_memory(
                self.palette_memory,
                0,
                JOINT_PALETTE_BUFFER_BYTES as u64,
                vk::MemoryMapFlags::empty(),
            )
            .map_err(|e| format!("SkinnedPipelineState: map palette 失败: {e}"))?;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        self.device.unmap_memory(self.palette_memory);
        Ok(bytes.len())
    }

    /// 获取/创建真实蒙皮实体的 palette descriptor set
    ///
    /// cache_key 通常为 EntityId；renderer 在替换网格时复用同一 palette buffer，
    /// 调用 `forget_entity_palette` 可让被移除/重传的实体重建 descriptor。
    unsafe fn entity_palette_set(
        &self,
        cache_key: u64,
        palette_buffer: vk::Buffer,
    ) -> Option<vk::DescriptorSet> {
        if let Some(set) = self.entity_palette_sets.lock().get(&cache_key) {
            return Some(*set);
        }
        let mut cache = self.entity_palette_sets.lock();
        if let Some(set) = cache.get(&cache_key) {
            return Some(*set);
        }
        if cache.len() >= MAX_SKINNED_ENTITIES {
            return None;
        }
        let set = allocate_descriptor_set(&self.device, self.entity_pool, self.set0_layout).ok()?;
        let light_info = vk::DescriptorBufferInfo {
            buffer: self.light_ubo,
            offset: 0,
            range: 32,
        };
        let palette_info = vk::DescriptorBufferInfo {
            buffer: palette_buffer,
            offset: 0,
            range: crate::skinned_mesh::MAX_PALETTE_BYTES as u64,
        };
        self.device.update_descriptor_sets(
            &[
                vk::WriteDescriptorSet {
                    dst_set: set,
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &light_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: set,
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    p_buffer_info: &palette_info,
                    ..Default::default()
                },
            ],
            &[],
        );
        cache.insert(cache_key, set);
        Some(set)
    }

    /// 忘记实体的 palette descriptor（remove/重新上传网格时调用）
    pub fn forget_entity_palette(&self, cache_key: u64) {
        self.entity_palette_sets.lock().remove(&cache_key);
    }

    /// 已缓存的每实体 palette descriptor 数量（调试/测试）
    pub fn entity_palette_set_count(&self) -> usize {
        self.entity_palette_sets.lock().len()
    }

    /// 绘制真实上传的蒙皮网格（vertex stride 64 + 每实体 palette）
    ///
    /// # Safety
    /// 必须处于 HDR render pass 内；buffer 生命周期需覆盖本次提交
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn record_mesh(
        &self,
        cmd: vk::CommandBuffer,
        cache_key: u64,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        index_count: u32,
        palette_buffer: vk::Buffer,
        model: [[f32; 4]; 4],
        base: &LegacyPushBase,
    ) {
        let Some(entity_set) = self.entity_palette_set(cache_key, palette_buffer) else {
            return;
        };
        self.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            self.layout,
            0,
            &[entity_set, self.set1],
            &[],
        );
        self.device
            .cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
        self.device.cmd_bind_index_buffer(
            cmd,
            index_buffer,
            0,
            vk::IndexType::UINT32,
        );
        let push = base.mesh_push(model, 0.0);
        self.device.cmd_push_constants(
            cmd,
            self.layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            cast_slice(&[push]),
        );
        self.device.cmd_draw_indexed(cmd, index_count, 1, 0, 0, 0);
    }

    /// 销毁蒙皮资源（调用方必须保证 GPU 已空闲）
    ///
    /// # Safety
    /// 必须在 `device_wait_idle()` 之后调用
    pub unsafe fn destroy(self) {
        let device = self.device.clone();
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.layout, None);
        device.destroy_descriptor_pool(self.pool, None);
        device.destroy_descriptor_set_layout(self.set0_layout, None);
        device.destroy_descriptor_set_layout(self.set1_layout, None);
        device.destroy_buffer(self.palette_buffer, None);
        device.free_memory(self.palette_memory, None);
        device.destroy_buffer(self.light_ubo, None);
        device.free_memory(self.light_ubo_memory, None);
        destroy_gpu_target(&device, self.white_texture);
        device.destroy_sampler(self.sampler, None);
        device.destroy_buffer(self.skinned_vertex_buffer, None);
        device.free_memory(self.skinned_vertex_memory, None);
        device.destroy_buffer(self.skinned_index_buffer, None);
        device.free_memory(self.skinned_index_memory, None);
        device.destroy_descriptor_pool(self.entity_pool, None);
        self.entity_palette_sets.lock().clear();
        let _ = (self.set0, self.set1);
    }
}

// === 合成蒙皮网格（GPU 冒烟 / 编辑器自检用，验证 stride64 + location 3/4 + palette） ===

/// 构造一个上下两段分别受 2 个关节影响的立方体（8 顶点 × 64B + 36 索引）
///
/// 顶点布局与 `hezhou_animation::SkinnedVertex` 契约一致（stride 64）。
pub fn synthetic_skinned_cube() -> (Vec<SkinnedVertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(8);
    const HALF: f32 = 0.5;
    for (index, corner) in [
        [-HALF, -HALF, -HALF],
        [HALF, -HALF, -HALF],
        [HALF, -HALF, HALF],
        [-HALF, -HALF, HALF],
        [-HALF, HALF, -HALF],
        [HALF, HALF, -HALF],
        [HALF, HALF, HALF],
        [-HALF, HALF, HALF],
    ]
    .iter()
    .enumerate()
    {
        let position = *corner;
        let length = (position[0] * position[0]
            + position[1] * position[1]
            + position[2] * position[2])
            .sqrt()
            .max(1e-6);
        let normal = [
            position[0] / length,
            position[1] / length,
            position[2] / length,
        ];
        // 下半部分受 joint0 控制，上半部分受 joint1 控制
        let joint = if index < 4 { 0u32 } else { 1u32 };
        vertices.push(SkinnedVertex {
            position,
            normal,
            uv: [0.0, 0.0],
            joint_indices: [joint, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
        });
    }
    let indices = vec![
        0, 1, 2, 0, 2, 3, // bottom
        4, 6, 5, 4, 7, 6, // top
        0, 4, 5, 0, 5, 1, // front
        1, 5, 6, 1, 6, 2, // right
        2, 6, 7, 2, 7, 3, // back
        3, 7, 4, 3, 4, 0, // left
    ];
    (vertices, indices)
}

/// 生成两关节动画 palette 字节（128B = 2 × mat4，column-major）
///
/// joint0 = 单位矩阵（下身固定），joint1 = 绕 Y 轴按 `time` 摆动（上身摆动）。
pub fn synthetic_skin_palette(time: f32) -> Vec<u8> {
    let angle = (time * 0.8).sin() * 0.5;
    let c = angle.cos();
    let s = angle.sin();
    // column-major 3x3 Y 旋转填进 mat4
    let joint1: [[f32; 4]; 4] = [
        [c, 0.0, -s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::with_capacity(2 * 64);
    for matrix in [identity, joint1] {
        for column in matrix {
            for value in column {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    bytes
}

/// 合成蒙皮网格的摆放矩阵（相机前方 2.5 单位、缩放 0.5、随 `time` 旋转）
pub fn synthetic_skinned_model_matrix(
    camera_pos: [f32; 3],
    yaw: f32,
    pitch: f32,
    time: f32,
) -> [[f32; 4]; 4] {
    let (_, _, forward) = camera_basis(yaw, pitch);
    // 放在相机前方，略低于视线中心
    let position = [
        camera_pos[0] + forward[0] * 2.5,
        camera_pos[1] + forward[1] * 2.5 - 0.25,
        camera_pos[2] + forward[2] * 2.5,
    ];
    let scale = 0.5f32;
    let angle = time * 0.6;
    let c = angle.cos();
    let s = angle.sin();
    [
        [c * scale, 0.0, -s * scale, 0.0],
        [0.0, scale, 0.0, 0.0],
        [s * scale, 0.0, c * scale, 0.0],
        [position[0], position[1], position[2], 1.0],
    ]
}

// === 相机矩阵（与 shaders/rotation.vert 完全一致，保证 PBR 取景与 legacy 相同） ===

/// 视图矩阵（column-major，可直接上传 GLSL）
pub fn camera_view_matrix(position: [f32; 3], yaw: f32, pitch: f32) -> [[f32; 4]; 4] {
    let (right, up, forward) = camera_basis(yaw, pitch);
    [
        [right[0], up[0], -forward[0], 0.0],
        [right[1], up[1], -forward[1], 0.0],
        [right[2], up[2], -forward[2], 0.0],
        [
            -dot3(right, position),
            -dot3(up, position),
            dot3(forward, position),
            1.0,
        ],
    ]
}

/// 投影矩阵（column-major，Vulkan 约定：Y 翻转 + 深度 [0,1]）
pub fn camera_projection_matrix(
    fov_degrees: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    let fov_rad = fov_degrees * std::f32::consts::PI / 180.0;
    let f = 1.0 / (fov_rad * 0.5).tan();
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    };
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, -f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// 平移矩阵（column-major，与 GLSL `translate()` 一致）
pub fn translation_matrix(position: [f32; 3]) -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [position[0], position[1], position[2], 1.0],
    ]
}

/// 3D Gizmo 顶点数据（3 轴 × 24 顶点 × 8 floats，与 renderer `build_gizmo_vertices` 一致）
///
/// 每轴两条交叉薄四边形（双面），顶点格式 `x,y,z, nx,ny,nz, u,v`（stride 32B）。
pub fn gizmo_vertex_data() -> Vec<f32> {
    let axes = [
        ([1.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0], [0.0f32, 0.0, 1.0]),
        ([0.0f32, 1.0, 0.0], [1.0f32, 0.0, 0.0], [0.0f32, 0.0, 1.0]),
        ([0.0f32, 0.0, 1.0], [1.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0]),
    ];
    let len = 2.0f32;
    let w = 0.05f32;
    let mut out = Vec::with_capacity(
        GIZMO_AXIS_COUNT as usize * GIZMO_VERTICES_PER_AXIS as usize * 8,
    );
    for (d, u, v) in axes {
        for plane in [u, v] {
            let a = [-plane[0] * w, -plane[1] * w, -plane[2] * w];
            let b = [plane[0] * w, plane[1] * w, plane[2] * w];
            let c = [d[0] * len + a[0], d[1] * len + a[1], d[2] * len + a[2]];
            let e = [d[0] * len + b[0], d[1] * len + b[1], d[2] * len + b[2]];
            let mut push = |p: [f32; 3]| {
                out.extend_from_slice(&[p[0], p[1], p[2], d[0], d[1], d[2], 0.0, 0.0]);
            };
            // 双面：正反各两个三角形
            push(a);
            push(b);
            push(c);
            push(c);
            push(b);
            push(e);
            push(a);
            push(c);
            push(b);
            push(c);
            push(e);
            push(b);
        }
    }
    out
}

// === Vulkan 资源创建辅助函数 ===

/// 创建离屏渲染/采样目标（renderer 或 GPU 测试用）
///
/// # Safety
/// `device` 必须由 `instance`/`physical_device` 创建
pub unsafe fn create_render_target(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    extent: ViewportExtent,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    aspect: vk::ImageAspectFlags,
) -> Result<GpuTarget, String> {
    let memory_properties = instance.get_physical_device_memory_properties(physical_device);
    create_gpu_target(device, &memory_properties, extent, format, usage, aspect)
}

/// 销毁离屏目标
///
/// # Safety
/// 调用前必须确保 GPU 已空闲且资源不再被引用
pub unsafe fn destroy_render_target(device: &ash::Device, target: GpuTarget) {
    destroy_gpu_target(device, target)
}

/// 创建 HOST_VISIBLE|HOST_COHERENT buffer（GPU 测试 / 动态数据上传用）
///
/// # Safety
/// `device` 必须由 `instance`/`physical_device` 创建
pub unsafe fn create_host_visible_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    size: u64,
    usage: vk::BufferUsageFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let memory_properties = instance.get_physical_device_memory_properties(physical_device);
    create_host_buffer(device, &memory_properties, size, usage)
}

fn binding(
    binding: u32,
    descriptor_type: vk::DescriptorType,
    stage_flags: vk::ShaderStageFlags,
) -> vk::DescriptorSetLayoutBinding<'static> {
    vk::DescriptorSetLayoutBinding {
        binding,
        descriptor_type,
        descriptor_count: 1,
        stage_flags,
        p_immutable_samplers: std::ptr::null(),
        ..Default::default()
    }
}

/// 判断颜色格式是否为 SRGB（SRGB 目标由硬件做 gamma 编码）
pub fn format_is_srgb(format: vk::Format) -> bool {
    matches!(
        format,
        vk::Format::R8G8B8A8_SRGB
            | vk::Format::B8G8R8A8_SRGB
            | vk::Format::A8B8G8R8_SRGB_PACK32
            | vk::Format::R8_SRGB
    )
}

unsafe fn find_memory_type(
    properties: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    required: vk::MemoryPropertyFlags,
) -> Result<u32, String> {
    for index in 0..properties.memory_type_count {
        if (type_bits & (1 << index)) != 0
            && properties.memory_types[index as usize]
                .property_flags
                .contains(required)
        {
            return Ok(index);
        }
    }
    Err(format!(
        "PbrForwardPass: 找不到满足 {required:?} 的显存类型"
    ))
}

unsafe fn create_gpu_target(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    extent: ViewportExtent,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    aspect: vk::ImageAspectFlags,
) -> Result<GpuTarget, String> {
    let image = device
        .create_image(
            &vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format,
                extent: vk::Extent3D {
                    width: extent.width,
                    height: extent.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 image 失败: {e}"))?;
    let requirements = device.get_image_memory_requirements(image);
    let memory_type = find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    let memory = device
        .allocate_memory(
            &vk::MemoryAllocateInfo {
                allocation_size: requirements.size,
                memory_type_index: memory_type,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 分配 image memory 失败: {e}"))?;
    device
        .bind_image_memory(image, memory, 0)
        .map_err(|e| format!("PbrForwardPass: 绑定 image memory 失败: {e}"))?;
    let view = device
        .create_image_view(
            &vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::TYPE_2D,
                format,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 image view 失败: {e}"))?;
    Ok(GpuTarget {
        image,
        memory,
        view,
    })
}

unsafe fn destroy_gpu_target(device: &ash::Device, target: GpuTarget) {
    if target.view != vk::ImageView::null() {
        device.destroy_image_view(target.view, None);
    }
    if target.image != vk::Image::null() {
        device.destroy_image(target.image, None);
    }
    if target.memory != vk::DeviceMemory::null() {
        device.free_memory(target.memory, None);
    }
}

unsafe fn create_host_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    size: u64,
    usage: vk::BufferUsageFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let buffer = device
        .create_buffer(
            &vk::BufferCreateInfo {
                size,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 buffer 失败: {e}"))?;
    let requirements = device.get_buffer_memory_requirements(buffer);
    let memory_type = find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    let memory = device
        .allocate_memory(
            &vk::MemoryAllocateInfo {
                allocation_size: requirements.size,
                memory_type_index: memory_type,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 分配 buffer memory 失败: {e}"))?;
    device
        .bind_buffer_memory(buffer, memory, 0)
        .map_err(|e| format!("PbrForwardPass: 绑定 buffer memory 失败: {e}"))?;
    Ok((buffer, memory))
}

unsafe fn create_sampler(device: &ash::Device) -> Result<vk::Sampler, String> {
    device
        .create_sampler(
            &vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                mipmap_mode: vk::SamplerMipmapMode::NEAREST,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
                anisotropy_enable: vk::FALSE,
                max_anisotropy: 1.0,
                compare_enable: vk::FALSE,
                compare_op: vk::CompareOp::ALWAYS,
                min_lod: 0.0,
                max_lod: 0.0,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 sampler 失败: {e}"))
}

unsafe fn create_shadow_compare_sampler(device: &ash::Device) -> Result<vk::Sampler, String> {
    device
        .create_sampler(
            &vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                mipmap_mode: vk::SamplerMipmapMode::NEAREST,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
                anisotropy_enable: vk::FALSE,
                max_anisotropy: 1.0,
                compare_enable: vk::TRUE,
                compare_op: vk::CompareOp::LESS_OR_EQUAL,
                min_lod: 0.0,
                max_lod: 0.0,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建阴影比较 sampler 失败: {e}"))
}

unsafe fn create_framebuffer(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    attachments: &[vk::ImageView],
    extent: ViewportExtent,
) -> Result<vk::Framebuffer, String> {
    device
        .create_framebuffer(
            &vk::FramebufferCreateInfo {
                render_pass,
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                width: extent.width,
                height: extent.height,
                layers: 1,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 framebuffer 失败: {e}"))
}

unsafe fn create_hdr_render_pass(device: &ash::Device) -> Result<vk::RenderPass, String> {
    let attachments = [
        vk::AttachmentDescription {
            format: HDR_COLOR_FORMAT,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ..Default::default()
        },
        vk::AttachmentDescription {
            format: vk::Format::D32_SFLOAT,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        },
    ];
    let color_reference = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };
    let depth_reference = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };
    let subpass = vk::SubpassDescription {
        pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
        color_attachment_count: 1,
        p_color_attachments: &color_reference,
        p_depth_stencil_attachment: &depth_reference,
        ..Default::default()
    };
    let dependencies = create_dependencies();
    device
        .create_render_pass(
            &vk::RenderPassCreateInfo {
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                subpass_count: 1,
                p_subpasses: &subpass,
                dependency_count: dependencies.len() as u32,
                p_dependencies: dependencies.as_ptr(),
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 HDR render pass 失败: {e}"))
}

unsafe fn create_ldr_render_pass(
    device: &ash::Device,
    format: vk::Format,
) -> Result<vk::RenderPass, String> {
    let attachments = [vk::AttachmentDescription {
        format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        // 调用方（renderer draw_frame）已把 LDR offscreen image 转为 COLOR_ATTACHMENT
        initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        ..Default::default()
    }];
    let color_reference = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };
    let subpass = vk::SubpassDescription {
        pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
        color_attachment_count: 1,
        p_color_attachments: &color_reference,
        ..Default::default()
    };
    let dependencies = create_dependencies();
    device
        .create_render_pass(
            &vk::RenderPassCreateInfo {
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                subpass_count: 1,
                p_subpasses: &subpass,
                dependency_count: dependencies.len() as u32,
                p_dependencies: dependencies.as_ptr(),
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 LDR render pass 失败: {e}"))
}

unsafe fn create_bloom_render_pass(device: &ash::Device) -> Result<vk::RenderPass, String> {
    let attachments = [vk::AttachmentDescription {
        format: HDR_COLOR_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        ..Default::default()
    }];
    let color_reference = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };
    let subpass = vk::SubpassDescription {
        pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
        color_attachment_count: 1,
        p_color_attachments: &color_reference,
        ..Default::default()
    };
    let dependencies = create_dependencies();
    device
        .create_render_pass(
            &vk::RenderPassCreateInfo {
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                subpass_count: 1,
                p_subpasses: &subpass,
                dependency_count: dependencies.len() as u32,
                p_dependencies: dependencies.as_ptr(),
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 Bloom render pass 失败: {e}"))
}

fn create_dependencies() -> [vk::SubpassDependency; 2] {
    [
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::SHADER_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
            ..Default::default()
        },
        vk::SubpassDependency {
            src_subpass: 0,
            dst_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
            src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            dependency_flags: vk::DependencyFlags::empty(),
            ..Default::default()
        },
    ]
}

unsafe fn create_descriptor_set_layout(
    device: &ash::Device,
    bindings: &[vk::DescriptorSetLayoutBinding<'_>],
) -> Result<vk::DescriptorSetLayout, String> {
    device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo {
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 descriptor set layout 失败: {e}"))
}

unsafe fn create_descriptor_pool(
    device: &ash::Device,
    max_sets: u32,
    sizes: &[(vk::DescriptorType, u32)],
) -> Result<vk::DescriptorPool, String> {
    let pool_sizes: Vec<vk::DescriptorPoolSize> = sizes
        .iter()
        .map(|(ty, count)| vk::DescriptorPoolSize {
            ty: *ty,
            descriptor_count: *count,
            ..Default::default()
        })
        .collect();
    device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo {
                max_sets,
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 descriptor pool 失败: {e}"))
}

unsafe fn allocate_descriptor_set(
    device: &ash::Device,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> Result<vk::DescriptorSet, String> {
    let layouts = [layout];
    device
        .allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
            descriptor_pool: pool,
            descriptor_set_count: 1,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        })
        .map(|sets| sets[0])
        .map_err(|e| format!("PbrForwardPass: 分配 descriptor set 失败: {e}"))
}

unsafe fn create_pipeline_layout(
    device: &ash::Device,
    push_constant_size: u32,
    push_stages: vk::ShaderStageFlags,
    set_layouts: &[vk::DescriptorSetLayout],
) -> Result<vk::PipelineLayout, String> {
    let push_range = vk::PushConstantRange {
        stage_flags: push_stages,
        offset: 0,
        size: push_constant_size,
    };
    device
        .create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo {
                set_layout_count: set_layouts.len() as u32,
                p_set_layouts: set_layouts.as_ptr(),
                push_constant_range_count: if push_constant_size > 0 { 1 } else { 0 },
                p_push_constant_ranges: if push_constant_size > 0 {
                    &push_range
                } else {
                    std::ptr::null()
                },
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 pipeline layout 失败: {e}"))
}

unsafe fn create_shader_module(
    device: &ash::Device,
    spirv: &[u8],
) -> Result<vk::ShaderModule, String> {
    device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo {
                code_size: spirv.len(),
                p_code: spirv.as_ptr() as *const u32,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 shader module 失败: {e}"))
}

struct PipelineShaders<'a> {
    vert: &'a [u8],
    frag: &'a [u8],
}

struct PipelineOptions<'a> {
    vertex_stride: u32,
    vertex_attributes: &'a [(u32, vk::Format, u32)],
    depth_test: bool,
    depth_write: bool,
    depth_compare: vk::CompareOp,
    color_attachment: bool,
    /// 是否启用 SRC_ALPHA/ONE_MINUS_SRC_ALPHA 混合（outline 用）
    alpha_blend: bool,
}

unsafe fn create_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    shaders: PipelineShaders<'_>,
    options: &PipelineOptions<'_>,
) -> Result<vk::Pipeline, String> {
    let vert_module = create_shader_module(device, shaders.vert)?;
    let frag_module = create_shader_module(device, shaders.frag)?;
    let stages = [
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vert_module,
            p_name: b"main\0".as_ptr() as *const i8,
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: frag_module,
            p_name: b"main\0".as_ptr() as *const i8,
            ..Default::default()
        },
    ];
    let vertex_binding = vk::VertexInputBindingDescription {
        binding: 0,
        stride: options.vertex_stride,
        input_rate: vk::VertexInputRate::VERTEX,
    };
    let vertex_attributes: Vec<vk::VertexInputAttributeDescription> = options
        .vertex_attributes
        .iter()
        .map(|(location, format, offset)| vk::VertexInputAttributeDescription {
            location: *location,
            binding: 0,
            format: *format,
            offset: *offset,
        })
        .collect();
    let vertex_input = vk::PipelineVertexInputStateCreateInfo {
        vertex_binding_description_count: if options.vertex_stride > 0 { 1 } else { 0 },
        p_vertex_binding_descriptions: if options.vertex_stride > 0 {
            &vertex_binding
        } else {
            std::ptr::null()
        },
        vertex_attribute_description_count: vertex_attributes.len() as u32,
        p_vertex_attribute_descriptions: vertex_attributes.as_ptr(),
        ..Default::default()
    };
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
        blend_enable: if options.alpha_blend { vk::TRUE } else { vk::FALSE },
        color_write_mask: vk::ColorComponentFlags::R
            | vk::ColorComponentFlags::G
            | vk::ColorComponentFlags::B
            | vk::ColorComponentFlags::A,
        src_color_blend_factor: if options.alpha_blend {
            vk::BlendFactor::SRC_ALPHA
        } else {
            vk::BlendFactor::ONE
        },
        dst_color_blend_factor: if options.alpha_blend {
            vk::BlendFactor::ONE_MINUS_SRC_ALPHA
        } else {
            vk::BlendFactor::ZERO
        },
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
    };
    let color_blend = vk::PipelineColorBlendStateCreateInfo {
        logic_op_enable: vk::FALSE,
        attachment_count: if options.color_attachment { 1 } else { 0 },
        p_attachments: if options.color_attachment {
            &color_blend_attachment
        } else {
            std::ptr::null()
        },
        ..Default::default()
    };
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: if options.depth_test { vk::TRUE } else { vk::FALSE },
        depth_write_enable: if options.depth_write { vk::TRUE } else { vk::FALSE },
        depth_compare_op: options.depth_compare,
        depth_bounds_test_enable: vk::FALSE,
        stencil_test_enable: vk::FALSE,
        ..Default::default()
    };
    let create_info = vk::GraphicsPipelineCreateInfo {
        stage_count: stages.len() as u32,
        p_stages: stages.as_ptr(),
        p_vertex_input_state: &vertex_input,
        p_input_assembly_state: &vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        },
        p_viewport_state: &vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        },
        p_rasterization_state: &vk::PipelineRasterizationStateCreateInfo {
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
            line_width: 1.0,
            ..Default::default()
        },
        p_multisample_state: &vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        },
        p_depth_stencil_state: &depth_stencil,
        p_color_blend_state: &color_blend,
        p_dynamic_state: &vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        },
        layout,
        render_pass,
        subpass: 0,
        ..Default::default()
    };
    let result = device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None);
    device.destroy_shader_module(vert_module, None);
    device.destroy_shader_module(frag_module, None);
    match result {
        Ok(pipelines) => Ok(pipelines[0]),
        Err((_, err)) => Err(format!("PbrForwardPass: 创建 graphics pipeline 失败: {err}")),
    }
}

/// 执行一次性命令（纹理上传/布局转换）
unsafe fn run_one_shot(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    record: impl FnOnce(vk::CommandBuffer),
) -> Result<(), String> {
    let command_buffer = device
        .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        })
        .map_err(|e| format!("PbrForwardPass: 分配一次性 command buffer 失败: {e}"))?[0];
    device
        .begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())
        .map_err(|e| format!("PbrForwardPass: begin 一次性 command buffer 失败: {e}"))?;
    record(command_buffer);
    device
        .end_command_buffer(command_buffer)
        .map_err(|e| format!("PbrForwardPass: end 一次性 command buffer 失败: {e}"))?;
    device
        .queue_submit(
            queue,
            &[vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &command_buffer,
                ..Default::default()
            }],
            vk::Fence::null(),
        )
        .map_err(|e| format!("PbrForwardPass: 提交一次性 command buffer 失败: {e}"))?;
    device
        .queue_wait_idle(queue)
        .map_err(|e| format!("PbrForwardPass: 等待一次性 command buffer 失败: {e}"))?;
    device.free_command_buffers(command_pool, &[command_buffer]);
    Ok(())
}

unsafe fn transition_image(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    aspect: vk::ImageAspectFlags,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
    src_access: vk::AccessFlags,
    dst_access: vk::AccessFlags,
) {
    device.cmd_pipeline_barrier(
        command_buffer,
        src_stage,
        dst_stage,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[vk::ImageMemoryBarrier {
            old_layout,
            new_layout,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: src_access,
            dst_access_mask: dst_access,
            ..Default::default()
        }],
    );
}

/// 上传 1x1 RGBA 贴图并转为 SHADER_READ_ONLY
unsafe fn upload_texture(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    rgba: &[u8; 4],
) -> Result<GpuTarget, String> {
    let image = device
        .create_image(
            &vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                extent: vk::Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::LINEAR,
                usage: vk::ImageUsageFlags::SAMPLED,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout: vk::ImageLayout::PREINITIALIZED,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 1x1 贴图失败: {e}"))?;
    let requirements = device.get_image_memory_requirements(image);
    let memory_type = find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    let memory = device
        .allocate_memory(
            &vk::MemoryAllocateInfo {
                allocation_size: requirements.size,
                memory_type_index: memory_type,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 分配 1x1 贴图显存失败: {e}"))?;
    device
        .bind_image_memory(image, memory, 0)
        .map_err(|e| format!("PbrForwardPass: 绑定 1x1 贴图显存失败: {e}"))?;
    let ptr = device
        .map_memory(memory, 0, requirements.size, vk::MemoryMapFlags::empty())
        .map_err(|e| format!("PbrForwardPass: map 1x1 贴图失败: {e}"))?;
    std::ptr::copy_nonoverlapping(rgba.as_ptr(), ptr as *mut u8, 4);
    device.unmap_memory(memory);

    run_one_shot(device, command_pool, queue, |command_buffer| {
        transition_image(
            device,
            command_buffer,
            image,
            vk::ImageAspectFlags::COLOR,
            vk::ImageLayout::PREINITIALIZED,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::AccessFlags::HOST_WRITE,
            vk::AccessFlags::SHADER_READ,
        );
    })?;

    let view = device
        .create_image_view(
            &vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建 1x1 贴图 view 失败: {e}"))?;
    Ok(GpuTarget {
        image,
        memory,
        view,
    })
}

/// 创建 1x1 无阴影占位深度图（D32，单层 array，采样永远返回受光）
unsafe fn create_dummy_shadow_target(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
) -> Result<GpuTarget, String> {
    let image = device
        .create_image(
            &vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::D32_SFLOAT,
                extent: vk::Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage: vk::ImageUsageFlags::SAMPLED,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建占位阴影图失败: {e}"))?;
    let requirements = device.get_image_memory_requirements(image);
    let memory_type = find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;
    let memory = device
        .allocate_memory(
            &vk::MemoryAllocateInfo {
                allocation_size: requirements.size,
                memory_type_index: memory_type,
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 分配占位阴影图显存失败: {e}"))?;
    device
        .bind_image_memory(image, memory, 0)
        .map_err(|e| format!("PbrForwardPass: 绑定占位阴影图显存失败: {e}"))?;
    run_one_shot(device, command_pool, queue, |command_buffer| {
        transition_image(
            device,
            command_buffer,
            image,
            vk::ImageAspectFlags::DEPTH,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::AccessFlags::empty(),
            vk::AccessFlags::SHADER_READ,
        );
    })?;
    let view = device
        .create_image_view(
            &vk::ImageViewCreateInfo {
                image,
                view_type: vk::ImageViewType::TYPE_2D_ARRAY,
                format: vk::Format::D32_SFLOAT,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            },
            None,
        )
        .map_err(|e| format!("PbrForwardPass: 创建占位阴影图 view 失败: {e}"))?;
    Ok(GpuTarget {
        image,
        memory,
        view,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_draw_constructors() {
        let model = [[1.0; 4]; 4];
        let primitive = MeshDraw::primitive(10, 36, model);
        assert!(!primitive.is_indexed());
        assert_eq!(primitive.vertex_offset, 10);
        assert_eq!(primitive.vertex_count, 36);
        let indexed = MeshDraw::indexed(100, 8, 200, 24, model);
        assert!(indexed.is_indexed());
        assert_eq!(indexed.index_count, 24);
    }

    #[test]
    fn push_constant_sizes_match_contracts() {
        assert_eq!(PBR_PUSH_CONSTANT_BYTES, 96);
        assert_eq!(PBR_FRAME_UBO_BYTES, 480);
        assert_eq!(SHADOW_PUSH_CONSTANT_BYTES, 128);
        assert_eq!(TONEMAP_PUSH_CONSTANT_BYTES, 16);
        assert_eq!(BLOOM_PUSH_CONSTANT_BYTES, 16);
        assert_eq!(JOINT_PALETTE_BUFFER_BYTES, MAX_SKIN_JOINTS * 64);
    }

    #[test]
    fn srgb_formats_detected() {
        assert!(format_is_srgb(vk::Format::B8G8R8A8_SRGB));
        assert!(format_is_srgb(vk::Format::R8G8B8A8_SRGB));
        assert!(!format_is_srgb(vk::Format::B8G8R8A8_UNORM));
        assert!(!format_is_srgb(vk::Format::R16G16B16A16_SFLOAT));
    }

    #[test]
    fn view_matrix_matches_rotation_vert_semantics() {
        // 相机在 (0,0,3)，yaw=pitch=0：世界原点应落在相机前方 3 单位
        let view = camera_view_matrix([0.0, 0.0, 3.0], 0.0, 0.0);
        let point = [0.0f32, 0.0, 0.0, 1.0];
        let transformed = [
            view[0][0] * point[0] + view[1][0] * point[1] + view[2][0] * point[2] + view[3][0],
            view[0][1] * point[0] + view[1][1] * point[1] + view[2][1] * point[2] + view[3][1],
            view[0][2] * point[0] + view[1][2] * point[1] + view[2][2] * point[2] + view[3][2],
        ];
        assert!((transformed[0]).abs() < 1e-6);
        assert!((transformed[1]).abs() < 1e-6);
        // 相机看向 -Z，原点在相机前方 → 视图空间 z = -3
        assert!((transformed[2] + 3.0).abs() < 1e-6);
    }

    #[test]
    fn projection_maps_near_far_to_zero_one() {
        let proj = camera_projection_matrix(60.0, 16.0 / 9.0, 0.1, 100.0);
        let transform = |z: f32| {
            // 齐次坐标 (0,0,z,1)
            let clip_z = proj[2][2] * z + proj[3][2];
            let clip_w = proj[2][3] * z + proj[3][3];
            clip_z / clip_w
        };
        assert!(transform(-0.1).abs() < 1e-4);
        assert!((transform(-100.0) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn projection_matches_opengl_vulkan_conventions() {
        // Y 轴翻转：Vulkan NDC y 向下
        let proj = camera_projection_matrix(60.0, 1.0, 0.1, 100.0);
        assert!(proj[1][1] < 0.0, "Vulkan 约定需要 Y 翻转");
        // 深度范围 [0,1]
        assert!(proj[3][2].abs() > 0.0);
    }

    #[test]
    fn sky_and_legacy_push_layouts_match_shaders() {
        assert_eq!(SKY_PUSH_CONSTANT_BYTES, 32);
        assert_eq!(std::mem::size_of::<SkyPushConstant>(), 32);
        assert_eq!(LEGACY_PUSH_CONSTANT_BYTES, 144);
        let sky = SkyPushConstant::new([0.1, 0.2, 0.3], [0.4, 0.5, 0.6]);
        assert_eq!(sky.top, [0.1, 0.2, 0.3, 1.0]);
        assert_eq!(sky.bottom, [0.4, 0.5, 0.6, 1.0]);

        let base = LegacyPushBase {
            viewport: [1280.0, 720.0],
            camera_pos: [1.0, 2.0, 3.0],
            camera_yaw: 0.2,
            camera_pitch: -0.3,
        };
        let push = base.outline_push(
            [[1.0; 4]; 4],
        );
        assert_eq!(push.outline_color, [1.0, 0.5, 0.0]);
        assert_eq!(push.is_selected, 1.0);
        assert_eq!(push.viewport_size, [1280.0, 720.0]);
        assert_eq!(push.camera_pos, [1.0, 2.0, 3.0]);
        assert_eq!(push.has_texture, 0.0);
        let bytes: [u8; 144] = bytemuck::cast(push);
        assert_eq!(bytes.len(), 144);
    }

    #[test]
    fn gizmo_vertex_data_matches_axis_layout() {
        let vertices = gizmo_vertex_data();
        assert_eq!(
            vertices.len(),
            GIZMO_AXIS_COUNT as usize * GIZMO_VERTICES_PER_AXIS as usize * 8
        );
        assert_eq!(
            vertices.len() as u64 * 4,
            GIZMO_VERTEX_BUFFER_BYTES,
            "gizmo 顶点数据字节数应与常量一致"
        );
        // 前 24 个顶点是 X 轴（法线 = (1,0,0)，位于第 3..5 个 float）
        for vertex in 0..24 {
            let offset = vertex * 8;
            assert_eq!(vertices[offset + 3], 1.0);
            assert_eq!(vertices[offset + 4], 0.0);
            assert_eq!(vertices[offset + 5], 0.0);
        }
        assert_eq!(GIZMO_AXIS_COLORS[0], [0.95, 0.25, 0.25]);
    }

    #[test]
    fn synthetic_skinned_cube_matches_animation_contract() {
        let (vertices, indices) = synthetic_skinned_cube();
        assert_eq!(vertices.len(), 8);
        assert_eq!(indices.len(), 36);
        assert_eq!(SKINNED_VERTEX_STRIDE, 64);
        // 下半 4 顶点 → joint0，上半 4 顶点 → joint1
        for (index, vertex) in vertices.iter().enumerate() {
            let expected_joint = if index < 4 { 0 } else { 1 };
            assert_eq!(vertex.joint_indices, [expected_joint, 0, 0, 0]);
            assert_eq!(vertex.joint_weights, [1.0, 0.0, 0.0, 0.0]);
            assert!(vertex.position.iter().all(|v| v.abs() <= 0.5));
        }
        // 索引都在顶点范围内
        assert!(indices.iter().all(|index| (*index as usize) < vertices.len()));
    }

    #[test]
    fn synthetic_palette_is_two_matrices_and_finite() {
        let bytes = synthetic_skin_palette(0.0);
        assert_eq!(bytes.len(), 2 * 64);
        assert_eq!(bytes.len(), joint_palette_bytes(2));
        // time=0 时角度=0 → 两个矩阵都是近似单位阵
        let values: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        assert_eq!(values.len(), 32);
        assert!(values.iter().all(|value| value.is_finite()));
        assert!((values[0] - 1.0).abs() < 1e-6);
        // 非零时间后矩阵仍有限
        let animated = synthetic_skin_palette(3.0);
        assert_eq!(animated.len(), 128);
    }

    #[test]
    fn synthetic_model_matrix_places_mesh_in_front_of_camera() {
        let matrix = synthetic_skinned_model_matrix([0.0, 0.0, 5.0], 0.0, 0.0, 0.0);
        // yaw=0,pitch=0 → forward=(0,0,-1)，网格应在 z = 5 - 2.5 = 2.5
        assert!((matrix[3][2] - 2.5).abs() < 1e-5);
        assert!((matrix[3][1] + 0.25).abs() < 1e-5);
        assert!(matrix.iter().flatten().all(|value| value.is_finite()));
    }

    #[test]
    fn translation_matrix_matches_glsl_translate() {
        let matrix = translation_matrix([1.0, 2.0, 3.0]);
        assert_eq!(matrix[3], [1.0, 2.0, 3.0, 1.0]);
    }

    // === 每实体材质 draw item（task-22） ===

    #[test]
    fn pbr_draw_item_material_push_constants_differ() {
        let mesh = MeshDraw::primitive(0, 36, [[1.0; 4]; 4]);
        let red_metal = PbrMaterial::metal([0.9, 0.1, 0.1], 0.15);
        let blue_rough = PbrMaterial::dielectric([0.1, 0.2, 0.9], 0.85);
        let item_a = PbrDrawItem::new(mesh, red_metal);
        let item_b = PbrDrawItem::new(mesh, blue_rough);
        assert!(item_a.texture.is_none());
        let push_a = item_a.push_constant();
        let push_b = item_b.push_constant();
        assert_ne!(push_a.base_color, push_b.base_color);
        assert_ne!(push_a.material_params, push_b.material_params);
        assert_eq!(push_a.material_params[0], 1.0); // metallic
        assert_eq!(push_b.material_params[0], 0.0);
        assert!(push_a.material_params[1] < push_b.material_params[1]); // roughness
    }

    #[test]
    fn pbr_draw_item_texture_binding_is_carried() {
        let mesh = MeshDraw::primitive(0, 36, [[1.0; 4]; 4]);
        let binding = PbrTextureBinding {
            cache_key: 42,
            view: vk::ImageView::null(),
            sampler: vk::Sampler::null(),
        };
        let item = PbrDrawItem::with_texture(mesh, PbrMaterial::default(), binding);
        assert_eq!(item.texture, Some(binding));
        assert_eq!(MAX_ENTITY_TEXTURE_SETS, 256);
        let plain = PbrDrawItem::new(mesh, PbrMaterial::default());
        assert!(plain.texture.is_none());
    }

    #[test]
    fn pbr_material_override_maps_to_item_material() {
        // 模拟 renderer 的实体材质解析：覆盖命中 vs 默认回退
        let default = PbrMaterial::default();
        let mut overrides = hezhou_render_pipeline::PbrMaterialOverrides::new(default);
        let red = PbrMaterial::metal([0.9, 0.1, 0.1], 0.2);
        overrides.set(
            hezhou_render_pipeline::PbrMaterialOverride::new(7, red).with_texture_path("tex/red.png"),
        );
        let mesh = MeshDraw::primitive(0, 3, [[1.0; 4]; 4]);
        let item_7 = PbrDrawItem::new(
            mesh,
            overrides.resolve_with_default(7, &default).clone(),
        );
        let item_8 = PbrDrawItem::new(
            mesh,
            overrides.resolve_with_default(8, &default).clone(),
        );
        assert_eq!(item_7.material, red);
        assert_eq!(item_8.material, default);
        assert_eq!(overrides.resolve_texture_path(7), Some("tex/red.png"));
        assert_eq!(overrides.resolve_texture_path(8), None);
    }
}
