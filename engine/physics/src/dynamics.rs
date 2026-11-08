//! 动态刚体求解：积分、接触冲量、摩擦、位置修正、睡眠。
//!
//! 0.3 第一批只做**线性动力学**：接触响应不产生角冲量（无惯性张量），
//! 旋转仅按角速度做一阶积分。理由：碰撞体仍是轴对齐 AABB 近似，
//! 引入角冲量会让旋转盒视觉与碰撞不一致；完整旋转动力学留后续版本。

use crate::world::PhysicsWorld;
use hezhou_core::math::Vec3;

/// 求解器配置（迭代次数、位置修正比例、恢复阈值、睡眠阈值）。
#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    /// 速度冲量迭代次数（越多堆叠越稳，默认 6）。
    pub velocity_iterations: u32,
    /// 位置修正迭代次数（默认 3）。
    pub position_iterations: u32,
    /// 每次位置修正的比例（0..1，默认 0.8）。
    pub position_correction: f32,
    /// 允许的穿透容差（米，默认 0.005）。
    pub penetration_slop: f32,
    /// 恢复系数生效的最小法向接近速度（米/秒，默认 1.0）。
    pub restitution_threshold: f32,
    /// 最大线速度限制（米/秒，默认 500，防止数值爆炸）。
    pub max_linear_velocity: f32,
    /// 自动睡眠的线速度阈值（米/秒，默认 0.05）。
    pub sleep_linear_threshold: f32,
    /// 自动睡眠的角速度阈值（弧度/秒，默认 0.05）。
    pub sleep_angular_threshold: f32,
    /// 低于阈值持续多久后进入睡眠（秒，默认 0.5）。
    pub sleep_delay: f32,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            velocity_iterations: 6,
            position_iterations: 3,
            position_correction: 0.8,
            penetration_slop: 0.005,
            restitution_threshold: 1.0,
            max_linear_velocity: 500.0,
            sleep_linear_threshold: 0.05,
            sleep_angular_threshold: 0.05,
            sleep_delay: 0.5,
        }
    }
}

/// 一个接触约束（内部）。
///
/// 法线约定：`normal` 从 `body_a` 指向 `body_b`；位置修正把 B 沿 `+normal` 推出、
/// A 沿 `-normal` 推出。
#[derive(Clone, Copy, Debug)]
pub(crate) struct Contact {
    pub body_a: u64,
    pub body_b: u64,
    pub normal: Vec3,
    pub penetration: f32,
    pub restitution: f32,
    pub friction: f32,
    /// 近似接触点（世界空间），保留给调试/后续受击反馈使用。
    #[allow(dead_code)]
    pub point: Vec3,
}

impl PhysicsWorld {
    /// 动态刚体速度积分：重力、线性/角阻尼、最大速度限制。
    pub(crate) fn integrate_velocities(&mut self, dt: f32) {
        let gravity = self.gravity;
        let max_speed = self.solver_config.max_linear_velocity;
        for body in self.bodies.values_mut() {
            if !body.is_dynamic() || body.is_sleeping() {
                continue;
            }
            let mut velocity = *body.velocity() + gravity * body.gravity_scale() * dt;

            // 线性阻尼：隐式形式 v *= 1/(1 + damping*dt)，任意步长下稳定。
            let linear_damping = body.linear_damping().max(0.0);
            if linear_damping > 0.0 {
                velocity = velocity * (1.0 / (1.0 + linear_damping * dt));
            }
            let speed = velocity.magnitude();
            if max_speed > 0.0 && speed > max_speed {
                velocity = velocity * (max_speed / speed);
            }
            body.set_velocity(velocity);

            let mut angular = *body.angular_velocity();
            let angular_damping = body.angular_damping().max(0.0);
            if angular_damping > 0.0 {
                angular = angular * (1.0 / (1.0 + angular_damping * dt));
            }
            body.set_angular_velocity(angular);
        }
    }

