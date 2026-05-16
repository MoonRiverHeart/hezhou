use crate::event::*;
use crate::gesture::*;
use crate::types::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct GestureRecognizer {
    active_gestures: Vec<ActiveGesture>,
    dfx: Arc<Mutex<DfxSystem>>,
}

struct ActiveGesture {
    target: WidgetId,
    gesture_type: GestureType,
    state: GestureState,
    start_time: u64,
    start_pos: Point,
    current_pos: Point,
    tap_count: u32,
}

impl GestureRecognizer {
    pub fn new(dfx: Arc<Mutex<DfxSystem>>) -> Self {
        Self {
            active_gestures: Vec::new(),
            dfx,
        }
    }
    
    pub fn process_event(&mut self, event: &Event) {
        match event.event_type {
            EventType::TouchBegin => {
                self.on_touch_begin(event);
            }
            EventType::TouchMove => {
                self.on_touch_move(event);
            }
            EventType::TouchEnd => {
                self.on_touch_end(event);
            }
            _ => {}
        }
    }
    
    fn on_touch_begin(&mut self, event: &Event) {
        if let EventData::Touch(touch) = &event.data {
            self.active_gestures.push(ActiveGesture {
                target: event.target,
                gesture_type: GestureType::Tap,
                state: GestureState::Possible,
                start_time: event.timestamp,
                start_pos: Point::new(touch.x, touch.y),
                current_pos: Point::new(touch.x, touch.y),
                tap_count: 0,
            });
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Debug,
                "GestureRecognizer",
                &format!("TouchBegin: target={}, pos={},{}", 
                    event.target.id, touch.x, touch.y)
            );
        }
    }
    
    fn on_touch_move(&mut self, event: &Event) {
        if let EventData::Touch(touch) = &event.data {
            for gesture in &mut self.active_gestures {
                if gesture.target == event.target {
                    gesture.current_pos = Point::new(touch.x, touch.y);
                    
                    let distance = gesture.start_pos.distance(&gesture.current_pos);
                    if distance > 10.0 && gesture.gesture_type == GestureType::Tap {
                        gesture.gesture_type = GestureType::Pan;
                        gesture.state = GestureState::Began;
                    }
                }
            }
        }
    }
    
    fn on_touch_end(&mut self, event: &Event) {
        if let EventData::Touch(touch) = &event.data {
            for gesture in &mut self.active_gestures {
                if gesture.target == event.target && gesture.state != GestureState::Failed {
                    let elapsed = event.timestamp - gesture.start_time;
                    let distance = gesture.start_pos.distance(&Point::new(touch.x, touch.y));
                    
                    match gesture.gesture_type {
                        GestureType::Tap => {
                            if elapsed < 500 && distance < 10.0 {
                                gesture.state = GestureState::Ended;
                                gesture.tap_count += 1;
                                
                                self.dfx.lock().get_logger().lock().log(
                                    LogLevel::Info,
                                    "GestureRecognizer",
                                    &format!("Tap gesture recognized: target={}, count={}", 
                                        gesture.target.id, gesture.tap_count)
                                );
                            } else {
                                gesture.state = GestureState::Failed;
                            }
                        }
                        GestureType::Pan => {
                            gesture.state = GestureState::Ended;
                        }
                        _ => {}
                    }
                }
            }
            
            self.active_gestures.retain(|g| g.state != GestureState::Ended && g.state != GestureState::Failed);
        }
    }
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(DfxSystem::new())))
    }
}