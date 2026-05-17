use hezhou_core::math::Vec3;

#[derive(Clone, Debug)]
pub struct RigidBody {
    position: Vec3,
    rotation: nalgebra::UnitQuaternion<f32>,
    velocity: Vec3,
    angular_velocity: Vec3,
    body_type: RigidBodyType,
    mass: f32,
    linear_damping: f32,
    angular_damping: f32,
    gravity_scale: f32,
    can_sleep: bool,
    is_sleeping: bool,
}

impl RigidBody {
    pub fn new() -> Self {
        Self {
            position: Vec3::zero(),
            rotation: nalgebra::UnitQuaternion::identity(),
            velocity: Vec3::zero(),
            angular_velocity: Vec3::zero(),
            body_type: RigidBodyType::Dynamic,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            can_sleep: true,
            is_sleeping: false,
        }
    }

    pub fn dynamic() -> Self {
        Self::new()
    }

    pub fn static_body() -> Self {
        Self {
            body_type: RigidBodyType::Static,
            ..Self::new()
        }
    }

    pub fn kinematic() -> Self {
        Self {
            body_type: RigidBodyType::Kinematic,
            ..Self::new()
        }
    }

    pub fn position(&self) -> &Vec3 {
        &self.position
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn rotation(&self) -> &nalgebra::UnitQuaternion<f32> {
        &self.rotation
    }

    pub fn set_rotation(&mut self, rotation: nalgebra::UnitQuaternion<f32>) {
        self.rotation = rotation;
    }

    pub fn velocity(&self) -> &Vec3 {
        &self.velocity
    }

    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }

    pub fn angular_velocity(&self) -> &Vec3 {
        &self.angular_velocity
    }

    pub fn set_angular_velocity(&mut self, angular_velocity: Vec3) {
        self.angular_velocity = angular_velocity;
    }

    pub fn body_type(&self) -> RigidBodyType {
        self.body_type
    }

    pub fn set_body_type(&mut self, body_type: RigidBodyType) {
        self.body_type = body_type;
    }

    pub fn mass(&self) -> f32 {
        self.mass
    }

    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass;
    }

    pub fn linear_damping(&self) -> f32 {
        self.linear_damping
    }

    pub fn set_linear_damping(&mut self, linear_damping: f32) {
        self.linear_damping = linear_damping;
    }

    pub fn angular_damping(&self) -> f32 {
        self.angular_damping
    }

    pub fn set_angular_damping(&mut self, angular_damping: f32) {
        self.angular_damping = angular_damping;
    }

    pub fn gravity_scale(&self) -> f32 {
        self.gravity_scale
    }

    pub fn set_gravity_scale(&mut self, gravity_scale: f32) {
        self.gravity_scale = gravity_scale;
    }

    pub fn is_sleeping(&self) -> bool {
        self.is_sleeping
    }

    pub fn wake_up(&mut self) {
        self.is_sleeping = false;
    }

    pub fn sleep(&mut self) {
        if self.can_sleep {
            self.is_sleeping = true;
        }
    }
}

impl Default for RigidBody {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RigidBodyType {
    Dynamic,
    Static,
    Kinematic,
}
