use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_dialog(
    handle: WidgetTreeHandle,
    parent_id: u64,
    title: *const c_char,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() || title.is_null() {
        return 0;
    }
    unsafe {
        let title_str = CStr::from_ptr(title).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let screen_size = crate::thunk::ui_get_screen_size();
        let x = (screen_size.0 - width) / 2.0;
        let y = (screen_size.1 - height) / 2.0;
        
        let mut dialog = crate::widgets::Dialog::new()
            .with_title(&title_str)
            .with_size(width, height);
        dialog.set_layout(Layout::new(x, y, width, height));
        dialog.set_content_scale(content_scale);
        
        let id = dialog.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(dialog), parent);
        tree.set_widget_layer(id, crate::widget_tree::RenderLayer::Overlay);
        dfx_debug!("FFI", "CreateDialog: id={}, title={}, size={}x{}, scale={}", id.id, title_str, width, height, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_set_content(
    handle: WidgetTreeHandle,
    dialog_id: u64,
    content_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    dialog.set_content(WidgetId::from_raw(content_id));
                    dfx_debug!("FFI", "DialogSetContent: dialog_id={}, content_id={}", dialog_id, content_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_add_button(
    handle: WidgetTreeHandle,
    dialog_id: u64,
    text: *const c_char,
    action: i32,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                use crate::widgets::dialog::DialogAction;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    let dialog_action = match action {
                        0 => DialogAction::Ok,
                        1 => DialogAction::Cancel,
                        2 => DialogAction::Yes,
                        3 => DialogAction::No,
                        _ => DialogAction::Custom,
                    };
                    dialog.add_button(&text_str, dialog_action);
                    dfx_debug!("FFI", "DialogAddButton: dialog_id={}, text={}, action={}", dialog_id, text_str, action);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_show(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    dialog.show();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_hide(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    dialog.hide();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_is_visible(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) -> bool {
    if handle.is_null() {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any().downcast_ref::<Dialog>() {
                    return dialog.is_visible();
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_get_result(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any().downcast_ref::<Dialog>() {
                    return dialog.result().unwrap_or(-1);
                }
            }
        }
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_set_on_result_thunk_ptr(
    handle: WidgetTreeHandle,
    dialog_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::DialogResultCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_dialog_result_callback(dialog_id, callback);
    dfx_debug!("FFI", "DialogSetOnResultThunkPtr: dialog_id={}", dialog_id);
}