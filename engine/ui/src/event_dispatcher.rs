use crate::event::*;
use crate::gesture::*;
use crate::gesture_recognizer::*;
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
        if event.event_type == EventType::TouchBegin {
            let focused_id = crate::thunk::ui_get_focused_input_field();
            if focused_id != 0 {
                let target = match &event.data {
                    EventData::Touch(touch) => {
                        self.widget_tree.lock().hit_test(Point::new(touch.x, touch.y))
                    }
                    EventData::Mouse(mouse) => {
                        self.widget_tree.lock().hit_test(Point::new(mouse.x, mouse.y))
                    }
                    _ => None,
                };
                
                let target_id = target.map(|t| t.id).unwrap_or(0);
                
                if target_id != focused_id {
                    let mut tree = self.widget_tree.lock();
                    let old_widget_id = crate::WidgetId::from_raw(focused_id);
                    if let Some(widget) = tree.get_widget_mut(old_widget_id) {
                        if widget.widget_type() == "InputField" {
                            if let Some(input) = widget.as_any_mut().downcast_mut::<crate::widgets::InputField>() {
                                input.blur_internal();
                            }
                        }
                    }
                    drop(tree);
                    crate::thunk::ui_clear_focused_input_field();
                }
            }
        }
        
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
            
            // 将MouseMove事件也dispatch给target widget（Dropdown需要MouseMove更新hovered_index）
            if let Some(target_id) = new_hovered {
                let mut move_event = event.clone();
                move_event.target = target_id;
                let path = self.widget_tree.lock().find_path(target_id);
                self.dispatch_bubbling(&path, &mut move_event);
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
        
        // Process gesture Tap on Button even when event.stopped — Button intentionally
        // stops propagation on TouchEnd, but we still need to recognize the Tap gesture
        // and trigger thunk callbacks. Only skip global_click for stopped events.
        if let Some(g) = gesture {
            if g.gesture_type == GestureType::Tap {
                let mut tree = self.widget_tree.lock();
                let widget_type = tree.get_widget(g.target)
                    .map(|w| w.widget_type())
                    .unwrap_or("");
                
                // Close any visible PopupMenu that was NOT the click target
                // (click-outside dismiss behavior for popup menus)
                let all_ids = tree.get_all_widget_ids();
                let popup_ids_to_close: Vec<u64> = all_ids.iter()
                    .filter_map(|wid| {
                        if wid.id != g.target.id {
                            if let Some(w) = tree.get_widget(*wid) {
                                if w.widget_type() == "PopupMenu" {
                                    if let Some(pm) = w.as_any().downcast_ref::<crate::widgets::PopupMenu>() {
                                        if pm.is_visible() {
                                            return Some(wid.id);
                                        }
                                    }
                                }
                            }
                        }
                        None
                    })
                    .collect();
                
                // Close any open Dropdown that was NOT the click target
                // (click-outside dismiss behavior for dropdown popups)
                let dropdown_ids_to_close: Vec<u64> = all_ids.iter()
                    .filter_map(|wid| {
                        if wid.id != g.target.id {
                            if let Some(w) = tree.get_widget(*wid) {
                                if w.widget_type() == "Dropdown" {
                                    if let Some(dd) = w.as_any().downcast_ref::<crate::widgets::Dropdown>() {
                                        if dd.is_open() {
                                            return Some(wid.id);
                                        }
                                    }
                                }
                            }
                        }
                        None
                    })
                    .collect();
                
                drop(tree);
                
                // Hide popup menus and notify C# that they closed
                for popup_id in popup_ids_to_close {
                    let mut tree = self.widget_tree.lock();
                    if let Some(w) = tree.get_widget_mut(crate::WidgetId::from_raw(popup_id)) {
                        if let Some(pm) = w.as_any_mut().downcast_mut::<crate::widgets::PopupMenu>() {
                            pm.hide();
                            crate::thunk::queue_callback(crate::thunk::PendingCallback::PopupMenuClose { widget_id: popup_id });
                        }
                    }
                    drop(tree);
                }
                
                // Close open dropdowns
                for dropdown_id in dropdown_ids_to_close {
                    let mut tree = self.widget_tree.lock();
                    if let Some(w) = tree.get_widget_mut(crate::WidgetId::from_raw(dropdown_id)) {
                        if let Some(dd) = w.as_any_mut().downcast_mut::<crate::widgets::Dropdown>() {
                            dd.close();
                        }
                    }
                    drop(tree);
                }
                
                // Auto-deselect PreviewWindow when clicking on anything that is NOT PreviewWindow
                // (click-outside deselect behavior — PreviewWindow和InputField互斥)
                let preview_ids_to_deselect: Vec<u64> = all_ids.iter()
                    .filter_map(|wid| {
                        if wid.id != g.target.id {
                            let mut tree = self.widget_tree.lock();
                            if let Some(w) = tree.get_widget(*wid) {
                                if w.widget_type() == "PreviewWindow" {
                                    if let Some(pw) = w.as_any().downcast_ref::<crate::widgets::PreviewWindow>() {
                                        if pw.is_selected() {
                                            drop(tree);
                                            return Some(wid.id);
                                        }
                                    }
                                }
                            }
                            drop(tree);
                        }
                        None
                    })
                    .collect();
                
                for preview_id in preview_ids_to_deselect {
                    let mut tree2 = self.widget_tree.lock();
                    if let Some(w) = tree2.get_widget_mut(crate::WidgetId::from_raw(preview_id)) {
                        if let Some(pw) = w.as_any_mut().downcast_mut::<crate::widgets::PreviewWindow>() {
                            pw.set_selected(false);
                        }
                    }
                    drop(tree2);
                }
                
                let mut tree = self.widget_tree.lock();
                let widget_type = tree.get_widget(g.target)
                    .map(|w| w.widget_type())
                    .unwrap_or("");
                drop(tree);
                
                // Button Tap: always trigger callback regardless of event.stopped
                if widget_type == "Button" {
                    let mut tree = self.widget_tree.lock();
                    if let Some(widget) = tree.get_widget_mut(g.target) {
                        use crate::widgets::Button;
                        if let Some(button) = widget.as_any_mut().downcast_mut::<Button>() {
                            button.trigger_click();
                        }
                    }
                    drop(tree);
                    crate::thunk::queue_callback(crate::thunk::PendingCallback::ButtonClick { widget_id: g.target.id });
                } else if widget_type == "Label" {
                    // Label with OnClick callback: trigger it (used for menu bar items)
                    if crate::thunk::has_onclick_callback(g.target.id) {
                        crate::thunk::queue_callback(crate::thunk::PendingCallback::ButtonClick { widget_id: g.target.id });
                    }
                } else if !event.stopped {
                    // For non-Button widgets, only trigger global_click if event not stopped
                    
                    // Widget types that handle their own click callbacks — skip global_click
                    const GLOBAL_CLICK_EXCLUDE: &[&str] = &[
                        "PopupMenu", "PreviewWindow", "Dialog",
                        "TreeView", "TreeNode", "GridView", "Dropdown", "InputField",
                        "ScrollView", "Label", "VStack", "HStack", "Panel",
                        "Checkbox", "Slider", "TextEdit", "TabWidget", "Image",
                    ];
                    let should_global_click = !GLOBAL_CLICK_EXCLUDE.contains(&widget_type);
                    
                    if should_global_click {
                        crate::thunk::ui_trigger_global_click(click_point.x, click_point.y);
                    }
                }
            }
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
        // Capturing phase: iterate from root to target's PARENT (skip target itself).
        // Target widget only receives events in the bubbling phase.
        // This follows W3C DOM event model: capture → target → bubble.
        let capture_path = if path.len() > 1 {
            &path[..path.len() - 1]
        } else {
            // Single-element path (target is root): no capturing phase needed
            &path[..0]
        };
        
        for widget_id in capture_path {
            if event.immediate_stopped {
                break;
            }

            let abs_layout = {
                let tree = self.widget_tree.lock();
                tree.get_absolute_layout(*widget_id)
            };
            
            // Convert Touch coordinates to local coords for capturing phase (same as bubbling)
            let converted_event = if let Some(abs_layout) = abs_layout {
                if let EventData::Touch(touch_data) = &event.data {
                    let window_x = touch_data.x;
                    let window_y = touch_data.y;
                    let relative_x = window_x - abs_layout.x;
                    let relative_y = window_y - abs_layout.y;
                    
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
            
            // RightClick事件保持绝对屏幕坐标（context menu定位需要屏幕位置）
            // 其他Touch/Mouse事件做坐标转换（屏幕绝对坐标 → widget局部坐标）
            let should_convert_coords = event.event_type != EventType::RightClick;
            
            let converted_event = if should_convert_coords {
                if let Some(abs_layout) = abs_layout {
                    match &event.data {
                        EventData::Touch(touch_data) => {
                            let relative_x = touch_data.x - abs_layout.x;
                            let relative_y = touch_data.y - abs_layout.y;
                            let mut converted = event.clone();
                            converted.data = EventData::Touch(TouchData::new(relative_x, relative_y, touch_data.pointer_id));
                            Some(converted)
                        }
                        EventData::Mouse(mouse_data) => {
                            let relative_x = mouse_data.x - abs_layout.x;
                            let relative_y = mouse_data.y - abs_layout.y;
                            let mut converted = event.clone();
                            converted.data = EventData::Mouse(MouseData::new(relative_x, relative_y, mouse_data.button));
                            Some(converted)
                        }
                        _ => None
                    }
                } else {
                    None
                }
            } else {
                None  // RightClick: 保持绝对坐标不转换
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
