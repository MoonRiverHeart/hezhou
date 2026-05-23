use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_split_view(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    orientation: u32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let orient = if orientation == 1 {
            crate::widgets::SplitOrientation::Vertical
        } else {
            crate::widgets::SplitOrientation::Horizontal
        };
        let mut split_view = crate::widgets::SplitView::new(width, height, orient);
        split_view.set_layout(Layout::new(x, y, width, height));

        let content_scale = crate::thunk::ui_get_content_scale();
        split_view.set_content_scale(content_scale);

        let id = split_view.id();

        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };

        tree.add_widget(Box::new(split_view), parent);
        dfx_debug!("FFI", "CreateSplitView: id={}, parent={}, orientation={}", id.id, parent_id, orientation);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_split_view_set_split_ratio(
    handle: WidgetTreeHandle,
    widget_id: u64,
    ratio: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "SplitView" {
                use crate::widgets::SplitView;
                if let Some(sv) = widget.as_any_mut().downcast_mut::<SplitView>() {
                    sv.set_split_ratio(ratio);
                    dfx_debug!("FFI", "SplitViewSetSplitRatio: widget_id={}, ratio={}", widget_id, ratio);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_split_view_get_split_ratio(
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
            if widget.widget_type() == "SplitView" {
                use crate::widgets::SplitView;
                if let Some(sv) = widget.as_any().downcast_ref::<SplitView>() {
                    return sv.get_split_ratio();
                }
            }
        }
        0.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_split_view_set_min_ratio(
    handle: WidgetTreeHandle,
    widget_id: u64,
    min: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "SplitView" {
                use crate::widgets::SplitView;
                if let Some(sv) = widget.as_any_mut().downcast_mut::<SplitView>() {
                    sv.set_min_ratio(min);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_split_view_set_max_ratio(
    handle: WidgetTreeHandle,
    widget_id: u64,
    max: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "SplitView" {
                use crate::widgets::SplitView;
                if let Some(sv) = widget.as_any_mut().downcast_mut::<SplitView>() {
                    sv.set_max_ratio(max);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_split_view_set_on_ratio_change_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::SplitViewRatioChangeCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_split_view_ratio_change_callback(widget_id, callback);
    dfx_debug!("FFI", "SplitViewSetOnRatioChangeThunkPtr: widget_id={}", widget_id);
}