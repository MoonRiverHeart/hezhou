use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_slider(
    handle: WidgetTreeHandle,
    parent_id: u64,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut slider = crate::widgets::Slider::new(0.0, 1.0, 0.0);
        slider.set_layout(Layout::new(0.0, 0.0, width, height));

        let content_scale = crate::thunk::ui_get_content_scale();
        slider.set_content_scale(content_scale);

        let id = slider.id();

        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };

        tree.add_widget(Box::new(slider), parent);
        dfx_debug!("FFI", "CreateSlider: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_slider_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    min: f32,
    max: f32,
    value: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut slider = crate::widgets::Slider::new(min, max, value);
        slider.set_layout(Layout::new(x, y, width, height));

        let content_scale = crate::thunk::ui_get_content_scale();
        slider.set_content_scale(content_scale);

        let id = slider.id();

        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };

        tree.add_widget(Box::new(slider), parent);
        dfx_debug!("FFI", "CreateSliderInParent: id={}, parent={}, min={}, max={}, value={}", id.id, parent_id, min, max, value);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_slider_set_value(
    handle: WidgetTreeHandle,
    widget_id: u64,
    value: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Slider" {
                use crate::widgets::Slider;
                if let Some(slider) = widget.as_any_mut().downcast_mut::<Slider>() {
                    slider.set_value(value);
                    dfx_debug!("FFI", "SliderSetValue: widget_id={}, value={}", widget_id, value);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_slider_get_value(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Slider" {
                use crate::widgets::Slider;
                if let Some(slider) = widget.as_any().downcast_ref::<Slider>() {
                    return slider.get_value();
                }
            }
        }
        0.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_slider_set_range(
    handle: WidgetTreeHandle,
    widget_id: u64,
    min: f32,
    max: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Slider" {
                use crate::widgets::Slider;
                if let Some(slider) = widget.as_any_mut().downcast_mut::<Slider>() {
                    slider.set_range(min, max);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_slider_set_on_change_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::SliderChangeCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_slider_change_callback(widget_id, callback);
    dfx_debug!("FFI", "SliderSetOnChangeThunkPtr: widget_id={}", widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_slider_set_step(
    handle: WidgetTreeHandle,
    widget_id: u64,
    step: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Slider" {
                use crate::widgets::Slider;
                if let Some(slider) = widget.as_any_mut().downcast_mut::<Slider>() {
                    slider.set_step(step);
                }
            }
        }
    }
}