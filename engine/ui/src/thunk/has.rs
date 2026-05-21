use super::UI_CALLBACKS;

pub fn has_onclick_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.onclicks.contains_key(&widget_id)
}

pub fn has_dropdown_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_dropdown_select.contains_key(&widget_id)
}

pub fn has_input_field_change_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_input_field_change.contains_key(&widget_id)
}

pub fn has_tab_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tab_select.contains_key(&widget_id)
}

pub fn has_tab_close_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tab_close.contains_key(&widget_id)
}

pub fn has_tree_node_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tree_node_select.contains_key(&widget_id)
}

pub fn has_tree_node_toggle_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tree_node_toggle.contains_key(&widget_id)
}

pub fn has_popup_menu_click_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_popup_menu_click.contains_key(&widget_id)
}

pub fn has_grid_view_click_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_grid_view_click.contains_key(&widget_id)
}

pub fn has_dialog_result_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_dialog_result.contains_key(&widget_id)
}

pub fn has_file_browser_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_file_browser_select.contains_key(&widget_id)
}

pub fn has_file_browser_double_click_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_file_browser_double_click.contains_key(&widget_id)
}