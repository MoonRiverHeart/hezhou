use hezhou_rhi_vulkan::VulkanRenderer;

fn main() {
    println!("=== Vulkan Triangle Rendering Demo ===\n");

    println!("[1] Creating Vulkan renderer with GLFW window...");
    let mut renderer = VulkanRenderer::new(800, 600, "Vulkan Triangle - Hezhou Engine")
        .expect("Failed to create Vulkan renderer");
    println!("    Renderer created successfully!\n");

    println!("[2] Running main loop (100 frames)...");
    for frame in 1..=100 {
        let running = renderer.draw_frame().expect("Draw frame failed");
        if !running {
            println!("    Window closed at frame {}", frame);
            break;
        }
        if frame % 20 == 0 {
            println!("    Frame {} rendered", frame);
        }
    }
    println!();

    println!("=== Demo Complete ===");
}
