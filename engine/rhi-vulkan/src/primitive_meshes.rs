use std::collections::HashMap;
use std::sync::OnceLock;
use hezhou_core::asset_library::MeshType;

/// Vertex format: position(vec3) + normal(vec3) = 24 bytes per vertex
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshRange {
    pub vertex_offset: u32,  // in vertices (not bytes!)
    pub vertex_count: u32,
}

pub struct PrimitiveMeshes {
    vertices: Vec<MeshVertex>,
    ranges: HashMap<MeshType, MeshRange>,
}

pub static PRIMITIVE_MESHES: OnceLock<PrimitiveMeshes> = OnceLock::new();

pub fn get_primitive_meshes() -> &'static PrimitiveMeshes {
    PRIMITIVE_MESHES.get_or_init(PrimitiveMeshes::new)
}

impl PrimitiveMeshes {
    pub fn new() -> Self {
        let mut vertices = Vec::new();
        let mut ranges = HashMap::new();

        // Cube: 36 vertices (6 faces × 6 vertices per face, flat normals)
        let cube_offset = vertices.len() as u32;
        Self::generate_cube(&mut vertices);
        ranges.insert(MeshType::Cube, MeshRange {
            vertex_offset: cube_offset,
            vertex_count: vertices.len() as u32 - cube_offset,
        });

        // Sphere: UV sphere
        let sphere_offset = vertices.len() as u32;
        Self::generate_sphere(&mut vertices);
        ranges.insert(MeshType::Sphere, MeshRange {
            vertex_offset: sphere_offset,
            vertex_count: vertices.len() as u32 - sphere_offset,
        });

        // Plane: 6 vertices
        let plane_offset = vertices.len() as u32;
        Self::generate_plane(&mut vertices);
        ranges.insert(MeshType::Plane, MeshRange {
            vertex_offset: plane_offset,
            vertex_count: vertices.len() as u32 - plane_offset,
        });

        // Cylinder
        let cylinder_offset = vertices.len() as u32;
        Self::generate_cylinder(&mut vertices);
        ranges.insert(MeshType::Cylinder, MeshRange {
            vertex_offset: cylinder_offset,
            vertex_count: vertices.len() as u32 - cylinder_offset,
        });

        // Cone
        let cone_offset = vertices.len() as u32;
        Self::generate_cone(&mut vertices);
        ranges.insert(MeshType::Cone, MeshRange {
            vertex_offset: cone_offset,
            vertex_count: vertices.len() as u32 - cone_offset,
        });

        Self { vertices, ranges }
    }

    pub fn vertices(&self) -> &[MeshVertex] {
        &self.vertices
    }

    pub fn ranges(&self) -> &HashMap<MeshType, MeshRange> {
        &self.ranges
    }

    pub fn get_range(&self, mesh_type: &MeshType) -> Option<&MeshRange> {
        self.ranges.get(mesh_type)
    }

