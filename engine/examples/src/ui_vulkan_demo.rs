use hezhou_rhi_vulkan::UIVulkanRenderer;

fn main() {
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

    println!("\n[3] 开始主循环...");
    println!("    - 点击按钮查看状态变化");
    println!("    - 按 ESC 或关闭窗口退出\n");

    let max_frames = 600u64;

    loop {
        renderer.process_events();

        match renderer.draw_frame() {
            Ok(running) => {
                if !running || renderer.get_frame_count() >= max_frames {
                    break;
                }
            }
            Err(e) => {
                println!("ERROR: {}", e);
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    println!("\n[4] 清理资源...");
    renderer.cleanup();

    println!("\n=== Demo Complete ===");
}
