use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::*;

pub type UpdateCallback = extern "C" fn(f32);
pub type WidgetCallback = extern "C" fn(u64);
pub type InitCallback = extern "C" fn();
pub type ResizeCallback = extern "C" fn(f32, f32);
pub type GlobalClickCallback = extern "C" fn(f32, f32);
pub type KeyCallback = extern "C" fn(u32, bool, u32);  // keycode, pressed, modifiers
pub type MouseMoveCallback = extern "C" fn(f32, f32, bool);  // x, y, dragging
pub type DropdownSelectCallback = extern "C" fn(u64, usize);  // widget_id, selected_index
pub type InputFieldChangeCallback = extern "C" fn(u64, *const std::ffi::c_char);  // widget_id, text
pub type TabSelectCallback = extern "C" fn(u64, usize);  // widget_id, tab_index
pub type TabCloseCallback = extern "C" fn(u64, usize);   // widget_id, tab_index
pub type TreeNodeSelectCallback = extern "C" fn(u64, u64);  // widget_id, user_data
pub type TreeNodeToggleCallback = extern "C" fn(u64);  // widget_id
pub type PopupMenuClickCallback = extern "C" fn(u64, usize);  // widget_id, action_id
pub type PopupMenuCloseCallback = extern "C" fn(u64);  // widget_id
pub type GridViewClickCallback = extern "C" fn(u64, usize, u64);  // widget_id, index, user_data
pub type DialogResultCallback = extern "C" fn(u64, i32);  // dialog_id, result
pub type FileBrowserSelectCallback = extern "C" fn(u64, *const std::ffi::c_char);  // browser_id, path
pub type FileBrowserDoubleClickCallback = extern "C" fn(u64, *const std::ffi::c_char);  // browser_id, path

static UI_CALLBACKS: LazyLock<Mutex<UICallbacks>> =
    LazyLock::new(|| Mutex::new(UICallbacks::new()));

static PRIMARY_BUTTON_ID: LazyLock<Mutex<Option<u64>>> =
    LazyLock::new(|| Mutex::new(None));

static SCREEN_SIZE: LazyLock<Mutex<(f32, f32)>> =
    LazyLock::new(|| Mutex::new((800.0, 600.0)));

static CONTENT_SCALE: LazyLock<Mutex<f32>> =
    LazyLock::new(|| Mutex::new(1.0));

pub struct UICallbacks {
    update: Option<UpdateCallback>,
    onclicks: HashMap<u64, WidgetCallback>,
    on_init: Option<InitCallback>,
    on_resize: Option<ResizeCallback>,
    on_global_click: Option<GlobalClickCallback>,
    on_key: Option<KeyCallback>,
    on_mouse_move: Option<MouseMoveCallback>,
    on_dropdown_select: HashMap<u64, DropdownSelectCallback>,
    on_input_field_change: HashMap<u64, InputFieldChangeCallback>,
    on_tab_select: HashMap<u64, TabSelectCallback>,
    on_tab_close: HashMap<u64, TabCloseCallback>,
    on_tree_node_select: HashMap<u64, TreeNodeSelectCallback>,
    on_tree_node_toggle: HashMap<u64, TreeNodeToggleCallback>,
    on_popup_menu_click: HashMap<u64, PopupMenuClickCallback>,
    on_popup_menu_close: HashMap<u64, PopupMenuCloseCallback>,
    on_grid_view_click: HashMap<u64, GridViewClickCallback>,
    on_dialog_result: HashMap<u64, DialogResultCallback>,
    on_file_browser_select: HashMap<u64, FileBrowserSelectCallback>,
    on_file_browser_double_click: HashMap<u64, FileBrowserDoubleClickCallback>,
}

impl UICallbacks {
    pub fn new() -> Self {
        Self {
            update: None,
            onclicks: HashMap::new(),
            on_init: None,
            on_resize: None,
            on_global_click: None,
            on_key: None,
            on_mouse_move: None,
            on_dropdown_select: HashMap::new(),
            on_input_field_change: HashMap::new(),
            on_tab_select: HashMap::new(),
            on_tab_close: HashMap::new(),
            on_tree_node_select: HashMap::new(),
            on_tree_node_toggle: HashMap::new(),
            on_popup_menu_click: HashMap::new(),
            on_popup_menu_close: HashMap::new(),
            on_grid_view_click: HashMap::new(),
            on_dialog_result: HashMap::new(),
            on_file_browser_select: HashMap::new(),
            on_file_browser_double_click: HashMap::new(),
        }
    }
    
