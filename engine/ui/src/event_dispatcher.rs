use crate::types::*;
use crate::event::*;
use crate::widget_tree::*;
use crate::gesture_recognizer::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct EventDispatcher {
    widget_tree: Arc<Mutex<WidgetTree>>,
    gesture_recognizer: Arc<Mutex<GestureRecognizer>>,
    dfx: Arc<Mutex<DfxSystem>>,
}

impl EventDispatcher {
    pub fn new(dfx: Arc<Mutex<DfxSystem>>) -> Self {
        Self {
            widget_tree: Arc::new(Mutex::new(WidgetTree::new())),
            gesture_recognizer: Arc::new(Mutex::new(GestureRecognizer::new(Arc::clone(&dfx)))),
            dfx,
        }
    }
    
    pub fn set_widget_tree(&mut self, tree: Arc<Mutex<WidgetTree>>) {
        self.widget_tree = tree;
    }
    
    pub fn dispatch_event(&mut self, event: &mut Event) {
        if let EventData::Touch(touch) = &event.data {
            let target = self.widget_tree.lock().hit_test(Point::new(touch.x, touch.y));
            event.target = target.unwrap_or(WidgetId::invalid());
        }
        
        let path = self.widget_tree.lock().find_path(event.target);
        
        self.dispatch_capturing(&path, event);
        
        if !event.immediate_stopped {
            self.dispatch_bubbling(&path, event);
        }
        
        self.gesture_recognizer.lock().process_event(event);
        
        println!("[EventDispatcher] Event: type={}, target={}, stopped={}", 
            event.event_type, event.target.id, event.stopped);
    }
    
    fn dispatch_capturing(&mut self, path: &[WidgetId], event: &mut Event) {
        for widget_id in path {
            if event.immediate_stopped { break; }
            
            let mut tree = self.widget_tree.lock();
            if let Some(widget) = tree.get_widget_mut(*widget_id) {
                let result = widget.as_mut().on_event(event);
                match result {
                    EventResult::ImmediateStop => {
                        event.immediate_stopped = true;
                    }
                    _ => {}
                }
            }
        }
    }
    
    fn dispatch_bubbling(&mut self, path: &[WidgetId], event: &mut Event) {
        for widget_id in path.iter().rev() {
            if event.stopped || event.immediate_stopped { break; }
            
            let mut tree = self.widget_tree.lock();
            if let Some(widget) = tree.get_widget_mut(*widget_id) {
                let result = widget.as_mut().on_event(event);
                match result {
                    EventResult::Stopped => {
                        event.stopped = true;
                    }
                    EventResult::ImmediateStop => {
                        event.immediate_stopped = true;
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(DfxSystem::new())))
    }
}