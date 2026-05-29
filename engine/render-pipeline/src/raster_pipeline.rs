//! RasterPipeline — 从 UIVulkanRenderer 提取的游戏光栅化渲染管线
//!
//! 实现 RenderPipeline trait 的5个hook点:
//! - prepare: 上传light UBO数据、更新纹理缓存、准备push constant数据
//! - record: 录制游戏渲染命令（offscreen render pass + entity遍历 + outline）
//! - post_process: FXAA后处理（全屏quad抗锯齿）
//! - composite: 返回preview_descriptor_set供UI采样
//! - cleanup: 空操作（默认实现）
//!
//! **重要约束**:
//! - push constant布局严格匹配GLSL std430/std140对齐规则（144 bytes）
//! - 所有Pipeline必须使用p_dynamic_state for VIEWPORT+SCISSOR
//! - RasterPipeline不直接创建Vulkan资源，只引用UIVulkanRenderer已创建的资源
//! - 实际Vulkan分配/销毁操作由rhi-vulkan层完成

use std::collections::HashMap;
use ash::vk;
use bytemuck::{Pod, Zeroable};
use hezhou_core::{
    LocalTransform, RenderableComponent, DirectionalLightComponent,
    MeshType, Mat4, Vec3,
};

use crate::pipeline::{RenderPipeline, RenderPassOutput};
use crate::context::{RenderContext, RenderMode};
use crate::resource::{PipelineResources, PipelineResourceDesc};

// === 辅助类型定义 ===

/// Primitive mesh的GPU buffer范围
///
/// 与rhi-vulkan/primitive_meshes.rs中的MeshRange对应，
/// 但增加了index字段以支持cmd_draw_indexed。
/// Primitive mesh使用cmd_draw（无index buffer），index字段为0。
#[derive(Clone, Copy, Debug, Default)]
pub struct PrimitiveMeshRange {
    /// 顶点偏移（vertex单位，非字节）
    pub vertex_offset: u32,
    /// 顶点数量
    pub vertex_count: u32,
    /// 索引偏移（index单位）— primitive mesh为0
    pub index_offset: u32,
    /// 索引数量 — primitive mesh为0
    pub index_count: u32,
}

/// 自定义OBJ模型的GPU buffer信息
///
/// 从UIVulkanRenderer的CustomMeshInfo提取，
/// 包含vertex/index偏移和纹理/材质参数。
#[derive(Clone, Debug)]
pub struct CustomMeshInfo {
    /// 在game_mesh_buffer中的偏移（vertex单位）
    pub vertex_offset: u32,
    /// 顶点数量
    pub vertex_count: u32,
    /// 在game_index_buffer中的偏移（index单位）
    pub index_offset: u32,
    /// 索引数量
    pub index_count: u32,
    /// 是否有纹理（push constant的has_texture标志）
    pub has_texture: bool,
    /// 纹理descriptor set（如果有）
    pub texture_descriptor_set: Option<vk::DescriptorSet>,
    /// 高光强度
    pub specular_strength: f32,
    /// 环境光强度
    pub ambient_strength: f32,
    /// 高光指数
    pub shininess: f32,
}

/// 纹理缓存条目
///
/// 从UIVulkanRenderer的TextureCacheEntry提取，
/// 存储已上传纹理的GPU资源handle。
#[derive(Clone, Debug)]
pub struct TextureCacheEntry {
    /// 纹理image
    pub image: vk::Image,
    /// 纹理image memory
    pub memory: vk::DeviceMemory,
    /// 纹理image view
    pub view: vk::ImageView,
    /// 纹理sampler
    pub sampler: vk::Sampler,
    /// 纹理descriptor set（用于shader采样）
    pub descriptor_set: vk::DescriptorSet,
}

