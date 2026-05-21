use crate::*;
use parking_lot::Mutex;
use std::sync::Arc;

use super::EventDispatcherHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_touch_begin(
    handle: EventDispatcherHandle,
    x: f32,
    y: f32,
    pointer_id: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::TouchBegin, timestamp)
            .with_data(EventData::Touch(TouchData::new(x, y, pointer_id)));
        dispatcher.dispatch_event(&mut event);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_touch_end(
    handle: EventDispatcherHandle,
    x: f32,
    y: f32,
    pointer_id: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::TouchEnd, timestamp)
            .with_data(EventData::Touch(TouchData::new(x, y, pointer_id)));
        dispatcher.dispatch_event(&mut event);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_key_down(
    handle: EventDispatcherHandle,
    keycode: u32,
    modifiers: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::KeyDown, timestamp)
            .with_data(EventData::Key(KeyData::new(keycode, modifiers)));
        dispatcher.dispatch_event(&mut event);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_key_up(
    handle: EventDispatcherHandle,
    keycode: u32,
    modifiers: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::KeyUp, timestamp)
            .with_data(EventData::Key(KeyData::new(keycode, modifiers)));
        dispatcher.dispatch_event(&mut event);
    }
}