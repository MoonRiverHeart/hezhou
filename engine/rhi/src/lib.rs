pub mod handle;
pub mod device;
pub mod swapchain;
pub mod command;
pub mod pipeline;
pub mod buffer;
pub mod texture;
pub mod shader;
pub mod pass;
pub mod framebuffer;
pub mod error;

pub use handle::*;
pub use device::{Device, DeviceCapabilities};
pub use swapchain::{SwapChainDesc, ColorSpace, PresentMode};
pub use command::{CommandBuffer, IndexType, ClearValue};
pub use pipeline::{PipelineDesc, ShaderStage, RasterizationState, DepthStencilState, BlendState, PipelineLayout};
pub use buffer::{BufferDesc, BufferType, BufferUsage, MemoryLocation};
pub use texture::{TextureDesc, TextureFormat, TextureType, TextureUsage};
pub use shader::{ShaderDesc, ShaderSource};
pub use pass::{RenderPassDesc, AttachmentDesc, AttachmentLoadOp, AttachmentStoreOp, ImageLayout};
pub use framebuffer::FramebufferDesc;
pub use error::RhiError;

pub type RhiResult<T> = Result<T, RhiError>;