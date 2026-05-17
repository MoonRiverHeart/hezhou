use hezhou_rhi_vulkan::TriangleDemo;

fn main() {
    println!("=== Vulkan Triangle Rendering Demo ===\n");

    println!("[1] Initializing Vulkan...");
    let mut demo = TriangleDemo::new(800, 600).expect("Failed to create Vulkan demo");
    println!("    Vulkan initialized!\n");

    println!("[2] Running main loop (100 frames)...");
    for frame in 1..=100 {
        let running = demo.draw_frame().expect("Draw frame failed");
        if !running {
            break;
        }

        if frame % 20 == 0 {
            println!("    Frame {} rendered", frame);
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    println!();

    println!("=== Demo Complete ===");
    println!("\nNote: This demo renders a triangle using Vulkan.");
    println!("      Surface created with dummy HWND (no visible window).");
}
