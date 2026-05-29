use crate::primitive_meshes::MeshVertex;

/// OBJ模型加载结果
pub struct ObjMeshData {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    pub name: String,
}

/// 从OBJ文件加载mesh数据
pub fn load_obj_file(path: &str) -> Result<ObjMeshData, String> {
    let (models, _materials) = tobj::load_obj(path, &tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ..Default::default()
    }).map_err(|e| format!("Failed to load OBJ file {}: {}", path, e))?;
    
    if models.is_empty() {
        return Err(format!("OBJ file {} contains no models", path));
    }
    
    // 使用第一个模型
    let model = &models[0];
    let mesh = &model.mesh;
    
    let mut vertices = Vec::with_capacity(mesh.positions.len() / 3);
    
    // tobj single_index=true 时，positions/normals/texcoords 数组长度一致
    // mesh.indices 就是可直接使用的三角形索引
    let pos_count = mesh.positions.len() / 3;
    let has_normals = !mesh.normals.is_empty();
    let has_texcoords = !mesh.texcoords.is_empty();
    
    for i in 0..pos_count {
        let px = mesh.positions[i * 3];
        let py = mesh.positions[i * 3 + 1];
        let pz = mesh.positions[i * 3 + 2];
        
        let normal = if has_normals {
            [mesh.normals[i * 3], mesh.normals[i * 3 + 1], mesh.normals[i * 3 + 2]]
        } else {
            // 默认向上normal（OBJ没有normal时）
            [0.0, 1.0, 0.0]
        };
        
        let uv = if has_texcoords {
            [mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1]]
        } else {
            // 默认UV（没有纹理坐标时）
            [0.0, 0.0]
        };
        
        vertices.push(MeshVertex {
            position: [px, py, pz],
            normal: normal,
            uv: uv,
        });
    }
    
    // mesh.indices 是三角形索引（每3个组成一个三角形）
    let indices: Vec<u32> = mesh.indices.iter().map(|i| *i as u32).collect();
    
    let name = model.name.clone();
    
    Ok(ObjMeshData {
        vertices,
        indices,
        name,
    })
}

/// 解析mesh_path获取自定义mesh路径
/// "asset://path/to/model.obj" → "path/to/model.obj"
/// "builtin://cube" 等 → None (使用primitive)
pub fn extract_custom_mesh_path(mesh_path: &str) -> Option<String> {
    if mesh_path.starts_with("asset://") {
        Some(mesh_path[8..].to_string())
    } else {
        None
    }
}

/// 判断mesh_path是否是自定义mesh
pub fn is_custom_mesh(mesh_path: &str) -> bool {
    mesh_path.starts_with("asset://")
}