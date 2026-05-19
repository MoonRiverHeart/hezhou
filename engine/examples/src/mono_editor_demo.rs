use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

static mut EXECUTOR: Option<MonoUIExecutor> = None;
static HOT_RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);
static mut RENDERER: Option<*mut UIVulkanRenderer> = None;

#[unsafe(no_mangle)]
pub extern "C" fn trigger_hot_reload() {
    HOT_RELOAD_REQUESTED.store(true, Ordering::SeqCst);
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
        ui_remove_widget: unsafe { std::mem::transmute(ui_ffi::ui_remove_widget as *const std::ffi::c_void) },
        ui_create_text_edit: unsafe { std::mem::transmute(ui_ffi::ui_create_text_edit as *const std::ffi::c_void) },
        ui_create_text_edit_in_parent: unsafe { std::mem::transmute(ui_ffi::ui_create_text_edit_in_parent as *const std::ffi::c_void) },
        ui_text_edit_set_text: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_set_text as *const std::ffi::c_void) },
        ui_text_edit_insert_char: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_insert_char as *const std::ffi::c_void) },
        ui_text_edit_delete_char: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_delete_char as *const std::ffi::c_void) },
        ui_text_edit_get_text_len: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_get_text_len as *const std::ffi::c_void) },
        ui_text_edit_get_text: unsafe { std::mem::transmute(ui_ffi::ui_text_edit_get_text as *const std::ffi::c_void) },
        ui_trigger_hot_reload: trigger_hot_reload,
        ui_set_game_preview_extent: set_game_preview_extent,
        ui_set_camera_params: set_camera_params,
        ui_is_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_is_preview_window_selected as *const std::ffi::c_void) },
        ui_set_preview_window_selected: unsafe { std::mem::transmute(ui_ffi::ui_set_preview_window_selected as *const std::ffi::c_void) },
        widget_tree_ptr: widget_tree_handle,
        dfx_handle: dfx_for_csharp as *mut std::ffi::c_void,
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
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
            dfx_info!("HotReload", "触发重载...");
            dfx_trace_begin!("HotReload", "reload");
            
            unsafe {
                if let Some(ref mut executor) = EXECUTOR {
                    dfx_info!("HotReload", "清理旧的UI...");
                    ui_ffi::ui_clear_widget_tree(widget_tree_handle as ui_ffi::WidgetTreeHandle);
                    
                    match executor.reload() {
                        Ok(_) => {
                            dfx_info!("HotReload", "Assembly reload成功!");
                            executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr as usize)
                                .expect("Initialize failed");
                            dfx_info!("HotReload", "UI重新初始化完成!");
                        }
                        Err(e) => {
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