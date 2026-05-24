use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_image(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut image = crate::widgets::Image::new(width, height);
        image.set_layout(Layout::new(x, y, width, height));

        let content_scale = crate::thunk::ui_get_content_scale();
        image.set_content_scale(content_scale);

        let id = image.id();

        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };

        tree.add_widget(Box::new(image), parent);
        dfx_debug!("FFI", "CreateImage: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_image_set_texture_id(
    handle: WidgetTreeHandle,
    widget_id: u64,
    texture_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Image" {
                use crate::widgets::Image;
                if let Some(img) = widget.as_any_mut().downcast_mut::<Image>() {
                    img.set_texture_id(texture_id);
                    dfx_debug!("FFI", "ImageSetTextureId: widget_id={}, texture_id={}", widget_id, texture_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_image_get_texture_id(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Image" {
                use crate::widgets::Image;
                if let Some(img) = widget.as_any().downcast_ref::<Image>() {
                    return img.get_texture_id();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_image_set_scale_mode(
    handle: WidgetTreeHandle,
    widget_id: u64,
    scale_mode: u32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Image" {
                use crate::widgets::ImageScaleMode;
                use crate::widgets::Image;
                let mode = match scale_mode {
                    0 => ImageScaleMode::Fill,
                    1 => ImageScaleMode::Fit,
                    2 => ImageScaleMode::Center,
                    3 => ImageScaleMode::Crop,
                    _ => ImageScaleMode::Fit,
                };
                if let Some(img) = widget.as_any_mut().downcast_mut::<Image>() {
                    img.set_scale_mode(mode);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_image_set_uv(
    handle: WidgetTreeHandle,
    widget_id: u64,
    uv_x: f32,
    uv_y: f32,
    uv_w: f32,
    uv_h: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Image" {
                use crate::widgets::Image;
                if let Some(img) = widget.as_any_mut().downcast_mut::<Image>() {
                    img.set_uv(Rect::new(uv_x, uv_y, uv_w, uv_h));
                }
            }
        }
    }
}