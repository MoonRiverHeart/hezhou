use ash::{vk, Device};
use rhi::*;
use super::context::VulkanContext;

struct Pipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

pub struct VulkanRenderer {
    device: Device,
    pipeline: Option<Pipeline>,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    staging_buffer: vk::Buffer,
    staging_buffer_memory: vk::DeviceMemory,
    staging_mapped: *mut u8,
    // 纹理相关
    texture_image: Option<vk::Image>,
    texture_memory: Option<vk::DeviceMemory>,
    texture_view: Option<vk::ImageView>,
    texture_sampler: Option<vk::Sampler>,
    descriptor_pool: Option<vk::DescriptorPool>,
    descriptor_set: Option<vk::DescriptorSet>,
    descriptor_layout: Option<vk::DescriptorSetLayout>,
    texture_id_counter: u32,
}

impl VulkanRenderer {
    pub fn new(context: &VulkanContext) -> Self {
        let max_vertices = 65536;
        let max_indices = 65536;
        let vertex_buffer_size = (max_vertices * std::mem::size_of::<Vertex>()) as u64;
        let index_buffer_size = (max_indices * std::mem::size_of::<u32>()) as u64;
        let staging_size = vertex_buffer_size + index_buffer_size;
        
        let device = context.device().clone();
        let physical_device = context.physical_device();
        let instance = context.instance();
        
        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        let find_host_visible_type = |type_filter: u32| -> u32 {
            for i in 0..mem_properties.memory_type_count {
                if (type_filter & (1 << i)) != 0 &&
                   mem_properties.memory_types[i as usize].property_flags
                       .contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                {
                    return i;
                }
            }
            panic!("No HOST_VISIBLE memory type found");
        };
        
        let find_device_local_type = |type_filter: u32| -> u32 {
            for i in 0..mem_properties.memory_type_count {
                if (type_filter & (1 << i)) != 0 &&
                   mem_properties.memory_types[i as usize].property_flags
                       .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                {
                    return i;
                }
            }
            find_host_visible_type(type_filter)
        };
        
        let (vertex_buffer, vertex_buffer_memory) = Self::create_buffer(
            &device, vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            &mem_properties, &find_device_local_type,
        );
        
        let (index_buffer, index_buffer_memory) = Self::create_buffer(
            &device, index_buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            &mem_properties, &find_device_local_type,
        );
        
        let (staging_buffer, staging_buffer_memory) = Self::create_buffer(
            &device, staging_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            &mem_properties, &find_host_visible_type,
        );
        
        let staging_mapped = unsafe {
            device.map_memory(staging_buffer_memory, 0, staging_size, vk::MemoryMapFlags::empty()).unwrap() as *mut u8
        };
        
        VulkanRenderer {
            device,
            pipeline: None,
            vertex_buffer, vertex_buffer_memory,
            index_buffer, index_buffer_memory,
            staging_buffer, staging_buffer_memory,
            staging_mapped,
            texture_image: None,
            texture_memory: None,
            texture_view: None,
            texture_sampler: None,
            descriptor_pool: None,
            descriptor_set: None,
            descriptor_layout: None,
            texture_id_counter: 0,
        }
    }
    