    /// 动态刚体位置积分；启用 CCD 的刚体先做扫掠，命中则停在接触点前。
    pub(crate) fn integrate_positions(&mut self, dt: f32) {
        let ids: Vec<u64> = self
            .bodies
            .iter()
            .filter(|(_, body)| body.is_dynamic() && !body.is_sleeping())
            .map(|(id, _)| *id)
            .collect();
        let ccd_global = self.ccd_enabled;

        for id in ids {
            let (displacement, ccd, restitution, own_inv_mass) = {
                let Some(body) = self.bodies.get(&id) else {
                    continue;
                };
                let displacement = *body.velocity() * dt;
                let restitution = self
                    .body_collider_map
                    .get(&id)
                    .and_then(|collider_id| self.colliders.get(collider_id))
                    .map(|collider| collider.restitution())
                    .unwrap_or(0.0);
                let ccd = if ccd_global && body.ccd_enabled() {
                    self.ccd_sweep(id, displacement, dt)
                } else {
                    None
                };
                let own_inv_mass = if body.is_movable() {
                    body.inverse_mass()
                } else {
                    0.0
                };
                (displacement, ccd, restitution, own_inv_mass)
            };

            // CCD 冲量在可变借用前算好（需要读取目标速度/逆质量）。
            let mut ccd_impulse = Vec3::zero();
            if let Some(hit) = &ccd {
                let own_velocity = self
                    .bodies
                    .get(&id)
                    .map(|body| *body.velocity())
                    .unwrap_or(Vec3::zero());
                let relative_velocity = own_velocity - hit.target_velocity;
                let normal_speed = Vec3::dot(&relative_velocity, &hit.normal);
                if normal_speed < 0.0 {
                    let inv_sum = own_inv_mass + hit.target_inv_mass;
                    if inv_sum > 0.0 {
                        let restitution = restitution.clamp(0.0, 1.0);
                        let normal_impulse = -(1.0 + restitution) * normal_speed / inv_sum;
                        ccd_impulse = hit.normal * normal_impulse;
                    }
                }
            }

            {
                let Some(body) = self.bodies.get_mut(&id) else {
                    continue;
                };
                if let Some(hit) = ccd {
                    // 停在接触点前一点点，避免离散检测再次判定为穿透。
                    let advance = (hit.t - 1e-4).max(0.0);
                    let position = *body.position() + displacement * advance + hit.normal * 1e-3;
                    body.set_position(position);
                    if own_inv_mass > 0.0 {
                        body.set_velocity(*body.velocity() + ccd_impulse * own_inv_mass);
                    }
                } else {
                    body.set_position(*body.position() + displacement);
                }
                body.integrate_rotation(dt);
            }

            // 动态-动态：对命中的动态目标施加反向冲量（动量守恒）。
            if let Some(hit) = ccd {
                if hit.target_inv_mass > 0.0 {
                    if let Some(target) = self.bodies.get_mut(&hit.target_body) {
                        target.set_velocity(*target.velocity() - ccd_impulse * hit.target_inv_mass);
                    }
                }
            }
        }
    }

    /// 接触把睡眠刚体唤醒（被快速移动的刚体撞击时）。
    pub(crate) fn wake_bodies_on_contacts(&mut self, contacts: &[Contact]) {
        let wake_speed = 0.2f32;
        for contact in contacts {
            let sleeping_a = self
                .bodies
                .get(&contact.body_a)
                .map(|body| body.is_sleeping())
                .unwrap_or(false);
            let sleeping_b = self
                .bodies
                .get(&contact.body_b)
                .map(|body| body.is_sleeping())
                .unwrap_or(false);
            if !sleeping_a && !sleeping_b {
                continue;
            }
            let speed_a = self
                .bodies
                .get(&contact.body_a)
                .map(|body| body.speed_squared())
                .unwrap_or(0.0)
                .sqrt();
            let speed_b = self
                .bodies
                .get(&contact.body_b)
                .map(|body| body.speed_squared())
                .unwrap_or(0.0)
                .sqrt();
            if sleeping_a && speed_b > wake_speed {
                if let Some(body) = self.bodies.get_mut(&contact.body_a) {
                    body.wake_up();
                }
            }
            if sleeping_b && speed_a > wake_speed {
                if let Some(body) = self.bodies.get_mut(&contact.body_b) {
                    body.wake_up();
                }
            }
        }
    }

