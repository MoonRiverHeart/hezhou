use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

static mut EXECUTOR: Option<MonoUIExecutor> = None;
static HOT_RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);
static mut RENDERER: Option<*mut UIVulkanRenderer> = None;
static mut SCENE: Option<*mut hezhou_core::Scene> = None;
static mut FFI_PTR: Option<*const hezhou_scripting::ffi_context::FfiContext> = None;
static mut STATUS_TEXT_CALLBACK: Option<hezhou_scripting::ffi_context::SetStatusTextFn> = None;
static mut HOT_RELOAD_COMPLETE_CALLBACK: Option<hezhou_scripting::ffi_context::OnHotReloadCompleteFn> = None;

#[derive(Clone, Debug)]
struct SavedScriptBinding {
    script_path: String,
    class_name: String,
    enabled: bool,
}

#[derive(Clone, Debug)]
struct SavedEntityBinding {
    entity_id: u64,
    bindings: Vec<SavedScriptBinding>,
}

static mut SAVED_BINDINGS: Vec<SavedEntityBinding> = Vec::new();

#[unsafe(no_mangle)]
pub extern "C" fn trigger_hot_reload() {
    HOT_RELOAD_REQUESTED.store(true, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_status_text(status_ptr: *const i8) {
    if !status_ptr.is_null() {
        let status = unsafe { std::ffi::CStr::from_ptr(status_ptr).to_string_lossy().into_owned() };
        dfx_info!("Status", "状态: {}", status);
    }
}

extern "C" fn on_hot_reload_complete_placeholder() {
    dfx_info!("HotReload", "OnHotReloadComplete callback placeholder called");
}

#[unsafe(no_mangle)]
pub extern "C" fn register_hot_reload_complete_callback(callback: hezhou_scripting::ffi_context::OnHotReloadCompleteFn) {
    unsafe {
        HOT_RELOAD_COMPLETE_CALLBACK = Some(callback);
        dfx_info!("HotReload", "Hot reload complete callback registered");
    }
}

fn save_scene_bindings() -> Vec<SavedEntityBinding> {
    unsafe {
        if let Some(scene_ptr) = SCENE {
            let scene = &*scene_ptr;
            let mut saved = Vec::new();
            
            for entity in &scene.root_entities {
                let bindings = scene.entity_bindings.get(&entity.id).cloned().unwrap_or_default();
                if !bindings.is_empty() {
                    let saved_bindings: Vec<SavedScriptBinding> = bindings.iter().map(|b| SavedScriptBinding {
                        script_path: b.script_path.clone(),
                        class_name: b.class_name.clone(),
                        enabled: b.enabled,
                    }).collect();
                    saved.push(SavedEntityBinding {
                        entity_id: entity.id,
                        bindings: saved_bindings,
                    });
                }
            }
            
            dfx_info!("HotReload", "保存绑定数据: {} entities, {} bindings", saved.len(), saved.iter().map(|e| e.bindings.len()).sum::<usize>());
            saved
        } else {
            Vec::new()
        }
    }
}

fn restore_scene_bindings(saved: &[SavedEntityBinding]) {
    unsafe {
        if let Some(scene_ptr) = SCENE {
            let scene = &mut *scene_ptr;
            
            for saved_entity in saved {
                let entity = hezhou_core::Entity::new(saved_entity.entity_id);
                for binding in &saved_entity.bindings {
                    scene.attach_script_binding(entity, binding.script_path.clone(), binding.class_name.clone());
                    if !binding.enabled {
                        let count = scene.get_script_binding_count(entity);
                        if count > 0 {
                            scene.set_script_binding_enabled(entity, count - 1, false);
                        }
                    }
                }
            }
            
            dfx_info!("HotReload", "恢复绑定数据: {} entities", saved.len());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_create_editor() -> *mut std::ffi::c_void {
    unsafe {
        let scene = Box::new(hezhou_core::Scene::new());
        let ptr = Box::into_raw(scene);
        SCENE = Some(ptr);
        ptr as *mut std::ffi::c_void
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_destroy_editor(scene: *mut std::ffi::c_void) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let _ = Box::from_raw(scene_ptr);
        SCENE = None;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_create_cube_editor(scene: *mut std::ffi::c_void) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = (*scene_ptr).create_cube();
        entity.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_attach_script_editor(scene: *mut std::ffi::c_void, entity_id: u64, 
                                             script_path: *const i8, class_name: *const i8) {
    if scene.is_null() {
        return;
    }
    let script_path_str = unsafe {
        std::ffi::CStr::from_ptr(script_path).to_string_lossy().into_owned()
    };
    let class_name_str = unsafe {
        std::ffi::CStr::from_ptr(class_name).to_string_lossy().into_owned()
    };
    
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        let script = hezhou_core::ScriptComponent::from_path(&script_path_str, &class_name_str);
        (*scene_ptr).attach_script(entity, script);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_game_state_editor(scene: *mut std::ffi::c_void, state: i32) {
    if scene.is_null() {
        return;
    }
    let state_enum = match state {
        0 => hezhou_core::GameState::Editing,
        1 => hezhou_core::GameState::Running,
        2 => hezhou_core::GameState::Paused,
        _ => hezhou_core::GameState::Editing,
    };
    
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).set_state(state_enum);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_game_state_editor(scene: *mut std::ffi::c_void) -> i32 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).state as i32
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_pick_entity_editor(scene: *mut std::ffi::c_void, 
                                           ox: f32, oy: f32, oz: f32,
                                           dx: f32, dy: f32, dz: f32) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let origin = hezhou_core::Vec3::new(ox, oy, oz);
        let dir = hezhou_core::Vec3::new(dx, dy, dz);
        match (*scene_ptr).pick_entity(origin, dir) {
            Some(e) => e.id,
            None => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_select_entity_editor(scene: *mut std::ffi::c_void, entity_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).select_entity(entity);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_update_editor(scene: *mut std::ffi::c_void, dt: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).update(dt);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_game_preview_extent(width: u32, height: u32) {
    unsafe {
        if let Some(renderer_ptr) = RENDERER {
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
        if let Some(renderer_ptr) = RENDERER {
            (*renderer_ptr).set_camera_params(yaw, pitch, x, y, z);
            dfx_info!("Demo", "Camera params set: yaw={}, pitch={}, pos=({}, {}, {})", yaw, pitch, x, y, z);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_renderer_game_state(state: i32) {
    unsafe {
        if let Some(renderer_ptr) = RENDERER {
            (*renderer_ptr).set_game_state(state);
            dfx_info!("Demo", "Renderer game_state set to: {}", state);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_renderer_game_state() -> i32 {
    unsafe {
        if let Some(renderer_ptr) = RENDERER {
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
        if let Some(renderer_ptr) = RENDERER {
            (*renderer_ptr).set_entity_transform(px, py, pz, rx, ry, rz, rw, sx, sy, sz);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_entity_angle(angle: f32) {
    unsafe {
        if let Some(renderer_ptr) = RENDERER {
            (*renderer_ptr).set_entity_angle(angle);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_entity_angle() -> f32 {
    unsafe {
        if let Some(renderer_ptr) = RENDERER {
            (*renderer_ptr).get_entity_angle()
        } else {
            0.0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_selected_entity(entity_id: u64, selected: bool) {
    unsafe {
        if let Some(renderer_ptr) = RENDERER {
            (*renderer_ptr).set_selected_entity(entity_id, selected);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_position_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                                    out_x: *mut f32, out_y: *mut f32, out_z: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_get_entity_position(scene as *mut hezhou_core::Scene, entity_id, out_x, out_y, out_z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_rotation_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                                    out_x: *mut f32, out_y: *mut f32, out_z: *mut f32, out_w: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() || out_w.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_get_entity_rotation(scene as *mut hezhou_core::Scene, entity_id, out_x, out_y, out_z, out_w);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_scale_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                                 out_x: *mut f32, out_y: *mut f32, out_z: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_get_entity_scale(scene as *mut hezhou_core::Scene, entity_id, out_x, out_y, out_z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_entity_position_editor(scene: *mut std::ffi::c_void, entity_id: u64, x: f32, y: f32, z: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_set_entity_position(scene as *mut hezhou_core::Scene, entity_id, x, y, z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_entity_scale_editor(scene: *mut std::ffi::c_void, entity_id: u64, x: f32, y: f32, z: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_set_entity_scale(scene as *mut hezhou_core::Scene, entity_id, x, y, z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_rotate_entity_editor(scene: *mut std::ffi::c_void, entity_id: u64, angle_degrees: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_rotate_entity(scene as *mut hezhou_core::Scene, entity_id, angle_degrees);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_entity_name_editor(scene: *mut std::ffi::c_void, entity_id: u64, name_ptr: *const i8) {
    if scene.is_null() || name_ptr.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_set_entity_name(scene as *mut hezhou_core::Scene, entity_id, name_ptr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_name_editor(scene: *mut std::ffi::c_void, entity_id: u64, buffer_ptr: *mut i8, buffer_size: usize) -> usize {
    if scene.is_null() || buffer_ptr.is_null() || buffer_size == 0 {
        return 0;
    }
    unsafe {
        hezhou_core::scene_get_entity_name(scene as *mut hezhou_core::Scene, entity_id, buffer_ptr, buffer_size)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_attach_script_binding_editor(scene: *mut std::ffi::c_void, entity_id: u64, 
                                                      script_path: *const i8, class_name: *const i8) {
    if scene.is_null() {
        return;
    }
    let script_path_str = unsafe {
        std::ffi::CStr::from_ptr(script_path).to_string_lossy().into_owned()
    };
    let class_name_str = unsafe {
        std::ffi::CStr::from_ptr(class_name).to_string_lossy().into_owned()
    };
    
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).attach_script_binding(entity, script_path_str, class_name_str);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_remove_script_binding_editor(scene: *mut std::ffi::c_void, entity_id: u64, script_index: usize) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).remove_script_binding(entity, script_index);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_script_binding_count_editor(scene: *mut std::ffi::c_void, entity_id: u64) -> usize {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).get_script_binding_count(entity)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_script_binding_info_editor(scene: *mut std::ffi::c_void, entity_id: u64, index: usize,
                                                         path_buffer: *mut i8, path_buffer_size: usize,
                                                         class_buffer: *mut i8, class_buffer_size: usize,
                                                         out_enabled: *mut bool) -> bool {
    if scene.is_null() || path_buffer.is_null() || class_buffer.is_null() || out_enabled.is_null() {
        return false;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        if let Some(binding) = (*scene_ptr).get_script_binding(entity, index) {
            let path_bytes = binding.script_path.as_bytes();
            let path_copy_len = path_bytes.len().min(path_buffer_size - 1);
            std::ptr::copy_nonoverlapping(path_bytes.as_ptr(), path_buffer as *mut u8, path_copy_len);
            *path_buffer.add(path_copy_len) = 0;
            
            let class_bytes = binding.class_name.as_bytes();
            let class_copy_len = class_bytes.len().min(class_buffer_size - 1);
            std::ptr::copy_nonoverlapping(class_bytes.as_ptr(), class_buffer as *mut u8, class_copy_len);
            *class_buffer.add(class_copy_len) = 0;
            
            *out_enabled = binding.enabled;
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_script_binding_enabled_editor(scene: *mut std::ffi::c_void, entity_id: u64, index: usize, enabled: bool) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).set_script_binding_enabled(entity, index, enabled);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_create_entity_editor(scene: *mut std::ffi::c_void) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = (*scene_ptr).create_entity();
        entity.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_count_editor(scene: *mut std::ffi::c_void) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).entity_count() as u64
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_id_editor(scene: *mut std::ffi::c_void, index: u64) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entities = &(*scene_ptr).root_entities;
        if index < entities.len() as u64 {
            entities[index as usize].id
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_remove_entity_editor(scene: *mut std::ffi::c_void, entity_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).remove_entity(entity);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot_mode = args.iter().any(|a| a == "--screenshot");
    let screenshot_delay = if screenshot_mode { 
        args.iter().position(|a| a == "--delay")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(3.0)
    } else { 0.0 };
    
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Info);
    dfx.lock().get_trace_analyzer().lock().enable();
    
    let log_path = format!("logs/hezhou_{}.log", chrono::Local::now().format("%Y-%m-%d"));
    std::fs::create_dir_all("logs").ok();
    if let Err(e) = dfx.lock().get_logger().lock().enable_file_output(&log_path) {
        dfx_error!("Demo", "Failed to enable file output: {}", e);
    }
    
    dfx_info!("Demo", "=== Hezhou Game Editor ===");
    dfx_info!("Demo", "Log file: {}", log_path);
    if screenshot_mode {
        dfx_info!("Demo", "Screenshot mode: delay={}s", screenshot_delay);
    }
    
    dfx_trace_begin!("Startup", "editor");
    dfx_info!("Demo", "[1] 创建编辑器窗口 (1280x720)...");
    
    dfx_trace_begin!("Window", "create");
    let mut renderer = UIVulkanRenderer::new(1280, 720, "Hezhou Game Editor")
        .expect("Failed to create renderer");
    unsafe { RENDERER = Some(&mut renderer as *mut UIVulkanRenderer); }
    dfx_trace_end!("Window", "create");
    dfx_info!("Demo", "窗口创建成功!");

    dfx_info!("Demo", "[2] 设置UI Root Panel...");
    dfx_trace_begin!("UI", "setup");
    renderer.setup_ui_for_script();
    ui_ffi::ui_set_screen_size(1280.0, 720.0);
    let content_scale = renderer.get_content_scale();
    ui_ffi::ui_set_content_scale(content_scale);
    dfx_trace_end!("UI", "setup");
    dfx_info!("Demo", "Content scale: {} (DPI: {})", content_scale, content_scale * 96.0);

    dfx_info!("Demo", "[3] 编译C#编辑器脚本...");
    dfx_trace_begin!("Script", "compile");
    compile_editor_script();
    dfx_trace_end!("Script", "compile");

    dfx_info!("Demo", "[4] 设置FFI Context...");
    let widget_tree_handle: WidgetTreeHandle = renderer.get_widget_tree_handle() as WidgetTreeHandle;
    
    let dfx_for_csharp = hezhou_dfx::dfx_create();
    hezhou_dfx::dfx_set_log_level(dfx_for_csharp, 2);
    
    let log_path_c = std::ffi::CString::new(log_path.clone()).unwrap();
    hezhou_dfx::dfx_enable_file_output(dfx_for_csharp, log_path_c.as_ptr());
    
    let ffi_ctx = FfiContext {
        ui_get_primary_button_id: ui_ffi::ui_get_primary_button_id,
        ui_set_primary_button_id: ui_ffi::ui_set_primary_button_id,
        ui_widget_set_text: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_text as *const std::ffi::c_void) },
        ui_button_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_button_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_register_update_thunk_ptr: ui_ffi::ui_register_update_thunk_ptr,
        ui_register_resize_thunk_ptr: ui_ffi::ui_register_resize_thunk_ptr,
        ui_register_global_click_thunk_ptr: ui_ffi::ui_register_global_click_thunk_ptr,
        ui_register_key_thunk_ptr: ui_ffi::ui_register_key_thunk_ptr,
        ui_register_mouse_move_thunk_ptr: ui_ffi::ui_register_mouse_move_thunk_ptr,
        ui_trigger_resize: ui_ffi::ui_trigger_resize,
        ui_get_screen_size: ui_ffi::ui_get_screen_size,
        ui_set_content_scale: ui_ffi::ui_set_content_scale,
        ui_get_content_scale: ui_ffi::ui_get_content_scale,
        ui_create_button: unsafe { std::mem::transmute(ui_ffi::ui_create_button as *const std::ffi::c_void) },
        ui_create_label: unsafe { std::mem::transmute(ui_ffi::ui_create_label as *const std::ffi::c_void) },
        ui_create_panel: unsafe { std::mem::transmute(ui_ffi::ui_create_panel as *const std::ffi::c_void) },
        ui_create_vstack: unsafe { std::mem::transmute(ui_ffi::ui_create_vstack as *const std::ffi::c_void) },
        ui_create_vstack_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_vstack_in_parent as *const std::ffi::c_void) },
        ui_create_hstack: unsafe { std::mem::transmute(ui_ffi::ui_create_hstack as *const std::ffi::c_void) },
        ui_create_hstack_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_hstack_in_parent as *const std::ffi::c_void) },
        ui_create_button_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_button_in_parent as *const std::ffi::c_void) },
        ui_create_label_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_label_in_parent as *const std::ffi::c_void) },
        ui_create_panel_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_panel_in_parent as *const std::ffi::c_void) },
        ui_create_preview_window: unsafe { std::mem::transmute(ui_ffi::ui_create_preview_window as *const std::ffi::c_void) },
        ui_set_preview_texture: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_texture as *const std::ffi::c_void) },
        ui_get_root_id: unsafe { std::mem::transmute(ui_ffi::ui_get_root_id as *const std::ffi::c_void) },
        ui_set_widget_layout: unsafe { std::mem::transmute(ui_ffi::ui_set_widget_layout as *const std::ffi::c_void) },
        ui_widget_set_position: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_position as *const std::ffi::c_void) },
        ui_widget_set_size: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_size as *const std::ffi::c_void) },
        ui_widget_set_layer: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_layer as *const std::ffi::c_void) },
        ui_widget_get_layer: unsafe { std::mem::transmute(ui_ffi::ui_widget_get_layer as *const std::ffi::c_void) },
        ui_remove_widget: unsafe { std::mem::transmute(ui_ffi::ui_remove_widget as *const std::ffi::c_void) },
        ui_create_text_edit: unsafe { std::mem::transmute(ui_ffi::ui_create_text_edit as *const std::ffi::c_void) },
        ui_create_text_edit_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_text_edit_in_parent as *const std::ffi::c_void) },
        ui_text_edit_set_text: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_set_text as *const std::ffi::c_void) },
        ui_text_edit_insert_char: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_insert_char as *const std::ffi::c_void) },
        ui_text_edit_delete_char: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_delete_char as *const std::ffi::c_void) },
        ui_text_edit_get_text_len: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_get_text_len as *const std::ffi::c_void) },
        ui_text_edit_get_text: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_get_text as *const std::ffi::c_void) },
        ui_text_edit_show_line_numbers: unsafe { std::mem::transmute(ui_ffi::ui_set_text_edit_show_line_numbers as *const std::ffi::c_void) },
        ui_trigger_hot_reload: trigger_hot_reload,
        ui_set_game_preview_extent: set_game_preview_extent,
        ui_set_camera_params: set_camera_params,
        ui_is_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_is_preview_window_selected as *const std::ffi::c_void) },
        ui_set_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_window_selected as *const std::ffi::c_void) },
        ui_set_preview_window_edit_mode: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_window_edit_mode as *const std::ffi::c_void) },
        ui_create_list: unsafe { std::mem::transmute(ui_ffi::ui_create_list as *const std::ffi::c_void) },
        ui_create_list_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_list_in_parent as *const std::ffi::c_void) },
        ui_create_list_item: unsafe { std::mem::transmute(ui_ffi::ui_create_list_item as *const std::ffi::c_void) },
        ui_create_list_item_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_list_item_in_parent as *const std::ffi::c_void) },