    pub fn draw(&mut self, context: &mut VulkanContext, commands: &[DrawCommand]) {
        let device = context.device();
        let cmd = context.current_command_buffer();
        
        let begin_info = vk::CommandBufferBeginInfo::default();
        unsafe { device.begin_command_buffer(cmd, &begin_info).unwrap(); }
        
        if self.pipeline.is_none() {
            self.pipeline = Some(Self::create_pipeline(
                device,
                context.render_pass(),
                context.framebuffer_width(),
                context.framebuffer_height(),
                self.descriptor_layout,
            ));
        }
        
        let image_index = context.current_image_index();
        let width = context.framebuffer_width();
        let height = context.framebuffer_height();
        let pipeline = self.pipeline.as_ref().unwrap();
        
        let mut all_vertices: Vec<Vertex> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();
        let mut cmd_rects: Vec<(f32, f32, f32, f32, f32)> = Vec::new();
        
        for cmd_data in commands {
            if cmd_data.vertices.is_empty() { continue; }
            
            let index_offset = all_vertices.len() as u32;
            all_vertices.extend_from_slice(&cmd_data.vertices);
            
            let min_x = cmd_data.vertices.iter().map(|v| v.position[0]).fold(f32::MAX, f32::min);
            let min_y = cmd_data.vertices.iter().map(|v| v.position[1]).fold(f32::MAX, f32::min);
            let max_x = cmd_data.vertices.iter().map(|v| v.position[0]).fold(f32::MIN, f32::max);
            let max_y = cmd_data.vertices.iter().map(|v| v.position[1]).fold(f32::MIN, f32::max);
            let radius = cmd_data.vertices[0].border_radius[0];
            
            if let Some(ref indices) = cmd_data.indices {
                all_indices.extend(indices.iter().map(|i| i + index_offset));
            }
            cmd_rects.push((min_x, min_y, max_x - min_x, max_y - min_y, radius));
        }
        
        if !all_vertices.is_empty() {
            let vertex_size = (all_vertices.len() * std::mem::size_of::<Vertex>()) as u64;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    all_vertices.as_ptr() as *const u8,
                    self.staging_mapped,
                    vertex_size as usize,
                );
            }
            let copy_region = vk::BufferCopy::default().size(vertex_size);
            unsafe { device.cmd_copy_buffer(cmd, self.staging_buffer, self.vertex_buffer, &[copy_region]); }
            
