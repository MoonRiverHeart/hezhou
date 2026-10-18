use crate::{Collider, ColliderShape, RigidBody};
use hezhou_core::math::Vec3;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionEvent {
    pub body_a: u64,
    pub body_b: u64,
}

pub struct PhysicsWorld {
    bodies: HashMap<u64, RigidBody>,
    colliders: HashMap<u64, Collider>,
    body_collider_map: HashMap<u64, u64>,
    gravity: Vec3,
    time_step: f32,
    next_id: u64,
    collision_events: Vec<CollisionEvent>,
    active_collisions: HashSet<(u64, u64)>,
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
        self.time_step = time_step;
    }

    pub fn create_body(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.insert(id, RigidBody::new());
        id
    }

    pub fn create_body_with_type(&mut self, body_type: crate::RigidBodyType) -> u64 {
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
    }

    pub fn get_body(&self, id: u64) -> Option<&RigidBody> {
        self.bodies.get(&id)
    }

    pub fn get_body_mut(&mut self, id: u64) -> Option<&mut RigidBody> {
        self.bodies.get_mut(&id)
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

    pub fn step(&mut self, dt: f32) {
        for body in self.bodies.values_mut() {
            if body.body_type() == crate::RigidBodyType::Dynamic && !body.is_sleeping() {
                let velocity = *body.velocity() + self.gravity * body.gravity_scale() * dt;
                body.set_velocity(velocity);

                let position = *body.position() + *body.velocity() * dt;
                body.set_position(position);
            }
        }
        self.detect_and_resolve_collisions();
    }

    /// 当前帧新产生的碰撞对（与上一帧比较）。
    pub fn collision_events(&self) -> &[CollisionEvent] {
        &self.collision_events
    }

    fn body_aabb(&self, body_id: u64) -> Option<([f32; 3], [f32; 3])> {
        let body = self.bodies.get(&body_id)?;
        let collider_id = *self.body_collider_map.get(&body_id)?;
        let collider = self.colliders.get(&collider_id)?;
        let p = *body.position();
        let o = *collider.offset();
        let c = [p.x + o.x, p.y + o.y, p.z + o.z];
        let (hx, hy, hz) = match collider.shape() {
            ColliderShape::Sphere { radius } => (*radius, *radius, *radius),
            ColliderShape::Box { half_extents } => (half_extents.x, half_extents.y, half_extents.z),
            ColliderShape::Cylinder { half_height, radius } => (*radius, *half_height, *radius),
            ColliderShape::Capsule { half_height, radius } => (*radius, half_height + radius, *radius),
            ColliderShape::Plane => return None,
        };
        Some((
            [c[0] - hx, c[1] - hy, c[2] - hz],
            [c[0] + hx, c[1] + hy, c[2] + hz],
        ))
    }

    fn detect_and_resolve_collisions(&mut self) {
        self.collision_events.clear();
        let body_ids: Vec<u64> = self.body_collider_map.keys().copied().collect();
        let mut current = HashSet::new();
        for i in 0..body_ids.len() {
            for j in (i + 1)..body_ids.len() {
                let a = body_ids[i];
                let b = body_ids[j];
                let (Some(ba), Some(bb)) = (self.body_aabb(a), self.body_aabb(b)) else { continue };
                let overlap = [
                    (ba.1[0] - bb.0[0]).min(bb.1[0] - ba.0[0]),
                    (ba.1[1] - bb.0[1]).min(bb.1[1] - ba.0[1]),
                    (ba.1[2] - bb.0[2]).min(bb.1[2] - ba.0[2]),
                ];
                if overlap[0] <= 0.0 || overlap[1] <= 0.0 || overlap[2] <= 0.0 {
                    continue;
                }
                let pair = (a.min(b), a.max(b));
                current.insert(pair);
                if !self.active_collisions.contains(&pair) {
                    self.collision_events.push(CollisionEvent { body_a: pair.0, body_b: pair.1 });
                }
                self.resolve_overlap(a, b, overlap, ba, bb);
            }
        }
        self.active_collisions = current;
    }

    fn resolve_overlap(&mut self, a: u64, b: u64, overlap: [f32; 3], ba: ([f32; 3], [f32; 3]), bb: ([f32; 3], [f32; 3])) {
        let (axis, depth) = if overlap[0] <= overlap[1] && overlap[0] <= overlap[2] {
            (0, overlap[0])
        } else if overlap[1] <= overlap[2] {
            (1, overlap[1])
        } else {
            (2, overlap[2])
        };
        let a_dynamic = self.bodies.get(&a).map(|x| x.body_type() == crate::RigidBodyType::Dynamic).unwrap_or(false);
        let b_dynamic = self.bodies.get(&b).map(|x| x.body_type() == crate::RigidBodyType::Dynamic).unwrap_or(false);
        if !a_dynamic && !b_dynamic { return; }
        let sign = if ba.0[axis] < bb.0[axis] { -1.0 } else { 1.0 };
        let move_a = if a_dynamic && b_dynamic { depth * 0.5 } else if a_dynamic { depth } else { 0.0 };
        let move_b = if a_dynamic && b_dynamic { depth * 0.5 } else if b_dynamic { depth } else { 0.0 };
        if move_a > 0.0 {
            if let Some(body) = self.bodies.get_mut(&a) {
                let mut p = *body.position();
                match axis { 0 => p.x += sign * move_a, 1 => p.y += sign * move_a, _ => p.z += sign * move_a }
                body.set_position(p);
                let mut v = *body.velocity();
                match axis { 0 => v.x *= -0.2, 1 => v.y *= -0.2, _ => v.z *= -0.2 }
                body.set_velocity(v);
            }
        }
        if move_b > 0.0 {
            if let Some(body) = self.bodies.get_mut(&b) {
                let mut p = *body.position();
                match axis { 0 => p.x -= sign * move_b, 1 => p.y -= sign * move_b, _ => p.z -= sign * move_b }
                body.set_position(p);
                let mut v = *body.velocity();
                match axis { 1 => v.y *= -0.2, _ => {} }
                body.set_velocity(v);
            }
        }
    }

    pub fn step_fixed(&mut self) {
        self.step(self.time_step);
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}
