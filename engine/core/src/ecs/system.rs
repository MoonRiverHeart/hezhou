pub trait System {
    fn name(&self) -> &'static str;
    fn update(&mut self, world: &mut crate::ecs::World, delta_time: f32);
    fn priority(&self) -> i32 {
        0
    }
    fn is_active(&self) -> bool {
        true
    }
}

pub struct SystemScheduler {
    systems: Vec<Box<dyn System>>,
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }
    
    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push(system);
        self.systems.sort_by_key(|s| s.priority());
    }
    
    pub fn update(&mut self, world: &mut crate::ecs::World, delta_time: f32) {
        for system in &mut self.systems {
            if system.is_active() {
                system.update(world, delta_time);
            }
        }
    }
    
    pub fn remove_system(&mut self, name: &str) {
        self.systems.retain(|s| s.name() != name);
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}