use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use hezhou_dfx::*;
use std::time::Duration;

pub extern "C" fn trigger_hot_reload() {}

fn main() {
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Debug);
    
    dfx_info!("Demo", "=== Thunk + Mono JIT UI Demo ===");
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
        dfx_handle: std::ptr::null_mut(),
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

    loop {
        renderer.process_events();

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