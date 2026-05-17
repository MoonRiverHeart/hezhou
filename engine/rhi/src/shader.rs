use crate::ShaderStage;

#[derive(Clone, Debug)]
pub struct ShaderDesc {
    pub stage: ShaderStage,
    pub source: ShaderSource,
    pub entry_point: String,
}

impl ShaderDesc {
    pub fn vertex(source: ShaderSource) -> Self {
        Self {
            stage: ShaderStage::Vertex,
            source,
            entry_point: "main".to_string(),
        }
    }

    pub fn fragment(source: ShaderSource) -> Self {
        Self {
            stage: ShaderStage::Fragment,
            source,
            entry_point: "main".to_string(),
        }
    }

    pub fn geometry(source: ShaderSource) -> Self {
        Self {
            stage: ShaderStage::Geometry,
            source,
            entry_point: "main".to_string(),
        }
    }

    pub fn compute(source: ShaderSource) -> Self {
        Self {
            stage: ShaderStage::Compute,
            source,
            entry_point: "main".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ShaderSource {
    Spirv(Vec<u32>),
    Glsl(String),
    Hlsl(String),
    Wgsl(String),
    File(String),
}

impl ShaderSource {
    pub fn spirv(bytes: &[u8]) -> Self {
        let spirv: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        Self::Spirv(spirv)
    }

    pub fn glsl(source: &str) -> Self {
        Self::Glsl(source.to_string())
    }
    pub fn hlsl(source: &str) -> Self {
        Self::Hlsl(source.to_string())
    }
    pub fn wgsl(source: &str) -> Self {
        Self::Wgsl(source.to_string())
    }
    pub fn file(path: &str) -> Self {
        Self::File(path.to_string())
    }
}
