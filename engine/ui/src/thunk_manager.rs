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