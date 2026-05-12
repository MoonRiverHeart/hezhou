use hezhou_rhi_vulkan::RotationRenderer;

fn main() {
    println!("=== Rust -> C# 函数指针调用 Demo ===\n");
    
    println!("[架构说明]");
    println!("  C# 端:");
    println!("    - 定义静态字段 rotation_speed = 90°/s");
    println!("    - 定义函数指针 CalculateRotationPtr");
    println!("    - [UnmanagedCallersOnly] 标记方法可被非托管代码调用");
    println!("    - CLR 自动生成 Thunk (跳板函数)");
    println!("  Rust 端:");
    println!("    - ROTATION_CALLBACK: Mutex<Option<extern \"C\" fn(f32) -> f32>>");
    println!("    - register_rotation_callback(thunk_ptr) 保存函数指针");
    println!("    - trigger_rotation_callback(dt) -> thunk(dt) -> C# CalculateRotation");
    println!("  调用路径:");
    println!("    Rust -> Thunk -> CLR 上下文切换 -> C# CalculateRotation -> 返回值\n");
    
    println!("[1] 创建 Vulkan Renderer...");
    let mut renderer = RotationRenderer::new(800, 600, "Rotation Demo - Rust calls C# thunk")
        .expect("Failed to create renderer");
    println!("    Renderer 初始化成功!\n");
    
    println!("[2] 运行渲染循环...");
    println!("    每帧调用 C# thunk 计算旋转角度\n");
    
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
}