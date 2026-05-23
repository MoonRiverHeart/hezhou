use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;
use super::EventDispatcherHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_layout(
    handle: WidgetTreeHandle,
    widget_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            widget.set_layout(Layout::new(x, y, width, height));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_background_color(
    handle: WidgetTreeHandle,
    widget_id: u64,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let new_style = widget
                .style()
                .clone()
                .with_background(Color::new(r, g, b, a));
            widget.set_style(new_style);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_x(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().x).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_y(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().y).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_width(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().width).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_height(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().height).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_position(handle: WidgetTreeHandle, widget_id: u64, x: f32, y: f32) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let layout = widget.layout();
            widget.set_layout(Layout::new(x, y, layout.width, layout.height));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_size(handle: WidgetTreeHandle, widget_id: u64, width: f32, height: f32) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let layout = widget.layout();
            widget.set_layout(Layout::new(layout.x, layout.y, width, height));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_widget_layout(
    handle: WidgetTreeHandle,
    widget_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            widget.set_layout(Layout::new(x, y, width, height));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_content_scale(scale: f32) {
    crate::thunk::ui_set_content_scale(scale);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_content_scale() -> f32 {
    crate::thunk::ui_get_content_scale()
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_screen_size(width: f32, height: f32) {
    crate::thunk::ui_set_screen_size(width, height);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_screen_size(out_width: *mut f32, out_height: *mut f32) {
    let (w, h) = crate::thunk::ui_get_screen_size();
    unsafe {
        if !out_width.is_null() {
            *out_width = w;
        }
        if !out_height.is_null() {
            *out_height = h;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_primary_button_id(id: u64) {
    crate::thunk::ui_set_primary_button_id(id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_primary_button_id() -> u64 {
    crate::thunk::ui_get_primary_button_id()
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_remove_widget(
    handle: WidgetTreeHandle,
    widget_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.remove_widget(id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_root_id(handle: WidgetTreeHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        tree.root.map(|r| r.id).unwrap_or(0)
    }
}

pub extern "C" fn ui_clear_widget_tree(handle: WidgetTreeHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        tree.clear();
    }
    crate::thunk::ui_clear_callbacks();
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_resize_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: ResizeCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_resize_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_global_click_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::GlobalClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_global_click_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_key_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::KeyCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_key_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_mouse_move_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::MouseMoveCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_mouse_move_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_mouse_wheel_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::MouseWheelCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_mouse_wheel_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_tree_node_right_click_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::TreeNodeRightClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_tree_node_right_click_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_resize(width: f32, height: f32) {
    crate::thunk::trigger_resize_callback(width, height);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_update_thunk(callback: UpdateCallback) {
    ui_register_update_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_update_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: UpdateCallback = unsafe { std::mem::transmute(callback_ptr) };
    ui_register_update_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_init_thunk(callback: InitCallback) {
    ui_register_init_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_init_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: InitCallback = unsafe { std::mem::transmute(callback_ptr) };
    ui_register_init_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_update(delta_time: f32) {
    trigger_update_callback(delta_time);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_onclick(widget_id: u64) {
    trigger_onclick_callback(widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_init() {
    trigger_init_callback();
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_has_onclick(widget_id: u64) -> bool {
    has_onclick_callback(widget_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_layer(
    handle: WidgetTreeHandle,
    widget_id: u64,
    layer: u32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        let render_layer = match layer {
            0 => crate::widget_tree::RenderLayer::Background,
            1 => crate::widget_tree::RenderLayer::Content,
            2 => crate::widget_tree::RenderLayer::Popup,
            3 => crate::widget_tree::RenderLayer::Overlay,
            _ => crate::widget_tree::RenderLayer::Content,
        };
        tree.set_widget_layer(id, render_layer);
        dfx_debug!("FFI", "WidgetSetLayer: widget_id={}, layer={}", widget_id, layer);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_layer(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> u32 {
    if handle.is_null() {
        return 1;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget_layer(id) as u32
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_debug_print_widget_tree(handle: WidgetTreeHandle) {
    if handle.is_null() { return; }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        tree.debug_print_tree();
    }
}

// ========== Automated UI Testing FFI ==========

/// Simulate a click at (x, y) through the full event pipeline:
/// TouchBegin → hit_test → widget on_event → gesture recognition → TouchEnd
/// Returns the widget_id that was hit (0 if nothing hit)
#[unsafe(no_mangle)]
pub extern "C" fn ui_simulate_click_at(
    handle: WidgetTreeHandle,
    event_dispatcher_handle: EventDispatcherHandle,
    x: f32,
    y: f32,
) -> u64 {
    let timestamp = 0;
    let hit_id = if handle.is_null() {
        0
    } else {
        unsafe {
            let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
            let tree = arc.lock();
            tree.hit_test(Point::new(x, y)).map(|id| id.id).unwrap_or(0)
        }
    };
    
    if !event_dispatcher_handle.is_null() {
        // TouchBegin (press)
        crate::ffi::event::ui_event_dispatcher_dispatch_touch_begin(
            event_dispatcher_handle, x, y, 0, timestamp,
        );
        // TouchEnd (release) — triggers gesture recognition
        crate::ffi::event::ui_event_dispatcher_dispatch_touch_end(
            event_dispatcher_handle, x, y, 0, timestamp,
        );
    }
    
    // Flush any pending callbacks triggered by the click
    crate::thunk::flush_pending_callbacks();
    
    hit_id
}

/// Get widget type string as a null-terminated C string written into a caller-provided buffer.
/// Returns the number of bytes written (excluding null terminator).
/// If widget_id is invalid or buffer too small, returns 0.
#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_type(
    handle: WidgetTreeHandle,
    widget_id: u64,
    buf: *mut u8,
    buf_len: u32,
) -> u32 {
    if handle.is_null() || buf.is_null() || buf_len == 0 {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        let widget_type = tree.get_widget(id)
            .map(|w| w.widget_type())
            .unwrap_or("");
        let type_bytes = widget_type.as_bytes();
        let write_len = std::cmp::min(type_bytes.len(), buf_len as usize - 1);
        std::ptr::copy_nonoverlapping(type_bytes.as_ptr(), buf, write_len);
        *buf.add(write_len) = 0; // null terminator
        write_len as u32
    }
}

/// Get widget layout (x, y, width, height) packed into a float array.
/// Caller provides a float[4] buffer. Returns 1 if widget found, 0 if not.
#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_layout(
    handle: WidgetTreeHandle,
    widget_id: u64,
    out_layout: *mut f32, // must point to float[4]
) -> u32 {
    if handle.is_null() || out_layout.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            let layout = widget.layout();
            *out_layout = layout.x;
            *out_layout.add(1) = layout.y;
            *out_layout.add(2) = layout.width;
            *out_layout.add(3) = layout.height;
            1
        } else {
            0
        }
    }
}

/// Get the parent widget ID. Returns 0 if widget has no parent or is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_parent(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_parent(id).map(|p| p.id).unwrap_or(0)
    }
}

/// Get the number of children of a widget. Returns 0 if widget is invalid.
#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_child_count(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_children(id).len() as u32
    }
}

/// Get the ID of a specific child by index. Returns 0 if index out of bounds or widget invalid.
#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_child_id(
    handle: WidgetTreeHandle,
    widget_id: u64,
    index: u32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        let children = tree.get_children(id);
            if (index as usize) < children.len() {
                children[index as usize].id
            } else {
                0
            }
    }
}

/// Get the total number of widgets in the tree.
#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_total_count(
    handle: WidgetTreeHandle,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        tree.get_all_widget_ids().len() as u32
    }
}

/// Dump the entire UI tree to a caller-provided buffer as a null-terminated string.
/// Each line: "[type] id=X pos=(x,y) size=(w,h) layer=L"
/// Returns number of bytes written (excluding null terminator). If buffer too small, returns 0.
#[unsafe(no_mangle)]
pub extern "C" fn ui_debug_dump_tree_to_buffer(
    handle: WidgetTreeHandle,
    buf: *mut u8,
    buf_len: u32,
) -> u32 {
    if handle.is_null() || buf.is_null() || buf_len == 0 {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let mut output = String::new();
        if let Some(root) = tree.root() {
            tree.dump_node_to_string(root, 0, &mut output);
        }
        let bytes = output.as_bytes();
        let write_len = std::cmp::min(bytes.len(), buf_len as usize - 1);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, write_len);
        *buf.add(write_len) = 0;
        write_len as u32
    }
}