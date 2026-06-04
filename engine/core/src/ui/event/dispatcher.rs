use std::collections::VecDeque;
use super::types::{UIEvent};

/// 事件队列
pub struct EventQueue {
    events: VecDeque<UIEvent>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }
    
    pub fn push(&mut self, event: UIEvent) {
        self.events.push_back(event);
    }
    
    pub fn pop(&mut self) -> Option<UIEvent> {
        self.events.pop_front()
    }
    
    pub fn peek(&self) -> Option<&UIEvent> {
        self.events.front()
    }
    
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
    
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// 事件处理器 trait
pub trait EventHandler {
    fn handle_event(&mut self, event: &UIEvent) -> bool;
}

/// 事件分发器
pub struct EventDispatcher {
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    
    pub fn add_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }
    
    pub fn dispatch(&mut self, event: &UIEvent) -> bool {
        for handler in &mut self.handlers {
            if handler.handle_event(event) {
                return true; // 事件已处理
            }
        }
        false
    }
    
    pub fn dispatch_queue(&mut self, queue: &mut EventQueue) {
        while let Some(event) = queue.pop() {
            self.dispatch(&event);
        }
    }
}