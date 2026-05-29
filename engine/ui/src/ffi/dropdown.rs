use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_dropdown(
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
        let mut dropdown = crate::widgets::Dropdown::new();
        dropdown.set_layout(Layout::new(0.0, 0.0, width, height));
        
        let content_scale = crate::thunk::ui_get_content_scale();
        dropdown.set_content_scale(content_scale);
        
        let id = dropdown.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(dropdown), parent);
        dfx_debug!("FFI", "CreateDropdown: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_set_options(
    handle: WidgetTreeHandle,
    widget_id: u64,
    options_ptr: *const c_char,
    options_count: usize,
) {
    if handle.is_null() || options_ptr.is_null() || options_count == 0 {
        return;
    }
    unsafe {
        dfx_info!("FFI", "DropdownSetOptions ENTER: widget_id={}, options_count={}", widget_id, options_count);
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dropdown" {
                use crate::widgets::Dropdown;
                if let Some(dropdown) = widget.as_any_mut().downcast_mut::<Dropdown>() {
// CStr::from_ptr读到第一个\0就停止，但options用\0分隔多个字符串
                    // 需要遍历options_ptr，逐个读\0分隔的字符串
                    let mut options = Vec::new();
                    let mut offset = 0isize;
                    for i in 0..options_count {
                        let ptr = options_ptr.add(offset as usize);
                        let cstr = CStr::from_ptr(ptr);
                        let s = cstr.to_string_lossy().into_owned();
                        let bytes_len = cstr.to_bytes().len();
                        dfx_info!("FFI", "DropdownSetOptions parse[{}]: offset={}, s={}, bytes_len={}", i, offset, s, bytes_len);
                        if !s.is_empty() {
                            options.push(s);
                        }
                        offset += bytes_len as isize + 1; // +1跳过\0分隔符
                    }
                    let count = options.len();
                    let options_display = options.join(",");
                    dropdown.set_options(options);
                    dfx_info!("FFI", "DropdownSetOptions: widget_id={}, count={}, options={}", widget_id, count, options_display);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_set_selected(
    handle: WidgetTreeHandle,
    widget_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dropdown" {
                use crate::widgets::Dropdown;
                if let Some(dropdown) = widget.as_any_mut().downcast_mut::<Dropdown>() {
                    dropdown.set_selected(index);
                    dfx_debug!("FFI", "DropdownSetSelected: widget_id={}, index={}", widget_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_get_selected(
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
            if widget.widget_type() == "Dropdown" {
                use crate::widgets::Dropdown;
                if let Some(dropdown) = widget.as_any().downcast_ref::<Dropdown>() {
                    return dropdown.selected_index();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::DropdownSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_dropdown_select_callback(widget_id, callback);
    dfx_debug!("FFI", "DropdownSetOnSelectThunkPtr: widget_id={}", widget_id);
}