/// Push Constant数据布局 — 144 bytes，严格匹配GLSL std430/std140对齐
///
/// GLSL PushConstants struct布局:
/// ```glsl
/// layout(push_constant) uniform PushConstants {
///     mat4 model;            // offset 0, size 64
///     vec3 outline_color;    // offset 64, size 12
///     float is_selected;     // offset 76, size 4
///     vec2 viewport_size;    // offset 80, size 8
///     vec2 _pad;             // offset 88, size 8 (对齐camera_pos到16字节边界)
///     vec3 camera_pos;       // offset 96, size 12
///     float camera_yaw;      // offset 108, size 4
///     float camera_pitch;    // offset 112, size 4
///     float _pad2;           // offset 116, size 4
///     float has_texture;     // offset 120, size 4
///     float specular_strength; // offset 124, size 4
///     float ambient_strength;  // offset 128, size 4
///     float shininess;       // offset 132, size 4
///     vec2 _end_pad;         // offset 136, size 8 (struct end padding to 144)
/// };
/// ```
///
/// **对齐验证**:
/// - model(mat4): offset 0, size 64 ✓
/// - outline_color(vec3): offset 64, alignment 16 → 64 % 16 == 0 ✓
/// - is_selected(float): offset 76, size 4 ✓
/// - viewport_size(vec2): offset 80, alignment 8 → 80 % 8 == 0 ✓
/// - _pad(vec2): offset 88, size 8 ✓
/// - camera_pos(vec3): offset 96, alignment 16 → 96 % 16 == 0 ✓
/// - camera_yaw(float): offset 108, size 4 ✓
/// - camera_pitch(float): offset 112, size 4 ✓
/// - _pad2(float): offset 116, size 4 ✓
/// - has_texture(float): offset 120, size 4 ✓
/// - specular_strength(float): offset 124, size 4 ✓
/// - ambient_strength(float): offset 128, size 4 ✓
/// - shininess(float): offset 132, size 4 ✓
/// - _end_pad(vec2): offset 136, size 8 ✓
/// - 总计: 144 bytes, struct alignment 16, 144 % 16 == 0 ✓
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PushConstantData {
    /// 模型矩阵 — column-major 4x4（Vulkan约定）
    /// offset 0-63, 4个vec4列
    pub model: [[f32; 4]; 4],
    /// outline颜色 + is_selected标志
    /// offset 64-79: outline_color(vec3, 12 bytes) + is_selected(float, 4 bytes)
    pub outline_color: [f32; 3],
    pub is_selected: f32,
    /// 视口尺寸（像素）
    /// offset 80-87: viewport_size(vec2, 8 bytes)
    pub viewport_size: [f32; 2],
    /// 对齐padding — 将camera_pos对齐到16字节边界
    /// offset 88-95: _pad(vec2, 8 bytes)
    pub _pad: [f32; 2],
    /// 摄像机位置 + yaw角度
    /// offset 96-107: camera_pos(vec3, 12 bytes) + camera_yaw(float, 4 bytes)
    pub camera_pos: [f32; 3],
    pub camera_yaw: f32,
    /// 摄像机pitch角度
    /// offset 112-115: camera_pitch(float, 4 bytes)
    pub camera_pitch: f32,
    /// 对齐padding
    /// offset 116-119: _pad2(float, 4 bytes)
    pub _pad2: f32,
    /// 纹理标志 + 材质参数
    /// offset 120-135: has_texture(4) + specular_strength(4) + ambient_strength(4) + shininess(4)
    pub has_texture: f32,
    pub specular_strength: f32,
    pub ambient_strength: f32,
    pub shininess: f32,
    /// struct end padding — 对齐到144 bytes
    /// offset 136-143: _end_pad(vec2, 8 bytes)
    pub _end_pad: [f32; 2],
}

/// Light UBO数据布局 — std430对齐，32 bytes
///
/// GLSL uniform buffer layout:
/// ```glsl
/// layout(std430, set=0, binding=0) uniform LightData {
///     vec3 direction;    // offset 0, size 12
///     float _pad;        // offset 12, size 4 (填充vec3→vec4)
///     vec3 color;        // offset 16, size 12
///     float intensity;   // offset 28, size 4 (填充vec3→vec4)
/// };
/// ```
///
/// **注意**: intensity在offset 28而非32（std430允许vec3后紧跟float填充）
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct LightUboData {
    /// 光照方向
    pub direction: [f32; 3],
    /// padding — 填充vec3→vec4
    pub _pad0: f32,
    /// 光照颜色
    pub color: [f32; 3],
    /// 光照强度 — std430允许填充vec3空隙
    pub intensity: f32,
}

// === 辅助函数 ===

/// 判断mesh_path是否为自定义mesh
///
/// 与rhi-vulkan/mesh_loader.rs的is_custom_mesh逻辑一致，
/// 但在render-pipeline crate中重新定义（不依赖rhi-vulkan）。
pub fn is_custom_mesh(mesh_path: &str) -> bool {
    mesh_path.starts_with("asset://") || mesh_path.starts_with("custom://")
}

/// 从MeshType的mesh_path字符串解析MeshType枚举
///
/// 与rhi-vulkan/primitive_meshes.rs的mesh_type_from_path逻辑一致。
pub fn mesh_type_from_path(path: &str) -> MeshType {
    match path {
        "builtin://cube" => MeshType::Cube,
        "builtin://sphere" => MeshType::Sphere,
        "builtin://plane" => MeshType::Plane,
        "builtin://cylinder" => MeshType::Cylinder,
        "builtin://cone" => MeshType::Cone,
        "builtin://bunny" => MeshType::Bunny,
        _ => MeshType::Custom,
    }
}

/// 计算模型矩阵 — 从LocalTransform构建TRS矩阵
///
/// 与UIVulkanRenderer::compute_model_matrix逻辑一致:
/// model = translate(position) * from_quaternion(rotation) * scale(scale)
fn compute_model_matrix(transform: &LocalTransform) -> [[f32; 4]; 4] {
    let t = Mat4::translate(transform.position);
    let r = Mat4::from_quaternion(transform.rotation);
    let s = Mat4::scale(transform.scale);
    let model = t * r * s;
    model.data
}

