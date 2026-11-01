//! `CharacterMotor`：胶囊角色控制器（0.2 自研动作物理核心）。
//!
//! 定位约定：`position` 是**脚底中心**（角色站立时脚底所在点）。
//! 胶囊中点（圆柱段中点）为 `position + up * (half_height + radius)`，
//! 与 `ColliderShape::Capsule` 的 `half_height`（圆柱段半高）语义一致。
//!
//! 能力：
//! - `move_and_slide`：扫掠 + 滑动 + 穿透纠正 + 基础台阶处理；
//! - `ground_check`：地面吸附与可行走法线判定；
//! - `step`：便捷入口（重力 + 跳跃 + 固定步内位移）。
//!
//! 斜坡处理：法线满足 `normal.y >= cos(max_slope_angle)` 视为可行走面，
//! 位移会被投影到坡面；更陡的表面按墙面处理，角色会沿坡面滑落。
//!
//! TODO(0.3)：移动平台、推箱子、空中控制曲线、可变胶囊朝向。

use crate::query::{Hit, QueryFilter};
use crate::world::PhysicsWorld;
use hezhou_core::math::Vec3;

/// 每次 `move_and_slide` 最多滑动次数。
const MAX_SLIDES: usize = 4;

/// `move_and_slide` 的返回结果。
#[derive(Clone, Debug)]
pub struct MoveResult {
    /// 移动前脚底位置。
    pub start_position: Vec3,
    /// 移动后脚底位置。
    pub position: Vec3,
    /// 实际位移（含滑动、台阶、穿透纠正）。
    pub moved: Vec3,
    /// 本次移动中发生的命中（按发生顺序）。
    pub hits: Vec<Hit>,
    /// 移动结束后是否站在可行走地面上。
    pub grounded: bool,
    /// 地面法线（未着地时为 +Y）。
    pub ground_normal: Vec3,
    /// 是否触发了台阶抬升。
    pub stepped: bool,
    /// 是否在移动前纠正了穿透。
    pub depenetrated: bool,
}

impl MoveResult {
    /// 是否撞到不可行走的墙面。
    pub fn touched_wall(&self) -> bool {
        self.hits.iter().any(|hit| {
            let n = *hit.normal();
            n.y.abs() < 0.5 && n.y > -0.5
        })
    }

    /// 第一面墙的法线。
    pub fn wall_normal(&self) -> Option<Vec3> {
        self.hits
            .iter()
            .map(|hit| *hit.normal())
            .find(|n| n.y.abs() < 0.5 && n.y > -0.5)
    }
}

/// 胶囊角色控制器。
#[derive(Clone, Debug)]
pub struct CharacterMotor {
    /// 脚底中心位置。
    pub position: Vec3,
    /// 当前速度（米/秒）。
    pub velocity: Vec3,
    /// 胶囊半径。
    pub radius: f32,
    /// 胶囊圆柱段半高（总高 = 2 * (half_height + radius)）。
    pub half_height: f32,
    /// 可行走最大坡度（度）。
    pub max_slope_angle: f32,
    /// 可自动跨越的台阶高度（米，<=0 关闭）。
    pub step_height: f32,
    /// 地面吸附距离（米，<=0 关闭吸附）。
    pub snap_to_ground_distance: f32,
    /// 碰撞皮肤厚度（米），保证角色与表面间留出间隙。
    pub skin_width: f32,
    /// 重力加速度（米/秒²，默认 -9.81）。
    pub gravity: f32,
    /// 跳跃初速度（米/秒）。
    pub jump_speed: f32,
    /// 最大下落速度（米/秒）。
    pub max_fall_speed: f32,
    /// 当前是否着地。
    pub grounded: bool,
    /// 当前地面法线。
    pub ground_normal: Vec3,
    /// 查询层掩码（角色关心哪些层）。
    pub layer_mask: u32,
    /// 查询是否包含触发器（角色移动一般应忽略触发器）。
    pub include_sensors: bool,
    /// 排除的刚体（通常为角色自身挂载的刚体 id）。
    pub exclude_body: Option<u64>,
}

