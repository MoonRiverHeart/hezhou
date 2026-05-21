use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

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
        dfx_info!("FFI", "WidgetSetLayer: widget_id={}, layer={}", widget_id, layer);
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