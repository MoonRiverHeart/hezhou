use hezhou_dfx::*;

use super::types::*;
use super::UI_CALLBACKS;

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_update_callback(callback: UpdateCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.update = Some(callback);
    dfx_info!("UI", "注册Update回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_onclick_callback(widget_id: u64, callback: WidgetCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.onclicks.insert(widget_id, callback);
    dfx_info!("UI", "注册OnClick回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_init_callback(callback: InitCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_init = Some(callback);
    dfx_info!("UI", "注册Init回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_resize_callback(callback: ResizeCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_resize = Some(callback);
    dfx_info!("UI", "注册Resize回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_global_click_callback(callback: GlobalClickCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_global_click = Some(callback);
    dfx_info!("UI", "注册GlobalClick回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_key_callback(callback: KeyCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_key = Some(callback);
    dfx_info!("UI", "注册Key回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_mouse_move_callback(callback: MouseMoveCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_mouse_move = Some(callback);
    dfx_info!("UI", "注册MouseMove回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_dropdown_select_callback(widget_id: u64, callback: DropdownSelectCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_dropdown_select.insert(widget_id, callback);
    dfx_info!("UI", "注册DropdownSelect回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_input_field_change_callback(widget_id: u64, callback: InputFieldChangeCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_input_field_change.insert(widget_id, callback);
    dfx_info!("UI", "注册InputFieldChange回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_tab_select_callback(widget_id: u64, callback: TabSelectCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_tab_select.insert(widget_id, callback);
    dfx_info!("UI", "注册TabSelect回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_tab_close_callback(widget_id: u64, callback: TabCloseCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_tab_close.insert(widget_id, callback);
    dfx_info!("UI", "注册TabClose回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_tree_node_select_callback(widget_id: u64, callback: TreeNodeSelectCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_tree_node_select.insert(widget_id, callback);
    dfx_info!("UI", "注册TreeNodeSelect回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_tree_node_toggle_callback(widget_id: u64, callback: TreeNodeToggleCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_tree_node_toggle.insert(widget_id, callback);
    dfx_info!("UI", "注册TreeNodeToggle回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_popup_menu_click_callback(widget_id: u64, callback: PopupMenuClickCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_popup_menu_click.insert(widget_id, callback);
    dfx_info!("UI", "注册PopupMenuClick回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_popup_menu_close_callback(widget_id: u64, callback: PopupMenuCloseCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_popup_menu_close.insert(widget_id, callback);
    dfx_info!("UI", "注册PopupMenuClose回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_grid_view_click_callback(widget_id: u64, callback: GridViewClickCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_grid_view_click.insert(widget_id, callback);
    dfx_info!("UI", "注册GridViewClick回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_dialog_result_callback(widget_id: u64, callback: DialogResultCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_dialog_result.insert(widget_id, callback);
    dfx_info!("UI", "注册DialogResult回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_file_browser_select_callback(widget_id: u64, callback: FileBrowserSelectCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_file_browser_select.insert(widget_id, callback);
    dfx_info!("UI", "注册FileBrowserSelect回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_file_browser_double_click_callback(widget_id: u64, callback: FileBrowserDoubleClickCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_file_browser_double_click.insert(widget_id, callback);
    dfx_info!("UI", "注册FileBrowserDoubleClick回调: widget={} callback={:?}", widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_focus_change_callback(callback: FocusChangeCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_focus_change = Some(callback);
    dfx_info!("UI", "注册FocusChange回调: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_entity_selected_callback(callback: EntitySelectedCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_entity_selected = Some(callback);
    dfx_info!("UI", "注册EntitySelected回调: {:?}", callback);
}