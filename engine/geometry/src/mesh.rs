use crate::{Vertex, BoundingBox};
use nalgebra::Vector3;

#[derive(Clone, Debug)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub bounding_box: BoundingBox,
}

impl MeshData {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        let bounding_box = Self::calculate_bounding_box(&vertices);
        Self {
            vertices,
            indices,
            bounding_box,
        }
    }

    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounding_box: BoundingBox::empty(),
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_buffer_size(&self) -> usize {
        self.vertices.len() * std::mem::size_of::<Vertex>()
    }

    pub fn index_buffer_size(&self) -> usize {
        self.indices.len() * std::mem::size_of::<u32>()
    }

    pub fn calculate_normals(&mut self) {
        for vertex in &mut self.vertices {
            vertex.normal = [0.0, 0.0, 0.0];
        }

        for chunk in self.indices.chunks(3) {
            if chunk.len() == 3 {
                let i0 = chunk[0] as usize;
                let i1 = chunk[1] as usize;
                let i2 = chunk[2] as usize;

                if i0 < self.vertices.len() && i1 < self.vertices.len() && i2 < self.vertices.len() {
                    let v0 = Vector3::from(self.vertices[i0].position);
                    let v1 = Vector3::from(self.vertices[i1].position);
                    let v2 = Vector3::from(self.vertices[i2].position);

                    let edge1 = v1 - v0;
                    let edge2 = v2 - v0;
                    let normal = edge1.cross(&edge2);

                    for idx in chunk {
                        let n = &mut self.vertices[*idx as usize].normal;
                        n[0] += normal.x;
                        n[1] += normal.y;
                        n[2] += normal.z;
                    }
                }
            }
        }

        for vertex in &mut self.vertices {
            let n = Vector3::from(vertex.normal);
            if let Some(normalized) = n.try_normalize(1e-6) {
                vertex.normal = [normalized.x, normalized.y, normalized.z];
            }
        }
    }

    pub fn calculate_tangents(&mut self) {
        for vertex in &mut self.vertices {
            vertex.tangent = [1.0, 0.0, 0.0];
            vertex.bitangent = [0.0, 0.0, 1.0];
        }

        for chunk in self.indices.chunks(3) {
            if chunk.len() == 3 {
                let i0 = chunk[0] as usize;
                let i1 = chunk[1] as usize;
                let i2 = chunk[2] as usize;

                if i0 < self.vertices.len() && i1 < self.vertices.len() && i2 < self.vertices.len() {
                    let v0 = &self.vertices[i0];
                    let v1 = &self.vertices[i1];
                    let v2 = &self.vertices[i2];

                    let pos0 = Vector3::from(v0.position);
                    let pos1 = Vector3::from(v1.position);
                    let pos2 = Vector3::from(v2.position);

                    let uv0 = Vector3::new(v0.uv[0], v0.uv[1], 0.0);
                    let uv1 = Vector3::new(v1.uv[0], v1.uv[1], 0.0);
                    let uv2 = Vector3::new(v2.uv[0], v2.uv[1], 0.0);

                    let edge1 = pos1 - pos0;
                    let edge2 = pos2 - pos0;
                    let delta_uv1 = uv1 - uv0;
                    let delta_uv2 = uv2 - uv0;

                    let det = delta_uv1.x * delta_uv2.y - delta_uv2.x * delta_uv1.y;
                    if det.abs() > 1e-6 {
                        let inv_det = 1.0 / det;
                        let tangent = (edge1 * delta_uv2.y - edge2 * delta_uv1.y) * inv_det;
                        let bitangent = (edge2 * delta_uv1.x - edge1 * delta_uv2.x) * inv_det;

                        for idx in chunk {
                            let v = &mut self.vertices[*idx as usize];
                            v.tangent[0] += tangent.x;
                            v.tangent[1] += tangent.y;
                            v.tangent[2] += tangent.z;
                            v.bitangent[0] += bitangent.x;
                            v.bitangent[1] += bitangent.y;
                            v.bitangent[2] += bitangent.z;
                        }
                    }
                }
            }
        }

        for vertex in &mut self.vertices {
            let t = Vector3::from(vertex.tangent);
            if let Some(normalized) = t.try_normalize(1e-6) {
                vertex.tangent = [normalized.x, normalized.y, normalized.z];
            }
            let b = Vector3::from(vertex.bitangent);
            if let Some(normalized) = b.try_normalize(1e-6) {
                vertex.bitangent = [normalized.x, normalized.y, normalized.z];
            }
        }
    }

    fn calculate_bounding_box(vertices: &[Vertex]) -> BoundingBox {
        if vertices.is_empty() {
            return BoundingBox::empty();
        }

        let mut min = Vector3::from(vertices[0].position);
        let mut max = Vector3::from(vertices[0].position);

        for vertex in &vertices[1..] {
            let pos = Vector3::from(vertex.position);
            min = min.inf(&pos);
            max = max.sup(&pos);
        }

        BoundingBox::new(min, max)
    }

    pub fn create_triangle() -> Self {
        let vertices = vec![
            Vertex::new([0.0, 0.5, 0.0]).with_uv([0.5, 1.0]).with_color([1.0, 0.0, 0.0, 1.0]),
            Vertex::new([-0.5, -0.5, 0.0]).with_uv([0.0, 0.0]).with_color([0.0, 1.0, 0.0, 1.0]),
            Vertex::new([0.5, -0.5, 0.0]).with_uv([1.0, 0.0]).with_color([0.0, 0.0, 1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        let mut mesh = Self::new(vertices, indices);
        mesh.calculate_normals();
        mesh
    }

    pub fn create_quad() -> Self {
        let vertices = vec![
            Vertex::new([-0.5, -0.5, 0.0]).with_uv([0.0, 0.0]).with_color([1.0, 1.0, 1.0, 1.0]),
            Vertex::new([0.5, -0.5, 0.0]).with_uv([1.0, 0.0]).with_color([1.0, 1.0, 1.0, 1.0]),
            Vertex::new([0.5, 0.5, 0.0]).with_uv([1.0, 1.0]).with_color([1.0, 1.0, 1.0, 1.0]),
            Vertex::new([-0.5, 0.5, 0.0]).with_uv([0.0, 1.0]).with_color([1.0, 1.0, 1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2, 0, 2, 3];
        let mut mesh = Self::new(vertices, indices);
        mesh.calculate_normals();
        mesh
    }

    pub fn create_cube() -> Self {
        let vertices = vec![
            Vertex::new([-0.5, -0.5, -0.5]).with_uv([0.0, 0.0]).with_normal([0.0, 0.0, -1.0]),
            Vertex::new([0.5, -0.5, -0.5]).with_uv([1.0, 0.0]).with_normal([0.0, 0.0, -1.0]),
            Vertex::new([0.5, 0.5, -0.5]).with_uv([1.0, 1.0]).with_normal([0.0, 0.0, -1.0]),
            Vertex::new([-0.5, 0.5, -0.5]).with_uv([0.0, 1.0]).with_normal([0.0, 0.0, -1.0]),
            Vertex::new([-0.5, -0.5, 0.5]).with_uv([0.0, 0.0]).with_normal([0.0, 0.0, 1.0]),
            Vertex::new([0.5, -0.5, 0.5]).with_uv([1.0, 0.0]).with_normal([0.0, 0.0, 1.0]),
            Vertex::new([0.5, 0.5, 0.5]).with_uv([1.0, 1.0]).with_normal([0.0, 0.0, 1.0]),
            Vertex::new([-0.5, 0.5, 0.5]).with_uv([0.0, 1.0]).with_normal([0.0, 0.0, 1.0]),
            Vertex::new([-0.5, 0.5, -0.5]).with_uv([0.0, 0.0]).with_normal([0.0, 1.0, 0.0]),
            Vertex::new([0.5, 0.5, -0.5]).with_uv([1.0, 0.0]).with_normal([0.0, 1.0, 0.0]),
            Vertex::new([0.5, 0.5, 0.5]).with_uv([1.0, 1.0]).with_normal([0.0, 1.0, 0.0]),
            Vertex::new([-0.5, 0.5, 0.5]).with_uv([0.0, 1.0]).with_normal([0.0, 1.0, 0.0]),
            Vertex::new([-0.5, -0.5, -0.5]).with_uv([0.0, 0.0]).with_normal([0.0, -1.0, 0.0]),
            Vertex::new([0.5, -0.5, -0.5]).with_uv([1.0, 0.0]).with_normal([0.0, -1.0, 0.0]),
            Vertex::new([0.5, -0.5, 0.5]).with_uv([1.0, 1.0]).with_normal([0.0, -1.0, 0.0]),
            Vertex::new([-0.5, -0.5, 0.5]).with_uv([0.0, 1.0]).with_normal([0.0, -1.0, 0.0]),
            Vertex::new([0.5, -0.5, -0.5]).with_uv([0.0, 0.0]).with_normal([1.0, 0.0, 0.0]),
            Vertex::new([0.5, 0.5, -0.5]).with_uv([1.0, 0.0]).with_normal([1.0, 0.0, 0.0]),
            Vertex::new([0.5, 0.5, 0.5]).with_uv([1.0, 1.0]).with_normal([1.0, 0.0, 0.0]),
            Vertex::new([0.5, -0.5, 0.5]).with_uv([0.0, 1.0]).with_normal([1.0, 0.0, 0.0]),
            Vertex::new([-0.5, -0.5, -0.5]).with_uv([0.0, 0.0]).with_normal([-1.0, 0.0, 0.0]),
            Vertex::new([-0.5, 0.5, -0.5]).with_uv([1.0, 0.0]).with_normal([-1.0, 0.0, 0.0]),
            Vertex::new([-0.5, 0.5, 0.5]).with_uv([1.0, 1.0]).with_normal([-1.0, 0.0, 0.0]),
            Vertex::new([-0.5, -0.5, 0.5]).with_uv([0.0, 1.0]).with_normal([-1.0, 0.0, 0.0]),
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3,
            4, 6, 5, 4, 7, 6,
            8, 9, 10, 8, 10, 11,
            12, 14, 13, 12, 15, 14,
            16, 17, 18, 16, 18, 19,
            20, 22, 21, 20, 23, 22,
        ];
        Self::new(vertices, indices)
    }
}