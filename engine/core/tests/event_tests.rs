use core::ui::event::{
    UIEvent, MouseEvent, MouseEventType, MouseButton,
    KeyboardEvent, KeyboardEventType, Key, SpecialKey,
    Point, Modifiers, EventQueue, EventDispatcher, EventHandler,
};

use std::time::SystemTime;

// 自定义事件处理器用于测试
#[derive(Debug)]
struct TestEventHandler {
    mouse_click_received: bool,
    key_press_received: bool,
    last_event: Option<String>,
}

impl TestEventHandler {
    fn new() -> Self {
        Self {
            mouse_click_received: false,
            key_press_received: false,
            last_event: None,
        }
    }
    
    fn reset(&mut self) {
        self.mouse_click_received = false;
        self.key_press_received = false;
        self.last_event = None;
    }
}

impl EventHandler for TestEventHandler {
    fn handle_event(&mut self, event: &UIEvent) -> bool {
        match event {
            UIEvent::Mouse(mouse_event) => {
                if let MouseEventType::Clicked = mouse_event.event_type {
                    self.mouse_click_received = true;
                    self.last_event = Some(format!("鼠标点击 at ({}, {})", 
                        mouse_event.position.x, mouse_event.position.y));
                    return true;
                }
            }
            UIEvent::Keyboard(key_event) => {
                if let KeyboardEventType::Pressed = key_event.event_type {
                    self.key_press_received = true;
                    self.last_event = Some(format!("按键按下: {:?}", key_event.key));
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_queue_creation() {
        let queue = EventQueue::new();
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_push_and_pop_event() {
        let mut queue = EventQueue::new();
        
        let click = UIEvent::Mouse(MouseEvent {
            timestamp: SystemTime::now(),
            position: Point::new(100.0, 200.0),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
            event_type: MouseEventType::Clicked,
            handled: false,
        });
        
        queue.push(click);
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);
        
        let popped = queue.pop();
        assert!(popped.is_some());
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_mouse_click_event() {
        let mut queue = EventQueue::new();
        
        let click = UIEvent::Mouse(MouseEvent {
            timestamp: SystemTime::now(),
            position: Point::new(100.0, 200.0),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
            event_type: MouseEventType::Clicked,
            handled: false,
        });
        
        queue.push(click);
        
        let mut handler = TestEventHandler::new();
        
        if let Some(event) = queue.pop() {
            handler.handle_event(&event);
        }
        
        assert!(handler.mouse_click_received);
        assert_eq!(handler.last_event, Some("鼠标点击 at (100, 200)".to_string()));
    }
    
    #[test]
    fn test_keyboard_press_event() {
        let mut queue = EventQueue::new();
        
        let key_press = UIEvent::Keyboard(KeyboardEvent {
            timestamp: SystemTime::now(),
            key: Key::Special(SpecialKey::Enter),
            modifiers: Modifiers::none(),
            event_type: KeyboardEventType::Pressed,
            handled: false,
        });
        
        queue.push(key_press);
        
        let mut handler = TestEventHandler::new();
        
        if let Some(event) = queue.pop() {
            handler.handle_event(&event);
        }
        
        assert!(handler.key_press_received);
        assert_eq!(handler.last_event, Some("按键按下: Special(Enter)".to_string()));
    }
    
    #[test]
    fn test_event_dispatcher() {
        let mut queue = EventQueue::new();
        
        let click = UIEvent::Mouse(MouseEvent {
            timestamp: SystemTime::now(),
            position: Point::new(100.0, 200.0),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
            event_type: MouseEventType::Clicked,
            handled: false,
        });
        
        let key_press = UIEvent::Keyboard(KeyboardEvent {
            timestamp: SystemTime::now(),
            key: Key::Special(SpecialKey::Enter),
            modifiers: Modifiers::none(),
            event_type: KeyboardEventType::Pressed,
            handled: false,
        });
        
        queue.push(click);
        queue.push(key_press);
        
        let mut dispatcher = EventDispatcher::new();
        let handler = Box::new(TestEventHandler::new());
        dispatcher.add_handler(handler);
        
        dispatcher.dispatch_queue(&mut queue);
        
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_event_timestamp() {
        let before = SystemTime::now();
        let click = UIEvent::Mouse(MouseEvent {
            timestamp: SystemTime::now(),
            position: Point::new(100.0, 200.0),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
            event_type: MouseEventType::Clicked,
            handled: false,
        });
        let after = SystemTime::now();
        
        let timestamp = click.timestamp();
        assert!(timestamp >= before);
        assert!(timestamp <= after);
    }
    
    #[test]
    fn test_event_handled_flag() {
        let mut click = UIEvent::Mouse(MouseEvent {
            timestamp: SystemTime::now(),
            position: Point::new(100.0, 200.0),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
            event_type: MouseEventType::Clicked,
            handled: false,
        });
        
        assert!(!click.is_handled());
        
        click.set_handled();
        assert!(click.is_handled());
    }
    
    #[test]
    fn test_modifiers() {
        let mut modifiers = Modifiers::none();
        assert!(modifiers.is_empty());
        
        modifiers.shift = true;
        assert!(!modifiers.is_empty());
        assert!(modifiers.shift);
        assert!(!modifiers.control);
        assert!(!modifiers.alt);
        assert!(!modifiers.super_key);
    }
    
    #[test]
    fn test_point_creation() {
        let point = Point::new(150.0, 250.0);
        assert_eq!(point.x, 150.0);
        assert_eq!(point.y, 250.0);
    }
    
    #[test]
    fn test_event_queue_clear() {
        let mut queue = EventQueue::new();
        
        let click = UIEvent::Mouse(MouseEvent {
            timestamp: SystemTime::now(),
            position: Point::new(100.0, 200.0),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
            event_type: MouseEventType::Clicked,
            handled: false,
        });
        
        queue.push(click);
        assert!(!queue.is_empty());
        
        queue.clear();
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_multiple_events_in_queue() {
        let mut queue = EventQueue::new();
        
        for i in 0..5 {
            let click = UIEvent::Mouse(MouseEvent {
                timestamp: SystemTime::now(),
                position: Point::new(100.0 + i as f32, 200.0),
                button: Some(MouseButton::Left),
                modifiers: Modifiers::none(),
                event_type: MouseEventType::Clicked,
                handled: false,
            });
            queue.push(click);
        }
        
        assert_eq!(queue.len(), 5);
        
        for i in 0..5 {
            let event = queue.pop();
            assert!(event.is_some());
            if let Some(UIEvent::Mouse(mouse)) = event {
                assert_eq!(mouse.position.x, 100.0 + i as f32);
            }
        }
        
        assert!(queue.is_empty());
    }
}