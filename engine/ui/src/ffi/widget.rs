use crate::*;
use crate::thunk::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

use super::{WidgetTreeHandle, ClickCallback};

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_tree_add_button(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: *const c_char,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut button = Button::new(&text_str);
        button.set_layout(Layout::new(x, y, width, height));
        let id = button.id();
        let parent = WidgetId::from_raw(parent_id);
        tree.add_widget(Box::new(button), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_tree_add_label(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: *const c_char,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut label = Label::new(&text_str);
        label.set_layout(Layout::new(x, y, width, height));
        let id = label.id();
        let parent = WidgetId::from_raw(parent_id);
        tree.add_widget(Box::new(label), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_tree_add_panel(
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
        let mut panel = Panel::new();
        panel.set_layout(Layout::new(x, y, width, height));
        let id = panel.id();
        let parent = WidgetId::from_raw(parent_id);
        tree.add_widget(Box::new(panel), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    text: *const c_char,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);

        if let Some(widget) = tree.get_widget_mut(id) {
            let type_name = widget.as_ref().widget_type();

            if type_name == "Button" {
                if let Some(button) = (widget.as_mut() as *mut dyn Widget as *mut Button).as_mut() {
                    button.set_text(&text_str);
                }
            } else if type_name == "Label" {
                if let Some(label) = (widget.as_mut() as *mut dyn Widget as *mut Label).as_mut() {
                    label.set_text(&text_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_button_set_on_click(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback: ClickCallback,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);

        if let Some(widget) = tree.get_widget_mut(id) {
            let type_name = widget.as_ref().widget_type();

            if type_name == "Button" {
                if let Some(button) = (widget.as_mut() as *mut dyn Widget as *mut Button).as_mut() {
                    let cb_wrapper: Box<dyn FnMut() + Send + Sync> = Box::new(move || {
                        callback(widget_id);
                    });
                    button.set_on_click(cb_wrapper);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_button_set_on_click_thunk(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback: WidgetCallback,
) {
    ui_register_onclick_callback(widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_button_set_on_click_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: WidgetCallback = unsafe { std::mem::transmute(callback_ptr) };
    ui_register_onclick_callback(widget_id, callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_button(
    handle: WidgetTreeHandle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: *const c_char,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut button = Button::new(&text_str);
        button.set_layout(Layout::new(x, y, width, height));
        let id = button.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(button), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_button_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    width: f32,
    height: f32,
    text: *const c_char,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut button = Button::new(&text_str);
        button.set_layout(Layout::new(0.0, 0.0, width, height));
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let font_size = 16.0 * content_scale;
        dfx_info!("FFI", "CreateButton: content_scale={}, font_size={}", content_scale, font_size);
        button.set_font_size(font_size);
        
        let id = button.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(button), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_text_width(
    text: *const c_char,
    font_size: f32,
) -> f32 {
    if text.is_null() {
        return 0.0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let font_atlas_guard = crate::font_atlas::get_font_atlas().lock();
        let (width, _) = font_atlas_guard.measure_text(0, &text_str, font_size);
        width + 40.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_label(
    handle: WidgetTreeHandle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: *const c_char,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut label = Label::new(&text_str);
        label.set_layout(Layout::new(x, y, width, height));
        let id = label.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(label), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_label_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    width: f32,
    height: f32,
    text: *const c_char,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut label = Label::new(&text_str);
        label.set_layout(Layout::new(0.0, 0.0, width, height));
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let font_size = 16.0 * content_scale;
        dfx_info!("FFI", "CreateLabel: content_scale={}, font_size={}", content_scale, font_size);
        label.set_font_size(font_size);
        
        let id = label.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(label), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_panel(
    handle: WidgetTreeHandle,
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
        let mut panel = Panel::new();
        panel.set_layout(Layout::new(x, y, width, height));
        let id = panel.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(panel), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_panel_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let mut panel = Panel::new();
        panel.set_layout(Layout::new(x, y, width, height));
        panel.set_style(
            Style::new()
                .with_background(Color::new(r, g, b, a))
                .with_border(Color::new(0.3, 0.3, 0.3, 1.0), 1.0, 0.0)
        );
        let id = panel.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(panel), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_vstack(
    handle: WidgetTreeHandle,
    spacing: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let vstack = VStack::new().with_spacing(spacing);
        let id = vstack.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(vstack), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_vstack_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    spacing: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let vstack = VStack::new().with_spacing(spacing);
        let id = vstack.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(vstack), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_hstack(
    handle: WidgetTreeHandle,
    spacing: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let hstack = HStack::new().with_spacing(spacing);
        let id = hstack.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(hstack), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_hstack_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    spacing: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let hstack = HStack::new().with_spacing(spacing);
        let id = hstack.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(hstack), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_list(
    handle: WidgetTreeHandle,
    spacing: f32,
    orientation: u32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        use crate::widgets::List;
        use crate::widgets::list::ListOrientation;
        let orient = if orientation == 0 {
            ListOrientation::Horizontal
        } else {
            ListOrientation::Vertical
        };
        let list = List::new().with_spacing(spacing).with_orientation(orient);
        let id = list.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(list), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_list_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    spacing: f32,
    orientation: u32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        use crate::widgets::List;
        use crate::widgets::list::ListOrientation;
        let orient = if orientation == 0 {
            ListOrientation::Horizontal
        } else {
            ListOrientation::Vertical
        };
        let list = List::new().with_spacing(spacing).with_orientation(orient);
        let id = list.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(list), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_list_item(
    handle: WidgetTreeHandle,
    text: *const c_char,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        use crate::widgets::ListItem;
        let text_str = CStr::from_ptr(text).to_string_lossy();
        let item = ListItem::new(&text_str);
        let id = item.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(item), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_list_item_in_parent(
    handle: WidgetTreeHandle,
    parent_id: u64,
    text: *const c_char,
    show_border: u32,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        use crate::widgets::ListItem;
        let text_str = CStr::from_ptr(text).to_string_lossy();
        let mut item = ListItem::new(&text_str);
        item.set_show_border(show_border != 0);
        let id = item.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(item), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_list_item_set_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    text: *const c_char,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "ListItem" {
                use crate::widgets::ListItem;
                if let Some(list_item) = widget.as_any_mut().downcast_mut::<ListItem>() {
                    let text_str = CStr::from_ptr(text).to_string_lossy();
                    list_item.set_text(&text_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_list_item_set_font_size(
    handle: WidgetTreeHandle,
    widget_id: u64,
    font_size: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "ListItem" {
                use crate::widgets::ListItem;
                if let Some(list_item) = widget.as_any_mut().downcast_mut::<ListItem>() {
                    list_item.set_font_size(font_size);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_flex_expand(
    handle: WidgetTreeHandle,
    widget_id: u64,
    expand: u32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let mut flags = widget.flags();
            flags.flex_expand = expand != 0;
            widget.set_flags(flags);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_cross_axis_fill(
    handle: WidgetTreeHandle,
    widget_id: u64,
    fill: u32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let mut flags = widget.flags();
            flags.cross_axis_fill = fill != 0;
            widget.set_flags(flags);
        }
    }
}