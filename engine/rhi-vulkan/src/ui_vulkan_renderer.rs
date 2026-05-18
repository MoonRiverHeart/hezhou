use ash::vk::{self, Handle};
use ash::khr::surface::Instance as SurfaceLoader;
use ash::khr::swapchain::Device as SwapchainLoader;
use glfw::{Glfw, PWindow, GlfwReceiver, WindowEvent, WindowMode, Action, Key};
use std::ffi::CString;
use std::collections::HashMap;
use hezhou_ui::{UISystem, UIInputHandler, Panel, Button, Label, TextEdit, Layout, DrawCommand, Widget, Style, Color, TextStyle, ffi::WidgetTreeHandle, ffi::ui_set_primary_button_id};
use hezhou_platform::{MouseAction, MouseEvent, MouseButton, CharEvent, KeyEvent, KeyAction, KeyCode, KeyModifiers};
use hezhou_dfx::{DfxSystem, LogLevel};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct CachedGlyph {
    x: f32,
    y: f32,
    w: usize,
    h: usize,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

pub struct GlyphCache {
    glyphs: HashMap<String, Vec<CachedGlyph>>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            glyphs: HashMap::new(),
        }
    }
    
    pub fn get(&self, text: &str) -> Option<&Vec<CachedGlyph>> {
        self.glyphs.get(text)
    }
    
    pub fn insert(&mut self, text: String, glyphs: Vec<CachedGlyph>) {
        self.glyphs.insert(text, glyphs);
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UIVulkanRenderer {
    glfw: Glfw,
    window: PWindow,
    event_receiver: GlfwReceiver<(f64, WindowEvent)>,
    instance: ash::Instance,
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
    extent: vk::Extent2D,
    surface_loader: SurfaceLoader,
    swapchain_loader: SwapchainLoader,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    font_texture: vk::Image,
    font_texture_memory: vk::DeviceMemory,
    font_texture_view: vk::ImageView,
    font_sampler: vk::Sampler,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    ui_system: Arc<Mutex<UISystem>>,
    input_handler: Arc<Mutex<UIInputHandler>>,
    dfx: Arc<Mutex<DfxSystem>>,
    frame_count: u64,
    button_id: u64,
    space_pressed: bool,
    glyph_cache: GlyphCache,
    button_clicked: Arc<AtomicBool>,
    needs_resize: bool,
    new_extent: vk::Extent2D,
    swapchain_format: vk::Format,
    physical_device: vk::PhysicalDevice,
    triangle_angle: f32,
    last_frame_time: f64,
}

impl UIVulkanRenderer {
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, String> {
        let dfx = Arc::new(Mutex::new(DfxSystem::new()));
        let logger = dfx.lock().get_logger();
        
        logger.lock().log(LogLevel::Info, "Vulkan", &format!("Creating window {}x{}", width, height), file!(), line!());
        
        unsafe {
            let mut glfw = glfw::init(glfw::fail_on_errors)
                .map_err(|e| format!("GLFW init failed: {}", e))?;
            
            glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
            
            let (mut window, event_receiver) = glfw
                .create_window(width, height, title, WindowMode::Windowed)
                .expect("Failed to create GLFW window");
            
            window.set_all_polling(true);
            
            logger.lock().log(LogLevel::Info, "Vulkan", &format!("Window created {}x{}", width, height), file!(), line!());
            
            let entry = ash::Entry::load().map_err(|e| format!("Failed to load Vulkan: {}", e))?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Instance created", file!(), line!());
            
            let app_name = CString::new(title).unwrap();
            let app_info = vk::ApplicationInfo {
                p_application_name: app_name.as_ptr(),
                application_version: 1,
                p_engine_name: CString::new("Hezhou UI").unwrap().as_ptr(),
                engine_version: 1,
                api_version: vk::API_VERSION_1_2,
                ..Default::default()
            };
            
            let glfw_extensions = glfw.get_required_instance_extensions()
                .expect("Failed to get extensions");
            let extension_names: Vec<CString> = glfw_extensions
                .iter()
                .map(|s| CString::new(s.as_str()).expect("Invalid extension"))
                .collect();
            let extensions: Vec<*const i8> = extension_names.iter().map(|s| s.as_ptr()).collect();
            
            let instance = entry.create_instance(&vk::InstanceCreateInfo {
                p_application_info: &app_info,
                pp_enabled_extension_names: extensions.as_ptr(),
                enabled_extension_count: extensions.len() as u32,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create instance: {}", e))?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Instance created", file!(), line!());
            
            let surface_loader = SurfaceLoader::new(&entry, &instance);
            
            let mut surface: vk::SurfaceKHR = vk::SurfaceKHR::null();
            let result = window.create_window_surface(
                instance.handle().as_raw() as glfw::ffi::VkInstance,
                std::ptr::null(),
                &mut surface as *mut vk::SurfaceKHR as *mut glfw::ffi::VkSurfaceKHR,
            );
            if result != 0 {
                return Err(format!("Failed to create surface: {}", result));
            }
            
            let physical_devices = instance.enumerate_physical_devices()
                .map_err(|e| format!("Failed to enumerate devices: {}", e))?;
            let physical_device = physical_devices[0];
            
            let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
            let graphics_queue_family = queue_families.iter()
                .position(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .unwrap() as u32;
            
            let queue_priority = 1.0f32;
            let device = instance.create_device(physical_device, &vk::DeviceCreateInfo {
                p_queue_create_infos: &vk::DeviceQueueCreateInfo {
                    queue_family_index: graphics_queue_family,
                    queue_count: 1,
                    p_queue_priorities: &queue_priority,
                    ..Default::default()
                },
                queue_create_info_count: 1,
                pp_enabled_extension_names: &[vk::KHR_SWAPCHAIN_NAME.as_ptr()] as *const _,
                enabled_extension_count: 1,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create device: {}", e))?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Device created", file!(), line!());
            
            let queue = device.get_device_queue(graphics_queue_family, 0);
            let swapchain_loader = SwapchainLoader::new(&instance, &device);
            
            let surface_formats = surface_loader.get_physical_device_surface_formats(physical_device, surface)
                .map_err(|e| format!("Failed to get surface formats: {}", e))?;
            let format = surface_formats[0].format;
            let color_space = surface_formats[0].color_space;
            
            let surface_caps = surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
                .map_err(|e| format!("Failed to get surface caps: {}", e))?;
            
            let extent = if surface_caps.current_extent.width != u32::MAX {
                surface_caps.current_extent
            } else {
                vk::Extent2D { width, height }
            };
            
            let swapchain = swapchain_loader.create_swapchain(&vk::SwapchainCreateInfoKHR {
                surface,
                min_image_count: 2,
                image_format: format,
                image_color_space: color_space,
                image_extent: extent,
                image_array_layers: 1,
                image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                pre_transform: surface_caps.current_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode: vk::PresentModeKHR::FIFO,
                clipped: vk::TRUE,
                old_swapchain: vk::SwapchainKHR::null(),
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create swapchain: {}", e))?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", &format!("Swapchain created {}x{}", extent.width, extent.height), file!(), line!());
            
            let swapchain_images = swapchain_loader.get_swapchain_images(swapchain)
                .map_err(|e| format!("Failed to get swapchain images: {}", e))?;
            
            let swapchain_image_views: Vec<vk::ImageView> = swapchain_images.iter()
                .map(|image| {
                    device.create_image_view(&vk::ImageViewCreateInfo {
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
                    }, None)
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to create image views: {}", e))?;
            
            let render_pass = device.create_render_pass(&vk::RenderPassCreateInfo {
                attachment_count: 1,
                p_attachments: &vk::AttachmentDescription {
                    format,
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                    ..Default::default()
                },
                subpass_count: 1,
                p_subpasses: &vk::SubpassDescription {
                    pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
                    color_attachment_count: 1,
                    p_color_attachments: &vk::AttachmentReference {
                        attachment: 0,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    },
                    ..Default::default()
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create render pass: {}", e))?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Render pass created", file!(), line!());
            
            let framebuffers: Vec<vk::Framebuffer> = swapchain_image_views.iter()
                .map(|view| {
                    device.create_framebuffer(&vk::FramebufferCreateInfo {
                        render_pass,
                        attachment_count: 1,
                        p_attachments: view,
                        width: extent.width,
                        height: extent.height,
                        layers: 1,
                        ..Default::default()
                    }, None)
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to create framebuffers: {}", e))?;
            
            let vert_shader = Self::create_shader_module(&device, include_bytes!("../../shaders/ui/ui.vert.spv"))?;
            let frag_shader = Self::create_shader_module(&device, include_bytes!("../../shaders/ui/ui.frag.spv"))?;
            
            let main_name = CString::new("main").unwrap();
            
            let descriptor_set_layout = device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                binding_count: 1,
                p_bindings: &vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::FRAGMENT,
                    p_immutable_samplers: std::ptr::null(),
                    ..Default::default()
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create descriptor set layout: {}", e))?;
            
            let descriptor_pool = device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                max_sets: 1,
                pool_size_count: 1,
                p_pool_sizes: &vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create descriptor pool: {}", e))?;
            
            let descriptor_set = device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &descriptor_set_layout,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate descriptor set: {}", e))?[0];
            
            let (font_texture, font_texture_memory) = Self::create_font_texture(&instance, &device, physical_device)?;
            
            let font_texture_view = device.create_image_view(&vk::ImageViewCreateInfo {
                image: font_texture,
                view_type: vk::ImageViewType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create font texture view: {}", e))?;
            
            let font_sampler = device.create_sampler(&vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                mip_lod_bias: 0.0,
                max_anisotropy: 1.0,
                compare_op: vk::CompareOp::NEVER,
                min_lod: 0.0,
                max_lod: 0.0,
                border_color: vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
                unnormalized_coordinates: 0,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create font sampler: {}", e))?;
            
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &vk::DescriptorImageInfo {
                        sampler: font_sampler,
                        image_view: font_texture_view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    },
                    ..Default::default()
                }],
                &[]
            );
            
            let pipeline_layout = device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                set_layout_count: 1,
                p_set_layouts: &descriptor_set_layout,
                push_constant_range_count: 1,
                p_push_constant_ranges: &vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    offset: 0,
                    size: 24,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create pipeline layout: {}", e))?;
            
            let pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(), &[vk::GraphicsPipelineCreateInfo {
                stage_count: 2,
                p_stages: &[
                    vk::PipelineShaderStageCreateInfo {
                        stage: vk::ShaderStageFlags::VERTEX,
                        module: vert_shader,
                        p_name: main_name.as_ptr(),
                        ..Default::default()
                    },
                    vk::PipelineShaderStageCreateInfo {
                        stage: vk::ShaderStageFlags::FRAGMENT,
                        module: frag_shader,
                        p_name: main_name.as_ptr(),
                        ..Default::default()
                    },
                ] as *const _,
                p_vertex_input_state: &vk::PipelineVertexInputStateCreateInfo {
                    vertex_binding_description_count: 1,
                    p_vertex_binding_descriptions: &vk::VertexInputBindingDescription {
                        binding: 0,
                        stride: 32,
                        input_rate: vk::VertexInputRate::VERTEX,
                    },
                    vertex_attribute_description_count: 3,
                    p_vertex_attribute_descriptions: &[
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
                    ] as *const _,
                    ..Default::default()
                },
                p_input_assembly_state: &vk::PipelineInputAssemblyStateCreateInfo {
                    topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                    primitive_restart_enable: vk::FALSE,
                    ..Default::default()
                },
                p_viewport_state: &vk::PipelineViewportStateCreateInfo {
                    viewport_count: 1,
                    scissor_count: 1,
                    ..Default::default()
                },
                p_rasterization_state: &vk::PipelineRasterizationStateCreateInfo {
                    polygon_mode: vk::PolygonMode::FILL,
                    cull_mode: vk::CullModeFlags::NONE,
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    line_width: 1.0,
                    ..Default::default()
                },
                p_multisample_state: &vk::PipelineMultisampleStateCreateInfo {
                    rasterization_samples: vk::SampleCountFlags::TYPE_1,
                    ..Default::default()
                },
                p_color_blend_state: &vk::PipelineColorBlendStateCreateInfo {
                    logic_op_enable: vk::FALSE,
                    attachment_count: 1,
                    p_attachments: &vk::PipelineColorBlendAttachmentState {
                        color_write_mask: vk::ColorComponentFlags::RGBA,
                        blend_enable: vk::TRUE,
                        src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
                        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                        color_blend_op: vk::BlendOp::ADD,
                        src_alpha_blend_factor: vk::BlendFactor::ONE,
                        dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                        alpha_blend_op: vk::BlendOp::ADD,
                    },
                    ..Default::default()
                },
                p_dynamic_state: &vk::PipelineDynamicStateCreateInfo {
                    dynamic_state_count: 2,
                    p_dynamic_states: &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR] as *const _,
                    ..Default::default()
                },
                layout: pipeline_layout,
                render_pass,
                subpass: 0,
                ..Default::default()
            }], None)
                .map_err(|(_, e)| format!("Failed to create pipeline: {}", e))?[0];
            
            device.destroy_shader_module(vert_shader, None);
            device.destroy_shader_module(frag_shader, None);
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Pipeline created", file!(), line!());
            
            let command_pool = device.create_command_pool(&vk::CommandPoolCreateInfo {
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create command pool: {}", e))?;
            
            let command_buffers = device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: framebuffers.len() as u32,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate command buffers: {}", e))?;
            
            let (vertex_buffer, vertex_buffer_memory) = Self::create_vertex_buffer(
                &instance, &device, physical_device, 65536
            )?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Vertex buffer created (64KB)", file!(), line!());
            
            let image_available_semaphores = (0..2)
                .map(|_| device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to create semaphores: {}", e))?;
            
            let render_finished_semaphores = (0..2)
                .map(|_| device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to create semaphores: {}", e))?;
            
            let in_flight_fences = (0..2)
                .map(|_| device.create_fence(&vk::FenceCreateInfo {
                    flags: vk::FenceCreateFlags::SIGNALED,
                    ..Default::default()
                }, None))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to create fences: {}", e))?;
            
            let ui_system = Arc::new(Mutex::new(UISystem::new()));
            let dfx = ui_system.lock().get_dfx();
            let event_dispatcher = ui_system.lock().get_event_dispatcher();
            let widget_tree = ui_system.lock().get_widget_tree();
            event_dispatcher.lock().set_widget_tree(widget_tree);
            let input_handler = Arc::new(Mutex::new(UIInputHandler::new(event_dispatcher)));
            input_handler.lock().set_screen_size(extent.width as f32, extent.height as f32);
            
            dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "UI system initialized", file!(), line!());
            
            Ok(Self {
                glfw,
                window,
                event_receiver,
                instance,
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
                extent,
                surface_loader,
                swapchain_loader,
                vertex_buffer,
                vertex_buffer_memory,
                font_texture,
                font_texture_memory,
                font_texture_view,
                font_sampler,
                descriptor_set_layout,
                descriptor_pool,
                descriptor_set,
                ui_system,
                input_handler,
                dfx,
                frame_count: 0,
                button_id: 0,
                space_pressed: false,
                glyph_cache: GlyphCache::new(),
                button_clicked: Arc::new(AtomicBool::new(false)),
                needs_resize: false,
                new_extent: extent,
                swapchain_format: format,
                physical_device,
                triangle_angle: 0.0,
                last_frame_time: 0.0,
            })
        }
    }
    
    fn create_shader_module(device: &ash::Device, spirv: &[u8]) -> Result<vk::ShaderModule, String> {
        let code: Vec<u32> = spirv.chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        
        unsafe {
            device.create_shader_module(&vk::ShaderModuleCreateInfo {
                code_size: code.len() * 4,
                p_code: code.as_ptr(),
                ..Default::default()
            }, None)
        }.map_err(|e| format!("Failed to create shader module: {}", e))
    }
    
    fn create_vertex_buffer(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        size: vk::DeviceSize,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
        unsafe {
            let buffer = device.create_buffer(&vk::BufferCreateInfo {
                size,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create buffer: {}", e))?;
            
            let mem_requirements = device.get_buffer_memory_requirements(buffer);
            let mem_properties = instance.get_physical_device_memory_properties(physical_device);
            
            let mem_type_index = Self::find_memory_type(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                &mem_properties
            );
            
            let memory = device.allocate_memory(&vk::MemoryAllocateInfo {
                allocation_size: mem_requirements.size,
                memory_type_index: mem_type_index,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to allocate memory: {}", e))?;
            
            device.bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| format!("Failed to bind memory: {}", e))?;
            
            Ok((buffer, memory))
        }
    }
    
    fn create_font_texture(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
    ) -> Result<(vk::Image, vk::DeviceMemory), String> {
        unsafe {
            let image = device.create_image(&vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                extent: vk::Extent3D { width: 1024, height: 1024, depth: 1 },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::LINEAR,
                usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout: vk::ImageLayout::PREINITIALIZED,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create font image: {}", e))?;
            
            let mem_requirements = device.get_image_memory_requirements(image);
            let mem_properties = instance.get_physical_device_memory_properties(physical_device);
            
            let mem_type_index = Self::find_memory_type(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                &mem_properties
            );
            
            let memory = device.allocate_memory(&vk::MemoryAllocateInfo {
                allocation_size: mem_requirements.size,
                memory_type_index: mem_type_index,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to allocate font texture memory: {}", e))?;
            
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| format!("Failed to bind font texture memory: {}", e))?;
            
            let data_ptr = device.map_memory(memory, 0, mem_requirements.size, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map font texture memory: {}", e))?;
            
            let texture_data = vec![0u8; 1024 * 1024 * 3];
            std::ptr::copy_nonoverlapping(texture_data.as_ptr(), data_ptr as *mut u8, 1024 * 1024 * 3);
            
            device.unmap_memory(memory);
            
            Ok((image, memory))
        }
    }
    
    fn find_memory_type(type_filter: u32, properties: vk::MemoryPropertyFlags, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> u32 {
        for i in 0..mem_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0 
                && mem_properties.memory_types[i as usize].property_flags.contains(properties) {
                return i;
            }
        }
        0
    }
    
pub fn setup_ui(&mut self) {
        let ui = self.ui_system.lock();
        let tree = ui.get_widget_tree();
        
        let mut tree_guard = tree.lock();
        
        let mut root_panel = Panel::new();
        root_panel.set_layout(Layout::new(0.0, 0.0, self.extent.width as f32, self.extent.height as f32));
        root_panel.set_style(Style::new().with_background(Color::transparent()));
        tree_guard.set_root(Box::new(root_panel));
        
        if let Some(root_id) = tree_guard.root {
            let mut vstack = hezhou_ui::widgets::VStack::new()
                .with_spacing(16.0);
            vstack.set_layout(Layout::new(
                (self.extent.width as f32 - 200.0) / 2.0,
                (self.extent.height as f32 - 150.0) / 2.0,
                0.0,
                0.0,
            ));
            let vstack_id = vstack.id();
            tree_guard.add_widget(Box::new(vstack), root_id);
            
            let mut button = Button::new("Click Me");
            button.set_layout(Layout::new(0.0, 0.0, 150.0, 40.0));
            self.button_id = button.id().id;
            
            ui_set_primary_button_id(self.button_id);
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Info,
                "UI",
                &format!("Button created with id={}", self.button_id),
                file!(),
                line!()
            );
            
            let clicked_flag = Arc::clone(&self.button_clicked);
            
            button.set_on_click(Box::new(move || {
                clicked_flag.store(true, Ordering::SeqCst);
            }));
            
            tree_guard.add_widget(Box::new(button), vstack_id);
            
            let mut label = Label::new("Welcome to Hezhou UI!");
            label.set_text_style(TextStyle::new().with_size(16.0).with_color(Color::white()));
            tree_guard.add_widget(Box::new(label), vstack_id);
            
            let mut text_edit = TextEdit::new();
            text_edit.set_text("Type here...");
            text_edit.set_layout(Layout::new(0.0, 0.0, 200.0, 40.0));
            tree_guard.add_widget(Box::new(text_edit), vstack_id);
            
            let mut hint_label = Label::new("Press SPACE to change text");
            hint_label.set_text_style(TextStyle::new().with_size(16.0).with_color(Color::new(1.0, 1.0, 0.0, 1.0)));
            hint_label.set_layout(Layout::new(10.0, 10.0, 500.0, 30.0));
            tree_guard.add_widget(Box::new(hint_label), root_id);
            
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "VStack created with Button and Label", file!(), line!());
            
let font_atlas = ui.get_font_atlas();
            tree_guard.perform_layout(font_atlas);
            
            tree_guard.recenter_widget(vstack_id, self.extent.width as f32, self.extent.height as f32);
            tree_guard.perform_layout(font_atlas);
            
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "VStack created with Button and Label", file!(), line!());
        }
        
        drop(tree_guard);
        
        let font_atlas = ui.get_font_atlas();
        let texture_data = font_atlas.get_atlas_texture().to_vec();
        
        drop(ui);
        
        unsafe {
            let mem_requirements = self.device.get_image_memory_requirements(self.font_texture);
            let data_ptr = self.device.map_memory(
                self.font_texture_memory,
                0,
                mem_requirements.size,
                vk::MemoryMapFlags::empty()
            ).map_err(|e| format!("Failed to map font texture in setup: {}", e)).unwrap();
            
            std::ptr::copy_nonoverlapping(texture_data.as_ptr(), data_ptr as *mut u8, texture_data.len());
            self.device.unmap_memory(self.font_texture_memory);
            
            let transition_cmd = self.device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                command_pool: self.command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: 1,
                ..Default::default()
            }).unwrap()[0];
            
            self.device.begin_command_buffer(transition_cmd, &vk::CommandBufferBeginInfo::default()).unwrap();
            
            let barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::PREINITIALIZED,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.font_texture,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::HOST_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };
            
            self.device.cmd_pipeline_barrier(
                transition_cmd,
                vk::PipelineStageFlags::HOST,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier]
            );
            
            self.device.end_command_buffer(transition_cmd).unwrap();
            
            self.device.queue_submit(self.queue, &[vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &transition_cmd,
                ..Default::default()
            }], vk::Fence::null()).unwrap();
            
            self.device.queue_wait_idle(self.queue).unwrap();
            
            self.device.free_command_buffers(self.command_pool, &[transition_cmd]);
            
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "FontAtlas", &format!("Uploaded texture ({} bytes), transitioned to SHADER_READ_ONLY", texture_data.len()), file!(), line!());
        }
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "UI tree setup complete", file!(), line!());
    }
    
    pub fn get_button_id(&self) -> u64 {
        self.button_id
    }
    
    pub fn setup_ui_for_script(&mut self) {
        let ui = self.ui_system.lock();
        let tree = ui.get_widget_tree();
        
        let mut tree_guard = tree.lock();
        
        let mut root_panel = Panel::new();
        root_panel.set_layout(Layout::new(0.0, 0.0, self.extent.width as f32, self.extent.height as f32));
        root_panel.set_style(Style::new().with_background(Color::transparent()));
        tree_guard.set_root(Box::new(root_panel));
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "Root panel created (C# will create widgets)", file!(), line!());
        
        drop(tree_guard);
        
        let font_atlas = ui.get_font_atlas();
        let texture_data = font_atlas.get_atlas_texture().to_vec();
        
        drop(ui);
        
        unsafe {
            let mem_requirements = self.device.get_image_memory_requirements(self.font_texture);
            let data_ptr = self.device.map_memory(
                self.font_texture_memory,
                0,
                mem_requirements.size,
                vk::MemoryMapFlags::empty()
            ).map_err(|e| format!("Failed to map font texture in setup: {}", e)).unwrap();
            
            std::ptr::copy_nonoverlapping(texture_data.as_ptr(), data_ptr as *mut u8, texture_data.len());
            self.device.unmap_memory(self.font_texture_memory);
            
            let transition_cmd = self.device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                command_pool: self.command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: 1,
                ..Default::default()
            }).unwrap()[0];
            
            self.device.begin_command_buffer(transition_cmd, &vk::CommandBufferBeginInfo::default()).unwrap();
            
            let barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::PREINITIALIZED,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.font_texture,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::HOST_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };
            
            self.device.cmd_pipeline_barrier(
                transition_cmd,
                vk::PipelineStageFlags::HOST,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier]
            );
            
            self.device.end_command_buffer(transition_cmd).unwrap();
            
            self.device.queue_submit(self.queue, &[vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &transition_cmd,
                ..Default::default()
            }], vk::Fence::null()).unwrap();
            
            self.device.queue_wait_idle(self.queue).unwrap();
            
            self.device.free_command_buffers(self.command_pool, &[transition_cmd]);
        }
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "UI setup complete (script will create widgets)", file!(), line!());
    }
    
    pub fn get_widget_tree_handle(&self) -> WidgetTreeHandle {
        let ui = self.ui_system.lock();
        let tree = ui.get_widget_tree();
        Box::into_raw(Box::new(tree)) as WidgetTreeHandle
    }
    
    pub fn get_ui_system(&self) -> Arc<Mutex<UISystem>> {
        Arc::clone(&self.ui_system)
    }
    
    pub fn is_space_pressed(&self) -> bool {
        self.space_pressed || self.window.get_key(Key::Space) == Action::Press
    }
    
    pub fn consume_space_press(&mut self) {
        self.space_pressed = false;
    }
    
    unsafe fn recreate_swapchain(&mut self) -> Result<(), String> {
        self.device.device_wait_idle()
            .map_err(|e| format!("Failed to wait for device idle: {}", e))?;
        
        for framebuffer in &self.framebuffers {
            self.device.destroy_framebuffer(*framebuffer, None);
        }
        for view in &self.swapchain_image_views {
            self.device.destroy_image_view(*view, None);
        }
        
        let surface_caps = self.surface_loader.get_physical_device_surface_capabilities(self.physical_device, self.surface)
            .map_err(|e| format!("Failed to get surface caps: {}", e))?;
        
        let extent = if surface_caps.current_extent.width != u32::MAX {
            surface_caps.current_extent
        } else {
            self.new_extent
        };
        
        let old_swapchain = self.swapchain;
        
        let swapchain = self.swapchain_loader.create_swapchain(&vk::SwapchainCreateInfoKHR {
            surface: self.surface,
            min_image_count: 2,
            image_format: self.swapchain_format,
            image_color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform: surface_caps.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: vk::PresentModeKHR::FIFO,
            clipped: vk::TRUE,
            old_swapchain,
            ..Default::default()
        }, None).map_err(|e| format!("Failed to recreate swapchain: {}", e))?;
        
        self.swapchain_loader.destroy_swapchain(old_swapchain, None);
        
        let swapchain_images = self.swapchain_loader.get_swapchain_images(swapchain)
            .map_err(|e| format!("Failed to get swapchain images: {}", e))?;
        
        let swapchain_image_views: Vec<vk::ImageView> = swapchain_images.iter()
            .map(|image| {
                self.device.create_image_view(&vk::ImageViewCreateInfo {
                    image: *image,
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: self.swapchain_format,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    ..Default::default()
                }, None)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to create image views: {}", e))?;
        
        let framebuffers: Vec<vk::Framebuffer> = swapchain_image_views.iter()
            .map(|view| {
                self.device.create_framebuffer(&vk::FramebufferCreateInfo {
                    render_pass: self.render_pass,
                    attachment_count: 1,
                    p_attachments: view,
                    width: extent.width,
                    height: extent.height,
                    layers: 1,
                    ..Default::default()
                }, None)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to create framebuffers: {}", e))?;
        
        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.swapchain_image_views = swapchain_image_views;
        self.framebuffers = framebuffers;
        self.extent = extent;
        
        self.input_handler.lock().set_screen_size(extent.width as f32, extent.height as f32);
        
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Info,
            "Vulkan",
            &format!("Swapchain recreated {}x{}", extent.width, extent.height),
            file!(),
            line!()
        );
        
        Ok(())
    }
    
    unsafe fn update_ui_layout(&mut self) {
        let ui = self.ui_system.lock();
        let tree = ui.get_widget_tree();
        let mut tree_guard = tree.lock();
        let font_atlas = ui.get_font_atlas();
        
        if let Some(root_id) = tree_guard.root {
            if let Some(root_widget) = tree_guard.get_widget_mut(root_id) {
                root_widget.set_layout(hezhou_ui::Layout::new(
                    0.0, 0.0, 
                    self.extent.width as f32, 
                    self.extent.height as f32
                ));
            }
            
            let vstack_id = tree_guard.get_children(root_id)
                .first()
                .copied()
                .unwrap_or_else(hezhou_ui::WidgetId::invalid);
            
            if vstack_id.is_valid() {
                tree_guard.recenter_widget(vstack_id, self.extent.width as f32, self.extent.height as f32);
                tree_guard.perform_layout(font_atlas);
            }
        }
        
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Info,
            "UI",
            &format!("UI layout updated for {}x{}", self.extent.width, self.extent.height),
            file!(),
            line!()
        );
    }
    
    pub fn draw_frame(&mut self) -> Result<bool, String> {
        if self.window.should_close() {
            return Ok(false);
        }
        
        let current_time = self.glfw.get_time();
        let delta_time = if self.last_frame_time > 0.0 {
            (current_time - self.last_frame_time) as f32 * 1000.0
        } else {
            16.0
        };
        self.last_frame_time = current_time;
        
        hezhou_ui::ffi::ui_trigger_update(delta_time);
        
        if self.needs_resize {
            unsafe {
                self.recreate_swapchain()?;
                self.update_ui_layout();
            }
            self.needs_resize = false;
            
            hezhou_ui::ffi::ui_trigger_resize(self.extent.width as f32, self.extent.height as f32);
        }
        
        if self.button_clicked.load(Ordering::SeqCst) {
            self.button_clicked.store(false, Ordering::SeqCst);
            
            use hezhou_ui::WidgetId;
            let widget_id = WidgetId::from_raw(self.button_id);
            
            let ui = self.ui_system.lock();
            let tree = ui.get_widget_tree();
            let mut tree_guard = tree.lock();
            
            if let Some(widget) = tree_guard.get_widget_mut(widget_id) {
                unsafe {
                    if let Some(button) = (widget.as_mut() as *mut dyn Widget as *mut Button).as_mut() {
                        button.set_text("hello");
                        self.dfx.lock().get_logger().lock().log(
                            LogLevel::Info,
                            "UI",
                            "Button text changed to 'hello' via click!",
                            file!(),
                            line!()
                        );
                    }
                }
            }
        }
        
        unsafe {
            let (image_index, _suboptimal) = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null()
            ).map_err(|e| format!("Failed to acquire image: {}", e))?;
            
            let image_index_usize = image_index as usize;
            
            self.device.wait_for_fences(&[self.in_flight_fences[self.current_frame]], true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fence: {}", e))?;
            
            self.device.reset_fences(&[self.in_flight_fences[self.current_frame]])
                .map_err(|e| format!("Failed to reset fence: {}", e))?;
            
            self.device.reset_command_buffer(self.command_buffers[image_index_usize], vk::CommandBufferResetFlags::RELEASE_RESOURCES)
                .map_err(|e| format!("Failed to reset command buffer: {}", e))?;
            
            self.device.begin_command_buffer(self.command_buffers[image_index_usize], &vk::CommandBufferBeginInfo::default())
                .map_err(|e| format!("Failed to begin command buffer: {}", e))?;
            
            self.device.cmd_begin_render_pass(
                self.command_buffers[image_index_usize],
                &vk::RenderPassBeginInfo {
                    render_pass: self.render_pass,
                    framebuffer: self.framebuffers[image_index_usize],
                    render_area: vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.extent,
                    },
                    clear_value_count: 1,
                    p_clear_values: &vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.1, 0.1, 0.15, 1.0],
                        },
                    },
                    _marker: std::marker::PhantomData,
                    p_next: std::ptr::null(),
                    s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                },
                vk::SubpassContents::INLINE
            );
            
            self.device.cmd_bind_pipeline(
                self.command_buffers[image_index_usize],
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline
            );
            
            self.device.cmd_bind_descriptor_sets(
                self.command_buffers[image_index_usize],
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.descriptor_set],
                &[]
            );
            
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.extent.width as f32,
                height: self.extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.extent,
            };
            
            self.device.cmd_set_viewport(self.command_buffers[image_index_usize], 0, &[viewport]);
            self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[scissor]);
            
