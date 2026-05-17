pub mod buffer;
pub mod command;
pub mod device;
pub mod error;
pub mod framebuffer;
pub mod handle;
pub mod pass;
pub mod pipeline;
pub mod shader;
pub mod swapchain;
pub mod texture;
pub mod ui;

pub use buffer::{BufferDesc, BufferType, BufferUsage, MemoryLocation};
pub use command::{ClearValue, CommandBuffer, IndexType};
pub use device::{Device, DeviceCapabilities};
pub use error::RhiError;
pub use framebuffer::FramebufferDesc;
pub use handle::*;
pub use pass::{AttachmentDesc, AttachmentLoadOp, AttachmentStoreOp, ImageLayout, RenderPassDesc};
pub use pipeline::{
    BlendState, DepthStencilState, PipelineDesc, PipelineLayout, RasterizationState, ShaderStage,
};
pub use shader::{ShaderDesc, ShaderSource};
pub use swapchain::{ColorSpace, PresentMode, SwapChainDesc};
pub use texture::{TextureDesc, TextureFormat, TextureType, TextureUsage};
pub use ui::{UIDrawData, UIPipelineDesc, UIRenderTarget, UIRenderer, UIVertex};

pub type RhiResult<T> = Result<T, RhiError>;
