use crate::{
    BufferDesc, BufferHandle, CommandPoolHandle, FramebufferDesc, FramebufferHandle, PipelineDesc,
    PipelineHandle, RenderPassDesc, RenderPassHandle, RhiError, RhiResult, ShaderDesc,
    ShaderHandle, SwapChainDesc, SwapChainHandle, TextureDesc, TextureHandle,
};

pub trait Device {
    fn capabilities(&self) -> &DeviceCapabilities;

    fn create_buffer(&self, desc: &BufferDesc) -> RhiResult<BufferHandle>;
    fn destroy_buffer(&self, buffer: BufferHandle);
    fn map_buffer(&self, buffer: BufferHandle) -> RhiResult<*mut u8>;
    fn unmap_buffer(&self, buffer: BufferHandle);
    fn write_buffer(&self, buffer: BufferHandle, offset: usize, data: &[u8]) -> RhiResult<()>;

    fn create_texture(&self, desc: &TextureDesc) -> RhiResult<TextureHandle>;
    fn destroy_texture(&self, texture: TextureHandle);

    fn create_shader(&self, desc: &ShaderDesc) -> RhiResult<ShaderHandle>;
    fn destroy_shader(&self, shader: ShaderHandle);

    fn create_pipeline(&self, desc: &PipelineDesc) -> RhiResult<PipelineHandle>;
    fn destroy_pipeline(&self, pipeline: PipelineHandle);

    fn create_render_pass(&self, desc: &RenderPassDesc) -> RhiResult<RenderPassHandle>;
    fn destroy_render_pass(&self, pass: RenderPassHandle);

    fn create_command_pool(&self) -> RhiResult<CommandPoolHandle>;
    fn destroy_command_pool(&self, pool: CommandPoolHandle);
    fn create_command_buffer(
        &self,
        pool: CommandPoolHandle,
    ) -> RhiResult<crate::CommandBufferHandle>;
    fn reset_command_pool(&self, pool: CommandPoolHandle);

    fn create_swapchain(&self, desc: &SwapChainDesc) -> RhiResult<SwapChainHandle>;
    fn destroy_swapchain(&self, swapchain: SwapChainHandle);
    fn resize_swapchain(
        &self,
        swapchain: SwapChainHandle,
        width: u32,
        height: u32,
    ) -> RhiResult<()>;

    fn create_framebuffer(&self, desc: &FramebufferDesc) -> RhiResult<FramebufferHandle>;
    fn destroy_framebuffer(&self, framebuffer: FramebufferHandle);

    fn wait_idle(&self) -> RhiResult<()>;

    fn begin_frame(&self) -> RhiResult<u32>;
    fn end_frame(&self) -> RhiResult<()>;
    fn present(&self) -> RhiResult<()>;
}

#[derive(Clone, Debug)]
pub struct DeviceCapabilities {
    pub max_texture_size: u32,
    pub max_texture_layers: u32,
    pub max_vertex_attributes: u32,
    pub max_uniform_buffers: u32,
    pub max_sampled_textures: u32,
    pub max_anisotropy: f32,
    pub supports_compute: bool,
    pub supports_tessellation: bool,
    pub supports_geometry_shader: bool,
    pub supports_ray_tracing: bool,
    pub supports_mesh_shader: bool,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            max_texture_size: 8192,
            max_texture_layers: 256,
            max_vertex_attributes: 16,
            max_uniform_buffers: 16,
            max_sampled_textures: 32,
            max_anisotropy: 16.0,
            supports_compute: true,
            supports_tessellation: false,
            supports_geometry_shader: false,
            supports_ray_tracing: false,
            supports_mesh_shader: false,
        }
    }
}
