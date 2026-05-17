use crate::*;
use crate::thunk_manager::*;
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
pub extern "C" fn ui_trigger_resize(width: f32, height: f32) {
    crate::thunk_manager::trigger_resize_callback(width, height);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_set_screen_size(width: f32, height: f32) {
    crate::thunk_manager::ui_set_screen_size(width, height);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_get_screen_size(out_width: *mut f32, out_height: *mut f32) {
    let (width, height) = crate::thunk_manager::ui_get_screen_size();
    if !out_width.is_null() {
        unsafe { *out_width = width; }
    }
    if !out_height.is_null() {
        unsafe { *out_height = height; }
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
        let id = button.id();
        let parent = WidgetId::from_raw(parent_id);
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
        let id = label.id();
        let parent = WidgetId::from_raw(parent_id);
        tree.add_widget(Box::new(label), parent);
        id.id
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
