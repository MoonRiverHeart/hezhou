use hezhou_scripting::ffi_context::OnHotReloadCompleteFn;
use hezhou_dfx::*;
use std::sync::atomic::Ordering;

#[unsafe(no_mangle)]
pub extern "C" fn trigger_hot_reload() {
    super::HOT_RELOAD_REQUESTED.store(true, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_status_text(status_ptr: *const i8) {
    if !status_ptr.is_null() {
        let status = unsafe { std::ffi::CStr::from_ptr(status_ptr).to_string_lossy().into_owned() };
        dfx_info!("Status", "状态更新: {}", status);
    }
}

pub extern "C" fn on_hot_reload_complete_placeholder() {
    // Placeholder — actual callback registered later from C#
}

#[unsafe(no_mangle)]
pub extern "C" fn register_hot_reload_complete_callback(callback: OnHotReloadCompleteFn) {
    unsafe {
        super::HOT_RELOAD_COMPLETE_CALLBACK = Some(callback);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_game_preview_extent(width: u32, height: u32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            if let Err(e) = (*renderer_ptr).set_game_preview_extent(width, height) {
                dfx_error!("Demo", "Failed to set game preview extent: {}", e);
            } else {
                // Success — extent updated silently
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_camera_params(yaw: f32, pitch: f32, x: f32, y: f32, z: f32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).set_camera_params(yaw, pitch, x, y, z);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_renderer_game_state(state: i32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).set_game_state(state);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_renderer_game_state() -> i32 {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).get_game_state()
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_entity_transform(px: f32, py: f32, pz: f32,
                                        rx: f32, ry: f32, rz: f32, rw: f32,
                                        sx: f32, sy: f32, sz: f32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).set_entity_transform(px, py, pz, rx, ry, rz, rw, sx, sy, sz);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_entity_angle(angle: f32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).set_entity_angle(angle);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_entity_angle() -> f32 {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).get_entity_angle()
        } else {
            0.0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_selected_entity(entity_id: u64, selected: bool) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).set_selected_entity(entity_id, selected);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn capture_screenshot_to_file(path_ptr: *const i8) -> i32 {
    unsafe {
        if path_ptr.is_null() {
            return -1;
        }
        let path = std::ffi::CStr::from_ptr(path_ptr).to_string_lossy().into_owned();
        if let Some(renderer_ptr) = super::RENDERER {
            match (*renderer_ptr).capture_screenshot(&path) {
                Ok(_) => 0,
                Err(_) => -2,
            }
        } else {
            -3
        }
    }
}

pub extern "C" fn set_widget_visible(handle: hezhou_ui::ffi::WidgetTreeHandle, widget_id: u64, visible: bool) {
    hezhou_ui::ffi::ui_set_widget_visible(handle, widget_id, visible);
}

// === Pipeline切换FFI函数 ===

#[unsafe(no_mangle)]
pub extern "C" fn get_pipeline_names(
    _handle: hezhou_scripting::ffi_context::WidgetTreeHandle,
    buffer: *mut std::ffi::c_char,
    buffer_size: usize,
) -> usize {
    unsafe {
        // 返回所有可用管线名（包括尚未注册的ray_tracing，switch_pipeline会懒注册）
        let all_names = ["rasterization", "ray_tracing"];
        let joined: String = all_names.join("\0");
        let bytes = joined.as_bytes();
        let copy_len = std::cmp::min(bytes.len(), buffer_size - 1);
        if copy_len > 0 && !buffer.is_null() {
            std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const std::ffi::c_char, buffer, copy_len);
            *buffer.add(copy_len) = 0;
        }
        dfx_info!("FFI", "get_pipeline_names: joined_len={}, copy_len={}, content={}", joined.len(), copy_len, joined.replace('\0', "|"));
        copy_len
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_pipeline(
    _handle: hezhou_scripting::ffi_context::WidgetTreeHandle,
    pipeline_name: *const std::ffi::c_char,
) -> i32 {
    unsafe {
        if pipeline_name.is_null() {
            return -1;
        }
        let name_str = std::ffi::CStr::from_ptr(pipeline_name).to_string_lossy().into_owned();
        if let Some(renderer_ptr) = super::RENDERER {
            if (*renderer_ptr).switch_pipeline(&name_str) {
                0  // 成功
            } else {
                -1  // 失败
            }
        } else {
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_active_pipeline_name(
    _handle: hezhou_scripting::ffi_context::WidgetTreeHandle,
    buffer: *mut std::ffi::c_char,
    buffer_size: usize,
) -> usize {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            let renderer = &mut *renderer_ptr;
            if let Some(registry) = renderer.pipeline_registry() {
                let name = registry.active_pipeline_name();
                let bytes = name.as_bytes();
                let copy_len = std::cmp::min(bytes.len(), buffer_size - 1);  // 保留1字节给末尾\0
                if copy_len > 0 && !buffer.is_null() {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const std::ffi::c_char, buffer, copy_len);
                    *buffer.add(copy_len) = 0;
                }
                copy_len
            } else {
                0
            }
        } else {
            0
        }
    }
}