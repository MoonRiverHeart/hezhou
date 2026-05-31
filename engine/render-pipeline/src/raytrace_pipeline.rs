//! RayTracePipeline — ray tracing compute shader管线实现
//!
//! 使用BVH加速结构 + Möller-Trumbore ray-triangle intersection，
//! 在compute shader中逐像素生成光线并追踪，输出到storage image。
//!
//! 实现 RenderPipeline trait 的5个hook点:
//! - prepare: 从scene收集mesh数据，构建BVH，上传到GPU SSBO
//! - record: 绑定compute pipeline + descriptor sets，cmd_dispatch
//! - post_process: 直接返回input（ray tracing不需要额外后处理）
//! - composite: 返回preview_descriptor_set供UI采样
//! - cleanup: 空操作
//!
//! **重要约束**:
//! - push constant布局严格匹配GLSL std430对齐规则（40 bytes）
//! - compute pipeline使用vk::PipelineBindPoint::COMPUTE（不是GRAPHICS）
//! - pipeline barrier: HOST→COMPUTE和COMPUTE→FRAGMENT必须正确设置
//! - BVH数据动态分配buffer（不假设固定节点数量）

use ash::vk;
use bytemuck::{Pod, Zeroable};
use hezhou_core::{
    LocalTransform, RenderableComponent, DirectionalLightComponent,
    MeshType, Mat4, Vec4,
};
use hezhou_geometry::MeshData;

use crate::pipeline::{RenderPipeline, RenderPassOutput};
use crate::context::RenderContext;
use crate::resource::{PipelineResources, PipelineResourceDesc};
use crate::bvh::Bvh;
use crate::raster_pipeline::LightUboData;

// === EmissiveTriangle数据布局 — 80 bytes, vec4 packing, 严格匹配GLSL std430 ===
//
// GLSL EmissiveTriangle struct布局:
// ```glsl
// struct EmissiveTriangle {
//     vec4 v0_area;    // v0.xyz + triangle_area (offset 0, 16 bytes)
//     vec4 v1_pad;     // v1.xyz + 0(pad) (offset 16, 16 bytes)
//     vec4 v2_pad;     // v2.xyz + 0(pad) (offset 32, 16 bytes)
//     vec4 normal_pad; // normal.xyz + 0(pad) (offset 48, 16 bytes)
//     vec4 color_int;  // color.xyz + intensity (offset 64, 16 bytes)
// };
// // 总计: 80 bytes per emissive triangle
// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EmissiveTriangle {
    /// 顶点v0坐标 + 三角形面积 [v0.x, v0.y, v0.z, area]
    v0_area: [f32; 4],
    /// 顶点v1坐标 + padding [v1.x, v1.y, v1.z, 0.0]
    v1_pad: [f32; 4],
    /// 顶点v2坐标 + padding [v2.x, v2.y, v2.z, 0.0]
    v2_pad: [f32; 4],
    /// 面法线 + padding [normal.x, normal.y, normal.z, 0.0]
    normal_pad: [f32; 4],
    /// 光源颜色 + 强度 [color.x, color.y, color.z, intensity]
    color_int: [f32; 4],
}

// === Push Constant数据布局 — 40 bytes, 严格匹配GLSL std430 ===
//
// GLSL PushConstants struct布局:
// ```glsl
// layout(push_constant) uniform PushConstants {
//     vec3 camera_pos;    // offset 0, size 12
//     float camera_yaw;   // offset 12, size 4 (填充vec3→vec4空隙)
//     float camera_pitch; // offset 16, size 4
//     float emissive_count; // offset 20, size 4 (原_pad1改为emissive三角形数量)
//     vec2 viewport_size; // offset 24, size 8
//     float fov;          // offset 32, size 4
//     float frame_index;  // offset 36, size 4 (帧累积索引，相机移动时重置为0)
// };
// ```
//
// **对齐验证**:
// - camera_pos(vec3): offset 0, size 12 ✓
// - camera_yaw(float): offset 12, size 4 ✓ (填充vec3空隙)
// - camera_pitch(float): offset 16, size 4 ✓
// - emissive_count(float): offset 20, size 4 ✓ (原_pad1字段，布局不变)
// - viewport_size(vec2): offset 24, alignment 8 → 24 % 8 == 0 ✓
// - fov(float): offset 32, size 4 ✓
// - frame_index(float): offset 36, size 4 ✓ (原_pad2字段，布局不变)
// - 总计: 40 bytes, struct alignment 8 (vec2最大成员), 40 % 8 == 0 ✓
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct RayTracePushConstant {
    /// 摄像机世界坐标位置 [x, y, z] — GLSL vec3 camera_pos
    /// offset 0-11, 12 bytes
    pub camera_pos: [f32; 3],
    /// 摄像机yaw角度（弧度） — GLSL float camera_yaw
    /// offset 12-15, 4 bytes (填充vec3→vec4空隙)
    pub camera_yaw: f32,
    /// 摄像机pitch角度（弧度） — GLSL float camera_pitch
    /// offset 16-19, 4 bytes
    pub camera_pitch: f32,
    /// emissive三角形数量 — GLSL float emissive_count
    /// offset 20-23, 4 bytes (原_pad1字段，用于路径追踪NEE)
    pub emissive_count: f32,
    /// 视口尺寸（像素） — GLSL vec2 viewport_size
    /// offset 24-31, 8 bytes
    pub viewport_size: [f32; 2],
    /// 视场角（弧度） — GLSL float fov
    /// offset 32-35, 4 bytes
    pub fov: f32,
    /// 帧累积索引 — GLSL float frame_index
    /// offset 36-39, 4 bytes (原_pad2字段，相机移动时重置为0)
    pub frame_index: f32,
}

/// RayTracePipeline — ray tracing compute shader管线
///
/// 持有compute pipeline、descriptor layouts/pool/sets、
/// storage buffers（BVH + vertices + indices + light）、
/// output storage image、preview sampler等GPU资源。
///
/// BVH数据在prepare阶段从scene mesh数据构建并上传到GPU。
/// record阶段绑定compute pipeline并dispatch。
/// 输出直接写入storage image（不需要offscreen FBO）。
pub struct RayTracePipeline {
    // === Vulkan handles ===
    /// Compute pipeline
    compute_pipeline: vk::Pipeline,
    /// Compute pipeline layout（push constant + descriptor set布局）
    compute_pipeline_layout: vk::PipelineLayout,
    /// Vulkan device（用于cmd_*操作和资源分配）
    device: ash::Device,

    // === Descriptor layouts ===
    /// Scene descriptor set layout (set 0: BVH + vertices + indices + light)
    scene_descriptor_set_layout: vk::DescriptorSetLayout,
    /// Output descriptor set layout (set 2: output image)
    output_descriptor_set_layout: vk::DescriptorSetLayout,

    // === Descriptor pool + sets ===
    /// Descriptor pool（分配scene + output descriptor sets）
    descriptor_pool: vk::DescriptorPool,
    /// Scene descriptor set (BVH + vertices + indices + light)
    scene_descriptor_set: vk::DescriptorSet,
    /// Output descriptor set (output image)
    output_descriptor_set: vk::DescriptorSet,

    // === Storage buffers ===
    /// BVH节点SSBO
    bvh_buffer: vk::Buffer,
    /// BVH buffer内存
    bvh_buffer_memory: vk::DeviceMemory,
    /// 顶点位置SSBO (vec4[]: position.xyz + pad)
    vertex_storage_buffer: vk::Buffer,
    /// 顶点buffer内存
    vertex_storage_buffer_memory: vk::DeviceMemory,
    /// 索引SSBO (uint[])
    index_storage_buffer: vk::Buffer,
    /// 索引buffer内存
    index_storage_buffer_memory: vk::DeviceMemory,
    /// 光照数据SSBO (vec4[]: 2 vec4s)
    light_ubo: vk::Buffer,
    /// 光照buffer内存
    light_ubo_memory: vk::DeviceMemory,
    /// 顶点颜色SSBO (vec4[]: color.rgb + emissive_flag.a)
    color_storage_buffer: vk::Buffer,
    /// 颜色buffer内存
    color_storage_buffer_memory: vk::DeviceMemory,
    /// emissive三角形SSBO (EmissiveTriangle[]: 80 bytes each)
    emissive_storage_buffer: vk::Buffer,
    /// emissive buffer内存
    emissive_storage_buffer_memory: vk::DeviceMemory,