/// 计算缩放模型矩阵 — 用于outline渲染（放大1.05倍）
///
/// 与UIVulkanRenderer::compute_scaled_model_matrix逻辑一致:
/// model = translate(position) * from_quaternion(rotation) * scale(scale * factor)
fn compute_scaled_model_matrix(transform: &LocalTransform, scale_factor: f32) -> [[f32; 4]; 4] {
    let t = Mat4::translate(transform.position);
    let r = Mat4::from_quaternion(transform.rotation);
    let s = Mat4::scale(Vec3::new(
        transform.scale.x * scale_factor,
        transform.scale.y * scale_factor,
        transform.scale.z * scale_factor,
    ));
    let model = t * r * s;
    model.data
}

/// 构建PushConstantData — 从模型矩阵、摄像机参数、材质参数组装144字节push constant
fn build_push_constant(
    model: [[f32; 4]; 4],
    outline_color: [f32; 3],
    is_selected: bool,
    viewport_width: f32,
    viewport_height: f32,
    camera_pos: [f32; 3],
    camera_yaw: f32,
    camera_pitch: f32,
    has_texture: bool,
    specular_strength: f32,
    ambient_strength: f32,
    shininess: f32,
) -> PushConstantData {
    PushConstantData {
        model,
        outline_color,
        is_selected: if is_selected { 1.0 } else { 0.0 },
        viewport_size: [viewport_width, viewport_height],
        _pad: [0.0, 0.0],
        camera_pos,
        camera_yaw,
        camera_pitch,
        _pad2: 0.0,
        has_texture: if has_texture { 1.0 } else { 0.0 },
        specular_strength,
        ambient_strength,
        shininess,
        _end_pad: [0.0, 0.0],
    }
}

// === RasterPipeline ===

/// 光栅化渲染管线 — 从UIVulkanRenderer提取的游戏渲染逻辑
///
/// RasterPipeline持有pipeline对象（vk::Pipeline等），
/// 而buffer/image等大资源通过PipelineResources引用。
///
/// 在Phase 3集成时，UIVulkanRenderer会将game_pipeline/game_pipeline_layout/
/// outline_pipeline等传入RasterPipeline::new()，
/// 同时将buffer/image资源分配到PipelineResources中。
///
/// **字段分类**:
/// 1. Pipeline自有字段 — 在Phase 3从UIVulkanRenderer传入
/// 2. 动态数据字段 — 每帧prepare阶段更新
pub struct RasterPipeline {
    // === Pipeline自有字段（Phase 3从UIVulkanRenderer传入） ===

    /// 游戏渲染pipeline（entity mesh绘制）
    game_pipeline: vk::Pipeline,
    /// 游戏渲染pipeline layout（push constant + descriptor set布局）
    game_pipeline_layout: vk::PipelineLayout,
    /// 选中实体outline pipeline（CULL_FRONT + alpha blend + scale>1）
    outline_pipeline: vk::Pipeline,

    /// FXAA后处理pipeline
    fxaa_pipeline: vk::Pipeline,
    /// FXAA pipeline layout
    fxaa_pipeline_layout: vk::PipelineLayout,
    /// FXAA descriptor set layout
    fxaa_descriptor_set_layout: vk::DescriptorSetLayout,
    /// FXAA descriptor pool
    fxaa_descriptor_pool: vk::DescriptorPool,
    /// FXAA descriptor set（采样offscreen image）
    fxaa_descriptor_set: vk::DescriptorSet,
    /// FXAA sampler
    fxaa_sampler: vk::Sampler,

    /// 游戏render pass（game + FXAA共用）
    game_render_pass: vk::RenderPass,

    /// Light UBO buffer
    light_ubo: vk::Buffer,
    /// Light UBO memory
    light_ubo_memory: vk::DeviceMemory,
    /// Light descriptor set（set 0，光照数据）
    light_descriptor_set: vk::DescriptorSet,
    /// 默认纹理descriptor set（set 1，无纹理时的fallback）
    default_texture_descriptor_set: vk::DescriptorSet,

    /// 纹理descriptor set layout
    texture_descriptor_set_layout: vk::DescriptorSetLayout,
    /// 纹理descriptor pool
    texture_descriptor_pool: vk::DescriptorPool,
    /// Light descriptor set layout
    light_descriptor_set_layout: vk::DescriptorSetLayout,
    /// Light descriptor pool
    light_descriptor_pool: vk::DescriptorPool,

    /// 游戏mesh vertex buffer
    game_mesh_buffer: vk::Buffer,
    /// 游戏mesh vertex buffer memory
    game_mesh_buffer_memory: vk::DeviceMemory,
    /// 游戏mesh index buffer
    game_index_buffer: vk::Buffer,
    /// 游戏mesh index buffer memory
    game_index_buffer_memory: vk::DeviceMemory,

    /// FXAA offscreen image（FXAA输出目标）
    offscreen_fxaa_image: vk::Image,
    /// FXAA offscreen image memory
    offscreen_fxaa_image_memory: vk::DeviceMemory,
    /// FXAA offscreen image view
    offscreen_fxaa_image_view: vk::ImageView,
    /// FXAA offscreen framebuffer
    offscreen_fxaa_framebuffer: vk::Framebuffer,

    /// Preview descriptor set（UI采样游戏渲染结果）
    preview_descriptor_set: vk::DescriptorSet,
    /// Preview sampler
    preview_sampler: vk::Sampler,

