use hezhou_core::math::Vec3u;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    R8Unorm,
    R8Snorm,
    R8Uint,
    R8Sint,
    Rg8Unorm,
    Rg8Snorm,
    Rg8Uint,
    Rg8Sint,
    Rgba8Unorm,
    Rgba8Snorm,
    Rgba8Uint,
    Rgba8Sint,
    Bgra8Unorm,
    Bgra8Snorm,
    R16Unorm,
    R16Snorm,
    R16Uint,
    R16Sint,
    R16Float,
    Rg16Unorm,
    Rg16Snorm,
    Rg16Uint,
    Rg16Sint,
    Rg16Float,
    Rgba16Unorm,
    Rgba16Snorm,
    Rgba16Uint,
    Rgba16Sint,
    Rgba16Float,
    R32Uint,
    R32Sint,
    R32Float,
    Rg32Uint,
    Rg32Sint,
    Rg32Float,
    Rgba32Uint,
    Rgba32Sint,
    Rgba32Float,
    Depth16,
    Depth24,
    Depth32,
    Depth24Stencil8,
    Depth32Stencil8,
}

impl TextureFormat {
    pub fn size(&self) -> usize {
        match self {
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint => 1,
            TextureFormat::Rg8Unorm
            | TextureFormat::Rg8Snorm
            | TextureFormat::Rg8Uint
            | TextureFormat::Rg8Sint => 2,
            TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8Snorm => 4,
            TextureFormat::R16Unorm
            | TextureFormat::R16Snorm
            | TextureFormat::R16Uint
            | TextureFormat::R16Sint
            | TextureFormat::R16Float => 2,
            TextureFormat::Rg16Unorm
            | TextureFormat::Rg16Snorm
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Float => 4,
            TextureFormat::Rgba16Unorm
            | TextureFormat::Rgba16Snorm
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float => 8,
            TextureFormat::R32Uint | TextureFormat::R32Sint | TextureFormat::R32Float => 4,
            TextureFormat::Rg32Uint | TextureFormat::Rg32Sint | TextureFormat::Rg32Float => 8,
            TextureFormat::Rgba32Uint | TextureFormat::Rgba32Sint | TextureFormat::Rgba32Float => {
                16
            }
            TextureFormat::Depth16 => 2,
            TextureFormat::Depth24 => 3,
            TextureFormat::Depth32 => 4,
            TextureFormat::Depth24Stencil8 => 4,
            TextureFormat::Depth32Stencil8 => 8,
        }
    }

    pub fn is_depth(&self) -> bool {
        matches!(
            self,
            TextureFormat::Depth16
                | TextureFormat::Depth24
                | TextureFormat::Depth32
                | TextureFormat::Depth24Stencil8
                | TextureFormat::Depth32Stencil8
        )
    }

    pub fn is_stencil(&self) -> bool {
        matches!(
            self,
            TextureFormat::Depth24Stencil8 | TextureFormat::Depth32Stencil8
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureType {
    Texture1D,
    Texture2D,
    Texture3D,
    Texture2DArray,
    TextureCube,
    TextureCubeArray,
}

#[derive(Clone, Debug)]
pub struct TextureDesc {
    pub texture_type: TextureType,
    pub format: TextureFormat,
    pub extent: Vec3u,
    pub mip_levels: u32,
    pub array_layers: u32,
    pub samples: u32,
    pub usage: TextureUsage,
}

impl TextureDesc {
    pub fn texture2d(format: TextureFormat, width: u32, height: u32) -> Self {
        Self {
            texture_type: TextureType::Texture2D,
            format,
            extent: Vec3u::new(width, height, 1),
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            usage: TextureUsage::default(),
        }
    }

    pub fn render_target(format: TextureFormat, width: u32, height: u32) -> Self {
        Self {
            texture_type: TextureType::Texture2D,
            format,
            extent: Vec3u::new(width, height, 1),
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            usage: TextureUsage {
                sampled: true,
                render_target: true,
                ..Default::default()
            },
        }
    }

    pub fn depth_target(format: TextureFormat, width: u32, height: u32) -> Self {
        Self {
            texture_type: TextureType::Texture2D,
            format,
            extent: Vec3u::new(width, height, 1),
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            usage: TextureUsage {
                sampled: true,
                depth_stencil: true,
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextureUsage {
    pub sampled: bool,
    pub storage: bool,
    pub render_target: bool,
    pub depth_stencil: bool,
    pub transfer_src: bool,
    pub transfer_dst: bool,
}
