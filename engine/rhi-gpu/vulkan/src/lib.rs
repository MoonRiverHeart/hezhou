pub mod context;
pub mod renderer;

use rhi::*;
use context::VulkanContext;
use renderer::VulkanRenderer;

pub struct VulkanRhi {
    renderer: VulkanRenderer,
    context: VulkanContext,
}

// Vulkan的所有操作都在主线程，标记Send+Sync是安全的
unsafe impl Send for VulkanRhi {}
unsafe impl Sync for VulkanRhi {}

impl Rhi for VulkanRhi {
    fn init(desc: &RhiInitDesc) -> Self {
        let context = VulkanContext::new(desc);
        let renderer = VulkanRenderer::new(&context);
        VulkanRhi { context, renderer }
    }
    
    fn begin_frame(&mut self) {
        self.context.begin_frame();
    }
    
    fn draw(&mut self, commands: &[DrawCommand]) {
        self.renderer.draw(&mut self.context, commands);
    }
    
    fn end_frame(&mut self) -> RenderResult {
        self.context.end_frame()
    }
    
    fn resize(&mut self, width: u32, height: u32) {
        self.context.resize(width, height);
    }
    
    fn framebuffer_size(&self) -> (u32, u32) {
        self.context.framebuffer_size()
    }
    
    fn wait_idle(&self) {
        self.context.wait_idle();
    }

    fn upload_texture(&mut self, data: &[u8], width: u32, height: u32) -> u32 {
        self.renderer.upload_texture(&self.context, data, width, height)
    }
}