            if !all_indices.is_empty() {
                let index_size = (all_indices.len() * std::mem::size_of::<u32>()) as u64;
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        all_indices.as_ptr() as *const u8,
                        self.staging_mapped.add(vertex_size as usize),
                        index_size as usize,
                    );
                }
                let copy_region = vk::BufferCopy::default().src_offset(vertex_size).size(index_size);
                unsafe { device.cmd_copy_buffer(cmd, self.staging_buffer, self.index_buffer, &[copy_region]); }
            }
            
            let memory_barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ);
            unsafe {
                device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::VERTEX_INPUT,
                    vk::DependencyFlags::empty(),
                    &[memory_barrier],
                    &[],
                    &[],
                );
            }
            
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue { float32: [0.1, 0.1, 0.15, 1.0] },
            }];
            
            let render_pass_begin = vk::RenderPassBeginInfo::default()
                .render_pass(context.render_pass())
                .framebuffer(context.framebuffers()[image_index])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D { width, height },
                })
                .clear_values(&clear_values);
            
            unsafe {
                device.cmd_begin_render_pass(cmd, &render_pass_begin, vk::SubpassContents::INLINE);
                device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
                
                // 绑定纹理 descriptor set
                if let Some(descriptor_set) = self.descriptor_set {
                    device.cmd_bind_descriptor_sets(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.layout,
                        0,
                        &[descriptor_set],
                        &[],
                    );
                }
                
                let viewport = vk::Viewport::default()
                    .x(0.0).y(0.0)
                    .width(width as f32).height(height as f32)
                    .min_depth(0.0).max_depth(1.0);
                device.cmd_set_viewport(cmd, 0, &[viewport]);
                
                let scissor = vk::Rect2D::default()
                    .offset(vk::Offset2D { x: 0, y: 0 })
                    .extent(vk::Extent2D { width, height });
                device.cmd_set_scissor(cmd, 0, &[scissor]);
                
                device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer], &[0]);
                
                let mut index_offset = 0u32;
                for (cmd_idx, (rx, ry, rw, rh, _radius)) in cmd_rects.iter().enumerate() {
                    let cmd_data = &commands[cmd_idx];
                    let push_data: [f32; 8] = [
                        width as f32, height as f32, 0.0, cmd_data.texture_id as f32,
                        *rx, *ry, *rw, *rh,
                    ];
                    device.cmd_push_constants(
                        cmd, pipeline.layout,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                        0,
                        bytemuck::cast::<[f32; 8], [u8; 32]>(push_data).as_slice(),
                    );
                    let vertex_count = cmd_data.vertices.len() as u32;
                    
                    if let Some(ref idx) = cmd_data.indices {
                        let index_count = idx.len() as u32;
                        device.cmd_bind_index_buffer(cmd, self.index_buffer, (index_offset * 4) as u64, vk::IndexType::UINT32);
                        device.cmd_draw_indexed(cmd, index_count, 1, 0, 0, 0);
                        index_offset += index_count;
                    } else {
                        device.cmd_draw(cmd, vertex_count, 1, index_offset, 0);
                        index_offset += vertex_count;
                    }
                }
                
                device.cmd_end_render_pass(cmd);
            }
        }
        
        unsafe { device.end_command_buffer(cmd).unwrap(); }
    }

    pub fn upload_texture(&mut self, context: &VulkanContext, data: &[u8], width: u32, height: u32) -> u32 {
        let device = context.device();
        let physical_device = context.physical_device();
        let instance = context.instance();
        
        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        let image_size = (width * height * 4) as u64;
        
        let find_host_visible = |type_filter: u32| -> u32 {
            for i in 0..mem_properties.memory_type_count {
                if (type_filter & (1 << i)) != 0 &&
                   mem_properties.memory_types[i as usize].property_flags
                       .contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                {
                    return i;
                }
            }
            0
        };
        
        // 创建 staging buffer 并上传数据
        let (staging_buf, staging_mem) = Self::create_buffer(
            device, image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            &mem_properties, &find_host_visible,
        );
        
        unsafe {
            let ptr = device.map_memory(staging_mem, 0, image_size, vk::MemoryMapFlags::empty()).unwrap();
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
            device.unmap_memory(staging_mem);
        }
        
        // 创建纹理 image
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        
        let texture_image = unsafe { device.create_image(&image_info, None).unwrap() };
        
        let requirements = unsafe { device.get_image_memory_requirements(texture_image) };
        let find_device_local = |type_filter: u32| -> u32 {
            for i in 0..mem_properties.memory_type_count {
                if (type_filter & (1 << i)) != 0 &&
                   mem_properties.memory_types[i as usize].property_flags
                       .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                {
                    return i;
                }
            }
            0
        };
        let memory_type_index = find_device_local(requirements.memory_type_bits);
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);
        
        let texture_memory = unsafe { device.allocate_memory(&alloc_info, None).unwrap() };
        unsafe { device.bind_image_memory(texture_image, texture_memory, 0).unwrap(); }
        
        // 创建 image view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(texture_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        let texture_view = unsafe { device.create_image_view(&view_info, None).unwrap() };
        
        // 创建 sampler
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let sampler = unsafe { device.create_sampler(&sampler_info, None).unwrap() };
        
        // 创建 descriptor set layout
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None).unwrap() };
        
        // 创建 descriptor pool
        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
        }];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None).unwrap() };
        
        // 分配 descriptor set
        let layouts = [descriptor_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };
        
        // 更新 descriptor set
        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(texture_view)
            .sampler(sampler);
        
        let image_infos = [image_info];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_infos);
        unsafe { device.update_descriptor_sets(&[write], &[]); }
        
        // 转换 image layout: UNDEFINED -> TRANSFER_DST
        let cmd = context.current_command_buffer();
        let begin_info = vk::CommandBufferBeginInfo::default();
        unsafe {
            device.begin_command_buffer(cmd, &begin_info).unwrap();
            
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(texture_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1,
                    base_array_layer: 0, layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
            
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            
            // 从 staging buffer 复制到 image
            let copy_region = vk::BufferImageCopy::default()
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_extent(vk::Extent3D { width, height, depth: 1 });
            
            device.cmd_copy_buffer_to_image(
                cmd, staging_buf, texture_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_region],
            );
            
            // 转换 image layout: TRANSFER_DST -> SHADER_READ_ONLY
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(texture_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1,
                    base_array_layer: 0, layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
            
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            
            device.end_command_buffer(cmd).unwrap();
            
            // 提交并等待完成
            let cmd_buffers = [cmd];
            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&cmd_buffers);
            device.queue_submit(context.graphics_queue(), &[submit_info], vk::Fence::null()).unwrap();
            device.queue_wait_idle(context.graphics_queue()).unwrap();
        }
        
        // 清理 staging buffer
        unsafe {
            device.destroy_buffer(staging_buf, None);
            device.free_memory(staging_mem, None);
        }
        
        // 保存纹理资源
        self.texture_image = Some(texture_image);
        self.texture_memory = Some(texture_memory);
        self.texture_view = Some(texture_view);
        self.texture_sampler = Some(sampler);
        self.descriptor_pool = Some(descriptor_pool);
        self.descriptor_set = Some(descriptor_set);
        self.descriptor_layout = Some(descriptor_layout);
        
        let id = self.texture_id_counter;
        self.texture_id_counter += 1;
        id
    }
    
    fn create_pipeline(device: &Device, render_pass: vk::RenderPass, width: u32, height: u32, descriptor_layout: Option<vk::DescriptorSetLayout>) -> Pipeline {
        let vert_bytes = include_bytes!("../../../../assets/shader/vert.spv");
        let frag_bytes = include_bytes!("../../../../assets/shader/frag.spv");
        
        let vert_module = Self::create_shader_module(device, vert_bytes);
        let frag_module = Self::create_shader_module(device, frag_bytes);
        
        let entry_name = std::ffi::CString::new("main").unwrap();
        
        let vert_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&entry_name);
        
        let frag_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&entry_name);
        
        let stages = [vert_stage, frag_stage];
        
        let vertex_binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);
        
        let vertex_bindings = [vertex_binding];
        let vertex_attributes = [
            vk::VertexInputAttributeDescription::default()
                .binding(0).location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0).location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(8),
            vk::VertexInputAttributeDescription::default()
                .binding(0).location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(24),
            vk::VertexInputAttributeDescription::default()
                .binding(0).location(3)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(32),
        ];
        
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&vertex_bindings)
            .vertex_attribute_descriptions(&vertex_attributes);
        
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        
        let viewport = vk::Viewport::default()
            .x(0.0).y(0.0)
            .width(width as f32).height(height as f32)
            .min_depth(0.0).max_depth(1.0);
        
        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(vk::Extent2D { width, height });
        
        let viewports = [viewport];
        let scissors = [scissor];
        
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);
        
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE);
        
        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);
        
        let color_blend_attachments = [color_blend_attachment];
        
        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);
        
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);
        
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(32);
        
        let push_constant_ranges = [push_constant_range];
        
        let set_layouts: Vec<vk::DescriptorSetLayout> = if let Some(layout) = descriptor_layout {
            vec![layout]
        } else {
            vec![]
        };
        
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(&set_layouts);
        
        let layout = unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() };
        
        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blend)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0);
        
        let pipeline = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ).unwrap()[0]
        };
        
        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }
        
        Pipeline { pipeline, layout }
    }
    
    fn create_shader_module(device: &Device, bytes: &[u8]) -> vk::ShaderModule {
        let words: &[u32] = unsafe {
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const u32,
                bytes.len() / 4,
            )
        };
        
        let create_info = vk::ShaderModuleCreateInfo::default()
            .code(words);
        
        unsafe { device.create_shader_module(&create_info, None).unwrap() }
    }
    
    fn create_buffer(
        device: &Device,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        find_type: &dyn Fn(u32) -> u32,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size).usage(usage).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { device.create_buffer(&buffer_info, None).unwrap() };
        
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type_index = find_type(mem_requirements.memory_type_bits);
        
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);
        
        let memory = unsafe { device.allocate_memory(&alloc_info, None).unwrap() };
        unsafe { device.bind_buffer_memory(buffer, memory, 0).unwrap(); }
        
        (buffer, memory)
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            if let Some(pipeline) = &self.pipeline {
                self.device.destroy_pipeline(pipeline.pipeline, None);
                self.device.destroy_pipeline_layout(pipeline.layout, None);
            }
            self.device.unmap_memory(self.staging_buffer_memory);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);
            self.device.destroy_buffer(self.staging_buffer, None);
            self.device.free_memory(self.staging_buffer_memory, None);
            
            if let Some(descriptor_pool) = self.descriptor_pool {
                self.device.destroy_descriptor_pool(descriptor_pool, None);
            }
            if let Some(descriptor_layout) = self.descriptor_layout {
                self.device.destroy_descriptor_set_layout(descriptor_layout, None);
            }
            if let Some(sampler) = self.texture_sampler {
                self.device.destroy_sampler(sampler, None);
            }
            if let Some(view) = self.texture_view {
                self.device.destroy_image_view(view, None);
            }
            if let Some(image) = self.texture_image {
                self.device.destroy_image(image, None);
            }
            if let Some(memory) = self.texture_memory {
                self.device.free_memory(memory, None);
            }
        }
    }
}