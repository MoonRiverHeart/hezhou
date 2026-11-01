//! CharacterMotor：角色移动 / 奔跑 / 跳跃 / 重力 / 地面吸附的纯逻辑组件。
//!
//! 组件本身不接触物理引擎：地面探测与胶囊扫掠通过
//! [`CharacterCollisionQuery`] 由集成层注入，因此核心手感逻辑可以脱离
//! 物理实现做确定性单元测试。
//!
//! 一个典型帧：
//! 1. 集成层把输入整理成 [`MotorInput`]（世界空间方向 + 冲刺 + 跳跃 + 外部位移）；
//! 2. 调用 [`CharacterMotor::step`]，内部依次处理加速、跳跃、重力、扫掠、地面吸附；
//! 3. 用 [`MotorOutput`] 驱动渲染位置、动画状态（见 `animation_bridge`）与相机锚点。

use crate::math::{move_towards, Vec3};
use crate::query::CharacterCollisionQuery;

/// 角色移动手感参数。
///
/// 这些是"能力默认值"，具体游戏可以在初始化时整体替换，不需要改引擎代码。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorConfig {
    /// 步行速度（米/秒）。
    pub walk_speed: f32,
    /// 奔跑速度（米/秒）。
    pub run_speed: f32,
    /// 地面加速度（米/秒²）。
    pub ground_acceleration: f32,
    /// 地面减速度（米/秒²）。
    pub ground_deceleration: f32,
    /// 空中加速度（米/秒²）。
    pub air_acceleration: f32,
    /// 重力（负值，米/秒²）。
    pub gravity: f32,
    /// 最大下落速度（正值，米/秒）。
    pub max_fall_speed: f32,
    /// 起跳初速度（米/秒）。
    pub jump_velocity: f32,
    /// 松开跳跃键时上升速度的衰减倍率（0..=1，实现可变跳跃高度）。
    pub jump_cut_multiplier: f32,
    /// 土狼时间：离开地面后仍允许起跳的宽限时间（秒）。
    pub coyote_time: f32,
    /// 跳跃输入缓冲时间（秒）。
    pub jump_buffer_time: f32,
    /// 地面吸附最大距离（米），用于下坡/小台阶。
    pub ground_snap_distance: f32,
    /// 最大可站立坡度（度）。
    pub max_slope_angle_deg: f32,
    /// 角色胶囊半径（米）。
    pub radius: f32,
    /// 角色胶囊总高度（米）。
    pub height: f32,
}

impl Default for MotorConfig {
    fn default() -> Self {
        Self {
            walk_speed: 4.0,
            run_speed: 7.0,
            ground_acceleration: 40.0,
            ground_deceleration: 50.0,
            air_acceleration: 15.0,
            gravity: -20.0,
            max_fall_speed: 25.0,
            jump_velocity: 7.5,
            jump_cut_multiplier: 0.5,
            coyote_time: 0.12,
            jump_buffer_time: 0.12,
            ground_snap_distance: 0.35,
            max_slope_angle_deg: 50.0,
            radius: 0.35,
            height: 1.8,
        }
    }
}

impl MotorConfig {
    /// 最大坡度的余弦值（用于判断接触面是否可站立）。
    pub fn slope_cos(&self) -> f32 {
        self.max_slope_angle_deg.to_radians().cos()
    }
}

/// 一帧的角色输入（世界空间）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorInput {
    /// 期望移动方向，水平分量；长度会被裁剪到 1。
    pub move_direction: Vec3,
    /// 是否冲刺（影响目标速度）。
    pub sprint: bool,
    /// 本帧是否按下跳跃（需要边沿触发，不是按住状态）。
    pub jump_pressed: bool,
    /// 跳跃键当前是否按住（用于可变跳跃高度）。
    pub jump_held: bool,
    /// 外部叠加位移（闪避/攻击位移/击退），以速度形式叠加到本帧移动。
    pub external_velocity: Vec3,
    /// 水平速度倍率（例如蓄力/收招时减速），1.0 为正常。
    pub speed_scale: f32,
}

