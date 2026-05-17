use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use std::time::Duration;

fn main() {
    println!("=== Hezhou Game Editor ===\n");
    
    println!("[布局说明]");
    println!("  - 顶部: 工具栏 (40px)");
    println!("  - 左侧: 项目结构 (250px)");
    println!("  - 左下: 资产管理 (200px)");
    println!("  - 中间: 游戏预览区域");
    println!("  - 右侧: 属性面板 (250px)");
    println!("  - 底部: 状态栏 (30px)\n");
    
    println!("[1] 创建编辑器窗口 (1280x720)...");
    let mut renderer = UIVulkanRenderer::new(1280, 720, "Hezhou Game Editor")
        .expect("Failed to create renderer");
    println!("    窗口创建成功!\n");

    println!("[2] 设置UI Root Panel...");
    renderer.setup_ui_for_script();
    println!("    Root Panel设置完成!\n");

    println!("[3] 编译C#编辑器脚本...");
    compile_editor_script();

    println!("[4] 设置FFI Context...");
    let widget_tree_handle: WidgetTreeHandle = renderer.get_widget_tree_handle() as WidgetTreeHandle;
    
    let ffi_ctx = FfiContext {
        ui_get_primary_button_id: ui_ffi::ui_get_primary_button_id,
        ui_set_primary_button_id: ui_ffi::ui_set_primary_button_id,
        ui_widget_set_text: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_text as *const std::ffi::c_void) },
        ui_button_set_on_click_thunk_ptr: unsafe { std::mem::transmute(ui_ffi::ui_button_set_on_click_thunk_ptr as *const std::ffi::c_void) },
        ui_register_update_thunk_ptr: ui_ffi::ui_register_update_thunk_ptr,
        ui_register_resize_thunk_ptr: ui_ffi::ui_register_resize_thunk_ptr,
        ui_trigger_resize: ui_ffi::ui_trigger_resize,
        ui_get_screen_size: ui_ffi::ui_get_screen_size,
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
        widget_tree_ptr: widget_tree_handle,
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    println!("    FfiContext已设置, ptr={:?}\n", ffi_ptr);

    println!("[5] 加载Mono DLL...");
    let dll_path = "scripts/bin/Mono/EditorScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    println!("    加载成功!\n");

    println!("[6] 调用EditorScript.Initialize...");
    executor.call_static_with_ptr_namespace("Hezhou", "EditorScript", "Initialize", ffi_ptr as usize)
        .expect("Initialize failed");
    println!("    编辑器UI创建成功!\n");

    println!("[7] 开始主循环...");
    println!("    FPS显示在状态栏\n");

    loop {
        renderer.process_events();

        match renderer.draw_frame() {
            Ok(running) => {
                if !running {
                    break;
                }
            }
            Err(e) => {
                println!("ERROR: {}", e);
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(16));
    }

    println!("\n[8] 清理资源...");
    renderer.cleanup();

    println!("\n=== Editor Closed ===");
}

fn compile_editor_script() {
    use std::process::Command;
    
    let result = Command::new("mcs")
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
                println!("    EditorScript.dll编译成功");
            } else {
                println!("    编译失败: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(_) => {
            println!("    mcs not found");
        }
    }
}