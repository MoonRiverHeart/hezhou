use crate::TextureFormat;

#[derive(Clone, Debug)]
pub struct SwapChainDesc {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub color_space: ColorSpace,
    pub present_mode: PresentMode,
    pub image_count: u32,
}

impl Default for SwapChainDesc {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            format: TextureFormat::Bgra8Unorm,
            color_space: ColorSpace::Srgb,
            present_mode: PresentMode::Fifo,
            image_count: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    Srgb,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentMode {
    Immediate,
    Fifo,
    Mailbox,
}