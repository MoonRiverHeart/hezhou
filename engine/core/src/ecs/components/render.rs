use crate::ecs::Component;
use crate::math::{Vec3, Quaternion};
use serde::{Deserialize, Serialize};

/// 每实体 PBR 材质（0.2，编辑器/Project 持久化用）。
///
/// 该结构是 core 的纯数据契约，不依赖渲染 crate；渲染层通过
/// `UIVulkanRenderer::set_entity_pbr_material_params` / `set_entity_pbr_texture`
/// 接收同一组参数。字段名与 `hezhou_render_pipeline::PbrMaterial` 保持一致。
///
/// 限制（0.2）：仅支持 albedo 单纹理槽（`albedo_texture_path`），
/// metallic/roughness/normal/AO 纹理槽在后续版本扩展。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PbrMaterial {
    /// 基础色 RGBA（线性空间）。
    pub albedo: [f32; 4],
    /// 金属度（0=非金属，1=金属）。
    pub metallic: f32,
    /// 粗糙度（0=镜面，1=完全粗糙）。
    pub roughness: f32,
    /// 法线贴图强度。
    pub normal_scale: f32,
    /// 环境光遮蔽强度。
    pub ao: f32,
    /// 自发光 RGB。
    pub emissive: [f32; 3],
    /// 自发光强度。
    pub emissive_strength: f32,
    /// 基础色（albedo）纹理路径；`None` 表示使用中性贴图。
    pub albedo_texture_path: Option<String>,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            albedo: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            normal_scale: 1.0,
            ao: 1.0,
            emissive: [0.0, 0.0, 0.0],
            emissive_strength: 1.0,
            albedo_texture_path: None,
        }
    }
}

impl PbrMaterial {
    /// 收敛非法值：NaN/∞ 回退默认值，数值 clamp 到合理范围。
    ///
    /// 用于编辑器输入/工程旧数据，保证渲染层不会收到 NaN。
    pub fn sanitized(mut self) -> Self {
        let defaults = Self::default();
        let clamp_unit = |value: f32, fallback: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                fallback
            }
        };
        for (index, component) in self.albedo.iter_mut().enumerate() {
            *component = clamp_unit(*component, defaults.albedo[index]);
        }
        for (index, component) in self.emissive.iter_mut().enumerate() {
            *component = if component.is_finite() {
                component.max(0.0)
            } else {
                defaults.emissive[index]
            };
        }
        self.metallic = clamp_unit(self.metallic, defaults.metallic);
        self.roughness = clamp_unit(self.roughness, defaults.roughness);
        self.normal_scale = if self.normal_scale.is_finite() {
            self.normal_scale.clamp(0.0, 8.0)
        } else {
            defaults.normal_scale
        };
        self.ao = clamp_unit(self.ao, defaults.ao);
        self.emissive_strength = if self.emissive_strength.is_finite() {
            self.emissive_strength.clamp(0.0, 64.0)
        } else {
            defaults.emissive_strength
        };
        self
    }

    /// 序列化为 JSON（编辑器 FFI / Project 共用）。
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 从 JSON 解析（字段缺省取默认值，未知字段忽略）。
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let material: Self = serde_json::from_str(json)?;
        Ok(material.sanitized())
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RenderableComponent {
    pub mesh_path: String,
    pub material_path: Option<String>,
    pub texture_path: Option<String>,   // 纹理文件路径（PNG/JPEG），None=无纹理
    pub visible: bool,
    pub cast_shadow: bool,
    pub receive_shadow: bool,
    /// 每实体 PBR 材质（编辑器 Inspector / Project 持久化）。
    pub pbr_material: Option<PbrMaterial>,
}

impl Component for RenderableComponent {
    fn type_id() -> crate::ecs::ComponentTypeId { 100 }
    fn type_name() -> &'static str { "RenderableComponent" }
}

impl Default for RenderableComponent {
    fn default() -> Self {
        Self {
            mesh_path: String::new(),
            material_path: None,
            texture_path: None,
            visible: true,
            cast_shadow: true,
            receive_shadow: true,
            pbr_material: None,
        }
    }
}

