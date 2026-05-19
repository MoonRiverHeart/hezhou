use crate::*;
use crate::thunk_manager::*;
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
        let font_atlas = create_font_atlas();
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
