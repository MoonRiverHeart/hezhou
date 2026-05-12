pub mod bus;
pub mod dispatcher;
pub mod queue;

pub use bus::*;
pub use dispatcher::*;
pub use queue::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventType {
    PreUpdate = 0,
    PostUpdate = 1,
    EntityCreated = 2,
    EntityDestroyed = 3,
    ComponentAdded = 4,
    ComponentRemoved = 5,
    Custom = 100,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub event_type: EventType,
    pub data: usize,
    pub timestamp: f64,
}

impl Event {
    pub fn new(event_type: EventType, data: usize) -> Self {
        Self {
            event_type,
            data,
            timestamp: 0.0,
        }
    }
}

pub type EventCallback = extern "C" fn(&Event);