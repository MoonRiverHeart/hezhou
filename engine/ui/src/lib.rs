pub mod types;
pub mod widget;
pub mod event;
pub mod gesture;
pub mod animation;
pub mod canvas;
pub mod layout;
pub mod style;
pub mod widget_tree;
pub mod event_dispatcher;
pub mod gesture_recognizer;
pub mod animation_engine;

pub use types::*;
pub use widget::*;
pub use event::*;
pub use gesture::*;
pub use animation::*;
pub use canvas::*;
pub use layout::*;
pub use style::*;
pub use widget_tree::*;
pub use event_dispatcher::*;
pub use gesture_recognizer::*;
pub use animation_engine::*;

use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct UISystem {
    widget_tree: Arc<Mutex<WidgetTree>>,
    event_dispatcher: Arc<Mutex<EventDispatcher>>,
    gesture_recognizer: Arc<Mutex<GestureRecognizer>>,
    animation_engine: Arc<Mutex<AnimationEngine>>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl UISystem {
    pub fn new() -> Self {
        let dfx = Arc::new(Mutex::new(DfxSystem::new()));
        
        Self {
            widget_tree: Arc::new(Mutex::new(WidgetTree::new())),
            event_dispatcher: Arc::new(Mutex::new(EventDispatcher::new(Arc::clone(&dfx)))),
            gesture_recognizer: Arc::new(Mutex::new(GestureRecognizer::new(Arc::clone(&dfx)))),
            animation_engine: Arc::new(Mutex::new(AnimationEngine::new(Arc::clone(&dfx)))),
            dfx,
        }
    }
    
    pub fn update(&mut self, delta_time: f32) {
        let _trace = ScopedTrace::new("ui_system_update");
        
        self.animation_engine.lock().update(delta_time);
        self.widget_tree.lock().update_layout();
    }
    
    pub fn get_widget_tree(&self) -> Arc<Mutex<WidgetTree>> {
        Arc::clone(&self.widget_tree)
    }
    
    pub fn get_event_dispatcher(&self) -> Arc<Mutex<EventDispatcher>> {
        Arc::clone(&self.event_dispatcher)
    }
    
    pub fn get_animation_engine(&self) -> Arc<Mutex<AnimationEngine>> {
        Arc::clone(&self.animation_engine)
    }
}

impl Default for UISystem {
    fn default() -> Self {
        Self::new()
    }
}