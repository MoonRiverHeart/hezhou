use hezhou_rhi_vulkan::RotationRenderer;

fn main() {
    println!("=== Rust -> C# 旋转三角形 Demo ===\n");
    
    println!("[1] 创建 Vulkan Renderer + ScriptManager...");
    let mut renderer = RotationRenderer::new(800, 600, "Rotation Demo - Rust calls C#")
        .expect("Failed to create renderer");
    println!("    Renderer 和脚本系统初始化成功!\n");
    
    println!("[2] 运行渲染循环...");
    println!("    每帧调用 C# callback 'calculate_rotation' 计算旋转角度");
    println!("    旋转速度: 90 度/秒\n");
    
    let mut current_angle = 0.0f32;
    let mut frame_count = 0u32;
    
    loop {
        let running = renderer.draw_frame(&mut current_angle).expect("Draw frame failed");
        if !running {
            println!("\n    窗口关闭，停止渲染");
            break;
        }
        
        frame_count += 1;
        
        if frame_count % 60 == 0 {
            println!("    Frame {}: angle = {:.1}°", frame_count, current_angle);
        }
        
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    
    println!("\n=== Demo Complete ===");
    println!("\n说明:");
    println!("  - Rust 每帧调用 scripting callback 'calculate_rotation'");
    println!("  - Callback 模拟 C# 代码，返回角度增量 (90°/秒)");
    println!("  - Vulkan 使用 push constant 将角度传递给 shader");
    println!("  - Shader 在顶点着色器中应用旋转");
}