    // === 动态数据字段（每帧prepare阶段更新） ===

    /// Primitive mesh的GPU buffer范围映射
    primitive_ranges: HashMap<MeshType, PrimitiveMeshRange>,
    /// 自定义mesh的GPU buffer信息映射
    custom_meshes: HashMap<String, CustomMeshInfo>,
    /// 纹理缓存（mesh_path → GPU资源）
    texture_cache: HashMap<String, TextureCacheEntry>,

    /// Vulkan device handle（用于cmd_bind等操作）
    device: ash::Device,
}

impl RasterPipeline {
    /// 创建RasterPipeline
    ///
    /// 在Phase 3集成时，UIVulkanRenderer调用此方法，
    /// 将已创建的pipeline和GPU资源传入。
    ///
    /// **注意**: 此构造函数参数较多，Phase 3集成时可能需要
    /// 改为builder模式或配置结构体。
    pub fn new(
        game_pipeline: vk::Pipeline,
        game_pipeline_layout: vk::PipelineLayout,
        outline_pipeline: vk::Pipeline,
        fxaa_pipeline: vk::Pipeline,
        fxaa_pipeline_layout: vk::PipelineLayout,
        fxaa_descriptor_set_layout: vk::DescriptorSetLayout,
        fxaa_descriptor_pool: vk::DescriptorPool,
        fxaa_descriptor_set: vk::DescriptorSet,
        fxaa_sampler: vk::Sampler,
        game_render_pass: vk::RenderPass,
        light_ubo: vk::Buffer,
        light_ubo_memory: vk::DeviceMemory,
        light_descriptor_set: vk::DescriptorSet,
        default_texture_descriptor_set: vk::DescriptorSet,
        texture_descriptor_set_layout: vk::DescriptorSetLayout,
        texture_descriptor_pool: vk::DescriptorPool,
        light_descriptor_set_layout: vk::DescriptorSetLayout,
        light_descriptor_pool: vk::DescriptorPool,
        game_mesh_buffer: vk::Buffer,
        game_mesh_buffer_memory: vk::DeviceMemory,
        game_index_buffer: vk::Buffer,
        game_index_buffer_memory: vk::DeviceMemory,
        offscreen_fxaa_image: vk::Image,
        offscreen_fxaa_image_memory: vk::DeviceMemory,
        offscreen_fxaa_image_view: vk::ImageView,
        offscreen_fxaa_framebuffer: vk::Framebuffer,
        preview_descriptor_set: vk::DescriptorSet,
        preview_sampler: vk::Sampler,
        device: ash::Device,
    ) -> Self {
        Self {
            game_pipeline,
            game_pipeline_layout,
            outline_pipeline,
            fxaa_pipeline,
            fxaa_pipeline_layout,
            fxaa_descriptor_set_layout,
            fxaa_descriptor_pool,
            fxaa_descriptor_set,
            fxaa_sampler,
            game_render_pass,
            light_ubo,
            light_ubo_memory,
            light_descriptor_set,
            default_texture_descriptor_set,
            texture_descriptor_set_layout,
            texture_descriptor_pool,
            light_descriptor_set_layout,
            light_descriptor_pool,
            game_mesh_buffer,
            game_mesh_buffer_memory,
            game_index_buffer,
            game_index_buffer_memory,
            offscreen_fxaa_image,
            offscreen_fxaa_image_memory,
            offscreen_fxaa_image_view,
            offscreen_fxaa_framebuffer,
            preview_descriptor_set,
            preview_sampler,
            primitive_ranges: HashMap::new(),
            custom_meshes: HashMap::new(),
            texture_cache: HashMap::new(),
            device,
        }
    }

    /// 更新动态数据 — 每帧由UIVulkanRenderer调用
    ///
    /// primitive_ranges/custom_meshes/texture_cache每帧可能变化
    /// （新entity创建、纹理加载、mesh上传等），需要从UIVulkanRenderer同步到RasterPipeline。
    pub fn update_dynamic_data(
        &mut self,
        primitive_ranges: HashMap<MeshType, PrimitiveMeshRange>,
        custom_meshes: HashMap<String, CustomMeshInfo>,
        texture_cache: HashMap<String, TextureCacheEntry>,
    ) {
        self.primitive_ranges = primitive_ranges;
        self.custom_meshes = custom_meshes;
        self.texture_cache = texture_cache;
    }

