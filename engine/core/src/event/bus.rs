use crate::event::*;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub struct EventBus {
    listeners: HashMap<EventType, Vec<(i32, EventCallback)>>,
    pending: Arc<Mutex<Vec<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe(&mut self, event_type: EventType, callback: EventCallback, priority: i32) {
        if !self.listeners.contains_key(&event_type) {
            self.listeners.insert(event_type, Vec::new());
        }

        self.listeners
            .get_mut(&event_type)
            .unwrap()
            .push((priority, callback));

        self.listeners
            .get_mut(&event_type)
            .unwrap()
            .sort_by_key(|(p, _)| -p);
    }

    pub fn unsubscribe(&mut self, event_type: EventType) {
        self.listeners.remove(&event_type);
    }

    pub fn publish(&self, event: Event) {
        self.pending.lock().push(event);
    }

    pub fn dispatch(&mut self) {
        let events: Vec<Event> = self.pending.lock().drain(..).collect();

        for event in events {
            self.dispatch_event(&event);
        }
    }

    pub fn dispatch_event(&self, event: &Event) {
        if let Some(listeners) = self.listeners.get(&event.event_type) {
            for (_, callback) in listeners {
                callback(event);
            }
        }
    }

    pub fn dispatch_pre_update(&mut self) {
        self.dispatch_event(&Event::new(EventType::PreUpdate, 0));
        self.dispatch();
    }

    pub fn dispatch_post_update(&mut self) {
        self.dispatch_event(&Event::new(EventType::PostUpdate, 0));
        self.dispatch();
    }

    pub fn dispatch_entity_created(&mut self, entity_id: usize) {
        self.publish(Event::new(EventType::EntityCreated, entity_id));
    }

    pub fn dispatch_entity_destroyed(&mut self, entity_id: usize) {
        self.publish(Event::new(EventType::EntityDestroyed, entity_id));
    }

    pub fn clear(&mut self) {
        self.listeners.clear();
        self.pending.lock().clear();
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