    fn push_triangle(vertices: &mut Vec<MeshVertex>, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], normal: [f32; 3]) {
        vertices.push(MeshVertex { position: p0, normal });
        vertices.push(MeshVertex { position: p1, normal });
        vertices.push(MeshVertex { position: p2, normal });
    }

    /// Cube: 1×1×1 centered at origin, 36 vertices with flat normals
    fn generate_cube(vertices: &mut Vec<MeshVertex>) {
        // 6 faces, each with 2 triangles = 6 vertices per face
        // Front face (Z+)
        Self::push_triangle(vertices, [-0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [ 0.5, -0.5,  0.5], [0.0, 0.0, 1.0]);
        Self::push_triangle(vertices, [-0.5, -0.5,  0.5], [-0.5,  0.5,  0.5], [ 0.5,  0.5,  0.5], [0.0, 0.0, 1.0]);
        // Back face (Z-)
        Self::push_triangle(vertices, [ 0.5, -0.5, -0.5], [-0.5,  0.5, -0.5], [-0.5, -0.5, -0.5], [0.0, 0.0, -1.0]);
        Self::push_triangle(vertices, [ 0.5, -0.5, -0.5], [ 0.5,  0.5, -0.5], [-0.5,  0.5, -0.5], [0.0, 0.0, -1.0]);
        // Left face (X-)
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [-0.5,  0.5,  0.5], [-0.5, -0.5,  0.5], [-1.0, 0.0, 0.0]);
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [-0.5,  0.5, -0.5], [-0.5,  0.5,  0.5], [-1.0, 0.0, 0.0]);
        // Right face (X+)
        Self::push_triangle(vertices, [ 0.5, -0.5,  0.5], [ 0.5,  0.5, -0.5], [ 0.5, -0.5, -0.5], [1.0, 0.0, 0.0]);
        Self::push_triangle(vertices, [ 0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [ 0.5,  0.5, -0.5], [1.0, 0.0, 0.0]);
        // Top face (Y+)
        Self::push_triangle(vertices, [-0.5,  0.5,  0.5], [ 0.5,  0.5, -0.5], [ 0.5,  0.5,  0.5], [0.0, 1.0, 0.0]);
        Self::push_triangle(vertices, [-0.5,  0.5,  0.5], [-0.5,  0.5, -0.5], [ 0.5,  0.5, -0.5], [0.0, 1.0, 0.0]);
        // Bottom face (Y-)
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [ 0.5, -0.5,  0.5], [-0.5, -0.5,  0.5], [0.0, -1.0, 0.0]);
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [ 0.5, -0.5, -0.5], [ 0.5, -0.5,  0.5], [0.0, -1.0, 0.0]);
    }

    /// Sphere: UV sphere, 16 longitude × 12 latitude + 2 poles
    fn generate_sphere(vertices: &mut Vec<MeshVertex>) {
        let radius = 0.5;
        let lon_segments = 16;
        let lat_bands = 12;

        // Top pole (latitude band 0)
        let top_y = radius;
        for i in 0..lon_segments {
            let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
            let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;
            let lat1 = std::f32::consts::PI * 1.0 / lat_bands as f32;
            let y1 = radius * lat1.cos();
            let r1 = radius * lat1.sin();

            let p0 = [0.0, top_y, 0.0];
            let p1 = [r1 * lon0.cos(), y1, r1 * lon0.sin()];
            let p2 = [r1 * lon1.cos(), y1, r1 * lon1.sin()];

            let n0 = Self::normalize3(p0);
            let n1 = Self::normalize3(p1);
            let n2 = Self::normalize3(p2);

            vertices.push(MeshVertex { position: p0, normal: n0 });
            vertices.push(MeshVertex { position: p2, normal: n2 });
            vertices.push(MeshVertex { position: p1, normal: n1 });
        }

        // Middle bands
        for j in 1..(lat_bands - 1) {
            let lat0 = std::f32::consts::PI * j as f32 / lat_bands as f32;
            let lat1 = std::f32::consts::PI * (j + 1) as f32 / lat_bands as f32;
            let y0 = radius * lat0.cos();
            let r0 = radius * lat0.sin();
            let y1 = radius * lat1.cos();
            let r1 = radius * lat1.sin();

            for i in 0..lon_segments {
                let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
                let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;

                let p00 = [r0 * lon0.cos(), y0, r0 * lon0.sin()];
                let p01 = [r0 * lon1.cos(), y0, r0 * lon1.sin()];
                let p10 = [r1 * lon0.cos(), y1, r1 * lon0.sin()];
                let p11 = [r1 * lon1.cos(), y1, r1 * lon1.sin()];

                let n00 = Self::normalize3(p00);
                let n01 = Self::normalize3(p01);
                let n10 = Self::normalize3(p10);
                let n11 = Self::normalize3(p11);

                // Triangle 1
                vertices.push(MeshVertex { position: p00, normal: n00 });
                vertices.push(MeshVertex { position: p11, normal: n11 });
                vertices.push(MeshVertex { position: p01, normal: n01 });
                // Triangle 2
                vertices.push(MeshVertex { position: p00, normal: n00 });
                vertices.push(MeshVertex { position: p10, normal: n10 });
                vertices.push(MeshVertex { position: p11, normal: n11 });
            }
        }

        // Bottom pole (latitude band lat_bands)
        let bottom_y = -radius;
        for i in 0..lon_segments {
            let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
            let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;
            let lat0 = std::f32::consts::PI * (lat_bands - 1) as f32 / lat_bands as f32;
            let y0 = radius * lat0.cos();
            let r0 = radius * lat0.sin();

            let p0 = [0.0, bottom_y, 0.0];
            let p1 = [r0 * lon0.cos(), y0, r0 * lon0.sin()];
            let p2 = [r0 * lon1.cos(), y0, r0 * lon1.sin()];

            let n0 = Self::normalize3(p0);
            let n1 = Self::normalize3(p1);
            let n2 = Self::normalize3(p2);

            vertices.push(MeshVertex { position: p0, normal: n0 });
            vertices.push(MeshVertex { position: p1, normal: n1 });
            vertices.push(MeshVertex { position: p2, normal: n2 });
        }
    }

    /// Plane: 1×1 centered at origin, normal = [0, 1, 0]
    fn generate_plane(vertices: &mut Vec<MeshVertex>) {
        let normal = [0.0, 1.0, 0.0];
        Self::push_triangle(vertices, [-0.5, 0.0, -0.5], [ 0.5, 0.0,  0.5], [ 0.5, 0.0, -0.5], normal);
        Self::push_triangle(vertices, [-0.5, 0.0, -0.5], [-0.5, 0.0,  0.5], [ 0.5, 0.0,  0.5], normal);
    }

    /// Cylinder: radius 0.5, height 1, centered at origin
    fn generate_cylinder(vertices: &mut Vec<MeshVertex>) {
        let radius = 0.5;
        let half_height = 0.5;
        let segments = 16;

        // Side faces
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let x0 = radius * angle0.cos();
            let z0 = radius * angle0.sin();
            let x1 = radius * angle1.cos();
            let z1 = radius * angle1.sin();

            // Side normal = normalized radial direction
            let nx0 = angle0.cos();
            let nz0 = angle0.sin();
            let nx1 = angle1.cos();
            let nz1 = angle1.sin();

            // Triangle 1 (top-left, bottom-left, bottom-right)
            Self::push_triangle(vertices,
                [x0, half_height, z0],  // top-left
                [x0, -half_height, z0],  // bottom-left
                [x1, -half_height, z1],  // bottom-right
                [nx0, 0.0, nz0],
            );
            // Triangle 2 (top-left, bottom-right, top-right)
            Self::push_triangle(vertices,
                [x0, half_height, z0],  // top-left
                [x1, -half_height, z1],  // bottom-right
                [x1, half_height, z1],  // top-right
                [nx1, 0.0, nz1],
            );
        }

        // Top cap (Y+)
        let top_normal = [0.0, 1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            Self::push_triangle(vertices,
                [0.0, half_height, 0.0],  // center
                [radius * angle0.cos(), half_height, radius * angle0.sin()],
                [radius * angle1.cos(), half_height, radius * angle1.sin()],
                top_normal,
            );
        }

        // Bottom cap (Y-)
        let bottom_normal = [0.0, -1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            Self::push_triangle(vertices,
                [0.0, -half_height, 0.0],  // center
                [radius * angle1.cos(), -half_height, radius * angle1.sin()],
                [radius * angle0.cos(), -half_height, radius * angle0.sin()],
                bottom_normal,
            );
        }
    }

    /// Cone: base radius 0.5, height 1, centered at origin
    fn generate_cone(vertices: &mut Vec<MeshVertex>) {
        let base_radius = 0.5;
        let half_height = 0.5;
        let segments = 16;
        let tip_y = half_height;
        let base_y = -half_height;

        // Side faces
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let x0 = base_radius * angle0.cos();
            let z0 = base_radius * angle0.sin();
            let x1 = base_radius * angle1.cos();
            let z1 = base_radius * angle1.sin();

            // Compute side normal from slant direction
            // Slant vector from base to tip: (0 - x0, tip_y - base_y, 0 - z0)
            // Normal = cross(slant, tangent) where tangent = (x1-x0, 0, z1-z0)
            let slant = [-x0, tip_y - base_y, -z0];
            let tangent = [x1 - x0, 0.0, z1 - z0];
            let n = Self::normalize3(Self::cross3(slant, tangent));

            Self::push_triangle(vertices,
                [0.0, tip_y, 0.0],  // tip
                [x0, base_y, z0],
                [x1, base_y, z1],
                n,
            );
        }

        // Bottom cap (Y-)
        let bottom_normal = [0.0, -1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            Self::push_triangle(vertices,
                [0.0, base_y, 0.0],  // center
                [base_radius * angle1.cos(), base_y, base_radius * angle1.sin()],
                [base_radius * angle0.cos(), base_y, base_radius * angle0.sin()],
                bottom_normal,
            );
        }
    }

    fn normalize3(v: [f32; 3]) -> [f32; 3] {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 0.0 {
            [v[0] / len, v[1] / len, v[2] / len]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
}

/// Parse mesh_path string to MeshType
pub fn mesh_type_from_path(path: &str) -> MeshType {
    match path {
        "builtin://cube" => MeshType::Cube,
        "builtin://sphere" => MeshType::Sphere,
        "builtin://plane" => MeshType::Plane,
        "builtin://cylinder" => MeshType::Cylinder,
        "builtin://cone" => MeshType::Cone,
        _ => MeshType::Cube, // fallback
    }
}