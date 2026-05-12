use ash::vk;
use ash::khr::surface::Instance as SurfaceLoader;
use ash::khr::swapchain::Device as SwapchainLoader;

pub struct TriangleDemo {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
    max_frames_in_flight: usize,
    extent: vk::Extent2D,
    surface_loader: SurfaceLoader,
    swapchain_loader: SwapchainLoader,
}

impl TriangleDemo {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        unsafe {
            let entry = ash::Entry::load().map_err(|e| format!("Failed to load Vulkan: {}", e))?;
            
            let app_name = std::ffi::CString::new("Triangle Demo").unwrap();
            let engine_name = std::ffi::CString::new("Hezhou").unwrap();
            
            let app_info = vk::ApplicationInfo {
                p_application_name: app_name.as_ptr(),
                application_version: 1,
                p_engine_name: engine_name.as_ptr(),
                engine_version: 1,
                api_version: vk::API_VERSION_1_2,
                ..Default::default()
            };
            
            let extensions = [
                vk::KHR_SURFACE_NAME.as_ptr(),
                vk::KHR_WIN32_SURFACE_NAME.as_ptr(),
            ];
            
            let create_info = vk::InstanceCreateInfo {
                p_application_info: &app_info,
                pp_enabled_extension_names: extensions.as_ptr(),
                enabled_extension_count: extensions.len() as u32,
                ..Default::default()
            };
            
            let instance = entry.create_instance(&create_info, None)
                .map_err(|e| format!("Failed to create instance: {}", e))?;
            
            let surface_loader = SurfaceLoader::new(&entry, &instance);
            
            let win32_surface_loader = ash::khr::win32_surface::Instance::new(&entry, &instance);
            
            let surface_info = vk::Win32SurfaceCreateInfoKHR {
                hinstance: 0,
                hwnd: 0,
                ..Default::default()
            };
            
            let surface = win32_surface_loader.create_win32_surface(&surface_info, None)
                .map_err(|e| format!("Failed to create surface: {}", e))?;
            
            let physical_devices = instance.enumerate_physical_devices()
                .map_err(|e| format!("Failed to enumerate physical devices: {}", e))?;
            
            let physical_device = physical_devices[0];
            
