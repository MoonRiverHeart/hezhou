pub mod ecs;
pub mod event;
pub mod ffi;
pub mod math;
pub mod time_loop;
pub mod scene_ffi;
pub mod property_ffi;
pub mod asset_library;
pub mod project;
pub mod asset_ffi;

pub use ecs::*;
pub use event::*;
pub use math::*;
pub use time_loop::*;
pub use scene_ffi::*;
pub use property_ffi::*;
pub use asset_library::*;
pub use project::*;
pub use asset_ffi::*;

pub struct Engine {
    pub world: World,
    pub event_bus: EventBus,
    pub time: Time,
    pub is_running: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            event_bus: EventBus::new(),
            time: Time::new(),
            is_running: false,
        }
    }

    pub fn start(&mut self) {
        self.is_running = true;
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn run_frame(&mut self, delta_time: f32) {
        self.time.update(delta_time);

        self.event_bus.dispatch_pre_update();
        self.world.update(delta_time);
        self.event_bus.dispatch_post_update();
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
