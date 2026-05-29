use ash::vk;
use std::ffi::CString;

/// 图形管线封装
pub struct VulkanPipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    vert_shader: vk::ShaderModule,
    frag_shader: vk::ShaderModule,
}

impl VulkanPipeline {
    pub fn new_simple_triangle(device: &ash::Device, render_pass: vk::RenderPass, extent: vk::Extent2D) -> Result<Self, hezhou_rhi::RhiError> {
        unsafe {
            let vert_shader_code = include_bytes!("../../shaders/triangle.vert.spv");
            let frag_shader_code = include_bytes!("../../shaders/triangle.frag.spv");
            
            let vert_shader = Self::create_shader_module(device, vert_shader_code)?;
            let frag_shader = Self::create_shader_module(device, frag_shader_code)?;
            
            let main_name = CString::new("main").unwrap();
            
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
            
            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default();
            
            let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
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
            
            let rasterizer = vk::PipelineRasterizationStateCreateInfo {
                depth_clamp_enable: vk::FALSE,
                rasterizer_discard_enable: vk::FALSE,
                polygon_mode: vk::PolygonMode::FILL,
                line_width: 1.0,
                cull_mode: vk::CullModeFlags::BACK,
                front_face: vk::FrontFace::CLOCKWISE,
                ..Default::default()
            };
            
            let multisampling = vk::PipelineMultisampleStateCreateInfo {
                sample_shading_enable: vk::FALSE,
                rasterization_samples: vk::SampleCountFlags::TYPE_1,
                ..Default::default()
            };
            
            let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
                color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A,
                blend_enable: vk::FALSE,
                src_color_blend_factor: vk::BlendFactor::ONE,
                dst_color_blend_factor: vk::BlendFactor::ZERO,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ONE,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
            };
            
            let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
                logic_op_enable: vk::FALSE,
                attachment_count: 1,
                p_attachments: &color_blend_attachment,
                ..Default::default()
            };
            
            let layout_info = vk::PipelineLayoutCreateInfo::default();
            
            let layout = device.create_pipeline_layout(&layout_info, None)
                .map_err(|e| hezhou_rhi::RhiError::PipelineCreationFailed(e.to_string()))?;
            
            let pipeline_info = vk::GraphicsPipelineCreateInfo {
                stage_count: 2,
                p_stages: stages.as_ptr(),
                p_vertex_input_state: &vertex_input_state,
                p_input_assembly_state: &input_assembly,
                p_viewport_state: &viewport_state,
                p_rasterization_state: &rasterizer,
                p_multisample_state: &multisampling,
                p_color_blend_state: &color_blend_state,
                layout,
                render_pass,
                subpass: 0,
                ..Default::default()
            };
            
            let pipelines = device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| hezhou_rhi::RhiError::PipelineCreationFailed(e.to_string()))?;
            
            let pipeline = pipelines[0];
            
            Ok(Self { pipeline, layout, vert_shader, frag_shader })
        }
    }
    
    unsafe fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule, hezhou_rhi::RhiError> {
        let spirv: Vec<u32> = code.chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        
        let shader_info = vk::ShaderModuleCreateInfo {
            code_size: spirv.len() * 4,
            p_code: spirv.as_ptr(),
            ..Default::default()
        };
        
        device.create_shader_module(&shader_info, None)
            .map_err(|e| hezhou_rhi::RhiError::ShaderCompilationFailed(e.to_string()))
    }
    
    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
    
    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }
    
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_shader_module(self.vert_shader, None);
            device.destroy_shader_module(self.frag_shader, None);
        }
    }
}

/// 计算管线封装：从 SPIR-V 创建 compute pipeline
pub struct VulkanComputePipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    compute_shader: vk::ShaderModule,
}

