use hezhou_rhi::{Device, DeviceCapabilities, BufferHandle, BufferDesc, TextureHandle, TextureDesc, ShaderHandle, ShaderDesc, PipelineHandle, PipelineDesc, RenderPassHandle, RenderPassDesc, CommandPoolHandle, SwapChainHandle, SwapChainDesc, FramebufferHandle, FramebufferDesc, CommandBufferHandle, RhiResult, RhiError};

pub struct VulkanDeviceStub {
    capabilities: DeviceCapabilities,
}

impl VulkanDeviceStub {
    pub fn new() -> RhiResult<Self> {
        Ok(Self { capabilities: DeviceCapabilities::default() })
    }
}

impl Device for VulkanDeviceStub {
    fn capabilities(&self) -> &DeviceCapabilities { &self.capabilities }
    fn create_buffer(&self, _desc: &BufferDesc) -> RhiResult<BufferHandle> { Ok(BufferHandle(1)) }
    fn destroy_buffer(&self, _buffer: BufferHandle) {}
    fn map_buffer(&self, _buffer: BufferHandle) -> RhiResult<*mut u8> { Err(RhiError::MappingFailed("stub".to_string())) }
    fn unmap_buffer(&self, _buffer: BufferHandle) {}
    fn write_buffer(&self, _buffer: BufferHandle, _offset: usize, _data: &[u8]) -> RhiResult<()> { Ok(()) }
    fn create_texture(&self, _desc: &TextureDesc) -> RhiResult<TextureHandle> { Ok(TextureHandle(1)) }
    fn destroy_texture(&self, _texture: TextureHandle) {}
    fn create_shader(&self, _desc: &ShaderDesc) -> RhiResult<ShaderHandle> { Ok(ShaderHandle(1)) }
    fn destroy_shader(&self, _shader: ShaderHandle) {}
    fn create_pipeline(&self, _desc: &PipelineDesc) -> RhiResult<PipelineHandle> { Ok(PipelineHandle(1)) }
    fn destroy_pipeline(&self, _pipeline: PipelineHandle) {}
    fn create_render_pass(&self, _desc: &RenderPassDesc) -> RhiResult<RenderPassHandle> { Ok(RenderPassHandle(1)) }
    fn destroy_render_pass(&self, _pass: RenderPassHandle) {}
    fn create_command_pool(&self) -> RhiResult<CommandPoolHandle> { Ok(CommandPoolHandle(1)) }
    fn destroy_command_pool(&self, _pool: CommandPoolHandle) {}
    fn create_command_buffer(&self, _pool: CommandPoolHandle) -> RhiResult<CommandBufferHandle> { Ok(CommandBufferHandle(1)) }
    fn reset_command_pool(&self, _pool: CommandPoolHandle) {}
    fn create_swapchain(&self, _desc: &SwapChainDesc) -> RhiResult<SwapChainHandle> { Ok(SwapChainHandle(1)) }
    fn destroy_swapchain(&self, _swapchain: SwapChainHandle) {}
    fn resize_swapchain(&self, _swapchain: SwapChainHandle, _width: u32, _height: u32) -> RhiResult<()> { Ok(()) }
    fn create_framebuffer(&self, _desc: &FramebufferDesc) -> RhiResult<FramebufferHandle> { Ok(FramebufferHandle(1)) }
    fn destroy_framebuffer(&self, _framebuffer: FramebufferHandle) {}
    fn wait_idle(&self) -> RhiResult<()> { Ok(()) }
    fn begin_frame(&self) -> RhiResult<u32> { Ok(0) }
    fn end_frame(&self) -> RhiResult<()> { Ok(()) }
    fn present(&self) -> RhiResult<()> { Ok(()) }
}

pub use VulkanDeviceStub as VulkanDevice;