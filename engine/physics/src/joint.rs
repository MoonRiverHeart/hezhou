//! 关节（0.3 第一版）：距离关节 + 固定关节。
//!
//! - [`DistanceJoint`]：约束两刚体锚点间距离为 `rest_length`；
//! - [`FixedJoint`]：约束两刚体锚点的相对位置（锁定相对平移），表现为“焊接/父子跟随”；
//!
//! 两者都采用速度层（消除相对速度）+ 位置层（按逆质量比例修正误差）迭代求解，
//! `stiffness` 控制柔性。
//!
//! 0.3 限制：
//! - 锚点偏移不随刚体旋转（简化版）；固定关节不约束相对旋转（无角冲量）；
//! - 无铰链/弹簧/角度限制/完整约束矩阵求解；
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

/// 固定关节：锁定两刚体锚点的相对位置（类似“焊死/父子跟随”）。
///
/// 约束形式：`(pos_b + anchor_b) - (pos_a + anchor_a) = target_offset`。
/// 0.3 只约束平移，不约束相对旋转（无角冲量/惯性张量）。
#[derive(Clone, Debug)]
pub struct FixedJoint {
    pub body_a: u64,
    pub body_b: u64,
    /// A 的世界锚点相对 `body_a` 位置的偏移。
    pub anchor_a: Vec3,
    /// B 的世界锚点相对 `body_b` 位置的偏移。
    pub anchor_b: Vec3,
    /// 期望的相对位置差 `(pos_b + anchor_b) - (pos_a + anchor_a)`。
    pub target_offset: Vec3,
    /// 位置修正比例 `0..=1`（1 近似刚性）。
    pub stiffness: f32,
}

impl FixedJoint {
    pub fn new(body_a: u64, body_b: u64, target_offset: Vec3) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a: Vec3::zero(),
            anchor_b: Vec3::zero(),
            target_offset,
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

    /// 当前两锚点的相对位置差；刚体不存在时返回 `None`。
    pub fn current_offset(&self, world: &PhysicsWorld) -> Option<Vec3> {
        let a = world.body_position(self.body_a)? + self.anchor_a;
        let b = world.body_position(self.body_b)? + self.anchor_b;
        Some(b - a)
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

    /// 添加固定关节：以当前两刚体位置差作为期望相对位置。
    ///
    /// 返回关节下标；刚体不存在时 `target_offset` 记为 0（求解时自然跳过）。
    pub fn add_fixed_joint(&mut self, body_a: u64, body_b: u64) -> usize {
        let offset = match (self.body_position(body_a), self.body_position(body_b)) {
            (Some(a), Some(b)) => b - a,
            _ => Vec3::zero(),
        };
        self.add_fixed_joint_with_offset(body_a, body_b, offset)
    }

    /// 添加固定关节并显式指定期望相对位置差。
    pub fn add_fixed_joint_with_offset(
        &mut self,
        body_a: u64,
        body_b: u64,
        target_offset: Vec3,
    ) -> usize {
        self.fixed_joints
            .push(FixedJoint::new(body_a, body_b, target_offset));
        self.fixed_joints.len() - 1
    }

    /// 追加一个已配置好的固定关节。
    pub fn push_fixed_joint(&mut self, joint: FixedJoint) -> usize {
        self.fixed_joints.push(joint);
        self.fixed_joints.len() - 1
    }

    pub fn fixed_joints(&self) -> &[FixedJoint] {
        &self.fixed_joints
    }

    pub fn fixed_joint_mut(&mut self, index: usize) -> Option<&mut FixedJoint> {
        self.fixed_joints.get_mut(index)
    }

    pub fn remove_fixed_joint(&mut self, index: usize) -> Option<FixedJoint> {
        if index < self.fixed_joints.len() {
            Some(self.fixed_joints.remove(index))
        } else {
            None
        }
    }

    pub fn fixed_joint_count(&self) -> usize {
        self.fixed_joints.len()
    }

    pub fn distance_joint_count(&self) -> usize {
        self.joints.len()
    }

    /// 清空所有关节（距离关节 + 固定关节）。
    pub fn clear_joints(&mut self) {
        self.joints.clear();
        self.fixed_joints.clear();
    }

    /// 关节总数（距离关节 + 固定关节）。
    pub fn joint_count(&self) -> usize {
        self.joints.len() + self.fixed_joints.len()
    }

    /// 关节求解：速度层 + 位置层（距离关节 + 固定关节）。
    pub(crate) fn solve_joints(&mut self, _dt: f32) {
        if self.joints.is_empty() && self.fixed_joints.is_empty() {
            return;
        }
        let iterations = self.solver_config.velocity_iterations.max(1);
        for _ in 0..iterations {
            self.solve_distance_joints_once();
            self.solve_fixed_joints_once();
        }
    }

    fn solve_distance_joints_once(&mut self) {
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

    /// 固定关节求解：锁定两锚点相对平移（速度层 + 位置层）。
    fn solve_fixed_joints_once(&mut self) {
        for index in 0..self.fixed_joints.len() {
            let (body_a, body_b, anchor_a, anchor_b, target_offset, stiffness) = {
                let joint = &self.fixed_joints[index];
                (
                    joint.body_a,
                    joint.body_b,
                    joint.anchor_a,
                    joint.anchor_b,
                    joint.target_offset,
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

            // 位置误差：C = (pos_b + anchor_b) - (pos_a + anchor_a) - target_offset。
            let error = (position_b + anchor_b) - (position_a + anchor_a) - target_offset;
            if error.magnitude() > 1e-5 {
                let correction = error * stiffness.clamp(0.0, 1.0);
                if inv_a > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&body_a) {
                        body.set_position(*body.position() + correction * (inv_a / inv_sum));
                    }
                }
                if inv_b > 0.0 {
                    if let Some(body) = self.bodies.get_mut(&body_b) {
                        body.set_position(*body.position() - correction * (inv_b / inv_sum));
                    }
                }
            }

            // 速度修正：消除三轴相对速度（锁定相对平移）。
            let (Some(body_ref_a), Some(body_ref_b)) =
                (self.bodies.get(&body_a), self.bodies.get(&body_b))
            else {
                continue;
            };
            let relative_velocity = *body_ref_b.velocity() - *body_ref_a.velocity();
            let impulse = relative_velocity * (-1.0 / inv_sum);
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
