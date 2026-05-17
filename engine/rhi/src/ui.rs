use crate::buffer::BufferDesc;
use crate::error::RhiError;
use crate::handle::*;
use crate::shader::{ShaderDesc, ShaderSource};
use crate::texture::{TextureDesc, TextureFormat};

#[repr(C)]
pub struct UIRenderTarget {
    width: u32,
    height: u32,
    format: TextureFormat,
    texture: Option<TextureHandle>,
}

impl UIRenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::Rgba8Unorm,
            texture: None,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn desc(&self) -> TextureDesc {
        TextureDesc::render_target(self.format, self.width, self.height)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UIVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl UIVertex {
    pub fn new(x: f32, y: f32, r: f32, g: f32, b: f32, a: f32, u: f32, v: f32) -> Self {
        Self {
            position: [x, y],
            color: [r, g, b, a],
            uv: [u, v],
        }
    }

    pub fn position_only(x: f32, y: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(x, y, r, g, b, a, 0.0, 0.0)
    }
}

#[repr(C)]
pub struct UIDrawData {
    pub vertices: Vec<UIVertex>,
    pub indices: Vec<u32>,
    pub texture: Option<TextureHandle>,
}

impl UIDrawData {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
        }
    }

    pub fn add_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    ) {
        let v0 = UIVertex::position_only(x, y, r, g, b, a);
        let v1 = UIVertex::position_only(x + width, y, r, g, b, a);
        let v2 = UIVertex::position_only(x + width, y + height, r, g, b, a);
        let v3 = UIVertex::position_only(x, y + height, r, g, b, a);

        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[v0, v1, v2, v3]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub fn add_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        width: f32,
    ) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            return;
        }

        let nx = -dy / len * width / 2.0;
        let ny = dx / len * width / 2.0;

        let v0 = UIVertex::position_only(x1 - nx, y1 - ny, r, g, b, a);
        let v1 = UIVertex::position_only(x1 + nx, y1 + ny, r, g, b, a);
        let v2 = UIVertex::position_only(x2 + nx, y2 + ny, r, g, b, a);
        let v3 = UIVertex::position_only(x2 - nx, y2 - ny, r, g, b, a);

        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[v0, v1, v2, v3]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub fn vertex_data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<UIVertex>(),
            )
        }
    }

    pub fn index_data(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.indices.as_ptr() as *const u8,
                self.indices.len() * std::mem::size_of::<u32>(),
            )
        }
    }
}

impl Default for UIDrawData {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UIPipelineDesc {
    pub shader_vert: ShaderDesc,
    pub shader_frag: ShaderDesc,
}

impl UIPipelineDesc {
    pub fn new() -> Self {
        Self {
            shader_vert: ShaderDesc::vertex(ShaderSource::Spirv(Self::load_shader(
                include_bytes!("../../shaders/rotation.vert.spv"),
            ))),
            shader_frag: ShaderDesc::fragment(ShaderSource::Spirv(Self::load_shader(
                include_bytes!("../../shaders/rotation.frag.spv"),
            ))),
        }
    }

    fn load_shader(bytes: &[u8]) -> Vec<u32> {
        bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }
}

impl Default for UIPipelineDesc {
    fn default() -> Self {
        Self::new()
    }
}

pub trait UIRenderer {
    fn create_render_target(&mut self, width: u32, height: u32)
        -> Result<UIRenderTarget, RhiError>;
    fn destroy_render_target(&mut self, target: UIRenderTarget);

    fn begin_frame(&mut self, target: &UIRenderTarget);
    fn end_frame(&mut self);

    fn draw(&mut self, data: &UIDrawData);

    fn create_texture(
        &mut self,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<TextureHandle, RhiError>;
    fn destroy_texture(&mut self, texture: TextureHandle);
}
