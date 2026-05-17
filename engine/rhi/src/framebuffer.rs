use crate::{RenderPassHandle, TextureHandle};

#[derive(Clone, Debug)]
pub struct FramebufferDesc {
    pub render_pass: RenderPassHandle,
    pub attachments: Vec<TextureHandle>,
    pub width: u32,
    pub height: u32,
    pub layers: u32,
}
