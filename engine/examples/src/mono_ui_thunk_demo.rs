use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::{MonoUIExecutor, ffi_context::{FfiContext, WidgetTreeHandle}};
use hezhou_ui::ffi as ui_ffi;
use std::time::Duration;

fn main() {
    println!("=== Thunk + Mono JIT UI Demo ===\n");
    
    println!("[1] 初始化 Vulkan UI Renderer...");
    let mut renderer = UIVulkanRenderer::new(800, 600, "C# UI Demo - C# Creates Widgets!")
        .expect("Failed to create renderer");
    println!("    Renderer初始化成功!\n");

    println!("[2] 设置UI Root Panel (C#将创建控件)...");
    renderer.setup_ui_for_script();
    println!("    Root Panel设置完成!\n");

    println!("[3] 编译C#脚本...");
    compile_csharp_script();

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
        ui_widget_set_position: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_position as *const std::ffi::c_void) },
        ui_widget_set_size: unsafe { std::mem::transmute(ui_ffi::ui_widget_set_size as *const std::ffi::c_void) },
        widget_tree_ptr: widget_tree_handle,
    };
    hezhou_scripting::ffi_context::set_ffi_context(ffi_ctx);
    let ffi_ptr = hezhou_scripting::ffi_context::get_ffi_context_ptr();
    println!("    FfiContext已设置, ptr={:?}\n", ffi_ptr);

    println!("[5] 加载Mono DLL...");
    let dll_path = "scripts/bin/Mono/TestScript.dll";
    let executor = MonoUIExecutor::new(dll_path)
        .expect("Failed to load Mono DLL");
    println!("    加载成功!\n");

    println!("[6] 调用Initialize(ffiContextPtr)...");
    executor.call_static_with_ptr("TestScript", "Initialize", ffi_ptr as usize)
        .expect("Initialize failed");
    println!("    Initialize调用成功!\n");

    println!("[7] 开始主循环...");
    println!("    窗口显示: 第一行Label, 第二行Button\n");

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

    println!("\n=== Demo Complete ===");
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
                println!("    TestScript.dll编译成功");
            } else {
                println!("    编译失败: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(_) => {
            println!("    mcs not found");
        }
    }
}