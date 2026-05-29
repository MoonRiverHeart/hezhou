//! 管线注册表 — 运行时管线管理
//!
//! PipelineRegistry管理所有已注册的管线实现，
//! 支持运行时切换活跃管线，并通过ResourceManager管理资源生命周期。

use std::collections::HashMap;
use ash::vk;
use crate::pipeline::{RenderPipeline, RenderPassOutput};
use crate::context::RenderContext;
use crate::resource::{PipelineResources, PipelineResourceDesc, ResourceManager};
use crate::raster_pipeline::{RasterPipeline, PrimitiveMeshRange, CustomMeshInfo, TextureCacheEntry};
use hezhou_core::asset_library::MeshType;

/// 管线注册表 — 运行时管线管理
///
/// 维护所有已注册的管线实现，跟踪当前活跃管线，
/// 并通过ResourceManager管理管线资源生命周期。
///
/// **重要约束**:
/// - switch_pipeline时必须先device_wait_idle()，确保GPU不再使用旧管线资源
/// - 销毁FBO资源前必须device_wait_idle()（否则VK_ERROR_DEVICE_LOST）
/// - 所有pipeline必须使用p_dynamic_state for VIEWPORT+SCISSOR
pub struct PipelineRegistry {
    /// 所有已注册的管线实现
    pipelines: HashMap<String, Box<dyn RenderPipeline>>,
    /// 当前活跃管线名称
    active_pipeline: String,
    /// 资源管理器
    resource_manager: ResourceManager,
    /// 当前活跃管线的GPU资源
    active_resources: PipelineResources,
}

impl PipelineRegistry {
    /// 创建PipelineRegistry
    ///
    /// 需要Vulkan device和物理设备内存属性用于ResourceManager。
    pub fn new(device: vk::Device, memory_properties: vk::PhysicalDeviceMemoryProperties) -> Self {
        Self {
            pipelines: HashMap::new(),
            active_pipeline: String::new(),
            resource_manager: ResourceManager::new(device, memory_properties),
            active_resources: PipelineResources::default(),
        }
    }

    /// 注册新管线
    ///
    /// 管线必须实现RenderPipeline trait。注册后可通过name切换到此管线。
    /// 如果是第一个注册的管线，自动设为活跃管线。
    pub fn register(&mut self, pipeline: Box<dyn RenderPipeline>) {
        let name = pipeline.name().to_string();
        let desc = pipeline.resource_requirements();
        self.pipelines.insert(name.clone(), pipeline);
        // 如果是第一个注册的管线，自动设为活跃
        if self.active_pipeline.is_empty() {
            self.active_pipeline = name;
            self.resource_manager.update_desc(desc.clone());
            self.active_resources = self.resource_manager.allocate_resources(&desc);
        }
    }

    /// 切换活跃管线
    ///
    /// **重要**: 调用此方法前必须先调用 `device.device_wait_idle()`，
    /// 确保GPU不再使用旧管线资源。否则可能导致VK_ERROR_DEVICE_LOST。
    ///
    /// 切换流程:
    /// 1. 调用旧管线的cleanup
    /// 2. 将旧管线资源标记为待销毁（mark_for_destroy）
    /// 3. 根据新管线resource_requirements分配资源
    /// 4. 设置新管线为活跃
    ///
    /// 待销毁资源的实际销毁由rhi-vulkan层在device_wait_idle()后完成:
    /// ```ignore
    /// device.device_wait_idle();  // 必须先等GPU完成
    /// let pending = registry.resource_manager_mut().take_pending_destroy();
    /// // 销毁pending中的所有资源...
    /// ```
    pub fn switch_pipeline(&mut self, name: &str, ctx: &RenderContext) -> bool {
        if !self.pipelines.contains_key(name) {
            return false;
        }

        // 清理旧管线
        if let Some(old_pipeline) = self.pipelines.get_mut(&self.active_pipeline) {
            old_pipeline.cleanup(ctx);
        }

        // 将旧管线资源标记为待销毁（实际销毁在device_wait_idle后由rhi-vulkan完成）
        let old_resources = std::mem::take(&mut self.active_resources);
        self.resource_manager.mark_for_destroy(old_resources);

        // 切换到新管线
        self.active_pipeline = name.to_string();

        // 更新资源描述并分配新资源
        let new_desc = self.pipelines
            .get(name)
            .map(|p| p.resource_requirements())
            .unwrap_or_default();

        self.resource_manager.update_desc(new_desc.clone());
        self.active_resources = self.resource_manager.allocate_resources(&new_desc);

        true
    }

    /// 获取当前活跃管线名称
    pub fn active_pipeline_name(&self) -> &str {
        &self.active_pipeline
    }

    /// 获取当前活跃管线的引用
    ///
    /// 返回None如果没有活跃管线。
    pub fn active_pipeline(&self) -> Option<&dyn RenderPipeline> {
        self.pipelines
            .get(&self.active_pipeline)
            .map(|p| p.as_ref())
    }

    /// 获取当前活跃管线的可变引用
    ///
    /// 返回None如果没有活跃管线。
    pub fn active_pipeline_mut(&mut self) -> Option<&mut (dyn RenderPipeline + '_)> {
        self.pipelines
            .get_mut(&self.active_pipeline)
            .map(|p| p.as_mut() as &mut dyn RenderPipeline)
    }