    pub(crate) fn solver_inverse_mass(&self, body_id: u64) -> f32 {
        self.bodies
            .get(&body_id)
            .map(|body| {
                if body.is_movable() {
                    body.inverse_mass()
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0)
    }

    /// 速度冲量求解：法向冲量 + 库仑摩擦（线性，无角冲量）。
    pub(crate) fn solve_contact_velocities(&mut self, contacts: &[Contact]) {
        let iterations = self.solver_config.velocity_iterations.max(1);
        let restitution_threshold = self.solver_config.restitution_threshold;

        for iteration in 0..iterations {
            for contact in contacts {
                let inv_a = self.solver_inverse_mass(contact.body_a);
                let inv_b = self.solver_inverse_mass(contact.body_b);
                let inv_sum = inv_a + inv_b;
                if inv_sum <= 0.0 {
                    continue;
                }
                let (Some(body_a), Some(body_b)) = (
                    self.bodies.get(&contact.body_a),
                    self.bodies.get(&contact.body_b),
                ) else {
                    continue;
                };
                let velocity_a = *body_a.velocity();
                let velocity_b = *body_b.velocity();
                let normal = contact.normal;
                let relative = velocity_b - velocity_a;
                let normal_speed = Vec3::dot(&relative, &normal);
                // 已经在分离：不施加法向冲量（位置修正仍会处理残余穿透）。
                if normal_speed > 0.0 {
                    continue;
                }

                // 恢复系数只在第一次迭代生效，且需要足够接近速度。
                let restitution = if iteration == 0 && -normal_speed > restitution_threshold {
                    contact.restitution.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let normal_impulse = -(1.0 + restitution) * normal_speed / inv_sum;
                let impulse = normal * normal_impulse;

                if inv_a > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&contact.body_a) {
                        body.set_velocity(*body.velocity() - impulse * inv_a);
                    }
                }
                if inv_b > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&contact.body_b) {
                        body.set_velocity(*body.velocity() + impulse * inv_b);
                    }
                }

                // 摩擦：重新读取速度，消除切向相对速度（库仑上限）。
                let Some(body_a) = self.bodies.get(&contact.body_a) else {
                    continue;
                };
                let Some(body_b) = self.bodies.get(&contact.body_b) else {
                    continue;
                };
                let relative = *body_b.velocity() - *body_a.velocity();
                let normal_component = Vec3::dot(&relative, &normal);
                let tangential = relative - normal * normal_component;
                let tangential_speed = tangential.magnitude();
                if tangential_speed <= 1e-5 {
                    continue;
                }
                let tangent = tangential * (1.0 / tangential_speed);
                let raw = -tangential_speed / inv_sum;
                let max_friction = contact.friction.max(0.0) * normal_impulse.abs();
                let friction_impulse = raw.clamp(-max_friction, max_friction);
                let impulse = tangent * friction_impulse;

                if inv_a > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&contact.body_a) {
                        body.set_velocity(*body.velocity() - impulse * inv_a);
                    }
                }
                if inv_b > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&contact.body_b) {
                        body.set_velocity(*body.velocity() + impulse * inv_b);
                    }
                }
            }
        }
    }

    /// 位置修正一次（返回是否发生了修正）。
    pub(crate) fn correct_positions_once(&mut self, contacts: &[Contact]) -> bool {
        let config = self.solver_config;
        let mut corrected = false;
        for contact in contacts {
            if contact.penetration <= config.penetration_slop {
                continue;
            }
            let inv_a = self.solver_inverse_mass(contact.body_a);
            let inv_b = self.solver_inverse_mass(contact.body_b);
            let inv_sum = inv_a + inv_b;
            if inv_sum <= 0.0 {
                continue;
            }
            let correction =
                (contact.penetration - config.penetration_slop) * config.position_correction;
            if inv_a > 0.0 {
                if let Some(body) = self.bodies.get_mut(&contact.body_a) {
                    body.set_position(
                        *body.position() - contact.normal * (correction * inv_a / inv_sum),
                    );
                    corrected = true;
                }
            }
            if inv_b > 0.0 {
                if let Some(body) = self.bodies.get_mut(&contact.body_b) {
                    body.set_position(
                        *body.position() + contact.normal * (correction * inv_b / inv_sum),
                    );
                    corrected = true;
                }
            }
        }
        corrected
    }

    /// 自动睡眠：低线速度/角速度持续 `sleep_delay` 后进入睡眠。
    pub(crate) fn update_sleeping(&mut self, dt: f32) {
        if !self.auto_sleep {
            return;
        }
        let config = self.solver_config;
        let linear_threshold_sq = config.sleep_linear_threshold * config.sleep_linear_threshold;
        let angular_threshold_sq = config.sleep_angular_threshold * config.sleep_angular_threshold;
        for body in self.bodies.values_mut() {
            if !body.is_dynamic() || body.is_sleeping() || !body.can_sleep() {
                continue;
            }
            let angular = *body.angular_velocity();
            let low_linear = body.speed_squared() < linear_threshold_sq;
            let low_angular = angular.x * angular.x + angular.y * angular.y + angular.z * angular.z
                < angular_threshold_sq;
            if low_linear && low_angular {
                body.add_sleep_time(dt);
                if body.sleep_timer() >= config.sleep_delay {
                    body.set_velocity(Vec3::zero());
                    body.set_angular_velocity(Vec3::zero());
                    body.sleep();
                }
            } else {
                body.reset_sleep_timer();
            }
        }
    }
}
