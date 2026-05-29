//! RenderPipeline trait 定义及 RenderPassOutput 类型
//!
//! RenderPipeline 是渲染管线的核心抽象，定义了5个hook点:
//! - prepare: 数据准备阶段（CPU侧）
//! - record: 命令录制阶段（GPU侧）
//! - post_process: 后处理阶段
//! - composite: UI合成阶段
//! - cleanup: 资源清理阶段
//!
//! 所有管线必须 Send + Sync，支持多线程注册和帧渲染。

use ash::vk;
use crate::context::RenderContext;
use crate::resource::{PipelineResources, PipelineResourceDesc};

/// 渲染管线 trait — 所有管线实现必须满足 Send + Sync
///
/// 5个hook点对应渲染帧的不同阶段:
/// 1. prepare — CPU侧数据准备，更新buffer/上传纹理
/// 2. record — GPU侧命令录制，写入command buffer
/// 3. post_process — 后处理（可选），对record输出做进一步处理
/// 4. composite — UI合成，将3D渲染结果与UI overlay合并
/// 5. cleanup — 资源清理（可选默认实现）
///
/// **重要约束**:
/// - 所有pipeline必须使用 p_dynamic_state for VIEWPORT+SCISSOR
/// - 所有push constant布局必须严格匹配GLSL std430/std140对齐规则
/// - 所有vertex type必须有UV坐标（NEVER omit UV）
pub trait RenderPipeline: Send + Sync {
    /// 管线名称，用于PipelineRegistry查找和切换
    fn name(&self) -> &str;

    /// 描述此管线需要的GPU资源（buffer大小、纹理数量等）
    fn resource_requirements(&self) -> PipelineResourceDesc;

    /// CPU侧数据准备阶段
    ///
    /// 在此阶段更新vertex buffer、上传纹理、计算push constant数据等。
    /// resources在此阶段可写（&mut），用于填充GPU资源。
    fn prepare(&mut self, ctx: &RenderContext, resources: &mut PipelineResources);

    /// GPU侧命令录制阶段
    ///
    /// 将渲染命令写入command buffer。resources在此阶段只读（&），
    /// 因为GPU可能正在使用这些资源。
    fn record(
        &mut self,
        ctx: &RenderContext,
        resources: &PipelineResources,
        cmd_buffer: vk::CommandBuffer,
    ) -> RenderPassOutput;

    /// 后处理阶段
    ///
    /// 对record阶段的输出做进一步处理（如bloom、tone mapping等）。
    /// input是record阶段的输出，返回处理后的结果。
    fn post_process(
        &mut self,
        ctx: &RenderContext,
        resources: &PipelineResources,
        input: &RenderPassOutput,
        cmd_buffer: vk::CommandBuffer,
    ) -> RenderPassOutput;

    /// UI合成阶段
    ///
    /// 将3D渲染结果与UI overlay合并。返回用于最终呈现的descriptor set，
    /// 供UI preview采样使用。
    fn composite(
        &mut self,
        ctx: &RenderContext,
        output: &RenderPassOutput,
        cmd_buffer: vk::CommandBuffer,
    ) -> Option<vk::DescriptorSet>;

    /// 资源清理阶段（可选）
    ///
    /// 在管线切换或销毁时调用，用于释放管线专属资源。
    /// 默认实现为空操作。
    fn cleanup(&mut self, _ctx: &RenderContext) {}

    /// Resize管线专属的GPU资源（如compute shader output image）
    ///
    /// 当offscreen FBO resize时，管线可能有自己的GPU资源需要同步resize。
    /// 调用方已确保device_wait_idle()，GPU空闲，安全销毁旧资源。
    /// 默认实现为空操作（大多数管线没有专属resize需求）。
    fn resize(&mut self, _width: u32, _height: u32) {}

    /// 提供可变Any引用 — 用于downcast到具体管线类型
    ///
    /// PipelineRegistry需要通过downcast访问具体管线（如RasterPipeline）的特有方法，
    /// 因为active_pipeline_mut()返回&mut dyn RenderPipeline无法调用具体方法。
    /// 默认实现返回self的可变Any引用。
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        // 默认实现：大多数管线不需要downcast，返回空Any
        // 需要downcast的管线必须覆盖此方法
        unimplemented!("as_any_mut: 管线{}未实现as_any_mut，如需downcast请覆盖此方法", self.name())
    }
}

/// 渲染Pass的输出结果
///
/// 包含渲染后的color image及其view、extent和descriptor set，
/// 用于传递给后处理或UI合成阶段，以及UI preview采样。
#[derive(Clone, Copy, Debug)]
pub struct RenderPassOutput {
    /// 渲染输出的color image
    pub color_image: vk::Image,
    /// color image的image view
    pub color_image_view: vk::ImageView,
    /// 渲染输出的extent（宽高）
    pub extent: vk::Extent2D,
    /// 用于采样此输出的descriptor set（UI preview采样用）
    pub descriptor_set: vk::DescriptorSet,
}

impl Default for RenderPassOutput {
    fn default() -> Self {
        Self {
            color_image: vk::Image::null(),
            color_image_view: vk::ImageView::null(),
            extent: vk::Extent2D { width: 0, height: 0 },
            descriptor_set: vk::DescriptorSet::null(),
        }
    }
}

impl RenderPassOutput {
    /// 检查输出是否有效（非null handle）
    pub fn is_valid(&self) -> bool {
        self.color_image != vk::Image::null()
            && self.color_image_view != vk::ImageView::null()
            && self.extent.width > 0
            && self.extent.height > 0
    }
}