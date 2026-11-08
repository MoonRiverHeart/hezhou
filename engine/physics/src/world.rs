use crate::collider::{layers_interact, Collider};
use crate::dynamics::{Contact, SolverConfig};
use crate::fixed::FixedTimestep;
use crate::intersect::aabb_overlap;
use crate::joint::DistanceJoint;
use crate::rigid_body::{RigidBody, RigidBodyType};
use hezhou_core::math::Vec3;
use std::collections::{HashMap, HashSet};

/// 两个非传感器刚体开始接触的事件（与上一帧比较去重）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionEvent {
    pub body_a: u64,
    pub body_b: u64,
}

/// 触发器事件类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerEventKind {
    /// 进入触发器。
    Enter,
    /// 离开触发器。
    Exit,
}

/// 触发器（sensor）进入/离开事件。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerEvent {
    pub kind: TriggerEventKind,
    /// 触发器自身的刚体 id。
    pub trigger_body: u64,
    /// 触发器碰撞体 id。
    pub trigger_collider: u64,
    /// 与触发器重叠的另一个刚体 id。
    pub other_body: u64,
    pub other_collider: u64,
}

/// 物理世界内部：碰撞体在世界空间的可查询视图。
pub(crate) struct ColliderEntry<'a> {
    pub body_id: u64,
    pub collider_id: u64,
    pub body: &'a RigidBody,
    pub collider: &'a Collider,
    /// 碰撞体世界位置（刚体位置 + 局部偏移）。
    pub position: Vec3,
    /// `Plane` 碰撞体的世界法线（刚体局部 +Y 旋转到世界空间）。
    pub plane_normal: Vec3,
}

/// 刚体宽相阶段缓存（轴对齐包围盒近似）。
struct BodyBroadphase {
    body_id: u64,
    collider_id: u64,
    min: Vec3,
    max: Vec3,
    is_sensor: bool,
    layer: u32,
    mask: u32,
    inv_mass: f32,
    restitution: f32,
    friction: f32,
}

/// 无限平面宽相缓存（`Plane` 碰撞体视为静态碰撞面）。
struct PlaneBroadphase {
    body_id: u64,
    point: Vec3,
    normal: Vec3,
    layer: u32,
    mask: u32,
    restitution: f32,
    friction: f32,
}

