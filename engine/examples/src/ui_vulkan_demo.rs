use hezhou_rhi_vulkan::UIVulkanRenderer;
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
            .unwrap_or_else(|| "screenshots/ui_vulkan_demo.png".to_string())
    } else { String::new() };

    println!("=== UI Vulkan Rendering Demo ===\n");

    println!("[1] 初始化 GLFW + Vulkan...");
    let mut renderer = match UIVulkanRenderer::new(800, 600, "Hezhou UI - Vulkan Demo") {
        Ok(r) => r,
        Err(e) => {
            println!("ERROR: {}", e);
            return;
        }
    };

    println!("\n[2] 设置 UI 控件...");
    renderer.setup_ui();

    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
        println!("\n[Screenshot mode] delay={}s, output={}", screenshot_delay, screenshot_path);
    }

    println!("\n[3] 开始主循环...");
    println!("    - 点击按钮查看状态变化");
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

    println!("\n[4] 清理资源...");
    
    if screenshot_mode {
        println!("    Screenshot mode - skipping cleanup");
    } else {
        renderer.cleanup();
    }

    println!("\n=== Demo Complete ===");
}
