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
    s_pressed: bool,
    glyph_cache: GlyphCache,
    button_clicked: Arc<AtomicBool>,
    needs_resize: bool,
    new_extent: vk::Extent2D,
    swapchain_format: vk::Format,
    physical_device: vk::PhysicalDevice,
    triangle_angle: f32,
    last_frame_time: f64,
    content_scale: f32,
    
    // Game preview rendering (offscreen + FXAA)
    game_render_pass: vk::RenderPass,
    game_pipeline: vk::Pipeline,
    game_pipeline_layout: vk::PipelineLayout,
    offscreen_image: vk::Image,
    offscreen_image_memory: vk::DeviceMemory,
    offscreen_image_view: vk::ImageView,
    offscreen_framebuffer: vk::Framebuffer,
    offscreen_extent: vk::Extent2D,
    
    // FXAA output (offscreen image after FXAA processing)
    offscreen_fxaa_image: vk::Image,
    offscreen_fxaa_image_memory: vk::DeviceMemory,
    offscreen_fxaa_image_view: vk::ImageView,
    offscreen_fxaa_framebuffer: vk::Framebuffer,
    
    // Preview texture descriptor (for UI to sample FXAA output)
    preview_descriptor_set: vk::DescriptorSet,
    
    // FXAA post-processing
    fxaa_pipeline: vk::Pipeline,
    fxaa_pipeline_layout: vk::PipelineLayout,
    fxaa_descriptor_set_layout: vk::DescriptorSetLayout,
    fxaa_descriptor_pool: vk::DescriptorPool,
    fxaa_descriptor_set: vk::DescriptorSet,
    fxaa_sampler: vk::Sampler,
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
            
            let (scale_x, scale_y) = window.get_content_scale();
            let content_scale = scale_x;
            logger.lock().log(LogLevel::Info, "Vulkan", &format!("Content scale: {} (DPI: {})", content_scale, content_scale * 96.0), file!(), line!());
            
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
                max_sets: 2,
                pool_size_count: 1,
                p_pool_sizes: &vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 2,
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
            
            // === Game Preview Rendering Setup ===
            logger.lock().log(LogLevel::Info, "Vulkan", "Creating game preview resources...", file!(), line!());
            
            // Offscreen image for game preview (use fixed size 512x512)
            let offscreen_extent = vk::Extent2D { width: 512, height: 512 };
            let offscreen_format = vk::Format::R8G8B8A8_UNORM;
            
            let (offscreen_image, offscreen_image_memory) = Self::create_offscreen_image(
                &instance, &device, physical_device, offscreen_extent, offscreen_format
            )?;
            
            let offscreen_image_view = device.create_image_view(&vk::ImageViewCreateInfo {
                image: offscreen_image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: offscreen_format,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create offscreen image view: {}", e))?;
            
            // Game render pass (render to offscreen)
            let game_render_pass = device.create_render_pass(&vk::RenderPassCreateInfo {
                attachment_count: 1,
                p_attachments: &vk::AttachmentDescription {
                    format: offscreen_format,
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                    stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                    stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                    initial_layout: vk::ImageLayout::UNDEFINED,
                    final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
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
            }, None).map_err(|e| format!("Failed to create game render pass: {}", e))?;
            
            let offscreen_framebuffer = device.create_framebuffer(&vk::FramebufferCreateInfo {
                render_pass: game_render_pass,
                attachment_count: 1,
                p_attachments: &offscreen_image_view,
                width: offscreen_extent.width,
                height: offscreen_extent.height,
                layers: 1,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create offscreen framebuffer: {}", e))?;
            
            // FXAA output image (same size/format as game offscreen)
            let (offscreen_fxaa_image, offscreen_fxaa_image_memory) = Self::create_offscreen_image(
                &instance, &device, physical_device, offscreen_extent, offscreen_format
            )?;
            
            let offscreen_fxaa_image_view = device.create_image_view(&vk::ImageViewCreateInfo {
                image: offscreen_fxaa_image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: offscreen_format,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create FXAA offscreen image view: {}", e))?;
            
            // FXAA framebuffer (uses same game_render_pass)
            let offscreen_fxaa_framebuffer = device.create_framebuffer(&vk::FramebufferCreateInfo {
                render_pass: game_render_pass,
                attachment_count: 1,
                p_attachments: &offscreen_fxaa_image_view,
                width: offscreen_extent.width,
                height: offscreen_extent.height,
                layers: 1,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create FXAA framebuffer: {}", e))?;
            
            // Game pipeline (simple triangle shader, reuse rotation.vert/frag)
            let game_vert_code = include_bytes!("../../shaders/rotation.vert.spv");
            let game_frag_code = include_bytes!("../../shaders/rotation.frag.spv");
            
            let game_vert_shader = Self::create_shader_module(&device, game_vert_code)?;
            let game_frag_shader = Self::create_shader_module(&device, game_frag_code)?;
            
            let game_pipeline_layout = device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                push_constant_range_count: 1,
                p_push_constant_ranges: &vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    offset: 0,
                    size: 4,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create game pipeline layout: {}", e))?;
            
            let game_pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(), &[
                vk::GraphicsPipelineCreateInfo {
                    stage_count: 2,
                    p_stages: &[
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::VERTEX,
                            module: game_vert_shader,
                            p_name: b"main\0".as_ptr() as *const i8,
                            ..Default::default()
                        },
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::FRAGMENT,
                            module: game_frag_shader,
                            p_name: b"main\0".as_ptr() as *const i8,
                            ..Default::default()
                        },
                    ] as *const _,
                    p_vertex_input_state: &vk::PipelineVertexInputStateCreateInfo {
                        ..Default::default()
                    },
                    p_input_assembly_state: &vk::PipelineInputAssemblyStateCreateInfo {
                        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                        primitive_restart_enable: vk::FALSE,
                        ..Default::default()
                    },
                    p_viewport_state: &vk::PipelineViewportStateCreateInfo {
                        viewport_count: 1,
                        p_viewports: &vk::Viewport {
                            x: 0.0,
                            y: 0.0,
                            width: offscreen_extent.width as f32,
                            height: offscreen_extent.height as f32,
                            min_depth: 0.0,
                            max_depth: 1.0,
                        },
                        scissor_count: 1,
                        p_scissors: &vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: offscreen_extent,
                        },
                        ..Default::default()
                    },
                    p_rasterization_state: &vk::PipelineRasterizationStateCreateInfo {
                        polygon_mode: vk::PolygonMode::FILL,
                        cull_mode: vk::CullModeFlags::NONE,
                        front_face: vk::FrontFace::CLOCKWISE,
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
                            blend_enable: vk::FALSE,
                            src_color_blend_factor: vk::BlendFactor::ONE,
                            dst_color_blend_factor: vk::BlendFactor::ZERO,
                            color_blend_op: vk::BlendOp::ADD,
                            src_alpha_blend_factor: vk::BlendFactor::ONE,
                            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                            alpha_blend_op: vk::BlendOp::ADD,
                            color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    layout: game_pipeline_layout,
                    render_pass: game_render_pass,
                    subpass: 0,
                    ..Default::default()
                }
            ], None).map_err(|(_, e)| format!("Failed to create game pipeline: {}", e))?[0];
            
            device.destroy_shader_module(game_vert_shader, None);
            device.destroy_shader_module(game_frag_shader, None);
            
            // Preview texture sampler (for UI to sample offscreen image)
            let preview_sampler = device.create_sampler(&vk::SamplerCreateInfo {
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
            }, None).map_err(|e| format!("Failed to create preview sampler: {}", e))?;
            
            // Preview descriptor set (for UI to display offscreen texture)
            let preview_descriptor_set = device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &descriptor_set_layout,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate preview descriptor set: {}", e))?[0];
            
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: preview_descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &vk::DescriptorImageInfo {
                        sampler: preview_sampler,
                        image_view: offscreen_image_view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    },
                    ..Default::default()
                }],
                &[]
            );
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Game preview resources created", file!(), line!());
            
            // FXAA pipeline setup
            let fxaa_vert_code = include_bytes!("../../shaders/fxaa.vert.spv");
            let fxaa_frag_code = include_bytes!("../../shaders/fxaa.frag.spv");
            
            let fxaa_vert_shader = Self::create_shader_module(&device, fxaa_vert_code)?;
            let fxaa_frag_shader = Self::create_shader_module(&device, fxaa_frag_code)?;
            
            let fxaa_descriptor_set_layout = device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
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
            }, None).map_err(|e| format!("Failed to create FXAA descriptor set layout: {}", e))?;
            
            let fxaa_pipeline_layout = device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                set_layout_count: 1,
                p_set_layouts: &fxaa_descriptor_set_layout,
                push_constant_range_count: 1,
                p_push_constant_ranges: &vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::FRAGMENT,
                    offset: 0,
                    size: 8, // vec2 resolution (2 * 4 bytes)
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create FXAA pipeline layout: {}", e))?;
            
            let fxaa_pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(), &[
                vk::GraphicsPipelineCreateInfo {
                    stage_count: 2,
                    p_stages: &[
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::VERTEX,
                            module: fxaa_vert_shader,
                            p_name: b"main\0".as_ptr() as *const i8,
                            ..Default::default()
                        },
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::FRAGMENT,
                            module: fxaa_frag_shader,
                            p_name: b"main\0".as_ptr() as *const i8,
                            ..Default::default()
                        },
                    ] as *const _,
                    p_vertex_input_state: &vk::PipelineVertexInputStateCreateInfo {
                        ..Default::default()
                    },
                    p_input_assembly_state: &vk::PipelineInputAssemblyStateCreateInfo {
                        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                        primitive_restart_enable: vk::FALSE,
                        ..Default::default()
                    },
                    p_viewport_state: &vk::PipelineViewportStateCreateInfo {
                        viewport_count: 1,
                        p_viewports: &vk::Viewport {
                            x: 0.0,
                            y: 0.0,
                            width: offscreen_extent.width as f32,
                            height: offscreen_extent.height as f32,
                            min_depth: 0.0,
                            max_depth: 1.0,
                        },
                        scissor_count: 1,
                        p_scissors: &vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: offscreen_extent,
                        },
                        ..Default::default()
                    },
                    p_rasterization_state: &vk::PipelineRasterizationStateCreateInfo {
                        polygon_mode: vk::PolygonMode::FILL,
                        cull_mode: vk::CullModeFlags::NONE,
                        front_face: vk::FrontFace::CLOCKWISE,
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
                            blend_enable: vk::FALSE,
                            src_color_blend_factor: vk::BlendFactor::ONE,
                            dst_color_blend_factor: vk::BlendFactor::ZERO,
                            color_blend_op: vk::BlendOp::ADD,
                            src_alpha_blend_factor: vk::BlendFactor::ONE,
                            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                            alpha_blend_op: vk::BlendOp::ADD,
                            color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    layout: fxaa_pipeline_layout,
                    render_pass: game_render_pass,
                    subpass: 0,
                    ..Default::default()
                }
            ], None).map_err(|(_, e)| format!("Failed to create FXAA pipeline: {}", e))?[0];
            
            device.destroy_shader_module(fxaa_vert_shader, None);
            device.destroy_shader_module(fxaa_frag_shader, None);
            
            // FXAA descriptor set
            let fxaa_descriptor_pool = device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                max_sets: 1,
                pool_size_count: 1,
                p_pool_sizes: &vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create FXAA descriptor pool: {}", e))?;
            
            let fxaa_descriptor_set = device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                descriptor_pool: fxaa_descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &fxaa_descriptor_set_layout,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate FXAA descriptor set: {}", e))?[0];
            
            // Update FXAA descriptor set to sample from offscreen image (game output)
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: fxaa_descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &vk::DescriptorImageInfo {
                        sampler: preview_sampler,
                        image_view: offscreen_image_view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    },
                    ..Default::default()
                }],
                &[]
            );
            
            let fxaa_sampler = preview_sampler; // reuse
            
            logger.lock().log(LogLevel::Info, "Vulkan", "FXAA pipeline created", file!(), line!());
            
            // Update preview descriptor set to point to FXAA output (offscreen_fxaa)
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: preview_descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &vk::DescriptorImageInfo {
                        sampler: preview_sampler,
                        image_view: offscreen_image_view, // 直接显示game pass输出（调试）
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    },
                    ..Default::default()
                }],
                &[]
            );
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Preview descriptor set updated for FXAA output", file!(), line!());
            
            let command_buffers = device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: framebuffers.len() as u32,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate command buffers: {}", e))?;
            
            let (vertex_buffer, vertex_buffer_memory) = Self::create_vertex_buffer(
                &instance, &device, physical_device, 33554432
            )?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Vertex buffer created (32MB)", file!(), line!());
            
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
            s_pressed: false,
                glyph_cache: GlyphCache::new(),
                button_clicked: Arc::new(AtomicBool::new(false)),
                needs_resize: false,
                new_extent: extent,
                swapchain_format: format,
                physical_device,
                triangle_angle: 0.0,
                last_frame_time: 0.0,
                content_scale,
                
                game_render_pass,
                game_pipeline,
                game_pipeline_layout,
                offscreen_image,
                offscreen_image_memory,
                offscreen_image_view,
                offscreen_framebuffer,
                offscreen_extent,
                
                offscreen_fxaa_image,
                offscreen_fxaa_image_memory,
                offscreen_fxaa_image_view,
                offscreen_fxaa_framebuffer,
                
                preview_descriptor_set,
                
                fxaa_pipeline,
                fxaa_pipeline_layout,
                fxaa_descriptor_set_layout,
                fxaa_descriptor_pool,
                fxaa_descriptor_set,
                fxaa_sampler,
            })
        }
    }
    
    fn create_offscreen_image(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        extent: vk::Extent2D,
        format: vk::Format,
    ) -> Result<(vk::Image, vk::DeviceMemory), String> {
        unsafe {
            let image = device.create_image(&vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format,
                extent: vk::Extent3D {
                    width: extent.width,
                    height: extent.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create offscreen image: {}", e))?;
            
            let mem_requirements = device.get_image_memory_requirements(image);
            let mem_properties = instance.get_physical_device_memory_properties(physical_device);
            
            let memory_type_index = Self::find_memory_type(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                &mem_properties
            );
            
            let memory = device.allocate_memory(&vk::MemoryAllocateInfo {
                allocation_size: mem_requirements.size,
                memory_type_index,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to allocate offscreen memory: {}", e))?;
            
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| format!("Failed to bind offscreen memory: {}", e))?;
            
            Ok((image, memory))
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
                extent: vk::Extent3D { width: 2048, height: 2048, depth: 1 },
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
            
            let texture_data = vec![0u8; 2048 * 2048 * 4];
            std::ptr::copy_nonoverlapping(texture_data.as_ptr(), data_ptr as *mut u8, 2048 * 2048 * 4);
            
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
    
    pub fn get_content_scale(&self) -> f32 {
        self.content_scale
    }
    
    pub fn is_space_pressed(&self) -> bool {
        self.space_pressed || self.window.get_key(Key::Space) == Action::Press
    }
    
    pub fn consume_space_press(&mut self) {
        self.space_pressed = false;
    }
    
    pub fn is_s_pressed(&self) -> bool {
        self.s_pressed || self.window.get_key(Key::S) == Action::Press
    }
    
    pub fn consume_s_press(&mut self) {
        self.s_pressed = false;
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
            
            // === Game Pass: Render triangle to offscreen ===
            self.triangle_angle += 90.0 * delta_time / 1000.0; // 90度/秒, delta_time is milliseconds
            if self.triangle_angle > 360.0 {
                self.triangle_angle -= 360.0;
            }
            
            // Transition offscreen image to COLOR_ATTACHMENT_OPTIMAL
            let game_barrier_begin = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.offscreen_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                ..Default::default()
            };
            self.device.cmd_pipeline_barrier(
                self.command_buffers[image_index_usize],
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[game_barrier_begin]
            );
            
            // Begin game render pass
            self.device.cmd_begin_render_pass(
                self.command_buffers[image_index_usize],
                &vk::RenderPassBeginInfo {
                    render_pass: self.game_render_pass,
                    framebuffer: self.offscreen_framebuffer,
                    render_area: vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.offscreen_extent,
                    },
                    clear_value_count: 1,
                    p_clear_values: &vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.05, 0.05, 0.1, 1.0],
                        },
                    },
                    _marker: std::marker::PhantomData,
                    p_next: std::ptr::null(),
                    s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                },
                vk::SubpassContents::INLINE
            );
            
            // Bind game pipeline
            self.device.cmd_bind_pipeline(
                self.command_buffers[image_index_usize],
                vk::PipelineBindPoint::GRAPHICS,
                self.game_pipeline
            );
            
            // Set viewport and scissor for game pass
            let game_viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.offscreen_extent.width as f32,
                height: self.offscreen_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let game_scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.offscreen_extent,
            };
            self.device.cmd_set_viewport(self.command_buffers[image_index_usize], 0, &[game_viewport]);
            self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[game_scissor]);
            
            // Draw triangle (rotation shader uses built-in vertex positions, no vertex buffer needed)
            let push_constant_data = [self.triangle_angle.to_radians()];
            self.device.cmd_push_constants(
                self.command_buffers[image_index_usize],
                self.game_pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                bytemuck::cast_slice(&push_constant_data)
            );
            self.device.cmd_draw(self.command_buffers[image_index_usize], 36, 1, 0, 0); // 36 vertices for cube
            
            // End game render pass
            self.device.cmd_end_render_pass(self.command_buffers[image_index_usize]);
            
            // Transition offscreen image to SHADER_READ_ONLY_OPTIMAL
            let game_barrier_end = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.offscreen_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };
            self.device.cmd_pipeline_barrier(
                self.command_buffers[image_index_usize],
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[game_barrier_end]
            );
            
            // === FXAA Pass: Apply FXAA to offscreen image ===
            // Transition offscreen_fxaa to COLOR_ATTACHMENT_OPTIMAL
            let fxaa_barrier_begin = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.offscreen_fxaa_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                ..Default::default()
            };
            self.device.cmd_pipeline_barrier(
                self.command_buffers[image_index_usize],
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[fxaa_barrier_begin]
            );
            
            // Begin FXAA render pass
            self.device.cmd_begin_render_pass(
                self.command_buffers[image_index_usize],
                &vk::RenderPassBeginInfo {
                    render_pass: self.game_render_pass,
                    framebuffer: self.offscreen_fxaa_framebuffer,
                    render_area: vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.offscreen_extent,
                    },
                    clear_value_count: 1,
                    p_clear_values: &vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    },
                    _marker: std::marker::PhantomData,
                    p_next: std::ptr::null(),
                    s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                },
                vk::SubpassContents::INLINE
            );
            
            // Bind FXAA pipeline and descriptor set
            self.device.cmd_bind_pipeline(
                self.command_buffers[image_index_usize],
                vk::PipelineBindPoint::GRAPHICS,
                self.fxaa_pipeline
            );
            self.device.cmd_bind_descriptor_sets(
                self.command_buffers[image_index_usize],
                vk::PipelineBindPoint::GRAPHICS,
                self.fxaa_pipeline_layout,
                0,
                &[self.fxaa_descriptor_set],
                &[]
            );
            
            // Set viewport and scissor for FXAA pass
            let fxaa_viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.offscreen_extent.width as f32,
                height: self.offscreen_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let fxaa_scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.offscreen_extent,
            };
            self.device.cmd_set_viewport(self.command_buffers[image_index_usize], 0, &[fxaa_viewport]);
            self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[fxaa_scissor]);
            
            // Push resolution constant (vec2: width, height)
            let resolution_data = [self.offscreen_extent.width as f32, self.offscreen_extent.height as f32];
            self.device.cmd_push_constants(
                self.command_buffers[image_index_usize],
                self.fxaa_pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&resolution_data)
            );
            
            // Draw fullscreen quad (FXAA shader uses gl_VertexIndex for vertices)
            self.device.cmd_draw(self.command_buffers[image_index_usize], 6, 1, 0, 0);
            
            // End FXAA render pass
            self.device.cmd_end_render_pass(self.command_buffers[image_index_usize]);
            
            // Transition offscreen_fxaa to SHADER_READ_ONLY_OPTIMAL
            let fxaa_barrier_end = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.offscreen_fxaa_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };
            self.device.cmd_pipeline_barrier(
                self.command_buffers[image_index_usize],
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[fxaa_barrier_end]
            );
            
            // === UI Pass: Render UI to swapchain ===
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
            let mut preview_vertices: Vec<f32> = Vec::new();
            
            // 渲染UI控件
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
DrawCommand::Text { bounds, width, height, font_color, text, font_size, alignment, .. } => {
                            let text_str = if text.is_empty() {
                                ""
                            } else {
                                std::str::from_utf8(text).unwrap_or("")
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
                        DrawCommand::Image { bounds, width, height, texture_id, uv } => {
                            // Separate preview texture quads from regular UI
                            let x = bounds.x;
                            let y = bounds.y;
                            let w = *width;
                            let h = *height;
                            let u0 = uv.x;
                            let v0 = uv.y;
                            let u1 = uv.x + uv.width;
                            let v1 = uv.y + uv.height;
                            
                            let r = 1.0;
                            let g = 1.0;
                            let b = 1.0;
                            let a = 1.0;
                            
                            let quad_vertices = [
                                x, y, r, g, b, a, u0, v0,
                                x + w, y, r, g, b, a, u1, v0,
                                x, y + h, r, g, b, a, u0, v1,
                                x + w, y, r, g, b, a, u1, v0,
                                x + w, y + h, r, g, b, a, u1, v1,
                                x, y + h, r, g, b, a, u0, v1,
                            ];
                            
                            // texture_id == 1 means preview texture
                            if *texture_id == 1 {
                                preview_vertices.extend_from_slice(&quad_vertices);
                            } else {
                                vertices.extend_from_slice(&quad_vertices);
                            }
                        }
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
            
            // Render preview texture quads (if any)
            if !preview_vertices.is_empty() {
                // Upload preview vertices after UI vertices
                let preview_offset = vertex_data.len() as u64;
                let preview_data: &[u8] = bytemuck::cast_slice(&preview_vertices);
                let preview_ptr = self.device.map_memory(
                    self.vertex_buffer_memory,
                    preview_offset,
                    preview_data.len() as vk::DeviceSize,
                    vk::MemoryMapFlags::empty()
                ).map_err(|e| format!("Failed to map preview memory: {}", e))?;
                std::ptr::copy_nonoverlapping(preview_data.as_ptr(), preview_ptr as *mut u8, preview_data.len());
                self.device.unmap_memory(self.vertex_buffer_memory);
                
                // Bind preview descriptor set
                self.device.cmd_bind_descriptor_sets(
                    self.command_buffers[image_index_usize],
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &[self.preview_descriptor_set],
                    &[]
                );
                
                // Set push constants for RGB texture mode (enable_msdf = false)
                let preview_push_constants = [
                    self.extent.width as f32,
                    self.extent.height as f32,
                    0.0,
                    0.0,
                    0.0,
                    0.0, // px_range=0, enable_msdf=false
                ];
                self.device.cmd_push_constants(
                    self.command_buffers[image_index_usize],
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    bytemuck::cast_slice(&preview_push_constants)
                );
                
                // Draw preview quads
                self.device.cmd_bind_vertex_buffers(
                    self.command_buffers[image_index_usize],
                    0,
                    &[self.vertex_buffer],
                    &[preview_offset]
                );
                self.device.cmd_draw(
                    self.command_buffers[image_index_usize],
                    (preview_vertices.len() / 8) as u32,
                    1,
                    0,
                    0
                );
                
                // Restore font descriptor set
                self.device.cmd_bind_descriptor_sets(
                    self.command_buffers[image_index_usize],
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &[self.descriptor_set],
                    &[]
                );
            }
            
            if self.frame_count == 0 {
                self.dfx.lock().get_logger().lock().log(LogLevel::Trace, "Render", &format!("Frame {}: {} vertices + {} preview vertices", self.frame_count, vertices.len() / 8, preview_vertices.len() / 8), file!(), line!());
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
                        if key == Key::S {
                            self.s_pressed = true;
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
                    if codepoint >= ' ' {
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
    
    pub fn get_extent(&self) -> (u32, u32) {
        (self.extent.width, self.extent.height)
    }
    
    pub fn get_glfw_time(&self) -> f64 {
        self.glfw.get_time()
    }
    
    pub fn capture_screenshot(&mut self, filepath: &str) -> Result<(), String> {
        unsafe {
            self.device.device_wait_idle()
                .map_err(|e| format!("Failed to wait for device idle: {}", e))?;
            
            let fence = self.device.create_fence(&vk::FenceCreateInfo::default(), None)
                .map_err(|e| format!("Failed to create fence: {}", e))?;
            
            let (image_index, _suboptimal) = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                vk::Semaphore::null(),
                fence
            ).map_err(|e| format!("Failed to acquire image: {}", e))?;
            
            self.device.wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fence: {}", e))?;
            self.device.destroy_fence(fence, None);
            
            let image_index_usize = image_index as usize;
            let swapchain_image = self.swapchain_images[image_index_usize];
            
            let width = self.extent.width;
            let height = self.extent.height;
            let buffer_size = (width * height * 4) as usize;
            
            let buffer_create_info = vk::BufferCreateInfo {
                size: buffer_size as u64,
                usage: vk::BufferUsageFlags::TRANSFER_DST,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                flags: vk::BufferCreateFlags::empty(),
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
                _marker: std::marker::PhantomData,
                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
            };
            
            let buffer = self.device.create_buffer(&buffer_create_info, None)
                .map_err(|e| format!("Failed to create buffer: {}", e))?;
            
            let memory_requirements = self.device.get_buffer_memory_requirements(buffer);
            
            let memory_properties = self.instance.get_physical_device_memory_properties(self.physical_device);
            let memory_type_index = memory_properties.memory_types.iter().enumerate()
                .find(|(i, mem_type)| {
                    (memory_requirements.memory_type_bits & (1 << i)) != 0
                        && mem_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                })
                .map(|(i, _)| i as u32)
                .expect("Failed to find suitable memory type");
            
            let allocate_info = vk::MemoryAllocateInfo {
                allocation_size: memory_requirements.size,
                memory_type_index,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
                _marker: std::marker::PhantomData,
            };
            
            let buffer_memory = self.device.allocate_memory(&allocate_info, None)
                .map_err(|e| format!("Failed to allocate buffer memory: {}", e))?;
            
            self.device.bind_buffer_memory(buffer, buffer_memory, 0)
                .map_err(|e| format!("Failed to bind buffer memory: {}", e))?;
            
            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
                command_pool: self.command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: 1,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
                _marker: std::marker::PhantomData,
            };
            
            let command_buffers = self.device.allocate_command_buffers(&command_buffer_allocate_info)
                .map_err(|e| format!("Failed to allocate command buffers: {}", e))?;
            let command_buffer = command_buffers[0];
            
            let begin_info = vk::CommandBufferBeginInfo {
                flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                p_inheritance_info: std::ptr::null(),
                p_next: std::ptr::null(),
                s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                _marker: std::marker::PhantomData,
            };
            
            self.device.begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| format!("Failed to begin command buffer: {}", e))?;
            
            let image_barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: swapchain_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::MEMORY_READ,
                dst_access_mask: vk::AccessFlags::TRANSFER_READ,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                _marker: std::marker::PhantomData,
            };
            
            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier]
            );
            
            let buffer_image_copy = vk::BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                image_extent: vk::Extent3D { width, height, depth: 1 },
            };
            
            self.device.cmd_copy_image_to_buffer(
                command_buffer,
                swapchain_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                buffer,
                &[buffer_image_copy]
            );
            
            let image_barrier2 = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: swapchain_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::TRANSFER_READ,
                dst_access_mask: vk::AccessFlags::MEMORY_READ,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                _marker: std::marker::PhantomData,
            };
            
            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier2]
            );
            
            self.device.end_command_buffer(command_buffer)
                .map_err(|e| format!("Failed to end command buffer: {}", e))?;
            
            let submit_info = vk::SubmitInfo {
                wait_semaphore_count: 0,
                p_wait_semaphores: std::ptr::null(),
                p_wait_dst_stage_mask: std::ptr::null(),
                command_buffer_count: 1,
                p_command_buffers: &command_buffer,
                signal_semaphore_count: 0,
                p_signal_semaphores: std::ptr::null(),
                p_next: std::ptr::null(),
                s_type: vk::StructureType::SUBMIT_INFO,
                _marker: std::marker::PhantomData,
            };
            
            let fence = self.device.create_fence(&vk::FenceCreateInfo::default(), None)
                .map_err(|e| format!("Failed to create fence: {}", e))?;
            
            self.device.queue_submit(self.queue, &[submit_info], fence)
                .map_err(|e| format!("Failed to submit queue: {}", e))?;
            
            self.device.wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fence: {}", e))?;
            
            self.device.destroy_fence(fence, None);
            
            let data_ptr = self.device.map_memory(buffer_memory, 0, buffer_size as u64, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map memory: {}", e))?;
            
            let data_slice = std::slice::from_raw_parts(data_ptr as *const u8, buffer_size);
            
            let mut pixels: Vec<u8> = data_slice.to_vec();
            
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize * 4;
                    let r = pixels[idx];
                    let g = pixels[idx + 1];
                    let b = pixels[idx + 2];
                    let a = pixels[idx + 3];
                    pixels[idx] = b;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = r;
                    pixels[idx + 3] = a;
                }
            }
            
            self.device.unmap_memory(buffer_memory);
            
            let img_buffer: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = 
                image::ImageBuffer::from_raw(width, height, pixels)
                    .expect("Failed to create image buffer");
            
            img_buffer.save(filepath)
                .map_err(|e| format!("Failed to save image: {}", e))?;
            
            self.device.free_command_buffers(self.command_pool, &command_buffers);
            self.device.destroy_buffer(buffer, None);
            self.device.free_memory(buffer_memory, None);
            
            self.device.device_wait_idle()
                .map_err(|e| format!("Failed to wait for device idle after screenshot: {}", e))?;
            
            let present_info = vk::PresentInfoKHR {
                wait_semaphore_count: 0,
                p_wait_semaphores: std::ptr::null(),
                swapchain_count: 1,
                p_swapchains: &self.swapchain,
                p_image_indices: &image_index,
                p_results: std::ptr::null_mut(),
                p_next: std::ptr::null(),
                s_type: vk::StructureType::PRESENT_INFO_KHR,
                _marker: std::marker::PhantomData,
            };
            
            self.swapchain_loader.queue_present(self.queue, &present_info).ok();
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Info,
                "Screenshot",
                &format!("Screenshot saved to {}", filepath),
                file!(),
                line!()
            );
            
            Ok(())
        }
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
