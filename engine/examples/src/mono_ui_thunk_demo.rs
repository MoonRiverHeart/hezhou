use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::time::{Duration, Instant};

pub extern "C" fn trigger_hot_reload() {}
pub extern "C" fn set_game_preview_extent(_width: u32, _height: u32) {}
pub extern "C" fn set_camera_params(_yaw: f32, _pitch: f32, _x: f32, _y: f32, _z: f32) {}
pub extern "C" fn register_key_stub(_ptr: *const std::ffi::c_void) {}
pub extern "C" fn register_mouse_move_stub(_ptr: *const std::ffi::c_void) {}
pub extern "C" fn set_preview_window_edit_mode_stub(_handle: WidgetTreeHandle, _id: u64, _mode: bool) {}
pub extern "C" fn scene_create_stub() -> *mut std::ffi::c_void { std::ptr::null_mut() }
pub extern "C" fn scene_destroy_stub(_scene: *mut std::ffi::c_void) {}
pub extern "C" fn scene_create_cube_stub(_scene: *mut std::ffi::c_void) -> u64 { 0 }
pub extern "C" fn scene_attach_script_stub(_scene: *mut std::ffi::c_void, _id: u64, _path: *const i8, _class: *const i8) {}
pub extern "C" fn scene_set_game_state_stub(_scene: *mut std::ffi::c_void, _state: i32) {}
pub extern "C" fn scene_get_game_state_stub(_scene: *mut std::ffi::c_void) -> i32 { 0 }
pub extern "C" fn scene_pick_entity_stub(_scene: *mut std::ffi::c_void, _ox: f32, _oy: f32, _oz: f32, _dx: f32, _dy: f32, _dz: f32) -> u64 { 0 }
pub extern "C" fn scene_select_entity_stub(_scene: *mut std::ffi::c_void, _id: u64) {}
pub extern "C" fn scene_update_stub(_scene: *mut std::ffi::c_void, _dt: f32) {}
pub extern "C" fn set_renderer_game_state_stub(_state: i32) {}
pub extern "C" fn get_renderer_game_state_stub() -> i32 { 0 }
pub extern "C" fn set_entity_transform_stub(_px: f32, _py: f32, _pz: f32, _rx: f32, _ry: f32, _rz: f32, _rw: f32, _sx: f32, _sy: f32, _sz: f32) {}
pub extern "C" fn set_entity_angle_stub(_angle: f32) {}
pub extern "C" fn get_entity_angle_stub() -> f32 { 0.0 }
pub extern "C" fn scene_get_entity_position_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: *mut f32, _y: *mut f32, _z: *mut f32) {}
pub extern "C" fn scene_get_entity_rotation_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: *mut f32, _y: *mut f32, _z: *mut f32, _w: *mut f32) {}
pub extern "C" fn scene_get_entity_scale_stub(_scene: *mut std::ffi::c_void, _id: u64, _x: *mut f32, _y: *mut f32, _z: *mut f32) {}
pub extern "C" fn scene_rotate_entity_stub(_scene: *mut std::ffi::c_void, _id: u64, _angle: f32) {}
pub extern "C" fn set_selected_entity_stub(_id: u64, _selected: bool) {}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot_mode = args.iter().any(|a| a == "--screenshot");
    let screenshot_delay = if screenshot_mode { 
        args.iter().position(|a| a == "--delay")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(2.0)
    } else { 0.0 };
    let screenshot_path = if screenshot_mode {
        args.iter().position(|a| a == "--output")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| "screenshots/mono_ui_thunk_demo.png".to_string())
    } else { String::new() };
    
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Debug);
    
    dfx_info!("Demo", "=== Thunk + Mono JIT UI Demo ===");
    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
        dfx_info!("Demo", "[Screenshot mode] delay={}s, output={}", screenshot_delay, screenshot_path);
    }
    dfx_info!("Demo", "[1] 初始化 Vulkan UI Renderer...");
    
    let mut renderer = UIVulkanRenderer::new(800, 600, "C# UI Demo")
        .expect("Failed to create renderer");
    dfx_info!("Demo", "Renderer初始化成功!");

    dfx_info!("Demo", "[2] 设置UI Root Panel...");
    renderer.setup_ui_for_script();
    dfx_info!("Demo", "Root Panel设置完成!");

    dfx_info!("Demo", "[3] 编译C#脚本...");
    compile_csharp_script();

    dfx_info!("Demo", "[4] 设置FFI Context...");
    let widget_tree_handle: WidgetTreeHandle = renderer.get_widget_tree_handle() as WidgetTreeHandle;
    
    let ffi_ctx = FfiContext {
        ui_get_primary_button_id: ui_ffi::ui_get_primary_button_id,
        ui_set_primary_button_id: ui_ffi::ui_set_primary_button_id,
        ui_widget_set_text: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_text as *const std::ffi::c_void) },
        ui_button_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_button_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_register_update_thunk_ptr: ui_ffi::ui_register_update_thunk_ptr,
        ui_register_resize_thunk_ptr: ui_ffi::ui_register_resize_thunk_ptr,
        ui_register_global_click_thunk_ptr: ui_ffi::ui_register_global_click_thunk_ptr,
        ui_register_key_thunk_ptr: register_key_stub,
        ui_register_mouse_move_thunk_ptr: register_mouse_move_stub,
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
        ui_set_preview_window_edit_mode: unsafe { std::mem::transmute(set_preview_window_edit_mode_stub as *const std::ffi::c_void) },
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
        scene_create: scene_create_stub,
        scene_destroy: scene_destroy_stub,
        scene_create_cube: scene_create_cube_stub,
        scene_attach_script: scene_attach_script_stub,
        scene_set_game_state: scene_set_game_state_stub,
        scene_get_game_state: scene_get_game_state_stub,
        scene_pick_entity: scene_pick_entity_stub,
        scene_select_entity: scene_select_entity_stub,
        scene_update: scene_update_stub,
        set_renderer_game_state: set_renderer_game_state_stub,
        get_renderer_game_state: get_renderer_game_state_stub,
        set_entity_transform: set_entity_transform_stub,
        set_entity_angle: set_entity_angle_stub,
        get_entity_angle: get_entity_angle_stub,
        scene_get_entity_position: scene_get_entity_position_stub,
        scene_get_entity_rotation: scene_get_entity_rotation_stub,
        scene_get_entity_scale: scene_get_entity_scale_stub,
        scene_rotate_entity: scene_rotate_entity_stub,
        set_selected_entity: set_selected_entity_stub,
        widget_tree_ptr: widget_tree_handle,
        dfx_handle: std::ptr::null_mut(),
        dfx_log: unsafe { std::mem::transmute(hezhou_dfx::dfx_log as *const std::ffi::c_void) },
        dfx_trace_begin: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_begin as *const std::ffi::c_void) },
        dfx_trace_end: unsafe { std::mem::transmute(hezhou_dfx::dfx_trace_end as *const std::ffi::c_void) },
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    dfx_info!("Demo", "FfiContext已设置, ptr={:?}", ffi_ptr);

    dfx_info!("Demo", "[5] 加载Mono DLL...");
    let dll_path = "scripts/bin/Mono/TestScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    dfx_info!("Demo", "加载成功!");

    dfx_info!("Demo", "[6] 调用Initialize...");
    executor.call_static_with_ptr("TestScript", "Initialize", ffi_ptr as usize)
        .expect("Initialize failed");
    dfx_info!("Demo", "Initialize调用成功!");

    dfx_info!("Demo", "[7] 开始主循环...");
    
    let start_time = Instant::now();
    let mut screenshot_taken = false;

    loop {
        renderer.process_events();
        
        if screenshot_mode && !screenshot_taken {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed >= screenshot_delay {
                dfx_info!("Screenshot", "Taking screenshot...");
                if let Err(e) = renderer.capture_screenshot(&screenshot_path) {
                    dfx_error!("Screenshot", "Failed: {}", e);
                } else {
                    dfx_info!("Screenshot", "Saved: {}", screenshot_path);
                }
                screenshot_taken = true;
                break;
            }
        }

        match renderer.draw_frame() {
            Ok(running) => {
                if !running {
                    break;
                }
            }
            Err(e) => {
                dfx_error!("Demo", "{}", e);
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(16));
    }

    dfx_info!("Demo", "[8] 清理资源...");
    
    if screenshot_mode {
        dfx_info!("Demo", "Screenshot mode - skipping cleanup");
    } else {
        renderer.cleanup();
    }
    dfx_info!("Demo", "=== Demo Complete ===");
}

fn compile_csharp_script() {
    use std::process::Command;
    
    let result = Command::new("mcs")
        .args([
            "-target:library",
            "-out:scripts/bin/Mono/TestScript.dll",
            "scripts/TestScript.cs",
            "scripts/UI.cs",
        ])
        .output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                dfx_info!("Demo", "TestScript.dll编译成功");
            } else {
                dfx_error!("Demo", "编译失败: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(_) => {
            dfx_info!("Demo", "mcs not found");
        }
    }
}