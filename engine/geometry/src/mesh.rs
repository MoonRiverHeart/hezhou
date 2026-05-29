use crate::{BoundingBox, Vertex};
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

                if i0 < self.vertices.len() && i1 < self.vertices.len() && i2 < self.vertices.len()
                {
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

                if i0 < self.vertices.len() && i1 < self.vertices.len() && i2 < self.vertices.len()
                {
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
            Vertex::new([0.0, 0.5, 0.0])
                .with_uv([0.5, 1.0])
                .with_color([1.0, 0.0, 0.0, 1.0]),
            Vertex::new([-0.5, -0.5, 0.0])
                .with_uv([0.0, 0.0])
                .with_color([0.0, 1.0, 0.0, 1.0]),
            Vertex::new([0.5, -0.5, 0.0])
                .with_uv([1.0, 0.0])
                .with_color([0.0, 0.0, 1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2];
        let mut mesh = Self::new(vertices, indices);
        mesh.calculate_normals();
        mesh
    }

    pub fn create_quad() -> Self {
        let vertices = vec![
            Vertex::new([-0.5, -0.5, 0.0])
                .with_uv([0.0, 0.0])
                .with_color([1.0, 1.0, 1.0, 1.0]),
            Vertex::new([0.5, -0.5, 0.0])
                .with_uv([1.0, 0.0])
                .with_color([1.0, 1.0, 1.0, 1.0]),
            Vertex::new([0.5, 0.5, 0.0])
                .with_uv([1.0, 1.0])
                .with_color([1.0, 1.0, 1.0, 1.0]),
            Vertex::new([-0.5, 0.5, 0.0])
                .with_uv([0.0, 1.0])
                .with_color([1.0, 1.0, 1.0, 1.0]),
        ];
        let indices = vec![0, 1, 2, 0, 2, 3];
        let mut mesh = Self::new(vertices, indices);
        mesh.calculate_normals();
        mesh
    }

    pub fn create_cube() -> Self {
        let vertices = vec![
            Vertex::new([-0.5, -0.5, -0.5])
                .with_uv([0.0, 0.0])
                .with_normal([0.0, 0.0, -1.0]),
            Vertex::new([0.5, -0.5, -0.5])
                .with_uv([1.0, 0.0])
                .with_normal([0.0, 0.0, -1.0]),
            Vertex::new([0.5, 0.5, -0.5])
                .with_uv([1.0, 1.0])
                .with_normal([0.0, 0.0, -1.0]),
            Vertex::new([-0.5, 0.5, -0.5])
                .with_uv([0.0, 1.0])
                .with_normal([0.0, 0.0, -1.0]),
            Vertex::new([-0.5, -0.5, 0.5])
                .with_uv([0.0, 0.0])
                .with_normal([0.0, 0.0, 1.0]),
            Vertex::new([0.5, -0.5, 0.5])
                .with_uv([1.0, 0.0])
                .with_normal([0.0, 0.0, 1.0]),
            Vertex::new([0.5, 0.5, 0.5])
                .with_uv([1.0, 1.0])
                .with_normal([0.0, 0.0, 1.0]),
            Vertex::new([-0.5, 0.5, 0.5])
                .with_uv([0.0, 1.0])
                .with_normal([0.0, 0.0, 1.0]),
            Vertex::new([-0.5, 0.5, -0.5])
                .with_uv([0.0, 0.0])
                .with_normal([0.0, 1.0, 0.0]),
            Vertex::new([0.5, 0.5, -0.5])
                .with_uv([1.0, 0.0])
                .with_normal([0.0, 1.0, 0.0]),
            Vertex::new([0.5, 0.5, 0.5])
                .with_uv([1.0, 1.0])
                .with_normal([0.0, 1.0, 0.0]),
            Vertex::new([-0.5, 0.5, 0.5])
                .with_uv([0.0, 1.0])
                .with_normal([0.0, 1.0, 0.0]),
            Vertex::new([-0.5, -0.5, -0.5])
                .with_uv([0.0, 0.0])
                .with_normal([0.0, -1.0, 0.0]),
            Vertex::new([0.5, -0.5, -0.5])
                .with_uv([1.0, 0.0])
                .with_normal([0.0, -1.0, 0.0]),
            Vertex::new([0.5, -0.5, 0.5])
                .with_uv([1.0, 1.0])
                .with_normal([0.0, -1.0, 0.0]),
            Vertex::new([-0.5, -0.5, 0.5])
                .with_uv([0.0, 1.0])
                .with_normal([0.0, -1.0, 0.0]),
            Vertex::new([0.5, -0.5, -0.5])
                .with_uv([0.0, 0.0])
                .with_normal([1.0, 0.0, 0.0]),
            Vertex::new([0.5, 0.5, -0.5])
                .with_uv([1.0, 0.0])
                .with_normal([1.0, 0.0, 0.0]),
            Vertex::new([0.5, 0.5, 0.5])
                .with_uv([1.0, 1.0])
                .with_normal([1.0, 0.0, 0.0]),
            Vertex::new([0.5, -0.5, 0.5])
                .with_uv([0.0, 1.0])
                .with_normal([1.0, 0.0, 0.0]),
            Vertex::new([-0.5, -0.5, -0.5])
                .with_uv([0.0, 0.0])
                .with_normal([-1.0, 0.0, 0.0]),
            Vertex::new([-0.5, 0.5, -0.5])
                .with_uv([1.0, 0.0])
                .with_normal([-1.0, 0.0, 0.0]),
            Vertex::new([-0.5, 0.5, 0.5])
                .with_uv([1.0, 1.0])
                .with_normal([-1.0, 0.0, 0.0]),
            Vertex::new([-0.5, -0.5, 0.5])
                .with_uv([0.0, 1.0])
                .with_normal([-1.0, 0.0, 0.0]),
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3,
            4, 5, 6, 4, 6, 7,
            8, 9, 10, 8, 10, 11,
            12, 13, 14, 12, 14, 15,
            16, 17, 18, 16, 18, 19,
            20, 21, 22, 20, 22, 23,
        ];
        Self::new(vertices, indices)
    }

    /// 创建低多边形bunny模型（程序化生成）
    ///
    /// 由以下部分组成:
    /// - 身体: 扁椭球体（8经×6纬）
    /// - 头部: 较小球体（8经×6纬）
    /// - 2个耳朵: 拉长锥体（8段）
    /// - 4条腿: 短圆柱体（8段）
    ///
    /// 总计约200-500三角形，足以验证BVH和ray tracing。
    /// 使用indices buffer以便BVH构建。
    pub fn create_bunny() -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // === 身体: 扁椭球体 ===
        // 缩放: sx=0.6, sy=0.35, sz=0.4
        // 偏移: ox=0, oy=0, oz=0
        Self::bunny_ellipsoid(&mut vertices, &mut indices, 0.6, 0.35, 0.4, 0.0, 0.0, 0.0, 8, 6);

        // === 头部: 较小球体 ===
        // 缩放: 0.25（均匀）
        // 偏移: ox=0, oy=0.45, oz=0.35
        Self::bunny_ellipsoid(&mut vertices, &mut indices, 0.25, 0.25, 0.25, 0.0, 0.45, 0.35, 8, 6);

        // === 左耳朵: 拉长锥体 ===
        Self::bunny_cone(&mut vertices, &mut indices, 0.06, 0.35, -0.12, 0.75, 0.35, 8);

        // === 右耳朵: 拉长锥体 ===
        Self::bunny_cone(&mut vertices, &mut indices, 0.06, 0.35, 0.12, 0.75, 0.35, 8);

        // === 4条腿: 短圆柱体 ===
        Self::bunny_cylinder(&mut vertices, &mut indices, 0.08, 0.3, -0.25, -0.35, 0.2, 8);
        Self::bunny_cylinder(&mut vertices, &mut indices, 0.08, 0.3, 0.25, -0.35, 0.2, 8);
        Self::bunny_cylinder(&mut vertices, &mut indices, 0.08, 0.3, -0.25, -0.35, -0.2, 8);
        Self::bunny_cylinder(&mut vertices, &mut indices, 0.08, 0.3, 0.25, -0.35, -0.2, 8);

        let mut mesh = Self::new(vertices, indices);
        mesh.calculate_normals();
        mesh
    }

    /// Bunny椭球体部件生成（用于身体和头部）
    fn bunny_ellipsoid(
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        sx: f32, sy: f32, sz: f32,
        ox: f32, oy: f32, oz: f32,
        lon_segments: u32, lat_bands: u32,
    ) {
        let base_idx = vertices.len() as u32;

        // 顶极点
        let top_y = 0.5 * sy + oy;
        vertices.push(Vertex::new([ox, top_y, oz]).with_uv([0.5, 0.0]).with_normal(Self::ellipsoid_normal(ox, top_y, oz, sx, sy, sz, ox, oy, oz)));

        // 中间纬度带顶点
        for j in 1..lat_bands {
            let lat = std::f32::consts::PI * j as f32 / lat_bands as f32;
            let y = 0.5 * lat.cos() * sy + oy;
            let r = 0.5 * lat.sin();
            for i in 0..lon_segments {
                let lon = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
                let px = r * lon.cos() * sx + ox;
                let pz = r * lon.sin() * sz + oz;
                let u = lon / (2.0 * std::f32::consts::PI);
                let v = lat / std::f32::consts::PI;
                vertices.push(Vertex::new([px, y, pz]).with_uv([u, v]).with_normal(Self::ellipsoid_normal(px, y, pz, sx, sy, sz, ox, oy, oz)));
            }
        }

        // 底极点
        let bottom_y = -0.5 * sy + oy;
        vertices.push(Vertex::new([ox, bottom_y, oz]).with_uv([0.5, 1.0]).with_normal(Self::ellipsoid_normal(ox, bottom_y, oz, sx, sy, sz, ox, oy, oz)));

        // 顶极点三角形
        let top_idx = base_idx;
        for i in 0..lon_segments {
            let i0 = base_idx + 1 + i;
            let i1 = base_idx + 1 + (i + 1) % lon_segments;
            indices.push(top_idx);
            indices.push(i1);
            indices.push(i0);
        }

        // 中间纬度带三角形
        for j in 1..(lat_bands - 1) {
            let row0_start = base_idx + 1 + (j - 1) * lon_segments;
            let row1_start = base_idx + 1 + j * lon_segments;
            for i in 0..lon_segments {
                let i0 = row0_start + i;
                let i1 = row0_start + (i + 1) % lon_segments;
                let i2 = row1_start + i;
                let i3 = row1_start + (i + 1) % lon_segments;
                indices.push(i0);
                indices.push(i3);
                indices.push(i1);
                indices.push(i0);
                indices.push(i2);
                indices.push(i3);
            }
        }

        // 底极点三角形
        let bottom_idx = vertices.len() as u32 - 1;
        let last_row_start = base_idx + 1 + (lat_bands - 2) * lon_segments;
        for i in 0..lon_segments {
            let i0 = last_row_start + i;
            let i1 = last_row_start + (i + 1) % lon_segments;
            indices.push(bottom_idx);
            indices.push(i0);
            indices.push(i1);
        }
    }

    /// 计算椭球体法线: normalize((position - center) / scale)
    fn ellipsoid_normal(px: f32, py: f32, pz: f32, sx: f32, sy: f32, sz: f32, ox: f32, oy: f32, oz: f32) -> [f32; 3] {
        let nx = (px - ox) / sx;
        let ny = (py - oy) / sy;
        let nz = (pz - oz) / sz;
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if len > 0.0 {
            [nx / len, ny / len, nz / len]
        } else {
            [0.0, 1.0, 0.0]
        }
    }

    /// Bunny锥体部件生成（用于耳朵）
    fn bunny_cone(
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        base_radius: f32, height: f32,
        ox: f32, oy: f32, oz: f32,
        segments: u32,
    ) {
        let base_idx = vertices.len() as u32;
        let half_height = height / 2.0;
        let tip_y = half_height + oy;
        let base_y = -half_height + oy;

        // tip顶点
        vertices.push(Vertex::new([ox, tip_y, oz]).with_uv([0.5, 0.0]).with_normal([0.0, 1.0, 0.0]));

        // base圆环顶点
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let px = base_radius * angle.cos() + ox;
            let pz = base_radius * angle.sin() + oz;
            let u = angle / (2.0 * std::f32::consts::PI);
            vertices.push(Vertex::new([px, base_y, pz]).with_uv([u, 1.0]).with_normal([angle.cos(), 0.0, angle.sin()]));
        }

        // base中心顶点
        vertices.push(Vertex::new([ox, base_y, oz]).with_uv([0.5, 0.5]).with_normal([0.0, -1.0, 0.0]));

        let tip_idx = base_idx;
        let base_center_idx = vertices.len() as u32 - 1;

        // 侧面三角形
        for i in 0..segments {
            let i0 = base_idx + 1 + i;
            let i1 = base_idx + 1 + (i + 1) % segments;
            indices.push(tip_idx);
            indices.push(i0);
            indices.push(i1);
        }

        // 底面三角形
        for i in 0..segments {
            let i0 = base_idx + 1 + i;
            let i1 = base_idx + 1 + (i + 1) % segments;
            indices.push(base_center_idx);
            indices.push(i1);
            indices.push(i0);
        }
    }

    /// Bunny圆柱体部件生成（用于腿）
    fn bunny_cylinder(
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        radius: f32, height: f32,
        ox: f32, oy: f32, oz: f32,
        segments: u32,
    ) {
        let base_idx = vertices.len() as u32;
        let half_height = height / 2.0;

        // 顶圆环顶点
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let px = radius * angle.cos() + ox;
            let pz = radius * angle.sin() + oz;
            let u = angle / (2.0 * std::f32::consts::PI);
            vertices.push(Vertex::new([px, half_height + oy, pz]).with_uv([u, 0.0]).with_normal([angle.cos(), 0.0, angle.sin()]));
        }

        // 底圆环顶点
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let px = radius * angle.cos() + ox;
            let pz = radius * angle.sin() + oz;
            let u = angle / (2.0 * std::f32::consts::PI);
            vertices.push(Vertex::new([px, -half_height + oy, pz]).with_uv([u, 1.0]).with_normal([angle.cos(), 0.0, angle.sin()]));
        }

        // 顶面中心
        vertices.push(Vertex::new([ox, half_height + oy, oz]).with_uv([0.5, 0.5]).with_normal([0.0, 1.0, 0.0]));
        // 底面中心
        vertices.push(Vertex::new([ox, -half_height + oy, oz]).with_uv([0.5, 0.5]).with_normal([0.0, -1.0, 0.0]));

        let top_center_idx = vertices.len() as u32 - 2;
        let bottom_center_idx = vertices.len() as u32 - 1;

        // 侧面三角形
        for i in 0..segments {
            let top_i0 = base_idx + i;
            let top_i1 = base_idx + (i + 1) % segments;
            let bottom_i0 = base_idx + segments + i;
            let bottom_i1 = base_idx + segments + (i + 1) % segments;
            // Triangle 1
            indices.push(top_i0);
            indices.push(bottom_i0);
            indices.push(bottom_i1);
            // Triangle 2
            indices.push(top_i0);
            indices.push(bottom_i1);
            indices.push(top_i1);
        }

        // 顶面三角形
        for i in 0..segments {
            let top_i0 = base_idx + i;
            let top_i1 = base_idx + (i + 1) % segments;
            indices.push(top_center_idx);
            indices.push(top_i0);
            indices.push(top_i1);
        }

        // 底面三角形
        for i in 0..segments {
            let bottom_i0 = base_idx + segments + i;
            let bottom_i1 = base_idx + segments + (i + 1) % segments;
            indices.push(bottom_center_idx);
            indices.push(bottom_i1);
            indices.push(bottom_i0);
        }
    }

    /// 创建经典Cornell Box渲染测试场景
    ///
    /// 包含5面墙(地板/天花板/后墙/左墙红/右墙绿) + 2个旋转盒子 + 天花板area light面板。
    /// 所有面使用per-vertex颜色和手动指定的面法线，winding order为逆时针从外侧看。
    /// 总计72个顶点, 102个索引(34个三角形)。
    pub fn create_cornell_box() -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // 房间尺寸: x[0, 5.5], y[0, 5.6], z[0, 5.5] (前墙开放，用于观察)
        let rx = 5.5;
        let ry = 5.6;
        let rz = 5.5;

        // 颜色定义
        let white = [0.73, 0.73, 0.73, 1.0];
        let red = [0.65, 0.05, 0.05, 1.0];
        let green = [0.12, 0.45, 0.15, 1.0];
        let light_color = [1.0, 1.0, 1.0, 2.0]; // alpha>1.5标记为emissive面光源

        // === 5面墙 ===

        // 地板 (y=0): 白色, 法线朝上 [0, 1, 0]
        Self::cornell_quad(&mut vertices, &mut indices,
            [0.0, 0.0, 0.0], [0.0, 0.0, rz], [rx, 0.0, rz], [rx, 0.0, 0.0],
            [0.0, 1.0, 0.0], white);

        // 天花板 (y=ry): 白色, 法线朝下 [0, -1, 0]
        Self::cornell_quad(&mut vertices, &mut indices,
            [0.0, ry, 0.0], [rx, ry, 0.0], [rx, ry, rz], [0.0, ry, rz],
            [0.0, -1.0, 0.0], white);

        // 后墙 (z=rz): 白色, 法线朝前(朝观察者) [0, 0, -1]
        Self::cornell_quad(&mut vertices, &mut indices,
            [0.0, 0.0, rz], [0.0, ry, rz], [rx, ry, rz], [rx, 0.0, rz],
            [0.0, 0.0, -1.0], white);

        // 左墙 (x=0): 红色, 法线朝右(朝房间内) [1, 0, 0]
        Self::cornell_quad(&mut vertices, &mut indices,
            [0.0, 0.0, rz], [0.0, 0.0, 0.0], [0.0, ry, 0.0], [0.0, ry, rz],
            [1.0, 0.0, 0.0], red);

        // 右墙 (x=rx): 绿色, 法线朝左(朝房间内) [-1, 0, 0]
        Self::cornell_quad(&mut vertices, &mut indices,
            [rx, 0.0, 0.0], [rx, 0.0, rz], [rx, ry, rz], [rx, ry, 0.0],
            [-1.0, 0.0, 0.0], green);

        // === 2个盒子 ===

        // 高盒: 中心(1.85, 1.65, 1.65), 半宽(0.5, 1.65, 0.5), Y轴旋转18°
        Self::cornell_rotated_box(&mut vertices, &mut indices,
            1.85, 1.65, 1.65, 0.5, 1.65, 0.5, 18.0, white);

        // 矮盒: 中心(3.6, 0.825, 3.3), 半宽(0.85, 0.825, 0.85), Y轴旋转-16°
        Self::cornell_rotated_box(&mut vertices, &mut indices,
            3.6, 0.825, 3.3, 0.85, 0.825, 0.85, -16.0, white);

        // === Area light面板 ===
        // 天花板下方发光区域: 中心(2.75, 5.58, 2.75), 尺寸1.3×1.3, 法线朝下
        let light_y = 5.58;
        let light_cx = 2.75;
        let light_cz = 2.75;
        let light_half = 0.65; // 1.3 / 2
        Self::cornell_quad(&mut vertices, &mut indices,
            [light_cx - light_half, light_y, light_cz - light_half],
            [light_cx + light_half, light_y, light_cz - light_half],
            [light_cx + light_half, light_y, light_cz + light_half],
            [light_cx - light_half, light_y, light_cz + light_half],
            [0.0, -1.0, 0.0], light_color);

        Self::new(vertices, indices)
    }

    /// 创建UV Sphere（均匀球体）
    ///
    /// 参数：半径0.5，16经线(lon_segments)，12纬线(lat_bands)
    /// 顶极点+中间纬度带+底极点，法线=normalize(position)
    /// 总计约192个顶点, 约288个三角形。
    pub fn create_sphere() -> Self {
        let radius = 0.5;
        let lon_segments = 16;
        let lat_bands = 12;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let white = [0.73, 0.73, 0.73, 1.0];

        // 顶极点
        vertices.push(Vertex::new([0.0, radius, 0.0]).with_uv([0.5, 0.0]).with_normal([0.0, 1.0, 0.0]).with_color(white));

        // 中间纬度带顶点
        for j in 1..lat_bands {
            let lat = std::f32::consts::PI * j as f32 / lat_bands as f32;
            let y = radius * lat.cos();
            let r = radius * lat.sin();
            for i in 0..lon_segments {
                let lon = 2.0 * std::f32::consts::PI * i as f32 / lon_segments as f32;
                let px = r * lon.cos();
                let pz = r * lon.sin();
                let u = lon / (2.0 * std::f32::consts::PI);
                let v = lat / std::f32::consts::PI;
                // 法线 = normalize(position)（球体中心在原点）
                let len = (px * px + y * y + pz * pz).sqrt();
                let nx = px / len;
                let ny = y / len;
                let nz = pz / len;
                vertices.push(Vertex::new([px, y, pz]).with_uv([u, v]).with_normal([nx, ny, nz]).with_color(white));
            }
        }

        // 底极点
        vertices.push(Vertex::new([0.0, -radius, 0.0]).with_uv([0.5, 1.0]).with_normal([0.0, -1.0, 0.0]).with_color(white));

        let base_idx = 0u32;

        // 顶极点fan三角形
        let top_idx = base_idx;
        for i in 0..lon_segments {
            let i0 = base_idx + 1 + i;
            let i1 = base_idx + 1 + (i + 1) % lon_segments;
            indices.push(top_idx);
            indices.push(i1);
            indices.push(i0);
        }

        // 中间纬度带quads（每个quad拆为2个三角形）
        for j in 1..(lat_bands - 1) {
            let row0_start = base_idx + 1 + (j - 1) * lon_segments;
            let row1_start = base_idx + 1 + j * lon_segments;
            for i in 0..lon_segments {
                let i0 = row0_start + i;
                let i1 = row0_start + (i + 1) % lon_segments;
                let i2 = row1_start + i;
                let i3 = row1_start + (i + 1) % lon_segments;
                indices.push(i0);
                indices.push(i3);
                indices.push(i1);
                indices.push(i0);
                indices.push(i2);
                indices.push(i3);
            }
        }

        // 底极点fan三角形
        let bottom_idx = vertices.len() as u32 - 1;
        let last_row_start = base_idx + 1 + (lat_bands - 2) * lon_segments;
        for i in 0..lon_segments {
            let i0 = last_row_start + i;
            let i1 = last_row_start + (i + 1) % lon_segments;
            indices.push(bottom_idx);
            indices.push(i0);
            indices.push(i1);
        }

        Self::new(vertices, indices)
    }

    /// 创建地面平面
    ///
    /// 参数：5.5×5.5平面在y=0，法线朝上[0,1,0]
    /// 4个顶点，2个三角形。
    pub fn create_plane() -> Self {
        let white = [0.73, 0.73, 0.73, 1.0];
        let normal = [0.0, 1.0, 0.0];
        let half = 2.75;
        let vertices = vec![
            Vertex::new([-half, 0.0, -half]).with_uv([0.0, 0.0]).with_normal(normal).with_color(white),
            Vertex::new([half, 0.0, -half]).with_uv([1.0, 0.0]).with_normal(normal).with_color(white),
            Vertex::new([half, 0.0, half]).with_uv([1.0, 1.0]).with_normal(normal).with_color(white),
            Vertex::new([-half, 0.0, half]).with_uv([0.0, 1.0]).with_normal(normal).with_color(white),
        ];
        let indices = vec![0, 1, 2, 0, 2, 3];
        Self::new(vertices, indices)
    }

    /// 创建圆柱体
    ///
    /// 参数：半径0.5，高度1.0，16段(segments)
    /// 顶圆环+底圆环+顶面中心+底面中心
    /// 侧面+顶面+底面三角形，调用calculate_normals()
    pub fn create_cylinder() -> Self {
        let radius = 0.5;
        let height = 1.0;
        let segments = 16;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let white = [0.73, 0.73, 0.73, 1.0];
        let half_height = height / 2.0;

        // 顶圆环顶点
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let px = radius * angle.cos();
            let pz = radius * angle.sin();
            let u = angle / (2.0 * std::f32::consts::PI);
            vertices.push(Vertex::new([px, half_height, pz]).with_uv([u, 0.0]).with_color(white));
        }

        // 底圆环顶点
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let px = radius * angle.cos();
            let pz = radius * angle.sin();
            let u = angle / (2.0 * std::f32::consts::PI);
            vertices.push(Vertex::new([px, -half_height, pz]).with_uv([u, 1.0]).with_color(white));
        }

        // 顶面中心
        vertices.push(Vertex::new([0.0, half_height, 0.0]).with_uv([0.5, 0.5]).with_normal([0.0, 1.0, 0.0]).with_color(white));
        // 底面中心
        vertices.push(Vertex::new([0.0, -half_height, 0.0]).with_uv([0.5, 0.5]).with_normal([0.0, -1.0, 0.0]).with_color(white));

        let base_idx = 0u32;
        let top_center_idx = vertices.len() as u32 - 2;
        let bottom_center_idx = vertices.len() as u32 - 1;

        // 侧面三角形
        for i in 0..segments {
            let top_i0 = base_idx + i;
            let top_i1 = base_idx + (i + 1) % segments;
            let bottom_i0 = base_idx + segments + i;
            let bottom_i1 = base_idx + segments + (i + 1) % segments;
            // Triangle 1
            indices.push(top_i0);
            indices.push(bottom_i0);
            indices.push(bottom_i1);
            // Triangle 2
            indices.push(top_i0);
            indices.push(bottom_i1);
            indices.push(top_i1);
        }

        // 顶面三角形
        for i in 0..segments {
            let top_i0 = base_idx + i;
            let top_i1 = base_idx + (i + 1) % segments;
            indices.push(top_center_idx);
            indices.push(top_i0);
            indices.push(top_i1);
        }

        // 底面三角形
        for i in 0..segments {
            let bottom_i0 = base_idx + segments + i;
            let bottom_i1 = base_idx + segments + (i + 1) % segments;
            indices.push(bottom_center_idx);
            indices.push(bottom_i1);
            indices.push(bottom_i0);
        }

        let mut mesh = Self::new(vertices, indices);
        mesh.calculate_normals();
        mesh
    }

    /// 创建圆锥体
    ///
    /// 参数：底面半径0.5，高度1.0，16段(segments)
    /// tip顶点+底圆环+底面中心
    /// 侧面+底面三角形，调用calculate_normals()
    pub fn create_cone() -> Self {
        let base_radius = 0.5;
        let height = 1.0;
        let segments = 16;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let white = [0.73, 0.73, 0.73, 1.0];
        let half_height = height / 2.0;
        let tip_y = half_height;
        let base_y = -half_height;

        // tip顶点
        vertices.push(Vertex::new([0.0, tip_y, 0.0]).with_uv([0.5, 0.0]).with_color(white));

        // base圆环顶点
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let px = base_radius * angle.cos();
            let pz = base_radius * angle.sin();
            let u = angle / (2.0 * std::f32::consts::PI);
            vertices.push(Vertex::new([px, base_y, pz]).with_uv([u, 1.0]).with_color(white));
        }

        // base中心顶点
        vertices.push(Vertex::new([0.0, base_y, 0.0]).with_uv([0.5, 0.5]).with_color(white));

        let base_idx = 0u32;
        let tip_idx = base_idx;
        let base_center_idx = vertices.len() as u32 - 1;

        // 侧面三角形
        for i in 0..segments {
            let i0 = base_idx + 1 + i;
            let i1 = base_idx + 1 + (i + 1) % segments;
            indices.push(tip_idx);
            indices.push(i0);
            indices.push(i1);
        }

        // 底面三角形
        for i in 0..segments {
            let i0 = base_idx + 1 + i;
            let i1 = base_idx + 1 + (i + 1) % segments;
            indices.push(base_center_idx);
            indices.push(i1);
            indices.push(i0);
        }

        let mut mesh = Self::new(vertices, indices);
        mesh.calculate_normals();
        mesh
    }

    /// Cornell Box辅助: 添加一个四边形面(4顶点+2三角形)
    /// 顶点顺序保证逆时针winding从法线方向(外侧)看
    fn cornell_quad(
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
    ) {
        let base = vertices.len() as u32;
        vertices.push(Vertex::new(p0).with_uv([0.0, 0.0]).with_normal(normal).with_color(color));
        vertices.push(Vertex::new(p1).with_uv([1.0, 0.0]).with_normal(normal).with_color(color));
        vertices.push(Vertex::new(p2).with_uv([1.0, 1.0]).with_normal(normal).with_color(color));
        vertices.push(Vertex::new(p3).with_uv([0.0, 1.0]).with_normal(normal).with_color(color));
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Cornell Box辅助: 添加一个Y轴旋转的盒子(24顶点+12三角形)
    /// 顶点顺序保证逆时针winding从旋转后法线方向(外侧)看
    fn cornell_rotated_box(
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u32>,
        cx: f32, cy: f32, cz: f32,
        hx: f32, hy: f32, hz: f32,
        angle_deg: f32,
        color: [f32; 4],
    ) {
        let angle = angle_deg * std::f32::consts::PI / 180.0;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        // Y轴旋转: position绕中心旋转, normal绕Y轴旋转
        let rotate_pos = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
            [
                lx * cos_a + lz * sin_a + cx,
                ly + cy,
                -lx * sin_a + lz * cos_a + cz,
            ]
        };
        let rotate_normal = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
            [
                nx * cos_a + nz * sin_a,
                ny,
                -nx * sin_a + nz * cos_a,
            ]
        };

        // 8个局部角落(相对于中心)
        let corners: [[f32; 3]; 8] = [
            [-hx, -hy, -hz],  // c0: front-bottom-left
            [ hx, -hy, -hz],  // c1: front-bottom-right
            [ hx,  hy, -hz],  // c2: front-top-right
            [-hx,  hy, -hz],  // c3: front-top-left
            [-hx, -hy,  hz],  // c4: back-bottom-left
            [ hx, -hy,  hz],  // c5: back-bottom-right
            [ hx,  hy,  hz],  // c6: back-top-right
            [-hx,  hy,  hz],  // c7: back-top-left
        ];

        // 6个面: (corner_i0, i1, i2, i3, local_normal)
        // 顶点顺序保证CCW winding从外侧(法线方向)看
        let faces: [(usize, usize, usize, usize, [f32; 3]); 6] = [
            (0, 3, 2, 1, [0.0,  0.0, -1.0]),  // Front: normal朝-z
            (5, 6, 7, 4, [0.0,  0.0,  1.0]),  // Back: normal朝+z
            (3, 7, 6, 2, [0.0,  1.0,  0.0]),  // Top: normal朝+y
            (0, 1, 5, 4, [0.0, -1.0,  0.0]),  // Bottom: normal朝-y
            (1, 2, 6, 5, [1.0,  0.0,  0.0]),  // Right: normal朝+x
            (0, 4, 7, 3, [-1.0, 0.0,  0.0]),  // Left: normal朝-x
        ];

        let base = vertices.len() as u32;

        for (i0, i1, i2, i3, norm) in &faces {
            let p0 = rotate_pos(corners[*i0][0], corners[*i0][1], corners[*i0][2]);
            let p1 = rotate_pos(corners[*i1][0], corners[*i1][1], corners[*i1][2]);
            let p2 = rotate_pos(corners[*i2][0], corners[*i2][1], corners[*i2][2]);
            let p3 = rotate_pos(corners[*i3][0], corners[*i3][1], corners[*i3][2]);
            let n = rotate_normal(norm[0], norm[1], norm[2]);

            vertices.push(Vertex::new(p0).with_uv([0.0, 0.0]).with_normal(n).with_color(color));
            vertices.push(Vertex::new(p1).with_uv([1.0, 0.0]).with_normal(n).with_color(color));
            vertices.push(Vertex::new(p2).with_uv([1.0, 1.0]).with_normal(n).with_color(color));
            vertices.push(Vertex::new(p3).with_uv([0.0, 1.0]).with_normal(n).with_color(color));
        }

        // 6面 × 6索引 = 36索引
        for face in 0..6 {
            let f_base = base + face * 4;
            indices.extend_from_slice(&[f_base, f_base + 1, f_base + 2, f_base, f_base + 2, f_base + 3]);
        }
    }
}
