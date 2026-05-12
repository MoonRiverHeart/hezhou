use crate::{Collider, RigidBody};
use hezhou_core::math::Vec3;
use std::collections::HashMap;

pub struct PhysicsWorld {
    bodies: HashMap<u64, RigidBody>,
    colliders: HashMap<u64, Collider>,
    body_collider_map: HashMap<u64, u64>,
    gravity: Vec3,
    time_step: f32,
    next_id: u64,
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
        let body_id_to_remove = self.body_collider_map.iter()
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
        for (id, body) in self.bodies.iter_mut() {
            if body.body_type() == crate::RigidBodyType::Dynamic && !body.is_sleeping() {
                let velocity = *body.velocity() + self.gravity * body.gravity_scale() * dt;
                body.set_velocity(velocity);
                
                let position = *body.position() + *body.velocity() * dt;
                body.set_position(position);
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