    /// 收集场景中的light数据
    ///
    /// 遍历scene entities查找DirectionalLightComponent，
    /// 如果没有找到则使用默认光照参数。
    fn collect_light_data(&self, ctx: &RenderContext) -> LightUboData {
        if ctx.scene.is_null() {
            // 无场景时使用默认光照
            return LightUboData {
                direction: [-0.5, -1.0, -0.5],
                _pad0: 0.0,
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            };
        }

        // 安全访问Scene（renderer持有所有权，管线只读取）
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
            // 默认fallback: direction=(-0.5,-1.0,-0.5), color=(1,1,1), intensity=1.0
            LightUboData {
                direction: [-0.5, -1.0, -0.5],
                _pad0: 0.0,
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            }
        }
    }

    /// 上传light UBO数据到GPU
    ///
    /// 将LightUboData（32 bytes）映射到light_ubo_memory并上传。
    fn upload_light_ubo(&self, light_data: &LightUboData) {
        unsafe {
            let data_ptr = self.device
                .map_memory(self.light_ubo_memory, 0, 32, vk::MemoryMapFlags::empty())
                .expect("上传light UBO数据失败: map_memory失败");
            std::ptr::copy_nonoverlapping(
                light_data as *const LightUboData as *const u8,
                data_ptr as *mut u8,
                32,
            );
            self.device.unmap_memory(self.light_ubo_memory);
        }
    }

    /// 获取实体的纹理descriptor set
    ///
    /// 优先使用renderable.texture_path（per-entity纹理），
    /// 其次使用CustomMeshInfo的texture_descriptor_set，
    /// 最后返回None（使用default_texture_descriptor_set）。
    fn get_entity_texture_ds(
        &self,
        renderable: &RenderableComponent,
        custom_info: Option<&CustomMeshInfo>,
    ) -> Option<vk::DescriptorSet> {
        // 优先使用per-entity纹理路径
        if let Some(ref texture_path) = renderable.texture_path {
            if let Some(entry) = self.texture_cache.get(texture_path) {
                return Some(entry.descriptor_set);
            }
        }
        // 其次使用CustomMeshInfo的纹理
        if let Some(info) = custom_info {
            if let Some(tex_set) = info.texture_descriptor_set {
                return Some(tex_set);
            }
        }
        None
    }
}

impl RenderPipeline for RasterPipeline {
    /// 管线名称 — "rasterization"
    fn name(&self) -> &str {
        "rasterization"
    }

    /// 提供可变Any引用 — 支持PipelineRegistry downcast到RasterPipeline
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// 描述此管线需要的GPU资源
    ///
    /// RasterPipeline需要:
    /// - offscreen渲染目标（FBO）+ depth attachment
    /// - vertex/index buffer（游戏mesh数据）
    /// - uniform buffer（light UBO）
    /// - 纹理数量（per-entity纹理 + 默认纹理）
    /// - push constant 144 bytes（严格匹配GLSL std430/std140对齐）
    fn resource_requirements(&self) -> PipelineResourceDesc {
        PipelineResourceDesc {
            vertex_buffer_size: 1024 * 1024,  // 1MB vertex buffer
            index_buffer_size: 512 * 1024,    // 512KB index buffer
            texture_count: 64,                // 最多64个纹理
            needs_depth_attachment: true,      // 游戏渲染需要depth attachment
            needs_offscreen_target: true,      // 游戏渲染需要offscreen FBO
            push_constant_size: 144,           // 144 bytes，严格匹配GLSL布局
            uniform_buffer_size: 32,           // 32 bytes light UBO
        }
    }

    /// CPU侧数据准备阶段
    ///
    /// 在此阶段:
    /// 1. 收集light数据并上传到GPU UBO
    /// 2. 更新texture_cache（新纹理上传）
    /// 3. 更新custom_meshes（新mesh buffer上传）
    /// 4. 准备push constant数据（模型矩阵、摄像机参数等）
    fn prepare(&mut self, ctx: &RenderContext, _resources: &mut PipelineResources) {
        // 1. 收集light数据并上传到GPU
        let light_data = self.collect_light_data(ctx);
        self.upload_light_ubo(&light_data);

        // 2-3. texture_cache和custom_meshes的更新
        // 在Phase 3集成时，UIVulkanRenderer会在prepare前更新这些数据
        // RasterPipeline只读取已更新的数据，不自己创建Vulkan资源

        // 4. push constant数据在record阶段按entity计算
        // prepare阶段只做全局数据准备（light UBO等）
    }