impl VulkanComputePipeline {
    /// 从 SPIR-V 字节码创建计算管线
    /// - spirv_code: compute shader 的 SPIR-V 字节码
    /// - descriptor_set_layouts: descriptor set layout 数组
    /// - push_constant_ranges: push constant range 数组
    pub fn create(
        device: &ash::Device,
        spirv_code: &[u8],
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> Result<Self, hezhou_rhi::RhiError> {
        unsafe {
            // 创建 shader module
            let compute_shader = create_shader_module_from_bytes(device, spirv_code)?;

            let main_name = CString::new("main").unwrap();

            // 计算管线只有一个 shader stage
            let compute_stage = vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::COMPUTE,
                module: compute_shader,
                p_name: main_name.as_ptr(),
                ..Default::default()
            };

            // 创建 pipeline layout（descriptor sets + push constants）
            let layout_info = vk::PipelineLayoutCreateInfo {
                set_layout_count: descriptor_set_layouts.len() as u32,
                p_set_layouts: descriptor_set_layouts.as_ptr(),
                push_constant_range_count: push_constant_ranges.len() as u32,
                p_push_constant_ranges: push_constant_ranges.as_ptr(),
                ..Default::default()
            };

            let layout = device
                .create_pipeline_layout(&layout_info, None)
                .map_err(|e| hezhou_rhi::RhiError::PipelineCreationFailed(e.to_string()))?;

            // 创建 compute pipeline — 不需要 viewport/scissor/dynamic state
            let pipeline_info = vk::ComputePipelineCreateInfo {
                stage: compute_stage,
                layout,
                ..Default::default()
            };

            let pipelines = device
                .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| hezhou_rhi::RhiError::PipelineCreationFailed(e.to_string()))?;

            let pipeline = pipelines[0];

            Ok(Self {
                pipeline,
                layout,
                compute_shader,
            })
        }
    }

    /// 从 SPIR-V u32 数组创建计算管线
    pub fn create_from_spirv_u32(
        device: &ash::Device,
        spirv: &[u32],
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> Result<Self, hezhou_rhi::RhiError> {
        unsafe {
            let compute_shader = create_shader_module_from_u32(device, spirv)?;

            let main_name = CString::new("main").unwrap();

            let compute_stage = vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::COMPUTE,
                module: compute_shader,
                p_name: main_name.as_ptr(),
                ..Default::default()
            };

            let layout_info = vk::PipelineLayoutCreateInfo {
                set_layout_count: descriptor_set_layouts.len() as u32,
                p_set_layouts: descriptor_set_layouts.as_ptr(),
                push_constant_range_count: push_constant_ranges.len() as u32,
                p_push_constant_ranges: push_constant_ranges.as_ptr(),
                ..Default::default()
            };

            let layout = device
                .create_pipeline_layout(&layout_info, None)
                .map_err(|e| hezhou_rhi::RhiError::PipelineCreationFailed(e.to_string()))?;

            let pipeline_info = vk::ComputePipelineCreateInfo {
                stage: compute_stage,
                layout,
                ..Default::default()
            };

            let pipelines = device
                .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| hezhou_rhi::RhiError::PipelineCreationFailed(e.to_string()))?;

            let pipeline = pipelines[0];

            Ok(Self {
                pipeline,
                layout,
                compute_shader,
            })
        }
    }

    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_shader_module(self.compute_shader, None);
        }
    }
}

/// 从字节码创建 shader module
unsafe fn create_shader_module_from_bytes(
    device: &ash::Device,
    code: &[u8],
) -> Result<vk::ShaderModule, hezhou_rhi::RhiError> {
    let spirv: Vec<u32> = code
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    create_shader_module_from_u32(device, &spirv)
}

/// 从 u32 SPIR-V 创建 shader module
unsafe fn create_shader_module_from_u32(
    device: &ash::Device,
    spirv: &[u32],
) -> Result<vk::ShaderModule, hezhou_rhi::RhiError> {
    let shader_info = vk::ShaderModuleCreateInfo {
        code_size: spirv.len() * 4,
        p_code: spirv.as_ptr(),
        ..Default::default()
    };
    device
        .create_shader_module(&shader_info, None)
        .map_err(|e| hezhou_rhi::RhiError::ShaderCompilationFailed(e.to_string()))
}

/// 在 Vulkan command buffer 上录制 compute dispatch 命令
/// - device: ash Device（command buffer 录制函数通过 Device trait 调用）
/// - cmd: 已 begin 的 vk::CommandBuffer
/// - compute_pipeline: 要绑定的 compute pipeline
/// - pipeline_layout: pipeline layout（用于 descriptor sets）
/// - group_count_x/y/z: 工作组数量
pub unsafe fn cmd_dispatch(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    compute_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    group_count_x: u32,
    group_count_y: u32,
    group_count_z: u32,
) {
    // 绑定 compute pipeline
    device.cmd_bind_pipeline(
        cmd,
        vk::PipelineBindPoint::COMPUTE,
        compute_pipeline,
    );
    // dispatch 工作组
    device.cmd_dispatch(cmd, group_count_x, group_count_y, group_count_z);
}

/// 在 Vulkan command buffer 上录制 compute dispatch + descriptor set 绑定
pub unsafe fn cmd_dispatch_with_descriptor_sets(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    compute_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_sets: &[vk::DescriptorSet],
    group_count_x: u32,
    group_count_y: u32,
    group_count_z: u32,
) {
    // 绑定 compute pipeline
    device.cmd_bind_pipeline(
        cmd,
        vk::PipelineBindPoint::COMPUTE,
        compute_pipeline,
    );
    // 绑定 descriptor sets
    if !descriptor_sets.is_empty() {
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            descriptor_sets,
            &[],
        );
    }
    // dispatch 工作组
    device.cmd_dispatch(cmd, group_count_x, group_count_y, group_count_z);
}

/// 计算→片段 pipeline barrier：compute shader 写入后，fragment shader 读取前
/// 用于 compute shader 输出 storage buffer/image → fragment shader 采样
pub unsafe fn cmd_compute_to_fragment_barrier(device: &ash::Device, cmd: vk::CommandBuffer) {
    let barrier = vk::MemoryBarrier {
        src_access_mask: vk::AccessFlags::SHADER_WRITE,
        dst_access_mask: vk::AccessFlags::SHADER_READ,
        ..Default::default()
    };
    device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[barrier],
        &[],
        &[],
    );
}

/// 主机→计算 pipeline barrier：CPU 写入 buffer 后，compute shader 读取前
pub unsafe fn cmd_host_to_compute_barrier(device: &ash::Device, cmd: vk::CommandBuffer) {
    let barrier = vk::MemoryBarrier {
        src_access_mask: vk::AccessFlags::HOST_WRITE,
        dst_access_mask: vk::AccessFlags::SHADER_READ,
        ..Default::default()
    };
    device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::HOST,
        vk::PipelineStageFlags::COMPUTE_SHADER,
        vk::DependencyFlags::empty(),
        &[barrier],
        &[],
        &[],
    );
}