            let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
            let graphics_queue_family = queue_families.iter()
                .position(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .unwrap() as u32;
            
            let queue_priority = 1.0f32;
            let queue_create_info = vk::DeviceQueueCreateInfo {
                queue_family_index: graphics_queue_family,
                queue_count: 1,
                p_queue_priorities: &queue_priority,
                ..Default::default()
            };
            
            let device_extensions = [vk::KHR_SWAPCHAIN_NAME.as_ptr()];
            
            let device_create_info = vk::DeviceCreateInfo {
                p_queue_create_infos: &queue_create_info,
                queue_create_info_count: 1,
                pp_enabled_extension_names: device_extensions.as_ptr(),
                enabled_extension_count: device_extensions.len() as u32,
                ..Default::default()
            };
            
            let device = instance.create_device(physical_device, &device_create_info, None)
                .map_err(|e| format!("Failed to create device: {}", e))?;
            
            let swapchain_loader = SwapchainLoader::new(&instance, &device);
            
            let queue = device.get_device_queue(graphics_queue_family, 0);
            
            let surface_formats = surface_loader.get_physical_device_surface_formats(physical_device, surface)
                .map_err(|e| format!("Failed to get surface formats: {}", e))?;
            
            let surface_caps = surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
                .map_err(|e| format!("Failed to get surface capabilities: {}", e))?;
            
            let present_modes = surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
                .map_err(|e| format!("Failed to get present modes: {}", e))?;
            
            let format = surface_formats[0].format;
            let color_space = surface_formats[0].color_space;
            
            let extent = if surface_caps.current_extent.width != u32::MAX {
                surface_caps.current_extent
            } else {
                vk::Extent2D { width, height }
            };
            
            let present_mode = *present_modes.iter()
                .find(|m| **m == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(&vk::PresentModeKHR::FIFO);
            
            let image_count = surface_caps.min_image_count + 1;
            
            let swapchain_create_info = vk::SwapchainCreateInfoKHR {
                surface,
                min_image_count: image_count,
                image_format: format,
                image_color_space: color_space,
                image_extent: extent,
                image_array_layers: 1,
                image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                pre_transform: surface_caps.current_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode,
                clipped: vk::TRUE,
                old_swapchain: vk::SwapchainKHR::null(),
                ..Default::default()
            };
            
            let swapchain = swapchain_loader.create_swapchain(&swapchain_create_info, None)
                .map_err(|e| format!("Failed to create swapchain: {}", e))?;
            
            let swapchain_images = swapchain_loader.get_swapchain_images(swapchain)
                .map_err(|e| format!("Failed to get swapchain images: {}", e))?;
            
            let swapchain_image_views: Vec<vk::ImageView> = swapchain_images.iter()
                .map(|image| {
                    let create_info = vk::ImageViewCreateInfo {
                        image: *image,
                        view_type: vk::ImageViewType::TYPE_2D,
                        format,
                        subresource_range: vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                        ..Default::default()
                    };
                    device.create_image_view(&create_info, None)
                        .map_err(|e| format!("Failed to create image view: {}", e))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            let color_attachment = vk::AttachmentDescription {
                format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            };
            
            let color_attachment_ref = vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            };
            
            let subpass = vk::SubpassDescription {
                pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
                color_attachment_count: 1,
                p_color_attachments: &color_attachment_ref,
                ..Default::default()
            };
            
            let render_pass_create_info = vk::RenderPassCreateInfo {
                attachment_count: 1,
                p_attachments: &color_attachment,
                subpass_count: 1,
                p_subpasses: &subpass,
                ..Default::default()
            };
            
            let render_pass = device.create_render_pass(&render_pass_create_info, None)
                .map_err(|e| format!("Failed to create render pass: {}", e))?;
            
            let framebuffers: Vec<vk::Framebuffer> = swapchain_image_views.iter()
                .map(|view| {
                    let create_info = vk::FramebufferCreateInfo {
                        render_pass,
                        attachment_count: 1,
                        p_attachments: &*view,
                        width: extent.width,
                        height: extent.height,
                        layers: 1,
                        ..Default::default()
                    };
                    device.create_framebuffer(&create_info, None)
                        .map_err(|e| format!("Failed to create framebuffer: {}", e))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            let vert_shader_code = include_bytes!("../../shaders/triangle.vert.spv");
            let frag_shader_code = include_bytes!("../../shaders/triangle.frag.spv");
            
            let vert_shader_module = Self::create_shader_module(&device, vert_shader_code)?;
            let frag_shader_module = Self::create_shader_module(&device, frag_shader_code)?;
            
            let main_name = std::ffi::CString::new("main").unwrap();
            
            let vert_stage = vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                module: vert_shader_module,
                p_name: main_name.as_ptr(),
                ..Default::default()
            };
            
            let frag_stage = vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: frag_shader_module,
                p_name: main_name.as_ptr(),
                ..Default::default()
            };
            
            let stages = [vert_stage, frag_stage];
            
            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default();
            
            let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                primitive_restart_enable: vk::FALSE,
                ..Default::default()
            };
            
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };
            
            let viewport_state = vk::PipelineViewportStateCreateInfo {
                viewport_count: 1,
                p_viewports: &viewport,
                scissor_count: 1,
                p_scissors: &scissor,
                ..Default::default()
            };
            
            let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
                depth_clamp_enable: vk::FALSE,
                rasterizer_discard_enable: vk::FALSE,
                polygon_mode: vk::PolygonMode::FILL,
                line_width: 1.0,
                cull_mode: vk::CullModeFlags::BACK,
                front_face: vk::FrontFace::CLOCKWISE,
                ..Default::default()
            };
            
            let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                sample_shading_enable: vk::FALSE,
                rasterization_samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            };
            
