use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_text_edit(
    handle: WidgetTreeHandle,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let text_edit = TextEdit::with_size(width, height);
        let id = text_edit.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(text_edit), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_text_edit_in_parent(
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
        let mut text_edit = TextEdit::with_size(width, height);
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let font_size = 16.0 * content_scale;
        dfx_info!("FFI", "CreateTextEdit: content_scale={}, font_size={}", content_scale, font_size);
        text_edit.set_font_size(font_size);
        
        let id = text_edit.id();
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        tree.add_widget(Box::new(text_edit), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_text_edit_show_line_numbers(
    handle: WidgetTreeHandle,
    widget_id: u64,
    show: bool,
) {
    if handle.is_null() || widget_id == 0 {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                text_edit.set_show_line_numbers(show);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_set_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    text: *const std::ffi::c_char,
) {
    if handle.is_null() || text.is_null() {
        dfx_info!("FFI", "ui_text_edit_set_text: handle or text is null");
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        dfx_info!("FFI", "ui_text_edit_set_text: widget_id={}, looking for widget", widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            dfx_info!("FFI", "Found widget, type={}", widget.widget_type());
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                    let text_str = std::ffi::CStr::from_ptr(text).to_string_lossy();
                    dfx_info!("FFI", "Setting text: {} chars, font_size={}", text_str.len(), text_edit.get_text_style().font_size);
                    text_edit.set_text(&text_str);
                    dfx_info!("FFI", "✓ Text set successfully");
                }
            }
        } else {
            dfx_info!("FFI", "Widget not found for id={}", widget_id);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_insert_char(
    handle: WidgetTreeHandle,
    widget_id: u64,
    c: std::ffi::c_char,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                    if c != 0 {
                        text_edit.insert_char(c as u8 as char);
                    }
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_delete_char(
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
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                    text_edit.delete_char();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_get_text_len(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any().downcast_ref::<TextEdit>() {
                    return text_edit.get_text().len();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_get_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    buffer: *mut std::ffi::c_char,
    buffer_size: usize,
) {
    if handle.is_null() || buffer.is_null() || buffer_size == 0 {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any().downcast_ref::<TextEdit>() {
                    let text = text_edit.get_text();
                    let copy_len = text.len().min(buffer_size - 1);
                    std::ptr::copy_nonoverlapping(
                        text.as_ptr(),
                        buffer as *mut u8,
                        copy_len,
                    );
                    *buffer.add(copy_len) = 0;
                }
            }
        }
    }
}