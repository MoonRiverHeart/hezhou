use parking_lot::Mutex;
use std::sync::LazyLock;

use hezhou_dfx::*;

pub static SCREEN_SIZE: LazyLock<Mutex<(f32, f32)>> =
    LazyLock::new(|| Mutex::new((800.0, 600.0)));

pub static CONTENT_SCALE: LazyLock<Mutex<f32>> =
    LazyLock::new(|| Mutex::new(1.0));

pub static PRIMARY_BUTTON_ID: LazyLock<Mutex<Option<u64>>> =
    LazyLock::new(|| Mutex::new(None));

pub static FOCUSED_INPUT_FIELD: LazyLock<Mutex<Option<u64>>> =
    LazyLock::new(|| Mutex::new(None));

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

pub fn ui_set_primary_button_id(id: u64) {
    let mut primary_id = PRIMARY_BUTTON_ID.lock();
    *primary_id = Some(id);
    dfx_debug!("UI", "设置主按钮ID: {}", id);
}

pub fn ui_get_primary_button_id() -> u64 {
    let primary_id = PRIMARY_BUTTON_ID.lock();
    primary_id.unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_focused_input_field(widget_id: u64) {
    let mut focused = FOCUSED_INPUT_FIELD.lock();
    *focused = Some(widget_id);
    // Focus set — no log needed (not a user-initiated action)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_clear_focused_input_field() {
    let mut focused = FOCUSED_INPUT_FIELD.lock();
    *focused = None;
    dfx_debug!("UI", "清除focus InputField");
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_focused_input_field() -> u64 {
    let focused = FOCUSED_INPUT_FIELD.lock();
    focused.unwrap_or(0)
}

pub fn is_input_field_focused(widget_id: u64) -> bool {
    let focused = FOCUSED_INPUT_FIELD.lock();
    *focused == Some(widget_id)
}