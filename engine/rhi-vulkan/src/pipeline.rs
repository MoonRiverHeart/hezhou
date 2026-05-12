use ash::vk;
use std::ffi::CString;

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