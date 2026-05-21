use parking_lot::Mutex;
use std::sync::LazyLock;
use hezhou_dfx::*;

/// Centralized focus management for the UI system.
/// Replaces the ad-hoc FOCUSED_INPUT_FIELD global with a proper focus chain.
pub struct FocusState {
    /// Currently focused widget (any type, not just InputField)
    focused_widget: Option<u64>,
    /// Currently selected entity in the editor
    selected_entity: Option<u64>,
    /// Focus scope stack: panels that trap focus
    focus_scopes: Vec<u64>,
}

pub static FOCUS_STATE: LazyLock<Mutex<FocusState>> =
    LazyLock::new(|| Mutex::new(FocusState {
        focused_widget: None,
        selected_entity: None,
        focus_scopes: Vec::new(),
    }));

/// Request focus for a widget.
/// Queues a FocusChange event if the focus actually changes.
pub fn request_focus(widget_id: u64) {
    let mut state = FOCUS_STATE.lock();
    let old_focus = state.focused_widget;
    if old_focus != Some(widget_id) {
        // Notify the old widget that it lost focus
        if let Some(old_id) = old_focus {
            crate::thunk::queue_callback(crate::thunk::PendingCallback::FocusChange {
                widget_id: old_id,
                is_focused: false,
            });
        }
        state.focused_widget = Some(widget_id);
        // Notify the new widget that it gained focus
        crate::thunk::queue_callback(crate::thunk::PendingCallback::FocusChange {
            widget_id: widget_id,
            is_focused: true,
        });
        dfx_debug!("Focus", "Focus changed: old={} new={}", old_focus.unwrap_or(0), widget_id);
    }
}

/// Release focus from a widget.
/// If the widget is currently focused, clears focus and queues FocusChange(false).
pub fn clear_widget_focus(widget_id: u64) {
    let mut state = FOCUS_STATE.lock();
    if state.focused_widget == Some(widget_id) {
        state.focused_widget = None;
        crate::thunk::queue_callback(crate::thunk::PendingCallback::FocusChange {
            widget_id: widget_id,
            is_focused: false,
        });
        dfx_debug!("Focus", "Focus released: {}", widget_id);
    }
}

/// Clear all widget focus (e.g., when clicking empty space).
pub fn clear_all_focus() {
    let mut state = FOCUS_STATE.lock();
    if let Some(old_id) = state.focused_widget {
        crate::thunk::queue_callback(crate::thunk::PendingCallback::FocusChange {
            widget_id: old_id,
            is_focused: false,
        });
        dfx_debug!("Focus", "Focus cleared: {}", old_id);
    }
    state.focused_widget = None;
}

/// Get the currently focused widget.
pub fn get_focused_widget() -> Option<u64> {
    FOCUS_STATE.lock().focused_widget
}

/// Check if a widget is focused.
pub fn is_widget_focused(widget_id: u64) -> bool {
    FOCUS_STATE.lock().focused_widget == Some(widget_id)
}

/// Select an entity in the editor.
/// Queues an EntitySelected event.
pub fn select_entity(entity_id: u64) {
    let mut state = FOCUS_STATE.lock();
    let old_entity = state.selected_entity;
    if old_entity != Some(entity_id) {
        if let Some(old_id) = old_entity {
            crate::thunk::queue_callback(crate::thunk::PendingCallback::EntitySelected {
                entity_id: old_id,
                is_selected: false,
            });
        }
        state.selected_entity = Some(entity_id);
        crate::thunk::queue_callback(crate::thunk::PendingCallback::EntitySelected {
            entity_id: entity_id,
            is_selected: true,
        });
        dfx_debug!("Focus", "Entity selected: old={} new={}", old_entity.unwrap_or(0), entity_id);
    }
}

/// Deselect the current entity.
pub fn deselect_entity() {
    let mut state = FOCUS_STATE.lock();
    if let Some(old_id) = state.selected_entity {
        crate::thunk::queue_callback(crate::thunk::PendingCallback::EntitySelected {
            entity_id: old_id,
            is_selected: false,
        });
        dfx_debug!("Focus", "Entity deselected: {}", old_id);
    }
    state.selected_entity = None;
}

/// Get the currently selected entity.
pub fn get_selected_entity() -> Option<u64> {
    FOCUS_STATE.lock().selected_entity
}

// === FFI exports ===

#[unsafe(no_mangle)]
pub extern "C" fn ui_focus_request(widget_id: u64) {
    request_focus(widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_focus_release_widget(widget_id: u64) {
    clear_widget_focus(widget_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_focus_clear_all() {
    clear_all_focus();
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_focus_get_focused() -> u64 {
    get_focused_widget().unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_focus_is_widget_focused(widget_id: u64) -> bool {
    is_widget_focused(widget_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_select(entity_id: u64) {
    select_entity(entity_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_deselect() {
    deselect_entity();
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_selected() -> u64 {
    get_selected_entity().unwrap_or(0)
}