impl Default for MotorInput {
    fn default() -> Self {
        Self {
            move_direction: Vec3::ZERO,
            sprint: false,
            jump_pressed: false,
            jump_held: false,
            external_velocity: Vec3::ZERO,
            speed_scale: 1.0,
        }
    }
}

impl MotorInput {
    /// 只带移动方向的输入。
    pub fn with_direction(direction: Vec3) -> Self {
        Self {
            move_direction: direction,
            ..Self::default()
        }
    }
}

/// 一帧的角色输出，供渲染/动画/相机读取。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorOutput {
    /// 脚底位置（世界空间）。
    pub position: Vec3,
    /// 当前速度。
    pub velocity: Vec3,
    /// 是否站在地面上。
    pub grounded: bool,
    /// 本帧是否起跳。
    pub just_jumped: bool,
    /// 本帧是否落地。
    pub just_landed: bool,
    /// 水平速度大小。
    pub speed: f32,
    /// 移动强度（0..=1），可用于动画混合 Idle/Move/Run。
    pub move_intensity: f32,
}

/// 角色移动组件（纯逻辑）。
#[derive(Clone, Debug)]
pub struct CharacterMotor {
    config: MotorConfig,
    position: Vec3,
    velocity: Vec3,
    grounded: bool,
    coyote_timer: f32,
    jump_buffer_timer: f32,
    jump_cut_available: bool,
}

impl CharacterMotor {
    /// 在原点创建角色。
    pub fn new(config: MotorConfig) -> Self {
        Self::with_position(config, Vec3::ZERO)
    }

    /// 在指定脚底位置创建角色。
    pub fn with_position(config: MotorConfig, position: Vec3) -> Self {
        Self {
            config,
            position,
            velocity: Vec3::ZERO,
            grounded: false,
            coyote_timer: 0.0,
            jump_buffer_timer: 0.0,
            jump_cut_available: false,
        }
    }

    /// 当前配置。
    pub fn config(&self) -> &MotorConfig {
        &self.config
    }

    /// 修改配置（通常用于难度/质量档切换）。
    pub fn set_config(&mut self, config: MotorConfig) {
        self.config = config;
    }

    /// 脚底位置。
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// 当前速度。
    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    /// 是否在地面上。
    pub fn is_grounded(&self) -> bool {
        self.grounded
    }

    /// 传送到指定脚底位置并清空速度/计时器（重生、场景切换用）。
    pub fn teleport(&mut self, position: Vec3) {
        self.position = position;
        self.velocity = Vec3::ZERO;
        self.grounded = false;
        self.coyote_timer = 0.0;
        self.jump_buffer_timer = 0.0;
        self.jump_cut_available = false;
    }