            let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
                color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A,
                blend_enable: vk::FALSE,
                ..Default::default()
            };
            
            let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
                logic_op_enable: vk::FALSE,
                attachment_count: 1,
                p_attachments: &color_blend_attachment,
                ..Default::default()
            };
            
            let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default();
            
            let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_create_info, None)
                .map_err(|e| format!("Failed to create pipeline layout: {}", e))?;
            
            let pipeline_create_info = vk::GraphicsPipelineCreateInfo {
                stage_count: 2,
                p_stages: stages.as_ptr(),
                p_vertex_input_state: &vertex_input_state,
                p_input_assembly_state: &input_assembly_state,
                p_viewport_state: &viewport_state,
                p_rasterization_state: &rasterization_state,
                p_multisample_state: &multisample_state,
                p_color_blend_state: &color_blend_state,
                layout: pipeline_layout,
                render_pass,
                subpass: 0,
                ..Default::default()
            };
            
            let pipelines = device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_create_info], None)
                .map_err(|(_, e)| format!("Failed to create pipeline: {}", e))?;
            
            let pipeline = pipelines[0];
            
            device.destroy_shader_module(vert_shader_module, None);
            device.destroy_shader_module(frag_shader_module, None);
            
            let command_pool_create_info = vk::CommandPoolCreateInfo {
                queue_family_index: graphics_queue_family,
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                ..Default::default()
            };
            
            let command_pool = device.create_command_pool(&command_pool_create_info, None)
                .map_err(|e| format!("Failed to create command pool: {}", e))?;
            
            let command_buffer_alloc_info = vk::CommandBufferAllocateInfo {
                command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: framebuffers.len() as u32,
                ..Default::default()
            };
            
            let command_buffers = device.allocate_command_buffers(&command_buffer_alloc_info)
                .map_err(|e| format!("Failed to allocate command buffers: {}", e))?;
            
            for (i, cmd) in command_buffers.iter().enumerate() {
                let begin_info = vk::CommandBufferBeginInfo::default();
                
                device.begin_command_buffer(*cmd, &begin_info)
                    .map_err(|e| format!("Failed to begin command buffer: {}", e))?;
                
                let clear_value = vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.1, 0.1, 0.1, 1.0],
                    },
                };
                
                let render_pass_begin_info = vk::RenderPassBeginInfo {
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
                
                device.cmd_begin_render_pass(*cmd, &render_pass_begin_info, vk::SubpassContents::INLINE);
                device.cmd_bind_pipeline(*cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
                device.cmd_draw(*cmd, 3, 1, 0, 0);
                device.cmd_end_render_pass(*cmd);
                
                device.end_command_buffer(*cmd)
                    .map_err(|e| format!("Failed to end command buffer: {}", e))?;
            }
            
            let max_frames_in_flight = 2;
            
            let image_available_semaphores: Vec<vk::Semaphore> = (0..max_frames_in_flight)
                .map(|_| {
                    let create_info = vk::SemaphoreCreateInfo::default();
                    device.create_semaphore(&create_info, None)
                        .map_err(|e| format!("Failed to create semaphore: {}", e))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            let render_finished_semaphores: Vec<vk::Semaphore> = (0..max_frames_in_flight)
                .map(|_| {
                    let create_info = vk::SemaphoreCreateInfo::default();
                    device.create_semaphore(&create_info, None)
                        .map_err(|e| format!("Failed to create semaphore: {}", e))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            let in_flight_fences: Vec<vk::Fence> = (0..max_frames_in_flight)
                .map(|_| {
                    let create_info = vk::FenceCreateInfo {
                        flags: vk::FenceCreateFlags::SIGNALED,
                        ..Default::default()
                    };
                    device.create_fence(&create_info, None)
                        .map_err(|e| format!("Failed to create fence: {}", e))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            Ok(Self {
                entry,
                instance,
                physical_device,
                device,
                queue,
                command_pool,
                render_pass,
                pipeline_layout,
                pipeline,
                surface,
                swapchain,
                swapchain_images,
                swapchain_image_views,
                framebuffers,
                command_buffers,
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
                current_frame: 0,
                max_frames_in_flight,
                extent,
                surface_loader,
                swapchain_loader,
            })
        }
    }
    
    unsafe fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule, String> {
        let spirv: Vec<u32> = code.chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: spirv.len() * 4,
            p_code: spirv.as_ptr(),
            ..Default::default()
        };
        
        device.create_shader_module(&create_info, None)
            .map_err(|e| format!("Failed to create shader module: {}", e))
    }
    
    pub fn draw_frame(&mut self) -> Result<bool, String> {
        unsafe {
            self.device.wait_for_fences(&[self.in_flight_fences[self.current_frame]], true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fence: {}", e))?;
            
            self.device.reset_fences(&[self.in_flight_fences[self.current_frame]])
                .map_err(|e| format!("Failed to reset fence: {}", e))?;
            
            let (image_index, _) = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            ).map_err(|e| format!("Failed to acquire next image: {}", e))?;
            
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            
            let submit_info = vk::SubmitInfo {
                wait_semaphore_count: 1,
                p_wait_semaphores: &self.image_available_semaphores[self.current_frame],
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: &self.command_buffers[image_index as usize],
                signal_semaphore_count: 1,
                p_signal_semaphores: &self.render_finished_semaphores[self.current_frame],
                ..Default::default()
            };
            
            self.device.queue_submit(self.queue, &[submit_info], self.in_flight_fences[self.current_frame])
                .map_err(|e| format!("Failed to submit queue: {}", e))?;
            
            let present_info = vk::PresentInfoKHR {
                wait_semaphore_count: 1,
                p_wait_semaphores: &self.render_finished_semaphores[self.current_frame],
                swapchain_count: 1,
                p_swapchains: &self.swapchain,
                p_image_indices: &image_index,
                ..Default::default()
            };
            
            self.swapchain_loader.queue_present(self.queue, &present_info)
                .map_err(|e| format!("Failed to present: {}", e))?;
            
            self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
            
            Ok(true)
        }
    }
}

impl Drop for TriangleDemo {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            
            self.device.destroy_command_pool(self.command_pool, None);
            
            for fence in &self.in_flight_fences {
                self.device.destroy_fence(*fence, None);
            }
            
            for semaphore in &self.image_available_semaphores {
                self.device.destroy_semaphore(*semaphore, None);
            }
            
            for semaphore in &self.render_finished_semaphores {
                self.device.destroy_semaphore(*semaphore, None);
            }
            
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            
            for fb in &self.framebuffers {
                self.device.destroy_framebuffer(*fb, None);
            }
            
            for view in &self.swapchain_image_views {
                self.device.destroy_image_view(*view, None);
            }
            
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
            
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}