    // === Output storage image ===
    /// 输出storage image (rgba8)
    output_image: vk::Image,
    /// 输出image内存
    output_image_memory: vk::DeviceMemory,
    /// 输出image view
    output_image_view: vk::ImageView,

    // === Accumulation storage image (帧累积降噪) ===
    /// 累积storage image (rgba32f) — 存储帧累积的HDR颜色
    accumulation_image: vk::Image,
    /// 累积image内存
    accumulation_image_memory: vk::DeviceMemory,
    /// 累积image view
    accumulation_image_view: vk::ImageView,

    // === Preview (UI采样输出) ===
    /// Preview sampler（UI采样ray tracing输出）
    preview_sampler: vk::Sampler,
    /// Preview descriptor set（UI preview采样用）
    preview_descriptor_set: vk::DescriptorSet,
    /// Preview descriptor set layout
    preview_descriptor_set_layout: vk::DescriptorSetLayout,

    // === BVH数据缓存 ===
    /// BVH树结构（每帧prepare阶段重建）
    bvh_data: Bvh,
    /// 平铺顶点数据: [x,y,z,pad,x,y,z,pad,...] — vec4格式
    mesh_vertices_flat: Vec<f32>,
    /// 索引数据
    mesh_indices: Vec<u32>,
    /// 平铺颜色数据: [r,g,b,a,r,g,b,a,...] — vec4格式, alpha>1.5=emissive
    mesh_colors_flat: Vec<f32>,
    /// emissive三角形数据: EmissiveTriangle数组(80 bytes each)
    emissive_triangles: Vec<EmissiveTriangle>,
    /// emissive三角形数量
    emissive_count: u32,

    // === Buffer容量追踪 — 动态resize时只增不减 ===
    /// BVH buffer当前分配容量（字节）
    bvh_buffer_cap: usize,
    /// Vertex buffer当前分配容量（字节）
    vertex_buffer_cap: usize,
    /// Index buffer当前分配容量（字节）
    index_buffer_cap: usize,
    /// Color buffer当前分配容量（字节）
    color_buffer_cap: usize,
    /// Emissive buffer当前分配容量（字节）
    emissive_buffer_cap: usize,

    // === Resize support ===
    /// 物理设备内存属性（resize output_image和storage buffer时需要分配新memory）
    memory_properties: vk::PhysicalDeviceMemoryProperties,

    // === Layout transition ===
    /// 首帧标记：output_image刚创建或刚resize时，layout barrier必须用UNDEFINED作为old_layout
    /// 否则barrier误用SHADER_READ_ONLY_OPTIMAL → VK验证错误 → 崩溃
    first_frame: bool,

    // === 帧累积 ===
    /// 帧累积索引 — 每帧递增，相机移动或场景变化时重置为0
    frame_index: u32,
    /// 前一帧mesh数据签名 — 用于检测场景变化自动重置帧累积
    /// 存储: [顶点数量, v0.x, v0.y, v0.z, v_last.x, v_last.y, v_last.z]
    prev_mesh_signature: [f32; 7],

    // === 延迟销毁列表 — resize时不能立即销毁的旧output image资源 ===
    /// GPU可能还在使用旧image（通过descriptor set引用），必须延迟到确认GPU空闲后再销毁
    /// 通过drain_deferred_destroys()转移到ui_vulkan_renderer的统一延迟销毁列表
    deferred_destroy_image_views: Vec<vk::ImageView>,
    deferred_destroy_images: Vec<vk::Image>,
    deferred_destroy_memories: Vec<vk::DeviceMemory>,
}

impl RayTracePipeline {
    /// 创建RayTracePipeline
    ///
    /// 需要Vulkan device、物理设备内存属性、视口尺寸。
    /// 创建compute pipeline、descriptor layouts/pool/sets、
    /// storage buffers、output image、preview sampler等所有GPU资源。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: ash::Device,
        memory_properties: vk::PhysicalDeviceMemoryProperties,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Self {
        // 1. 读取raytrace.comp.spv字节
        let spirv_bytes = Self::load_spirv();
        // 2. 创建shader module
        let shader_module = unsafe {
            device.create_shader_module(
                &vk::ShaderModuleCreateInfo {
                    code_size: spirv_bytes.len(),
                    p_code: spirv_bytes.as_ptr() as *const u32,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建shader module失败")
        };
        // 3. 创建descriptor set layouts
        let scene_descriptor_set_layout = Self::create_scene_descriptor_set_layout(&device);
        let output_descriptor_set_layout = Self::create_output_descriptor_set_layout(&device);
        let preview_descriptor_set_layout = Self::create_preview_descriptor_set_layout(&device);
        // 4. 创建pipeline layout (push constant range + descriptor set layouts)
        let push_constant_range = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: 40, // 40 bytes, 严格匹配GLSL std430
        };

        // set 0: scene data, set 1: 空（与raster pipeline兼容），set 2: output image
        let empty_set_layout = Self::create_empty_descriptor_set_layout(&device);

        let set_layouts = [
            scene_descriptor_set_layout,   // set 0
            empty_set_layout,              // set 1 (空)
            output_descriptor_set_layout,  // set 2
        ];

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    set_layout_count: set_layouts.len() as u32,
                    p_set_layouts: &set_layouts as *const _,
                    push_constant_range_count: 1,
                    p_push_constant_ranges: &[push_constant_range] as *const _,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建pipeline layout失败")
        };
        // 5. 创建compute pipeline
        let compute_pipeline = unsafe {
            let pipeline_info = vk::ComputePipelineCreateInfo {
                stage: vk::PipelineShaderStageCreateInfo {
                    stage: vk::ShaderStageFlags::COMPUTE,
                    module: shader_module,
                    p_name: c"main".as_ptr(),
                    ..Default::default()
                },
                layout: pipeline_layout,
                ..Default::default()
            };

            device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ).expect("RayTracePipeline: 创建compute pipeline失败")[0]
        };
        // 6. 创建storage buffers
        unsafe { device.destroy_shader_module(shader_module, None); }

        // 6. 创建storage buffers (BVH + vertices + indices + light)
        // 初始数据为空，由collect_mesh_data在prepare时根据scene动态填充
        let bvh = Bvh::empty();
        let initial_bvh_size = bvh.nodes_byte_size();
        let initial_vertex_count = 0;
        let initial_index_count = 0;

        let bvh_buffer_cap = initial_bvh_size.max(4);
        let vertex_buffer_cap = (initial_vertex_count * 16).max(16);
        let index_buffer_cap = (initial_index_count * 4).max(4);
        let color_buffer_cap = (initial_vertex_count * 16).max(16);
        let emissive_buffer_cap = 16; // 初始16字节，动态增长

        let (bvh_buffer, bvh_buffer_memory) = Self::create_storage_buffer(
            &device, &memory_properties, bvh_buffer_cap,
        );
        let (vertex_storage_buffer, vertex_storage_buffer_memory) = Self::create_storage_buffer(
            &device, &memory_properties, vertex_buffer_cap,
        );
        let (index_storage_buffer, index_storage_buffer_memory) = Self::create_storage_buffer(
            &device, &memory_properties, index_buffer_cap,
        );
        let (light_ubo, light_ubo_memory) = Self::create_storage_buffer(
            &device, &memory_properties, 32,
        );
        let (color_storage_buffer, color_storage_buffer_memory) = Self::create_storage_buffer(
            &device, &memory_properties, color_buffer_cap,
        );
        let (emissive_storage_buffer, emissive_storage_buffer_memory) = Self::create_storage_buffer(
            &device, &memory_properties, emissive_buffer_cap,
        );

        // 7. 创建output storage image (rgba8)
        let (output_image, output_image_memory, output_image_view) =
            Self::create_output_image(&device, &memory_properties, viewport_width, viewport_height);

        // 7b. 创建accumulation storage image (rgba32f) — 帧累积降噪缓冲区
        let (accumulation_image, accumulation_image_memory, accumulation_image_view) =
            Self::create_accumulation_image(&device, &memory_properties, viewport_width, viewport_height);

        // 8. 创建descriptor pool + allocate descriptor sets
        let descriptor_pool = Self::create_descriptor_pool(&device);
        let (scene_descriptor_set, output_descriptor_set, preview_descriptor_set) =
            Self::allocate_descriptor_sets(
                &device,
                descriptor_pool,
                scene_descriptor_set_layout,
                output_descriptor_set_layout,
                preview_descriptor_set_layout,
            );

