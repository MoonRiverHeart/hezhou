use crate::collider::{layers_interact, Collider};
use crate::fixed::FixedTimestep;
use crate::intersect::aabb_overlap;
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

/// 刚体宽相阶段缓存。
struct BodyBroadphase {
    body_id: u64,
    collider_id: u64,
    min: Vec3,
    max: Vec3,
    is_sensor: bool,
    layer: u32,
    mask: u32,
    is_dynamic: bool,
}

/// 物理世界：刚体 + 碰撞体 + 层/掩码 + 触发器 + 查询接口。
///
/// 0.2 约定：
/// - `Box` 为轴对齐盒，忽略刚体旋转（`Plane` 例外，其法线来自刚体旋转）；
/// - `Capsule` 轴固定为世界 Y 轴；
/// - 一个刚体最多挂一个碰撞体（`body_collider_map`）。
///
/// TODO(0.3)：任意朝向盒/胶囊、关节、CCD、布娃娃（见 `future` 模块）。
pub struct PhysicsWorld {
    bodies: HashMap<u64, RigidBody>,
    colliders: HashMap<u64, Collider>,
    body_collider_map: HashMap<u64, u64>,
    gravity: Vec3,
    time_step: f32,
    next_id: u64,
    collision_events: Vec<CollisionEvent>,
    active_collisions: HashSet<(u64, u64)>,
    trigger_events: Vec<TriggerEvent>,
    active_triggers: HashSet<(u64, u64)>,
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
    pub fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        for body in self.bodies.values_mut() {
            if body.body_type() == RigidBodyType::Dynamic && !body.is_sleeping() {
                let velocity = *body.velocity() + self.gravity * body.gravity_scale() * dt;
                body.set_velocity(velocity);

                let position = *body.position() + *body.velocity() * dt;
                body.set_position(position);
            }
        }
        self.detect_and_resolve_collisions();
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

    /// 收集参与碰撞的碰撞体宽相数据（`Plane` 暂不参与动态刚体分离）。
    fn collect_broadphase(&self) -> Vec<BodyBroadphase> {
        let mut out = Vec::new();
        for (&body_id, body) in &self.bodies {
            let Some(&collider_id) = self.body_collider_map.get(&body_id) else {
                continue;
            };
            let Some(collider) = self.colliders.get(&collider_id) else {
                continue;
            };
            let Some(half) = collider.shape().aabb_half_extents() else {
                // TODO(0.3)：无限平面参与动态刚体分离。
                continue;
            };
            let center = *body.position() + *collider.offset();
            out.push(BodyBroadphase {
                body_id,
                collider_id,
                min: center - half,
                max: center + half,
                is_sensor: collider.is_sensor(),
                layer: collider.layer(),
                mask: collider.mask(),
                is_dynamic: body.is_dynamic(),
            });
        }
        out
    }

    fn detect_and_resolve_collisions(&mut self) {
        self.collision_events.clear();
        self.trigger_events.clear();

        let entries = self.collect_broadphase();
        let mut current_solid: HashSet<(u64, u64)> = HashSet::new();
        let mut current_triggers: HashSet<(u64, u64)> = HashSet::new();

        for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                let a = &entries[i];
                let b = &entries[j];
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
                self.resolve_overlap(a, b);
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
    }

    fn resolve_overlap(&mut self, a: &BodyBroadphase, b: &BodyBroadphase) {
        let overlap = [
            (a.max.x - b.min.x).min(b.max.x - a.min.x),
            (a.max.y - b.min.y).min(b.max.y - a.min.y),
            (a.max.z - b.min.z).min(b.max.z - a.min.z),
        ];
        if overlap[0] <= 0.0 || overlap[1] <= 0.0 || overlap[2] <= 0.0 {
            return;
        }
        let (axis, depth) = if overlap[0] <= overlap[1] && overlap[0] <= overlap[2] {
            (0usize, overlap[0])
        } else if overlap[1] <= overlap[2] {
            (1usize, overlap[1])
        } else {
            (2usize, overlap[2])
        };
        let a_dynamic = a.is_dynamic;
        let b_dynamic = b.is_dynamic;
        if !a_dynamic && !b_dynamic {
            return;
        }
        let sign = if component(a.min, axis) < component(b.min, axis) {
            -1.0
        } else {
            1.0
        };
        let move_a = if a_dynamic && b_dynamic {
            depth * 0.5
        } else if a_dynamic {
            depth
        } else {
            0.0
        };
        let move_b = if a_dynamic && b_dynamic {
            depth * 0.5
        } else if b_dynamic {
            depth
        } else {
            0.0
        };
        if move_a > 0.0 {
            if let Some(body) = self.bodies.get_mut(&a.body_id) {
                let mut p = *body.position();
                add_axis(&mut p, axis, sign * move_a);
                body.set_position(p);
                let mut v = *body.velocity();
                mul_axis(&mut v, axis, -0.2);
                body.set_velocity(v);
            }
        }
        if move_b > 0.0 {
            if let Some(body) = self.bodies.get_mut(&b.body_id) {
                let mut p = *body.position();
                add_axis(&mut p, axis, -sign * move_b);
                body.set_position(p);
                // 仅 y 轴做垂直阻尼，保持原有手感。
                if axis == 1 {
                    let mut v = *body.velocity();
                    v.y *= -0.2;
                    body.set_velocity(v);
                }
            }
        }
    }
}

fn component(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn add_axis(v: &mut Vec3, axis: usize, delta: f32) {
    match axis {
        0 => v.x += delta,
        1 => v.y += delta,
        _ => v.z += delta,
    }
}

fn mul_axis(v: &mut Vec3, axis: usize, factor: f32) {
    match axis {
        0 => v.x *= factor,
        1 => v.y *= factor,
        _ => v.z *= factor,
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}