impl RenderableComponent {
    pub fn new(mesh_path: String) -> Self {
        Self {
            mesh_path,
            ..Default::default()
        }
    }
    
    pub fn cube() -> Self {
        Self::new("builtin://cube".to_string())
    }
    
    pub fn sphere() -> Self {
        Self::new("builtin://sphere".to_string())
    }
    
    pub fn plane() -> Self {
        Self::new("builtin://plane".to_string())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BoundsComponent {
    pub min: Vec3,
    pub max: Vec3,
    pub local_center: Vec3,
    pub local_extent: Vec3,
}

impl Component for BoundsComponent {
    fn type_id() -> crate::ecs::ComponentTypeId { 101 }
    fn type_name() -> &'static str { "BoundsComponent" }
}

impl Default for BoundsComponent {
    fn default() -> Self {
        Self {
            min: Vec3::new(-1.0, -1.0, -1.0),
            max: Vec3::new(1.0, 1.0, 1.0),
            local_center: Vec3::zero(),
            local_extent: Vec3::new(2.0, 2.0, 2.0),
        }
    }
}

impl BoundsComponent {
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            max,
            local_center: (min + max) * 0.5,
            local_extent: max - min,
        }
    }
    
    pub fn cube(size: f32) -> Self {
        let half = size * 0.5;
        Self::from_min_max(
            Vec3::new(-half, -half, -half),
            Vec3::new(half, half, half),
        )
    }
    
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x
            && point.y >= self.min.y && point.y <= self.max.y
            && point.z >= self.min.z && point.z <= self.max.z
    }
    
    pub fn intersects_ray(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        let mut t_min: f32 = 0.0;
        let mut t_max: f32 = f32::MAX;
        
        for i in 0..3 {
            let axis = match i {
                0 => direction.x,
                1 => direction.y,
                2 => direction.z,
                _ => 0.0,
            };
            
            let min_val = match i {
                0 => self.min.x,
                1 => self.min.y,
                2 => self.min.z,
                _ => 0.0,
            };
            
            let max_val = match i {
                0 => self.max.x,
                1 => self.max.y,
                2 => self.max.z,
                _ => 0.0,
            };
            
            let origin_val = match i {
                0 => origin.x,
                1 => origin.y,
                2 => origin.z,
                _ => 0.0,
            };
            
            if axis.abs() < 1e-6 {
                if origin_val < min_val || origin_val > max_val {
                    return None;
                }
            } else {
                let t1 = (min_val - origin_val) / axis;
                let t2 = (max_val - origin_val) / axis;
                
                let (t_near, t_far) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
                
                t_min = t_min.max(t_near);
                t_max = t_max.min(t_far);
                
                if t_min > t_max {
                    return None;
                }
            }
        }
        
        Some(t_min)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalTransform {
    pub position: Vec3,
    pub rotation: Quaternion,
    pub scale: Vec3,
}

impl Component for LocalTransform {
    fn type_id() -> crate::ecs::ComponentTypeId { 102 }
    fn type_name() -> &'static str { "LocalTransform" }
}

impl LocalTransform {
    pub fn new() -> Self {
        Self {
            position: Vec3::zero(),
            rotation: Quaternion::identity(),
            scale: Vec3::one(),
        }
    }
    
    pub fn at_position(pos: Vec3) -> Self {
        Self {
            position: pos,
            ..Self::default()
        }
    }
}

// === Light Components ===

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DirectionalLightComponent {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

impl Component for DirectionalLightComponent {
    fn type_id() -> crate::ecs::ComponentTypeId { 103 }
    fn type_name() -> &'static str { "DirectionalLightComponent" }
}

impl Default for DirectionalLightComponent {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.5),
            color: Vec3::one(),
            intensity: 1.0,
        }
    }
}

impl DirectionalLightComponent {
    pub fn new(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self { direction, color, intensity }
    }
}
