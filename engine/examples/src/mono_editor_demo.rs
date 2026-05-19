use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

static mut EXECUTOR: Option<MonoUIExecutor> = None;
static HOT_RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn trigger_hot_reload() {
    HOT_RELOAD_REQUESTED.store(true, Ordering::SeqCst);
}

fn main() {
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Info);
    
    let log_path = format!("logs/hezhou_{}.log", chrono::Local::now().format("%Y-%m-%d"));
    std::fs::create_dir_all("logs").ok();
    if let Err(e) = dfx.lock().get_logger().lock().enable_file_output(&log_path) {
        dfx_error!("Demo", "Failed to enable file output: {}", e);
    }
    
    dfx_info!("Demo", "=== Hezhou Game Editor ===");
    dfx_info!("Demo", "Log file: {}", log_path);
    dfx_info!("Demo", "[1] 创建编辑器窗口 (1280x720)...");
    
    let mut renderer = UIVulkanRenderer::new(1280, 720, "Hezhou Game Editor")
        .expect("Failed to create renderer");
    dfx_info!("Demo", "窗口创建成功!");

    dfx_info!("Demo", "[2] 设置UI Root Panel...");
    renderer.setup_ui_for_script();
    ui_ffi::ui_set_screen_size(1280.0, 720.0);
    let content_scale = renderer.get_content_scale();
    ui_ffi::ui_set_content_scale(content_scale);
    dfx_info!("Demo", "Content scale: {} (DPI: {})", content_scale, content_scale * 96.0);

    dfx_info!("Demo", "[3] 编译C#编辑器脚本...");
    compile_editor_script();

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
        widget_tree_ptr: widget_tree_handle,
        dfx_handle: dfx_for_csharp as *mut std::ffi::c_void,
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    dfx_info!("Demo", "FfiContext已设置, ptr={:?}", ffi_ptr);

    dfx_info!("Demo", "[5] 加载Mono DLL...");
    let dll_path = "scripts/bin/Mono/EditorScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    
    unsafe {
        EXECUTOR = Some(executor);
    }
    dfx_info!("Demo", "加载成功!");

    dfx_info!("Demo", "[6] 调用EditorScript.Initialize...");
    unsafe {
        if let Some(ref executor) = EXECUTOR {
            executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr as usize)
                .expect("Initialize failed");
        }
    }
    dfx_info!("Demo", "编辑器UI创建成功!");

    dfx_info!("Demo", "[7] 开始主循环...");

    loop {
        renderer.process_events();
        
        if HOT_RELOAD_REQUESTED.load(Ordering::SeqCst) {
            HOT_RELOAD_REQUESTED.store(false, Ordering::SeqCst);
            dfx_info!("HotReload", "触发重载...");
            
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
    renderer.cleanup();
    dfx_info!("Demo", "=== Editor Closed ===");
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