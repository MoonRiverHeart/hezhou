use hezhou_core::{Mat4, Quaternion, Vec3};

pub type CameraId = u64;

#[repr(C)]
pub struct Camera {
    pub id: CameraId,
    pub position: Vec3,
    pub rotation: Quaternion,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(id: CameraId) -> Self {
        Self {
            id,
            position: Vec3::new(0.0, 0.0, -5.0),
            rotation: Quaternion::identity(),
            fov: 60.0,
            near: 0.1,
            far: 100.0,
            aspect: 1.0,
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.position = Vec3::new(x, y, z);
    }

    pub fn set_rotation(&mut self, quat: Quaternion) {
        self.rotation = quat;
    }

    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }

    pub fn set_clip_planes(&mut self, near: f32, far: f32) {
        self.near = near;
        self.far = far;
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    pub fn look_at(&mut self, target: Vec3) {
        let forward = (target - self.position).normalized();
        let up = Vec3::up();
        let right = Vec3::cross(&forward, &up).normalized();
        let up = Vec3::cross(&right, &forward);

        // Calculate rotation from basis vectors
        self.rotation = Quaternion::from_axis_angle(Vec3::up(), 0.0);
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        Mat4::look_at(
            self.position,
            self.position + self.rotation.rotate_vector(Vec3::forward()),
            Vec3::up(),
        )
    }

    pub fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov, self.aspect, self.near, self.far)
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
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(0)
    }
}