/// 物理世界：刚体 + 碰撞体 + 层/掩码 + 触发器 + 动态求解 + 查询接口。
///
/// 0.3 第一批能力：
/// - 动态刚体：重力/线速度积分、质量/逆质量、线性/角阻尼；
/// - 动态 vs 静态/动态接触：AABB/球/胶囊的包围盒接触、冲量 + 摩擦 + 位置修正；
/// - CCD 基础（球/胶囊 vs 静态，可配置）；
/// - 距离关节（可配置刚度）；
/// - 可选自动睡眠。
///
/// 约定与限制（详见 crate README）：
/// - `Box` 为轴对齐盒，碰撞检测用 AABB 近似（忽略刚体旋转）；
/// - `Capsule` 轴固定世界 Y 轴；
/// - 接触响应只含线性冲量，无角冲量/惯性张量（旋转碰撞响应后续版本）；
/// - 一个刚体最多挂一个碰撞体。
pub struct PhysicsWorld {
    pub(crate) bodies: HashMap<u64, RigidBody>,
    pub(crate) colliders: HashMap<u64, Collider>,
    pub(crate) body_collider_map: HashMap<u64, u64>,
    pub(crate) gravity: Vec3,
    pub(crate) time_step: f32,
    pub(crate) next_id: u64,
    pub(crate) collision_events: Vec<CollisionEvent>,
    pub(crate) active_collisions: HashSet<(u64, u64)>,
    pub(crate) trigger_events: Vec<TriggerEvent>,
    pub(crate) active_triggers: HashSet<(u64, u64)>,
    pub(crate) joints: Vec<DistanceJoint>,
    pub(crate) solver_config: SolverConfig,
    pub(crate) ccd_enabled: bool,
    pub(crate) auto_sleep: bool,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            bodies: HashMap::new(),
            colliders: HashMap::new(),
            body_collider_map: HashMap::new(),
            gravity: Vec3::new(0.0, -9.81, 0.0),
            time_step: 1.0 / 60.0,
            next_id: 1,
            collision_events: Vec::new(),
            active_collisions: HashSet::new(),
            trigger_events: Vec::new(),
            active_triggers: HashSet::new(),
            joints: Vec::new(),
            solver_config: SolverConfig::default(),
            ccd_enabled: false,
            auto_sleep: false,
        }
    }

    pub fn gravity(&self) -> &Vec3 {
        &self.gravity
    }

    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = gravity;
    }

    pub fn time_step(&self) -> f32 {
        self.time_step
    }

    pub fn set_time_step(&mut self, time_step: f32) {
        if time_step.is_finite() && time_step > 0.0 {
            self.time_step = time_step;
        }
    }

    /// 求解器配置（迭代次数/位置修正比例/恢复阈值等）。
    pub fn solver_config(&self) -> &SolverConfig {
        &self.solver_config
    }

    pub fn set_solver_config(&mut self, config: SolverConfig) {
        self.solver_config = config;
    }

    /// CCD 全局开关（默认关闭；刚体还需要 `set_ccd_enabled(true)`）。
    pub fn ccd_enabled(&self) -> bool {
        self.ccd_enabled
    }

    pub fn set_ccd_enabled(&mut self, enabled: bool) {
        self.ccd_enabled = enabled;
    }

    /// 自动睡眠开关（默认关闭，避免影响 0.2 既有行为）。
    pub fn auto_sleep(&self) -> bool {
        self.auto_sleep
    }

    pub fn set_auto_sleep(&mut self, enabled: bool) {
        self.auto_sleep = enabled;
    }

    pub fn create_body(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.insert(id, RigidBody::new());
        id
    }

    pub fn create_body_with_type(&mut self, body_type: RigidBodyType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut body = RigidBody::new();
        body.set_body_type(body_type);
        self.bodies.insert(id, body);
        id
    }

    pub fn destroy_body(&mut self, id: u64) {
        if let Some(collider_id) = self.body_collider_map.remove(&id) {
            self.colliders.remove(&collider_id);
        }
        self.bodies.remove(&id);
        self.joints
            .retain(|joint| joint.body_a != id && joint.body_b != id);
        // 清理事件跟踪状态，避免删除后产生残留 Exit 事件。
        self.active_collisions.retain(|(a, b)| *a != id && *b != id);
        self.active_triggers.retain(|(a, b)| *a != id && *b != id);
    }

    pub fn get_body(&self, id: u64) -> Option<&RigidBody> {
        self.bodies.get(&id)
    }

    pub fn get_body_mut(&mut self, id: u64) -> Option<&mut RigidBody> {
        self.bodies.get_mut(&id)
    }

    /// 便捷读取刚体世界位置。
    pub fn body_position(&self, id: u64) -> Option<Vec3> {
        self.bodies.get(&id).map(|body| *body.position())
    }

    pub fn create_collider(&mut self, shape: crate::ColliderShape, body_id: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.colliders.insert(id, Collider::new(shape));
        self.body_collider_map.insert(body_id, id);
        id
    }

    pub fn destroy_collider(&mut self, id: u64) {
        self.colliders.remove(&id);
        let body_id_to_remove = self
            .body_collider_map
            .iter()
            .find(|(_, collider_id)| **collider_id == id)
            .map(|(body_id, _)| *body_id);
        if let Some(body_id) = body_id_to_remove {
            self.body_collider_map.remove(&body_id);
        }
    }

    pub fn get_collider(&self, id: u64) -> Option<&Collider> {
        self.colliders.get(&id)
    }

    pub fn get_collider_mut(&mut self, id: u64) -> Option<&mut Collider> {
        self.colliders.get_mut(&id)
    }

    /// 遍历所有 (刚体, 碰撞体) 组合并给出世界空间查询视图。
    pub(crate) fn each_collider(&self, mut f: impl FnMut(ColliderEntry<'_>)) {
        for (body_id, body) in &self.bodies {
            let Some(collider_id) = self.body_collider_map.get(body_id) else {
                continue;
            };
            let Some(collider) = self.colliders.get(collider_id) else {
                continue;
            };
            let position = *body.position() + *collider.offset();
            let plane_normal = body.rotation_up();
            f(ColliderEntry {
                body_id: *body_id,
                collider_id: *collider_id,
                body,
                collider,
                position,
                plane_normal,
            });
        }
    }

    /// 固定步长推进一步。
    ///
    /// 流水线：速度积分 → 位置积分（含 CCD）→ 接触检测/事件 → 速度冲量迭代
    /// → 位置修正迭代 → 关节求解 → 睡眠更新。
    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.integrate_velocities(dt);
        self.integrate_positions(dt);

        let contacts = self.detect_contacts_and_events();
        self.wake_bodies_on_contacts(&contacts);
        self.solve_contact_velocities(&contacts);

        let position_iterations = self.solver_config.position_iterations;
        for _ in 0..position_iterations {
            let current = self.detect_contacts();
            if current.is_empty() {
                break;
            }
            if !self.correct_positions_once(&current) {
                break;
            }
        }

        self.solve_joints(dt);
        self.update_sleeping(dt);
    }

    /// 用 `FixedTimestep` 驱动：累加真实帧时间并执行相应次数的固定步。
    /// 返回本次执行的固定步数（供上层做渲染插值）。
    pub fn advance_fixed(&mut self, timestep: &mut FixedTimestep, frame_dt: f32) -> u32 {
        let steps = timestep.advance(frame_dt);
        for _ in 0..steps {
            self.step(timestep.step());
        }
        steps
    }

    /// 用世界自身 `time_step` 推进一步。
    pub fn step_fixed(&mut self) {
        self.step(self.time_step);
    }

    /// 当前帧新产生的碰撞对（非传感器，与上一帧比较去重）。
    pub fn collision_events(&self) -> &[CollisionEvent] {
        &self.collision_events
    }

    /// 当前帧的触发器进入/离开事件。
    pub fn trigger_events(&self) -> &[TriggerEvent] {
        &self.trigger_events
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }

    /// 收集宽相数据：普通碰撞体（AABB）+ 无限平面。
    fn collect_broadphase(&self) -> (Vec<BodyBroadphase>, Vec<PlaneBroadphase>) {
        let mut bodies = Vec::new();
        let mut planes = Vec::new();
        for (&body_id, body) in &self.bodies {
            let Some(&collider_id) = self.body_collider_map.get(&body_id) else {
                continue;
            };
            let Some(collider) = self.colliders.get(&collider_id) else {
                continue;
            };
            let center = *body.position() + *collider.offset();
            let inv_mass = if body.is_movable() {
                body.inverse_mass()
            } else {
                0.0
            };
            match collider.shape() {
                crate::ColliderShape::Plane => planes.push(PlaneBroadphase {
                    body_id,
                    point: center,
                    normal: body.rotation_up(),
                    layer: collider.layer(),
                    mask: collider.mask(),
                    restitution: collider.restitution(),
                    friction: collider.friction(),
                }),
                _ => {
                    let Some(half) = collider.shape().aabb_half_extents() else {
                        continue;
                    };
                    bodies.push(BodyBroadphase {
                        body_id,
                        collider_id,
                        min: center - half,
                        max: center + half,
                        is_sensor: collider.is_sensor(),
                        layer: collider.layer(),
                        mask: collider.mask(),
                        inv_mass,
                        restitution: collider.restitution(),
                        friction: collider.friction(),
                    });
                }
            }
        }
        (bodies, planes)
    }

    /// 只做纯几何接触检测（供位置修正迭代复用，不修改事件状态）。
    fn detect_contacts(&self) -> Vec<Contact> {
        let (bodies, planes) = self.collect_broadphase();
        let mut out = Vec::new();

        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let a = &bodies[i];
                let b = &bodies[j];
                if a.is_sensor || b.is_sensor {
                    continue;
                }
                if !layers_interact(a.layer, a.mask, b.layer, b.mask) {
                    continue;
                }
                // 双方都不可推动时不需要求解接触（静态-静态留作事件即可）。
                if a.inv_mass <= 0.0 && b.inv_mass <= 0.0 {
                    continue;
                }
                if let Some(contact) = contact_from_aabb(a, b) {
                    out.push(contact);
                }
            }
        }

        for body in &bodies {
            if body.is_sensor || body.inv_mass <= 0.0 {
                continue;
            }
            for plane in &planes {
                if !layers_interact(body.layer, body.mask, plane.layer, plane.mask) {
                    continue;
                }
                if let Some(contact) = contact_from_plane(body, plane) {
                    out.push(contact);
                }
            }
        }

        out
    }

    /// 接触检测 + 碰撞/触发器事件更新，返回本步接触列表。
    fn detect_contacts_and_events(&mut self) -> Vec<Contact> {
        self.collision_events.clear();
        self.trigger_events.clear();

        let (bodies, planes) = self.collect_broadphase();
        let mut current_solid: HashSet<(u64, u64)> = HashSet::new();
        let mut current_triggers: HashSet<(u64, u64)> = HashSet::new();

        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let a = &bodies[i];
                let b = &bodies[j];
                if !aabb_overlap(a.min, a.max, b.min, b.max) {
                    continue;
                }
                if !layers_interact(a.layer, a.mask, b.layer, b.mask) {
                    continue;
                }
                let pair = (a.body_id.min(b.body_id), a.body_id.max(b.body_id));

                if a.is_sensor || b.is_sensor {
                    // 触发器只报告重叠，不参与位置分离。
                    current_triggers.insert(pair);
                    if !self.active_triggers.contains(&pair) {
                        let (trigger, other) = if a.is_sensor { (a, b) } else { (b, a) };
                        self.trigger_events.push(TriggerEvent {
                            kind: TriggerEventKind::Enter,
                            trigger_body: trigger.body_id,
                            trigger_collider: trigger.collider_id,
                            other_body: other.body_id,
                            other_collider: other.collider_id,
                        });
                    }
                    continue;
                }

                current_solid.insert(pair);
                if !self.active_collisions.contains(&pair) {
                    self.collision_events.push(CollisionEvent {
                        body_a: pair.0,
                        body_b: pair.1,
                    });
                }
            }
        }

        // 动态刚体 vs 无限平面：接触 + 碰撞事件。
        for body in &bodies {
            if body.is_sensor || body.inv_mass <= 0.0 {
                continue;
            }
            for plane in &planes {
                if !layers_interact(body.layer, body.mask, plane.layer, plane.mask) {
                    continue;
                }
                let Some(_contact) = contact_from_plane(body, plane) else {
                    continue;
                };
                let pair = (
                    body.body_id.min(plane.body_id),
                    body.body_id.max(plane.body_id),
                );
                current_solid.insert(pair);
                if !self.active_collisions.contains(&pair) {
                    self.collision_events.push(CollisionEvent {
                        body_a: pair.0,
                        body_b: pair.1,
                    });
                }
            }
        }

        // 触发器离开事件：上一帧在、当前帧不在。
        let previous_triggers: Vec<(u64, u64)> = self.active_triggers.iter().copied().collect();
        for pair in previous_triggers {
            if current_triggers.contains(&pair) {
                continue;
            }
            let collider_of =
                |body: u64| -> Option<u64> { self.body_collider_map.get(&body).copied() };
            let (Some(ca), Some(cb)) = (collider_of(pair.0), collider_of(pair.1)) else {
                continue;
            };
            let a_is_sensor = self
                .colliders
                .get(&ca)
                .map(|c| c.is_sensor())
                .unwrap_or(false);
            let (trigger_body, trigger_collider, other_body, other_collider) = if a_is_sensor {
                (pair.0, ca, pair.1, cb)
            } else {
                (pair.1, cb, pair.0, ca)
            };
            self.trigger_events.push(TriggerEvent {
                kind: TriggerEventKind::Exit,
                trigger_body,
                trigger_collider,
                other_body,
                other_collider,
            });
        }

        self.active_collisions = current_solid;
        self.active_triggers = current_triggers;

        self.detect_contacts()
    }
}

