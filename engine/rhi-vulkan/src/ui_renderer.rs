use ash::vk;
use hezhou_rhi::*;
use hezhou_rhi::error::RhiError;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct VulkanUIRenderer {
    device: Arc<Mutex<ash::Device>>,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,
    
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    
    current_target: Option<UIRenderTarget>,
}

impl VulkanUIRenderer {
    pub fn new(
        device: Arc<Mutex<ash::Device>>,
        instance: ash::Instance,
        physical_device: vk::PhysicalDevice
    ) -> Result<Self, RhiError> {
        let device_guard = device.lock();
        
        let descriptor_set_layout = Self::create_descriptor_set_layout(&device_guard)?;
        let pipeline_layout = Self::create_pipeline_layout(&device_guard, descriptor_set_layout)?;
        let render_pass = Self::create_render_pass(&device_guard)?;
        let pipeline = Self::create_pipeline(&device_guard, pipeline_layout, render_pass)?;
        
        let (vertex_buffer, vertex_buffer_memory) = Self::create_buffer(
            &device_guard, &instance, physical_device, 1024 * 1024, vk::BufferUsageFlags::VERTEX_BUFFER
        )?;
        let (index_buffer, index_buffer_memory) = Self::create_buffer(
            &device_guard, &instance, physical_device, 512 * 1024, vk::BufferUsageFlags::INDEX_BUFFER
        )?;
        
        let (descriptor_pool, descriptor_set) = 
            Self::create_descriptor_sets(&device_guard, descriptor_set_layout)?;
        
        let command_pool = Self::create_command_pool(&device_guard)?;
        let command_buffer = Self::allocate_command_buffer(&device_guard, command_pool)?;
        
        drop(device_guard);
        
        Ok(Self {
            device,
            instance,
            physical_device,
            pipeline,
            pipeline_layout,
            render_pass,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
            command_pool,
            command_buffer,
            current_target: None,
        })
    }
    
    fn create_pipeline_layout(device: &ash::Device, set_layout: vk::DescriptorSetLayout) -> Result<vk::PipelineLayout, RhiError> {
        let push_constant_range = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: 16,
        };
        
        let layout_info = vk::PipelineLayoutCreateInfo {
            push_constant_range_count: 1,
            p_push_constant_ranges: &push_constant_range,
            set_layout_count: 1,
            p_set_layouts: &set_layout,
            ..Default::default()
        };
        
        unsafe {
            device.create_pipeline_layout(&layout_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to create pipeline layout: {}", e)))
        }
    }
    
