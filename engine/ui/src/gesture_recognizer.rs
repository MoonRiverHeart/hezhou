use crate::event::*;
use crate::gesture::*;
use crate::types::*;
use hezhou_dfx::{DfxSystem, dfx_debug};
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
    velocity: Point,
    tap_count: u32,
    last_tap_time: u64,
}

impl GestureRecognizer {
    pub fn new(dfx: Arc<Mutex<DfxSystem>>) -> Self {
        Self {
            active_gestures: Vec::new(),
            dfx,
        }
    }

    pub fn process_event(&mut self, event: &Event) -> Option<Gesture> {
        match event.event_type {
            EventType::TouchBegin => self.on_touch_begin(event),
            EventType::TouchMove => self.on_touch_move(event),
            EventType::TouchEnd => self.on_touch_end(event),
            EventType::TouchCancel => self.on_touch_cancel(event),
            _ => None,
        }
    }

    fn on_touch_begin(&mut self, event: &Event) -> Option<Gesture> {
        if let EventData::Touch(touch) = &event.data {
            let gesture = ActiveGesture {
                target: event.target,
                gesture_type: GestureType::Tap,
                state: GestureState::Possible,
                start_time: event.timestamp,
                start_pos: Point::new(touch.x, touch.y),
                current_pos: Point::new(touch.x, touch.y),
                velocity: Point::zero(),
                tap_count: 0,
                last_tap_time: 0,
            };

            self.active_gestures.push(gesture);

            dfx_debug!("Gesture", "TouchBegin: target={}", event.target.id);
        }
        None
    }

    fn on_touch_move(&mut self, event: &Event) -> Option<Gesture> {
        if let EventData::Touch(touch) = &event.data {
            for gesture in &mut self.active_gestures {
                if gesture.target == event.target {
                    gesture.current_pos = Point::new(touch.x, touch.y);

                    let distance = gesture.start_pos.distance(&gesture.current_pos);

                    if distance > 10.0 && gesture.gesture_type == GestureType::Tap {
                        gesture.gesture_type = GestureType::Pan;
                        gesture.state = GestureState::Began;

                        return Some(Gesture::new(GestureType::Pan, gesture.target).with_data(
                            GestureData::Pan(PanData {
                                start_x: gesture.start_pos.x,
                                start_y: gesture.start_pos.y,
                                current_x: gesture.current_pos.x,
                                current_y: gesture.current_pos.y,
                                velocity_x: gesture.velocity.x,
                                velocity_y: gesture.velocity.y,
                            }),
                        ));
                    }
                }
            }
        }
        None
    }

    fn on_touch_end(&mut self, event: &Event) -> Option<Gesture> {
        if let EventData::Touch(touch) = &event.data {
            for gesture in &mut self.active_gestures.iter_mut() {
                if gesture.target == event.target && gesture.state != GestureState::Failed {
                    let elapsed = event.timestamp - gesture.start_time;
                    let distance = gesture.start_pos.distance(&Point::new(touch.x, touch.y));

                    match gesture.gesture_type {
                        GestureType::Tap => {
                            if elapsed < 500 && distance < 10.0 {
                                gesture.state = GestureState::Ended;
                                gesture.tap_count += 1;

                                let gesture_type = if gesture.tap_count == 2 {
                                    GestureType::DoubleTap
                                } else if elapsed > 300 {
                                    GestureType::LongPress
                                } else {
                                    GestureType::Tap
                                };

                                dfx_debug!("Gesture", "{} recognized: target={}",
                                    match gesture_type {
                                        GestureType::Tap => "Tap",
                                        GestureType::DoubleTap => "DoubleTap",
                                        GestureType::LongPress => "LongPress",
                                        _ => "Unknown",
                                    },
                                    gesture.target.id
                                );

                                return Some(
                                    Gesture::new(gesture_type, gesture.target).with_data(
                                        GestureData::Tap(
                                            TapData::new(touch.x, touch.y)
                                                .with_count(gesture.tap_count),
                                        ),
                                    ),
                                );
                            } else {
                                gesture.state = GestureState::Failed;
                            }
                        }

                        GestureType::Pan => {
                            gesture.state = GestureState::Ended;

                            return Some(Gesture::new(GestureType::Pan, gesture.target).with_data(
                                GestureData::Pan(PanData {
                                    start_x: gesture.start_pos.x,
                                    start_y: gesture.start_pos.y,
                                    current_x: gesture.current_pos.x,
                                    current_y: gesture.current_pos.y,
                                    velocity_x: gesture.velocity.x,
                                    velocity_y: gesture.velocity.y,
                                }),
                            ));
                        }

                        _ => {}
                    }
                }
            }

            self.active_gestures
                .retain(|g| g.state != GestureState::Ended && g.state != GestureState::Failed);
        }
        None
    }

    fn on_touch_cancel(&mut self, event: &Event) -> Option<Gesture> {
        self.active_gestures.retain(|g| g.target != event.target);
        None
    }
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(DfxSystem::new())))
    }
}
