use hezhou_rhi_vulkan::VulkanDemo;

fn main() {
    println!("=== Vulkan Triangle Demo ===\n");

    println!("[1] Initializing Vulkan...");
    let demo = VulkanDemo::new().expect("Failed to create Vulkan demo");
    println!("    Vulkan initialized successfully!\n");

    println!("[2] Demo info:");
    println!("    Device: initialized");
    println!("    RenderPass: {:?}", demo.render_pass());
    println!("    Pipeline: {:?}", demo.pipeline());
    println!("    Queue: {:?}", demo.queue());
    println!();

    println!("=== Vulkan Triangle Demo Complete ===");
    println!("\nNote: This demo initializes Vulkan but doesn't render yet.");
    println!("      Full rendering requires swapchain + window integration.");
}
