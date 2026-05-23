use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_scroll_view(
    handle: WidgetTreeHandle,
    parent_id: u64,
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
        let mut scroll_view = crate::widgets::ScrollView::new(width, height);
        scroll_view.set_layout(Layout::new(x, y, width, height));

        let content_scale = crate::thunk::ui_get_content_scale();
        scroll_view.set_content_scale(content_scale);

        let id = scroll_view.id();

        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };

        tree.add_widget(Box::new(scroll_view), parent);
        dfx_debug!("FFI", "CreateScrollView: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_scroll_view_set_scroll_offset(
    handle: WidgetTreeHandle,
    widget_id: u64,
    offset_y: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "ScrollView" {
                use crate::widgets::ScrollView;
                if let Some(sv) = widget.as_any_mut().downcast_mut::<ScrollView>() {
                    sv.set_scroll_offset_y(offset_y);
                    dfx_debug!("FFI", "ScrollViewSetScrollOffset: widget_id={}, offset_y={}", widget_id, offset_y);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_scroll_view_get_scroll_offset(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "ScrollView" {
                use crate::widgets::ScrollView;
                if let Some(sv) = widget.as_any().downcast_ref::<ScrollView>() {
                    return sv.get_scroll_offset_y();
                }
            }
        }
        0.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_scroll_view_set_show_scrollbars(
    handle: WidgetTreeHandle,
    widget_id: u64,
    show_v: u32,
    show_h: u32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "ScrollView" {
                use crate::widgets::ScrollView;
                if let Some(sv) = widget.as_any_mut().downcast_mut::<ScrollView>() {
                    sv.set_show_scrollbars(show_v != 0, show_h != 0);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_scroll_view_set_on_scroll_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::ScrollViewScrollCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_scroll_view_scroll_callback(widget_id, callback);
    dfx_debug!("FFI", "ScrollViewSetOnScrollThunkPtr: widget_id={}", widget_id);
}