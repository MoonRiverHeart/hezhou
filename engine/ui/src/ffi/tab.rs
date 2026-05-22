use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::CStr;
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_tab_widget(
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
        
        let mut tab_widget = crate::widgets::TabWidget::new()
            .with_layout(x, y, width, height);
        
        let content_scale = crate::thunk::ui_get_content_scale();
        tab_widget.set_content_scale(content_scale);
        
        let id = tab_widget.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(tab_widget), parent);
        dfx_debug!("FFI", "CreateTabWidget: id={}, parent={}, scale={}", id.id, parent_id, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_add_tab(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    title: *const std::ffi::c_char,
    content_widget_id: u64,
    closable: bool,
) -> u32 {
    if handle.is_null() || title.is_null() {
        return 0;
    }
    unsafe {
        let title_str = CStr::from_ptr(title).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any_mut().downcast_mut::<TabWidget>() {
                    let content_id = WidgetId::from_raw(content_widget_id);
                    let index = tab_widget.add_tab(&title_str, content_id, closable);
                    dfx_debug!("FFI", "TabWidgetAddTab: widget_id={}, title={}, content={}, closable={}, index={}", 
                        tab_widget_id, title_str, content_widget_id, closable, index);
                    return index as u32;
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_set_active(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any_mut().downcast_mut::<TabWidget>() {
                    tab_widget.set_active(index);
                    dfx_debug!("FFI", "TabWidgetSetActive: widget_id={}, index={}", tab_widget_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_get_active(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any().downcast_ref::<TabWidget>() {
                    return tab_widget.active_index();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_remove_tab(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any_mut().downcast_mut::<TabWidget>() {
                    tab_widget.remove_tab(index);
                    dfx_debug!("FFI", "TabWidgetRemoveTab: widget_id={}, index={}", tab_widget_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::TabSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_tab_select_callback(tab_widget_id, callback);
    dfx_debug!("FFI", "TabWidgetSetOnSelectThunkPtr: widget_id={}", tab_widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_set_on_close_thunk_ptr(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::TabCloseCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_tab_close_callback(tab_widget_id, callback);
    dfx_debug!("FFI", "TabWidgetSetOnCloseThunkPtr: widget_id={}", tab_widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_get_tab_count(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any().downcast_ref::<TabWidget>() {
                    return tab_widget.tab_count();
                }
            }
        }
        0
    }
}