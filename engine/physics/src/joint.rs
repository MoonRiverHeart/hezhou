//! 距离关节（0.3 第一版）。
//!
//! 约束两刚体上两个世界锚点之间的距离为 `rest_length`：
//! - 速度层：消除沿约束轴的相对速度；
//! - 位置层：按逆质量比例修正当前位置误差（`stiffness` 控制柔性）。
//!
//! 0.3 限制：
//! - 锚点偏移不随刚体旋转（简化版）；
//! - 只有距离约束，无固定关节/铰链/角度限制/弹簧、无完整约束矩阵求解；
//! - 关节求解在接触求解之后执行，极端堆叠场景可能需要更多迭代。

use crate::world::PhysicsWorld;
use hezhou_core::math::Vec3;

/// 距离关节：保持两刚体锚点间距离。
#[derive(Clone, Debug)]
pub struct DistanceJoint {
    pub body_a: u64,
    pub body_b: u64,
    /// A 的世界锚点相对 `body_a` 位置的偏移。
    pub anchor_a: Vec3,
    /// B 的世界锚点相对 `body_b` 位置的偏移。
    pub anchor_b: Vec3,
    /// 期望距离（米）。
    pub rest_length: f32,
    /// 位置修正比例 `0..=1`（1 近似刚性）。
    pub stiffness: f32,
}

impl DistanceJoint {
    pub fn new(body_a: u64, body_b: u64, rest_length: f32) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a: Vec3::zero(),
            anchor_b: Vec3::zero(),
            rest_length: rest_length.max(0.0),
            stiffness: 1.0,
        }
    }

    pub fn with_anchors(mut self, anchor_a: Vec3, anchor_b: Vec3) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }

    pub fn with_stiffness(mut self, stiffness: f32) -> Self {
        self.stiffness = stiffness.clamp(0.0, 1.0);
        self
    }

    /// 当前两锚点的世界距离；刚体不存在时返回 `None`。
    pub fn current_length(&self, world: &PhysicsWorld) -> Option<f32> {
        let a = world.body_position(self.body_a)? + self.anchor_a;
        let b = world.body_position(self.body_b)? + self.anchor_b;
        Some((b - a).magnitude())
    }
}

impl PhysicsWorld {
    /// 添加距离关节，返回关节下标（可用 [`PhysicsWorld::distance_joint_mut`] 修改）。
    pub fn add_distance_joint(&mut self, body_a: u64, body_b: u64, rest_length: f32) -> usize {
        self.joints
            .push(DistanceJoint::new(body_a, body_b, rest_length));
        self.joints.len() - 1
    }

    /// 追加一个已配置好的距离关节。
    pub fn push_distance_joint(&mut self, joint: DistanceJoint) -> usize {
        self.joints.push(joint);
        self.joints.len() - 1
    }

    pub fn distance_joints(&self) -> &[DistanceJoint] {
        &self.joints
    }

    pub fn distance_joint_mut(&mut self, index: usize) -> Option<&mut DistanceJoint> {
        self.joints.get_mut(index)
    }

    pub fn remove_distance_joint(&mut self, index: usize) -> Option<DistanceJoint> {
        if index < self.joints.len() {
            Some(self.joints.remove(index))
        } else {
            None
        }
    }

    pub fn clear_joints(&mut self) {
        self.joints.clear();
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// 距离关节求解：速度层 + 位置层。
    pub(crate) fn solve_joints(&mut self, _dt: f32) {
        if self.joints.is_empty() {
            return;
        }
        let iterations = self.solver_config.velocity_iterations.max(1);
        for _ in 0..iterations {
            for index in 0..self.joints.len() {
                let (body_a, body_b, anchor_a, anchor_b, rest_length, stiffness) = {
                    let joint = &self.joints[index];
                    (
                        joint.body_a,
                        joint.body_b,
                        joint.anchor_a,
                        joint.anchor_b,
                        joint.rest_length,
                        joint.stiffness,
                    )
                };
                let (Some(position_a), Some(position_b)) =
                    (self.body_position(body_a), self.body_position(body_b))
                else {
                    continue;
                };
                // 关节会把睡着的刚体拉醒（避免关节失效）。
                if let Some(body) = self.bodies.get_mut(&body_a) {
                    if body.is_sleeping() {
                        body.wake_up();
                    }
                }
                if let Some(body) = self.bodies.get_mut(&body_b) {
                    if body.is_sleeping() {
                        body.wake_up();
                    }
                }

                let inv_a = self.solver_inverse_mass(body_a);
                let inv_b = self.solver_inverse_mass(body_b);
                let inv_sum = inv_a + inv_b;
                if inv_sum <= 0.0 {
                    continue;
                }

                let world_a = position_a + anchor_a;
                let world_b = position_b + anchor_b;
                let delta = world_b - world_a;
                let distance = delta.magnitude();
                if distance <= 1e-6 {
                    continue;
                }
                let normal = delta * (1.0 / distance);
                let error = distance - rest_length;

                // 位置修正：把两端拉近/推远。
                if error.abs() > 1e-5 {
                    let correction = error * stiffness.clamp(0.0, 1.0);
                    if inv_a > 0.0 {
                        if let Some(body) = self.bodies.get_mut(&body_a) {
                            body.set_position(
                                *body.position() + normal * (correction * inv_a / inv_sum),
                            );
                        }
                    }
                    if inv_b > 0.0 {
                        if let Some(body) = self.bodies.get_mut(&body_b) {
                            body.set_position(
                                *body.position() - normal * (correction * inv_b / inv_sum),
                            );
                        }
                    }
                }

                // 速度修正：消除沿约束轴的相对速度。
                let (Some(body_ref_a), Some(body_ref_b)) =
                    (self.bodies.get(&body_a), self.bodies.get(&body_b))
                else {
                    continue;
                };
                let along = Vec3::dot(&(*body_ref_b.velocity() - *body_ref_a.velocity()), &normal);
                let impulse = normal * (-along / inv_sum);
                if inv_a > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&body_a) {
                        body.set_velocity(*body.velocity() - impulse * inv_a);
                    }
                }
                if inv_b > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&body_b) {
                        body.set_velocity(*body.velocity() + impulse * inv_b);
                    }
                }
            }
        }
    }
}
