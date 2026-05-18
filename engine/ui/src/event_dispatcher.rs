use crate::event::*;
use crate::gesture::*;
use crate::gesture_recognizer::*;
use crate::thunk_manager::*;
use crate::types::*;
use crate::widget_tree::*;
use hezhou_dfx::DfxSystem;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct EventDispatcher {
    widget_tree: Arc<Mutex<WidgetTree>>,
    gesture_recognizer: Arc<Mutex<GestureRecognizer>>,
    dfx: Arc<Mutex<DfxSystem>>,
    hovered_widget: Option<WidgetId>,
}

impl EventDispatcher {
    pub fn new(dfx: Arc<Mutex<DfxSystem>>) -> Self {
        Self {
            widget_tree: Arc::new(Mutex::new(WidgetTree::new())),
            gesture_recognizer: Arc::new(Mutex::new(GestureRecognizer::new(Arc::clone(&dfx)))),
            dfx,
            hovered_widget: None,
        }
    }

    pub fn set_widget_tree(&mut self, tree: Arc<Mutex<WidgetTree>>) {
        self.widget_tree = tree;
    }

    pub fn widget_tree_ptr(&self) -> *const Mutex<WidgetTree> {
        Arc::as_ptr(&self.widget_tree)
    }

    pub fn dispatch_event(&mut self, event: &mut Event) {
        // 对于 MouseMove 事件，处理 hover 状态变化
        if event.event_type == EventType::MouseMove {
            let target = match &event.data {
                EventData::Mouse(mouse) => {
                    let point = Point::new(mouse.x, mouse.y);
                    self.widget_tree.lock().hit_test(point)
                }
                _ => None,
            };
            
            let new_hovered = target;
            let old_hovered = self.hovered_widget;
            
            // 如果 hover 状态发生变化
            if new_hovered != old_hovered {
                // 发送 MouseLeave 给旧的 widget
                if let Some(old_id) = old_hovered {
                    let timestamp = event.timestamp;
                    let mut leave_event = Event::new(EventType::MouseLeave, timestamp);
                    leave_event.target = old_id;
                    let path = self.widget_tree.lock().find_path(old_id);
                    self.dispatch_bubbling(&path, &mut leave_event);
                }
                
                // 发送 MouseEnter 给新的 widget
                if let Some(new_id) = new_hovered {
                    let timestamp = event.timestamp;
                    if let EventData::Mouse(mouse_data) = &event.data {
                        let mut enter_event = Event::new(EventType::MouseEnter, timestamp)
                            .with_data(EventData::Mouse(*mouse_data));
                        enter_event.target = new_id;
                        let path = self.widget_tree.lock().find_path(new_id);
                        self.dispatch_bubbling(&path, &mut enter_event);
                    }
                }
                
                self.hovered_widget = new_hovered;
            }
            return;
        }
        
        let (target, click_point) = match &event.data {
            EventData::Touch(touch) => {
                let point = Point::new(touch.x, touch.y);
                (self.widget_tree.lock().hit_test(point), point)
            }
            EventData::Mouse(mouse) => {
                let point = Point::new(mouse.x, mouse.y);
                (self.widget_tree.lock().hit_test(point), point)
            }
            EventData::Key(_) => {
                // KeyDown/KeyUp事件广播到所有widget（不做hit_test）
                // TextEdit检查focused字段来决定是否处理
                (None, Point::new(0.0, 0.0))
            }
            _ => (None, Point::new(0.0, 0.0)),
        };
        
        // 如果是Key事件，广播到所有widget
        if matches!(event.data, EventData::Key(_)) {
            self.broadcast_key_event(event);
            return;
        }
        
        event.target = target.unwrap_or(WidgetId::invalid());

        let path = self.widget_tree.lock().find_path(event.target);

        self.dispatch_capturing(&path, event);

        if !event.immediate_stopped {
            self.dispatch_bubbling(&path, event);
        }

        let gesture = self.gesture_recognizer.lock().process_event(event);
        
        if let Some(g) = gesture {
            if g.gesture_type == GestureType::Tap {
                let mut tree = self.widget_tree.lock();
                let widget_type = tree.get_widget(g.target)
                    .map(|w| w.widget_type())
                    .unwrap_or("");
                drop(tree);
                
                if widget_type == "Button" {
                    let mut tree = self.widget_tree.lock();
                    if let Some(widget) = tree.get_widget_mut(g.target) {
                        use crate::widgets::Button;
                        if let Some(button) = widget.as_any_mut().downcast_mut::<Button>() {
                            button.trigger_click();
                        }
                    }
                    drop(tree);
                    crate::thunk_manager::trigger_onclick_callback(g.target.id);
                } else {
                    crate::thunk_manager::ui_trigger_global_click(click_point.x, click_point.y);
                }
            }
        } else if event.event_type == EventType::TouchEnd {
            crate::thunk_manager::ui_trigger_global_click(click_point.x, click_point.y);
        }
    }
    
    fn broadcast_key_event(&mut self, event: &mut Event) {
        let mut tree = self.widget_tree.lock();
        
        let widget_ids: Vec<WidgetId> = tree.get_all_widget_ids();
        
        for widget_id in widget_ids {
            if event.immediate_stopped {
                break;
            }
            
            if let Some(widget) = tree.get_widget_mut(widget_id) {
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

    fn dispatch_capturing(&mut self, path: &[WidgetId], event: &mut Event) {
        for widget_id in path {
            if event.immediate_stopped {
                break;
            }

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
            if event.stopped || event.immediate_stopped {
                break;
            }

            let abs_layout = {
                let tree = self.widget_tree.lock();
                tree.get_absolute_layout(*widget_id)
            };
            
            // 如果是Touch事件且需要坐标转换，创建转换后的副本
            let converted_event = if let Some(abs_layout) = abs_layout {
                if let EventData::Touch(touch_data) = &event.data {
                    let window_x = touch_data.x;
                    let window_y = touch_data.y;
                    let relative_x = window_x - abs_layout.x;
                    let relative_y = window_y - abs_layout.y;
                    
                    println!("[Dispatch] Widget {}: window ({}, {}) -> relative ({}, {})", 
                             widget_id.id, window_x, window_y, relative_x, relative_y);
                    
                    let mut converted = event.clone();
                    converted.data = EventData::Touch(TouchData::new(relative_x, relative_y, touch_data.pointer_id));
                    Some(converted)
                } else {
                    None
                }
            } else {
                None
            };

            let mut tree = self.widget_tree.lock();
            if let Some(widget) = tree.get_widget_mut(*widget_id) {
                let result = if let Some(converted) = converted_event {
                    widget.as_mut().on_event(&converted)
                } else {
                    widget.as_mut().on_event(event)
                };
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