    fn create_descriptor_set_layout(device: &ash::Device) -> Result<vk::DescriptorSetLayout, RhiError> {
        let binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            p_immutable_samplers: std::ptr::null(),
            _marker: std::marker::PhantomData,
        };
        
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: 1,
            p_bindings: &binding,
            ..Default::default()
        };
        
        unsafe {
            device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to create descriptor set layout: {}", e)))
        }
    }
    
    fn create_render_pass(device: &ash::Device) -> Result<vk::RenderPass, RhiError> {
        let color_attachment = vk::AttachmentDescription {
            format: vk::Format::R8G8B8A8_SRGB,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
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
        
        let render_pass_info = vk::RenderPassCreateInfo {
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            ..Default::default()
        };
        
        unsafe {
            device.create_render_pass(&render_pass_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to create render pass: {}", e)))
        }
    }
    
    fn create_pipeline(
        device: &ash::Device, 
        layout: vk::PipelineLayout, 
        render_pass: vk::RenderPass
    ) -> Result<vk::Pipeline, RhiError> {
        let vert_shader = Self::create_shader_module(device, include_bytes!("../../shaders/ui/ui.vert.spv"))?;
        let frag_shader = Self::create_shader_module(device, include_bytes!("../../shaders/ui/ui.frag.spv"))?;
        
        let main_name = std::ffi::CString::new("main").unwrap();
        
        let vert_stage = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vert_shader,
            p_name: main_name.as_ptr(),
            ..Default::default()
        };
        
        let frag_stage = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: frag_shader,
            p_name: main_name.as_ptr(),
            ..Default::default()
        };
        
        let stages = [vert_stage, frag_stage];
        
        let vertex_input_binding = vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<UIVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        };
        
        let vertex_input_attributes = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 8,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: 24,
            },
        ];
        
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &vertex_input_binding,
            vertex_attribute_description_count: 3,
            p_vertex_attribute_descriptions: &vertex_input_attributes[0],
            ..Default::default()
        };
        
        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };
        
        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };
        
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            ..Default::default()
        };
        
        let multisample_state = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A,
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
        };
        
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            ..Default::default()
        };
        
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: 2,
            p_dynamic_states: &dynamic_states[0],
            ..Default::default()
        };
        
        let pipeline_info = vk::GraphicsPipelineCreateInfo {
            stage_count: 2,
            p_stages: &stages[0],
            p_vertex_input_state: &vertex_input_state,
            p_input_assembly_state: &input_assembly_state,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterization_state,
            p_multisample_state: &multisample_state,
            p_color_blend_state: &color_blend_state,
            p_dynamic_state: &dynamic_state,
            layout,
            render_pass,
            subpass: 0,
            ..Default::default()
        };
        
        let pipeline = unsafe {
            device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| RhiError::InvalidOperation(format!("Failed to create pipeline: {}", e)))?[0]
        };
        
        unsafe {
            device.destroy_shader_module(vert_shader, None);
            device.destroy_shader_module(frag_shader, None);
        }
        
        Ok(pipeline)
    }
    
    fn create_shader_module(device: &ash::Device, spirv: &[u8]) -> Result<vk::ShaderModule, RhiError> {
        let code: Vec<u32> = spirv.chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: code.len() * 4,
            p_code: code.as_ptr(),
            ..Default::default()
        };
        
        unsafe {
            device.create_shader_module(&create_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to create shader module: {}", e)))
        }
    }
    
    fn create_buffer(
        device: &ash::Device, 
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice, 
        size: vk::DeviceSize, 
        usage: vk::BufferUsageFlags
    ) -> Result<(vk::Buffer, vk::DeviceMemory), RhiError> {
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        
        let buffer = unsafe {
            device.create_buffer(&buffer_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to create buffer: {}", e)))?
        };
        
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        
        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        let mem_type_index = Self::find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &mem_properties
        );
        
        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index: mem_type_index,
            ..Default::default()
        };
        
        let memory = unsafe {
            device.allocate_memory(&alloc_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to allocate memory: {}", e)))?
        };
        
        unsafe {
            device.bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to bind buffer memory: {}", e)))?;
        }
        
        Ok((buffer, memory))
    }
    
    fn find_memory_type(
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
        mem_properties: &vk::PhysicalDeviceMemoryProperties
    ) -> u32 {
        for i in 0..mem_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0 
                && mem_properties.memory_types[i as usize].property_flags.contains(properties) {
                return i;
            }
        }
        0
    }
    
    fn create_descriptor_sets(device: &ash::Device, set_layout: vk::DescriptorSetLayout) -> Result<(vk::DescriptorPool, vk::DescriptorSet), RhiError> {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 10,
            },
        ];
        
        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: 1,
            p_pool_sizes: &pool_sizes[0],
            max_sets: 10,
            ..Default::default()
        };
        
        let pool = unsafe {
            device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to create descriptor pool: {}", e)))?
        };
        
        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: pool,
            descriptor_set_count: 1,
            p_set_layouts: &set_layout,
            ..Default::default()
        };
        
        let sets = unsafe {
            device.allocate_descriptor_sets(&alloc_info)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to allocate descriptor sets: {}", e)))?
        };
        
        Ok((pool, sets[0]))
    }
    
    fn create_command_pool(device: &ash::Device) -> Result<vk::CommandPool, RhiError> {
        let pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            ..Default::default()
        };
        
        unsafe {
            device.create_command_pool(&pool_info, None)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to create command pool: {}", e)))
        }
    }
    
    fn allocate_command_buffer(device: &ash::Device, pool: vk::CommandPool) -> Result<vk::CommandBuffer, RhiError> {
        let alloc_info = vk::CommandBufferAllocateInfo {
            command_pool: pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };
        
        let buffers = unsafe {
            device.allocate_command_buffers(&alloc_info)
                .map_err(|e| RhiError::InvalidOperation(format!("Failed to allocate command buffer: {}", e)))?
        };
        
        Ok(buffers[0])
    }
}

