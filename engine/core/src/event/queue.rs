use crate::event::*;

pub struct DelayedEventQueue {
    events: Vec<(Event, f64)>,
}

impl DelayedEventQueue {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }
    
    pub fn push(&mut self, event: Event, delay_seconds: f64) {
        self.events.push((event, delay_seconds));
    }
    
    pub fn update(&mut self, current_time: f64, bus: &mut EventBus) {
        let ready_events: Vec<Event> = self.events.iter()
            .filter_map(|(event, delay)| {
                if event.timestamp + *delay <= current_time {
                    Some(event.clone())
                } else {
                    None
                }
            })
            .collect();
        
        self.events.retain(|(event, delay)| {
            event.timestamp + *delay > current_time
        });
        
        for event in ready_events {
            bus.publish(event);
        }
    }
    
    pub fn clear(&mut self) {
        self.events.clear();
    }
    
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

impl Default for DelayedEventQueue {
    fn default() -> Self {
        Self::new()
    }
}