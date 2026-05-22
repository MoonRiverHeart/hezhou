use super::types::*;
use super::global::SCREEN_SIZE;
use super::UI_CALLBACKS;
use hezhou_dfx::*;

pub fn trigger_update_callback(delta_time: f32) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.update
    };
    if let Some(cb) = callback {
        cb(delta_time);
    }
}

pub fn trigger_onclick_callback(widget_id: u64) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.onclicks.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id);
    }
}

pub fn trigger_init_callback() {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_init
    };
    if let Some(cb) = callback {
        cb();
    }
}

pub fn trigger_resize_callback(width: f32, height: f32) {
    let mut size = SCREEN_SIZE.lock();
    *size = (width, height);

    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_resize
    };
    if let Some(cb) = callback {
        cb(width, height);
    }
}

pub fn ui_trigger_key_event(keycode: u32, pressed: bool, modifiers: u32) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_key
    };
    if let Some(callback) = callback {
        callback(keycode, pressed, modifiers);
    }
}

pub fn ui_trigger_mouse_move_event(x: f32, y: f32, dragging: bool) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_mouse_move
    };
    if let Some(callback) = callback {
        callback(x, y, dragging);
    }
}

pub fn ui_trigger_global_click(x: f32, y: f32) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_global_click
    };
    if let Some(callback) = callback {
        callback(x, y);
    }
}

pub fn trigger_dropdown_select_callback(widget_id: u64, index: usize) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_dropdown_select.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id, index);
    }
}

pub fn trigger_input_field_change_callback(widget_id: u64, text: &str) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_input_field_change.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        let text_cstr = std::ffi::CString::new(text).unwrap();
        cb(widget_id, text_cstr.as_ptr());
    }
}

pub fn trigger_tab_select_callback(widget_id: u64, index: usize) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_tab_select.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id, index);
    }
}

pub fn trigger_tab_close_callback(widget_id: u64, index: usize) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_tab_close.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id, index);
    }
}

pub fn trigger_tree_node_select_callback(widget_id: u64, user_data: u64) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_tree_node_select.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id, user_data);
    }
}

pub fn trigger_tree_node_toggle_callback(widget_id: u64) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_tree_node_toggle.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id);
    }
}

pub fn trigger_popup_menu_click_callback(widget_id: u64, action_id: usize) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_popup_menu_click.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id, action_id);
    }
}

pub fn trigger_popup_menu_close_callback(widget_id: u64) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_popup_menu_close.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id);
    }
}

pub fn trigger_grid_view_click_callback(widget_id: u64, index: usize, user_data: u64) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        let cb = callbacks.on_grid_view_click.get(&widget_id).copied();
        dfx_info!("Thunk", "GridViewClick dispatch: widget_id={} index={} user_data={} callback_found={}", 
            widget_id, index, user_data, cb.is_some());
        cb
    };
    if let Some(cb) = callback {
        dfx_info!("Thunk", "GridViewClick invoking C# callback");
        cb(widget_id, index, user_data);
        dfx_info!("Thunk", "GridViewClick C# callback returned");
    } else {
        dfx_warn!("Thunk", "GridViewClick: NO callback registered for widget_id={}", widget_id);
    }
}

pub fn trigger_dialog_result_callback(dialog_id: u64, result: i32) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_dialog_result.get(&dialog_id).copied()
    };
    if let Some(cb) = callback {
        cb(dialog_id, result);
    }
}

pub fn trigger_file_browser_select_callback(browser_id: u64, path: &str) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_file_browser_select.get(&browser_id).copied()
    };
    if let Some(cb) = callback {
        let path_cstr = std::ffi::CString::new(path).unwrap();
        cb(browser_id, path_cstr.as_ptr());
    }
}

pub fn trigger_file_browser_double_click_callback(browser_id: u64, path: &str) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_file_browser_double_click.get(&browser_id).copied()
    };
    if let Some(cb) = callback {
        let path_cstr = std::ffi::CString::new(path).unwrap();
        cb(browser_id, path_cstr.as_ptr());
    }
}

pub fn trigger_focus_change_callback(widget_id: u64, is_focused: bool) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_focus_change
    };
    if let Some(cb) = callback {
        cb(widget_id, is_focused);
    }
}

pub fn trigger_entity_selected_callback(entity_id: u64, is_selected: bool) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_entity_selected
    };
    if let Some(cb) = callback {
        cb(entity_id, is_selected);
    }
}