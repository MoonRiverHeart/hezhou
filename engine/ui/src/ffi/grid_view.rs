use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_grid_view(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    cell_size: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let mut grid_view = crate::widgets::GridView::new()
            .with_cell_size(cell_size * content_scale)
            .with_spacing(8.0 * content_scale);
        grid_view.set_content_scale(content_scale);
        grid_view.set_layout(Layout::new(x, y, width, height));
        
        let id = grid_view.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(grid_view), parent);
        dfx_debug!("FFI", "CreateGridView: id={}, parent={}, cell_size={}, scale={}", id.id, parent_id, cell_size, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_add_item(
    handle: WidgetTreeHandle,
    grid_id: u64,
    label: *const c_char,
    user_data: u64,
) -> u32 {
    if handle.is_null() || label.is_null() {
        return 0;
    }
    unsafe {
        let label_str = CStr::from_ptr(label).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    let index = grid_view.add_item(label_str.clone(), user_data);
                    dfx_debug!("FFI", "GridViewAddItem: grid_id={}, label={}, user_data={}, index={}", grid_id, label_str, user_data, index);
                    return index as u32;
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_remove_item(
    handle: WidgetTreeHandle,
    grid_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    grid_view.remove_item(index);
                    dfx_debug!("FFI", "GridViewRemoveItem: grid_id={}, index={}", grid_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_set_selected(
    handle: WidgetTreeHandle,
    grid_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    grid_view.set_selected(index);
                    dfx_debug!("FFI", "GridViewSetSelected: grid_id={}, index={}", grid_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_get_selected(
    handle: WidgetTreeHandle,
    grid_id: u64,
) -> usize {
    if handle.is_null() {
        return usize::MAX;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any().downcast_ref::<GridView>() {
                    return grid_view.selected_index().unwrap_or(usize::MAX);
                }
            }
        }
        usize::MAX
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_get_selected_user_data(
    handle: WidgetTreeHandle,
    grid_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any().downcast_ref::<GridView>() {
                    return grid_view.selected_user_data().unwrap_or(0);
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_clear(
    handle: WidgetTreeHandle,
    grid_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    grid_view.clear();
                    dfx_debug!("FFI", "GridViewClear: grid_id={}", grid_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_item_count(
    handle: WidgetTreeHandle,
    grid_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any().downcast_ref::<GridView>() {
                    return grid_view.item_count();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_set_on_click_thunk_ptr(
    handle: WidgetTreeHandle,
    grid_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::GridViewClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_grid_view_click_callback(grid_id, callback);
    dfx_debug!("FFI", "GridViewSetOnClickThunkPtr: grid_id={}", grid_id);
}