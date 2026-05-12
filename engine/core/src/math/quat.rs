use crate::math::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
    
    pub fn identity() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
    
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let s = half_angle.sin();
        let c = half_angle.cos();
        let normalized_axis = axis.normalized();
        
        Self::new(
            normalized_axis.x * s,
            normalized_axis.y * s,
            normalized_axis.z * s,
            c,
        )
    }
    
    pub fn from_euler(x: f32, y: f32, z: f32) -> Self {
        let cx = (x * 0.5).cos();
        let sx = (x * 0.5).sin();
        let cy = (y * 0.5).cos();
        let sy = (y * 0.5).sin();
        let cz = (z * 0.5).cos();
        let sz = (z * 0.5).sin();
        
        Self::new(
            sx * cy * cz - cx * sy * sz,
            cx * sy * cz + sx * cy * sz,
            cx * cy * sz - sx * sy * cz,
            cx * cy * cz + sx * sy * sz,
        )
    }
    
    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }
    
    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 {
            Self::new(self.x / mag, self.y / mag, self.z / mag, self.w / mag)
        } else {
            Self::identity()
        }
    }
    
    pub fn conjugate(&self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }
    
    pub fn inverse(&self) -> Self {
        let mag_sq = self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
        if mag_sq > 0.0 {
            let conj = self.conjugate();
            Self::new(conj.x / mag_sq, conj.y / mag_sq, conj.z / mag_sq, conj.w / mag_sq)
        } else {
            Self::identity()
        }
    }
    
    pub fn multiply(a: &Self, b: &Self) -> Self {
        Self::new(
            a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
            a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        )
    }
    
    pub fn rotate_vector(&self, v: Vec3) -> Vec3 {
        let qv = Quaternion::new(v.x, v.y, v.z, 0.0);
        let result = Quaternion::multiply(
            &Quaternion::multiply(self, &qv),
            &self.conjugate(),
        );
        Vec3::new(result.x, result.y, result.z)
    }
    
    pub fn slerp(a: &Self, b: &Self, t: f32) -> Self {
        let dot = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
        let dot = dot.clamp(-1.0, 1.0);
        
        let theta = dot.acos();
        let sin_theta = theta.sin();
        
        if sin_theta.abs() < 0.0001 {
            return Self::new(
                a.x + (b.x - a.x) * t,
                a.y + (b.y - a.y) * t,
                a.z + (b.z - a.z) * t,
                a.w + (b.w - a.w) * t,
            );
        }
        
        let wa = ((1.0 - t) * theta).sin() / sin_theta;
        let wb = (t * theta).sin() / sin_theta;
        
        Self::new(
            a.x * wa + b.x * wb,
            a.y * wa + b.y * wb,
            a.z * wa + b.z * wb,
            a.w * wa + b.w * wb,
        )
    }
}

impl std::ops::Mul for Quaternion {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Quaternion::multiply(&self, &other)
    }
}