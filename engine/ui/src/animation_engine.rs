use crate::animation::*;
use crate::types::*;
use hezhou_dfx::*;
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
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Info,
                "AnimationEngine",
                &format!("Animation started: id={}, duration={}s", id.id, anim.duration)
            );
        }
    }
    
    pub fn pause_animation(&mut self, id: AnimationId) {
        if let Some(anim) = self.animations.get_mut(&id) {
            anim.paused = true;
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Debug,
                "AnimationEngine",
                &format!("Animation paused: id={}", id.id)
            );
        }
    }
    
    pub fn cancel_animation(&mut self, id: AnimationId) {
        if let Some(anim) = self.animations.get_mut(&id) {
            anim.running = false;
            anim.paused = false;
            
            self.running_animations.retain(|running_id| *running_id != id);
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Debug,
                "AnimationEngine",
                &format!("Animation cancelled: id={}", id.id)
            );
        }
    }
    
    pub fn update(&mut self, delta_time: f32) {
        let _trace = ScopedTrace::new("ui_animation_update");
        
        for id in &self.running_animations {
            let anim = self.animations.get_mut(id).unwrap();
            
            if anim.paused { continue; }
            
            anim.elapsed_time += delta_time;
            
            let current_value = anim.current_value();
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Trace,
                "AnimationEngine",
                &format!("Animation {} updated: progress={}, value={}", id.id, 
                    anim.elapsed_time / anim.duration, current_value)
            );
            
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
                    
                    self.dfx.lock().get_logger().lock().log(
                        LogLevel::Info,
                        "AnimationEngine",
                        &format!("Animation completed: id={}", id.id)
                    );
                }
            }
        }
        
        self.running_animations.retain(|id| {
            self.animations.get(id).map(|a| a.running).unwrap_or(false)
        });
        
        self.dfx.lock().get_perf_monitor().lock().record_counter(
            "ui_running_animations",
            self.running_animations.len() as f32
        );
    }
    
    pub fn get_animation_value(&self, id: AnimationId) -> Option<f32> {
        self.animations.get(&id).map(|anim| anim.current_value())
    }
    
    pub fn is_animation_running(&self, id: AnimationId) -> bool {
        self.animations.get(&id).map(|anim| anim.running).unwrap_or(false)
    }
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(DfxSystem::new())))
    }
}