        // 9. 更新descriptor sets — 绑定buffers和image
        Self::update_scene_descriptor_set(
            &device,
            scene_descriptor_set,
            bvh_buffer,
            bvh_buffer_cap,
            vertex_storage_buffer,
            vertex_buffer_cap,
            index_storage_buffer,
            index_buffer_cap,
            light_ubo,
            32,
            color_storage_buffer,
            color_buffer_cap,
            emissive_storage_buffer,
            emissive_buffer_cap,
        );
        Self::update_output_descriptor_set(
            &device,
            output_descriptor_set,
            output_image_view,
            accumulation_image_view,
        );

        // 10. 创建preview sampler + 更新preview descriptor set
        let preview_sampler = Self::create_preview_sampler(&device);
        Self::update_preview_descriptor_set(
            &device,
            preview_descriptor_set,
            output_image_view,
            preview_sampler,
        );

        // 11. 准备flat vertex数据（空初始数据，由collect_mesh_data动态填充）
        let mesh_vertices_flat: Vec<f32> = Vec::new();
        let mesh_indices: Vec<u32> = Vec::new();
        let mesh_colors_flat: Vec<f32> = Vec::new();
        let emissive_triangles: Vec<EmissiveTriangle> = Vec::new();
        let emissive_count: u32 = 0;
        let color_bytes = bytemuck::cast_slice(&mesh_colors_flat);
        Self::upload_to_buffer(&device, color_storage_buffer_memory, color_bytes);
        // 上传初始emissive数据（空）
        let emissive_bytes = bytemuck::cast_slice(&emissive_triangles);
        Self::upload_to_buffer(&device, emissive_storage_buffer_memory, emissive_bytes);

        Self {
            compute_pipeline,
            compute_pipeline_layout: pipeline_layout,
            device,
            scene_descriptor_set_layout,
            output_descriptor_set_layout,
            descriptor_pool,
            scene_descriptor_set,
            output_descriptor_set,
            bvh_buffer,
            bvh_buffer_memory,
            vertex_storage_buffer,
            vertex_storage_buffer_memory,
            index_storage_buffer,
            index_storage_buffer_memory,
            light_ubo,
            light_ubo_memory,
            color_storage_buffer,
            color_storage_buffer_memory,
            emissive_storage_buffer,
            emissive_storage_buffer_memory,
            output_image,
            output_image_memory,
            output_image_view,
            accumulation_image,
            accumulation_image_memory,
            accumulation_image_view,
            preview_sampler,
            preview_descriptor_set,
            preview_descriptor_set_layout,
            bvh_data: bvh,
            mesh_vertices_flat,
            mesh_indices,
            mesh_colors_flat,
            emissive_triangles,
            emissive_count,
            bvh_buffer_cap,
            vertex_buffer_cap,
            index_buffer_cap,
            color_buffer_cap,
            emissive_buffer_cap,
            memory_properties,
            first_frame: true,  // 初始帧output_image从UNDEFINED layout开始
            frame_index: 0,    // 帧累积索引，初始为0
            prev_mesh_signature: [0.0; 7],  // 前一帧mesh签名，初始为0
            deferred_destroy_image_views: Vec::new(),
            deferred_destroy_images: Vec::new(),
            deferred_destroy_memories: Vec::new(),
        }
    }

    /// Resize output storage image to match new viewport dimensions
    ///
    /// **重要**: 旧资源不再立即销毁，而是推入deferred列表，由调用方
    /// 通过drain_deferred_destroys()转移到ui_vulkan_renderer的统一延迟销毁机制。
    /// 这样避免了resize时的device_wait_idle stall，GPU资源在3帧后安全释放。
    pub fn resize_output_image(&mut self, width: u32, height: u32) {
        // 1. 将旧output image资源推入延迟销毁列表 — 不立即销毁
        // GPU可能还在通过旧descriptor set引用旧image，立即销毁可能导致崩溃
        self.deferred_destroy_image_views.push(self.output_image_view);
        self.deferred_destroy_images.push(self.output_image);
        self.deferred_destroy_memories.push(self.output_image_memory);

        // 1b. 将旧accumulation image资源推入延迟销毁列表
        self.deferred_destroy_image_views.push(self.accumulation_image_view);
        self.deferred_destroy_images.push(self.accumulation_image);
        self.deferred_destroy_memories.push(self.accumulation_image_memory);

        // 2. 创建新output image
        let (new_image, new_memory, new_view) = Self::create_output_image(
            &self.device, &self.memory_properties, width, height,
        );

        // 2b. 创建新accumulation image
        let (new_accum_image, new_accum_memory, new_accum_view) = Self::create_accumulation_image(
            &self.device, &self.memory_properties, width, height,
        );

        // 3. 更新struct字段
        self.output_image = new_image;
        self.output_image_memory = new_memory;
        self.output_image_view = new_view;
        self.accumulation_image = new_accum_image;
        self.accumulation_image_memory = new_accum_memory;
        self.accumulation_image_view = new_accum_view;

        // 4. 更新output descriptor set（compute shader的STORAGE_IMAGE — output + accumulation）
        Self::update_output_descriptor_set(
            &self.device, self.output_descriptor_set, self.output_image_view, self.accumulation_image_view,
        );

        // 5. 更新preview descriptor set（UI的COMBINED_IMAGE_SAMPLER）
        Self::update_preview_descriptor_set(
            &self.device, self.preview_descriptor_set, self.output_image_view, self.preview_sampler,
        );

        // 6. 标记first_frame + 重置frame_index：新image从UNDEFINED layout开始
        self.first_frame = true;
        self.frame_index = 0;
    }

    /// 取出所有延迟销毁资源 — 转移到ui_vulkan_renderer的统一延迟销毁列表
    ///
    /// 调用方在pipeline.resize()后调用此方法，将pipeline的deferred资源
    /// 推入renderer的帧级延迟销毁列表（附带退役帧号），
    /// 由flush_deferred_destroy_periodic()在3帧后安全释放。
    pub fn drain_deferred_destroys(&mut self) -> (Vec<vk::ImageView>, Vec<vk::Image>, Vec<vk::DeviceMemory>) {
        let ivs = self.deferred_destroy_image_views.drain(..).collect();
        let imgs = self.deferred_destroy_images.drain(..).collect();
        let mems = self.deferred_destroy_memories.drain(..).collect();
        (ivs, imgs, mems)
    }

    /// 重置帧累积 — 相机移动或场景变化时调用
    ///
    /// 将frame_index重置为0，并标记first_frame=true，
    /// 使accumulation buffer在下一帧从UNDEFINED layout开始（内容被清空）。
    /// shader中frame_index=0时，累积公式 (prev * 0 + new) / 1 = new，
    /// 即第一帧直接使用新采样值，不混合旧数据。
    pub fn reset_accumulation(&mut self) {
        self.frame_index = 0;
        self.first_frame = true;
    }
}

impl Drop for RayTracePipeline {
    /// 清理所有Vulkan资源 — pipeline销毁时必须释放GPU资源
    fn drop(&mut self) {
        unsafe {
            // Storage buffers + memory
            self.device.destroy_buffer(self.bvh_buffer, None);
            self.device.free_memory(self.bvh_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_storage_buffer, None);
            self.device.free_memory(self.vertex_storage_buffer_memory, None);
            self.device.destroy_buffer(self.index_storage_buffer, None);
            self.device.free_memory(self.index_storage_buffer_memory, None);
            self.device.destroy_buffer(self.light_ubo, None);
            self.device.free_memory(self.light_ubo_memory, None);
            self.device.destroy_buffer(self.color_storage_buffer, None);
            self.device.free_memory(self.color_storage_buffer_memory, None);
            self.device.destroy_buffer(self.emissive_storage_buffer, None);
            self.device.free_memory(self.emissive_storage_buffer_memory, None);

            // Output image + memory + view
            self.device.destroy_image_view(self.output_image_view, None);
            self.device.destroy_image(self.output_image, None);
            self.device.free_memory(self.output_image_memory, None);

            // Accumulation image + memory + view
            self.device.destroy_image_view(self.accumulation_image_view, None);
            self.device.destroy_image(self.accumulation_image, None);
            self.device.free_memory(self.accumulation_image_memory, None);

            // 延迟销毁列表中的残留资源（resize后未被drain转移的旧资源）
            for iv in &self.deferred_destroy_image_views {
                self.device.destroy_image_view(*iv, None);
            }
            for img in &self.deferred_destroy_images {
                self.device.destroy_image(*img, None);
            }
            for mem in &self.deferred_destroy_memories {
                self.device.free_memory(*mem, None);
            }

            // Preview sampler
            self.device.destroy_sampler(self.preview_sampler, None);

            // Descriptor layouts + pool
            self.device.destroy_descriptor_set_layout(self.scene_descriptor_set_layout, None);
            self.device.destroy_descriptor_set_layout(self.output_descriptor_set_layout, None);
            self.device.destroy_descriptor_set_layout(self.preview_descriptor_set_layout, None);
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);

            // Compute pipeline + layout
            self.device.destroy_pipeline(self.compute_pipeline, None);
            self.device.destroy_pipeline_layout(self.compute_pipeline_layout, None);
        }
    }
}

