use crate::math::{Vec3, Quaternion, Mat4};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quaternion,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::zero(),
            rotation: Quaternion::identity(),
            scale: Vec3::one(),
        }
    }
    
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quaternion::identity(),
            scale: Vec3::one(),
        }
    }
    
    pub fn from_position_rotation(position: Vec3, rotation: Quaternion) -> Self {
        Self {
            position,
            rotation,
            scale: Vec3::one(),
        }
    }
    
    pub fn to_matrix(&self) -> Mat4 {
        let t = Mat4::translate(self.position);
        let r = Mat4::from_quaternion(self.rotation);
        let s = Mat4::scale(self.scale);
        t * r * s
    }
    
    pub fn forward(&self) -> Vec3 {
        self.rotation.rotate_vector(Vec3::forward())
    }
    
    pub fn up(&self) -> Vec3 {
        self.rotation.rotate_vector(Vec3::up())
    }
    
    pub fn right(&self) -> Vec3 {
        self.rotation.rotate_vector(Vec3::right())
    }
    
    pub fn look_at(&mut self, target: Vec3) {
        let direction = (target - self.position).normalized();
        let forward = Vec3::forward();
        let dot = Vec3::dot(&forward, &direction);
        
        if dot.abs() > 0.9999 {
            if dot > 0.0 {
                self.rotation = Quaternion::identity();
            } else {
                self.rotation = Quaternion::from_axis_angle(Vec3::up(), std::f32::consts::PI);
            }
        } else {
            let axis = Vec3::cross(&forward, &direction).normalized();
            let angle = dot.acos();
            self.rotation = Quaternion::from_axis_angle(axis, angle);
        }
    }
    
    pub fn translate(&mut self, offset: Vec3) {
        self.position = self.position + offset;
    }
    
    pub fn rotate(&mut self, axis: Vec3, angle: f32) {
        let delta = Quaternion::from_axis_angle(axis, angle);
        self.rotation = delta * self.rotation;
    }
    
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self {
            position: Vec3::lerp(&a.position, &b.position, t),
            rotation: Quaternion::slerp(&a.rotation, &b.rotation, t),
            scale: Vec3::lerp(&a.scale, &b.scale, t),
        }
    }
}