ui_list_item_set_text: unsafe { std::mem::transmute(ui_ffi::ui_list_item_set_text as *const std::ffi::c_void) },
        ui_list_item_set_font_size: unsafe { std::mem::transmute(ui_ffi::ui_list_item_set_font_size as *const std::ffi::c_void) },
        ui_create_dropdown: unsafe { std::mem::transmute(ui_ffi::ui_create_dropdown as *const std::ffi::c_void) },
        ui_dropdown_set_options: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_set_options as *const std::ffi::c_void) },
        ui_dropdown_set_selected: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_set_selected as *const std::ffi::c_void) },
        ui_dropdown_get_selected: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_get_selected as *const std::ffi::c_void) },
        ui_dropdown_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_dropdown_set_on_select_thunk_ptr as *const std::ffi::c_void) },
        ui_create_input_field: unsafe { std::mem::transmute(ui_ffi::ui_create_input_field as *const std::ffi::c_void) },
        ui_input_field_set_text: unsafe { std::mem::transmute(ui_ffi::ui_input_field_set_text as *const std::ffi::c_void) },
        ui_input_field_get_text: unsafe { std::mem::transmute(ui_ffi::ui_input_field_get_text as *const std::ffi::c_void) },
        ui_input_field_set_on_change_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_input_field_set_on_change_thunk_ptr as *const std::ffi::c_void) },
        ui_input_field_set_placeholder: unsafe { std::mem::transmute(ui_ffi::ui_input_field_set_placeholder as *const std::ffi::c_void) },
        ui_create_tab_widget: unsafe { std::mem::transmute(ui_ffi::ui_create_tab_widget as *const std::ffi::c_void) },
        ui_tab_widget_add_tab: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_add_tab as *const std::ffi::c_void) },
        ui_tab_widget_set_active: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_set_active as *const std::ffi::c_void) },
        ui_tab_widget_get_active: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_get_active as *const std::ffi::c_void) },
        ui_tab_widget_remove_tab: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_remove_tab as *const std::ffi::c_void) },
        ui_tab_widget_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_set_on_select_thunk_ptr as *const std::ffi::c_void) },
        ui_tab_widget_set_on_close_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_set_on_close_thunk_ptr as *const std::ffi::c_void) },
        ui_tab_widget_get_tab_count: unsafe { std::mem::transmute(ui_ffi::ui_tab_widget_get_tab_count as *const std::ffi::c_void) },
        ui_create_tree_view: unsafe { std::mem::transmute(ui_ffi::ui_create_tree_view as *const std::ffi::c_void) },
        ui_tree_view_add_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_add_node as *const std::ffi::c_void) },
        ui_tree_view_remove_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_remove_node as *const std::ffi::c_void) },
        ui_tree_view_set_selected: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_set_selected as *const std::ffi::c_void) },
        ui_tree_view_get_selected: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_get_selected as *const std::ffi::c_void) },
        ui_tree_view_expand_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_expand_node as *const std::ffi::c_void) },
        ui_tree_view_collapse_node: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_collapse_node as *const std::ffi::c_void) },
        ui_tree_view_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_set_on_select_thunk_ptr as *const std::ffi::c_void) },
        ui_tree_node_set_text: unsafe { std::mem::transmute(ui_ffi::ui_tree_node_set_text as *const std::ffi::c_void) },
        ui_tree_node_get_user_data: unsafe { std::mem::transmute(ui_ffi::ui_tree_node_get_user_data as *const std::ffi::c_void) },
        ui_tree_view_clear_selection: unsafe { std::mem::transmute(ui_ffi::ui_tree_view_clear_selection as *const std::ffi::c_void) },
        ui_create_popup_menu: unsafe { std::mem::transmute(ui_ffi::ui_create_popup_menu as *const std::ffi::c_void) },
        ui_popup_menu_add_item: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_add_item as *const std::ffi::c_void) },
        ui_popup_menu_add_separator: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_add_separator as *const std::ffi::c_void) },
        ui_popup_menu_show: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_show as *const std::ffi::c_void) },
        ui_popup_menu_hide: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_hide as *const std::ffi::c_void) },
        ui_popup_menu_is_visible: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_is_visible as *const std::ffi::c_void) },
        ui_popup_menu_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_popup_menu_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_create_grid_view: unsafe { std::mem::transmute(ui_ffi::ui_create_grid_view as *const std::ffi::c_void) },
        ui_grid_view_add_item: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_add_item as *const std::ffi::c_void) },
        ui_grid_view_remove_item: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_remove_item as *const std::ffi::c_void) },
        ui_grid_view_set_selected: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_set_selected as *const std::ffi::c_void) },
        ui_grid_view_get_selected: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_get_selected as *const std::ffi::c_void) },
        ui_grid_view_get_selected_user_data: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_get_selected_user_data as *const std::ffi::c_void) },
        ui_grid_view_clear: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_clear as *const std::ffi::c_void) },
        ui_grid_view_item_count: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_item_count as *const std::ffi::c_void) },
        ui_grid_view_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_grid_view_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_create_dialog: unsafe { std::mem::transmute(ui_ffi::ui_create_dialog as *const std::ffi::c_void) },
        ui_dialog_set_content: unsafe { std::mem::transmute(ui_ffi::ui_dialog_set_content as *const std::ffi::c_void) },
        ui_dialog_add_button: unsafe { std::mem::transmute(ui_ffi::ui_dialog_add_button as *const std::ffi::c_void) },
        ui_dialog_show: unsafe { std::mem::transmute(ui_ffi::ui_dialog_show as *const std::ffi::c_void) },
        ui_dialog_hide: unsafe { std::mem::transmute(ui_ffi::ui_dialog_hide as *const std::ffi::c_void) },
        ui_dialog_is_visible: unsafe { std::mem::transmute(ui_ffi::ui_dialog_is_visible as *const std::ffi::c_void) },
        ui_dialog_get_result: unsafe { std::mem::transmute(ui_ffi::ui_dialog_get_result as *const std::ffi::c_void) },
        ui_dialog_set_on_result_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_dialog_set_on_result_thunk_ptr as *const std::ffi::c_void) },
        ui_create_file_browser: unsafe { std::mem::transmute(ui_ffi::ui_create_file_browser as *const std::ffi::c_void) },
        ui_file_browser_set_path: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_path as *const std::ffi::c_void) },
        ui_file_browser_set_filter: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_filter as *const std::ffi::c_void) },
        ui_file_browser_navigate_up: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_navigate_up as *const std::ffi::c_void) },
        ui_file_browser_refresh: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_refresh as *const std::ffi::c_void) },
        ui_file_browser_get_selected_path: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_get_selected_path as *const std::ffi::c_void) },
        ui_file_browser_get_current_path: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_get_current_path as *const std::ffi::c_void) },
        ui_file_browser_set_on_select_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_on_select_thunk_ptr as *const std::ffi::c_void) },
        ui_file_browser_set_on_double_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_file_browser_set_on_double_click_thunk_ptr as *const std::ffi::c_void) },
        scene_create: scene_create_editor,
        scene_destroy: scene_destroy_editor,
        scene_create_cube: scene_create_cube_editor,
        scene_attach_script: scene_attach_script_editor,
        scene_set_game_state: scene_set_game_state_editor,
        scene_get_game_state: scene_get_game_state_editor,
        scene_pick_entity: scene_pick_entity_editor,
        scene_select_entity: scene_select_entity_editor,
        scene_update: scene_update_editor,
        set_renderer_game_state: set_renderer_game_state,
        get_renderer_game_state: get_renderer_game_state,
        set_entity_transform: set_entity_transform,
        set_entity_angle: set_entity_angle,
        get_entity_angle: get_entity_angle,
        scene_get_entity_position: scene_get_entity_position_editor,
        scene_get_entity_rotation: scene_get_entity_rotation_editor,
        scene_get_entity_scale: scene_get_entity_scale_editor,
        scene_set_entity_position: scene_set_entity_position_editor,
        scene_set_entity_scale: scene_set_entity_scale_editor,
        scene_rotate_entity: scene_rotate_entity_editor,
        scene_set_entity_name: scene_set_entity_name_editor,
        scene_get_entity_name: scene_get_entity_name_editor,
        set_selected_entity: set_selected_entity,
        scene_attach_script_binding: scene_attach_script_binding_editor,
        scene_remove_script_binding: scene_remove_script_binding_editor,
        scene_get_script_binding_count: scene_get_script_binding_count_editor,
        scene_get_script_binding_info: scene_get_script_binding_info_editor,
        scene_set_script_binding_enabled: scene_set_script_binding_enabled_editor,
        scene_create_entity: scene_create_entity_editor,
        scene_get_entity_count: scene_get_entity_count_editor,
        scene_get_entity_id: scene_get_entity_id_editor,
        scene_remove_entity: scene_remove_entity_editor,
        widget_tree_ptr: widget_tree_handle,
        dfx_handle: dfx_for_csharp as *mut std::ffi::c_void,
        dfx_log: unsafe { std::mem::transmute(hezhou_dfx::dfx_log as *const std::ffi::c_void) },
        dfx_trace_begin: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_begin as *const std::ffi::c_void) },
        dfx_trace_end: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_end as *const std::ffi::c_void) },
        set_status_text: set_status_text,
        on_hot_reload_complete: on_hot_reload_complete_placeholder,
        ui_debug_print_widget_tree: unsafe { std::mem::transmute(ui_ffi::ui_debug_print_widget_tree as *const std::ffi::c_void) },
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    unsafe { FFI_PTR = Some(ffi_ptr); }
    dfx_info!("Demo", "FfiContext已设置, ptr={:?}", ffi_ptr);

    dfx_info!("Demo", "[5] 加载Mono DLL...");
    dfx_trace_begin!("Mono", "load");
    let dll_path = "scripts/bin/Mono/EditorScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    dfx_trace_end!("Mono", "load");
    
    unsafe {
        EXECUTOR = Some(executor);
    }
    dfx_info!("Demo", "加载成功!");

    dfx_info!("Demo", "[6] 调用EditorScript.Initialize...");
    dfx_trace_begin!("Mono", "initialize");
    unsafe {
        if let Some(ref executor) = EXECUTOR {
            executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr as usize)
                .expect("Initialize failed");
        }
    }
    dfx_trace_end!("Mono", "initialize");
    dfx_trace_end!("Startup", "editor");
    dfx_info!("Demo", "编辑器UI创建成功!");

    let screenshot_path = if screenshot_mode {
        args.iter().position(|a| a == "--output")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| format!("screenshots/mono_editor_demo.png"))
    } else { String::new() };
    
    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
    }
    
    dfx_info!("Demo", "[7] 开始主循环...");
    dfx_info!("Demo", "Trace will be saved to traces/trace_latest.json on exit");

    let mut frame_count = 0u64;
    let start_time = Instant::now();
    let mut screenshot_taken = false;
    
    loop {
        frame_count += 1;
        dfx_trace_begin!("Frame", "render");
        
        renderer.process_events();
        
        if screenshot_mode && !screenshot_taken {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed >= screenshot_delay {
                dfx_trace_begin!("DrawFrame", "render");
                match renderer.draw_frame() {
                    Ok(_) => {
                        dfx_trace_end!("DrawFrame", "render");
                        dfx_trace_end!("Frame", "render");
                        dfx_info!("Screenshot", "Taking screenshot after {:.1}s...", elapsed);
                        if let Err(e) = renderer.capture_screenshot(&screenshot_path) {
                            dfx_error!("Screenshot", "Failed: {}", e);
                        } else {
                            dfx_info!("Screenshot", "Saved: {}", screenshot_path);
                        }
                        screenshot_taken = true;
                        break;
                    }
                    Err(e) => {
                        dfx_trace_end!("DrawFrame", "render");
                        dfx_trace_end!("Frame", "render");
                        dfx_error!("Demo", "Draw error: {}", e);
                        break;
                    }
                }
            }
        }
        
        if HOT_RELOAD_REQUESTED.load(Ordering::SeqCst) {
            HOT_RELOAD_REQUESTED.store(false, Ordering::SeqCst);
            dfx_info!("HotReload", "触发热更新...");
            dfx_trace_begin!("HotReload", "reload");
            
            unsafe {
                if let Some(ref mut executor) = EXECUTOR {
                    let ffi_ptr_val = FFI_PTR.unwrap_or(std::ptr::null());
                    
                    if let Some(callback) = STATUS_TEXT_CALLBACK {
                        let status_cstr = std::ffi::CString::new("正在热更新脚本...").unwrap();
                        callback(status_cstr.as_ptr());
                    }
                    
                    dfx_info!("HotReload", "[1] 保存Entity-Script绑定数据...");
                    let saved_bindings = save_scene_bindings();
                    SAVED_BINDINGS = saved_bindings;
                    
                    dfx_info!("HotReload", "[2] 清理旧的UI widgets...");
                    ui_ffi::ui_clear_widget_tree(widget_tree_handle as ui_ffi::WidgetTreeHandle);
                    
                    dfx_info!("HotReload", "[3] 卸载当前assembly...");
                    executor.shutdown();
                    
                    dfx_info!("HotReload", "[4] 重新编译C#脚本...");
                    let compile_result = recompile_editor_script();
                    
                    if !compile_result {
                        if let Some(callback) = STATUS_TEXT_CALLBACK {
                            let status_cstr = std::ffi::CString::new("热更新失败: 编译错误").unwrap();
                            callback(status_cstr.as_ptr());
                        }
                        dfx_error!("HotReload", "编译失败!");
                        
                        dfx_info!("HotReload", "尝试恢复旧assembly...");
                        executor.reload().ok();
                        executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr_val as usize).ok();
                        
                        dfx_trace_end!("HotReload", "reload");
                        continue;
                    }
                    
                    dfx_info!("HotReload", "[5] 加载新assembly...");
                    match executor.reload() {
                        Ok(_) => {
                            dfx_info!("HotReload", "Assembly reload成功!");
                            
                            dfx_info!("HotReload", "[6] 调用Initialize重建UI...");
                            executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr_val as usize)
                                .expect("Initialize failed");
                            
                            dfx_info!("HotReload", "[7] 恢复Entity-Script绑定数据...");
                            restore_scene_bindings(&SAVED_BINDINGS);
                            
                            dfx_info!("HotReload", "[8] 调用OnHotReloadComplete回调...");
                            if let Some(callback) = HOT_RELOAD_COMPLETE_CALLBACK {
                                callback();
                            }
                            
                            if let Some(callback) = STATUS_TEXT_CALLBACK {
                                let status_cstr = std::ffi::CString::new("热更新完成").unwrap();
                                callback(status_cstr.as_ptr());
                            }
                            
                            dfx_info!("HotReload", "UI重新初始化完成!");
                        }
                        Err(e) => {
                            if let Some(callback) = STATUS_TEXT_CALLBACK {
                                let status_cstr = std::ffi::CString::new(format!("热更新失败: {:?}", e).as_str()).unwrap();
                                callback(status_cstr.as_ptr());
                            }
                            dfx_error!("HotReload", "Reload失败: {:?}", e);
                        }
                    }
                }
            }
            dfx_trace_end!("HotReload", "reload");
        }

        dfx_trace_begin!("DrawFrame", "render");
        match renderer.draw_frame() {
            Ok(running) => {
                dfx_trace_end!("DrawFrame", "render");
                
                if !running {
                    dfx_trace_end!("Frame", "render");
                    break;
                }
            }
            Err(e) => {
                dfx_trace_end!("DrawFrame", "render");
                dfx_error!("Demo", "{}", e);
                break;
            }
        }

        dfx_trace_end!("Frame", "render");
        
        if frame_count % 300 == 0 {
            std::fs::create_dir_all("traces").ok();
            let trace_path = "traces/trace_latest.json";
            if let Err(e) = dfx.lock().get_trace_analyzer().lock().save_to_file(trace_path) {
                dfx_error!("Demo", "Failed to save trace: {}", e);
            }
            dfx.lock().get_trace_analyzer().lock().clear();
        }
        
        std::thread::sleep(Duration::from_millis(16));
    }

    dfx_info!("Demo", "[8] 清理资源...");
    
    unsafe {
        if let Some(ref mut executor) = EXECUTOR {
            dfx_info!("Demo", "先卸载Mono assembly...");
            executor.shutdown();
        }
        EXECUTOR = None;
    }
    
    dfx_info!("Demo", "清理UI widgets...");
    ui_ffi::ui_clear_widget_tree(widget_tree_handle as ui_ffi::WidgetTreeHandle);
    dfx_info!("Demo", "UI widgets清理完成");
    
    dfx_info!("Demo", "保存trace...");
    std::fs::create_dir_all("traces").ok();
    let trace_path = format!("traces/trace_{}.json", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    {
        let dfx_guard = dfx.lock();
        let trace_analyzer_arc = dfx_guard.get_trace_analyzer();
        let trace_analyzer = trace_analyzer_arc.lock();
        let result = trace_analyzer.save_to_file(&trace_path);
        // 先释放锁再输出日志
        drop(trace_analyzer);
        drop(dfx_guard);
        match result {
            Ok(_) => dfx_info!("Demo", "Trace saved to {}", trace_path),
            Err(e) => dfx_error!("Demo", "Failed to save trace: {}", e),
        }
    }
    dfx_info!("Demo", "Trace保存完成");
    
    dfx_info!("Demo", "=== Editor Closed ===");
    std::process::exit(0);
}

