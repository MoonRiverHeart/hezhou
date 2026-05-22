use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_preview_window(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    texture_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let mut preview = crate::widgets::PreviewWindow::new(texture_id);
        preview.set_layout(Layout::new(x, y, width, height));
        
        let id = preview.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(preview), parent);
        dfx_debug!("FFI", "CreatePreviewWindow: id={}, texture_id={}", id.id, texture_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_preview_texture(
    handle: WidgetTreeHandle,
    widget_id: u64,
    texture_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if let Some(preview) = widget.as_any_mut().downcast_mut::<crate::widgets::PreviewWindow>() {
                preview.set_texture_id(texture_id);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_is_preview_window_selected(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> bool {
    if handle.is_null() {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if let Some(preview) = widget.as_any().downcast_ref::<crate::widgets::PreviewWindow>() {
                return preview.is_selected();
            }
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_preview_window_selected(
    handle: WidgetTreeHandle,
    widget_id: u64,
    selected: bool,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if let Some(preview) = widget.as_any_mut().downcast_mut::<crate::widgets::PreviewWindow>() {
                preview.set_selected(selected);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_preview_window_edit_mode(
    handle: WidgetTreeHandle,
    widget_id: u64,
    edit_mode: bool,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if let Some(preview) = widget.as_any_mut().downcast_mut::<crate::widgets::PreviewWindow>() {
                preview.set_edit_mode(edit_mode);
                dfx_debug!("FFI", "PreviewWindow edit_mode set to: {}", edit_mode);
            }
        }
    }
}