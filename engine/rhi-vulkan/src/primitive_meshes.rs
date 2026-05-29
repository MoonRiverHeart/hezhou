use std::collections::HashMap;
use std::sync::OnceLock;
use hezhou_core::asset_library::MeshType;

/// Vertex format: position(vec3) + normal(vec3) + uv(vec2) = 32 bytes per vertex
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
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

        // Bunny: 低多边形兔子模型（程序化生成）
        let bunny_offset = vertices.len() as u32;
        Self::generate_bunny(&mut vertices);
        ranges.insert(MeshType::Bunny, MeshRange {
            vertex_offset: bunny_offset,
            vertex_count: vertices.len() as u32 - bunny_offset,
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

    fn push_triangle(vertices: &mut Vec<MeshVertex>, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], normal: [f32; 3], uv0: [f32; 2], uv1: [f32; 2], uv2: [f32; 2]) {
        vertices.push(MeshVertex { position: p0, normal, uv: uv0 });
        vertices.push(MeshVertex { position: p1, normal, uv: uv1 });
        vertices.push(MeshVertex { position: p2, normal, uv: uv2 });
    }

    /// Cube: 1×1×1 centered at origin, 36 vertices with flat normals
    fn generate_cube(vertices: &mut Vec<MeshVertex>) {
        // 6 faces, each with 2 triangles = 6 vertices per face
        // 每个面UV映射: tri1=[0,0],[1,1],[1,0], tri2=[0,0],[0,1],[1,1]
        // Front face (Z+)
        Self::push_triangle(vertices, [-0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [ 0.5, -0.5,  0.5], [0.0, 0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]);
        Self::push_triangle(vertices, [-0.5, -0.5,  0.5], [-0.5,  0.5,  0.5], [ 0.5,  0.5,  0.5], [0.0, 0.0, 1.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        // Back face (Z-)
        Self::push_triangle(vertices, [ 0.5, -0.5, -0.5], [-0.5,  0.5, -0.5], [-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]);
        Self::push_triangle(vertices, [ 0.5, -0.5, -0.5], [ 0.5,  0.5, -0.5], [-0.5,  0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        // Left face (X-)
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [-0.5,  0.5,  0.5], [-0.5, -0.5,  0.5], [-1.0, 0.0, 0.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]);
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [-0.5,  0.5, -0.5], [-0.5,  0.5,  0.5], [-1.0, 0.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        // Right face (X+)
        Self::push_triangle(vertices, [ 0.5, -0.5,  0.5], [ 0.5,  0.5, -0.5], [ 0.5, -0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]);
        Self::push_triangle(vertices, [ 0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [ 0.5,  0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        // Top face (Y+)
        Self::push_triangle(vertices, [-0.5,  0.5,  0.5], [ 0.5,  0.5, -0.5], [ 0.5,  0.5,  0.5], [0.0, 1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]);
        Self::push_triangle(vertices, [-0.5,  0.5,  0.5], [-0.5,  0.5, -0.5], [ 0.5,  0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        // Bottom face (Y-)
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [ 0.5, -0.5,  0.5], [-0.5, -0.5,  0.5], [0.0, -1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]);
        Self::push_triangle(vertices, [-0.5, -0.5, -0.5], [ 0.5, -0.5, -0.5], [ 0.5, -0.5,  0.5], [0.0, -1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
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

            // UV: pole=[u_center, 0], 其他按经纬度
            let u_center = (lon0 + lon1) / 2.0 / (2.0 * std::f32::consts::PI);
            let u0 = lon0 / (2.0 * std::f32::consts::PI);
            let u1 = lon1 / (2.0 * std::f32::consts::PI);
            let v1 = lat1 / std::f32::consts::PI;

            vertices.push(MeshVertex { position: p0, normal: n0, uv: [u_center, 0.0] });
            vertices.push(MeshVertex { position: p2, normal: n2, uv: [u1, v1] });
            vertices.push(MeshVertex { position: p1, normal: n1, uv: [u0, v1] });
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

                // UV坐标按经纬度映射
                let u0 = lon0 / (2.0 * std::f32::consts::PI);
                let u1 = lon1 / (2.0 * std::f32::consts::PI);
                let v0 = lat0 / std::f32::consts::PI;
                let v1 = lat1 / std::f32::consts::PI;

                // Triangle 1
                vertices.push(MeshVertex { position: p00, normal: n00, uv: [u0, v0] });
                vertices.push(MeshVertex { position: p11, normal: n11, uv: [u1, v1] });
                vertices.push(MeshVertex { position: p01, normal: n01, uv: [u1, v0] });
                // Triangle 2
                vertices.push(MeshVertex { position: p00, normal: n00, uv: [u0, v0] });
                vertices.push(MeshVertex { position: p10, normal: n10, uv: [u0, v1] });
                vertices.push(MeshVertex { position: p11, normal: n11, uv: [u1, v1] });
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

            // UV: pole=[u_center, 1], 其他按经纬度
            let u_center = (lon0 + lon1) / 2.0 / (2.0 * std::f32::consts::PI);
            let u0 = lon0 / (2.0 * std::f32::consts::PI);
            let u1 = lon1 / (2.0 * std::f32::consts::PI);
            let v0 = lat0 / std::f32::consts::PI;

            vertices.push(MeshVertex { position: p0, normal: n0, uv: [u_center, 1.0] });
            vertices.push(MeshVertex { position: p1, normal: n1, uv: [u0, v0] });
            vertices.push(MeshVertex { position: p2, normal: n2, uv: [u1, v0] });
        }
    }

    /// Plane: 1×1 centered at origin, normal = [0, 1, 0]
    fn generate_plane(vertices: &mut Vec<MeshVertex>) {
        let normal = [0.0, 1.0, 0.0];
        // UV: tri1=[0,0],[1,1],[1,0], tri2=[0,0],[0,1],[1,1]
        Self::push_triangle(vertices, [-0.5, 0.0, -0.5], [ 0.5, 0.0,  0.5], [ 0.5, 0.0, -0.5], normal, [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]);
        Self::push_triangle(vertices, [-0.5, 0.0, -0.5], [-0.5, 0.0,  0.5], [ 0.5, 0.0,  0.5], normal, [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
    }

    /// Cylinder: radius 0.5, height 1, centered at origin
    fn generate_cylinder(vertices: &mut Vec<MeshVertex>) {
        let radius = 0.5;
        let half_height = 0.5;
        let segments = 16;

        // Side faces — UV: u=angle/(2PI), v=y映射(half_height→0, -half_height→1)
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

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            // Triangle 1 (top-left, bottom-left, bottom-right)
            Self::push_triangle(vertices,
                [x0, half_height, z0],  // top-left
                [x0, -half_height, z0],  // bottom-left
                [x1, -half_height, z1],  // bottom-right
                [nx0, 0.0, nz0],
                [u0, 0.0], [u0, 1.0], [u1, 1.0],
            );
            // Triangle 2 (top-left, bottom-right, top-right)
            Self::push_triangle(vertices,
                [x0, half_height, z0],  // top-left
                [x1, -half_height, z1],  // bottom-right
                [x1, half_height, z1],  // top-right
                [nx1, 0.0, nz1],
                [u0, 0.0], [u1, 1.0], [u1, 0.0],
            );
        }

        // Top cap (Y+) — UV: 中心=[0.5,0.5], 外圈按角度和径向距离
        let top_normal = [0.0, 1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [0.0, half_height, 0.0],  // center
                [radius * angle0.cos(), half_height, radius * angle0.sin()],
                [radius * angle1.cos(), half_height, radius * angle1.sin()],
                top_normal,
                [0.5, 0.5], [0.5 + 0.5 * u0.cos(), 0.5 + 0.5 * u0.sin()], [0.5 + 0.5 * u1.cos(), 0.5 + 0.5 * u1.sin()],
            );
        }

        // Bottom cap (Y-) — UV: 中心=[0.5,0.5], 外圈按角度和径向距离
        let bottom_normal = [0.0, -1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [0.0, -half_height, 0.0],  // center
                [radius * angle1.cos(), -half_height, radius * angle1.sin()],
                [radius * angle0.cos(), -half_height, radius * angle0.sin()],
                bottom_normal,
                [0.5, 0.5], [0.5 + 0.5 * u1.cos(), 0.5 + 0.5 * u1.sin()], [0.5 + 0.5 * u0.cos(), 0.5 + 0.5 * u0.sin()],
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

        // Side faces — UV: tip的v=0, base的v=1, u按角度
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

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [0.0, tip_y, 0.0],  // tip
                [x0, base_y, z0],
                [x1, base_y, z1],
                n,
                [(u0 + u1) / 2.0, 0.0], [u0, 1.0], [u1, 1.0],
            );
        }

        // Bottom cap (Y-) — UV: 中心=[0.5,0.5], 外圈按角度和径向距离
        let bottom_normal = [0.0, -1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [0.0, base_y, 0.0],  // center
                [base_radius * angle1.cos(), base_y, base_radius * angle1.sin()],
                [base_radius * angle0.cos(), base_y, base_radius * angle0.sin()],
                bottom_normal,
                [0.5, 0.5], [0.5 + 0.5 * u1.cos(), 0.5 + 0.5 * u1.sin()], [0.5 + 0.5 * u0.cos(), 0.5 + 0.5 * u0.sin()],
            );
        }
    }

    /// Bunny: 低多边形兔子模型（程序化生成）
    ///
    /// 由以下部分组成:
    /// - 身体: 1个扁椭球体（宽>高>深）
    /// - 头部: 1个较小的球体
    /// - 2个耳朵: 2个拉长的锥体
    /// - 4条腿: 4个短圆柱体
    ///
    /// 每个部件用sphere/cone/cylinder的简化版本生成顶点，
    /// 应用偏移和缩放变换后合并到一个vertex buffer中。
    /// 总计约200-500三角形，足以验证BVH和ray tracing。
    fn generate_bunny(vertices: &mut Vec<MeshVertex>) {
        // === 身体: 扁椭球体 ===
        // 缩放: x=0.6(宽), y=0.35(矮), z=0.4(深)
        // 偏移: y=0.0（居中）
        Self::generate_bunny_body(vertices);

        // === 头部: 较小球体 ===
        // 缩放: 0.25
        // 偏移: x=0, y=0.45, z=0.35
        Self::generate_bunny_head(vertices);

        // === 左耳朵: 拉长锥体 ===
        // 缩放: radius=0.06, height=0.35
        // 偏移: x=-0.12, y=0.75, z=0.35
        Self::generate_bunny_ear(vertices, -0.12, 0.75, 0.35);

        // === 右耳朵: 拉长锥体 ===
        // 偏移: x=0.12, y=0.75, z=0.35
        Self::generate_bunny_ear(vertices, 0.12, 0.75, 0.35);

        // === 4条腿: 短圆柱体 ===
        // 前左: x=-0.25, y=-0.35, z=0.2
        // 前右: x=0.25, y=-0.35, z=0.2
        // 后左: x=-0.25, y=-0.35, z=-0.2
        // 后右: x=0.25, y=-0.35, z=-0.2
        Self::generate_bunny_leg(vertices, -0.25, -0.35, 0.2);
        Self::generate_bunny_leg(vertices, 0.25, -0.35, 0.2);
        Self::generate_bunny_leg(vertices, -0.25, -0.35, -0.2);
        Self::generate_bunny_leg(vertices, 0.25, -0.35, -0.2);
    }

    /// Bunny身体: 扁椭球体
    ///
    /// 用低段数(8经×6纬)的UV sphere生成，然后应用缩放变换。
    /// 缩放: sx=0.6, sy=0.35, sz=0.4
    /// 偏移: ox=0, oy=0, oz=0
    fn generate_bunny_body(vertices: &mut Vec<MeshVertex>) {
        let sx = 0.6;  // 宽度缩放
        let sy = 0.35; // 高度缩放（扁）
        let sz = 0.4;  // 深度缩放
        let ox = 0.0;
        let oy = 0.0;
        let oz = 0.0;
        let lon_segments = 8;
        let lat_bands = 6;

        // 顶极点
        let top_y = 0.5 * sy + oy;
        for i in 0..lon_segments {
            let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
            let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;
            let lat1 = std::f32::consts::PI * 1.0 / lat_bands as f32;
            let y1 = 0.5 * lat1.cos() * sy + oy;
            let r1 = 0.5 * lat1.sin();

            let p0 = [ox, top_y, oz];
            let p1 = [r1 * lon0.cos() * sx + ox, y1, r1 * lon0.sin() * sz + oz];
            let p2 = [r1 * lon1.cos() * sx + ox, y1, r1 * lon1.sin() * sz + oz];

            // 椭球体法线: normalize(position / scale)
            let n0 = Self::normalize3([(p0[0] - ox) / sx, (p0[1] - oy) / sy, (p0[2] - oz) / sz]);
            let n1 = Self::normalize3([(p1[0] - ox) / sx, (p1[1] - oy) / sy, (p1[2] - oz) / sz]);
            let n2 = Self::normalize3([(p2[0] - ox) / sx, (p2[1] - oy) / sy, (p2[2] - oz) / sz]);

            let u_center = (lon0 + lon1) / 2.0 / (2.0 * std::f32::consts::PI);
            let u0 = lon0 / (2.0 * std::f32::consts::PI);
            let u1 = lon1 / (2.0 * std::f32::consts::PI);
            let v1 = lat1 / std::f32::consts::PI;

            vertices.push(MeshVertex { position: p0, normal: n0, uv: [u_center, 0.0] });
            vertices.push(MeshVertex { position: p2, normal: n2, uv: [u1, v1] });
            vertices.push(MeshVertex { position: p1, normal: n1, uv: [u0, v1] });
        }

        // 中间纬度带
        for j in 1..(lat_bands - 1) {
            let lat0 = std::f32::consts::PI * j as f32 / lat_bands as f32;
            let lat1 = std::f32::consts::PI * (j + 1) as f32 / lat_bands as f32;
            let y0 = 0.5 * lat0.cos() * sy + oy;
            let r0 = 0.5 * lat0.sin();
            let y1 = 0.5 * lat1.cos() * sy + oy;
            let r1 = 0.5 * lat1.sin();

            for i in 0..lon_segments {
                let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
                let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;

                let p00 = [r0 * lon0.cos() * sx + ox, y0, r0 * lon0.sin() * sz + oz];
                let p01 = [r0 * lon1.cos() * sx + ox, y0, r0 * lon1.sin() * sz + oz];
                let p10 = [r1 * lon0.cos() * sx + ox, y1, r1 * lon0.sin() * sz + oz];
                let p11 = [r1 * lon1.cos() * sx + ox, y1, r1 * lon1.sin() * sz + oz];

                let n00 = Self::normalize3([(p00[0] - ox) / sx, (p00[1] - oy) / sy, (p00[2] - oz) / sz]);
                let n01 = Self::normalize3([(p01[0] - ox) / sx, (p01[1] - oy) / sy, (p01[2] - oz) / sz]);
                let n10 = Self::normalize3([(p10[0] - ox) / sx, (p10[1] - oy) / sy, (p10[2] - oz) / sz]);
                let n11 = Self::normalize3([(p11[0] - ox) / sx, (p11[1] - oy) / sy, (p11[2] - oz) / sz]);

                let u0 = lon0 / (2.0 * std::f32::consts::PI);
                let u1 = lon1 / (2.0 * std::f32::consts::PI);
                let v0 = lat0 / std::f32::consts::PI;
                let v1 = lat1 / std::f32::consts::PI;

                // Triangle 1
                vertices.push(MeshVertex { position: p00, normal: n00, uv: [u0, v0] });
                vertices.push(MeshVertex { position: p11, normal: n11, uv: [u1, v1] });
                vertices.push(MeshVertex { position: p01, normal: n01, uv: [u1, v0] });
                // Triangle 2
                vertices.push(MeshVertex { position: p00, normal: n00, uv: [u0, v0] });
                vertices.push(MeshVertex { position: p10, normal: n10, uv: [u0, v1] });
                vertices.push(MeshVertex { position: p11, normal: n11, uv: [u1, v1] });
            }
        }

        // 底极点
        let bottom_y = -0.5 * sy + oy;
        for i in 0..lon_segments {
            let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
            let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;
            let lat0 = std::f32::consts::PI * (lat_bands - 1) as f32 / lat_bands as f32;
            let y0 = 0.5 * lat0.cos() * sy + oy;
            let r0 = 0.5 * lat0.sin();

            let p0 = [ox, bottom_y, oz];
            let p1 = [r0 * lon0.cos() * sx + ox, y0, r0 * lon0.sin() * sz + oz];
            let p2 = [r0 * lon1.cos() * sx + ox, y0, r0 * lon1.sin() * sz + oz];

            let n0 = Self::normalize3([(p0[0] - ox) / sx, (p0[1] - oy) / sy, (p0[2] - oz) / sz]);
            let n1 = Self::normalize3([(p1[0] - ox) / sx, (p1[1] - oy) / sy, (p1[2] - oz) / sz]);
            let n2 = Self::normalize3([(p2[0] - ox) / sx, (p2[1] - oy) / sy, (p2[2] - oz) / sz]);

            let u_center = (lon0 + lon1) / 2.0 / (2.0 * std::f32::consts::PI);
            let u0 = lon0 / (2.0 * std::f32::consts::PI);
            let u1 = lon1 / (2.0 * std::f32::consts::PI);
            let v0 = lat0 / std::f32::consts::PI;

            vertices.push(MeshVertex { position: p0, normal: n0, uv: [u_center, 1.0] });
            vertices.push(MeshVertex { position: p1, normal: n1, uv: [u0, v0] });
            vertices.push(MeshVertex { position: p2, normal: n2, uv: [u1, v0] });
        }
    }

    /// Bunny头部: 较小球体
    ///
    /// 用低段数(8经×6纬)的UV sphere生成，然后应用缩放变换。
    /// 缩放: 0.25（均匀缩放）
    /// 偏移: ox=0, oy=0.45, oz=0.35
    fn generate_bunny_head(vertices: &mut Vec<MeshVertex>) {
        let scale = 0.25;
        let ox = 0.0;
        let oy = 0.45;
        let oz = 0.35;
        let lon_segments = 8;
        let lat_bands = 6;

        // 顶极点
        let top_y = 0.5 * scale + oy;
        for i in 0..lon_segments {
            let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
            let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;
            let lat1 = std::f32::consts::PI * 1.0 / lat_bands as f32;
            let y1 = 0.5 * lat1.cos() * scale + oy;
            let r1 = 0.5 * lat1.sin() * scale;

            let p0 = [ox, top_y, oz];
            let p1 = [r1 * lon0.cos() + ox, y1, r1 * lon0.sin() + oz];
            let p2 = [r1 * lon1.cos() + ox, y1, r1 * lon1.sin() + oz];

            let n0 = Self::normalize3([p0[0] - ox, p0[1] - oy, p0[2] - oz]);
            let n1 = Self::normalize3([p1[0] - ox, p1[1] - oy, p1[2] - oz]);
            let n2 = Self::normalize3([p2[0] - ox, p2[1] - oy, p2[2] - oz]);

            let u_center = (lon0 + lon1) / 2.0 / (2.0 * std::f32::consts::PI);
            let u0 = lon0 / (2.0 * std::f32::consts::PI);
            let u1 = lon1 / (2.0 * std::f32::consts::PI);
            let v1 = lat1 / std::f32::consts::PI;

            vertices.push(MeshVertex { position: p0, normal: n0, uv: [u_center, 0.0] });
            vertices.push(MeshVertex { position: p2, normal: n2, uv: [u1, v1] });
            vertices.push(MeshVertex { position: p1, normal: n1, uv: [u0, v1] });
        }

        // 中间纬度带
        for j in 1..(lat_bands - 1) {
            let lat0 = std::f32::consts::PI * j as f32 / lat_bands as f32;
            let lat1 = std::f32::consts::PI * (j + 1) as f32 / lat_bands as f32;
            let y0 = 0.5 * lat0.cos() * scale + oy;
            let r0 = 0.5 * lat0.sin() * scale;
            let y1 = 0.5 * lat1.cos() * scale + oy;
            let r1 = 0.5 * lat1.sin() * scale;

            for i in 0..lon_segments {
                let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
                let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;

                let p00 = [r0 * lon0.cos() + ox, y0, r0 * lon0.sin() + oz];
                let p01 = [r0 * lon1.cos() + ox, y0, r0 * lon1.sin() + oz];
                let p10 = [r1 * lon0.cos() + ox, y1, r1 * lon0.sin() + oz];
                let p11 = [r1 * lon1.cos() + ox, y1, r1 * lon1.sin() + oz];

                let n00 = Self::normalize3([p00[0] - ox, p00[1] - oy, p00[2] - oz]);
                let n01 = Self::normalize3([p01[0] - ox, p01[1] - oy, p01[2] - oz]);
                let n10 = Self::normalize3([p10[0] - ox, p10[1] - oy, p10[2] - oz]);
                let n11 = Self::normalize3([p11[0] - ox, p11[1] - oy, p11[2] - oz]);

                let u0 = lon0 / (2.0 * std::f32::consts::PI);
                let u1 = lon1 / (2.0 * std::f32::consts::PI);
                let v0 = lat0 / std::f32::consts::PI;
                let v1 = lat1 / std::f32::consts::PI;

                vertices.push(MeshVertex { position: p00, normal: n00, uv: [u0, v0] });
                vertices.push(MeshVertex { position: p11, normal: n11, uv: [u1, v1] });
                vertices.push(MeshVertex { position: p01, normal: n01, uv: [u1, v0] });
                vertices.push(MeshVertex { position: p00, normal: n00, uv: [u0, v0] });
                vertices.push(MeshVertex { position: p10, normal: n10, uv: [u0, v1] });
                vertices.push(MeshVertex { position: p11, normal: n11, uv: [u1, v1] });
            }
        }

        // 底极点
        let bottom_y = -0.5 * scale + oy;
        for i in 0..lon_segments {
            let lon0 = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
            let lon1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / lon_segments as f32;
            let lat0 = std::f32::consts::PI * (lat_bands - 1) as f32 / lat_bands as f32;
            let y0 = 0.5 * lat0.cos() * scale + oy;
            let r0 = 0.5 * lat0.sin() * scale;

            let p0 = [ox, bottom_y, oz];
            let p1 = [r0 * lon0.cos() + ox, y0, r0 * lon0.sin() + oz];
            let p2 = [r0 * lon1.cos() + ox, y0, r0 * lon1.sin() + oz];

            let n0 = Self::normalize3([p0[0] - ox, p0[1] - oy, p0[2] - oz]);
            let n1 = Self::normalize3([p1[0] - ox, p1[1] - oy, p1[2] - oz]);
            let n2 = Self::normalize3([p2[0] - ox, p2[1] - oy, p2[2] - oz]);

            let u_center = (lon0 + lon1) / 2.0 / (2.0 * std::f32::consts::PI);
            let u0 = lon0 / (2.0 * std::f32::consts::PI);
            let u1 = lon1 / (2.0 * std::f32::consts::PI);
            let v0 = lat0 / std::f32::consts::PI;

            vertices.push(MeshVertex { position: p0, normal: n0, uv: [u_center, 1.0] });
            vertices.push(MeshVertex { position: p1, normal: n1, uv: [u0, v0] });
            vertices.push(MeshVertex { position: p2, normal: n2, uv: [u1, v0] });
        }
    }

    /// Bunny耳朵: 拉长锥体
    ///
    /// 用8段锥体生成，缩放radius=0.06, height=0.35
    /// 偏移由参数指定
    fn generate_bunny_ear(vertices: &mut Vec<MeshVertex>, ox: f32, oy: f32, oz: f32) {
        let base_radius = 0.06;
        let half_height = 0.175;  // height=0.35, half=0.175
        let segments = 8;
        let tip_y = half_height + oy;
        let base_y = -half_height + oy;

        // 侧面
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let x0 = base_radius * angle0.cos() + ox;
            let z0 = base_radius * angle0.sin() + oz;
            let x1 = base_radius * angle1.cos() + ox;
            let z1 = base_radius * angle1.sin() + oz;

            let slant = [ox - x0, tip_y - base_y, oz - z0];
            let tangent = [x1 - x0, 0.0, z1 - z0];
            let n = Self::normalize3(Self::cross3(slant, tangent));

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [ox, tip_y, oz],  // tip
                [x0, base_y, z0],
                [x1, base_y, z1],
                n,
                [(u0 + u1) / 2.0, 0.0], [u0, 1.0], [u1, 1.0],
            );
        }

        // 底面
        let bottom_normal = [0.0, -1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [ox, base_y, oz],  // center
                [base_radius * angle1.cos() + ox, base_y, base_radius * angle1.sin() + oz],
                [base_radius * angle0.cos() + ox, base_y, base_radius * angle0.sin() + oz],
                bottom_normal,
                [0.5, 0.5], [0.5 + 0.5 * u1.cos(), 0.5 + 0.5 * u1.sin()], [0.5 + 0.5 * u0.cos(), 0.5 + 0.5 * u0.sin()],
            );
        }
    }

    /// Bunny腿: 短圆柱体
    ///
    /// 用8段圆柱体生成，缩放radius=0.08, height=0.3
    /// 偏移由参数指定
    fn generate_bunny_leg(vertices: &mut Vec<MeshVertex>, ox: f32, oy: f32, oz: f32) {
        let radius = 0.08;
        let half_height = 0.15;  // height=0.3, half=0.15
        let segments = 8;

        // 侧面
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let x0 = radius * angle0.cos() + ox;
            let z0 = radius * angle0.sin() + oz;
            let x1 = radius * angle1.cos() + ox;
            let z1 = radius * angle1.sin() + oz;

            let nx0 = angle0.cos();
            let nz0 = angle0.sin();
            let nx1 = angle1.cos();
            let nz1 = angle1.sin();

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            // Triangle 1
            Self::push_triangle(vertices,
                [x0, half_height + oy, z0],
                [x0, -half_height + oy, z0],
                [x1, -half_height + oy, z1],
                [nx0, 0.0, nz0],
                [u0, 0.0], [u0, 1.0], [u1, 1.0],
            );
            // Triangle 2
            Self::push_triangle(vertices,
                [x0, half_height + oy, z0],
                [x1, -half_height + oy, z1],
                [x1, half_height + oy, z1],
                [nx1, 0.0, nz1],
                [u0, 0.0], [u1, 1.0], [u1, 0.0],
            );
        }

        // 顶面
        let top_normal = [0.0, 1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [ox, half_height + oy, oz],
                [radius * angle0.cos() + ox, half_height + oy, radius * angle0.sin() + oz],
                [radius * angle1.cos() + ox, half_height + oy, radius * angle1.sin() + oz],
                top_normal,
                [0.5, 0.5], [0.5 + 0.5 * u0.cos(), 0.5 + 0.5 * u0.sin()], [0.5 + 0.5 * u1.cos(), 0.5 + 0.5 * u1.sin()],
            );
        }

        // 底面
        let bottom_normal = [0.0, -1.0, 0.0];
        for i in 0..segments {
            let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let angle1 = 2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;

            let u0 = angle0 / (2.0 * std::f32::consts::PI);
            let u1 = angle1 / (2.0 * std::f32::consts::PI);

            Self::push_triangle(vertices,
                [ox, -half_height + oy, oz],
                [radius * angle1.cos() + ox, -half_height + oy, radius * angle1.sin() + oz],
                [radius * angle0.cos() + ox, -half_height + oy, radius * angle0.sin() + oz],
                bottom_normal,
                [0.5, 0.5], [0.5 + 0.5 * u1.cos(), 0.5 + 0.5 * u1.sin()], [0.5 + 0.5 * u0.cos(), 0.5 + 0.5 * u0.sin()],
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
/// "builtin://xxx" → 对应primitive类型
/// "asset://xxx" → Custom（由mesh_loader处理）
pub fn mesh_type_from_path(path: &str) -> MeshType {
    match path {
        "builtin://cube" => MeshType::Cube,
        "builtin://sphere" => MeshType::Sphere,
        "builtin://plane" => MeshType::Plane,
        "builtin://cylinder" => MeshType::Cylinder,
        "builtin://cone" => MeshType::Cone,
        "builtin://bunny" => MeshType::Bunny,
        _ => MeshType::Custom,  // asset:// 或其他自定义路径 → Custom
    }
}