impl UIRenderer for VulkanUIRenderer {
    fn create_render_target(&mut self, width: u32, height: u32) -> Result<UIRenderTarget, RhiError> {
        Ok(UIRenderTarget::new(width, height))
    }
    
    fn destroy_render_target(&mut self, _target: UIRenderTarget) {}
    
    fn begin_frame(&mut self, target: &UIRenderTarget) {
        self.current_target = Some(UIRenderTarget::new(target.width(), target.height()));
        
        let device = self.device.lock();
        unsafe {
            device.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::RELEASE_RESOURCES)
                .expect("Failed to reset command buffer");
            
            device.begin_command_buffer(self.command_buffer, &vk::CommandBufferBeginInfo::default())
                .expect("Failed to begin command buffer");
        }
    }
    
    fn end_frame(&mut self) {
        let device = self.device.lock();
        unsafe {
            device.end_command_buffer(self.command_buffer)
                .expect("Failed to end command buffer");
        }
        
        self.current_target = None;
    }
    
    fn draw(&mut self, data: &UIDrawData) {
        if data.vertices.is_empty() {
            return;
        }
        
        let device = self.device.lock();
        
        unsafe {
            let vertex_data = data.vertex_data();
            let vertex_ptr = device.map_memory(self.vertex_buffer_memory, 0, vertex_data.len() as vk::DeviceSize, vk::MemoryMapFlags::empty())
                .expect("Failed to map vertex memory");
            
            std::ptr::copy_nonoverlapping(vertex_data.as_ptr(), vertex_ptr as *mut u8, vertex_data.len());
            device.unmap_memory(self.vertex_buffer_memory);
            
            let index_data = data.index_data();
            let index_ptr = device.map_memory(self.index_buffer_memory, 0, index_data.len() as vk::DeviceSize, vk::MemoryMapFlags::empty())
                .expect("Failed to map index memory");
            
            std::ptr::copy_nonoverlapping(index_data.as_ptr(), index_ptr as *mut u8, index_data.len());
            device.unmap_memory(self.index_buffer_memory);
        }
        
        unsafe {
            device.cmd_bind_pipeline(self.command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(self.command_buffer, 0, &[self.vertex_buffer], &[0]);
            device.cmd_bind_index_buffer(self.command_buffer, self.index_buffer, 0, vk::IndexType::UINT32);
            
            if let Some(target) = &self.current_target {
                let viewport = vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: target.width() as f32,
                    height: target.height() as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                };
                
                let scissor = vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D { width: target.width(), height: target.height() },
                };
                
                device.cmd_set_viewport(self.command_buffer, 0, &[viewport]);
                device.cmd_set_scissor(self.command_buffer, 0, &[scissor]);
                
                let push_constants = [target.width() as f32, target.height() as f32, 0.0, 0.0];
                device.cmd_push_constants(
                    self.command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    bytemuck::cast_slice(&push_constants)
                );
            }
            
            device.cmd_draw_indexed(self.command_buffer, data.indices.len() as u32, 1, 0, 0, 0);
        }
    }
    
    fn create_texture(&mut self, _width: u32, _height: u32, _data: &[u8]) -> Result<TextureHandle, RhiError> {
        Ok(TextureHandle::null())
    }
    
    fn destroy_texture(&mut self, _texture: TextureHandle) {}
}

impl Drop for VulkanUIRenderer {
    fn drop(&mut self) {
        let device = self.device.lock();
        unsafe {
            device.device_wait_idle().expect("Failed to wait for device idle");
            
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_render_pass(self.render_pass, None);
            
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
            
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}