    /// 获取当前活跃管线的GPU资源
    pub fn active_resources(&self) -> &PipelineResources {
        &self.active_resources
    }

    /// 获取当前活跃管线的GPU资源（可变）
    pub fn active_resources_mut(&mut self) -> &mut PipelineResources {
        &mut self.active_resources
    }

    /// 获取所有已注册管线名称
    pub fn pipeline_names(&self) -> Vec<&str> {
        self.pipelines.keys().map(|s| s.as_str()).collect()
    }

    /// 获取指定管线的资源需求描述
    pub fn pipeline_desc(&self, name: &str) -> Option<PipelineResourceDesc> {
        self.pipelines.get(name).map(|p| p.resource_requirements())
    }

    /// 获取ResourceManager引用
    pub fn resource_manager(&self) -> &ResourceManager {
        &self.resource_manager
    }

    /// 获取ResourceManager可变引用（用于take_pending_destroy等操作）
    pub fn resource_manager_mut(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }

    /// resize后同步UIVulkanRenderer的新offscreen handle到PipelineResources
    ///
    /// 当apply_pending_offscreen_resize()销毁旧资源并重建新资源后，
    /// PipelineResources中的旧handle已失效，必须同步新handle，
    /// 否则RasterPipeline使用已销毁handle → ACCESS_VIOLATION崩溃。
    pub fn update_offscreen_resources(
        &mut self,
        offscreen_image: vk::Image,
        offscreen_memory: vk::DeviceMemory,
        offscreen_view: vk::ImageView,
        offscreen_framebuffer: vk::Framebuffer,
        depth_image: vk::Image,
        depth_memory: vk::DeviceMemory,
        depth_view: vk::ImageView,
        fxaa_image: vk::Image,
        fxaa_memory: vk::DeviceMemory,
        fxaa_view: vk::ImageView,
        fxaa_framebuffer: vk::Framebuffer,
    ) {
        self.active_resources.offscreen_image = Some(offscreen_image);
        self.active_resources.offscreen_memory = Some(offscreen_memory);
        self.active_resources.offscreen_view = Some(offscreen_view);
        self.active_resources.offscreen_framebuffer = Some(offscreen_framebuffer);
        self.active_resources.depth_image = Some(depth_image);
        self.active_resources.depth_memory = Some(depth_memory);
        self.active_resources.depth_view = Some(depth_view);
        self.active_resources.fxaa_image = Some(fxaa_image);
        self.active_resources.fxaa_memory = Some(fxaa_memory);
        self.active_resources.fxaa_view = Some(fxaa_view);
        self.active_resources.fxaa_framebuffer = Some(fxaa_framebuffer);
    }

/// 更新RasterPipeline的动态数据（primitive_ranges/custom_meshes/texture_cache）
    ///
    /// 通过downcast访问RasterPipeline特有方法。
    /// 如果当前活跃管线不是RasterPipeline，此方法不做任何操作。
    pub fn update_raster_dynamic_data(
        &mut self,
        primitive_ranges: HashMap<MeshType, PrimitiveMeshRange>,
        custom_meshes: HashMap<String, CustomMeshInfo>,
        texture_cache: HashMap<String, TextureCacheEntry>,
    ) {
        // 遍历所有pipeline找到RasterPipeline（通过name="rasterization"）
        for pipeline in self.pipelines.values_mut() {
            if pipeline.name() == "rasterization" {
                if let Some(raster) = pipeline.as_any_mut().downcast_mut::<RasterPipeline>() {
                    raster.update_dynamic_data(primitive_ranges, custom_meshes, texture_cache);
                    return;
                }
            }
        }
    }

/// 执行一帧渲染
    ///
    /// 按顺序调用活跃管线的5个hook点:
    /// 1. prepare — CPU侧数据准备
    /// 2. record — GPU侧命令录制
    /// 3. post_process — 后处理
    /// 4. composite — UI合成
    ///
    /// 返回composite阶段的descriptor set（用于最终呈现），
    /// 或None如果没有活跃管线。
    ///
    /// 直接访问struct字段而非通过方法，避免双重可变借用冲突。
    pub fn render_frame(
        &mut self,
        ctx: &RenderContext,
        cmd_buffer: vk::CommandBuffer,
    ) -> Option<(vk::DescriptorSet, RenderPassOutput)> {
        // 直接访问字段，Rust允许同时可变借用不同struct字段
        let pipeline = self.pipelines.get_mut(&self.active_pipeline)?;

        // 1. prepare — CPU侧数据准备（resources可写）
        pipeline.prepare(ctx, &mut self.active_resources);

        // 2. record — GPU侧命令录制（resources只读）
        let pass_output = pipeline.record(ctx, &self.active_resources, cmd_buffer);

        // 3. post_process — 后处理
        let processed_output = pipeline.post_process(ctx, &self.active_resources, &pass_output, cmd_buffer);

        // 4. composite — UI合成
        let ds = pipeline.composite(ctx, &processed_output, cmd_buffer);

        // composite必须返回descriptor set（否则UI preview无法采样渲染结果）
        let ds = ds.expect("composite必须返回descriptor set用于UI preview采样");

        Some((ds, processed_output))
    }
}