mod types;
mod global;
mod pending;
mod register;
mod trigger;
mod has;

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::LazyLock;

use hezhou_dfx::*;

pub use types::*;
pub use global::*;
pub use pending::{PendingCallback, queue_callback, flush_pending_callbacks};
pub use register::*;
pub use trigger::*;
pub use has::*;

static UI_CALLBACKS: LazyLock<Mutex<UICallbacks>> =
    LazyLock::new(|| Mutex::new(UICallbacks::new()));

pub struct UICallbacks {
    pub update: Option<UpdateCallback>,
    pub onclicks: HashMap<u64, WidgetCallback>,
    pub on_init: Option<InitCallback>,
    pub on_resize: Option<ResizeCallback>,
    pub on_global_click: Option<GlobalClickCallback>,
    pub on_key: Option<KeyCallback>,
    pub on_mouse_move: Option<MouseMoveCallback>,
    pub on_dropdown_select: HashMap<u64, DropdownSelectCallback>,
    pub on_input_field_change: HashMap<u64, InputFieldChangeCallback>,
    pub on_tab_select: HashMap<u64, TabSelectCallback>,
    pub on_tab_close: HashMap<u64, TabCloseCallback>,
    pub on_tree_node_select: Option<TreeNodeSelectCallback>,
    pub on_tree_node_toggle: Option<TreeNodeToggleCallback>,
    pub on_popup_menu_click: HashMap<u64, PopupMenuClickCallback>,
    pub on_popup_menu_close: HashMap<u64, PopupMenuCloseCallback>,
    pub on_grid_view_click: HashMap<u64, GridViewClickCallback>,
    pub on_dialog_result: HashMap<u64, DialogResultCallback>,
    pub on_file_browser_select: HashMap<u64, FileBrowserSelectCallback>,
    pub on_file_browser_double_click: HashMap<u64, FileBrowserDoubleClickCallback>,
    pub on_focus_change: Option<FocusChangeCallback>,
    pub on_entity_selected: Option<EntitySelectedCallback>,
    pub on_checkbox_change: HashMap<u64, CheckboxChangeCallback>,
    pub on_slider_change: HashMap<u64, SliderChangeCallback>,
    pub on_scroll_view_scroll: HashMap<u64, ScrollViewScrollCallback>,
    pub on_split_view_ratio_change: HashMap<u64, SplitViewRatioChangeCallback>,
    pub on_mouse_wheel: Option<MouseWheelCallback>,
    pub on_tree_node_right_click: Option<TreeNodeRightClickCallback>,
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
                on_tree_node_select: None,
                on_tree_node_toggle: None,
                on_popup_menu_click: HashMap::new(),
                on_popup_menu_close: HashMap::new(),
                on_grid_view_click: HashMap::new(),
                on_dialog_result: HashMap::new(),
                on_file_browser_select: HashMap::new(),
                on_file_browser_double_click: HashMap::new(),
                on_focus_change: None,
                on_entity_selected: None,
                on_checkbox_change: HashMap::new(),
                on_slider_change: HashMap::new(),
                on_scroll_view_scroll: HashMap::new(),
                on_split_view_ratio_change: HashMap::new(),
                on_mouse_wheel: None,
                on_tree_node_right_click: None,
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
        self.on_tree_node_select = None;
        self.on_tree_node_toggle = None;
        self.on_popup_menu_click.clear();
        self.on_popup_menu_close.clear();
        self.on_grid_view_click.clear();
        self.on_dialog_result.clear();
        self.on_file_browser_select.clear();
        self.on_file_browser_double_click.clear();
        self.on_focus_change = None;
        self.on_entity_selected = None;
        self.on_checkbox_change.clear();
        self.on_slider_change.clear();
        self.on_scroll_view_scroll.clear();
        self.on_split_view_ratio_change.clear();
        self.on_mouse_wheel = None;
        self.on_tree_node_right_click = None;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_clear_callbacks() {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.clear();
    dfx_info!("UI", "清除所有回调");
}