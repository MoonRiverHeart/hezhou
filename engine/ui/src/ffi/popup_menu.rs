use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_popup_menu(
    handle: WidgetTreeHandle,
    parent_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let mut popup_menu = crate::widgets::PopupMenu::new();
        popup_menu.set_content_scale(content_scale);
        
        let id = popup_menu.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(popup_menu), parent);
        tree.set_widget_layer(id, crate::widget_tree::RenderLayer::Popup);
        dfx_info!("FFI", "CreatePopupMenu: id={}, parent={}, scale={}", id.id, parent_id, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_add_item(
    handle: WidgetTreeHandle,
    menu_id: u64,
    text: *const c_char,
    shortcut: *const c_char,
    action_id: usize,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let shortcut_str = if shortcut.is_null() {
            None
        } else {
            Some(CStr::from_ptr(shortcut).to_string_lossy().into_owned())
        };
        
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.add_item(text_str.clone(), shortcut_str, action_id);
                    dfx_info!("FFI", "PopupMenuAddItem: menu_id={}, text={}, action_id={}", menu_id, text_str, action_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_add_separator(
    handle: WidgetTreeHandle,
    menu_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.add_separator();
                    dfx_info!("FFI", "PopupMenuAddSeparator: menu_id={}", menu_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_show(
    handle: WidgetTreeHandle,
    menu_id: u64,
    x: f32,
    y: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.show(x, y);
                    dfx_info!("FFI", "PopupMenuShow: menu_id={}, x={}, y={}", menu_id, x, y);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_hide(
    handle: WidgetTreeHandle,
    menu_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.hide();
                    dfx_info!("FFI", "PopupMenuHide: menu_id={}", menu_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_is_visible(
    handle: WidgetTreeHandle,
    menu_id: u64,
) -> bool {
    if handle.is_null() {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any().downcast_ref::<PopupMenu>() {
                    return popup_menu.is_visible();
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_set_on_click_thunk_ptr(
    handle: WidgetTreeHandle,
    menu_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::PopupMenuClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_popup_menu_click_callback(menu_id, callback);
    dfx_info!("FFI", "PopupMenuSetOnClickThunkPtr: menu_id={}", menu_id);
}