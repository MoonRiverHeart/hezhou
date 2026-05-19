pub mod animation;
pub mod animation_engine;
pub mod canvas;
pub mod event;
pub mod event_dispatcher;
pub mod ffi;
pub mod font_atlas;
pub mod gesture;
pub mod gesture_recognizer;
pub mod layout;
pub mod platform;
pub mod style;
pub mod thunk_manager;
pub mod types;
pub mod widget;
pub mod widget_tree;
pub mod widgets;

#[cfg(feature = "msdf")]
pub mod msdf;

pub use animation::*;
pub use animation_engine::*;
pub use canvas::*;
pub use event::*;
pub use event_dispatcher::*;
pub use font_atlas::*;
pub use gesture::*;
pub use gesture_recognizer::*;
pub use layout::*;
pub use platform::*;
pub use style::*;
pub use thunk_manager::*;
pub use types::*;
pub use widget::*;
pub use widget_tree::*;
pub use widgets::*;

use hezhou_dfx::*;
use std::sync::Arc;

pub struct UISystem {
    widget_tree: Arc<parking_lot::Mutex<WidgetTree>>,
    event_dispatcher: Arc<parking_lot::Mutex<EventDispatcher>>,
    gesture_recognizer: Arc<parking_lot::Mutex<GestureRecognizer>>,
    animation_engine: Arc<parking_lot::Mutex<AnimationEngine>>,
    font_atlas: FontAtlas,
    dfx: Arc<parking_lot::Mutex<DfxSystem>>,
}

impl UISystem {
    pub fn new() -> Self {
        let dfx = Arc::new(parking_lot::Mutex::new(DfxSystem::new()));
        let font_atlas = create_font_atlas();
        
        Self {
            widget_tree: Arc::new(parking_lot::Mutex::new(WidgetTree::new())),
            event_dispatcher: Arc::new(parking_lot::Mutex::new(EventDispatcher::new(Arc::clone(&dfx)))),
            gesture_recognizer: Arc::new(parking_lot::Mutex::new(GestureRecognizer::new(Arc::clone(&dfx)))),
            animation_engine: Arc::new(parking_lot::Mutex::new(AnimationEngine::new(Arc::clone(&dfx)))),
            font_atlas,
            dfx,
        }
    }
    
    pub fn update(&mut self, delta_time: f32) {
        let perf_monitor = self.dfx.lock().get_perf_monitor();
        perf_monitor.lock().begin_frame();
        
        self.animation_engine.lock().update(delta_time);
        self.widget_tree.lock().update_layout();
        
        perf_monitor.lock().end_frame();
    }
    
    pub fn enable_perf_monitoring(&mut self) {
        self.dfx.lock().get_perf_monitor().lock().enable();
    }
    
    pub fn disable_perf_monitoring(&mut self) {
        self.dfx.lock().get_perf_monitor().lock().disable();
    }
    
    pub fn get_fps(&self) -> f32 {
        self.dfx.lock().get_perf_monitor().lock().get_fps()
    }
    
    pub fn get_frame_time(&self) -> f32 {
        self.dfx.lock().get_perf_monitor().lock().get_frame_time_ms()
    }
    
    pub fn get_perf_stats(&self) -> Option<hezhou_dfx::PerformanceSnapshot> {
        self.dfx.lock().get_perf_monitor().lock().get_latest_snapshot()
    }
    
    pub fn get_widget_tree(&self) -> Arc<parking_lot::Mutex<WidgetTree>> {
        Arc::clone(&self.widget_tree)
    }
    
    pub fn get_font_atlas(&self) -> &FontAtlas {
        &self.font_atlas
    }
    
    pub fn get_dfx(&self) -> Arc<parking_lot::Mutex<DfxSystem>> {
        Arc::clone(&self.dfx)
    }
    
    pub fn get_event_dispatcher(&self) -> Arc<parking_lot::Mutex<EventDispatcher>> {
        Arc::clone(&self.event_dispatcher)
    }
    
    pub fn get_animation_engine(&self) -> Arc<parking_lot::Mutex<AnimationEngine>> {
        Arc::clone(&self.animation_engine)
    }
}

impl Default for UISystem {
    fn default() -> Self {
        Self::new()
    }
}
