use crate::color::Color;
use hezhou_core::math::{Mat4, Transform, Vec3};

pub type MeshId = u64;

#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec3,
    pub color: Color,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Vec3::zero(),
            normal: Vec3::up(),
            uv: Vec3::zero(),
            color: Color::white(),
        }
    }
}

pub struct Mesh {
    pub id: MeshId,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub transform: Transform,
}

impl Mesh {
    pub fn new(id: MeshId) -> Self {
        Self {
            id,
            vertices: Vec::new(),
            indices: Vec::new(),
            transform: Transform::new(),
        }
    }

    pub fn create_triangle(id: MeshId) -> Self {
        let vertices = vec![
            Vertex {
                position: Vec3::new(-0.5, -0.5, 0.0),
                normal: Vec3::forward(),
                uv: Vec3::new(0.0, 0.0, 0.0),
                color: Color::red(),
            },
            Vertex {
                position: Vec3::new(0.5, -0.5, 0.0),
                normal: Vec3::forward(),
                uv: Vec3::new(1.0, 0.0, 0.0),
                color: Color::green(),
            },
            Vertex {
                position: Vec3::new(0.0, 0.5, 0.0),
                normal: Vec3::forward(),
                uv: Vec3::new(0.5, 1.0, 0.0),
                color: Color::blue(),
            },
        ];

        Self {
            id,
            vertices,
            indices: vec![0, 1, 2],
            transform: Transform::new(),
        }
    }

    pub fn create_quad(id: MeshId) -> Self {
        let vertices = vec![
            Vertex {
                position: Vec3::new(-0.5, -0.5, 0.0),
                normal: Vec3::forward(),
                uv: Vec3::new(0.0, 0.0, 0.0),
                color: Color::white(),
            },
            Vertex {
                position: Vec3::new(0.5, -0.5, 0.0),
                normal: Vec3::forward(),
                uv: Vec3::new(1.0, 0.0, 0.0),
                color: Color::white(),
            },
            Vertex {
                position: Vec3::new(0.5, 0.5, 0.0),
                normal: Vec3::forward(),
                uv: Vec3::new(1.0, 1.0, 0.0),
                color: Color::white(),
            },
            Vertex {
                position: Vec3::new(-0.5, 0.5, 0.0),
                normal: Vec3::forward(),
                uv: Vec3::new(0.0, 1.0, 0.0),
                color: Color::white(),
            },
        ];

        Self {
            id,
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            transform: Transform::new(),
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.transform.position = Vec3::new(x, y, z);
    }

    pub fn set_rotation(&mut self, quat: hezhou_core::Quaternion) {
        self.transform.rotation = quat;
    }

    pub fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        self.transform.scale = Vec3::new(x, y, z);
    }

    pub fn get_model_matrix(&self) -> Mat4 {
        self.transform.to_matrix()
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }
}
