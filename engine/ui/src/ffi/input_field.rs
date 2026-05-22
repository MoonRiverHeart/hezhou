use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_input_field(
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
        let mut input_field = crate::widgets::InputField::new();
        input_field.set_layout(Layout::new(0.0, 0.0, width, height));
        
        let content_scale = crate::thunk::ui_get_content_scale();
        input_field.set_content_scale(content_scale);
        
        let id = input_field.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(input_field), parent);
        dfx_debug!("FFI", "CreateInputField: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_set_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    text: *const c_char,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "InputField" {
                use crate::widgets::InputField;
                if let Some(input_field) = widget.as_any_mut().downcast_mut::<InputField>() {
                    let text_str = CStr::from_ptr(text).to_string_lossy();
                    input_field.set_text(&text_str);
                    dfx_debug!("FFI", "InputFieldSetText: widget_id={}, text_len={}", widget_id, text_str.len());
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_get_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if handle.is_null() || buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "InputField" {
                use crate::widgets::InputField;
                if let Some(input_field) = widget.as_any().downcast_ref::<InputField>() {
                    let text = input_field.text();
                    let copy_len = text.len().min(buffer_size - 1);
                    std::ptr::copy_nonoverlapping(
                        text.as_ptr(),
                        buffer as *mut u8,
                        copy_len,
                    );
                    *buffer.add(copy_len) = 0;
                    return copy_len;
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_set_on_change_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::InputFieldChangeCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_input_field_change_callback(widget_id, callback);
    dfx_debug!("FFI", "InputFieldSetOnChangeThunkPtr: widget_id={}", widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_set_placeholder(
    handle: WidgetTreeHandle,
    widget_id: u64,
    placeholder: *const c_char,
) {
    if handle.is_null() || placeholder.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "InputField" {
                use crate::widgets::InputField;
                if let Some(input_field) = widget.as_any_mut().downcast_mut::<InputField>() {
                    let placeholder_str = CStr::from_ptr(placeholder).to_string_lossy();
                    input_field.set_placeholder(&placeholder_str);
                }
            }
        }
    }
}