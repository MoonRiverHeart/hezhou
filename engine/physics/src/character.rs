use hezhou_core::math::Vec3;

/// 简单角色控制器：重力 + 水平移动 + 跳跃 + 地面吸附。
/// 不依赖物理刚体，适合作为 CharacterController 的稳定基础实现。
#[derive(Clone, Debug)]
pub struct CharacterController {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub height: f32,
    pub move_speed: f32,
    pub jump_speed: f32,
    pub gravity: f32,
    pub grounded: bool,
}

impl CharacterController {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
            velocity: Vec3::zero(),
            radius: 0.3,
            height: 1.8,
            move_speed: 5.0,
            jump_speed: 6.0,
            gravity: -9.81,
            grounded: false,
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// 步进：move_x/move_z 为本地输入方向（-1..1），jump 为跳跃按键。
    /// ground_y 为地面高度；角色脚底低于该值时吸附到地面。
    pub fn step(&mut self, dt: f32, move_x: f32, move_z: f32, jump: bool, ground_y: f32) {
        let mut vx = move_x * self.move_speed;
        let mut vz = move_z * self.move_speed;
        if vx.abs() + vz.abs() > self.move_speed {
            let len = (vx * vx + vz * vz).sqrt();
            vx = vx / len * self.move_speed;
            vz = vz / len * self.move_speed;
        }
        self.velocity.x = vx;
        self.velocity.z = vz;

        if self.grounded && jump {
            self.velocity.y = self.jump_speed;
            self.grounded = false;
        }
        self.velocity.y += self.gravity * dt;

        self.position.x += self.velocity.x * dt;
        self.position.y += self.velocity.y * dt;
        self.position.z += self.velocity.z * dt;

        if self.position.y <= ground_y {
            self.position.y = ground_y;
            self.velocity.y = 0.0;
            self.grounded = true;
        } else {
            self.grounded = false;
        }
    }

    pub fn is_grounded(&self) -> bool {
        self.grounded
    }
}

impl Default for CharacterController {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}
