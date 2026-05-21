use parking_lot::Mutex;
use std::sync::LazyLock;

use super::trigger::*;

pub enum PendingCallback {
    GridViewClick { widget_id: u64, index: usize, user_data: u64 },
    PopupMenuClick { widget_id: u64, action_id: usize },
    PopupMenuClose { widget_id: u64 },
    TreeNodeSelect { widget_id: u64, user_data: u64 },
    TreeNodeToggle { widget_id: u64 },
    DropdownSelect { widget_id: u64, index: usize },
    TabSelect { widget_id: u64, index: usize },
    TabClose { widget_id: u64, index: usize },
    ButtonClick { widget_id: u64 },
    InputFieldChange { widget_id: u64, text: String },
    DialogResult { dialog_id: u64, result: i32 },
    FileBrowserSelect { browser_id: u64, path: String },
    FileBrowserDoubleClick { browser_id: u64, path: String },
}

static PENDING_CALLBACKS: LazyLock<Mutex<Vec<PendingCallback>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub fn queue_callback(callback: PendingCallback) {
    PENDING_CALLBACKS.lock().push(callback);
}

pub fn flush_pending_callbacks() {
    let callbacks: Vec<PendingCallback> = PENDING_CALLBACKS.lock().drain(..).collect();
    for cb in callbacks {
        match cb {
            PendingCallback::GridViewClick { widget_id, index, user_data } => {
                trigger_grid_view_click_callback(widget_id, index, user_data);
            }
            PendingCallback::PopupMenuClick { widget_id, action_id } => {
                trigger_popup_menu_click_callback(widget_id, action_id);
            }
            PendingCallback::PopupMenuClose { widget_id } => {
                trigger_popup_menu_close_callback(widget_id);
            }
            PendingCallback::TreeNodeSelect { widget_id, user_data } => {
                trigger_tree_node_select_callback(widget_id, user_data);
            }
            PendingCallback::TreeNodeToggle { widget_id } => {
                trigger_tree_node_toggle_callback(widget_id);
            }
            PendingCallback::DropdownSelect { widget_id, index } => {
                trigger_dropdown_select_callback(widget_id, index);
            }
            PendingCallback::TabSelect { widget_id, index } => {
                trigger_tab_select_callback(widget_id, index);
            }
            PendingCallback::ButtonClick { widget_id } => {
                trigger_onclick_callback(widget_id);
            }
            PendingCallback::InputFieldChange { widget_id, text } => {
                trigger_input_field_change_callback(widget_id, &text);
            }
            PendingCallback::TabClose { widget_id, index } => {
                trigger_tab_close_callback(widget_id, index);
            }
            PendingCallback::DialogResult { dialog_id, result } => {
                trigger_dialog_result_callback(dialog_id, result);
            }
            PendingCallback::FileBrowserSelect { browser_id, path } => {
                trigger_file_browser_select_callback(browser_id, &path);
            }
            PendingCallback::FileBrowserDoubleClick { browser_id, path } => {
                trigger_file_browser_double_click_callback(browser_id, &path);
            }
        }
    }
}