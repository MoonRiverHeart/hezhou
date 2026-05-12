use ash::vk;
use ash::khr::surface::Instance as SurfaceLoader;
use ash::khr::swapchain::Device as SwapchainLoader;

pub struct VulkanRenderer {
    glfw: glfw::Glfw,
    window: glfw::PWindow,
    demo: crate::demo::VulkanDemo,
    surface: vk::SurfaceKHR,
    swapchain: crate::swapchain::VulkanSwapchain,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    surface_loader: SurfaceLoader,
}

impl VulkanRenderer {
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, hezhou_rhi::RhiError> {
        unsafe {
            let mut glfw = glfw::init(|_, _| {})
                .map_err(|e| hezhou_rhi::RhiError::SurfaceCreationFailed(e.to_string()))?;
            
            glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
            
            let (window, _events) = glfw.create_window(width, height, title, glfw::WindowMode::Windowed)
                .expect("Failed to create GLFW window");
            
            let demo = crate::demo::VulkanDemo::new()?;
            let device = demo.device();
            let instance = demo.instance();
            let entry = demo.entry();
            let physical_device = demo.physical_device();
            
            let surface_loader = SurfaceLoader::new(entry, instance);
            
            let surface = Self::create_surface(entry, instance, &window)?;
            
            let swapchain = crate::swapchain::VulkanSwapchain::new(
                entry,
                instance,
                physical_device,
                device,
                surface,
                demo.render_pass(),
                width,
                height,
            )?;
            
            let command_buffers = Self::create_command_buffers(
                device,
                demo.command_pool(),
                demo.render_pass(),
                swapchain.framebuffers(),
                demo.pipeline(),
                swapchain.extent(),
            )?;
            
            let image_available_semaphore = crate::context::create_semaphore(device)?;
            let render_finished_semaphore = crate::context::create_semaphore(device)?;
            let in_flight_fence = crate::context::create_fence(device, true)?;
            
            Ok(Self {
                glfw,
                window,
                demo,
                surface,
                swapchain,
                command_buffers,
                image_available_semaphore,
                render_finished_semaphore,
                in_flight_fence,
                surface_loader,
            })
        }
    }
    
    unsafe fn create_surface(entry: &ash::Entry, instance: &ash::Instance, window: &glfw::Window) -> Result<vk::SurfaceKHR, hezhou_rhi::RhiError> {
        use ash::khr::win32_surface::Instance as Win32SurfaceLoader;
        
        let win32_loader = Win32SurfaceLoader::new(entry, instance);
        
        let hwnd = window.get_win32_window() as isize;
        
        let surface_info = vk::Win32SurfaceCreateInfoKHR {
            hinstance: 0,
            hwnd,
            ..Default::default()
        };
        
        win32_loader.create_win32_surface(&surface_info, None)
            .map_err(|e| hezhou_rhi::RhiError::SurfaceCreationFailed(e.to_string()))
    }
    
    unsafe fn create_command_buffers(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        render_pass: vk::RenderPass,
        framebuffers: &[vk::Framebuffer],
        pipeline: vk::Pipeline,
        extent: vk::Extent2D,
    ) -> Result<Vec<vk::CommandBuffer>, hezhou_rhi::RhiError> {
        let alloc_info = vk::CommandBufferAllocateInfo {
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: framebuffers.len() as u32,
            ..Default::default()
        };
        
        let command_buffers = device.allocate_command_buffers(&alloc_info)
            .map_err(|e| hezhou_rhi::RhiError::CommandPoolCreationFailed(e.to_string()))?;
        
        for (i, cmd) in command_buffers.iter().enumerate() {
            let begin_info = vk::CommandBufferBeginInfo::default();
            
            device.begin_command_buffer(*cmd, &begin_info)
                .map_err(|e| hezhou_rhi::RhiError::InvalidOperation(e.to_string()))?;
            
            let clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            };
            
            let render_pass_begin = vk::RenderPassBeginInfo {
                render_pass,
                framebuffer: framebuffers[i],
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                },
                clear_value_count: 1,
                p_clear_values: &clear_value,
                ..Default::default()
            };
            
            device.cmd_begin_render_pass(*cmd, &render_pass_begin, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(*cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
            device.cmd_draw(*cmd, 3, 1, 0, 0);
            device.cmd_end_render_pass(*cmd);
            
            device.end_command_buffer(*cmd)
                .map_err(|e| hezhou_rhi::RhiError::InvalidOperation(e.to_string()))?;
        }
        
        Ok(command_buffers)
    }
    
    pub fn draw_frame(&mut self) -> Result<bool, hezhou_rhi::RhiError> {
        unsafe {
            self.glfw.poll_events();
            
            if self.window.should_close() {
                return Ok(false);
            }
            
            let device = self.demo.device();
            
            device.wait_for_fences(&[self.in_flight_fence], true, u64::MAX)
                .map_err(|e| hezhou_rhi::RhiError::InvalidOperation(e.to_string()))?;
            
            device.reset_fences(&[self.in_flight_fence])
                .map_err(|e| hezhou_rhi::RhiError::InvalidOperation(e.to_string()))?;
            
            let (image_index, _suboptimal) = self.swapchain.loader().acquire_next_image(
                self.swapchain.swapchain(),
                u64::MAX,
                self.image_available_semaphore,
                vk::Fence::null(),
            ).map_err(|e| hezhou_rhi::RhiError::InvalidOperation(e.to_string()))?;
            
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            
            let submit_info = vk::SubmitInfo {
                wait_semaphore_count: 1,
                p_wait_semaphores: &self.image_available_semaphore,
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: &self.command_buffers[image_index as usize],
                signal_semaphore_count: 1,
                p_signal_semaphores: &self.render_finished_semaphore,
                ..Default::default()
            };
            
            self.demo.device().queue_submit(self.demo.queue(), &[submit_info], self.in_flight_fence)
                .map_err(|e| hezhou_rhi::RhiError::InvalidOperation(e.to_string()))?;
            
            let present_info = vk::PresentInfoKHR {
                wait_semaphore_count: 1,
                p_wait_semaphores: &self.render_finished_semaphore,
                swapchain_count: 1,
                p_swapchains: &self.swapchain.swapchain(),
                p_image_indices: &image_index,
                ..Default::default()
            };
            
            self.swapchain.loader().queue_present(self.demo.queue(), &present_info)
                .map_err(|e| hezhou_rhi::RhiError::InvalidOperation(e.to_string()))?;
            
            Ok(true)
        }
    }
    
    pub fn window(&self) -> &glfw::PWindow {
        &self.window
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let device = self.demo.device();
            
            device.device_wait_idle().ok();
            
            device.destroy_semaphore(self.image_available_semaphore, None);
            device.destroy_semaphore(self.render_finished_semaphore, None);
            device.destroy_fence(self.in_flight_fence, None);
            
            device.free_command_buffers(self.demo.command_pool(), &self.command_buffers);
            
            self.swapchain.destroy(device);
            
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}