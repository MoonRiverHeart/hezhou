use ash::vk;

pub struct VulkanDemo {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    queue_family: u32,
    command_pool: vk::CommandPool,
    render_pass: vk::RenderPass,
    pipeline: crate::pipeline::VulkanPipeline,
}

impl VulkanDemo {
    pub fn new() -> Result<Self, hezhou_rhi::RhiError> {
        unsafe {
            let entry = ash::Entry::load()
                .map_err(|e| hezhou_rhi::RhiError::DeviceCreationFailed(e.to_string()))?;
            
            let instance = crate::context::create_instance(&entry, "Triangle Demo")?;
            
            let physical_device = crate::context::select_physical_device(&instance)?;
            
            let queue_family = crate::context::find_graphics_queue_family(&instance, physical_device)?;
            
            let (device, queue) = crate::context::create_device(&instance, physical_device, queue_family)?;
            
            let command_pool = crate::context::create_command_pool(&device, queue_family)?;
            
            let render_pass = crate::context::create_render_pass(&device, vk::Format::B8G8R8A8_UNORM)?;
            
            let pipeline = crate::pipeline::VulkanPipeline::new_simple_triangle(
                &device,
                render_pass,
                vk::Extent2D { width: 800, height: 600 },
            )?;
            
            Ok(Self {
                entry,
                instance,
                physical_device,
                device,
                queue,
                queue_family,
                command_pool,
                render_pass,
                pipeline,
            })
        }
    }
    
    pub fn entry(&self) -> &ash::Entry {
        &self.entry
    }
    
    pub fn instance(&self) -> &ash::Instance {
        &self.instance
    }
    
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }
    
    pub fn device(&self) -> &ash::Device {
        &self.device
    }
    
    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }
    
    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline.pipeline()
    }
    
    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool
    }
    
    pub fn queue(&self) -> vk::Queue {
        self.queue
    }
}

impl Drop for VulkanDemo {
    fn drop(&mut self) {
        unsafe {
            self.pipeline.destroy(&self.device);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}