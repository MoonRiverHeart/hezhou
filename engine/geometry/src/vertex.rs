use nalgebra::{Vector2, Vector3, Vector4};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            tangent: [1.0, 0.0, 0.0],
            bitangent: [0.0, 0.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn with_normal(mut self, normal: [f32; 3]) -> Self {
        self.normal = normal;
        self
    }

    pub fn with_uv(mut self, uv: [f32; 2]) -> Self {
        self.uv = uv;
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn with_tangent(mut self, tangent: [f32; 3], bitangent: [f32; 3]) -> Self {
        self.tangent = tangent;
        self.bitangent = bitangent;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexAttribute {
    Position,
    Normal,
    UV,
    Tangent,
    Bitangent,
    Color,
    Custom(u32),
}

impl VertexAttribute {
    pub fn location(&self) -> u32 {
        match self {
            VertexAttribute::Position => 0,
            VertexAttribute::Normal => 1,
            VertexAttribute::UV => 2,
            VertexAttribute::Tangent => 3,
            VertexAttribute::Bitangent => 4,
            VertexAttribute::Color => 5,
            VertexAttribute::Custom(i) => 10 + i,
        }
    }

    pub fn format(&self) -> VertexFormat {
        match self {
            VertexAttribute::Position => VertexFormat::Float3,
            VertexAttribute::Normal => VertexFormat::Float3,
            VertexAttribute::UV => VertexFormat::Float2,
            VertexAttribute::Tangent => VertexFormat::Float3,
            VertexAttribute::Bitangent => VertexFormat::Float3,
            VertexAttribute::Color => VertexFormat::Float4,
            VertexAttribute::Custom(_) => VertexFormat::Float4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VertexFormat {
    Float,
    Float2,
    Float3,
    Float4,
    Int,
    Int2,
    Int3,
    Int4,
}

impl VertexFormat {
    pub fn size(&self) -> usize {
        match self {
            VertexFormat::Float => 4,
            VertexFormat::Float2 => 8,
            VertexFormat::Float3 => 12,
            VertexFormat::Float4 => 16,
            VertexFormat::Int => 4,
            VertexFormat::Int2 => 8,
            VertexFormat::Int3 => 12,
            VertexFormat::Int4 => 16,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VertexLayout {
    pub attributes: Vec<(VertexAttribute, usize)>,
    pub stride: usize,
}

impl VertexLayout {
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
            stride: 0,
        }
    }

    pub fn add(mut self, attr: VertexAttribute) -> Self {
        let offset = self.stride;
        self.stride += attr.format().size();
        self.attributes.push((attr, offset));
        self
    }

    pub fn default_vertex() -> Self {
        Self::new()
            .add(VertexAttribute::Position)
            .add(VertexAttribute::Normal)
            .add(VertexAttribute::UV)
            .add(VertexAttribute::Tangent)
            .add(VertexAttribute::Bitangent)
            .add(VertexAttribute::Color)
    }
}

impl Default for VertexLayout {
    fn default() -> Self {
        Self::default_vertex()
    }
}