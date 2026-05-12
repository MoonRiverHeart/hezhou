use crate::event::*;

pub struct EventDispatcher {
    queue: Vec<Event>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
        }
    }
    
    pub fn enqueue(&mut self, event: Event) {
        self.queue.push(event);
    }
    
    pub fn dispatch_all(&mut self, bus: &mut EventBus) {
        for event in self.queue.drain(..) {
            bus.publish(event);
        }
        bus.dispatch();
    }
    
    pub fn dispatch_immediate(&self, bus: &mut EventBus, event: Event) {
        bus.dispatch_event(&event);
    }
    
    pub fn clear(&mut self) {
        self.queue.clear();
    }
    
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}