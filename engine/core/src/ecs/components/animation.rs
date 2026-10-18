use crate::ecs::Component;
use crate::math::{Quaternion, Vec3};
use crate::ecs::components::LocalTransform;

#[derive(Clone, Debug)]
pub struct AnimationKeyframe {
    pub time: f32,
    pub position: Vec3,
    pub rotation: Quaternion,
    pub scale: Vec3,
}

impl AnimationKeyframe {
    pub fn new(time: f32, position: Vec3, rotation: Quaternion, scale: Vec3) -> Self {
        Self { time, position, rotation, scale }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    pub keys: Vec<AnimationKeyframe>,
}

impl AnimationClip {
    pub fn new(name: &str, duration: f32) -> Self {
        Self { name: name.to_string(), duration, looping: true, keys: Vec::new() }
    }

    pub fn with_key(mut self, key: AnimationKeyframe) -> Self {
        self.keys.push(key);
        self.keys.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        self
    }

    pub fn sample(&self, time: f32) -> Option<LocalTransform> {
        if self.keys.is_empty() { return None; }
        let t = if self.duration > 0.0 {
            if self.looping { time.rem_euclid(self.duration) } else { time.min(self.duration) }
        } else { 0.0 };
        let mut a = &self.keys[0];
        let mut b = &self.keys[self.keys.len() - 1];
        for i in 0..self.keys.len() {
            if self.keys[i].time <= t { a = &self.keys[i]; }
            if self.keys[i].time >= t { b = &self.keys[i]; break; }
        }
        if a.time >= b.time { return Some(a.to_transform()); }
        let k = ((t - a.time) / (b.time - a.time)).clamp(0.0, 1.0);
        let mut out = LocalTransform::new();
        out.position = Vec3::new(
            a.position.x + (b.position.x - a.position.x) * k,
            a.position.y + (b.position.y - a.position.y) * k,
            a.position.z + (b.position.z - a.position.z) * k,
        );
        out.rotation = Quaternion::slerp(&a.rotation, &b.rotation, k);
        out.scale = Vec3::new(
            a.scale.x + (b.scale.x - a.scale.x) * k,
            a.scale.y + (b.scale.y - a.scale.y) * k,
            a.scale.z + (b.scale.z - a.scale.z) * k,
        );
        Some(out)
    }
}

impl AnimationKeyframe {
    fn to_transform(&self) -> LocalTransform {
        LocalTransform { position: self.position, rotation: self.rotation, scale: self.scale }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationPlayer {
    pub clip: AnimationClip,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
}

impl Component for AnimationPlayer {
    fn type_id() -> crate::ecs::ComponentTypeId { 301 }
    fn type_name() -> &'static str { "AnimationPlayer" }
}

impl AnimationPlayer {
    pub fn new(clip: AnimationClip) -> Self {
        Self { clip, time: 0.0, speed: 1.0, playing: true }
    }
}

/// 每帧推进动画时间并写入 LocalTransform。
pub struct AnimationSystem;

impl crate::ecs::System for AnimationSystem {
    fn name(&self) -> &'static str { "AnimationSystem" }
    fn update(&mut self, world: &mut crate::ecs::World, dt: f32) {
        let entities: Vec<crate::ecs::Entity> = world.all_entities()
            .into_iter()
            .filter(|e| world.get_component::<AnimationPlayer>(*e).is_some()
                && world.get_component::<LocalTransform>(*e).is_some())
            .collect();
        for entity in entities {
            let Some(mut player) = world.get_component::<AnimationPlayer>(entity) else { continue };
            if !player.playing { continue; }
            player.time += dt * player.speed;
            if let Some(transform) = player.clip.sample(player.time) {
                world.add_component(entity, transform);
            }
            world.add_component(entity, player);
        }
    }
    fn priority(&self) -> i32 { 50 }
}