    /// 直接设置速度（击退/外部系统用）。
    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }

    /// 推进一帧。
    ///
    /// `dt` 会被限制在 `[0, 0.1]`，避免掉帧时穿墙式的大位移；
    /// 手感与帧率无关的部分由固定时间步集成层保证。
    pub fn step(
        &mut self,
        dt: f32,
        input: &MotorInput,
        query: &dyn CharacterCollisionQuery,
    ) -> MotorOutput {
        let dt = dt.clamp(0.0, 0.1);
        let was_grounded = self.grounded;
        let slope_cos = self.config.slope_cos();
        let mut just_jumped = false;

        // 1. 跳跃输入缓冲
        if input.jump_pressed {
            self.jump_buffer_timer = self.config.jump_buffer_time;
        } else {
            self.jump_buffer_timer = (self.jump_buffer_timer - dt).max(0.0);
        }

        // 2. 水平目标速度与加减速
        let direction = input.move_direction.horizontal().clamp_length(1.0);
        let input_scale = direction.magnitude();
        let target_speed = (if input.sprint {
            self.config.run_speed
        } else {
            self.config.walk_speed
        }) * input_scale
            * input.speed_scale.max(0.0);
        let target_velocity = direction.normalized() * target_speed;
        let current_horizontal = self.velocity.horizontal();
        let horizontal_speed = current_horizontal.magnitude();
        let acceleration = if !was_grounded {
            self.config.air_acceleration
        } else if target_speed >= horizontal_speed {
            self.config.ground_acceleration
        } else {
            self.config.ground_deceleration
        };
        let mut horizontal_velocity =
            move_towards(current_horizontal, target_velocity, acceleration * dt);

        // 3. 起跳（土狼时间 + 输入缓冲）
        if self.jump_buffer_timer > 0.0 && (was_grounded || self.coyote_timer > 0.0) {
            self.velocity.y = self.config.jump_velocity;
            self.coyote_timer = 0.0;
            self.jump_buffer_timer = 0.0;
            self.jump_cut_available = true;
            just_jumped = true;
        }

        // 4. 重力与可变跳跃高度（起跳帧与站立帧不叠加重力）
        if was_grounded && !just_jumped {
            self.velocity.y = self.velocity.y.min(0.0);
        } else {
            self.velocity.y += self.config.gravity * dt;
            if !input.jump_held && self.velocity.y > 0.0 && self.jump_cut_available {
                self.velocity.y *= self.config.jump_cut_multiplier.clamp(0.0, 1.0);
                self.jump_cut_available = false;
            }
            self.velocity.y = self.velocity.y.max(-self.config.max_fall_speed);
        }

        // 5. 扫掠移动（先清空 grounded，由本帧接触/吸附重新确认）
        self.grounded = false;
        let movement_velocity = horizontal_velocity + input.external_velocity.horizontal();
        let delta = Vec3::new(
            movement_velocity.x * dt,
            self.velocity.y * dt,
            movement_velocity.z * dt,
        );
        let result =
            query.move_capsule(self.position, delta, self.config.radius, self.config.height);
        self.position = result.position;

        // 6. 接触解算
        for contact in &result.contacts {
            if contact.normal.y >= slope_cos {
                if delta.y <= 0.0 {
                    self.grounded = true;
                    self.velocity.y = self.velocity.y.min(0.0);
                    self.jump_cut_available = false;
                }
            } else if contact.normal.y <= -0.5 {
                if self.velocity.y > 0.0 {
                    self.velocity.y = 0.0;
                    self.jump_cut_available = false;
                }
            } else {
                let normal = contact.normal.horizontal().normalized();
                let into_wall = horizontal_velocity.dot(&normal);
                if into_wall < 0.0 {
                    horizontal_velocity -= normal * into_wall;
                }
            }
        }

        // 7. 地面吸附（下坡、小台阶；起跳当帧跳过）
        if !just_jumped && self.velocity.y <= 0.0 && (was_grounded || self.grounded) {
            if let Some(hit) = query.ground_probe(
                self.position,
                self.config.radius,
                self.config.ground_snap_distance,
            ) {
                if hit.normal.y >= slope_cos {
                    self.position = hit.point;
                    self.velocity.y = 0.0;
                    self.grounded = true;
                }
            }
        }

        // 8. 速度状态与计时器
        self.velocity.x = horizontal_velocity.x;
        self.velocity.z = horizontal_velocity.z;
        if self.grounded || was_grounded {
            self.coyote_timer = self.config.coyote_time;
        } else {
            self.coyote_timer = (self.coyote_timer - dt).max(0.0);
        }

        let speed = self.velocity.horizontal_length();
        let just_landed = !was_grounded && self.grounded;
        MotorOutput {
            position: self.position,
            velocity: self.velocity,
            grounded: self.grounded,
            just_jumped,
            just_landed,
            speed,
            move_intensity: (speed / self.config.run_speed.max(f32::EPSILON)).clamp(0.0, 1.0),
        }
    }
}

impl Default for CharacterMotor {
    fn default() -> Self {
        Self::new(MotorConfig::default())
    }
}