    /// GPU侧命令录制阶段
    ///
    /// 录制游戏渲染命令:
    /// 1. 开始game render pass（offscreen framebuffer）
    /// 2. 清屏（深蓝背景 + depth 1.0）
    /// 3. 设置viewport + scissor
    /// 4. 绑定light descriptor set (set 0) + texture descriptor set (set 1)
    /// 5. 遍历scene entities:
    ///    - 获取RenderableComponent + LocalTransform
    ///    - 计算模型矩阵
    ///    - 推push constants
    ///    - 绑定pipeline + descriptor sets + index buffer
    ///    - cmd_draw / cmd_draw_indexed绘制每个mesh
    /// 6. Editing模式下渲染outline（选中实体）
    /// 7. 结束render pass
    /// 8. 返回RenderPassOutput
    fn record(
        &mut self,
        ctx: &RenderContext,
        resources: &PipelineResources,
        cmd_buffer: vk::CommandBuffer,
    ) -> RenderPassOutput {
        // 获取offscreen framebuffer和extent
        let framebuffer = resources.offscreen_framebuffer
            .expect("RasterPipeline::record: offscreen_framebuffer未分配");
        let extent = vk::Extent2D {
            width: ctx.viewport_extent.width,
            height: ctx.viewport_extent.height,
        };

        // 1. 开始game render pass
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.05, 0.05, 0.1, 1.0],  // 深蓝背景
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let render_pass_begin = vk::RenderPassBeginInfo {
            render_pass: self.game_render_pass,
            framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            },
            clear_value_count: 2,
            p_clear_values: &clear_values as *const _,
            _marker: std::marker::PhantomData,
            p_next: std::ptr::null(),
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
        };
        // Vulkan命令录制都是unsafe操作（ash API要求）
        // 这些是命令录制而非资源创建/销毁，是管线工作的必要部分
        unsafe {
            self.device.cmd_begin_render_pass(cmd_buffer, &render_pass_begin, vk::SubpassContents::INLINE);

            // 2-3. 设置viewport + scissor（动态状态，pipeline使用p_dynamic_state）
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };
            self.device.cmd_set_viewport(cmd_buffer, 0, &[viewport]);
            self.device.cmd_set_scissor(cmd_buffer, 0, &[scissor]);

            // 4. 绑定light descriptor set (set 0) + default texture descriptor set (set 1)
            self.device.cmd_bind_descriptor_sets(
                cmd_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.game_pipeline_layout,
                0,
                &[self.light_descriptor_set, self.default_texture_descriptor_set],
                &[],
            );
            
            // 绑定vertex buffer（游戏mesh数据）
            self.device.cmd_bind_vertex_buffers(
                cmd_buffer,
                0,
                &[self.game_mesh_buffer],
                &[0],
            );

            // 5. 遍历scene entities并绘制
            if !ctx.scene.is_null() {
                let scene = &*ctx.scene;
                let camera_pos = [ctx.camera.position[0], ctx.camera.position[1], ctx.camera.position[2]];
                let camera_yaw = ctx.camera.yaw;
                let camera_pitch = ctx.camera.pitch;
                let viewport_w = extent.width as f32;
                let viewport_h = extent.height as f32;

                for entity in &scene.root_entities {
                    // 获取RenderableComponent
                    let renderable_opt = scene.world.get_component::<RenderableComponent>(*entity);
                    if renderable_opt.is_none() {
                        continue;
                    }
                    let renderable = renderable_opt.unwrap();
                    if !renderable.visible {
                        continue;
                    }

                    // 获取LocalTransform
                    let transform_opt = scene.world.get_component::<LocalTransform>(*entity);
                    if transform_opt.is_none() {
                        continue;
                    }
                    let transform = transform_opt.unwrap();

                    // 判断是否选中
                    let is_selected = ctx.selected_entity_ids.contains(&entity.id);

                    if is_custom_mesh(&renderable.mesh_path) {
                        // === 自定义mesh渲染 ===
                        if let Some(custom_info) = self.custom_meshes.get(&renderable.mesh_path) {
                            let model = compute_model_matrix(&transform);
                            let outline_color = if is_selected { [1.0, 0.5, 0.0] } else { [0.0, 0.0, 0.0] };

                            // 获取per-entity纹理descriptor set
                            let entity_texture_ds = self.get_entity_texture_ds(&renderable, Some(custom_info));
                            let has_texture_f = entity_texture_ds.is_some();

                            // 构建push constant数据（144 bytes）
                            let push_data = build_push_constant(
                                model,
                                outline_color,
                                is_selected,
                                viewport_w,
                                viewport_h,
                                camera_pos,
                                camera_yaw,
                                camera_pitch,
                                has_texture_f,
                                custom_info.specular_strength,
                                custom_info.ambient_strength,
                                custom_info.shininess,
                            );

                            // 绑定game pipeline
                            self.device.cmd_bind_pipeline(
                                cmd_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                self.game_pipeline,
                            );

                            // 绑定per-entity纹理descriptor set（如果有纹理则替换default）
                            if let Some(tex_ds) = entity_texture_ds {
                                self.device.cmd_bind_descriptor_sets(
                                    cmd_buffer,
                                    vk::PipelineBindPoint::GRAPHICS,
                                    self.game_pipeline_layout,
                                    0,
                                    &[self.light_descriptor_set, tex_ds],
                                    &[],
                                );
                            }

                            // 推push constants
                            self.device.cmd_push_constants(
                                cmd_buffer,
                                self.game_pipeline_layout,
                                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                0,
                                bytemuck::cast_slice(&[push_data]),
                            );

                            // 绑定index buffer
                            self.device.cmd_bind_index_buffer(
                                cmd_buffer,
                                self.game_index_buffer,
                                0,
                                vk::IndexType::UINT32,
                            );

                            // 用index buffer绘制
                            self.device.cmd_draw_indexed(
                                cmd_buffer,
                                custom_info.index_count,
                                1,
                                custom_info.index_offset,
                                custom_info.vertex_offset as i32,
                                0,
                            );

                            // 如果选中，绘制outline（1.05放大 + CULL_FRONT + alpha blend）
                            if is_selected && ctx.mode == RenderMode::Editing {
                                let outline_model = compute_scaled_model_matrix(&transform, 1.05);
                                let outline_push_data = build_push_constant(
                                    outline_model,
                                    [1.0, 0.5, 0.0],  // outline颜色：橙色
                                    true,               // is_selected=1.0 for outline
                                    viewport_w,
                                    viewport_h,
                                    camera_pos,
                                    camera_yaw,
                                    camera_pitch,
                                    false,              // outline无纹理
                                    0.3,                // specular
                                    0.15,               // ambient
                                    32.0,               // shininess
                                );

                                self.device.cmd_bind_pipeline(
                                    cmd_buffer,
                                    vk::PipelineBindPoint::GRAPHICS,
                                    self.outline_pipeline,
                                );

                                self.device.cmd_push_constants(
                                    cmd_buffer,
                                    self.game_pipeline_layout,
                                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                    0,
                                    bytemuck::cast_slice(&[outline_push_data]),
                                );

                                // outline也用cmd_draw_indexed（与主mesh相同方式）
                                self.device.cmd_bind_index_buffer(
                                    cmd_buffer,
                                    self.game_index_buffer,
                                    0,
                                    vk::IndexType::UINT32,
                                );

                                self.device.cmd_draw_indexed(
                                    cmd_buffer,
                                    custom_info.index_count,
                                    1,
                                    custom_info.index_offset,
                                    custom_info.vertex_offset as i32,
                                    0,
                                );
                            }
                        }
                    } else {
                        // === Primitive mesh渲染（使用cmd_draw，无index buffer） ===
                        let mesh_type = mesh_type_from_path(&renderable.mesh_path);
                        let range = self.primitive_ranges.get(&mesh_type);
                        if range.is_none() {
                            continue;
                        }
                        let range = range.unwrap();

                        let model = compute_model_matrix(&transform);
                        let outline_color = if is_selected { [1.0, 0.5, 0.0] } else { [0.0, 0.0, 0.0] };

                        // 获取per-entity纹理descriptor set
                        let entity_texture_ds = self.get_entity_texture_ds(&renderable, None);
                        let has_texture_f = entity_texture_ds.is_some();

                        // 构建push constant数据（144 bytes）
                        let push_data = build_push_constant(
                            model,
                            outline_color,
                            is_selected,
                            viewport_w,
                            viewport_h,
                            camera_pos,
                            camera_yaw,
                            camera_pitch,
                            has_texture_f,
                            0.3,   // specular_strength（primitive默认值）
                            0.15,  // ambient_strength（primitive默认值）
                            32.0,  // shininess（primitive默认值）
                        );

                        // 绑定game pipeline
                        self.device.cmd_bind_pipeline(
                            cmd_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            self.game_pipeline,
                        );

                        // 绑定per-entity纹理descriptor set（如果有纹理则替换default）
                        if let Some(tex_ds) = entity_texture_ds {
                            self.device.cmd_bind_descriptor_sets(
                                cmd_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                self.game_pipeline_layout,
                                0,
                                &[self.light_descriptor_set, tex_ds],
                                &[],
                            );
                        }

                        // 推push constants
                        self.device.cmd_push_constants(
                            cmd_buffer,
                            self.game_pipeline_layout,
                            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                            0,
                            bytemuck::cast_slice(&[push_data]),
                        );

                        // Primitive mesh使用cmd_draw（无index buffer）
                        self.device.cmd_draw(
                            cmd_buffer,
                            range.vertex_count,
                            1,
                            range.vertex_offset,
                            0,
                        );

                        // 如果选中，绘制outline
                        if is_selected && ctx.mode == RenderMode::Editing {
                            let outline_model = compute_scaled_model_matrix(&transform, 1.05);
                            let outline_push_data = build_push_constant(
                                outline_model,
                                [1.0, 0.5, 0.0],
                                true,
                                viewport_w,
                                viewport_h,
                                camera_pos,
                                camera_yaw,
                                camera_pitch,
                                false,
                                0.3,
                                0.15,
                                32.0,
                            );

                            self.device.cmd_bind_pipeline(
                                cmd_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                self.outline_pipeline,
                            );

                            self.device.cmd_push_constants(
                                cmd_buffer,
                                self.game_pipeline_layout,
                                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                0,
                                bytemuck::cast_slice(&[outline_push_data]),
                            );

                            // outline也用cmd_draw（primitive mesh无index buffer）
                            self.device.cmd_draw(
                                cmd_buffer,
                                range.vertex_count,
                                1,
                                range.vertex_offset,
                                0,
                            );
                        }
                    }
                }
            }

            // 7. 结束game render pass
            self.device.cmd_end_render_pass(cmd_buffer);
        } // unsafe块结束

        // 8. 返回RenderPassOutput
        // offscreen image layout transition由render pass implicit end处理
        // (finalLayout=SHADER_READ_ONLY_OPTIMAL, subpass dependency同步)
        let offscreen_image = resources.offscreen_image
            .expect("RasterPipeline::record: offscreen_image未分配");
        let offscreen_view = resources.offscreen_view
            .expect("RasterPipeline::record: offscreen_view未分配");

        RenderPassOutput {
            color_image: offscreen_image,
            color_image_view: offscreen_view,
            extent,
            descriptor_set: self.preview_descriptor_set,
        }
    }

    /// 后处理阶段 — FXAA抗锯齿
    ///
    /// 流程:
    /// 1. 转换FXAA output image到COLOR_ATTACHMENT_OPTIMAL layout
    /// 2. 开始FXAA render pass
    /// 3. 绑定FXAA pipeline + descriptor sets
    /// 4. 设置viewport + scissor
    /// 5. 推resolution push constant (vec2: width, height)
    /// 6. 绘制全屏quad (6 vertices)
    /// 7. 结束FXAA render pass
    /// 8. FXAA output自动转换到SHADER_READ_ONLY_OPTIMAL（render pass finalLayout）
    /// 9. 返回FXAA处理后的RenderPassOutput
    fn post_process(
        &mut self,
        ctx: &RenderContext,
        resources: &PipelineResources,
        input: &RenderPassOutput,
        cmd_buffer: vk::CommandBuffer,
    ) -> RenderPassOutput {
        let extent = input.extent;

let fxaa_image = resources.fxaa_image
            .expect("RasterPipeline::post_process: fxaa_image未分配");
        let fxaa_framebuffer = resources.fxaa_framebuffer
            .expect("RasterPipeline::post_process: fxaa_framebuffer未分配");
        let fxaa_view = resources.fxaa_view
            .expect("RasterPipeline::post_process: fxaa_view未分配");

        // 1. 转换FXAA output image到COLOR_ATTACHMENT_OPTIMAL layout
        // 第一帧: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL
        // 后续帧: SHADER_READ_ONLY_OPTIMAL → COLOR_ATTACHMENT_OPTIMAL
        let fxaa_old_layout = if ctx.frame_index == 0 {
            vk::ImageLayout::UNDEFINED
        } else {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        };
        let fxaa_src_stage = if ctx.frame_index == 0 {
            vk::PipelineStageFlags::TOP_OF_PIPE
        } else {
            vk::PipelineStageFlags::FRAGMENT_SHADER
        };
        let fxaa_barrier = vk::ImageMemoryBarrier {
            old_layout: fxaa_old_layout,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
image: fxaa_image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: if ctx.frame_index == 0 {
                vk::AccessFlags::empty()
            } else {
                vk::AccessFlags::SHADER_READ
            },
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };
        // FXAA render pass数据准备（不涉及unsafe操作）
        let fxaa_clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let fxaa_render_pass_begin = vk::RenderPassBeginInfo {
            render_pass: self.game_render_pass,
            framebuffer: fxaa_framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            },
            clear_value_count: 2,
            p_clear_values: &fxaa_clear_values as *const _,
            _marker: std::marker::PhantomData,
            p_next: std::ptr::null(),
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
        };
        let fxaa_viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let fxaa_scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };
        let resolution_data = [extent.width as f32, extent.height as f32];

        // Vulkan命令录制都是unsafe操作（ash API要求）
        unsafe {
            self.device.cmd_pipeline_barrier(
                cmd_buffer,
                fxaa_src_stage,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[fxaa_barrier],
            );

            // 2. 开始FXAA render pass
            self.device.cmd_begin_render_pass(cmd_buffer, &fxaa_render_pass_begin, vk::SubpassContents::INLINE);

            // 3. 绑定FXAA pipeline + descriptor sets
            self.device.cmd_bind_pipeline(
                cmd_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.fxaa_pipeline,
            );
            self.device.cmd_bind_descriptor_sets(
                cmd_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.fxaa_pipeline_layout,
                0,
                &[self.fxaa_descriptor_set],
                &[],
            );

            // 4. 设置viewport + scissor（动态状态）
            self.device.cmd_set_viewport(cmd_buffer, 0, &[fxaa_viewport]);
            self.device.cmd_set_scissor(cmd_buffer, 0, &[fxaa_scissor]);

            // 5. 推resolution push constant (vec2: width, height)
            self.device.cmd_push_constants(
                cmd_buffer,
                self.fxaa_pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&resolution_data),
            );

            // 6. 绘制全屏quad（FXAA shader使用gl_VertexIndex生成顶点）
            self.device.cmd_draw(cmd_buffer, 6, 1, 0, 0);

            // 7. 结束FXAA render pass
            self.device.cmd_end_render_pass(cmd_buffer);
        } // unsafe块结束

        // 8. FXAA output自动转换到SHADER_READ_ONLY_OPTIMAL
        // (render pass finalLayout=SHADER_READ_ONLY_OPTIMAL, subpass dependency同步)

        // 9. 返回FXAA处理后的RenderPassOutput
        // FXAA输出替代原始offscreen输出，供UI preview采样
        RenderPassOutput {
            color_image: fxaa_image,
            color_image_view: fxaa_view,
            extent,
            descriptor_set: self.preview_descriptor_set,
        }
    }

    /// UI合成阶段 — 返回preview_descriptor_set供UI采样
    ///
    /// UI preview窗口通过此descriptor set采样FXAA处理后的游戏渲染结果。
    fn composite(
        &mut self,
        _ctx: &RenderContext,
        _output: &RenderPassOutput,
        _cmd_buffer: vk::CommandBuffer,
    ) -> Option<vk::DescriptorSet> {
        Some(self.preview_descriptor_set)
    }

    // cleanup使用默认实现（空操作）
    // RasterPipeline不直接创建Vulkan资源，资源由rhi-vulkan层管理
}