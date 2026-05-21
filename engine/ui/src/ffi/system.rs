use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

use super::{WidgetTreeHandle, EventDispatcherHandle};

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_create() -> *mut UISystem {
    let system = Box::new(UISystem::new());
    Box::into_raw(system)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_destroy(system: *mut UISystem) {
    if !system.is_null() {
        unsafe {
            let _ = Box::from_raw(system);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_update(system: *mut UISystem, delta_time: f32) {
    if system.is_null() {
        return;
    }
    unsafe {
        (*system).update(delta_time);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_get_widget_tree(system: *const UISystem) -> WidgetTreeHandle {
    if system.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let arc = (*system).get_widget_tree();
        Box::into_raw(Box::new(arc)) as WidgetTreeHandle
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_get_event_dispatcher(system: *const UISystem) -> EventDispatcherHandle {
    if system.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let arc = (*system).get_event_dispatcher();
        Box::into_raw(Box::new(arc)) as EventDispatcherHandle
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_tree_handle_destroy(handle: WidgetTreeHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut Arc<Mutex<WidgetTree>>);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_handle_destroy(handle: EventDispatcherHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut Arc<Mutex<EventDispatcher>>);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_tree_create_root_panel(
    handle: WidgetTreeHandle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut panel = Panel::new();
        panel.set_layout(Layout::new(x, y, width, height));
        tree.set_root(Box::new(panel));
        tree.root.map(|r| r.id).unwrap_or(0)
    }
}