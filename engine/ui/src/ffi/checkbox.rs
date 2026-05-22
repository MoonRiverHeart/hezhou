use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_checkbox(
    handle: WidgetTreeHandle,
    parent_id: u64,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut checkbox = crate::widgets::Checkbox::new("");
        checkbox.set_layout(Layout::new(0.0, 0.0, width, height));

        let content_scale = crate::thunk::ui_get_content_scale();
        checkbox.set_content_scale(content_scale);

        let id = checkbox.id();

        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };

        tree.add_widget(Box::new(checkbox), parent);
        dfx_debug!("FFI", "CreateCheckbox: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_checkbox_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text_ptr: *const c_char,
) -> u64 {
    if handle.is_null() || text_ptr.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text_ptr).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut checkbox = crate::widgets::Checkbox::new(&text_str);
        checkbox.set_layout(Layout::new(x, y, width, height));

        let content_scale = crate::thunk::ui_get_content_scale();
        checkbox.set_content_scale(content_scale);

        let id = checkbox.id();

        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };

        tree.add_widget(Box::new(checkbox), parent);
        dfx_debug!("FFI", "CreateCheckboxInParent: id={}, parent={}, text={}", id.id, parent_id, text_str);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_checkbox_set_checked(
    handle: WidgetTreeHandle,
    widget_id: u64,
    checked: u32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Checkbox" {
                use crate::widgets::Checkbox;
                if let Some(checkbox) = widget.as_any_mut().downcast_mut::<Checkbox>() {
                    checkbox.set_checked(checked != 0);
                    dfx_debug!("FFI", "CheckboxSetChecked: widget_id={}, checked={}", widget_id, checked != 0);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_checkbox_get_checked(
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
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Checkbox" {
                use crate::widgets::Checkbox;
                if let Some(checkbox) = widget.as_any().downcast_ref::<Checkbox>() {
                    return if checkbox.is_checked() { 1 } else { 0 };
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_checkbox_set_on_change_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::CheckboxChangeCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_checkbox_change_callback(widget_id, callback);
    dfx_debug!("FFI", "CheckboxSetOnChangeThunkPtr: widget_id={}", widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_checkbox_set_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    text_ptr: *const c_char,
) {
    if handle.is_null() || text_ptr.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text_ptr).to_string_lossy();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Checkbox" {
                use crate::widgets::Checkbox;
                if let Some(checkbox) = widget.as_any_mut().downcast_mut::<Checkbox>() {
                    checkbox.set_text(&text_str);
                }
            }
        }
    }
}