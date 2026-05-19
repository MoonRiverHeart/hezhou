use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_scripting::MonoUIExecutor;
use hezhou_ui::{Button, Widget, WidgetId};
use std::path::Path;
use std::time::{Duration, Instant};

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
            .unwrap_or_else(|| "screenshots/mono_ui_demo.png".to_string())
    } else { String::new() };

    println!("=== Mono UI + C# Click Callback Demo ===\n");

    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
        println!("[Screenshot mode] delay={}s, output={}\n", screenshot_delay, screenshot_path);
    }

    println!("[架构说明]");
    println!("  - Vulkan渲染Button控件，初始文字'button'");
    println!("  - Mono JIT执行C#脚本");
    println!("  - 按 **空格键** 触发C#回调");
    println!("  - C#调用FFI改变Button文字为'hello'\n");

    let scripts_dir = Path::new("scripts");

    println!("[1] 编译 C# UI脚本...");
    let build_script = scripts_dir.join("build_ui_mono.ps1");
    let build_result = std::process::Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-File"])
        .arg(&build_script)
        .current_dir(std::env::current_dir().unwrap())
        .output()
        .expect("Failed to run build script");

    if !build_result.status.success() {
        println!("    ERROR: C# compilation failed");
        println!("{}", String::from_utf8_lossy(&build_result.stderr));
        return;
    }

    let build_output = String::from_utf8_lossy(&build_result.stdout);
    let assembly_name = build_output
        .lines()
        .find(|line| line.contains("AssemblyName:"))
        .map(|line| line.split(':').last().unwrap().trim())
        .unwrap();

    println!("    C# DLL compiled: {}.dll", assembly_name);

    let dll_path = scripts_dir
        .join("bin/Mono/Release/net8.0")
        .join(format!("{}.dll", assembly_name));
    println!("    DLL path: {}", dll_path.display());

    println!("\n[2] 初始化 Vulkan UI Renderer...");
    let mut renderer =
        match UIVulkanRenderer::new(800, 600, "Mono UI Demo - Press SPACE to Change Button Text") {
            Ok(r) => r,
            Err(e) => {
                println!("ERROR: {}", e);
                return;
            }
        };

    println!("\n[3] 设置 UI 控件...");
    renderer.setup_ui();

    let button_id = renderer.get_button_id();
    println!("    Button ID: {}", button_id);

    println!("\n[4] 初始化 Mono JIT...");
    let mono_executor = match MonoUIExecutor::new(dll_path.to_str().unwrap()) {
        Ok(exec) => exec,
        Err(e) => {
            println!("ERROR: Mono init failed: {}", e);
            renderer.cleanup();
            return;
        }
    };

    println!("    Mono executor initialized");

    println!("\n[5] 开始主循环...");
    println!("    - 按 **空格键** 触发C# OnButtonClick");
    println!("    - Button文字将变为'hello'");
    println!("    - 按 ESC 或关闭窗口退出\n");

    let start_time = Instant::now();
    let mut screenshot_taken = false;

    loop {
        renderer.process_events();
        
        if screenshot_mode && !screenshot_taken {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed >= screenshot_delay {
                println!("    Taking screenshot...");
                if let Err(e) = renderer.capture_screenshot(&screenshot_path) {
                    println!("    ERROR: {}", e);
                } else {
                    println!("    Saved: {}", screenshot_path);
                }
                screenshot_taken = true;
                break;
            }
        }

        if renderer.is_space_pressed() {
            renderer.consume_space_press();

            let ui_system = renderer.get_ui_system();
            let ui = ui_system.lock();
            let tree = ui.get_widget_tree();
            let mut tree_guard = tree.lock();

            let widget_id = WidgetId::from_raw(button_id);

            if let Some(widget) = tree_guard.get_widget_mut(widget_id) {
                unsafe {
                    if let Some(button) =
                        (widget.as_mut() as *mut dyn Widget as *mut Button).as_mut()
                    {
                        button.set_text("hello");
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
                println!("ERROR: {}", e);
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(16));
    }

    println!("\n[6] 清理资源...");
    
    if screenshot_mode {
        println!("    Screenshot mode - skipping cleanup");
    } else {
        renderer.cleanup();
    }

    println!("\n=== Demo Complete ===");
}