/// AABB 接触：沿最小穿透轴生成法线（从 A 指向 B）。
fn contact_from_aabb(a: &BodyBroadphase, b: &BodyBroadphase) -> Option<Contact> {
    if !aabb_overlap(a.min, a.max, b.min, b.max) {
        return None;
    }
    let overlap = [
        (a.max.x - b.min.x).min(b.max.x - a.min.x),
        (a.max.y - b.min.y).min(b.max.y - a.min.y),
        (a.max.z - b.min.z).min(b.max.z - a.min.z),
    ];
    if overlap[0] <= 0.0 || overlap[1] <= 0.0 || overlap[2] <= 0.0 {
        return None;
    }
    let (axis, depth) = if overlap[0] <= overlap[1] && overlap[0] <= overlap[2] {
        (0usize, overlap[0])
    } else if overlap[1] <= overlap[2] {
        (1usize, overlap[1])
    } else {
        (2usize, overlap[2])
    };
    let sign = if component(a.min, axis) < component(b.min, axis) {
        1.0
    } else {
        -1.0
    };
    let normal = axis_vec(axis) * sign;
    let overlap_min = Vec3::new(
        a.min.x.max(b.min.x),
        a.min.y.max(b.min.y),
        a.min.z.max(b.min.z),
    );
    let overlap_max = Vec3::new(
        a.max.x.min(b.max.x),
        a.max.y.min(b.max.y),
        a.max.z.min(b.max.z),
    );
    let point = (overlap_min + overlap_max) * 0.5;
    Some(Contact {
        body_a: a.body_id,
        body_b: b.body_id,
        normal,
        penetration: depth,
        restitution: a.restitution.max(b.restitution),
        friction: (a.friction + b.friction) * 0.5,
        point,
    })
}

/// 动态刚体 AABB vs 无限平面接触（平面为 A，刚体为 B）。
fn contact_from_plane(body: &BodyBroadphase, plane: &PlaneBroadphase) -> Option<Contact> {
    let center = (body.min + body.max) * 0.5;
    let half = (body.max - body.min) * 0.5;
    let n = plane.normal.normalized();
    let sd = Vec3::dot(&(center - plane.point), &n);
    let extent = n.x.abs() * half.x + n.y.abs() * half.y + n.z.abs() * half.z;
    let depth = extent - sd.abs();
    if depth <= 0.0 {
        return None;
    }
    let normal = if sd >= 0.0 { n } else { -n };
    let point = center - normal * extent;
    Some(Contact {
        body_a: plane.body_id,
        body_b: body.body_id,
        normal,
        penetration: depth,
        restitution: plane.restitution.max(body.restitution),
        friction: (plane.friction + body.friction) * 0.5,
        point,
    })
}

fn component(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn axis_vec(axis: usize) -> Vec3 {
    match axis {
        0 => Vec3::new(1.0, 0.0, 0.0),
        1 => Vec3::new(0.0, 1.0, 0.0),
        _ => Vec3::new(0.0, 0.0, 1.0),
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}
