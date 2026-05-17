use thiserror::Error;

#[derive(Error, Debug)]
pub enum RhiError {
    #[error("Device creation failed: {0}")]
    DeviceCreationFailed(String),

    #[error("Buffer creation failed: {0}")]
    BufferCreationFailed(String),

    #[error("Texture creation failed: {0}")]
    TextureCreationFailed(String),

    #[error("Shader compilation failed: {0}")]
    ShaderCompilationFailed(String),

    #[error("Pipeline creation failed: {0}")]
    PipelineCreationFailed(String),

    #[error("Render pass creation failed: {0}")]
    RenderPassCreationFailed(String),

    #[error("Framebuffer creation failed: {0}")]
    FramebufferCreationFailed(String),

    #[error("Command pool creation failed: {0}")]
    CommandPoolCreationFailed(String),

    #[error("Mapping failed: {0}")]
    MappingFailed(String),

    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Swapchain creation failed: {0}")]
    SwapchainCreationFailed(String),

    #[error("Surface creation failed: {0}")]
    SurfaceCreationFailed(String),

    #[error("Surface lost")]
    SurfaceLost,

    #[error("Device lost")]
    DeviceLost,

    #[error("Unknown error: {0}")]
    Unknown(String),
}
