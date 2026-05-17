use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::*;

pub type UpdateCallback = extern "C" fn(f32);
pub type WidgetCallback = extern "C" fn(u64);
pub type InitCallback = extern "C" fn();

static UI_CALLBACKS: LazyLock<Mutex<UICallbacks>> =
    LazyLock::new(|| Mutex::new(UICallbacks::new()));

pub struct UICallbacks {
    update: Option<UpdateCallback>,
    onclicks: HashMap<u64, WidgetCallback>,
    on_init: Option<InitCallback>,
}

impl UICallbacks {
    pub fn new() -> Self {
        Self {
            update: None,
            onclicks: HashMap::new(),
            on_init: None,
        }
    }
    
    pub fn clear(&mut self) {
        self.update = None;
        self.onclicks.clear();
        self.on_init = None;
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
pub extern "C" fn ui_clear_callbacks() {
    let mut callbacks = UI_CALLBACKS.lock();
    callbacks.clear();
    dfx_info!("UI", "清除所有回调");
}

pub fn trigger_update_callback(delta_time: f32) {
    let callbacks = UI_CALLBACKS.lock();
    if let Some(cb) = callbacks.update {
        cb(delta_time);
    }
}

pub fn trigger_onclick_callback(widget_id: u64) {
    let callbacks = UI_CALLBACKS.lock();
    if let Some(cb) = callbacks.onclicks.get(&widget_id) {
        cb(widget_id);
    }
}

pub fn trigger_init_callback() {
    let callbacks = UI_CALLBACKS.lock();
    if let Some(cb) = callbacks.on_init {
        cb();
    }
}

pub fn has_onclick_callback(widget_id: u64) -> bool {
    let callbacks = UI_CALLBACKS.lock();
    callbacks.onclicks.contains_key(&widget_id)
}