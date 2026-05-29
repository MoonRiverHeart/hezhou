use crate::ecs::Component;
use crate::math::{Vec3, Quaternion};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RenderableComponent {
    pub mesh_path: String,
    pub material_path: Option<String>,
    pub texture_path: Option<String>,   // 纹理文件路径（PNG/JPEG），None=无纹理
    pub visible: bool,
    pub cast_shadow: bool,
    pub receive_shadow: bool,
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