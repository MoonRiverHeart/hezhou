use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_file_browser(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    initial_path: *const c_char,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let initial_path_str = if initial_path.is_null() {
            ".".to_string()
        } else {
            CStr::from_ptr(initial_path).to_string_lossy().into_owned()
        };
        
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let mut file_browser = crate::widgets::FileBrowser::new()
            .with_initial_path(&initial_path_str)
            .with_layout(x, y, width, height);
        file_browser.set_content_scale(content_scale);
        
        let id = file_browser.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(file_browser), parent);
        dfx_info!("FFI", "CreateFileBrowser: id={}, parent={}, path={}, scale={}", id.id, parent_id, initial_path_str, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_path(
    handle: WidgetTreeHandle,
    browser_id: u64,
    path: *const c_char,
) {
    if handle.is_null() || path.is_null() {
        return;
    }
    unsafe {
        let path_str = CStr::from_ptr(path).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.set_path(&path_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_filter(
    handle: WidgetTreeHandle,
    browser_id: u64,
    filter: *const c_char,
) {
    if handle.is_null() || filter.is_null() {
        return;
    }
    unsafe {
        let filter_str = CStr::from_ptr(filter).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.set_filter(&filter_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_navigate_up(
    handle: WidgetTreeHandle,
    browser_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.navigate_up();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_refresh(
    handle: WidgetTreeHandle,
    browser_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.refresh();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_get_selected_path(
    handle: WidgetTreeHandle,
    browser_id: u64,
    buffer: *mut c_char,
    size: usize,
) -> bool {
    if handle.is_null() || buffer.is_null() || size == 0 {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any().downcast_ref::<FileBrowser>() {
                    if let Some(path) = file_browser.get_selected_path() {
                        let copy_len = path.len().min(size - 1);
                        std::ptr::copy_nonoverlapping(
                            path.as_ptr(),
                            buffer as *mut u8,
                            copy_len,
                        );
                        *buffer.add(copy_len) = 0;
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_get_current_path(
    handle: WidgetTreeHandle,
    browser_id: u64,
    buffer: *mut c_char,
    size: usize,
) -> bool {
    if handle.is_null() || buffer.is_null() || size == 0 {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any().downcast_ref::<FileBrowser>() {
                    let path = file_browser.current_path();
                    let copy_len = path.len().min(size - 1);
                    std::ptr::copy_nonoverlapping(
                        path.as_ptr(),
                        buffer as *mut u8,
                        copy_len,
                    );
                    *buffer.add(copy_len) = 0;
                    return true;
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    browser_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::FileBrowserSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_file_browser_select_callback(browser_id, callback);
    dfx_info!("FFI", "FileBrowserSetOnSelectThunkPtr: browser_id={}", browser_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_on_double_click_thunk_ptr(
    handle: WidgetTreeHandle,
    browser_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::FileBrowserDoubleClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_file_browser_double_click_callback(browser_id, callback);
    dfx_info!("FFI", "FileBrowserSetOnDoubleClickThunkPtr: browser_id={}", browser_id);
}