use ash::vk;
use hezhou_rhi::{
    BufferHandle, CommandBuffer, CommandBufferHandle, ClearValue, FramebufferHandle, IndexType,
    PipelineHandle, RenderPassHandle, TextureHandle,
};
use hezhou_core::math::Vec3u;

/// Vulkan command buffer 封装，实现 RHI CommandBuffer trait
/// 内部持有 raw vk::CommandBuffer 和 ash Device 引用
/// ash API 中 command buffer 录制函数通过 Device trait 调用（device.cmd_xxx(cmd, ...)）
pub struct VulkanCommandBuffer {
    device: ash::Device,
    cmd: vk::CommandBuffer,
    handle_id: u64,
}

impl VulkanCommandBuffer {
    pub fn new(device: ash::Device, cmd: vk::CommandBuffer, handle_id: u64) -> Self {
        Self { device, cmd, handle_id }
    }

    /// 获取底层 Vulkan command buffer，用于直接 ash API 调用
    pub fn raw(&self) -> vk::CommandBuffer {
        self.cmd
    }

    /// 获取 ash Device 引用
    pub fn device(&self) -> &ash::Device {
        &self.device
    }
}

impl CommandBuffer for VulkanCommandBuffer {
    fn handle(&self) -> CommandBufferHandle {
        CommandBufferHandle(self.handle_id)
    }

    fn begin(&mut self) {
        unsafe {
            self.device
                .begin_command_buffer(self.cmd, &vk::CommandBufferBeginInfo::default())
                .unwrap();
        }
    }

    fn end(&mut self) {
        unsafe {
            self.device.end_command_buffer(self.cmd).unwrap();
        }
    }

    fn begin_render_pass(
        &mut self,
        _pass: RenderPassHandle,
        _framebuffer: FramebufferHandle,
        _clear_values: &[ClearValue],
    ) {
        // TODO: 通过 handle 映射查找实际 vk::RenderPass 和 vk::Framebuffer
    }

    fn end_render_pass(&mut self) {
        unsafe {
            self.device.cmd_end_render_pass(self.cmd);
        }
    }

    fn bind_pipeline(&mut self, _pipeline: PipelineHandle) {
        // TODO: 通过 handle 映射查找实际 vk::Pipeline，根据 PipelineType 选择 bind point
    }

    fn bind_vertex_buffer(&mut self, _slot: u32, _buffer: BufferHandle, _offset: usize) {
        // TODO: 通过 handle 映射查找实际 vk::Buffer
    }

    fn bind_index_buffer(&mut self, _buffer: BufferHandle, _offset: usize, _index_type: IndexType) {
        // TODO: 通过 handle 映射查找实际 vk::Buffer
    }

    fn bind_uniform_buffer(
        &mut self,
        _set: u32,
        _binding: u32,
        _buffer: BufferHandle,
        _offset: usize,
        _size: usize,
    ) {
        // TODO: 通过 handle 映射查找实际 vk::Buffer 和 vk::DescriptorSet
    }

    fn bind_texture(&mut self, _set: u32, _binding: u32, _texture: TextureHandle) {
        // TODO: 通过 handle 映射查找实际 vk::DescriptorSet
    }

    fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) {
        let viewport = vk::Viewport {
            x,
            y,
            width,
            height,
            min_depth,
            max_depth,
        };
        unsafe {
            self.device.cmd_set_viewport(self.cmd, 0, &[viewport]);
        }
    }

    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) {
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: x as i32, y: y as i32 },
            extent: vk::Extent2D { width, height },
        };
        unsafe {
            self.device.cmd_set_scissor(self.cmd, 0, &[scissor]);
        }
    }

    fn set_push_constants(&mut self, _offset: usize, _data: &[u8]) {
        // TODO: 需要知道 pipeline layout 和 stage flags 才能调用
    }

    fn draw(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.cmd_draw(self.cmd, vertex_count, instance_count, first_vertex, first_instance);
        }
    }

    fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.cmd_draw_indexed(
                self.cmd,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }

    fn copy_buffer(
        &mut self,
        _src: BufferHandle,
        _dst: BufferHandle,
        _src_offset: usize,
        _dst_offset: usize,
        _size: usize,
    ) {
        // TODO: 通过 handle 映射查找实际 vk::Buffer
    }

    fn copy_buffer_to_texture(
        &mut self,
        _src: BufferHandle,
        _dst: TextureHandle,
        _dst_offset: Vec3u,
        _extent: Vec3u,
    ) {
        // TODO: 通过 handle 映射查找实际 vk::Buffer 和 vk::Image
    }

    /// 计算着色器 dispatch：执行 (group_count_x × group_count_y × group_count_z) 个工作组
    fn dispatch(&mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        unsafe {
            self.device.cmd_dispatch(self.cmd, group_count_x, group_count_y, group_count_z);
        }
    }
}