impl RayTracePipeline {
    /// 读取raytrace.comp.spv字节
    fn load_spirv() -> Vec<u8> {
        let spirv_path = std::path::Path::new("shaders/raytrace.comp.spv");
        std::fs::read(spirv_path)
            .expect("RayTracePipeline: 读取raytrace.comp.spv失败 — 请确保shader已编译")
    }

    /// 创建scene descriptor set layout (set 0)
    fn create_scene_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        let bindings = [
            // binding 0: BVH nodes SSBO
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
            // binding 1: vertices SSBO
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
            // binding 2: indices SSBO
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
            // binding 3: light data SSBO
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
            // binding 4: vertex colors SSBO
            vk::DescriptorSetLayoutBinding {
                binding: 4,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
            // binding 5: emissive triangles SSBO
            vk::DescriptorSetLayoutBinding {
                binding: 5,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
        ];

        unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo {
                    binding_count: bindings.len() as u32,
                    p_bindings: &bindings as *const _,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建scene descriptor set layout失败")
        }
    }

    /// 创建空descriptor set layout (set 1)
    fn create_empty_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo {
                    binding_count: 0,
                    p_bindings: &[] as *const _,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建空descriptor set layout失败")
        }
    }

    /// 创建output descriptor set layout (set 2: output image + accumulation image)
    fn create_output_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        let bindings = [
            // binding 0: output storage image (rgba8)
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
            // binding 1: accumulation storage image (rgba32f)
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
        ];

        unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo {
                    binding_count: bindings.len() as u32,
                    p_bindings: &bindings as *const _,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建output descriptor set layout失败")
        }
    }

    /// 创建preview descriptor set layout
    fn create_preview_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
        let bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                ..Default::default()
            },
        ];

        unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo {
                    binding_count: bindings.len() as u32,
                    p_bindings: &bindings as *const _,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建preview descriptor set layout失败")
        }
    }

    /// 创建storage buffer + 分配device memory
    fn create_storage_buffer(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        size: usize,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer = unsafe {
            device.create_buffer(
                &vk::BufferCreateInfo {
                    size: size as u64,
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::TRANSFER_DST,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建storage buffer失败")
        };

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type_index = Self::find_memory_type(
            memory_properties,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let buffer_memory = unsafe {
            device.allocate_memory(
                &vk::MemoryAllocateInfo {
                    allocation_size: memory_requirements.size,
                    memory_type_index,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 分配buffer memory失败")
        };

        unsafe {
            device.bind_buffer_memory(buffer, buffer_memory, 0)
                .expect("RayTracePipeline: 绑定buffer memory失败");
        }

        (buffer, buffer_memory)
    }

    /// 动态resize storage buffer — 当数据超出当前容量时重建
    ///
    /// 流程: device_wait_idle → destroy旧buffer+memory → create新buffer(2倍增长+最小64KB) → bind → 返回新handle
    /// 容量只增不减，避免反复resize。
    fn ensure_buffer_capacity(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        current_buffer: vk::Buffer,
        current_memory: vk::DeviceMemory,
        _current_cap: usize,
        required_size: usize,
    ) -> (vk::Buffer, vk::DeviceMemory, usize) {
        // 新大小: 2倍增长 + 最小64KB，避免频繁resize
        let new_size = (required_size * 2).max(65536);

        // AGENTS.md规则: destroy FBO/buffer前必须device_wait_idle
        unsafe { device.device_wait_idle().expect("RayTracePipeline: device_wait_idle失败") };

        // 销毁旧buffer和memory
        unsafe {
            device.destroy_buffer(current_buffer, None);
            device.free_memory(current_memory, None);
        }

        // 创建新buffer
        let (new_buffer, new_memory) = Self::create_storage_buffer(device, memory_properties, new_size);

        (new_buffer, new_memory, new_size)
    }

    /// 创建output storage image (rgba8)
    fn create_output_image(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        width: u32,
        height: u32,
    ) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
        let image = unsafe {
            device.create_image(
                &vk::ImageCreateInfo {
                    image_type: vk::ImageType::TYPE_2D,
                    format: vk::Format::R8G8B8A8_UNORM,
                    extent: vk::Extent3D { width, height, depth: 1 },
                    mip_levels: 1,
                    array_layers: 1,
                    samples: vk::SampleCountFlags::TYPE_1,
                    tiling: vk::ImageTiling::OPTIMAL,
                    usage: vk::ImageUsageFlags::STORAGE
                        | vk::ImageUsageFlags::SAMPLED
                        | vk::ImageUsageFlags::TRANSFER_SRC,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建output image失败")
        };

        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
        let memory_type_index = Self::find_memory_type(
            memory_properties,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let image_memory = unsafe {
            device.allocate_memory(
                &vk::MemoryAllocateInfo {
                    allocation_size: memory_requirements.size,
                    memory_type_index,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 分配output image memory失败")
        };

        unsafe {
            device.bind_image_memory(image, image_memory, 0)
                .expect("RayTracePipeline: 绑定output image memory失败");
        }

        let image_view = unsafe {
            device.create_image_view(
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
            ).expect("RayTracePipeline: 创建output image view失败")
        };

        (image, image_memory, image_view)
    }

    /// 创建accumulation storage image (rgba32f) — 帧累积降噪缓冲区
    fn create_accumulation_image(
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        width: u32,
        height: u32,
    ) -> (vk::Image, vk::DeviceMemory, vk::ImageView) {
        let image = unsafe {
            device.create_image(
                &vk::ImageCreateInfo {
                    image_type: vk::ImageType::TYPE_2D,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    extent: vk::Extent3D { width, height, depth: 1 },
                    mip_levels: 1,
                    array_layers: 1,
                    samples: vk::SampleCountFlags::TYPE_1,
                    tiling: vk::ImageTiling::OPTIMAL,
                    usage: vk::ImageUsageFlags::STORAGE
                        | vk::ImageUsageFlags::TRANSFER_SRC,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建accumulation image失败")
        };

        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
        let memory_type_index = Self::find_memory_type(
            memory_properties,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let image_memory = unsafe {
            device.allocate_memory(
                &vk::MemoryAllocateInfo {
                    allocation_size: memory_requirements.size,
                    memory_type_index,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 分配accumulation image memory失败")
        };

        unsafe {
            device.bind_image_memory(image, image_memory, 0)
                .expect("RayTracePipeline: 绑定accumulation image memory失败");
        }

        let image_view = unsafe {
            device.create_image_view(
                &vk::ImageViewCreateInfo {
                    image,
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: vk::Format::R32G32B32A32_SFLOAT,
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
            ).expect("RayTracePipeline: 创建accumulation image view失败")
        };

        (image, image_memory, image_view)
    }

    /// 创建descriptor pool
    fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 6,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 6,  // output + accumulation + preview + resize备用
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 4,
            },
        ];

        unsafe {
            device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo {
                    max_sets: 8,
                    pool_size_count: pool_sizes.len() as u32,
                    p_pool_sizes: &pool_sizes as *const _,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建descriptor pool失败")
        }
    }

    /// 分配descriptor sets (scene + output + preview)
    fn allocate_descriptor_sets(
        device: &ash::Device,
        descriptor_pool: vk::DescriptorPool,
        scene_layout: vk::DescriptorSetLayout,
        output_layout: vk::DescriptorSetLayout,
        preview_layout: vk::DescriptorSetLayout,
    ) -> (vk::DescriptorSet, vk::DescriptorSet, vk::DescriptorSet) {
        let layouts = [scene_layout, output_layout, preview_layout];

        let descriptor_sets = unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo {
                    descriptor_pool,
                    descriptor_set_count: layouts.len() as u32,
                    p_set_layouts: &layouts as *const _,
                    ..Default::default()
                },
            ).expect("RayTracePipeline: 分配descriptor sets失败")
        };

        (descriptor_sets[0], descriptor_sets[1], descriptor_sets[2])
    }

    /// 更新scene descriptor set — 绑定BVH + vertices + indices + light + colors + emissive buffers
    fn update_scene_descriptor_set(
        device: &ash::Device,
        descriptor_set: vk::DescriptorSet,
        bvh_buffer: vk::Buffer,
        bvh_size: usize,
        vertex_buffer: vk::Buffer,
        vertex_size: usize,
        index_buffer: vk::Buffer,
        index_size: usize,
        light_buffer: vk::Buffer,
        light_size: usize,
        color_buffer: vk::Buffer,
        color_size: usize,
        emissive_buffer: vk::Buffer,
        emissive_size: usize,
    ) {
        let buffer_infos = [
            vk::DescriptorBufferInfo { buffer: bvh_buffer, offset: 0, range: bvh_size as u64 },
            vk::DescriptorBufferInfo { buffer: vertex_buffer, offset: 0, range: vertex_size as u64 },
            vk::DescriptorBufferInfo { buffer: index_buffer, offset: 0, range: index_size as u64 },
            vk::DescriptorBufferInfo { buffer: light_buffer, offset: 0, range: light_size as u64 },
            vk::DescriptorBufferInfo { buffer: color_buffer, offset: 0, range: color_size as u64 },
            vk::DescriptorBufferInfo { buffer: emissive_buffer, offset: 0, range: emissive_size as u64 },
        ];

        let writes = [
            vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: descriptor_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_infos[0],
                p_texel_buffer_view: std::ptr::null(),
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: descriptor_set,
                dst_binding: 1,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_infos[1],
                p_texel_buffer_view: std::ptr::null(),
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: descriptor_set,
                dst_binding: 2,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_infos[2],
                p_texel_buffer_view: std::ptr::null(),
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: descriptor_set,
                dst_binding: 3,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_infos[3],
                p_texel_buffer_view: std::ptr::null(),
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: descriptor_set,
                dst_binding: 4,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_infos[4],
                p_texel_buffer_view: std::ptr::null(),
                ..Default::default()
            },
            // binding 5: emissive triangles SSBO
            vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: descriptor_set,
                dst_binding: 5,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_infos[5],
                p_texel_buffer_view: std::ptr::null(),
                ..Default::default()
            },
        ];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
    }

    /// 更新output descriptor set — 绑定output storage image + accumulation storage image
    fn update_output_descriptor_set(
        device: &ash::Device,
        descriptor_set: vk::DescriptorSet,
        output_image_view: vk::ImageView,
        accumulation_image_view: vk::ImageView,
    ) {
        let image_infos = [
            // binding 0: output image (rgba8)
            vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::GENERAL,
                image_view: output_image_view,
                sampler: vk::Sampler::null(),
            },
            // binding 1: accumulation image (rgba32f)
            vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::GENERAL,
                image_view: accumulation_image_view,
                sampler: vk::Sampler::null(),
            },
        ];

        let writes = [
            vk::WriteDescriptorSet {
                dst_set: descriptor_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: &image_infos[0],
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_set,
                dst_binding: 1,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: &image_infos[1],
                ..Default::default()
            },
        ];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
    }

    /// 更新preview descriptor set — 绑定combined image sampler
    fn update_preview_descriptor_set(
        device: &ash::Device,
        descriptor_set: vk::DescriptorSet,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
    ) {
        let image_info = [
            vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view,
                sampler,
            },
        ];

        let write_descriptor = vk::WriteDescriptorSet {
            dst_set: descriptor_set,
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            p_image_info: &image_info[0],
            ..Default::default()
        };

        unsafe {
            device.update_descriptor_sets(&[write_descriptor], &[]);
        }
    }

    /// 创建preview sampler
    fn create_preview_sampler(device: &ash::Device) -> vk::Sampler {
        unsafe {
            device.create_sampler(
                &vk::SamplerCreateInfo {
                    mag_filter: vk::Filter::LINEAR,
                    min_filter: vk::Filter::LINEAR,
                    mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                    address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    min_lod: 0.0,
                    max_lod: 0.0,
                    ..Default::default()
                },
                None,
            ).expect("RayTracePipeline: 创建preview sampler失败")
        }
    }

    /// 查找合适的memory type index
    fn find_memory_type(
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        memory_type_bits: u32,
        flags: vk::MemoryPropertyFlags,
    ) -> u32 {
        for i in 0..memory_properties.memory_type_count {
            if (memory_type_bits & (1 << i)) != 0
                && (memory_properties.memory_types[i as usize].property_flags & flags) == flags
            {
                return i;
            }
        }
        panic!("RayTracePipeline: 找不到合适的memory type");
    }

    /// 将MeshData的vertices平铺为vec4格式: [x,y,z,pad,x,y,z,pad,...]
    fn flatten_vertices(mesh: &MeshData) -> Vec<f32> {
        let mut flat = Vec::with_capacity(mesh.vertices.len() * 4);
        for vertex in &mesh.vertices {
            flat.push(vertex.position[0]);
            flat.push(vertex.position[1]);
            flat.push(vertex.position[2]);
            flat.push(0.0); // padding
        }
        flat
    }

    /// 将MeshData的vertex颜色平铺为vec4格式: [r,g,b,a,r,g,b,a,...]
    /// alpha > 1.5 表示emissive材质
    fn flatten_colors(mesh: &MeshData) -> Vec<f32> {
        let mut flat = Vec::with_capacity(mesh.vertices.len() * 4);
        for vertex in &mesh.vertices {
            flat.push(vertex.color[0]);
            flat.push(vertex.color[1]);
            flat.push(vertex.color[2]);
            flat.push(vertex.color[3]); // alpha: >1.5=emissive
        }
        flat
    }

    /// 上传数据到storage buffer（HOST_VISIBLE + HOST_COHERENT）
    fn upload_to_buffer(
        device: &ash::Device,
        buffer_memory: vk::DeviceMemory,
        data: &[u8],
    ) {
        unsafe {
            let data_ptr = device.map_memory(
                buffer_memory,
                0,
                data.len() as u64,
                vk::MemoryMapFlags::empty(),
            ).expect("RayTracePipeline: map_memory失败");
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                data_ptr as *mut u8,
                data.len(),
            );
            device.unmap_memory(buffer_memory);
        }
    }

    /// 收集场景中的light数据
    fn collect_light_data(&self, ctx: &RenderContext) -> LightUboData {
        if ctx.scene.is_null() {
            return LightUboData {
                direction: [-0.5, -1.0, -0.5],
                _pad0: 0.0,
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            };
        }

        let scene = unsafe { &*ctx.scene };
        let mut found_light = None;
        for entity in &scene.root_entities {
            if let Some(light_comp) = scene.world.get_component::<DirectionalLightComponent>(*entity) {
                found_light = Some(light_comp);
                break;
            }
        }

        if let Some(light_comp) = found_light {
            LightUboData {
                direction: [light_comp.direction.x, light_comp.direction.y, light_comp.direction.z],
                _pad0: 0.0,
                color: [light_comp.color.x, light_comp.color.y, light_comp.color.z],
                intensity: light_comp.intensity,
            }
        } else {
            LightUboData {
                direction: [-0.5, -1.0, -0.5],
                _pad0: 0.0,
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            }
        }
    }

    /// 收集场景中的mesh数据并构建BVH
    fn collect_mesh_data(&mut self, ctx: &RenderContext) {
        self.mesh_vertices_flat.clear();
        self.mesh_indices.clear();
        self.mesh_colors_flat.clear();
        self.emissive_triangles.clear();
        self.emissive_count = 0;

        if ctx.scene.is_null() {
            // Scene为空时不渲染任何几何体 — 不使用bunny回退
            self.bvh_data = Bvh::empty();
            return;
        }

        let scene = unsafe { &*ctx.scene };
        let mut found_mesh = false;

        for entity in &scene.root_entities {
            let renderable_opt = scene.world.get_component::<RenderableComponent>(*entity);
            if renderable_opt.is_none() { continue; }
            let renderable = renderable_opt.unwrap();
            if !renderable.visible { continue; }

            let mesh_type = match renderable.mesh_path.as_str() {
                "builtin://cube" => MeshType::Cube,
                "builtin://sphere" => MeshType::Sphere,
                "builtin://plane" => MeshType::Plane,
                "builtin://cylinder" => MeshType::Cylinder,
                "builtin://cone" => MeshType::Cone,
                "builtin://bunny" => MeshType::Bunny,
                "builtin://cornell_box" => MeshType::CornellBox,
                _ => MeshType::Custom, // asset://xxx路径 → Custom mesh
            };

            let mesh_data = match mesh_type {
                MeshType::Cube => MeshData::create_cube(),
                MeshType::Sphere => MeshData::create_sphere(),
                MeshType::Plane => MeshData::create_plane(),
                MeshType::Cylinder => MeshData::create_cylinder(),
                MeshType::Cone => MeshData::create_cone(),
                MeshType::Bunny => MeshData::create_bunny(),
                MeshType::CornellBox => MeshData::create_cornell_box(),
                MeshType::Custom => {
                    // Custom mesh: 从OBJ文件加载，不再fallback为cube
                    // mesh_path格式为"asset://path/to/model.obj"，需提取实际文件路径
                    let obj_path = renderable.mesh_path.strip_prefix("asset://")
                        .unwrap_or(&renderable.mesh_path);
                    // 直接使用tobj加载OBJ文件（render-pipeline有自己的tobj依赖）
                    match tobj::load_obj(obj_path, &tobj::LoadOptions {
                        single_index: true,
                        triangulate: true,
                        ignore_lines: true,
                        ignore_points: true,
                    }) {
                        Ok((models, _materials)) => {
                            if models.is_empty() {
                                eprintln!("RayTrace: OBJ文件{}无模型数据", obj_path);
                                MeshData::new(Vec::new(), Vec::new())
                            } else {
                                // 使用第一个模型
                                let mesh = &models[0].mesh;
                                let vertices: Vec<hezhou_geometry::Vertex> = (0..mesh.positions.len() / 3)
                                    .map(|i| hezhou_geometry::Vertex::new([
                                        mesh.positions[i * 3],
                                        mesh.positions[i * 3 + 1],
                                        mesh.positions[i * 3 + 2],
                                    ]))
                                    .collect();
                                let indices: Vec<u32> = mesh.indices.iter().map(|i| *i as u32).collect();
                                MeshData::new(vertices, indices)
                            }
                        }
                        Err(e) => {
                            // OBJ加载失败时使用空mesh，不fallback为cube
                            eprintln!("RayTrace: Custom mesh加载失败: {} — {}", obj_path, e);
                            MeshData::new(Vec::new(), Vec::new())
                        }
                    }
                }
            };

            let transform_opt = scene.world.get_component::<LocalTransform>(*entity);
            let transform = transform_opt.unwrap_or_default();

            // 应用world transform: Mat4 * Vec4(x,y,z,1.0)
            let model_matrix = Mat4::translate(transform.position)
                * Mat4::from_quaternion(transform.rotation)
                * Mat4::scale(transform.scale);

            let base_vertex_count = self.mesh_vertices_flat.len() / 4;
            for vertex in &mesh_data.vertices {
                let pos_v4 = Vec4::new(vertex.position[0], vertex.position[1], vertex.position[2], 1.0);
                let transformed = model_matrix * pos_v4;
                self.mesh_vertices_flat.push(transformed.x);
                self.mesh_vertices_flat.push(transformed.y);
                self.mesh_vertices_flat.push(transformed.z);
                self.mesh_vertices_flat.push(0.0);
                // 颜色数据: RGBA vec4, alpha>1.5=emissive
                self.mesh_colors_flat.push(vertex.color[0]);
                self.mesh_colors_flat.push(vertex.color[1]);
                self.mesh_colors_flat.push(vertex.color[2]);
                self.mesh_colors_flat.push(vertex.color[3]);
            }

            for idx in &mesh_data.indices {
                self.mesh_indices.push(*idx + base_vertex_count as u32);
            }

            found_mesh = true;
        }

        if !found_mesh {
            // 没有可渲染的entity时不显示任何几何体 — 不使用bunny回退
            // mesh_vertices_flat/mesh_indices/mesh_colors_flat已在上方clear
        }

        // 构建BVH
        if !self.mesh_indices.is_empty() {
            let vertices: Vec<hezhou_geometry::Vertex> = self.mesh_vertices_flat
                .chunks(4)
                .map(|chunk| hezhou_geometry::Vertex::new([chunk[0], chunk[1], chunk[2]]))
                .collect();
            let bvh_mesh = MeshData::new(vertices, self.mesh_indices.clone());
            self.bvh_data = Bvh::build(&bvh_mesh);
        } else {
            self.bvh_data = Bvh::empty();
        }

        // 收集emissive三角形 — 遍历mesh_indices，每3个索引一个三角形
        // 检查3个顶点的alpha(w分量)，如果任何一个>1.5 → emissive三角形
        // 计算三角形面积、面法线、平均颜色，构建EmissiveTriangle数据
        let tri_count = self.mesh_indices.len() / 3;
        for tri_idx in 0..tri_count {
            let i0 = self.mesh_indices[tri_idx * 3] as usize;
            let i1 = self.mesh_indices[tri_idx * 3 + 1] as usize;
            let i2 = self.mesh_indices[tri_idx * 3 + 2] as usize;

            // 检查emissive标志: alpha > 1.5
            let a0 = self.mesh_colors_flat[i0 * 4 + 3];
            let a1 = self.mesh_colors_flat[i1 * 4 + 3];
            let a2 = self.mesh_colors_flat[i2 * 4 + 3];
            if a0 <= 1.5 && a1 <= 1.5 && a2 <= 1.5 {
                continue; // 不是emissive三角形，跳过
            }

            // 获取三角形3个顶点位置
            let v0 = [
                self.mesh_vertices_flat[i0 * 4],
                self.mesh_vertices_flat[i0 * 4 + 1],
                self.mesh_vertices_flat[i0 * 4 + 2],
            ];
            let v1 = [
                self.mesh_vertices_flat[i1 * 4],
                self.mesh_vertices_flat[i1 * 4 + 1],
                self.mesh_vertices_flat[i1 * 4 + 2],
            ];
            let v2 = [
                self.mesh_vertices_flat[i2 * 4],
                self.mesh_vertices_flat[i2 * 4 + 1],
                self.mesh_vertices_flat[i2 * 4 + 2],
            ];

            // 计算三角形面积 = 0.5 * |cross(v1-v0, v2-v0)|
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cross = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            let area = 0.5 * f32::sqrt(cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]);

            // 计算面法线 = normalize(cross(v1-v0, v2-v0))
            let normal_len = f32::sqrt(cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]);
            let normal = if normal_len > 1e-6 {
                [cross[0] / normal_len, cross[1] / normal_len, cross[2] / normal_len]
            } else {
                [0.0, 1.0, 0.0] // fallback
            };

            // 取3个顶点颜色的平均值作为光源颜色
            let c0 = [
                self.mesh_colors_flat[i0 * 4],
                self.mesh_colors_flat[i0 * 4 + 1],
                self.mesh_colors_flat[i0 * 4 + 2],
            ];
            let c1 = [
                self.mesh_colors_flat[i1 * 4],
                self.mesh_colors_flat[i1 * 4 + 1],
                self.mesh_colors_flat[i1 * 4 + 2],
            ];
            let c2 = [
                self.mesh_colors_flat[i2 * 4],
                self.mesh_colors_flat[i2 * 4 + 1],
                self.mesh_colors_flat[i2 * 4 + 2],
            ];
            let avg_color = [
                (c0[0] + c1[0] + c2[0]) / 3.0,
                (c0[1] + c1[1] + c2[1]) / 3.0,
                (c0[2] + c1[2] + c2[2]) / 3.0,
            ];

            // emissive强度 = 8.0 (与现有emissive亮度一致)
            let intensity = 8.0;

            self.emissive_triangles.push(EmissiveTriangle {
                v0_area: [v0[0], v0[1], v0[2], area],
                v1_pad: [v1[0], v1[1], v1[2], 0.0],
                v2_pad: [v2[0], v2[1], v2[2], 0.0],
                normal_pad: [normal[0], normal[1], normal[2], 0.0],
                color_int: [avg_color[0], avg_color[1], avg_color[2], intensity],
            });
        }
        self.emissive_count = self.emissive_triangles.len() as u32;

        // Bug3修复: 如果场景有DirectionalLight但没有emissive三角形，
        // 为方向光创建一个远处的emissive面片作为NEE光源。
        // 方向光没有mesh，NEE需要面积>0的emissive三角形才能采样光源。
        if self.emissive_count == 0 && !ctx.scene.is_null() {
            let scene = unsafe { &*ctx.scene };
            for entity in &scene.root_entities {
                if let Some(light_comp) = scene.world.get_component::<hezhou_core::DirectionalLightComponent>(*entity) {
                    // 在光源方向远处创建一个大面片（模拟太阳）
                    // direction是光照传播方向（从光源到场景），面片中心取反方向 = -direction * 100（太阳位置）
                    // 面片面积 = 50.0 * 50.0 = 2500.0（足够大，确保NEE命中率高）
                    let dir = light_comp.direction;
                    let dir_len = f32::sqrt(dir.x * dir.x + dir.y * dir.y + dir.z * dir.z);
                    let norm_dir = if dir_len > 1e-6 {
                        [dir.x / dir_len, dir.y / dir_len, dir.z / dir_len]
                    } else {
                        [-0.577, -0.577, -0.577] // fallback方向
                    };
                    // 面片中心 = -光照传播方向 * 100（光源所在方向远处）
                    // 默认direction=(-0.5,-1.0,-0.5)表示光从上方来，太阳位置在上方
                    let center = [-norm_dir[0] * 100.0, -norm_dir[1] * 100.0, -norm_dir[2] * 100.0];
                    let area = 2500.0; // 50x50面片

                    // 计算面片的两个垂直轴（垂直于光照方向）
                    // right = cross(norm_dir, world_up)，up = cross(right, norm_dir)
                    let world_up = if f32::abs(norm_dir[1]) < 0.99 {
                        [0.0, 1.0, 0.0]
                    } else {
                        [1.0, 0.0, 0.0] // 光照方向接近垂直时用world X
                    };
                    let right = [
                        norm_dir[1] * world_up[2] - norm_dir[2] * world_up[1],
                        norm_dir[2] * world_up[0] - norm_dir[0] * world_up[2],
                        norm_dir[0] * world_up[1] - norm_dir[1] * world_up[0],
                    ];
                    let right_len = f32::sqrt(right[0] * right[0] + right[1] * right[1] + right[2] * right[2]);
                    let norm_right = if right_len > 1e-6 {
                        [right[0] / right_len, right[1] / right_len, right[2] / right_len]
                    } else {
                        [1.0, 0.0, 0.0]
                    };
                    let up = [
                        norm_right[1] * norm_dir[2] - norm_right[2] * norm_dir[1],
                        norm_right[2] * norm_dir[0] - norm_right[0] * norm_dir[2],
                        norm_right[0] * norm_dir[1] - norm_right[1] * norm_dir[0],
                    ];

                    // 面片4个顶点（2个三角形）
                    let half_size = 25.0; // 面片半边长
                    let v0 = [center[0] - norm_right[0] * half_size - up[0] * half_size,
                              center[1] - norm_right[1] * half_size - up[1] * half_size,
                              center[2] - norm_right[2] * half_size - up[2] * half_size];
                    let v1 = [center[0] + norm_right[0] * half_size - up[0] * half_size,
                              center[1] + norm_right[1] * half_size - up[1] * half_size,
                              center[2] + norm_right[2] * half_size - up[2] * half_size];
                    let v2 = [center[0] - norm_right[0] * half_size + up[0] * half_size,
                              center[1] - norm_right[1] * half_size + up[1] * half_size,
                              center[2] - norm_right[2] * half_size + up[2] * half_size];
                    let v3 = [center[0] + norm_right[0] * half_size + up[0] * half_size,
                              center[1] + norm_right[1] * half_size + up[1] * half_size,
                              center[2] + norm_right[2] * half_size + up[2] * half_size];

                    // 面法线 = 光照传播方向norm_dir（面片发光面朝向场景方向）
                    // norm_dir指向光照传播方向（从太阳到场景），面片发射面朝此方向=朝向场景
                    let light_normal = [norm_dir[0], norm_dir[1], norm_dir[2]];
                    // 光源颜色和强度
                    let light_color = [light_comp.color.x, light_comp.color.y, light_comp.color.z];
                    let light_intensity = light_comp.intensity;

                    // Triangle 1: v0, v1, v2
                    self.emissive_triangles.push(EmissiveTriangle {
                        v0_area: [v0[0], v0[1], v0[2], area / 2.0],
                        v1_pad: [v1[0], v1[1], v1[2], 0.0],
                        v2_pad: [v2[0], v2[1], v2[2], 0.0],
                        normal_pad: [light_normal[0], light_normal[1], light_normal[2], 0.0],
                        color_int: [light_color[0], light_color[1], light_color[2], light_intensity],
                    });
                    // Triangle 2: v1, v3, v2
                    self.emissive_triangles.push(EmissiveTriangle {
                        v0_area: [v1[0], v1[1], v1[2], area / 2.0],
                        v1_pad: [v3[0], v3[1], v3[2], 0.0],
                        v2_pad: [v2[0], v2[1], v2[2], 0.0],
                        normal_pad: [light_normal[0], light_normal[1], light_normal[2], 0.0],
                        color_int: [light_color[0], light_color[1], light_color[2], light_intensity],
                    });
                    self.emissive_count = self.emissive_triangles.len() as u32;
                    break; // 只处理第一个DirectionalLight
                }
            }
        }
    }
}

impl RenderPipeline for RayTracePipeline {
    fn name(&self) -> &str {
        "ray_tracing"
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// 通过trait方法调用resize，避免downcast_mut路径
    fn resize(&mut self, width: u32, height: u32) {
        self.resize_output_image(width, height);
    }

    fn resource_requirements(&self) -> PipelineResourceDesc {
        PipelineResourceDesc {
            vertex_buffer_size: 0,
            index_buffer_size: 0,
            texture_count: 1,
            needs_depth_attachment: false,
            needs_offscreen_target: false,
            push_constant_size: 40,
            uniform_buffer_size: 0,
        }
    }

    fn prepare(&mut self, ctx: &RenderContext, _resources: &mut PipelineResources) {
        // 1. 收集mesh数据 + 构建BVH
        self.collect_mesh_data(ctx);

        // 1b. 检测mesh数据变化（实体transform变化等） — 自动重置帧累积
        let n = self.mesh_vertices_flat.len();
        let mut sig: [f32; 7] = [n as f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        if n >= 3 {
            sig[1] = self.mesh_vertices_flat[0]; // v0.x
            sig[2] = self.mesh_vertices_flat[1]; // v0.y
            sig[3] = self.mesh_vertices_flat[2]; // v0.z
            sig[4] = self.mesh_vertices_flat[n - 4]; // v_last.x
            sig[5] = self.mesh_vertices_flat[n - 3]; // v_last.y
            sig[6] = self.mesh_vertices_flat[n - 2]; // v_last.z
        }
        if sig != self.prev_mesh_signature {
            self.reset_accumulation();
            self.prev_mesh_signature = sig;
        }

        // 2. 收集light数据
        let light_data = self.collect_light_data(ctx);

        // 3. 计算各buffer所需大小
        let bvh_required = self.bvh_data.nodes_byte_size().max(4);
        let vertex_required = (self.mesh_vertices_flat.len() * 4).max(16);
        let index_required = (self.mesh_indices.len() * 4).max(4);
        let color_required = (self.mesh_colors_flat.len() * 4).max(16);
        let emissive_required = (self.emissive_triangles.len() * 80).max(16);

        // 4. 动态resize: 数据超出容量时重建buffer（容量只增不减）
        if bvh_required > self.bvh_buffer_cap {
            let (new_buf, new_mem, new_cap) = Self::ensure_buffer_capacity(
                &self.device, &self.memory_properties,
                self.bvh_buffer, self.bvh_buffer_memory,
                self.bvh_buffer_cap, bvh_required,
            );
            self.bvh_buffer = new_buf;
            self.bvh_buffer_memory = new_mem;
            self.bvh_buffer_cap = new_cap;
        }
        if vertex_required > self.vertex_buffer_cap {
            let (new_buf, new_mem, new_cap) = Self::ensure_buffer_capacity(
                &self.device, &self.memory_properties,
                self.vertex_storage_buffer, self.vertex_storage_buffer_memory,
                self.vertex_buffer_cap, vertex_required,
            );
            self.vertex_storage_buffer = new_buf;
            self.vertex_storage_buffer_memory = new_mem;
            self.vertex_buffer_cap = new_cap;
        }
        if index_required > self.index_buffer_cap {
            let (new_buf, new_mem, new_cap) = Self::ensure_buffer_capacity(
                &self.device, &self.memory_properties,
                self.index_storage_buffer, self.index_storage_buffer_memory,
                self.index_buffer_cap, index_required,
            );
            self.index_storage_buffer = new_buf;
            self.index_storage_buffer_memory = new_mem;
            self.index_buffer_cap = new_cap;
        }
        if color_required > self.color_buffer_cap {
            let (new_buf, new_mem, new_cap) = Self::ensure_buffer_capacity(
                &self.device, &self.memory_properties,
                self.color_storage_buffer, self.color_storage_buffer_memory,
                self.color_buffer_cap, color_required,
            );
            self.color_storage_buffer = new_buf;
            self.color_storage_buffer_memory = new_mem;
            self.color_buffer_cap = new_cap;
        }
        if emissive_required > self.emissive_buffer_cap {
            let (new_buf, new_mem, new_cap) = Self::ensure_buffer_capacity(
                &self.device, &self.memory_properties,
                self.emissive_storage_buffer, self.emissive_storage_buffer_memory,
                self.emissive_buffer_cap, emissive_required,
            );
            self.emissive_storage_buffer = new_buf;
            self.emissive_storage_buffer_memory = new_mem;
            self.emissive_buffer_cap = new_cap;
        }

        // 5. 上传数据到GPU storage buffers（resize后buffer handles已更新）
        let bvh_bytes = self.bvh_data.nodes_as_bytes();
        Self::upload_to_buffer(&self.device, self.bvh_buffer_memory, bvh_bytes);

        let vertex_bytes = bytemuck::cast_slice(&self.mesh_vertices_flat);
        Self::upload_to_buffer(&self.device, self.vertex_storage_buffer_memory, vertex_bytes);

        let index_bytes = bytemuck::cast_slice(&self.mesh_indices);
        Self::upload_to_buffer(&self.device, self.index_storage_buffer_memory, index_bytes);

        let light_array = [light_data];
        let light_bytes = bytemuck::cast_slice(&light_array);
        Self::upload_to_buffer(&self.device, self.light_ubo_memory, light_bytes);

        let color_bytes = bytemuck::cast_slice(&self.mesh_colors_flat);
        Self::upload_to_buffer(&self.device, self.color_storage_buffer_memory, color_bytes);

        let emissive_bytes = bytemuck::cast_slice(&self.emissive_triangles);
        Self::upload_to_buffer(&self.device, self.emissive_storage_buffer_memory, emissive_bytes);

        // 6. 更新descriptor sets（buffer handles和ranges可能因resize变化）
        Self::update_scene_descriptor_set(
            &self.device,
            self.scene_descriptor_set,
            self.bvh_buffer,
            bvh_required,
            self.vertex_storage_buffer,
            vertex_required,
            self.index_storage_buffer,
            index_required,
            self.light_ubo,
            32,
            self.color_storage_buffer,
            color_required,
            self.emissive_storage_buffer,
            emissive_required,
        );
    }

    fn record(
        &mut self,
        ctx: &RenderContext,
        _resources: &PipelineResources,
        cmd_buffer: vk::CommandBuffer,
    ) -> RenderPassOutput {
        let width = ctx.viewport_extent.width;
        let height = ctx.viewport_extent.height;
        let extent = vk::Extent2D { width, height };

        // 1. Pipeline barrier: HOST_WRITE → COMPUTE_READ
        let host_to_compute_barrier = vk::MemoryBarrier {
            src_access_mask: vk::AccessFlags::HOST_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            ..Default::default()
        };

        // 2. Pipeline barrier: output image → GENERAL layout
        // 使用first_frame标记而非frame_index：resize后output_image从UNDEFINED开始，
        // 但frame_index不归零，所以必须用first_frame来正确处理layout transition
        let image_old_layout = if self.first_frame {
            vk::ImageLayout::UNDEFINED
        } else {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        };
        let image_src_stage = if self.first_frame {
            vk::PipelineStageFlags::TOP_OF_PIPE
        } else {
            vk::PipelineStageFlags::FRAGMENT_SHADER
        };
        let image_src_access = if self.first_frame {
            vk::AccessFlags::empty()
        } else {
            vk::AccessFlags::SHADER_READ
        };

        // accumulation image: 首帧从UNDEFINED→GENERAL，后续帧保持GENERAL
        let accum_old_layout = if self.first_frame {
            vk::ImageLayout::UNDEFINED
        } else {
            vk::ImageLayout::GENERAL
        };
        let accum_src_access = if self.first_frame {
            vk::AccessFlags::empty()
        } else {
            vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ
        };

        let output_image_barrier = vk::ImageMemoryBarrier {
            old_layout: image_old_layout,
            new_layout: vk::ImageLayout::GENERAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: self.output_image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: image_src_access,
            dst_access_mask: vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ,
            ..Default::default()
        };

        let accumulation_image_barrier = vk::ImageMemoryBarrier {
            old_layout: accum_old_layout,
            new_layout: vk::ImageLayout::GENERAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: self.accumulation_image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: accum_src_access,
            dst_access_mask: vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ,
            ..Default::default()
        };

        unsafe {
            // HOST→COMPUTE barrier + output image + accumulation image layout transition
            self.device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::HOST | image_src_stage,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[host_to_compute_barrier],
                &[],
                &[output_image_barrier, accumulation_image_barrier],
            );

            // 3. Bind compute pipeline — 使用COMPUTE bind point
            self.device.cmd_bind_pipeline(
                cmd_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline,
            );

            // 4. Bind descriptor sets (scene set 0 + output set 2)
            self.device.cmd_bind_descriptor_sets(
                cmd_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline_layout,
                0,
                &[self.scene_descriptor_set],
                &[],
            );
            self.device.cmd_bind_descriptor_sets(
                cmd_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline_layout,
                2,
                &[self.output_descriptor_set],
                &[],
            );

            // 5. Push constants: 40 bytes
            let push_data = RayTracePushConstant {
                camera_pos: ctx.camera.position,
                camera_yaw: ctx.camera.yaw,
                camera_pitch: ctx.camera.pitch,
                emissive_count: self.emissive_count as f32,
                viewport_size: [width as f32, height as f32],
                fov: ctx.camera.fov,
                frame_index: self.frame_index as f32,
            };
            self.device.cmd_push_constants(
                cmd_buffer,
                self.compute_pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                bytemuck::cast_slice(&[push_data]),
            );

            // 6. cmd_dispatch — 每个workgroup处理8×8像素
            let dispatch_x = (width + 7) / 8;
            let dispatch_y = (height + 7) / 8;
            self.device.cmd_dispatch(cmd_buffer, dispatch_x, dispatch_y, 1);

            // 7. Pipeline barrier: COMPUTE_WRITE → FRAGMENT_READ
            let compute_to_fragment_barrier = vk::MemoryBarrier {
                src_access_mask: vk::AccessFlags::SHADER_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };

            let output_to_shader_read_barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::GENERAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.output_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::SHADER_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };

            // accumulation image保持GENERAL layout — compute shader每帧读写它
            let accumulation_barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::GENERAL,
                new_layout: vk::ImageLayout::GENERAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.accumulation_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ,
                dst_access_mask: vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };

            self.device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[compute_to_fragment_barrier],
                &[],
                &[output_to_shader_read_barrier, accumulation_barrier],
            );
        }

        // 8. 重置first_frame标记（output_image已成功从UNDEFINED→GENERAL→SHADER_READ_ONLY）
        self.first_frame = false;

        // 9. 递增frame_index（帧累积降噪）
        self.frame_index += 1;

        // 9. 返回RenderPassOutput
        RenderPassOutput {
            color_image: self.output_image,
            color_image_view: self.output_image_view,
            extent,
            descriptor_set: self.preview_descriptor_set,
        }
    }

    fn post_process(
        &mut self,
        _ctx: &RenderContext,
        _resources: &PipelineResources,
        input: &RenderPassOutput,
        _cmd_buffer: vk::CommandBuffer,
    ) -> RenderPassOutput {
        input.clone()
    }

    fn composite(
        &mut self,
        _ctx: &RenderContext,
        _output: &RenderPassOutput,
        _cmd_buffer: vk::CommandBuffer,
    ) -> Option<vk::DescriptorSet> {
        Some(self.preview_descriptor_set)
    }

    // cleanup使用默认实现（空操作）
}