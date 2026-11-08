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
    /// 该刚体是否参与 CCD 扫掠（0.3 基础 CCD，默认关闭）。
    ccd_enabled: bool,
    /// 低速度持续计时（自动睡眠用，0.3 可选能力）。
    sleep_timer: f32,
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
            ccd_enabled: false,
            sleep_timer: 0.0,
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

    /// 以欧拉角（弧度，XYZ 顺序）设置旋转。编辑器/脚本侧比四元数更易用。
    pub fn set_rotation_euler(&mut self, x: f32, y: f32, z: f32) {
        self.rotation = nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
    }

    /// 以“轴 + 角度（弧度）”设置旋转。
    pub fn set_rotation_axis_angle(&mut self, axis: Vec3, angle_radians: f32) {
        let axis = axis.normalized();
        if axis.magnitude() <= f32::EPSILON {
            self.rotation = nalgebra::UnitQuaternion::identity();
            return;
        }
        let axis = nalgebra::Vector3::new(axis.x, axis.y, axis.z);
        self.rotation = nalgebra::UnitQuaternion::from_axis_angle(
            &nalgebra::Unit::new_normalize(axis),
            angle_radians,
        );
    }

    /// 刚体局部 +Y 轴在世界空间的方向（`Plane` 碰撞体用它作平面法线）。
    pub fn rotation_up(&self) -> Vec3 {
        let v = self.rotation * nalgebra::Vector3::new(0.0, 1.0, 0.0);
        Vec3::new(v.x, v.y, v.z)
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

    pub fn is_dynamic(&self) -> bool {
        self.body_type == RigidBodyType::Dynamic
    }

    pub fn is_static(&self) -> bool {
        self.body_type == RigidBodyType::Static
    }

    pub fn is_kinematic(&self) -> bool {
        self.body_type == RigidBodyType::Kinematic
    }

    pub fn set_body_type(&mut self, body_type: RigidBodyType) {
        self.body_type = body_type;
    }

    pub fn mass(&self) -> f32 {
        self.mass
    }

    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass.max(0.0);
    }

    /// 逆质量：动态刚体返回 `1 / mass`（mass<=0 视为无限质量），其余类型返回 0。
    ///
    /// 冲量求解统一用逆质量：静态/运动学刚体天然不可推动。
    pub fn inverse_mass(&self) -> f32 {
        if self.body_type == RigidBodyType::Dynamic && self.mass > 0.0 {
            1.0 / self.mass
        } else {
            0.0
        }
    }

    /// 是否可被冲量/碰撞推动（动态且有质量且未睡眠）。
    pub fn is_movable(&self) -> bool {
        self.body_type == RigidBodyType::Dynamic && self.mass > 0.0 && !self.is_sleeping
    }

    /// 线速度大小的平方（用于睡眠阈值判断）。
    pub fn speed_squared(&self) -> f32 {
        let v = self.velocity;
        v.x * v.x + v.y * v.y + v.z * v.z
    }

    /// 施加线冲量（世界空间，单位 N·s）：`Δv = impulse * 1/mass`，并唤醒刚体。
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        let inv_mass = self.inverse_mass();
        if inv_mass <= 0.0 {
            return;
        }
        self.wake_up();
        self.velocity = self.velocity + impulse * inv_mass;
    }

    /// 按角速度积分旋转（世界空间角速度，近似一阶积分，0.3 第一版）。
    pub fn integrate_rotation(&mut self, dt: f32) {
        let w = self.angular_velocity;
        let angle = w.magnitude() * dt;
        if angle <= 1e-6 {
            return;
        }
        let axis = w.normalized();
        let axis = nalgebra::Vector3::new(axis.x, axis.y, axis.z);
        let delta =
            nalgebra::UnitQuaternion::from_axis_angle(&nalgebra::Unit::new_normalize(axis), angle);
        // 世界空间角速度：delta 左乘。
        self.rotation = delta * self.rotation;
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
        self.sleep_timer = 0.0;
    }

    pub fn sleep(&mut self) {
        if self.can_sleep {
            self.is_sleeping = true;
        }
    }

    pub fn can_sleep(&self) -> bool {
        self.can_sleep
    }

    pub fn set_can_sleep(&mut self, can_sleep: bool) {
        self.can_sleep = can_sleep;
    }

    /// 是否启用 CCD（需要 `PhysicsWorld::set_ccd_enabled(true)` 全局开关同时打开）。
    pub fn ccd_enabled(&self) -> bool {
        self.ccd_enabled
    }

    pub fn set_ccd_enabled(&mut self, enabled: bool) {
        self.ccd_enabled = enabled;
    }

    pub(crate) fn sleep_timer(&self) -> f32 {
        self.sleep_timer
    }

    pub(crate) fn add_sleep_time(&mut self, dt: f32) {
        self.sleep_timer += dt;
    }

    pub(crate) fn reset_sleep_timer(&mut self) {
        self.sleep_timer = 0.0;
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
