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
        dfx_info!("Status", "状态: {}", status);
    }
}

pub extern "C" fn on_hot_reload_complete_placeholder() {
    dfx_info!("HotReload", "OnHotReloadComplete callback placeholder called");
}

#[unsafe(no_mangle)]
pub extern "C" fn register_hot_reload_complete_callback(callback: OnHotReloadCompleteFn) {
    unsafe {
        super::HOT_RELOAD_COMPLETE_CALLBACK = Some(callback);
        dfx_info!("HotReload", "Hot reload complete callback registered");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_game_preview_extent(width: u32, height: u32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            if let Err(e) = (*renderer_ptr).set_game_preview_extent(width, height) {
                dfx_error!("Demo", "Failed to set game preview extent: {}", e);
            } else {
                dfx_info!("Demo", "Game preview extent set to {}x{}", width, height);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_camera_params(yaw: f32, pitch: f32, x: f32, y: f32, z: f32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).set_camera_params(yaw, pitch, x, y, z);
            dfx_info!("Demo", "Camera params set: yaw={}, pitch={}, pos=({}, {}, {})", yaw, pitch, x, y, z);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_renderer_game_state(state: i32) {
    unsafe {
        if let Some(renderer_ptr) = super::RENDERER {
            (*renderer_ptr).set_game_state(state);
            dfx_info!("Demo", "Renderer game_state set to: {}", state);
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