impl CharacterMotor {
    /// 以脚底位置创建默认角色（半径 0.3、总高 1.8）。
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
            velocity: Vec3::zero(),
            radius: 0.3,
            half_height: 0.6,
            max_slope_angle: 50.0,
            step_height: 0.35,
            snap_to_ground_distance: 0.15,
            skin_width: 0.02,
            gravity: -9.81,
            jump_speed: 6.0,
            max_fall_speed: -30.0,
            grounded: false,
            ground_normal: Vec3::up(),
            layer_mask: u32::MAX,
            include_sensors: false,
            exclude_body: None,
        }
    }

    /// 以脚底位置 + 半径 + 圆柱段半高创建（gameplay 适配层常用）。
    pub fn from_feet(feet: Vec3, radius: f32, half_height: f32) -> Self {
        Self::new(feet.x, feet.y, feet.z)
            .with_radius(radius)
            .with_half_height(half_height)
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.01);
        self
    }

    pub fn with_half_height(mut self, half_height: f32) -> Self {
        self.half_height = half_height.max(0.0);
        self
    }

    /// 以“总高”设置胶囊（圆柱段半高自动换算）。
    pub fn with_total_height(mut self, total_height: f32) -> Self {
        self.half_height = ((total_height - self.radius * 2.0) * 0.5).max(0.0);
        self
    }

    pub fn with_step_height(mut self, step_height: f32) -> Self {
        self.step_height = step_height.max(0.0);
        self
    }

    pub fn with_max_slope_angle(mut self, degrees: f32) -> Self {
        self.max_slope_angle = degrees.clamp(0.0, 89.0);
        self
    }

    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_jump_speed(mut self, jump_speed: f32) -> Self {
        self.jump_speed = jump_speed;
        self
    }

    pub fn with_layer_mask(mut self, layer_mask: u32) -> Self {
        self.layer_mask = layer_mask;
        self
    }

    pub fn with_sensors(mut self, include_sensors: bool) -> Self {
        self.include_sensors = include_sensors;
        self
    }

    pub fn with_exclude_body(mut self, body_id: u64) -> Self {
        self.exclude_body = Some(body_id);
        self
    }

    /// 胶囊总高。
    pub fn total_height(&self) -> f32 {
        (self.half_height + self.radius) * 2.0
    }

    /// 胶囊中点（圆柱段中点）。
    pub fn capsule_center(&self) -> Vec3 {
        self.capsule_center_at(self.position)
    }

    /// 给定脚底位置时的胶囊中点。
    pub fn capsule_center_at(&self, feet: Vec3) -> Vec3 {
        feet + Vec3::up() * (self.half_height + self.radius)
    }

    pub fn is_grounded(&self) -> bool {
        self.grounded
    }

    /// 法线是否可站立/行走。
    pub fn is_walkable(&self, normal: Vec3) -> bool {
        normal.y >= self.max_slope_angle.to_radians().cos()
    }

    /// 传送（清空速度与着地状态）。
    pub fn teleport(&mut self, feet: Vec3) {
        self.position = feet;
        self.velocity = Vec3::zero();
        self.grounded = false;
        self.ground_normal = Vec3::up();
    }

    fn query_filter(&self) -> QueryFilter {
        let mut filter = QueryFilter::new()
            .with_layer_mask(self.layer_mask)
            .with_sensors(self.include_sensors);
        if let Some(body_id) = self.exclude_body {
            filter = filter.exclude_body(body_id);
        }
        filter
    }

    /// 地面检测：向下做短扫掠/穿透检查，并把角色吸附到地面。
    ///
    /// 返回是否站在可行走面上；同时更新 `grounded` 与 `ground_normal`。
    pub fn ground_check(&mut self, world: &PhysicsWorld) -> bool {
        let filter = self.query_filter();
        self.probe_ground(world, filter)
    }

    fn probe_ground(&mut self, world: &PhysicsWorld, filter: QueryFilter) -> bool {
        let probe = self.snap_to_ground_distance.max(0.0) + self.skin_width;
        if let Some(hit) = world.sweep_capsule(
            self.capsule_center(),
            -Vec3::up() * probe,
            self.radius,
            self.half_height,
            filter,
        ) {
            let normal = *hit.normal();
            if self.is_walkable(normal) {
                let drop = (hit.distance() - self.skin_width).max(0.0);
                self.position = self.position - Vec3::up() * drop;
                self.grounded = true;
                self.ground_normal = normal;
                return true;
            }
        }
        self.grounded = false;
        self.ground_normal = Vec3::up();
        false
    }

    /// 移动并滑动：沿 `displacement` 扫掠胶囊，遇到碰撞时按法线滑动，
    /// 并处理基础台阶与地面吸附。
    ///
    /// `displacement` 是本步总位移（包含重力），调用方负责乘上固定步长。
    pub fn move_and_slide(&mut self, world: &PhysicsWorld, displacement: Vec3) -> MoveResult {
        let filter = self.query_filter();
        let start_position = self.position;
        let mut hits: Vec<Hit> = Vec::new();
        let mut remaining = displacement;
        let mut stepped = false;
        let mut depenetrated = false;
        let mut tried_step = false;

        // 1) 起始穿透纠正：生成/传送/被推入碰撞体时先把胶囊推出来。
        if let Some(penetration) = world.resolve_capsule_penetration(
            self.capsule_center(),
            self.radius,
            self.half_height,
            filter,
        ) {
            self.position = self.position + *penetration.normal() * penetration.depth();
            depenetrated = true;
        }

        // 2) 扫掠 + 滑动。
        if remaining.magnitude() > 1e-6 {
            for _ in 0..MAX_SLIDES {
                if remaining.magnitude() <= 1e-6 {
                    break;
                }
                let Some(hit) = world.sweep_capsule(
                    self.capsule_center(),
                    remaining,
                    self.radius,
                    self.half_height,
                    filter,
                ) else {
                    self.position = self.position + remaining;
                    break;
                };

                let normal = *hit.normal();
                let gap = hit.distance();
                let direction = remaining.normalized();
                let move_distance = (gap - self.skin_width).max(0.0);
                let moved = direction * move_distance;
                self.position = self.position + moved;
                remaining = remaining - moved;

                // 贴住表面（gap <= skin）时补出皮肤间隙，否则下一次扫掠会从 d=0 起步卡住。
                let push_out = (self.skin_width - gap).max(0.0);
                if push_out > 0.0 {
                    self.position = self.position + normal * push_out;
                }

                if self.is_walkable(normal) {
                    // 地面/可行走斜坡：剩余位移沿坡面滑动。
                    remaining = remaining - normal * Vec3::dot(&remaining, &normal);
                } else if normal.y < -0.5 {
                    // 撞头：吃掉向上的剩余位移。
                    if remaining.y > 0.0 {
                        remaining.y = 0.0;
                    }
                    remaining = remaining - normal * Vec3::dot(&remaining, &normal);
                } else if !tried_step && self.step_height > 0.0 && normal.y.abs() < 0.5 {
                    // 墙面：尝试基础台阶抬升。
                    tried_step = true;
                    let horizontal = Vec3::new(remaining.x, 0.0, remaining.z);
                    if let Some(moved_horizontal) = self.try_step(world, horizontal, filter) {
                        stepped = true;
                        remaining = Vec3::new(
                            remaining.x - moved_horizontal.x,
                            0.0,
                            remaining.z - moved_horizontal.z,
                        );
                        hits.push(hit);
                        continue;
                    }
                    remaining = remaining - normal * Vec3::dot(&remaining, &normal);
                } else {
                    remaining = remaining - normal * Vec3::dot(&remaining, &normal);
                }

                hits.push(hit);
            }
        }

        // 3) 地面吸附与最终着地判定（向上移动时不吸附，避免起跳瞬间被吸回地面）。
        let grounded;
        let ground_normal;
        if displacement.y <= 1e-6 {
            grounded = self.probe_ground(world, filter);
            ground_normal = self.ground_normal;
        } else {
            self.grounded = false;
            self.ground_normal = Vec3::up();
            grounded = false;
            ground_normal = Vec3::up();
        }

        MoveResult {
            start_position,
            position: self.position,
            moved: self.position - start_position,
            hits,
            grounded,
            ground_normal,
            stepped,
            depenetrated,
        }
    }

    /// 便捷步进入口：重力 + 跳跃 + 固定步内位移 + 速度修正。
    ///
    /// - `desired_velocity` 为期望的水平速度（x/z；y 由控制器接管）；
    /// - `jump` 仅在着地时生效；
    /// - 返回 `move_and_slide` 的结果，同时更新 `self.velocity`。
    pub fn step(
        &mut self,
        world: &PhysicsWorld,
        dt: f32,
        desired_velocity: Vec3,
        jump: bool,
    ) -> MoveResult {
        if !dt.is_finite() || dt <= 0.0 {
            return MoveResult {
                start_position: self.position,
                position: self.position,
                moved: Vec3::zero(),
                hits: Vec::new(),
                grounded: self.grounded,
                ground_normal: self.ground_normal,
                stepped: false,
                depenetrated: false,
            };
        }

        self.velocity.x = desired_velocity.x;
        self.velocity.z = desired_velocity.z;

        if jump && self.grounded {
            self.velocity.y = self.jump_speed;
            self.grounded = false;
        }
        self.velocity.y += self.gravity * dt;
        if self.velocity.y < self.max_fall_speed {
            self.velocity.y = self.max_fall_speed;
        }

        let displacement = self.velocity * dt;
        let result = self.move_and_slide(world, displacement);

        // 速度修正：撞墙投影、撞头清升速、落地清下落速度。
        for hit in &result.hits {
            let normal = *hit.normal();
            if normal.y < -0.5 {
                if self.velocity.y > 0.0 {
                    self.velocity.y = 0.0;
                }
            } else if !self.is_walkable(normal) {
                self.velocity = self.velocity - normal * Vec3::dot(&self.velocity, &normal);
            }
        }
        if result.grounded && self.velocity.y < 0.0 {
            self.velocity.y = 0.0;
        }

        result
    }

    /// 基础台阶处理：抬高 `step_height` 后向前试探，再向下贴地。
    ///
    /// 成功时返回水平方向的实际前进量（已更新 `self.position`）。
    fn try_step(
        &mut self,
        world: &PhysicsWorld,
        horizontal: Vec3,
        filter: QueryFilter,
    ) -> Option<Vec3> {
        let horizontal_len = horizontal.magnitude();
        if horizontal_len <= 1e-6 || self.step_height <= 0.0 {
            return None;
        }
        let start_feet = self.position;
        let raised_feet = start_feet + Vec3::up() * self.step_height;
        let raised_center = self.capsule_center_at(raised_feet);

        // 头顶空间不足时不能抬升。
        if world
            .resolve_capsule_penetration(raised_center, self.radius, self.half_height, filter)
            .is_some()
        {
            return None;
        }

        // 从抬升位置水平试探。
        let mut feet = raised_feet;
        let moved_horizontal;
        match world.sweep_capsule(
            raised_center,
            horizontal,
            self.radius,
            self.half_height,
            filter,
        ) {
            None => {
                feet = feet + horizontal;
                moved_horizontal = horizontal;
            }
            Some(hit) => {
                let distance = (hit.distance() - self.skin_width).max(0.0);
                if distance <= 1e-4 {
                    return None;
                }
                let moved = horizontal.normalized() * distance;
                feet = feet + moved;
                moved_horizontal = moved;
            }
        }
        if moved_horizontal.magnitude() <= 1e-5 {
            return None;
        }

        // 从新位置向下贴地。
        let down_probe = self.step_height + self.snap_to_ground_distance.max(0.0) + self.skin_width;
        let center = self.capsule_center_at(feet);
        let hit = world.sweep_capsule(
            center,
            Vec3::down() * down_probe,
            self.radius,
            self.half_height,
            filter,
        )?;
        let normal = *hit.normal();
        if !self.is_walkable(normal) {
            return None;
        }
        let drop = (hit.distance() - self.skin_width).max(0.0);
        let new_feet = feet - Vec3::up() * drop;

        // 只有确实抬升了才算台阶成功（避免原地“抬腿”）。
        if new_feet.y <= start_feet.y + 1e-4 {
            return None;
        }

        self.position = new_feet;
        self.grounded = true;
        self.ground_normal = normal;
        Some(moved_horizontal)
    }
}

impl Default for CharacterMotor {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl PhysicsWorld {
    /// 无状态胶囊移动求解（gameplay 适配层入口）。
    ///
    /// 以 `feet` 为脚底、按给定半径/圆柱段半高做一次 `move_and_slide`：
    /// - 不做重力/跳跃积分（由 gameplay 负责）；
    /// - 关闭自动台阶与地面吸附，行为可预测（台阶/吸附请用 `CharacterMotor`）；
    /// - 返回结果中的 `position` 即解算后的脚底位置，`hits` 即接触列表。
    pub fn move_capsule(
        &self,
        feet: Vec3,
        displacement: Vec3,
        radius: f32,
        half_height: f32,
        filter: QueryFilter,
    ) -> MoveResult {
        let mut motor = CharacterMotor::new(feet.x, feet.y, feet.z)
            .with_radius(radius)
            .with_half_height(half_height);
        motor.step_height = 0.0;
        motor.snap_to_ground_distance = 0.0;
        motor.gravity = 0.0;
        motor.layer_mask = filter.layer_mask();
        motor.include_sensors = filter.includes_sensors();
        motor.exclude_body = filter.excluded_body();
        motor.move_and_slide(self, displacement)
    }
}