fn compile_editor_script() {
    use std::process::Command;
    
    // Check if mcs.bat exists (standalone release mode)
    if !std::path::Path::new("C:\\Program Files\\Mono\\bin\\mcs.bat").exists() {
        dfx_info!("Demo", "mcs.bat not found - standalone release mode, skipping compilation");
        return;
    }
    
    let result = Command::new("C:\\Program Files\\Mono\\bin\\mcs.bat")
        .args([
            "-target:library",
            "-out:scripts/bin/Mono/EditorScript.dll",
            "scripts/EditorScript.cs",
            "scripts/UI.cs",
            "scripts/DFX.cs",
        ])
        .output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                dfx_info!("Demo", "EditorScript.dll编译成功");
            } else {
                dfx_error!("Demo", "编译失败: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            dfx_error!("Demo", "mcs not found: {:?}", e);
        }
    }
}

fn recompile_editor_script() -> bool {
    use std::process::Command;
    
    if !std::path::Path::new("C:\\Program Files\\Mono\\bin\\mcs.bat").exists() {
        dfx_info!("HotReload", "mcs.bat not found - assuming precompiled DLL");
        return true;
    }
    
    dfx_info!("HotReload", "执行mcs编译...");
    
    let result = Command::new("C:\\Program Files\\Mono\\bin\\mcs.bat")
        .args([
            "-target:library",
            "-out:scripts/bin/Mono/EditorScript.dll",
            "scripts/EditorScript.cs",
            "scripts/UI.cs",
            "scripts/DFX.cs",
        ])
        .output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                dfx_info!("HotReload", "✓ 编译成功");
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                dfx_error!("HotReload", "✗ 编译失败:\n{}", stderr);
                false
            }
        }
        Err(e) => {
            dfx_error!("HotReload", "✗ mcs执行失败: {:?}", e);
            false
        }
    }
}