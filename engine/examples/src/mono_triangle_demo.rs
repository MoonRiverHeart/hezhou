use hezhou_rhi_vulkan::MonoRotationRenderer;

fn main() {
    println!("=== Mono JIT 三角形旋转 + 热更新 Demo ===\n");

    println!("[架构说明]");
    println!("  - Vulkan 渲染三角形");
    println!("  - Mono JIT 调用 C# 计算旋转角度");
    println!("  - 按 R 键触发热更新");
    println!("  - 修改 RotationScript.cs 中的 _rotationSpeed");
    println!("  - 自动重新编译并 reload\n");

    println!("[1] 创建 Vulkan + Mono Renderer...");
    let mut renderer = MonoRotationRenderer::new(800, 600, "Mono Triangle - Press R to HotReload")
        .expect("Failed to create renderer");
    println!("    Renderer 初始化成功!\n");

    println!("[2] 运行渲染循环...");
    println!("    按 R 键触发热更新\n");

    let mut current_angle = 0.0f32;

    loop {
        let running = renderer
            .draw_frame(&mut current_angle)
            .expect("Draw frame failed");
        if !running {
            println!("\n    窗口关闭，停止渲染");
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    println!("\n=== Demo Complete ===");
}
