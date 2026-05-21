use crate::*;
use crate::thunk_manager::*;
use crate::widget_tree::RenderLayer;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::{c_char, CStr};
use std::sync::Arc;

pub type WidgetTreeHandle = *mut Arc<Mutex<WidgetTree>>;
pub type EventDispatcherHandle = *mut Arc<Mutex<EventDispatcher>>;
pub type ClickCallback = extern "C" fn(u64);

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_create() -> *mut UISystem {
    let system = Box::new(UISystem::new());
    Box::into_raw(system)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_destroy(system: *mut UISystem) {
    if !system.is_null() {
        unsafe {
            let _ = Box::from_raw(system);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_update(system: *mut UISystem, delta_time: f32) {
    if system.is_null() {
        return;
    }
    unsafe {
        (*system).update(delta_time);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_get_widget_tree(system: *const UISystem) -> WidgetTreeHandle {
    if system.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let arc = (*system).get_widget_tree();
        Box::into_raw(Box::new(arc)) as WidgetTreeHandle
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_system_get_event_dispatcher(system: *const UISystem) -> EventDispatcherHandle {
    if system.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let arc = (*system).get_event_dispatcher();
        Box::into_raw(Box::new(arc)) as EventDispatcherHandle
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_tree_handle_destroy(handle: WidgetTreeHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut Arc<Mutex<WidgetTree>>);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_handle_destroy(handle: EventDispatcherHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut Arc<Mutex<EventDispatcher>>);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_tree_create_root_panel(
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
        tree.set_root(Box::new(panel));
        tree.root.map(|r| r.id).unwrap_or(0)
    }
}

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
pub extern "C" fn ui_widget_set_layout(
    handle: WidgetTreeHandle,
    widget_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            widget.set_layout(Layout::new(x, y, width, height));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_background_color(
    handle: WidgetTreeHandle,
    widget_id: u64,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let new_style = widget
                .style()
                .clone()
                .with_background(Color::new(r, g, b, a));
            widget.set_style(new_style);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_touch_begin(
    handle: EventDispatcherHandle,
    x: f32,
    y: f32,
    pointer_id: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::TouchBegin, timestamp)
            .with_data(EventData::Touch(TouchData::new(x, y, pointer_id)));
        dispatcher.dispatch_event(&mut event);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_touch_end(
    handle: EventDispatcherHandle,
    x: f32,
    y: f32,
    pointer_id: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::TouchEnd, timestamp)
            .with_data(EventData::Touch(TouchData::new(x, y, pointer_id)));
        dispatcher.dispatch_event(&mut event);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_key_down(
    handle: EventDispatcherHandle,
    keycode: u32,
    modifiers: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::KeyDown, timestamp)
            .with_data(EventData::Key(KeyData::new(keycode, modifiers)));
        dispatcher.dispatch_event(&mut event);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_event_dispatcher_dispatch_key_up(
    handle: EventDispatcherHandle,
    keycode: u32,
    modifiers: u32,
    timestamp: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<EventDispatcher>>);
        let mut dispatcher = arc.lock();
        let mut event = Event::new(EventType::KeyUp, timestamp)
            .with_data(EventData::Key(KeyData::new(keycode, modifiers)));
        dispatcher.dispatch_event(&mut event);
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
pub extern "C" fn ui_register_update_thunk(callback: UpdateCallback) {
    ui_register_update_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_update_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: UpdateCallback = unsafe { std::mem::transmute(callback_ptr) };
    ui_register_update_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_init_thunk(callback: InitCallback) {
    ui_register_init_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_init_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: InitCallback = unsafe { std::mem::transmute(callback_ptr) };
    ui_register_init_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_update(delta_time: f32) {
    trigger_update_callback(delta_time);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_onclick(widget_id: u64) {
    trigger_onclick_callback(widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_init() {
    trigger_init_callback();
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_has_onclick(widget_id: u64) -> bool {
    has_onclick_callback(widget_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_primary_button_id(id: u64) {
    crate::thunk_manager::ui_set_primary_button_id(id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_primary_button_id() -> u64 {
    crate::thunk_manager::ui_get_primary_button_id()
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
pub extern "C" fn ui_get_text_width(
    text: *const c_char,
    font_size: f32,
) -> f32 {
    if text.is_null() {
        return 0.0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let font_atlas = crate::font_atlas::get_font_atlas();
        let (width, _) = font_atlas.measure_text(0, &text_str, font_size);
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
pub extern "C" fn ui_widget_get_x(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().x).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_y(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().y).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_width(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().width).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_height(handle: WidgetTreeHandle, widget_id: u64) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget(id).map(|w| w.layout().height).unwrap_or(0.0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_position(handle: WidgetTreeHandle, widget_id: u64, x: f32, y: f32) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let layout = widget.layout();
            widget.set_layout(Layout::new(x, y, layout.width, layout.height));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_size(handle: WidgetTreeHandle, widget_id: u64, width: f32, height: f32) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            let layout = widget.layout();
            widget.set_layout(Layout::new(layout.x, layout.y, width, height));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_resize_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: ResizeCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_resize_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_global_click_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::GlobalClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_global_click_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_key_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::KeyCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_key_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_register_mouse_move_thunk_ptr(callback_ptr: *const std::ffi::c_void) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::MouseMoveCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_mouse_move_callback(callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_trigger_resize(width: f32, height: f32) {
    crate::thunk_manager::trigger_resize_callback(width, height);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_screen_size(width: f32, height: f32) {
    crate::thunk_manager::ui_set_screen_size(width, height);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_screen_size(out_width: *mut f32, out_height: *mut f32) {
    let (w, h) = crate::thunk_manager::ui_get_screen_size();
    unsafe {
        if !out_width.is_null() {
            *out_width = w;
        }
        if !out_height.is_null() {
            *out_height = h;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_content_scale(scale: f32) {
    crate::thunk_manager::ui_set_content_scale(scale);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_content_scale() -> f32 {
    crate::thunk_manager::ui_get_content_scale()
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_remove_widget(
    handle: WidgetTreeHandle,
    widget_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.remove_widget(id);
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
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
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
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
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
pub extern "C" fn ui_create_preview_window(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    texture_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let mut preview = crate::widgets::PreviewWindow::new(texture_id);
        preview.set_layout(Layout::new(x, y, width, height));
        
        let id = preview.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(preview), parent);
        dfx_info!("FFI", "CreatePreviewWindow: id={}, texture_id={}", id.id, texture_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_preview_texture(
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
            if let Some(preview) = widget.as_any_mut().downcast_mut::<crate::widgets::PreviewWindow>() {
                preview.set_texture_id(texture_id);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_is_preview_window_selected(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> bool {
    if handle.is_null() {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if let Some(preview) = widget.as_any().downcast_ref::<crate::widgets::PreviewWindow>() {
                return preview.is_selected();
            }
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_preview_window_selected(
    handle: WidgetTreeHandle,
    widget_id: u64,
    selected: bool,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if let Some(preview) = widget.as_any_mut().downcast_mut::<crate::widgets::PreviewWindow>() {
                preview.set_selected(selected);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_preview_window_edit_mode(
    handle: WidgetTreeHandle,
    widget_id: u64,
    edit_mode: bool,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if let Some(preview) = widget.as_any_mut().downcast_mut::<crate::widgets::PreviewWindow>() {
                preview.set_edit_mode(edit_mode);
                dfx_info!("FFI", "PreviewWindow edit_mode set to: {}", edit_mode);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_widget_layout(
    handle: WidgetTreeHandle,
    widget_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            widget.set_layout(Layout::new(x, y, width, height));
        }
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
pub extern "C" fn ui_get_root_id(handle: WidgetTreeHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        tree.root.map(|r| r.id).unwrap_or(0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_text_edit(
    handle: WidgetTreeHandle,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let text_edit = TextEdit::with_size(width, height);
        let id = text_edit.id();
        let root_id = tree.root.unwrap_or(WidgetId::invalid());
        tree.add_widget(Box::new(text_edit), root_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_text_edit_in_parent(
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
        let mut text_edit = TextEdit::with_size(width, height);
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        let font_size = 16.0 * content_scale;
        dfx_info!("FFI", "CreateTextEdit: content_scale={}, font_size={}", content_scale, font_size);
        text_edit.set_font_size(font_size);
        
        let id = text_edit.id();
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        tree.add_widget(Box::new(text_edit), parent);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_text_edit_show_line_numbers(
    handle: WidgetTreeHandle,
    widget_id: u64,
    show: bool,
) {
    if handle.is_null() || widget_id == 0 {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                text_edit.set_show_line_numbers(show);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_set_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    text: *const std::ffi::c_char,
) {
    if handle.is_null() || text.is_null() {
        dfx_info!("FFI", "ui_text_edit_set_text: handle or text is null");
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        dfx_info!("FFI", "ui_text_edit_set_text: widget_id={}, looking for widget", widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            dfx_info!("FFI", "Found widget, type={}", widget.widget_type());
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                    let text_str = std::ffi::CStr::from_ptr(text).to_string_lossy();
                    dfx_info!("FFI", "Setting text: {} chars, font_size={}", text_str.len(), text_edit.get_text_style().font_size);
                    text_edit.set_text(&text_str);
                    dfx_info!("FFI", "✓ Text set successfully");
                }
            }
        } else {
            dfx_info!("FFI", "Widget not found for id={}", widget_id);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_insert_char(
    handle: WidgetTreeHandle,
    widget_id: u64,
    c: std::ffi::c_char,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                    if c != 0 {
                        text_edit.insert_char(c as u8 as char);
                    }
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_delete_char(
    handle: WidgetTreeHandle,
    widget_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any_mut().downcast_mut::<TextEdit>() {
                    text_edit.delete_char();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_get_text_len(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any().downcast_ref::<TextEdit>() {
                    return text_edit.get_text().len();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_text_edit_get_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    buffer: *mut std::ffi::c_char,
    buffer_size: usize,
) {
    if handle.is_null() || buffer.is_null() || buffer_size == 0 {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TextEdit" {
                use crate::widgets::TextEdit;
                if let Some(text_edit) = widget.as_any().downcast_ref::<TextEdit>() {
                    let text = text_edit.get_text();
                    let copy_len = text.len().min(buffer_size - 1);
                    std::ptr::copy_nonoverlapping(
                        text.as_ptr(),
                        buffer as *mut u8,
                        copy_len,
                    );
                    *buffer.add(copy_len) = 0;  // null terminator
                }
            }
        }
    }
}

pub extern "C" fn ui_clear_widget_tree(handle: WidgetTreeHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        tree.clear();
    }
    crate::thunk_manager::ui_clear_callbacks();
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
pub extern "C" fn ui_create_dropdown(
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
        let mut dropdown = crate::widgets::Dropdown::new();
        dropdown.set_layout(Layout::new(0.0, 0.0, width, height));
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        dropdown.set_content_scale(content_scale);
        
        let id = dropdown.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(dropdown), parent);
        dfx_info!("FFI", "CreateDropdown: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_set_options(
    handle: WidgetTreeHandle,
    widget_id: u64,
    options_ptr: *const c_char,
    options_count: usize,
) {
    if handle.is_null() || options_ptr.is_null() || options_count == 0 {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dropdown" {
                use crate::widgets::Dropdown;
                if let Some(dropdown) = widget.as_any_mut().downcast_mut::<Dropdown>() {
                    let options_str = CStr::from_ptr(options_ptr).to_string_lossy();
                    let options: Vec<String> = options_str.split('\0')
                        .filter(|s| !s.is_empty())
                        .take(options_count)
                        .map(|s| s.to_string())
                        .collect();
                    let count = options.len();
                    dropdown.set_options(options);
                    dfx_info!("FFI", "DropdownSetOptions: widget_id={}, count={}", widget_id, count);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_set_selected(
    handle: WidgetTreeHandle,
    widget_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dropdown" {
                use crate::widgets::Dropdown;
                if let Some(dropdown) = widget.as_any_mut().downcast_mut::<Dropdown>() {
                    dropdown.set_selected(index);
                    dfx_info!("FFI", "DropdownSetSelected: widget_id={}, index={}", widget_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_get_selected(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Dropdown" {
                use crate::widgets::Dropdown;
                if let Some(dropdown) = widget.as_any().downcast_ref::<Dropdown>() {
                    return dropdown.selected_index();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dropdown_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::DropdownSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_dropdown_select_callback(widget_id, callback);
    dfx_info!("FFI", "DropdownSetOnSelectThunkPtr: widget_id={}", widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_input_field(
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
        let mut input_field = crate::widgets::InputField::new();
        input_field.set_layout(Layout::new(0.0, 0.0, width, height));
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        input_field.set_content_scale(content_scale);
        
        let id = input_field.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(input_field), parent);
        dfx_info!("FFI", "CreateInputField: id={}, parent={}", id.id, parent_id);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_set_text(
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
            if widget.widget_type() == "InputField" {
                use crate::widgets::InputField;
                if let Some(input_field) = widget.as_any_mut().downcast_mut::<InputField>() {
                    let text_str = CStr::from_ptr(text).to_string_lossy();
                    input_field.set_text(&text_str);
                    dfx_info!("FFI", "InputFieldSetText: widget_id={}, text_len={}", widget_id, text_str.len());
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_get_text(
    handle: WidgetTreeHandle,
    widget_id: u64,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if handle.is_null() || buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "InputField" {
                use crate::widgets::InputField;
                if let Some(input_field) = widget.as_any().downcast_ref::<InputField>() {
                    let text = input_field.text();
                    let copy_len = text.len().min(buffer_size - 1);
                    std::ptr::copy_nonoverlapping(
                        text.as_ptr(),
                        buffer as *mut u8,
                        copy_len,
                    );
                    *buffer.add(copy_len) = 0;
                    return copy_len;
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_set_on_change_thunk_ptr(
    handle: WidgetTreeHandle,
    widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::InputFieldChangeCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_input_field_change_callback(widget_id, callback);
    dfx_info!("FFI", "InputFieldSetOnChangeThunkPtr: widget_id={}", widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_input_field_set_placeholder(
    handle: WidgetTreeHandle,
    widget_id: u64,
    placeholder: *const c_char,
) {
    if handle.is_null() || placeholder.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "InputField" {
                use crate::widgets::InputField;
                if let Some(input_field) = widget.as_any_mut().downcast_mut::<InputField>() {
                    let placeholder_str = CStr::from_ptr(placeholder).to_string_lossy();
                    input_field.set_placeholder(&placeholder_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_tab_widget(
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
        
        let mut tab_widget = crate::widgets::TabWidget::new()
            .with_layout(x, y, width, height);
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        tab_widget.set_content_scale(content_scale);
        
        let id = tab_widget.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(tab_widget), parent);
        dfx_info!("FFI", "CreateTabWidget: id={}, parent={}, scale={}", id.id, parent_id, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_add_tab(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    title: *const c_char,
    content_widget_id: u64,
    closable: bool,
) -> u32 {
    if handle.is_null() || title.is_null() {
        return 0;
    }
    unsafe {
        let title_str = CStr::from_ptr(title).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any_mut().downcast_mut::<TabWidget>() {
                    let content_id = WidgetId::from_raw(content_widget_id);
                    let index = tab_widget.add_tab(&title_str, content_id, closable);
                    dfx_info!("FFI", "TabWidgetAddTab: widget_id={}, title={}, content={}, closable={}, index={}", 
                        tab_widget_id, title_str, content_widget_id, closable, index);
                    return index as u32;
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_set_active(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any_mut().downcast_mut::<TabWidget>() {
                    tab_widget.set_active(index);
                    dfx_info!("FFI", "TabWidgetSetActive: widget_id={}, index={}", tab_widget_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_get_active(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any().downcast_ref::<TabWidget>() {
                    return tab_widget.active_index();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_remove_tab(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any_mut().downcast_mut::<TabWidget>() {
                    tab_widget.remove_tab(index);
                    dfx_info!("FFI", "TabWidgetRemoveTab: widget_id={}, index={}", tab_widget_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::TabSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_tab_select_callback(tab_widget_id, callback);
    dfx_info!("FFI", "TabWidgetSetOnSelectThunkPtr: widget_id={}", tab_widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_set_on_close_thunk_ptr(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::TabCloseCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_tab_close_callback(tab_widget_id, callback);
    dfx_info!("FFI", "TabWidgetSetOnCloseThunkPtr: widget_id={}", tab_widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tab_widget_get_tab_count(
    handle: WidgetTreeHandle,
    tab_widget_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(tab_widget_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TabWidget" {
                use crate::widgets::TabWidget;
                if let Some(tab_widget) = widget.as_any().downcast_ref::<TabWidget>() {
                    return tab_widget.tab_count();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_tree_view(
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
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        let mut tree_view = crate::widgets::TreeView::new()
            .with_layout(x, y, width, height)
            .with_content_scale(content_scale);
        
        let id = tree_view.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(tree_view), parent);
        dfx_info!("FFI", "CreateTreeView: id={}, parent={}, scale={}", id.id, parent_id, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_add_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    parent_node_id: u64,
    text: *const c_char,
    user_data: u64,
    has_children: bool,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        let mut node = crate::widgets::TreeNode::new(&text_str)
            .with_content_scale(content_scale)
            .with_user_data(user_data)
            .with_has_children(has_children);
        
        let node_id = node.id();
        
        if parent_node_id == 0 {
            node.set_depth(0);
            if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
                if widget.widget_type() == "TreeView" {
                    use crate::widgets::TreeView;
                    if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                        tree_view.add_root_node(node_id);
                    }
                }
            }
        } else {
            let parent_id = WidgetId::from_raw(parent_node_id);
            if let Some(widget) = tree.get_widget(parent_id) {
                if widget.widget_type() == "TreeNode" {
                    use crate::widgets::TreeNode;
                    if let Some(parent_node) = widget.as_any().downcast_ref::<TreeNode>() {
                        node.set_depth(parent_node.depth() + 1);
                        node.set_parent(parent_id);
                    }
                }
            }
            if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
                if widget.widget_type() == "TreeView" {
                    use crate::widgets::TreeView;
                    if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                        tree_view.add_child_node(parent_id, node_id);
                    }
                }
            }
        }
        
        tree.add_widget(Box::new(node), tree_view_widget_id);
        dfx_info!("FFI", "TreeViewAddNode: node_id={}, parent={}, text={}, user_data={}, has_children={}", 
            node_id.id, parent_node_id, text_str, user_data, has_children);
        node_id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_remove_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                    tree_view.remove_node(node_widget_id);
                }
            }
        }
        
        tree.remove_widget(node_widget_id);
        dfx_info!("FFI", "TreeViewRemoveNode: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_set_selected(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                    tree_view.set_selected(node_widget_id);
                }
            }
        }
        
        if let Some(widget) = tree.get_widget_mut(node_widget_id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_selected(true);
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewSetSelected: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_get_selected(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        
        if let Some(widget) = tree.get_widget(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any().downcast_ref::<TreeView>() {
                    return tree_view.selected_node().map(|id| id.id).unwrap_or(0);
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_expand_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(node_widget_id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_expanded(true);
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewExpandNode: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_collapse_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(node_widget_id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_expanded(false);
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewCollapseNode: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::TreeNodeSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_tree_node_select_callback(tree_view_id, callback);
    dfx_info!("FFI", "TreeViewSetOnSelectThunkPtr: tree_view_id={}", tree_view_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_node_set_text(
    handle: WidgetTreeHandle,
    node_id: u64,
    text: *const c_char,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_text(&text_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_node_get_user_data(
    handle: WidgetTreeHandle,
    node_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any().downcast_ref::<TreeNode>() {
                    return node.user_data();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_clear_selection(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        
        if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                    let selected = tree_view.selected_node();
                    tree_view.clear_selection();
                    
                    if let Some(selected_id) = selected {
                        if let Some(selected_widget) = tree.get_widget_mut(selected_id) {
                            if selected_widget.widget_type() == "TreeNode" {
                                use crate::widgets::TreeNode;
                                if let Some(node) = selected_widget.as_any_mut().downcast_mut::<TreeNode>() {
                                    node.set_selected(false);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewClearSelection: tree_view_id={}", tree_view_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_set_layer(
    handle: WidgetTreeHandle,
    widget_id: u64,
    layer: u32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        let render_layer = match layer {
            0 => RenderLayer::Background,
            1 => RenderLayer::Content,
            2 => RenderLayer::Popup,
            3 => RenderLayer::Overlay,
            _ => RenderLayer::Content,
        };
        tree.set_widget_layer(id, render_layer);
        dfx_info!("FFI", "WidgetSetLayer: widget_id={}, layer={}", widget_id, layer);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_widget_get_layer(
    handle: WidgetTreeHandle,
    widget_id: u64,
) -> u32 {
    if handle.is_null() {
        return 1;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(widget_id);
        tree.get_widget_layer(id) as u32
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_popup_menu(
    handle: WidgetTreeHandle,
    parent_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        let mut popup_menu = crate::widgets::PopupMenu::new();
        popup_menu.set_content_scale(content_scale);
        
        let id = popup_menu.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(popup_menu), parent);
        tree.set_widget_layer(id, crate::widget_tree::RenderLayer::Popup);
        dfx_info!("FFI", "CreatePopupMenu: id={}, parent={}, scale={}", id.id, parent_id, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_add_item(
    handle: WidgetTreeHandle,
    menu_id: u64,
    text: *const c_char,
    shortcut: *const c_char,
    action_id: usize,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let shortcut_str = if shortcut.is_null() {
            None
        } else {
            Some(CStr::from_ptr(shortcut).to_string_lossy().into_owned())
        };
        
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.add_item(text_str.clone(), shortcut_str, action_id);
                    dfx_info!("FFI", "PopupMenuAddItem: menu_id={}, text={}, action_id={}", menu_id, text_str, action_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_add_separator(
    handle: WidgetTreeHandle,
    menu_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.add_separator();
                    dfx_info!("FFI", "PopupMenuAddSeparator: menu_id={}", menu_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_show(
    handle: WidgetTreeHandle,
    menu_id: u64,
    x: f32,
    y: f32,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.show(x, y);
                    dfx_info!("FFI", "PopupMenuShow: menu_id={}, x={}, y={}", menu_id, x, y);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_hide(
    handle: WidgetTreeHandle,
    menu_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any_mut().downcast_mut::<PopupMenu>() {
                    popup_menu.hide();
                    dfx_info!("FFI", "PopupMenuHide: menu_id={}", menu_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_is_visible(
    handle: WidgetTreeHandle,
    menu_id: u64,
) -> bool {
    if handle.is_null() {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(menu_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "PopupMenu" {
                use crate::widgets::PopupMenu;
                if let Some(popup_menu) = widget.as_any().downcast_ref::<PopupMenu>() {
                    return popup_menu.is_visible();
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_popup_menu_set_on_click_thunk_ptr(
    handle: WidgetTreeHandle,
    menu_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::PopupMenuClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_popup_menu_click_callback(menu_id, callback);
    dfx_info!("FFI", "PopupMenuSetOnClickThunkPtr: menu_id={}", menu_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_grid_view(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    cell_size: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        let mut grid_view = crate::widgets::GridView::new()
            .with_cell_size(cell_size * content_scale)
            .with_spacing(8.0 * content_scale);
        grid_view.set_content_scale(content_scale);
        grid_view.set_layout(Layout::new(x, y, width, height));
        
        let id = grid_view.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(grid_view), parent);
        dfx_info!("FFI", "CreateGridView: id={}, parent={}, cell_size={}, scale={}", id.id, parent_id, cell_size, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_add_item(
    handle: WidgetTreeHandle,
    grid_id: u64,
    label: *const c_char,
    user_data: u64,
) -> u32 {
    if handle.is_null() || label.is_null() {
        return 0;
    }
    unsafe {
        let label_str = CStr::from_ptr(label).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    let index = grid_view.add_item(label_str.clone(), user_data);
                    dfx_info!("FFI", "GridViewAddItem: grid_id={}, label={}, user_data={}, index={}", grid_id, label_str, user_data, index);
                    return index as u32;
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_remove_item(
    handle: WidgetTreeHandle,
    grid_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    grid_view.remove_item(index);
                    dfx_info!("FFI", "GridViewRemoveItem: grid_id={}, index={}", grid_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_set_selected(
    handle: WidgetTreeHandle,
    grid_id: u64,
    index: usize,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    grid_view.set_selected(index);
                    dfx_info!("FFI", "GridViewSetSelected: grid_id={}, index={}", grid_id, index);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_get_selected(
    handle: WidgetTreeHandle,
    grid_id: u64,
) -> usize {
    if handle.is_null() {
        return usize::MAX;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any().downcast_ref::<GridView>() {
                    return grid_view.selected_index().unwrap_or(usize::MAX);
                }
            }
        }
        usize::MAX
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_get_selected_user_data(
    handle: WidgetTreeHandle,
    grid_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any().downcast_ref::<GridView>() {
                    return grid_view.selected_user_data().unwrap_or(0);
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_clear(
    handle: WidgetTreeHandle,
    grid_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any_mut().downcast_mut::<GridView>() {
                    grid_view.clear();
                    dfx_info!("FFI", "GridViewClear: grid_id={}", grid_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_item_count(
    handle: WidgetTreeHandle,
    grid_id: u64,
) -> usize {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(grid_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "GridView" {
                use crate::widgets::GridView;
                if let Some(grid_view) = widget.as_any().downcast_ref::<GridView>() {
                    return grid_view.item_count();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_grid_view_set_on_click_thunk_ptr(
    handle: WidgetTreeHandle,
    grid_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::GridViewClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_grid_view_click_callback(grid_id, callback);
    dfx_info!("FFI", "GridViewSetOnClickThunkPtr: grid_id={}", grid_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_dialog(
    handle: WidgetTreeHandle,
    parent_id: u64,
    title: *const c_char,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() || title.is_null() {
        return 0;
    }
    unsafe {
        let title_str = CStr::from_ptr(title).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        let screen_size = crate::thunk_manager::ui_get_screen_size();
        let x = (screen_size.0 - width) / 2.0;
        let y = (screen_size.1 - height) / 2.0;
        
        let mut dialog = crate::widgets::Dialog::new()
            .with_title(&title_str)
            .with_size(width, height);
        dialog.set_layout(Layout::new(x, y, width, height));
        dialog.set_content_scale(content_scale);
        
        let id = dialog.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(dialog), parent);
        tree.set_widget_layer(id, crate::widget_tree::RenderLayer::Overlay);
        dfx_info!("FFI", "CreateDialog: id={}, title={}, size={}x{}, scale={}", id.id, title_str, width, height, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_set_content(
    handle: WidgetTreeHandle,
    dialog_id: u64,
    content_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    dialog.set_content(WidgetId::from_raw(content_id));
                    dfx_info!("FFI", "DialogSetContent: dialog_id={}, content_id={}", dialog_id, content_id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_add_button(
    handle: WidgetTreeHandle,
    dialog_id: u64,
    text: *const c_char,
    action: i32,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                use crate::widgets::dialog::DialogAction;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    let dialog_action = match action {
                        0 => DialogAction::Ok,
                        1 => DialogAction::Cancel,
                        2 => DialogAction::Yes,
                        3 => DialogAction::No,
                        _ => DialogAction::Custom,
                    };
                    dialog.add_button(&text_str, dialog_action);
                    dfx_info!("FFI", "DialogAddButton: dialog_id={}, text={}, action={}", dialog_id, text_str, action);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_show(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    dialog.show();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_hide(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any_mut().downcast_mut::<Dialog>() {
                    dialog.hide();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_is_visible(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) -> bool {
    if handle.is_null() {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any().downcast_ref::<Dialog>() {
                    return dialog.is_visible();
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_get_result(
    handle: WidgetTreeHandle,
    dialog_id: u64,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(dialog_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "Dialog" {
                use crate::widgets::Dialog;
                if let Some(dialog) = widget.as_any().downcast_ref::<Dialog>() {
                    return dialog.result().unwrap_or(-1);
                }
            }
        }
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_dialog_set_on_result_thunk_ptr(
    handle: WidgetTreeHandle,
    dialog_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::DialogResultCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_dialog_result_callback(dialog_id, callback);
    dfx_info!("FFI", "DialogSetOnResultThunkPtr: dialog_id={}", dialog_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_file_browser(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    initial_path: *const c_char,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let initial_path_str = if initial_path.is_null() {
            ".".to_string()
        } else {
            CStr::from_ptr(initial_path).to_string_lossy().into_owned()
        };
        
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk_manager::ui_get_content_scale();
        let mut file_browser = crate::widgets::FileBrowser::new()
            .with_initial_path(&initial_path_str)
            .with_layout(x, y, width, height);
        file_browser.set_content_scale(content_scale);
        
        let id = file_browser.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(file_browser), parent);
        dfx_info!("FFI", "CreateFileBrowser: id={}, parent={}, path={}, scale={}", id.id, parent_id, initial_path_str, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_path(
    handle: WidgetTreeHandle,
    browser_id: u64,
    path: *const c_char,
) {
    if handle.is_null() || path.is_null() {
        return;
    }
    unsafe {
        let path_str = CStr::from_ptr(path).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.set_path(&path_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_filter(
    handle: WidgetTreeHandle,
    browser_id: u64,
    filter: *const c_char,
) {
    if handle.is_null() || filter.is_null() {
        return;
    }
    unsafe {
        let filter_str = CStr::from_ptr(filter).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.set_filter(&filter_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_navigate_up(
    handle: WidgetTreeHandle,
    browser_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.navigate_up();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_refresh(
    handle: WidgetTreeHandle,
    browser_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any_mut().downcast_mut::<FileBrowser>() {
                    file_browser.refresh();
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_get_selected_path(
    handle: WidgetTreeHandle,
    browser_id: u64,
    buffer: *mut c_char,
    size: usize,
) -> bool {
    if handle.is_null() || buffer.is_null() || size == 0 {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any().downcast_ref::<FileBrowser>() {
                    if let Some(path) = file_browser.get_selected_path() {
                        let copy_len = path.len().min(size - 1);
                        std::ptr::copy_nonoverlapping(
                            path.as_ptr(),
                            buffer as *mut u8,
                            copy_len,
                        );
                        *buffer.add(copy_len) = 0;
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_get_current_path(
    handle: WidgetTreeHandle,
    browser_id: u64,
    buffer: *mut c_char,
    size: usize,
) -> bool {
    if handle.is_null() || buffer.is_null() || size == 0 {
        return false;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(browser_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "FileBrowser" {
                use crate::widgets::FileBrowser;
                if let Some(file_browser) = widget.as_any().downcast_ref::<FileBrowser>() {
                    let path = file_browser.current_path();
                    let copy_len = path.len().min(size - 1);
                    std::ptr::copy_nonoverlapping(
                        path.as_ptr(),
                        buffer as *mut u8,
                        copy_len,
                    );
                    *buffer.add(copy_len) = 0;
                    return true;
                }
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    browser_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::FileBrowserSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_file_browser_select_callback(browser_id, callback);
    dfx_info!("FFI", "FileBrowserSetOnSelectThunkPtr: browser_id={}", browser_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_file_browser_set_on_double_click_thunk_ptr(
    handle: WidgetTreeHandle,
    browser_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk_manager::FileBrowserDoubleClickCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk_manager::ui_register_file_browser_double_click_callback(browser_id, callback);
    dfx_info!("FFI", "FileBrowserSetOnDoubleClickThunkPtr: browser_id={}", browser_id);
}