    pub fn clear(&mut self) {
        self.update = None;
        self.onclicks.clear();
        self.on_init = None;
        self.on_resize = None;
        self.on_global_click = None;
        self.on_key = None;
        self.on_mouse_move = None;
        self.on_dropdown_select.clear();
        self.on_input_field_change.clear();
        self.on_tab_select.clear();
        self.on_tab_close.clear();
        self.on_tree_node_select.clear();
        self.on_tree_node_toggle.clear();
        self.on_popup_menu_click.clear();
        self.on_popup_menu_close.clear();
        self.on_grid_view_click.clear();
        self.on_dialog_result.clear();
        self.on_file_browser_select.clear();
        self.on_file_browser_double_click.clear();
    }
}

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

pub fn ui_set_screen_size(width: f32, height: f32) {
    let mut size = SCREEN_SIZE.lock();
    *size = (width, height);
}

pub fn ui_get_screen_size() -> (f32, f32) {
    let size = SCREEN_SIZE.lock();
    *size
}

pub fn ui_set_content_scale(scale: f32) {
    let mut content_scale = CONTENT_SCALE.lock();
    *content_scale = scale;
}

pub fn ui_get_content_scale() -> f32 {
    let content_scale = CONTENT_SCALE.lock();
    *content_scale
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_clear_callbacks() {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.clear();
    dfx_info!("UI", "清除所有回调");
}

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

pub fn has_onclick_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.onclicks.contains_key(&widget_id)
}

pub fn ui_set_primary_button_id(id: u64) {
    let mut primary_id = PRIMARY_BUTTON_ID.lock();
    *primary_id = Some(id);
    dfx_info!("UI", "设置主按钮ID: {}", id);
}

pub fn ui_get_primary_button_id() -> u64 {
    let primary_id = PRIMARY_BUTTON_ID.lock();
    primary_id.unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_dropdown_select_callback(widget_id: u64, callback: DropdownSelectCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_dropdown_select.insert(widget_id, callback);
    dfx_info!("UI", "注册DropdownSelect回调: widget={} callback={:?}", widget_id, callback);
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

pub fn has_dropdown_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_dropdown_select.contains_key(&widget_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_input_field_change_callback(widget_id: u64, callback: InputFieldChangeCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_input_field_change.insert(widget_id, callback);
    dfx_info!("UI", "注册InputFieldChange回调: widget={} callback={:?}", widget_id, callback);
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

pub fn has_input_field_change_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_input_field_change.contains_key(&widget_id)
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

pub fn has_tab_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tab_select.contains_key(&widget_id)
}

pub fn has_tab_close_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tab_close.contains_key(&widget_id)
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

pub fn has_tree_node_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tree_node_select.contains_key(&widget_id)
}

pub fn has_tree_node_toggle_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_tree_node_toggle.contains_key(&widget_id)
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

pub fn has_popup_menu_click_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_popup_menu_click.contains_key(&widget_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_grid_view_click_callback(widget_id: u64, callback: GridViewClickCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_grid_view_click.insert(widget_id, callback);
    dfx_info!("UI", "注册GridViewClick回调: widget={} callback={:?}", widget_id, callback);
}

pub fn trigger_grid_view_click_callback(widget_id: u64, index: usize, user_data: u64) {
    let callback = {
        let callbacks = UI_CALLBACKS.lock();
        callbacks.on_grid_view_click.get(&widget_id).copied()
    };
    if let Some(cb) = callback {
        cb(widget_id, index, user_data);
    }
}

pub fn has_grid_view_click_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_grid_view_click.contains_key(&widget_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_dialog_result_callback(widget_id: u64, callback: DialogResultCallback) {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.on_dialog_result.insert(widget_id, callback);
    dfx_info!("UI", "注册DialogResult回调: widget={} callback={:?}", widget_id, callback);
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

pub fn has_dialog_result_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_dialog_result.contains_key(&widget_id)
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

pub fn has_file_browser_select_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_file_browser_select.contains_key(&widget_id)
}

pub fn has_file_browser_double_click_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.on_file_browser_double_click.contains_key(&widget_id)
}