let font_atlas = self.ui_system.lock().get_font_atlas();
            let px_range = 4.0;
            
            let push_constants = [
                self.extent.width as f32,
                self.extent.height as f32,
                0.0,
                0.0,
                px_range,
                0.0,
            ];
            self.device.cmd_push_constants(
                self.command_buffers[image_index_usize],
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&push_constants)
            );
            
            let render_data = {
                let ui = self.ui_system.lock();
                let tree = ui.get_widget_tree();
                let mut tree_guard = tree.lock();
                let font_atlas = ui.get_font_atlas();
                tree_guard.perform_layout(font_atlas);
                tree_guard.generate_render_data(font_atlas)
            };
            
            let mut vertices: Vec<f32> = Vec::new();
            
            // 添加旋转三角形（在预览区域中心，作为背景）
            let preview_x = 250.0f32;
            let preview_y = 40.0f32;
            let preview_w = self.extent.width as f32 - 500.0;
            let preview_h = self.extent.height as f32 - 70.0;
            
            let center_x = preview_x + preview_w / 2.0;
            let center_y = preview_y + preview_h / 2.0;
            let radius = 80.0;
            
            self.triangle_angle += 90.0 * 0.016; // 90度/秒
            if self.triangle_angle > 360.0 {
                self.triangle_angle -= 360.0;
            }
            
            let angle_rad = self.triangle_angle.to_radians();
            
            let p0 = (center_x + radius * angle_rad.cos(), center_y - radius * angle_rad.sin());
            let p1 = (center_x + radius * (angle_rad + 2.094).cos(), center_y - radius * (angle_rad + 2.094).sin());
            let p2 = (center_x + radius * (angle_rad + 4.189).cos(), center_y - radius * (angle_rad + 4.189).sin());
            
            let (r, g, b, a) = (1.0, 0.5, 0.8, 1.0);
            
            vertices.extend_from_slice(&[
                p0.0, p0.1, r, g, b, a, 0.0, 0.0,
                p1.0, p1.1, r, g, b, a, 0.0, 0.0,
                p2.0, p2.1, r, g, b, a, 0.0, 0.0,
            ]);
            
            // 渲染UI（在三角形之上）
            for cmd in render_data.iter().flat_map(|data| &data.draw_commands) {
                match cmd {
DrawCommand::Rect { bounds, width, height, fill_color, .. } => {
                        let x = bounds.x;
                        let y = bounds.y;
                        let w = *width;
                        let h = *height;
                        let r = fill_color.r;
                        let g = fill_color.g;
                        let b = fill_color.b;
                        let a = fill_color.a;
                        
                        vertices.extend_from_slice(&[
                            x, y, r, g, b, a, 0.0, 0.0,
                            x + w, y, r, g, b, a, 0.0, 0.0,
                            x, y + h, r, g, b, a, 0.0, 0.0,
                            x + w, y, r, g, b, a, 0.0, 0.0,
                            x + w, y + h, r, g, b, a, 0.0, 0.0,
                            x, y + h, r, g, b, a, 0.0, 0.0,
                        ]);
                    }
DrawCommand::Text { bounds, width, height, font_color, text, text_len, font_size, alignment, .. } => {
                            let text_str = if text.is_null() || *text_len == 0 {
                                ""
                            } else {
                                unsafe {
                                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(*text, *text_len))
                                }
                            };
                            
                            let ui_lock = self.ui_system.lock();
                            let font_atlas = ui_lock.get_font_atlas();
                            
                            let glyphs = if alignment.horizontal == hezhou_ui::HorizontalAlignment::Left {
                                let vertical_center = alignment.vertical == hezhou_ui::VerticalAlignment::Center;
                                font_atlas.layout_text_left(
                                    0,
                                    text_str,
                                    *font_size,
                                    bounds.x,
                                    bounds.y,
                                    *height,
                                    vertical_center,
                                )
                            } else {
                                font_atlas.layout_text_centered(
                                    0,
                                    text_str,
                                    *font_size,
                                    bounds.x,
                                    bounds.y,
                                    *width,
                                    *height,
                                )
                            };
                            
                            for (gx, gy, gw, gh, uv_x, uv_y, uv_w, uv_h) in glyphs {
                                let w = gw as f32;
                                let h = gh as f32;
                                
                                if w == 0.0 || h == 0.0 {
                                    continue;
                                }
                                
                                let x = gx;
                                let y = gy;
                                let u0 = uv_x;
                                let v0 = uv_y;
                                let u1 = uv_x + uv_w;
                                let v1 = uv_y + uv_h;
                                let r = font_color.r;
                                let g = font_color.g;
                                let b = font_color.b;
                                let a = font_color.a;
                                
                                vertices.extend_from_slice(&[
                                    x, y, r, g, b, a, u0, v0,
                                    x + w, y, r, g, b, a, u1, v0,
                                    x, y + h, r, g, b, a, u0, v1,
                                    x + w, y, r, g, b, a, u1, v0,
                                    x + w, y + h, r, g, b, a, u1, v1,
                                    x, y + h, r, g, b, a, u0, v1,
                                ]);
                            }
                        }
                        DrawCommand::Line { .. } => {}
                        DrawCommand::Image { .. } => {}
                        DrawCommand::Shadow { .. } => {}
                    DrawCommand::ClipRect { .. } => {}
                    DrawCommand::ClearClip => {}
                    DrawCommand::SetTransform { .. } => {}
                    DrawCommand::ResetTransform => {}
                }
            }
            
            let vertex_data: &[u8] = bytemuck::cast_slice(&vertices);
            
            let vertex_ptr = self.device.map_memory(
                self.vertex_buffer_memory,
                0,
                vertex_data.len() as vk::DeviceSize,
                vk::MemoryMapFlags::empty()
            ).map_err(|e| format!("Failed to map memory: {}", e))?;
            
            std::ptr::copy_nonoverlapping(vertex_data.as_ptr(), vertex_ptr as *mut u8, vertex_data.len());
            self.device.unmap_memory(self.vertex_buffer_memory);
            
            self.device.cmd_bind_vertex_buffers(
                self.command_buffers[image_index_usize],
                0,
                &[self.vertex_buffer],
                &[0]
            );
            
            self.device.cmd_draw(
                self.command_buffers[image_index_usize],
                (vertices.len() / 8) as u32,
                1,
                0,
                0
            );
            
            if self.frame_count == 0 {
                self.dfx.lock().get_logger().lock().log(LogLevel::Trace, "Render", &format!("Frame {}: {} vertices", self.frame_count, vertices.len() / 8), file!(), line!());
            }
            
            self.device.cmd_end_render_pass(self.command_buffers[image_index_usize]);
            self.device.end_command_buffer(self.command_buffers[image_index_usize])
                .map_err(|e| format!("Failed to end command buffer: {}", e))?;
            
            let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
            let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            
            let submit_info = vk::SubmitInfo {
                wait_semaphore_count: wait_semaphores.len() as u32,
                p_wait_semaphores: wait_semaphores.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: &self.command_buffers[image_index_usize],
                signal_semaphore_count: signal_semaphores.len() as u32,
                p_signal_semaphores: signal_semaphores.as_ptr(),
                _marker: std::marker::PhantomData,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::SUBMIT_INFO,
            };
            
            self.device.queue_submit(
                self.queue,
                &[submit_info],
                self.in_flight_fences[self.current_frame]
            ).map_err(|e| format!("Failed to submit queue: {}", e))?;
            
            let present_info = vk::PresentInfoKHR {
                wait_semaphore_count: signal_semaphores.len() as u32,
                p_wait_semaphores: signal_semaphores.as_ptr(),
                swapchain_count: 1,
                p_swapchains: &self.swapchain,
                p_image_indices: &image_index,
                p_results: std::ptr::null_mut(),
                _marker: std::marker::PhantomData,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::PRESENT_INFO_KHR,
            };
            
            self.swapchain_loader.queue_present(self.queue, &present_info)
                .map_err(|e| format!("Failed to present: {}", e))?;
            
            self.current_frame = (self.current_frame + 1) % self.image_available_semaphores.len();
            self.frame_count += 1;
            
            Ok(true)
        }
    }
    
    pub fn process_events(&mut self) {
        self.glfw.poll_events();
        
        let events: Vec<_> = glfw::flush_messages(&self.event_receiver).collect();
        if !events.is_empty() {
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Info, 
                "GLFW", 
                &format!("process_events: {} events received", events.len()), 
                file!(), 
                line!()
            );
        }
        
        for (_, event) in events {
            match event {
                WindowEvent::MouseButton(button, action, _) => {
                    let x = self.window.get_cursor_pos().0 as f32;
                    let y = self.window.get_cursor_pos().1 as f32;
                    
self.dfx.lock().get_logger().lock().log(
                        LogLevel::Info, 
                        "GLFW", 
                        &format!("MouseButton: {} {} at ({}, {})", 
                            match button {
                                glfw::MouseButtonLeft => "Left",
                                glfw::MouseButtonRight => "Right",
                                glfw::MouseButtonMiddle => "Middle",
                                _ => "Other",
                            },
                            match action {
                                glfw::Action::Press => "Press",
                                glfw::Action::Release => "Release",
                                glfw::Action::Repeat => "Repeat",
                            },
                            x, y
                        ), 
                        file!(), 
                        line!()
                    );
                    
                    let ui_button = match button {
                        glfw::MouseButtonLeft => MouseButton::Left,
                        glfw::MouseButtonRight => MouseButton::Right,
                        glfw::MouseButtonMiddle => MouseButton::Middle,
                        _ => MouseButton::Left,
                    };
                    
                    let ui_action = match action {
                        glfw::Action::Press => MouseAction::Press,
                        glfw::Action::Release => MouseAction::Release,
                        glfw::Action::Repeat => MouseAction::Press,
                    };
                    
                    let mouse_event = MouseEvent {
                        action: ui_action,
                        button: ui_button,
                        x,
                        y,
                        dx: 0.0,
                        dy: 0.0,
                    };
                    
                    self.input_handler.lock().on_mouse_event(&mouse_event, self.frame_count);
                    self.dfx.lock().get_logger().lock().log(
                        LogLevel::Debug, 
                        "GLFW", 
                        "MouseEvent dispatched to input_handler", 
                        file!(), 
                        line!()
                    );
                }
                WindowEvent::CursorPos(x, y) => {
                    let mouse_event = MouseEvent {
                        action: MouseAction::Move,
                        button: MouseButton::Left,
                        x: x as f32,
                        y: y as f32,
                        dx: 0.0,
                        dy: 0.0,
                    };
                    
                    self.input_handler.lock().on_mouse_event(&mouse_event, self.frame_count);
                }
                WindowEvent::Key(key, _, action, mods) => {
                    if action == Action::Press {
                        if key == Key::Space {
                            self.space_pressed = true;
                        }
                        if key == Key::Backspace {
                            self.input_handler.lock().on_key_event(&KeyEvent {
                                action: KeyAction::Press,
                                keycode: KeyCode::Backspace,
                                modifiers: KeyModifiers::default(),
                            }, self.frame_count);
                        }
                        
                        // 方向键
                        if key == Key::Left {
                            self.input_handler.lock().on_key_event(&KeyEvent {
                                action: KeyAction::Press,
                                keycode: KeyCode::Left,
                                modifiers: KeyModifiers::default(),
                            }, self.frame_count);
                        }
                        if key == Key::Right {
                            self.input_handler.lock().on_key_event(&KeyEvent {
                                action: KeyAction::Press,
                                keycode: KeyCode::Right,
                                modifiers: KeyModifiers::default(),
                            }, self.frame_count);
                        }
                        if key == Key::Up {
                            self.input_handler.lock().on_key_event(&KeyEvent {
                                action: KeyAction::Press,
                                keycode: KeyCode::Up,
                                modifiers: KeyModifiers::default(),
                            }, self.frame_count);
                        }
                        if key == Key::Down {
                            self.input_handler.lock().on_key_event(&KeyEvent {
                                action: KeyAction::Press,
                                keycode: KeyCode::Down,
                                modifiers: KeyModifiers::default(),
                            }, self.frame_count);
                        }
                        
                        // Home/End键
                        if key == Key::Home {
                            self.input_handler.lock().on_key_event(&KeyEvent {
                                action: KeyAction::Press,
                                keycode: KeyCode::Home,
                                modifiers: KeyModifiers::default(),
                            }, self.frame_count);
                        }
                        if key == Key::End {
                            self.input_handler.lock().on_key_event(&KeyEvent {
                                action: KeyAction::Press,
                                keycode: KeyCode::End,
                                modifiers: KeyModifiers::default(),
                            }, self.frame_count);
                        }
                        
                        // Ctrl+C/V/X 复制粘贴剪切
                        if mods == glfw::Modifiers::Control {
                            if key == Key::C {
                                self.input_handler.lock().on_key_event(&KeyEvent {
                                    action: KeyAction::Press,
                                    keycode: KeyCode::C,
                                    modifiers: KeyModifiers { ctrl: true, shift: false, alt: false },
                                }, self.frame_count);
                            }
                            if key == Key::V {
                                self.input_handler.lock().on_key_event(&KeyEvent {
                                    action: KeyAction::Press,
                                    keycode: KeyCode::V,
                                    modifiers: KeyModifiers { ctrl: true, shift: false, alt: false },
                                }, self.frame_count);
                            }
                            if key == Key::X {
                                self.input_handler.lock().on_key_event(&KeyEvent {
                                    action: KeyAction::Press,
                                    keycode: KeyCode::X,
                                    modifiers: KeyModifiers { ctrl: true, shift: false, alt: false },
                                }, self.frame_count);
                            }
                        }
                        
                        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "GLFW", &format!("Key pressed: {:?} mods={:?}", key, mods), file!(), line!());
                    }
                }
                WindowEvent::Char(codepoint) => {
                    if codepoint >= '\0' && codepoint <= '\x7F' {
                        self.input_handler.lock().on_char_event(&CharEvent {
                            codepoint: codepoint as u32,
                        }, self.frame_count);
                        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "GLFW", &format!("Char input: {} ({})", codepoint, codepoint as u32), file!(), line!());
                    }
                }
                WindowEvent::Close => {
                    self.window.set_should_close(true);
                }
                WindowEvent::Size(width, height) => {
                    if width > 0 && height > 0 {
                        self.needs_resize = true;
                        self.new_extent = vk::Extent2D { 
                            width: width as u32, 
                            height: height as u32 
                        };
                        self.dfx.lock().get_logger().lock().log(
                            LogLevel::Info, 
                            "GLFW", 
                            &format!("Window resized to {}x{}", width, height), 
                            file!(), 
                            line!()
                        );
                    }
                }
                _ => {}
            }
        }
    }
    
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }
    
    pub fn cleanup(&mut self) {
        unsafe {
            self.device.device_wait_idle().expect("Failed to wait for device idle");
            
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            
            for semaphore in &self.image_available_semaphores {
                self.device.destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.render_finished_semaphores {
                self.device.destroy_semaphore(*semaphore, None);
            }
            for fence in &self.in_flight_fences {
                self.device.destroy_fence(*fence, None);
            }
            
            for framebuffer in &self.framebuffers {
                self.device.destroy_framebuffer(*framebuffer, None);
            }
            for view in &self.swapchain_image_views {
                self.device.destroy_image_view(*view, None);
            }
            
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.device.destroy_command_pool(self.command_pool, None);
            
            self.surface_loader.destroy_surface(self.surface, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Cleanup complete", file!(), line!());
    }
}
