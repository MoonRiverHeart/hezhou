use crate::animation::*;
use crate::types::*;
use hezhou_dfx::{DfxSystem, dfx_debug, dfx_trace};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AnimationEngine {
    animations: HashMap<AnimationId, Animation>,
    running_animations: Vec<AnimationId>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl AnimationEngine {
    pub fn new(dfx: Arc<Mutex<DfxSystem>>) -> Self {
        Self {
            animations: HashMap::new(),
            running_animations: Vec::new(),
            dfx,
        }
    }

    pub fn create_animation(&mut self, animation: Animation) -> AnimationId {
        let id = animation.id;
        self.animations.insert(id, animation);
        id
    }

    pub fn start_animation(&mut self, id: AnimationId) {
        if let Some(anim) = self.animations.get_mut(&id) {
            anim.running = true;
            anim.paused = false;
            anim.elapsed_time = 0.0;

            if !self.running_animations.contains(&id) {
                self.running_animations.push(id);
            }

            dfx_debug!("Animation", "Animation started: id={}, duration={}s", id.id, anim.duration);
        }
    }

    pub fn pause_animation(&mut self, id: AnimationId) {
        if let Some(anim) = self.animations.get_mut(&id) {
            anim.paused = true;
            dfx_debug!("Animation", "Animation paused: id={}", id.id);
        }
    }

    pub fn cancel_animation(&mut self, id: AnimationId) {
        if let Some(anim) = self.animations.get_mut(&id) {
            anim.running = false;
            anim.paused = false;

            self.running_animations
                .retain(|running_id| *running_id != id);
            dfx_debug!("Animation", "Animation cancelled: id={}", id.id);
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        for id in &self.running_animations {
            let anim = self.animations.get_mut(id).unwrap();

            if anim.paused {
                continue;
            }

            anim.elapsed_time += delta_time;

            let current_value = anim.current_value();

            dfx_trace!("Animation", "Animation {} updated: progress={}, value={}", id.id, anim.elapsed_time / anim.duration, current_value);

            if anim.is_complete() {
                if anim.repeat_count > 1 {
                    anim.repeat_count -= 1;
                    anim.elapsed_time = 0.0;

                    if anim.auto_reverse {
                        let temp = anim.from_value;
                        anim.from_value = anim.to_value;
                        anim.to_value = temp;
                    }
                } else {
                    anim.running = false;
                    dfx_debug!("Animation", "Animation completed: id={}", id.id);
                }
            }
        }

        self.running_animations
            .retain(|id| self.animations.get(id).map(|a| a.running).unwrap_or(false));
    }

    pub fn get_animation_value(&self, id: AnimationId) -> Option<f32> {
        self.animations.get(&id).map(|anim| anim.current_value())
    }

    pub fn is_animation_running(&self, id: AnimationId) -> bool {
        self.animations
            .get(&id)
            .map(|anim| anim.running)
            .unwrap_or(false)
    }
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(DfxSystem::new())))
    }
}