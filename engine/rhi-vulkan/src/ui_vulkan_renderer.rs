use ash::vk::{self, Handle};
use ash::khr::surface::Instance as SurfaceLoader;
use ash::khr::swapchain::Device as SwapchainLoader;
use glfw::{Glfw, PWindow, GlfwReceiver, WindowEvent, WindowMode, Action, Key};
use std::ffi::CString;


use std::collections::HashMap;
use hezhou_ui::{UISystem, UIInputHandler, Panel, Button, Label, TextEdit, Layout, DrawCommand, Widget, Style, Color, TextStyle, ffi::WidgetTreeHandle, ffi::ui_set_primary_button_id};
use hezhou_platform::{MouseAction, MouseEvent, MouseButton, CharEvent, KeyEvent, KeyAction, KeyModifiers, KeyCode, convert_glfw_key};
use hezhou_dfx::{DfxSystem, LogLevel, dfx_debug, dfx_info};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use hezhou_core::asset_library::MeshType;
use crate::primitive_meshes::{self, MeshRange};

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

/// 纹理缓存条目：存储per-entity纹理的Vulkan资源
struct TextureCacheEntry {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    sampler: vk::Sampler,
    descriptor_set: vk::DescriptorSet,
}

/// 自定义OBJ模型的GPU buffer信息
#[derive(Clone)]
struct CustomMeshInfo {
    vertex_offset: u32,  // 在game_mesh_buffer中的偏移（vertex单位）
    vertex_count: u32,
    index_offset: u32,   // 在game_index_buffer中的偏移（index单位）
    index_count: u32,
    has_texture: bool,   // 是否有纹理（push constant的has_texture标志）
    texture_descriptor_set: Option<vk::DescriptorSet>,  // 纹理descriptor set（如果有）
    specular_strength: f32,    // 高光强度
    ambient_strength: f32,     // 环境光强度
    shininess: f32,            // 高光指数
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
    vertex_buffers: [vk::Buffer; 2],
    vertex_buffer_memories: [vk::DeviceMemory; 2],
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
    p_pressed: bool,  // P键 — 截取预览窗画面
    glyph_cache: GlyphCache,
    button_clicked: Arc<AtomicBool>,
    needs_resize: bool,
    new_extent: vk::Extent2D,
    pending_offscreen_resize: Option<vk::Extent2D>,  // Deferred FBO recreation to avoid mid-frame device_wait_idle
    swapchain_format: vk::Format,
    physical_device: vk::PhysicalDevice,
    first_frame: bool,  // Track first frame for correct image layout transitions
    triangle_angle: f32,
    last_frame_time: f64,
    content_scale: f32,
    
    // Game preview rendering (offscreen + FXAA)
    game_render_pass: vk::RenderPass,
    game_pipeline: vk::Pipeline,
    game_pipeline_layout: vk::PipelineLayout,
    texture_descriptor_set_layout: vk::DescriptorSetLayout,
    texture_descriptor_pool: vk::DescriptorPool,
    default_texture_descriptor_set: vk::DescriptorSet,
    default_texture_image: vk::Image,
    default_texture_memory: vk::DeviceMemory,
    default_texture_view: vk::ImageView,
    default_texture_sampler: vk::Sampler,
    outline_pipeline: vk::Pipeline,  // for rendering selection outline
    offscreen_image: vk::Image,
    offscreen_image_memory: vk::DeviceMemory,
    offscreen_image_view: vk::ImageView,
    offscreen_framebuffer: vk::Framebuffer,
    offscreen_extent: vk::Extent2D,
    offscreen_format: vk::Format,
    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,
    
    // FXAA output (offscreen image after FXAA processing)
    offscreen_fxaa_image: vk::Image,
    offscreen_fxaa_image_memory: vk::DeviceMemory,
    offscreen_fxaa_image_view: vk::ImageView,
    offscreen_fxaa_framebuffer: vk::Framebuffer,
    
    // Preview texture descriptor (for UI to sample FXAA output)
    preview_descriptor_set: vk::DescriptorSet,
    preview_sampler: vk::Sampler,
    
    // FXAA post-processing
    fxaa_pipeline: vk::Pipeline,
    fxaa_pipeline_layout: vk::PipelineLayout,
    fxaa_descriptor_set_layout: vk::DescriptorSetLayout,
    fxaa_descriptor_pool: vk::DescriptorPool,
    fxaa_descriptor_set: vk::DescriptorSet,
    fxaa_sampler: vk::Sampler,
    
    // Camera parameters
    camera_yaw: f32,
    camera_pitch: f32,
    camera_x: f32,
    camera_y: f32,
    camera_z: f32,
    
    // Game state: 0=Editing, 1=Running, 2=Paused
    game_state: i32,
    
    // Entity transforms for rendering (simple: single cube for now)
    entity_position: [f32; 3],
    entity_rotation: [f32; 4],  // quaternion (x, y, z, w)
    entity_scale: [f32; 3],
    entity_angle: f32,  // rotation angle in degrees (for simple rotation)
    
    // Selection highlight
    selected_entity_id: u64,
    is_entity_selected: bool,
    highlight_color: [f32; 4],  // RGBA (orange: 1.0, 0.6, 0.3, 1.0)
    
    // Game mesh buffer (separate from UI vertex buffer)
    game_mesh_buffer: vk::Buffer,
    game_mesh_buffer_memory: vk::DeviceMemory,
    primitive_ranges: HashMap<MeshType, MeshRange>,
    
    // 动态mesh管理（自定义OBJ模型）
    custom_meshes: HashMap<String, CustomMeshInfo>,
    texture_cache: HashMap<String, TextureCacheEntry>,
    // 动态数据脏标记 — custom_meshes/texture_cache变更时设为true，
    // 仅在flag=true时重建管线同步HashMap，避免每帧无意义克隆
    dirty_dynamic_data: bool,
    game_index_buffer: vk::Buffer,
    game_index_buffer_memory: vk::DeviceMemory,
    
    // Scene pointer for multi-entity rendering
    scene_ptr: Option<*mut hezhou_core::Scene>,
    
    // 可扩展渲染管线系统（可选启用）
    // 默认None走legacy路径，启用后委托给PipelineRegistry
    pipeline_registry: Option<hezhou_render_pipeline::PipelineRegistry>,
    
    // 截图源图像 — 记录当前帧实际渲染结果的image和extent
    // rasterization模式: offscreen_image(经FXAA后)
    // ray_tracing模式: RayTracePipeline.output_image
    screenshot_source_image: vk::Image,
    screenshot_source_extent: vk::Extent2D,
    
    // 延迟销毁列表 — resize时不能立即销毁的Vulkan资源（framebuffer等）
    // GPU可能还在使用旧帧的command buffer引用这些资源，必须延迟到确认GPU空闲后再销毁
    // 每项记录(退役帧号, handle) — 只销毁帧号 >= 当前帧-DEFERRED_DESTROY_FRAMES 的资源
    deferred_destroy_framebuffers: Vec<(u64, vk::Framebuffer)>,
    deferred_destroy_image_views: Vec<(u64, vk::ImageView)>,
    deferred_destroy_images: Vec<(u64, vk::Image)>,
    deferred_destroy_memories: Vec<(u64, vk::DeviceMemory)>,
    
    // 帧计数器 — 用于延迟销毁的帧号判断
    frame_counter: u64,
    
    // Light UBO (directional light data for fragment shader)
    light_ubo: vk::Buffer,
    light_ubo_memory: vk::DeviceMemory,
    light_descriptor_set_layout: vk::DescriptorSetLayout,
    light_descriptor_pool: vk::DescriptorPool,
    light_descriptor_set: vk::DescriptorSet,
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
            let mut extension_names: Vec<CString> = glfw_extensions
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
            
            // Log physical device limits — critical for push constant sizing
            let device_props = instance.get_physical_device_properties(physical_device);
            let dev_name = unsafe { std::ffi::CStr::from_ptr(device_props.device_name.as_ptr()) }
                .to_string_lossy().to_string();
            logger.lock().log(LogLevel::Info, "Vulkan", 
                &format!("Physical device: {}", dev_name),
                file!(), line!());
            
            
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
                        front_face: vk::FrontFace::CLOCKWISE,
                        line_width: 1.0,
                        ..Default::default()
                    },
                    p_depth_stencil_state: &vk::PipelineDepthStencilStateCreateInfo {
                        depth_test_enable: vk::FALSE,  // UI render pass没有depth attachment，启用depth test是Vulkan验证错误(VUID-04864)和未定义行为
                        depth_write_enable: vk::FALSE,  // 2D UI不需要depth write，用painter's algorithm(绘制顺序)处理重叠
                        depth_compare_op: vk::CompareOp::NEVER,  // depth_test=FALSE时compare op无意义，设NEVER明确
                        depth_bounds_test_enable: vk::FALSE,
                        stencil_test_enable: vk::FALSE,
                        min_depth_bounds: 0.0,
                        max_depth_bounds: 1.0,
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
            // initialLayout=COLOR_ATTACHMENT_OPTIMAL because we use explicit begin barriers
            // to transition from UNDEFINED/SHADER_READ_ONLY to COLOR_ATTACHMENT before render pass.
            // This prevents the render pass from doing an implicit begin transition that
            // conflicts with our explicit barriers (VUID-01197 fix).
            // Subpass dependencies ensure the implicit end transition (COLOR_ATTACHMENT →
            // SHADER_READ_ONLY, per finalLayout) is properly synchronized.
            let game_subpass_dependencies = [
                // External → Subpass 0: wait for previous shader reads before writing color
                vk::SubpassDependency {
                    src_subpass: vk::SUBPASS_EXTERNAL,
                    dst_subpass: 0,
                    src_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                    dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                    src_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
                    dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    dependency_flags: vk::DependencyFlags::empty(),
                    ..Default::default()
                },
                // Subpass 0 → External: make color/depth output visible to subsequent shader reads
                vk::SubpassDependency {
                    src_subpass: 0,
                    dst_subpass: vk::SUBPASS_EXTERNAL,
                    src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                    dst_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
                    src_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                    dependency_flags: vk::DependencyFlags::empty(),
                    ..Default::default()
                },
            ];
            let game_render_pass = device.create_render_pass(&vk::RenderPassCreateInfo {
                attachment_count: 2,
                p_attachments: &[
                    // Color attachment
                    vk::AttachmentDescription {
                        format: offscreen_format,
                        samples: vk::SampleCountFlags::TYPE_1,
                        load_op: vk::AttachmentLoadOp::CLEAR,
                        store_op: vk::AttachmentStoreOp::STORE,
                        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                        initial_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        ..Default::default()
                    },
                    // Depth attachment (load_op=CLEAR allows initialLayout=UNDEFINED)
                    vk::AttachmentDescription {
                        format: vk::Format::D32_SFLOAT,
                        samples: vk::SampleCountFlags::TYPE_1,
                        load_op: vk::AttachmentLoadOp::CLEAR,
                        store_op: vk::AttachmentStoreOp::DONT_CARE,
                        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                        initial_layout: vk::ImageLayout::UNDEFINED,
                        final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        ..Default::default()
                    }
                ] as *const _,
                subpass_count: 1,
                p_subpasses: &vk::SubpassDescription {
                    pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
                    color_attachment_count: 1,
                    p_color_attachments: &vk::AttachmentReference {
                        attachment: 0,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    },
                    p_depth_stencil_attachment: &vk::AttachmentReference {
                        attachment: 1,
                        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    },
                    ..Default::default()
                },
                dependency_count: game_subpass_dependencies.len() as u32,
                p_dependencies: game_subpass_dependencies.as_ptr(),
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create game render pass: {}", e))?;
            
            // Create depth image
            let (depth_image, depth_image_memory) = Self::create_depth_image(
                &instance, &device, physical_device, offscreen_extent
            ).map_err(|e| format!("Failed to create depth image: {}", e))?;
            
            let depth_image_view = device.create_image_view(&vk::ImageViewCreateInfo {
                image: depth_image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: vk::Format::D32_SFLOAT,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create depth image view: {}", e))?;
            
            let offscreen_framebuffer = device.create_framebuffer(&vk::FramebufferCreateInfo {
                render_pass: game_render_pass,
                attachment_count: 2,
                p_attachments: &[offscreen_image_view, depth_image_view] as *const _,
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
            
            // FXAA framebuffer (uses same game_render_pass with 2 attachments)
            let offscreen_fxaa_framebuffer = device.create_framebuffer(&vk::FramebufferCreateInfo {
                render_pass: game_render_pass,
                attachment_count: 2,
                p_attachments: &[offscreen_fxaa_image_view, depth_image_view] as *const _,
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
            
            // Light UBO descriptor set layout (for directional light data in fragment shader)
            let light_descriptor_set_layout = device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                binding_count: 1,
                p_bindings: &vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::FRAGMENT,
                    p_immutable_samplers: std::ptr::null(),
                    ..Default::default()
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create light descriptor set layout: {}", e))?;
            
            // Texture descriptor set layout (for combined image sampler in fragment shader)
            let texture_descriptor_set_layout = device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                binding_count: 1,
                p_bindings: &[
                    vk::DescriptorSetLayoutBinding {
                        binding: 0,
                        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: 1,
                        stage_flags: vk::ShaderStageFlags::FRAGMENT,
                        p_immutable_samplers: std::ptr::null(),
                        ..Default::default()
                    },
                ] as *const _,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create texture descriptor set layout: {}", e))?;
            
            let game_pipeline_layout = device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                set_layout_count: 2,
                p_set_layouts: &[light_descriptor_set_layout, texture_descriptor_set_layout] as *const _,
                push_constant_range_count: 1,
                p_push_constant_ranges: &vk::PushConstantRange {
                    stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    offset: 0,
                    size: 144, // mat4(64) + outline_color(12) + is_selected(4) + viewport_size(8) + _pad(8) + camera_pos(12) + camera_yaw(4) + camera_pitch(4) + _pad(4) + has_texture(4) + specular_strength(4) + ambient_strength(4) + shininess(4) = 144
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create game pipeline layout: {}", e))?;
            
            // Main game pipeline (FILL mode)
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
                        vertex_binding_description_count: 1,
                        p_vertex_binding_descriptions: &[vk::VertexInputBindingDescription {
                            binding: 0,
                            stride: 32,  // sizeof(MeshVertex) = position(12) + normal(12) + uv(8)
                            input_rate: vk::VertexInputRate::VERTEX,
                        }] as *const _,
                        vertex_attribute_description_count: 3,
                        p_vertex_attribute_descriptions: &[
                            vk::VertexInputAttributeDescription {
                                binding: 0,
                                location: 0,
                                format: vk::Format::R32G32B32_SFLOAT,
                                offset: 0,
                            },
                            vk::VertexInputAttributeDescription {
                                binding: 0,
                                location: 1,
                                format: vk::Format::R32G32B32_SFLOAT,
                                offset: 12,
                            },
                            vk::VertexInputAttributeDescription {
                                binding: 0,
                                location: 2,
                                format: vk::Format::R32G32_SFLOAT,
                                offset: 24,  // UV coordinates
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
                    p_depth_stencil_state: &vk::PipelineDepthStencilStateCreateInfo {
                        depth_test_enable: vk::TRUE,
                        depth_write_enable: vk::TRUE,
                        depth_compare_op: vk::CompareOp::LESS,
                        depth_bounds_test_enable: vk::FALSE,
                        stencil_test_enable: vk::FALSE,
                        ..Default::default()
                    },
                    p_dynamic_state: &vk::PipelineDynamicStateCreateInfo {
                        dynamic_state_count: 2,
                        p_dynamic_states: &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR] as *const _,
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
            
            // Outline pipeline (render back faces with orange color)
            let outline_vert_shader = Self::create_shader_module(&device, game_vert_code)?;
            let outline_frag_shader = Self::create_shader_module(&device, game_frag_code)?;
            
            let outline_pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(), &[
                vk::GraphicsPipelineCreateInfo {
                    stage_count: 2,
                    p_stages: &[
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::VERTEX,
                            module: outline_vert_shader,
                            p_name: b"main\0".as_ptr() as *const i8,
                            ..Default::default()
                        },
                        vk::PipelineShaderStageCreateInfo {
                            stage: vk::ShaderStageFlags::FRAGMENT,
                            module: outline_frag_shader,
                            p_name: b"main\0".as_ptr() as *const i8,
                            ..Default::default()
                        },
                    ] as *const _,
                    p_vertex_input_state: &vk::PipelineVertexInputStateCreateInfo {
                        vertex_binding_description_count: 1,
                        p_vertex_binding_descriptions: &[vk::VertexInputBindingDescription {
                            binding: 0,
                            stride: 32,  // sizeof(MeshVertex) = position(12) + normal(12) + uv(8)
                            input_rate: vk::VertexInputRate::VERTEX,
                        }] as *const _,
                        vertex_attribute_description_count: 3,
                        p_vertex_attribute_descriptions: &[
                            vk::VertexInputAttributeDescription {
                                binding: 0,
                                location: 0,
                                format: vk::Format::R32G32B32_SFLOAT,
                                offset: 0,
                            },
                            vk::VertexInputAttributeDescription {
                                binding: 0,
                                location: 1,
                                format: vk::Format::R32G32B32_SFLOAT,
                                offset: 12,
                            },
                            vk::VertexInputAttributeDescription {
                                binding: 0,
                                location: 2,
                                format: vk::Format::R32G32_SFLOAT,
                                offset: 24,  // UV coordinates
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
                        cull_mode: vk::CullModeFlags::NONE,  // Don't cull - winding order changes with rotation
                        front_face: vk::FrontFace::CLOCKWISE,
                        line_width: 1.0,
                        ..Default::default()
                    },
                    p_multisample_state: &vk::PipelineMultisampleStateCreateInfo {
                        rasterization_samples: vk::SampleCountFlags::TYPE_1,
                        ..Default::default()
                    },
                    p_dynamic_state: &vk::PipelineDynamicStateCreateInfo {
                        dynamic_state_count: 2,
                        p_dynamic_states: &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR] as *const _,
                        ..Default::default()
                    },
                    p_depth_stencil_state: &vk::PipelineDepthStencilStateCreateInfo {
                        depth_test_enable: vk::TRUE,   // Do depth test for proper occlusion
                        depth_write_enable: vk::FALSE,  // Don't write depth (read-only)
                        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,  // Render at same depth or closer
                        depth_bounds_test_enable: vk::FALSE,
                        stencil_test_enable: vk::FALSE,
                        ..Default::default()
                    },
                    p_color_blend_state: &vk::PipelineColorBlendStateCreateInfo {
                        logic_op_enable: vk::FALSE,
                        attachment_count: 1,
                        p_attachments: &vk::PipelineColorBlendAttachmentState {
                            blend_enable: vk::TRUE,
                            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
                            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
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
            ], None).map_err(|(_, e)| format!("Failed to create outline pipeline: {}", e))?[0];
            
            device.destroy_shader_module(outline_vert_shader, None);
            device.destroy_shader_module(outline_frag_shader, None);
            
            // Light UBO buffer (32 bytes): shader uses std430 layout where vec3 occupies 12 bytes (not padded to vec4).
            // direction (vec3): offset 0, 12 bytes. Gap: offset 12-15 (4 bytes, alignment pad for next vec3).
            // color (vec3): offset 16, 12 bytes.
            // intensity (float): offset 28, 4 bytes.
            // Struct total = 32 bytes, rounded to multiple of largest alignment (16) = 32 bytes.
            // Data: [dir.x, dir.y, dir.z, pad, color.x, color.y, color.z, intensity] = 8 floats = 32 bytes
            let light_ubo_size: vk::DeviceSize = 32;
            let light_ubo = device.create_buffer(&vk::BufferCreateInfo {
                size: light_ubo_size,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create light UBO buffer: {}", e))?;
            
            let light_ubo_mem_requirements = device.get_buffer_memory_requirements(light_ubo);
            let light_ubo_mem_properties = instance.get_physical_device_memory_properties(physical_device);
            let light_ubo_memory_type_index = Self::find_memory_type(
                light_ubo_mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                &light_ubo_mem_properties
            );
            
            let light_ubo_memory = device.allocate_memory(&vk::MemoryAllocateInfo {
                allocation_size: light_ubo_mem_requirements.size,
                memory_type_index: light_ubo_memory_type_index,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to allocate light UBO memory: {}", e))?;
            
            device.bind_buffer_memory(light_ubo, light_ubo_memory, 0)
                .map_err(|e| format!("Failed to bind light UBO memory: {}", e))?;
            
            // Write default light data to UBO: direction=(-0.5,-1.0,-0.5), color=(1,1,1), intensity=1.0
            // std430 layout: direction(3 floats, offset 0) + pad(1 float, offset 12) + color(3 floats, offset 16) + intensity(1 float, offset 28)
            // = 8 floats = 32 bytes. Intensity is at offset 28 (NOT 32 as in std140!)
            {
                let default_light_data: [f32; 8] = [-0.5, -1.0, -0.5, 0.0, 1.0, 1.0, 1.0, 1.0];
                let data_ptr = device.map_memory(light_ubo_memory, 0, light_ubo_size, vk::MemoryMapFlags::empty())
                    .map_err(|e| format!("Failed to map light UBO memory: {}", e))?;
                std::ptr::copy_nonoverlapping(
                    default_light_data.as_ptr() as *const u8,
                    data_ptr as *mut u8,
                    32,
                );
                device.unmap_memory(light_ubo_memory);
            }
            
            // Light descriptor pool and set
            let light_descriptor_pool = device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                max_sets: 1,
                pool_size_count: 1,
                p_pool_sizes: &vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create light descriptor pool: {}", e))?;
            
            // Texture descriptor pool (for combined image sampler)
            let texture_descriptor_pool = device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                max_sets: 64,
                pool_size_count: 1,
                p_pool_sizes: &[
                    vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count: 64,
                        ..Default::default()
                    },
                ] as *const _,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create texture descriptor pool: {}", e))?;
            
            let light_descriptor_set = device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                descriptor_pool: light_descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &light_descriptor_set_layout,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate light descriptor set: {}", e))?[0];
            
            // Update light descriptor set with UBO buffer info
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: light_descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &vk::DescriptorBufferInfo {
                        buffer: light_ubo,
                        offset: 0,
                        range: light_ubo_size,
                    },
                    ..Default::default()
                }],
                &[]
            );
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Light UBO and descriptor set created", file!(), line!());
            
 // Default texture (1x1 white, for entities without texture)
            let default_texture_data: [u8; 4] = [255, 255, 255, 255]; // 1x1 RGBA white
            let (default_texture_image, default_texture_memory) = Self::create_texture_2d_simple(
                &instance, &device, physical_device, 1, 1, &default_texture_data)?;
            
            // Transition default texture from PREINITIALIZED to SHADER_READ_ONLY_OPTIMAL
            // CRITICAL: create_texture_2d_simple creates image with PREINITIALIZED layout,
            // but descriptor set declares SHADER_READ_ONLY_OPTIMAL. Without this transition,
            // sampling the texture causes VUID-09600 → device lost → crash (0xcfffffff)
            unsafe {
                let transition_cmd = device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                    command_pool,
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: 1,
                    ..Default::default()
                }).map_err(|e| format!("Failed to allocate default texture transition cmd: {}", e))?[0];
                
                device.begin_command_buffer(transition_cmd, &vk::CommandBufferBeginInfo::default())
                    .map_err(|e| format!("Failed to begin default texture transition cmd: {}", e))?;
                
                let default_texture_barrier = vk::ImageMemoryBarrier {
                    old_layout: vk::ImageLayout::PREINITIALIZED,
                    new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    src_access_mask: vk::AccessFlags::HOST_WRITE,
                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                    src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    image: default_texture_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    ..Default::default()
                };
                
                device.cmd_pipeline_barrier(
                    transition_cmd,
                    vk::PipelineStageFlags::HOST,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[] as &[vk::MemoryBarrier],
                    &[] as &[vk::BufferMemoryBarrier],
                    &[default_texture_barrier],
                );
                
                device.end_command_buffer(transition_cmd)
                    .map_err(|e| format!("Failed to end default texture transition cmd: {}", e))?;
                
                device.queue_submit(queue, &[vk::SubmitInfo {
                    command_buffer_count: 1,
                    p_command_buffers: &transition_cmd,
                    ..Default::default()
                }], vk::Fence::null())
                    .map_err(|e| format!("Failed to submit default texture transition: {}", e))?;
                
                device.queue_wait_idle(queue)
                    .map_err(|e| format!("Failed to wait for default texture transition: {}", e))?;
                
                device.free_command_buffers(command_pool, &[transition_cmd]);
            }
            
            let default_texture_view = device.create_image_view(&vk::ImageViewCreateInfo {
                view_type: vk::ImageViewType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image: default_texture_image,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create default texture view: {}", e))?;
            
            let default_texture_sampler = device.create_sampler(&vk::SamplerCreateInfo {
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
            }, None).map_err(|e| format!("Failed to create default texture sampler: {}", e))?;
            
            // Allocate texture descriptor set
            let default_texture_descriptor_set = device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                descriptor_pool: texture_descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &texture_descriptor_set_layout,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate default texture descriptor set: {}", e))?[0];
            
            // Update texture descriptor set with image info
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: default_texture_descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &vk::DescriptorImageInfo {
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        image_view: default_texture_view,
                        sampler: default_texture_sampler,
                    },
                    ..Default::default()
                }],
                &[]
            );
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Default texture and descriptor set created", file!(), line!());
            
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
                    p_depth_stencil_state: &vk::PipelineDepthStencilStateCreateInfo {
                        depth_test_enable: vk::FALSE,
                        depth_write_enable: vk::FALSE,
                        depth_compare_op: vk::CompareOp::NEVER,
                        ..Default::default()
                    },
                    p_dynamic_state: &vk::PipelineDynamicStateCreateInfo {
                        dynamic_state_count: 2,
                        p_dynamic_states: &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR] as *const _,
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
            
            // 双缓冲vertex buffer：每帧使用current_frame索引对应的buffer，
            // 防止CPU写入buffer[N]时GPU还在读取buffer[N]（跨帧竞争条件）
            let (vb0, vbm0) = Self::create_vertex_buffer(
                &instance, &device, physical_device, 33554432
            )?;
            let (vb1, vbm1) = Self::create_vertex_buffer(
                &instance, &device, physical_device, 33554432
            )?;
            let vertex_buffers = [vb0, vb1];
            let vertex_buffer_memories = [vbm0, vbm1];
            
            logger.lock().log(LogLevel::Info, "Vulkan", "Vertex buffers created (2x 32MB, double-buffered)", file!(), line!());
            
            // Create game mesh buffer for primitive meshes + 预留1MB空间用于自定义mesh
            let primitive_meshes = primitive_meshes::get_primitive_meshes();
            let mesh_vertex_data: &[primitive_meshes::MeshVertex] = primitive_meshes.vertices();
            let primitive_data_size = (mesh_vertex_data.len() * std::mem::size_of::<primitive_meshes::MeshVertex>()) as vk::DeviceSize;  // MeshVertex = 32 bytes
            let mesh_buffer_total_size = primitive_data_size + (1024 * 1024) as vk::DeviceSize;  // 预留1MB
            
            let (game_mesh_buffer, game_mesh_buffer_memory) = Self::create_vertex_buffer(
                &instance, &device, physical_device, mesh_buffer_total_size
            )?;
            
            // Upload primitive mesh vertex data（只上传到buffer开头部分）
            unsafe {
                let data_ptr = device.map_memory(game_mesh_buffer_memory, 0, primitive_data_size, vk::MemoryMapFlags::empty())
                    .map_err(|e| format!("Failed to map game mesh buffer: {}", e))?;
                std::ptr::copy_nonoverlapping(
                    mesh_vertex_data.as_ptr() as *const u8,
                    data_ptr as *mut u8,
                    primitive_data_size as usize,
                );
                device.unmap_memory(game_mesh_buffer_memory);
            }
            
            let primitive_ranges = primitive_meshes.ranges().clone();
            
            // 创建初始index buffer（用于自定义mesh，初始4KB）
            let index_buffer_initial_size: vk::DeviceSize = 4096;
            let (game_index_buffer, game_index_buffer_memory) = Self::create_buffer_with_usage(
                &instance, &device, physical_device,
                index_buffer_initial_size,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            logger.lock().log(LogLevel::Info, "Vulkan", &format!("Game mesh buffer created ({}, {} vertices, total {} with 1MB reserve)", primitive_data_size, mesh_vertex_data.len(), mesh_buffer_total_size), file!(), line!());
            
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
                vertex_buffers,
                vertex_buffer_memories,
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
            p_pressed: false,
                glyph_cache: GlyphCache::new(),
                button_clicked: Arc::new(AtomicBool::new(false)),
                needs_resize: false,
                new_extent: extent,
                pending_offscreen_resize: None,
                swapchain_format: format,
                physical_device,
                triangle_angle: 0.0,
                last_frame_time: 0.0,
                content_scale,
                
                game_render_pass,
                game_pipeline,
                game_pipeline_layout,
                texture_descriptor_set_layout,
                texture_descriptor_pool,
                default_texture_descriptor_set,
                default_texture_image,
                default_texture_memory,
                default_texture_view,
                default_texture_sampler,
                outline_pipeline,
                offscreen_image,
                offscreen_image_memory,
                offscreen_image_view,
                offscreen_framebuffer,
                offscreen_extent,
                offscreen_format,
                depth_image,
                depth_image_memory,
                depth_image_view,
                
                offscreen_fxaa_image,
                offscreen_fxaa_image_memory,
                offscreen_fxaa_image_view,
                offscreen_fxaa_framebuffer,
                
                preview_descriptor_set,
                preview_sampler,
                
                fxaa_pipeline,
                fxaa_pipeline_layout,
                fxaa_descriptor_set_layout,
                fxaa_descriptor_pool,
                fxaa_descriptor_set,
                fxaa_sampler,
                
                camera_yaw: 0.0,
                camera_pitch: 0.0,
                camera_x: 0.0,
                camera_y: 0.0,
                camera_z: 3.0,
                game_state: 0,  // Editing
                entity_position: [0.0, 0.0, 0.0],
                entity_rotation: [0.0, 0.0, 0.0, 1.0],  // identity quaternion
                entity_scale: [1.0, 1.0, 1.0],
                entity_angle: 0.0,
                selected_entity_id: 0,
                is_entity_selected: false,
                highlight_color: [1.0, 0.6, 0.3, 0.3],  // orange with high transparency
                game_mesh_buffer,
                game_mesh_buffer_memory,
                primitive_ranges,
                custom_meshes: HashMap::new(),
                texture_cache: HashMap::new(),
                dirty_dynamic_data: true,  // 首帧必须同步
                first_frame: true,
                game_index_buffer,
                game_index_buffer_memory,
                scene_ptr: None,
                
                light_ubo,
                light_ubo_memory,
                light_descriptor_set_layout,
                light_descriptor_pool,
                light_descriptor_set,
                
                // 可扩展渲染管线系统（默认不启用，保持向后兼容）
                pipeline_registry: None,
                // 截图源初始为offscreen_image（初始化时尚未渲染，后续draw_frame会更新）
                screenshot_source_image: offscreen_image,
                screenshot_source_extent: offscreen_extent,
            deferred_destroy_framebuffers: Vec::new(),
            deferred_destroy_image_views: Vec::new(),
            deferred_destroy_images: Vec::new(),
            deferred_destroy_memories: Vec::new(),
            frame_counter: 0,
            })
        }
    }
    
    /// 启用可扩展渲染管线系统
    ///
    /// 将现有Vulkan资源传入RasterPipeline，创建PipelineRegistry。
    /// 启用后，draw_frame()的游戏渲染部分将委托给PipelineRegistry。
    /// 不启用时，走现有legacy渲染路径（向后兼容）。
    pub fn enable_pipeline_system(&mut self) {
        if self.pipeline_registry.is_some() {
            return;  // 已经启用
        }
        
        // 转换primitive_ranges: MeshRange → PrimitiveMeshRange
        let pipeline_primitive_ranges: std::collections::HashMap<MeshType, hezhou_render_pipeline::PrimitiveMeshRange> =
            self.primitive_ranges.iter().map(|(k, v)| {
                (*k, hezhou_render_pipeline::PrimitiveMeshRange {
                    vertex_offset: v.vertex_offset,
                    vertex_count: v.vertex_count,
                    index_offset: 0,  // primitive mesh无index buffer
                    index_count: 0,
                })
            }).collect();
        
        // 转换custom_meshes: 本地CustomMeshInfo → render-pipeline CustomMeshInfo
        let pipeline_custom_meshes: std::collections::HashMap<String, hezhou_render_pipeline::CustomMeshInfo> =
            self.custom_meshes.iter().map(|(k, v)| {
                (k.clone(), hezhou_render_pipeline::CustomMeshInfo {
                    vertex_offset: v.vertex_offset,
                    vertex_count: v.vertex_count,
                    index_offset: v.index_offset,
                    index_count: v.index_count,
                    has_texture: v.has_texture,
                    texture_descriptor_set: v.texture_descriptor_set,
                    specular_strength: v.specular_strength,
                    ambient_strength: v.ambient_strength,
                    shininess: v.shininess,
                })
            }).collect();
        
        // 转换texture_cache: 本地TextureCacheEntry → render-pipeline TextureCacheEntry
        let pipeline_texture_cache: std::collections::HashMap<String, hezhou_render_pipeline::TextureCacheEntry> =
            self.texture_cache.iter().map(|(k, v)| {
                (k.clone(), hezhou_render_pipeline::TextureCacheEntry {
                    image: v.image,
                    memory: v.memory,
                    view: v.view,
                    sampler: v.sampler,
                    descriptor_set: v.descriptor_set,
                })
            }).collect();
        
        // 创建RasterPipeline — 传入所有现有Vulkan资源handle
        // RasterPipeline只引用handle，不拥有资源（资源所有权仍在UIVulkanRenderer）
        let raster_pipeline = hezhou_render_pipeline::RasterPipeline::new(
            self.game_pipeline,
            self.game_pipeline_layout,
            self.outline_pipeline,
            self.fxaa_pipeline,
            self.fxaa_pipeline_layout,
            self.fxaa_descriptor_set_layout,
            self.fxaa_descriptor_pool,
            self.fxaa_descriptor_set,
            self.fxaa_sampler,
            self.game_render_pass,
            self.light_ubo,
            self.light_ubo_memory,
            self.light_descriptor_set,
            self.default_texture_descriptor_set,
            self.texture_descriptor_set_layout,
            self.texture_descriptor_pool,
            self.light_descriptor_set_layout,
            self.light_descriptor_pool,
            self.game_mesh_buffer,
            self.game_mesh_buffer_memory,
            self.game_index_buffer,
            self.game_index_buffer_memory,
            self.offscreen_fxaa_image,
            self.offscreen_fxaa_image_memory,
            self.offscreen_fxaa_image_view,
            self.offscreen_fxaa_framebuffer,
            self.preview_descriptor_set,
            self.preview_sampler,
            self.device.clone(),
        );
        
        // 获取物理设备内存属性
        let memory_properties = unsafe {
            self.instance.get_physical_device_memory_properties(self.physical_device)
        };
        
        // 创建PipelineRegistry
        let mut registry = hezhou_render_pipeline::PipelineRegistry::new(
            self.device.handle(),
            memory_properties,
        );
        
        // 注册RasterPipeline（自动设为活跃管线）
        registry.register(Box::new(raster_pipeline));
        
        // 用UIVulkanRenderer的现有offscreen资源覆盖PipelineResources的null handle
        // PipelineResources由register()分配，但初始为null handle
        // 需要填入真实handle，因为RasterPipeline::record()使用resources.offscreen_framebuffer等
        {
            let resources = registry.active_resources_mut();
            resources.offscreen_image = Some(self.offscreen_image);
            resources.offscreen_memory = Some(self.offscreen_image_memory);
            resources.offscreen_view = Some(self.offscreen_image_view);
            resources.offscreen_framebuffer = Some(self.offscreen_framebuffer);
            resources.depth_image = Some(self.depth_image);
            resources.depth_memory = Some(self.depth_image_memory);
            resources.depth_view = Some(self.depth_image_view);
            resources.fxaa_image = Some(self.offscreen_fxaa_image);
            resources.fxaa_memory = Some(self.offscreen_fxaa_image_memory);
            resources.fxaa_view = Some(self.offscreen_fxaa_image_view);
            resources.fxaa_framebuffer = Some(self.offscreen_fxaa_framebuffer);
            resources.vertex_buffer = Some(self.game_mesh_buffer);
            resources.vertex_buffer_memory = Some(self.game_mesh_buffer_memory);
            resources.index_buffer = Some(self.game_index_buffer);
            resources.index_buffer_memory = Some(self.game_index_buffer_memory);
        }
        
        // 更新RasterPipeline的动态数据（primitive_ranges/custom_meshes/texture_cache）
        // 启用时同步一次，后续仅在dirty时同步
        registry.update_raster_dynamic_data(
            pipeline_primitive_ranges,
            pipeline_custom_meshes,
            pipeline_texture_cache,
        );
        self.dirty_dynamic_data = false;  // 初始同步完成，清除flag
        
        self.pipeline_registry = Some(registry);
    }
    
    /// 获取pipeline_registry的只读引用
    pub fn pipeline_registry(&self) -> Option<&hezhou_render_pipeline::PipelineRegistry> {
        self.pipeline_registry.as_ref()
    }
    
    /// 获取pipeline_registry的可变引用
    pub fn pipeline_registry_mut(&mut self) -> Option<&mut hezhou_render_pipeline::PipelineRegistry> {
        self.pipeline_registry.as_mut()
    }
    
    /// 切换活跃渲染管线
    ///
    /// **重要**: 内部会调用device_wait_idle()确保GPU不再使用旧管线资源，
    /// 否则可能导致VK_ERROR_DEVICE_LOST。
    ///
    /// 切换流程:
    /// 1. device_wait_idle() — 硬性约束
    /// 2. 如果pipeline_name=="ray_tracing"且尚未注册，创建并注册RayTracePipeline
    /// 3. 构建临时RenderContext
    /// 4. registry.switch_pipeline(pipeline_name, &render_ctx)
    /// 5. take_pending_destroy() — 销毁旧管线资源
    ///
    /// 返回true表示切换成功，false表示失败（管线不存在等）
    pub fn switch_pipeline(&mut self, pipeline_name: &str) -> bool {
        // 1. 检查pipeline_registry是否存在
        let registry = match self.pipeline_registry.as_mut() {
            Some(r) => r,
            None => return false,
        };
        
        // 2. device_wait_idle() — 硬性约束，否则VK_ERROR_DEVICE_LOST
        unsafe {
            self.device.device_wait_idle().expect("device_wait_idle failed before pipeline switch");
        }
        
        // 3. 如果pipeline_name=="ray_tracing"且尚未注册，创建并注册RayTracePipeline
        if pipeline_name == "ray_tracing" && !registry.pipeline_names().contains(&"ray_tracing") {
            let memory_properties = unsafe {
                self.instance.get_physical_device_memory_properties(self.physical_device)
            };
            let raytrace_pipeline = hezhou_render_pipeline::RayTracePipeline::new(
                self.device.clone(),
                memory_properties,
                self.offscreen_extent.width,
                self.offscreen_extent.height,
            );
            registry.register(Box::new(raytrace_pipeline));
        }
        
        // 4. 构建临时RenderContext
        let game_state_enum = match self.game_state {
            0 => hezhou_core::ecs::GameState::Editing,
            1 => hezhou_core::ecs::GameState::Running,
            _ => hezhou_core::ecs::GameState::Paused,
        };
        let render_ctx = hezhou_render_pipeline::RenderContext {
            scene: self.scene_ptr.unwrap_or(std::ptr::null_mut()),
            mode: hezhou_render_pipeline::RenderMode::from_game_state(game_state_enum),
            camera: hezhou_render_pipeline::CameraParams {
                position: [self.camera_x, self.camera_y, self.camera_z],
                yaw: self.camera_yaw,
                pitch: self.camera_pitch,
                fov: 45.0,
                near: 0.1,
                far: 100.0,
            },
            viewport_extent: hezhou_render_pipeline::ViewportExtent::new(
                self.offscreen_extent.width,
                self.offscreen_extent.height,
            ),
            selected_entity_ids: if let Some(scene_ptr) = self.scene_ptr {
                unsafe { (*scene_ptr).selected_entities.iter().map(|e| e.id).collect() }
            } else {
                Vec::new()
            },
            frame_index: self.frame_count as u32,
        };
        
// 5. 在switch_pipeline前，从active_resources取出renderer自有资源
        //    这些资源(offscreen/depth/fxaa/vertex/index)是renderer层拥有的核心渲染资源，
        //    不应被mark_for_destroy收集和销毁。
        //    .take()设为None，防止std::mem::take把它们带走→mark_for_destroy销毁
        let saved_offscreen_image = registry.active_resources_mut().offscreen_image.take();
        let saved_offscreen_memory = registry.active_resources_mut().offscreen_memory.take();
        let saved_offscreen_view = registry.active_resources_mut().offscreen_view.take();
        let saved_offscreen_framebuffer = registry.active_resources_mut().offscreen_framebuffer.take();
        let saved_depth_image = registry.active_resources_mut().depth_image.take();
        let saved_depth_memory = registry.active_resources_mut().depth_memory.take();
        let saved_depth_view = registry.active_resources_mut().depth_view.take();
        let saved_fxaa_image = registry.active_resources_mut().fxaa_image.take();
        let saved_fxaa_memory = registry.active_resources_mut().fxaa_memory.take();
        let saved_fxaa_view = registry.active_resources_mut().fxaa_view.take();
        let saved_fxaa_framebuffer = registry.active_resources_mut().fxaa_framebuffer.take();
        let saved_vertex_buffer = registry.active_resources_mut().vertex_buffer.take();
        let saved_vertex_buffer_memory = registry.active_resources_mut().vertex_buffer_memory.take();
        let saved_index_buffer = registry.active_resources_mut().index_buffer.take();
        let saved_index_buffer_memory = registry.active_resources_mut().index_buffer_memory.take();
        
        // 6. registry.switch_pipeline(pipeline_name, &render_ctx)
        let success = registry.switch_pipeline(pipeline_name, &render_ctx);
        
        // 7. take_pending_destroy() — 销毁旧管线资源（不含renderer自有资源）
        if success {
            let pending = registry.resource_manager_mut().take_pending_destroy();
            // 销毁pending中的所有Vulkan资源
            for buffer in &pending.buffers {
                unsafe { self.device.destroy_buffer(*buffer, None); }
            }
            for memory in &pending.memories {
                unsafe { self.device.free_memory(*memory, None); }
            }
            for image in &pending.images {
                unsafe { self.device.destroy_image(*image, None); }
            }
            for image_view in &pending.image_views {
                unsafe { self.device.destroy_image_view(*image_view, None); }
            }
            for framebuffer in &pending.framebuffers {
                unsafe { self.device.destroy_framebuffer(*framebuffer, None); }
            }
        }
        
        // 8. 把renderer自有资源填回新的active_resources
        //    无论新管线是否needs_offscreen_target/needs_depth_attachment，
        //    draw_frame始终需要这些资源（预览窗渲染、UI pass等）
        if success {
            let resources = registry.active_resources_mut();
            resources.offscreen_image = saved_offscreen_image;
            resources.offscreen_memory = saved_offscreen_memory;
            resources.offscreen_view = saved_offscreen_view;
            resources.offscreen_framebuffer = saved_offscreen_framebuffer;
            resources.depth_image = saved_depth_image;
            resources.depth_memory = saved_depth_memory;
            resources.depth_view = saved_depth_view;
            resources.fxaa_image = saved_fxaa_image;
            resources.fxaa_memory = saved_fxaa_memory;
            resources.fxaa_view = saved_fxaa_view;
            resources.fxaa_framebuffer = saved_fxaa_framebuffer;
            resources.vertex_buffer = saved_vertex_buffer;
            resources.vertex_buffer_memory = saved_vertex_buffer_memory;
            resources.index_buffer = saved_index_buffer;
            resources.index_buffer_memory = saved_index_buffer_memory;
        }
        
        success
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
    
    fn create_depth_image(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        extent: vk::Extent2D,
    ) -> Result<(vk::Image, vk::DeviceMemory), String> {
        unsafe {
            let image = device.create_image(&vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::D32_SFLOAT,
                extent: vk::Extent3D {
                    width: extent.width,
                    height: extent.height,
                    depth: 1,
                },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create depth image: {}", e))?;
            
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
            }, None).map_err(|e| format!("Failed to allocate depth memory: {}", e))?;
            
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| format!("Failed to bind depth memory: {}", e))?;
            
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
    
    /// 通用buffer创建方法（支持自定义usage flags和memory properties）
    fn create_buffer_with_usage(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
        unsafe {
            let buffer = device.create_buffer(&vk::BufferCreateInfo {
                size,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create buffer: {}", e))?;
            
            let mem_requirements = device.get_buffer_memory_requirements(buffer);
            let mem_props = instance.get_physical_device_memory_properties(physical_device);
            
            let mem_type_index = Self::find_memory_type(
                mem_requirements.memory_type_bits,
                memory_properties,
                &mem_props
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
                extent: vk::Extent3D { width: 8192, height: 8192, depth: 1 },
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
            
            let texture_data = vec![0u8; 4096 * 4096 * 4];
            std::ptr::copy_nonoverlapping(texture_data.as_ptr(), data_ptr as *mut u8, 4096 * 4096 * 4);
            
            device.unmap_memory(memory);
            
            Ok((image, memory))
        }
    }
    
    /// 创建简单的2D纹理（LINEAR tiling + HOST_VISIBLE内存，适合小纹理如1x1 default texture）
    fn create_texture_2d_simple(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<(vk::Image, vk::DeviceMemory), String> {
        unsafe {
            let image = device.create_image(&vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                extent: vk::Extent3D { width, height, depth: 1 },
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::LINEAR,
                usage: vk::ImageUsageFlags::SAMPLED,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout: vk::ImageLayout::PREINITIALIZED,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create texture image: {}", e))?;
            
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
            }, None).map_err(|e| format!("Failed to allocate texture memory: {}", e))?;
            
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| format!("Failed to bind texture memory: {}", e))?;
            
            // 上传像素数据
            let data_ptr = device.map_memory(memory, 0, mem_requirements.size, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map texture memory: {}", e))?;
            
            // LINEAR tiling的image需要考虑row pitch对齐
            // 对于1x1纹理，row pitch通常等于4字节（R8G8B8A8），但Vulkan可能要求更大对齐
            // 安全做法：逐行拷贝，考虑可能的pitch差异
            let row_pitch = mem_requirements.size / height as u64;  // Vulkan保证size >= height * row_pitch
            if row_pitch == (width as u64 * 4) {
                // 无对齐padding，直接拷贝
                std::ptr::copy_nonoverlapping(data.as_ptr(), data_ptr as *mut u8, data.len());
            } else {
                // 有对齐padding，逐行拷贝
                for y in 0..height as usize {
                    let src_offset = y * width as usize * 4;
                    let dst_offset = y * row_pitch as usize;
                    std::ptr::copy_nonoverlapping(
                        data.as_ptr().add(src_offset),
                        (data_ptr as *mut u8).add(dst_offset),
                        width as usize * 4,
                    );
                }
            }
            
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
        // Root panel uses extent dimensions directly.
        // Widget tree coordinates share the same pixel space as extent.
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
            
let mut font_atlas_guard = ui.get_font_atlas().lock();
            tree_guard.ensure_text_rasterized(&mut *font_atlas_guard);
            drop(font_atlas_guard);
let mut font_atlas_guard = ui.get_font_atlas().lock();
            tree_guard.perform_layout(&*font_atlas_guard);
            
            tree_guard.recenter_widget(vstack_id, self.extent.width as f32, self.extent.height as f32);
            tree_guard.perform_layout(&*font_atlas_guard);
            
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "VStack created with Button and Label", file!(), line!());
        }
        
        drop(tree_guard);
        
        let font_atlas_guard = ui.get_font_atlas().lock();
        let texture_data = font_atlas_guard.get_atlas_texture().to_vec();
        
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
        // Root panel uses extent dimensions directly.
        // Widget tree coordinates share the same pixel space as extent.
        root_panel.set_layout(Layout::new(0.0, 0.0, self.extent.width as f32, self.extent.height as f32));
        root_panel.set_style(Style::new().with_background(Color::transparent()));
        tree_guard.set_root(Box::new(root_panel));
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "UI", "Root panel created (C# will create widgets)", file!(), line!());
        
        drop(tree_guard);
        
        let font_atlas_guard = ui.get_font_atlas().lock();
        let texture_data = font_atlas_guard.get_atlas_texture().to_vec();
        
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
    
    pub fn get_event_dispatcher_handle(&self) -> hezhou_ui::ffi::EventDispatcherHandle {
        let ui = self.ui_system.lock();
        let dispatcher = ui.get_event_dispatcher();
        Box::into_raw(Box::new(dispatcher)) as hezhou_ui::ffi::EventDispatcherHandle
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
    
    pub fn is_p_pressed(&self) -> bool {
        self.p_pressed || self.window.get_key(Key::P) == Action::Press
    }
    
    pub fn consume_p_press(&mut self) {
        self.p_pressed = false;
    }
    
unsafe fn recreate_swapchain(&mut self) -> Result<(), String> {
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "recreate_swapchain: calling device_wait_idle...", file!(), line!());
        self.device.device_wait_idle()
            .map_err(|e| format!("Failed to wait for device idle: {}", e))?;
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "recreate_swapchain: device_wait_idle done, destroying old framebuffers...", file!(), line!());
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "recreate_swapchain: device_wait_idle done", file!(), line!());
        
        for (i, framebuffer) in self.framebuffers.iter().enumerate() {
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", &format!("recreate_swapchain: destroying framebuffer {}...", i), file!(), line!());
            self.device.destroy_framebuffer(*framebuffer, None);
        }
        for (i, view) in self.swapchain_image_views.iter().enumerate() {
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", &format!("recreate_swapchain: destroying image_view {}...", i), file!(), line!());
            self.device.destroy_image_view(*view, None);
        }
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "recreate_swapchain: old swapchain resources destroyed, creating new swapchain...", file!(), line!());
        
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
        // Use extent dimensions directly for UI layout.
        // On Windows with GLFW, extent matches the window size (logical pixels on high DPI).
        // The widget tree coordinates use the same pixel space as extent.
        let width = self.extent.width as f32;
        let height = self.extent.height as f32;
        
        // First: set root widget layout to extent size
        {
            let ui = self.ui_system.lock();
            let tree = ui.get_widget_tree();
            let mut tree_guard = tree.lock();
            
            if let Some(root_id) = tree_guard.root {
                if let Some(root_widget) = tree_guard.get_widget_mut(root_id) {
                    root_widget.set_layout(hezhou_ui::Layout::new(
                        0.0, 0.0, 
                        width,
                        height
                    ));
                }
            }
            drop(tree_guard);
            drop(ui);
        }
        
        // Second: update global screen size so C# GetScreenSize is consistent
        hezhou_ui::thunk::ui_set_screen_size(width, height);
        
        // Third: rasterize all text + run full layout pass BEFORE triggering C# OnResize.
        // This ensures that when C# calls WidgetGetLayout() in UpdatePreviewExtent(),
        // the layout values are already computed for the current frame (not stale
        // from the previous frame, which caused the delayed/inverted resize bug),
        // and that all glyphs (including emoji/CJK fallback) are cached before measurement.
        {
            let ui = self.ui_system.lock();
            let tree = ui.get_widget_tree();
            let mut tree_guard = tree.lock();
            let font_atlas_mutex = ui.get_font_atlas();
            
            // Step 1: Ensure all text is rasterized with font fallback before layout
            {
                let mut font_atlas_guard = font_atlas_mutex.lock();
                tree_guard.ensure_text_rasterized(&mut *font_atlas_guard);
                
                if font_atlas_guard.is_atlas_dirty() {
                    let texture_data = font_atlas_guard.get_atlas_texture().to_vec();
                    font_atlas_guard.clear_atlas_dirty();
                    
                    unsafe {
                        let mem_requirements = self.device.get_image_memory_requirements(self.font_texture);
                        let data_ptr = self.device.map_memory(
                            self.font_texture_memory,
                            0,
                            mem_requirements.size,
                            vk::MemoryMapFlags::empty(),
                        ).expect("Failed to map font texture memory in resize");
                        
                        std::ptr::copy_nonoverlapping(texture_data.as_ptr(), data_ptr as *mut u8, texture_data.len());
                        self.device.unmap_memory(self.font_texture_memory);
                    }
                }
            }
            
            // Step 2: Now all glyphs cached, do layout
            let font_atlas_guard = font_atlas_mutex.lock();
            if let Some(root_id) = tree_guard.root {
                tree_guard.perform_layout(&*font_atlas_guard);
            }
            drop(tree_guard);
            drop(font_atlas_guard);
            drop(ui);
        }
        
        // Fourth: trigger C# OnResize — now C# reads correct (post-layout) dimensions
        hezhou_ui::ffi::ui_trigger_resize(width, height);
        
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Info,
            "UI",
            &format!("UI layout updated: extent={}x{}, scale={}", 
                self.extent.width, self.extent.height, self.content_scale),
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
            
            // Standard Vulkan synchronization order:
            // 1. wait_for_fences — ensures previous frame's submit completed
            //    (which consumed the semaphore from the previous acquire)
            // 2. reset_fences — put fence back to unsignaled state
            // 3. acquire_next_image — signal semaphore for this frame
            // This prevents VUID-01779 (semaphore with pending operations)
self.device.wait_for_fences(&[self.in_flight_fences[self.current_frame]], true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fence: {}", e))?;
            
            // NOTE: flush_deferred_destroy_periodic不再在draw_frame中调用
            // 实验证实：即使fence wait + 帧阈值保证GPU不再引用旧资源，
            // 运行时销毁Vulkan资源仍触发驱动内部HEAP_CORRUPTION。
            // 这是Vulkan驱动层面的问题，不是我们的同步逻辑错误。
            // 旧资源只累积在延迟列表中，在cleanup()时一次性销毁（device_wait_idle后安全）。
            // GPU资源暂时累积但resize不频繁，不会无限增长。
            
            self.device.reset_fences(&[self.in_flight_fences[self.current_frame]])
                .map_err(|e| format!("Failed to reset fence: {}", e))?;
            
            // Apply deferred offscreen FBO resize if pending.
            // Must wait for OTHER in-flight frames (not current) before destroying
            // old FBO resources, because other frames' command buffers may still
            // reference the old offscreen framebuffer/image.
            // CRITICAL: Do NOT wait_for_fences(ALL) because fence[current_frame]
            // was just reset to unsignaled — including it would cause infinite wait (deadlock).
            // Do NOT reset other frames' fences — leave them signaled so next frame's
            // wait_for_fences succeeds immediately.
            if self.pending_offscreen_resize.is_some() {
                for (i, fence) in self.in_flight_fences.iter().enumerate() {
                    if i != self.current_frame {
                        self.device.wait_for_fences(&[*fence], true, u64::MAX)
                            .map_err(|e| format!("Failed to wait for fence {} for resize: {}", i, e))?;
                    }
                }
                self.apply_pending_offscreen_resize()?;
            }
            
            self.dfx.lock().get_logger().lock().log(LogLevel::Trace, "Vulkan", "acquiring next image", file!(), line!());
            let (image_index, _suboptimal) = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null()
            ).map_err(|e| format!("Failed to acquire image: {}", e))?;
            
            let image_index_usize = image_index as usize;
            
            self.device.reset_command_buffer(self.command_buffers[image_index_usize], vk::CommandBufferResetFlags::RELEASE_RESOURCES)
                .map_err(|e| format!("Failed to reset command buffer: {}", e))?;
            
            self.device.begin_command_buffer(self.command_buffers[image_index_usize], &vk::CommandBufferBeginInfo::default())
                .map_err(|e| format!("Failed to begin command buffer: {}", e))?;
            
            // === Game Pass: Render cube to offscreen ===
            if self.game_state == 1 {  // Running
                self.entity_angle += 90.0 * delta_time / 1000.0; // 90度/秒, delta_time is milliseconds
                if self.entity_angle > 360.0 {
                    self.entity_angle -= 360.0;
                }
            }
            
            // Transition offscreen image to COLOR_ATTACHMENT_OPTIMAL
            // After the first frame, offscreen ends in SHADER_READ_ONLY_OPTIMAL,
            // so subsequent frames must transition FROM SHADER_READ_ONLY (not UNDEFINED)
            let offscreen_old_layout = if self.first_frame {
                vk::ImageLayout::UNDEFINED
            } else {
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            };
            let game_barrier_begin = vk::ImageMemoryBarrier {
                old_layout: offscreen_old_layout,
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
                src_access_mask: if self.first_frame { vk::AccessFlags::empty() } else { vk::AccessFlags::SHADER_READ },
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                ..Default::default()
            };
            let offscreen_src_stage = if self.first_frame {
                vk::PipelineStageFlags::TOP_OF_PIPE
            } else {
                vk::PipelineStageFlags::FRAGMENT_SHADER
            };
            self.device.cmd_pipeline_barrier(
                self.command_buffers[image_index_usize],
                offscreen_src_stage,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[game_barrier_begin]
            );
            
            // === Game Rendering: 双路径（pipeline / legacy）===
            // 实体角度更新和offscreen布局转换在条件分支外部（两条路径都需要）
            
            if let Some(ref mut registry) = self.pipeline_registry {
                // 仅在dynamic data变更时重建管线同步HashMap（dirty flag机制）
                // 避免每帧无意义的HashMap克隆开销
                if self.dirty_dynamic_data {
                    let pipeline_primitive_ranges: std::collections::HashMap<MeshType, hezhou_render_pipeline::PrimitiveMeshRange> =
                        self.primitive_ranges.iter().map(|(k, v)| {
                            (*k, hezhou_render_pipeline::PrimitiveMeshRange {
                                vertex_offset: v.vertex_offset,
                                vertex_count: v.vertex_count,
                                index_offset: 0,
                                index_count: 0,
                            })
                        }).collect();
                    let pipeline_custom_meshes: std::collections::HashMap<String, hezhou_render_pipeline::CustomMeshInfo> =
                        self.custom_meshes.iter().map(|(k, v)| {
                            (k.clone(), hezhou_render_pipeline::CustomMeshInfo {
                                vertex_offset: v.vertex_offset,
                                vertex_count: v.vertex_count,
                                index_offset: v.index_offset,
                                index_count: v.index_count,
                                has_texture: v.has_texture,
                                texture_descriptor_set: v.texture_descriptor_set,
                                specular_strength: v.specular_strength,
                                ambient_strength: v.ambient_strength,
                                shininess: v.shininess,
                            })
                        }).collect();
                    let pipeline_texture_cache: std::collections::HashMap<String, hezhou_render_pipeline::TextureCacheEntry> =
                        self.texture_cache.iter().map(|(k, v)| {
                            (k.clone(), hezhou_render_pipeline::TextureCacheEntry {
                                image: v.image,
                                memory: v.memory,
                                view: v.view,
                                sampler: v.sampler,
                                descriptor_set: v.descriptor_set,
                            })
                        }).collect();
                    registry.update_raster_dynamic_data(
                        pipeline_primitive_ranges,
                        pipeline_custom_meshes,
                        pipeline_texture_cache,
                    );
                    self.dirty_dynamic_data = false;
                }

                // Pipeline路径：委托给PipelineRegistry
                // 构建RenderContext — 每帧渲染参数
                let game_state_enum = match self.game_state {
                    0 => hezhou_core::ecs::GameState::Editing,
                    1 => hezhou_core::ecs::GameState::Running,
                    _ => hezhou_core::ecs::GameState::Paused,  // 2=Paused也映射到Paused
                };
                let render_ctx = hezhou_render_pipeline::RenderContext {
                    scene: self.scene_ptr.unwrap_or(std::ptr::null_mut()),
                    mode: hezhou_render_pipeline::RenderMode::from_game_state(game_state_enum),
                    camera: hezhou_render_pipeline::CameraParams {
                        position: [self.camera_x, self.camera_y, self.camera_z],
                        yaw: self.camera_yaw,
                        pitch: self.camera_pitch,
                        fov: 45.0,  // 默认FOV
                        near: 0.1,
                        far: 100.0,
                    },
                    viewport_extent: hezhou_render_pipeline::ViewportExtent::new(
                        self.offscreen_extent.width,
                        self.offscreen_extent.height,
                    ),
                    selected_entity_ids: if let Some(scene_ptr) = self.scene_ptr {
                        unsafe { (*scene_ptr).selected_entities.iter().map(|e| e.id).collect() }
                    } else {
                        Vec::new()
                    },
                    frame_index: self.frame_count as u32,  // 用frame_count而非current_frame（不循环）
                };
                
                // 委托给PipelineRegistry执行完整渲染帧
                // prepare → record → post_process → composite
                let render_result = registry.render_frame(&render_ctx, self.command_buffers[image_index_usize]);
                // 更新preview_descriptor_set和截图源图像
                if let Some((ds, output)) = render_result {
                    self.preview_descriptor_set = ds;
                    // 截图源：记录当前帧实际渲染结果的image和extent
                    // rasterization: offscreen_image (composite后仍是offscreen)
                    // ray_tracing: output_image (compute shader输出)
                    self.screenshot_source_image = output.color_image;
                    self.screenshot_source_extent = output.extent;
                }
            } else {
                // Legacy路径：保持现有游戏渲染代码不变
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
                    clear_value_count: 2,
                    p_clear_values: &[vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.05, 0.05, 0.1, 1.0],
                        },
                    }, vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    }] as *const _,
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
            
            // Bind game mesh vertex buffer
            self.device.cmd_bind_vertex_buffers(
                self.command_buffers[image_index_usize],
                0,
                &[self.game_mesh_buffer],
                &[0],
            );
            
            // Extract directional light data from Scene ECS and upload to UBO
            // std140 layout: [dir.x, dir.y, dir.z, 0(pad), color.x, color.y, color.z, 0(pad), intensity, 0(pad), 0(pad), 0(pad)] = 12 floats = 48 bytes
            let light_data: [f32; 8] = if let Some(scene_ptr) = self.scene_ptr {
                unsafe {
                    let scene = &*scene_ptr;
                    let mut found_light = None;
                    for entity in &scene.root_entities {
                        if let Some(light_comp) = scene.world.get_component::<hezhou_core::DirectionalLightComponent>(*entity) {
                            found_light = Some(light_comp);
                            break;
                        }
                    }
                    if let Some(light_comp) = found_light {
                        // std430 layout: [dir.x, dir.y, dir.z, pad, color.x, color.y, color.z, intensity] = 8 floats
                        // intensity at offset 28 (NOT 32 as std140 would place it)
                        [light_comp.direction.x, light_comp.direction.y, light_comp.direction.z, 0.0,
                         light_comp.color.x, light_comp.color.y, light_comp.color.z, light_comp.intensity]
                    } else {
                        // Default fallback: direction=(-0.5,-1.0,-0.5), color=(1,1,1), intensity=1.0
                        [-0.5, -1.0, -0.5, 0.0, 1.0, 1.0, 1.0, 1.0]
                    }
                }
            } else {
                // Default fallback when no scene exists
                [-0.5, -1.0, -0.5, 0.0, 1.0, 1.0, 1.0, 1.0]
            };
            
            // Upload light data to UBO (32 bytes, std430 layout)
            unsafe {
                let data_ptr = self.device.map_memory(self.light_ubo_memory, 0, 32, vk::MemoryMapFlags::empty())
                    .expect("Failed to map light UBO memory");
                std::ptr::copy_nonoverlapping(
                    light_data.as_ptr() as *const u8,
                    data_ptr as *mut u8,
                    32,
                );
                self.device.unmap_memory(self.light_ubo_memory);
            }
            
            // Bind light descriptor set (set 0) and texture descriptor set (set 1) for game pipeline
            self.device.cmd_bind_descriptor_sets(
                self.command_buffers[image_index_usize],
                vk::PipelineBindPoint::GRAPHICS,
                self.game_pipeline_layout,
                0,
                &[self.light_descriptor_set, self.default_texture_descriptor_set],
                &[],
            );
            
            // Multi-entity rendering loop
            if let Some(scene_ptr) = self.scene_ptr {
                unsafe {
                    let scene = &*scene_ptr;
                    for entity in &scene.root_entities {
                        // Get RenderableComponent
                        let renderable_opt = scene.world.get_component::<hezhou_core::RenderableComponent>(*entity);
                        if renderable_opt.is_none() {
                            continue;
                        }
                        let renderable = renderable_opt.unwrap();
                        if !renderable.visible { continue; }
                        
                        // Get LocalTransform
                        let transform = scene.world.get_component::<hezhou_core::LocalTransform>(*entity);
                        if transform.is_none() { continue; }
                        let transform = transform.unwrap();
                        
                        // Parse mesh_path → 判断是自定义mesh还是primitive mesh
                        if crate::mesh_loader::is_custom_mesh(&renderable.mesh_path) {
                            // 自定义mesh渲染（使用index buffer + cmd_draw_indexed）
                            if let Some(custom_info) = self.custom_meshes.get(&renderable.mesh_path) {
                                // Compute model matrix from transform
                                let model = Self::compute_model_matrix(&transform);
                                
                                // Is this entity selected?
                                let is_selected = scene.selected_entities.contains(entity);
                                let outline_color = if is_selected { [1.0f32, 0.5, 0.0] } else { [0.0f32, 0.0, 0.0] };
                                let is_selected_f = if is_selected { 1.0f32 } else { 0.0f32 };
                                // 自定义mesh的纹理：优先使用texture_path（per-entity），其次CustomMeshInfo
                                let entity_texture_ds = if let Some(ref texture_path) = renderable.texture_path {
                                    self.texture_cache.get(texture_path).map(|e| e.descriptor_set)
                                } else if let Some(tex_set) = custom_info.texture_descriptor_set {
                                    Some(tex_set)
                                } else {
                                    None
                                };
                                let has_texture_f = if entity_texture_ds.is_some() { 1.0f32 } else { 0.0f32 };
                                
                                let push_data = [
                                    // model matrix columns (column-major)
                                    model[0][0], model[0][1], model[0][2], model[0][3],
                                    model[1][0], model[1][1], model[1][2], model[1][3],
                                    model[2][0], model[2][1], model[2][2], model[2][3],
                                    model[3][0], model[3][1], model[3][2], model[3][3],
                                    // outline_color + is_selected (16 bytes at offset 64)
                                    outline_color[0], outline_color[1], outline_color[2], is_selected_f,
                                    // viewport_size (8 bytes at offset 80) + padding (8 bytes at offset 88)
                                    self.offscreen_extent.width as f32, self.offscreen_extent.height as f32,
                                    0.0f32, 0.0f32,  // pad to align camera_pos to 16-byte boundary (offset 96)
                                    // camera_pos (12 bytes at offset 96) + camera_yaw (4 bytes at offset 108) + camera_pitch (4 bytes at offset 112)
                                    self.camera_x, self.camera_y, self.camera_z, self.camera_yaw,
self.camera_pitch,
                                     0.0f32,  // _pad2 at offset 116
                                     // has_texture(4) + specular_strength(4) + ambient_strength(4) + shininess(4) at offsets 120-132 → 136 bytes
                                     has_texture_f, custom_info.specular_strength, custom_info.ambient_strength, custom_info.shininess,
                                     0.0f32, 0.0f32,  // struct end padding to 144 bytes
                                 ];
                                
                                // 绑定game pipeline
                                self.device.cmd_bind_pipeline(
                                    self.command_buffers[image_index_usize],
                                    vk::PipelineBindPoint::GRAPHICS,
                                    self.game_pipeline
                                );
                                
                                // 绑定纹理descriptor set（per-entity纹理优先）
                                if let Some(tex_ds) = entity_texture_ds {
                                    self.device.cmd_bind_descriptor_sets(
                                        self.command_buffers[image_index_usize],
                                        vk::PipelineBindPoint::GRAPHICS,
                                        self.game_pipeline_layout,
                                        0,
                                        &[self.light_descriptor_set, tex_ds],
                                        &[],
                                    );
                                }
                                
                                self.device.cmd_push_constants(
                                    self.command_buffers[image_index_usize],
                                    self.game_pipeline_layout,
                                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                    0,
                                    bytemuck::cast_slice(&push_data)
                                );
                                
                                // 绑定index buffer
                                self.device.cmd_bind_index_buffer(
                                    self.command_buffers[image_index_usize],
                                    self.game_index_buffer,
                                    0,
                                    vk::IndexType::UINT32,
                                );
                                
                                // 用index buffer绘制
                                self.device.cmd_draw_indexed(
                                    self.command_buffers[image_index_usize],
                                    custom_info.index_count,
                                    1,
                                    custom_info.index_offset,
                                    custom_info.vertex_offset as i32,
                                    0,
                                );
                                
                                // 如果选中，绘制outline（不用index buffer，用cmd_draw）
                                if is_selected {
                                    let outline_model = Self::compute_scaled_model_matrix(&transform, 1.05);
                                    let outline_push_data = [
                                        // outline model matrix columns (column-major)
                                        outline_model[0][0], outline_model[0][1], outline_model[0][2], outline_model[0][3],
                                        outline_model[1][0], outline_model[1][1], outline_model[1][2], outline_model[1][3],
                                        outline_model[2][0], outline_model[2][1], outline_model[2][2], outline_model[2][3],
                                        outline_model[3][0], outline_model[3][1], outline_model[3][2], outline_model[3][3],
                                        // outline_color + is_selected (1.0 for outline) at offset 64
                                        1.0f32, 0.5, 0.0, 1.0f32,
                                        // viewport_size (8 bytes) + padding (8 bytes) at offset 80
                                        self.offscreen_extent.width as f32, self.offscreen_extent.height as f32,
                                        0.0f32, 0.0f32,  // pad to align camera_pos to 16-byte boundary
                                        // camera_pos + camera_yaw at offset 96
                                        self.camera_x, self.camera_y, self.camera_z, self.camera_yaw,
                                        self.camera_pitch,
                                        0.0f32,  // _pad2 at offset 116
                                        // has_texture=0 + specular_strength(0.3) + ambient_strength(0.15) + shininess(32.0) at offsets 120-132
                                        0.0f32, 0.3f32, 0.15f32, 32.0f32,
                                        0.0f32, 0.0f32,  // struct end padding to 144 bytes
                                    ];
                                    
                                    self.device.cmd_bind_pipeline(
                                        self.command_buffers[image_index_usize],
                                        vk::PipelineBindPoint::GRAPHICS,
                                        self.outline_pipeline
                                    );
                                    
                                    self.device.cmd_push_constants(
                                        self.command_buffers[image_index_usize],
                                        self.game_pipeline_layout,
                                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                        0,
                                        bytemuck::cast_slice(&outline_push_data)
                                    );
                                    
                                    // outline用cmd_draw_indexed（与主mesh相同方式）
                                    self.device.cmd_bind_index_buffer(
                                        self.command_buffers[image_index_usize],
                                        self.game_index_buffer,
                                        0,
                                        vk::IndexType::UINT32,
                                    );
                                    
                                    self.device.cmd_draw_indexed(
                                        self.command_buffers[image_index_usize],
                                        custom_info.index_count,
                                        1,
                                        custom_info.index_offset,
                                        custom_info.vertex_offset as i32,
                                        0,
                                    );
                                }
                            }
                        } else {
                            // Primitive mesh渲染（使用cmd_draw，无index buffer）
                            let mesh_type = primitive_meshes::mesh_type_from_path(&renderable.mesh_path);
                            let range = self.primitive_ranges.get(&mesh_type);
                            if range.is_none() {
                                continue;
                            }
                            let range = range.unwrap();
                            
                            // Compute model matrix from transform
                            let model = Self::compute_model_matrix(&transform);
                            
                            // Is this entity selected?
                            let is_selected = scene.selected_entities.contains(entity);
                            
                            // Push constants: model(64) + outline_color(12) + is_selected(4) + viewport_size(8) + _pad(8) + camera_pos(12) + camera_yaw(4) + camera_pitch(4) + _pad(4) + has_texture(4) + specular_strength(4) + ambient_strength(4) + shininess(4) = 144 bytes
                            let outline_color = if is_selected { [1.0f32, 0.5, 0.0] } else { [0.0f32, 0.0, 0.0] };
                            let is_selected_f = if is_selected { 1.0f32 } else { 0.0f32 };
                            // 检查实体是否有自己的纹理
                            let entity_texture_ds = if let Some(ref texture_path) = renderable.texture_path {
                                self.texture_cache.get(texture_path).map(|e| e.descriptor_set)
                            } else {
                                None
                            };
                            let has_texture_f = if entity_texture_ds.is_some() { 1.0f32 } else { 0.0f32 };
                            
                            // Model matrix stored as column-major (Vulkan convention): 16 floats
                            let push_data = [
                                // model matrix columns (column-major)
                                model[0][0], model[0][1], model[0][2], model[0][3],
                                model[1][0], model[1][1], model[1][2], model[1][3],
                                model[2][0], model[2][1], model[2][2], model[2][3],
                                model[3][0], model[3][1], model[3][2], model[3][3],
                                // outline_color + is_selected (16 bytes at offset 64)
                                outline_color[0], outline_color[1], outline_color[2], is_selected_f,
                                // viewport_size (8 bytes at offset 80) + padding (8 bytes at offset 88)
                                self.offscreen_extent.width as f32, self.offscreen_extent.height as f32,
                                0.0f32, 0.0f32,  // pad to align camera_pos to 16-byte boundary (offset 96)
                                // camera_pos (12 bytes at offset 96) + camera_yaw (4 bytes at offset 108) + camera_pitch (4 bytes at offset 112)
                                self.camera_x, self.camera_y, self.camera_z, self.camera_yaw,
                                self.camera_pitch,
                                0.0f32,  // _pad2 at offset 116
                                // has_texture(4) + specular_strength(4) + ambient_strength(4) + shininess(4) at offsets 120-132
                                has_texture_f, 0.3f32, 0.15f32, 32.0f32,
                                0.0f32, 0.0f32,  // struct end padding to 144 bytes
                            ];
                            
                            // Draw normal entity
                            self.device.cmd_bind_pipeline(
                                self.command_buffers[image_index_usize],
                                vk::PipelineBindPoint::GRAPHICS,
                                self.game_pipeline
                            );
                            
                            // 绑定per-entity纹理descriptor set（如果有纹理则替换default）
                            if let Some(tex_ds) = entity_texture_ds {
                                self.device.cmd_bind_descriptor_sets(
                                    self.command_buffers[image_index_usize],
                                    vk::PipelineBindPoint::GRAPHICS,
                                    self.game_pipeline_layout,
                                    0,
                                    &[self.light_descriptor_set, tex_ds],
                                    &[],
                                );
                            }
                            
                            self.device.cmd_push_constants(
                                self.command_buffers[image_index_usize],
                                self.game_pipeline_layout,
                                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                0,
                                bytemuck::cast_slice(&push_data)
                            );
                            
                            self.device.cmd_draw(
                                self.command_buffers[image_index_usize],
                                range.vertex_count, 1, range.vertex_offset, 0
                            );
                            
                            // If selected, also draw outline (slightly scaled model matrix)
                            if is_selected {
                                let outline_model = Self::compute_scaled_model_matrix(&transform, 1.05);
                                let outline_push_data = [
                                    // outline model matrix columns (column-major)
                                    outline_model[0][0], outline_model[0][1], outline_model[0][2], outline_model[0][3],
                                    outline_model[1][0], outline_model[1][1], outline_model[1][2], outline_model[1][3],
                                    outline_model[2][0], outline_model[2][1], outline_model[2][2], outline_model[2][3],
                                    outline_model[3][0], outline_model[3][1], outline_model[3][2], outline_model[3][3],
                                    // outline_color + is_selected (1.0 for outline) at offset 64
                                    1.0f32, 0.5, 0.0, 1.0f32,
                                    // viewport_size (8 bytes) + padding (8 bytes) at offset 80
                                    self.offscreen_extent.width as f32, self.offscreen_extent.height as f32,
                                    0.0f32, 0.0f32,  // pad to align camera_pos to 16-byte boundary
                                    // camera_pos + camera_yaw at offset 96
                                    self.camera_x, self.camera_y, self.camera_z, self.camera_yaw,
                                    self.camera_pitch,
                                    0.0f32,  // _pad2 at offset 116
                                    // has_texture=0 + specular_strength(0.3) + ambient_strength(0.15) + shininess(32.0) at offsets 120-132
                                    0.0f32, 0.3f32, 0.15f32, 32.0f32,
                                    0.0f32, 0.0f32,  // struct end padding to 144 bytes
                                ];
                                
                                self.device.cmd_bind_pipeline(
                                    self.command_buffers[image_index_usize],
                                    vk::PipelineBindPoint::GRAPHICS,
                                    self.outline_pipeline
                                );
                                
                                self.device.cmd_push_constants(
                                    self.command_buffers[image_index_usize],
                                    self.game_pipeline_layout,
                                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                    0,
                                    bytemuck::cast_slice(&outline_push_data)
                                );
                                
                                self.device.cmd_draw(
                                    self.command_buffers[image_index_usize],
                                    range.vertex_count, 1, range.vertex_offset, 0
                                );
                            }
                        }
                    }
                }
            } else {
                // Fallback: draw single cube if no scene is set (legacy behavior)
                let model = hezhou_core::Mat4::translate(hezhou_core::Vec3::new(
                    self.entity_position[0], self.entity_position[1], self.entity_position[2]
                )) * hezhou_core::Mat4::from_quaternion(hezhou_core::Quaternion::from_axis_angle(
                    hezhou_core::Vec3::up(), self.entity_angle.to_radians()
                )) * hezhou_core::Mat4::scale(hezhou_core::Vec3::new(
                    self.entity_scale[0], self.entity_scale[1], self.entity_scale[2]
                ));
                
                let push_data = [
                    model.data[0][0], model.data[0][1], model.data[0][2], model.data[0][3],
                    model.data[1][0], model.data[1][1], model.data[1][2], model.data[1][3],
                    model.data[2][0], model.data[2][1], model.data[2][2], model.data[2][3],
                    model.data[3][0], model.data[3][1], model.data[3][2], model.data[3][3],
                    0.0f32, 0.0, 0.0, 0.0f32,  // outline_color + is_selected (unused in fallback)
                    self.offscreen_extent.width as f32, self.offscreen_extent.height as f32,
                    0.0f32, 0.0f32,  // padding to align camera_pos
                    self.camera_x, self.camera_y, self.camera_z, self.camera_yaw,
                    self.camera_pitch,
                    0.0f32,  // _pad2 at offset 116
                    // has_texture=0 + specular_strength(0.3) + ambient_strength(0.15) + shininess(32.0) at offsets 120-132
                    0.0f32, 0.3f32, 0.15f32, 32.0f32,  
                    0.0f32, 0.0f32,  // struct end padding to 144 bytes
                ];
                
                self.device.cmd_bind_pipeline(
                    self.command_buffers[image_index_usize],
                    vk::PipelineBindPoint::GRAPHICS,
                    self.game_pipeline
                );
                
                self.device.cmd_push_constants(
                    self.command_buffers[image_index_usize],
                    self.game_pipeline_layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    bytemuck::cast_slice(&push_data)
                );
                
                // Draw cube (36 vertices at offset 0)
                if let Some(range) = self.primitive_ranges.get(&MeshType::Cube) {
                    self.device.cmd_draw(
                        self.command_buffers[image_index_usize],
                        range.vertex_count, 1, range.vertex_offset, 0
                    );
                }
            }
            
            // End game render pass
            self.device.cmd_end_render_pass(self.command_buffers[image_index_usize]);
            
            // offscreen image layout transition is handled by render pass implicit
            // end transition (finalLayout=SHADER_READ_ONLY_OPTIMAL), synchronized by
            // subpass dependency (subpass 0 → EXTERNAL). No explicit barrier needed.
            
            // === FXAA Pass: Apply FXAA to offscreen image ===
            // Transition offscreen_fxaa to COLOR_ATTACHMENT_OPTIMAL
            let fxaa_old_layout = if self.first_frame {
                vk::ImageLayout::UNDEFINED
            } else {
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            };
            let fxaa_barrier_begin = vk::ImageMemoryBarrier {
                old_layout: fxaa_old_layout,
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
                src_access_mask: if self.first_frame { vk::AccessFlags::empty() } else { vk::AccessFlags::SHADER_READ },
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                ..Default::default()
            };
            let fxaa_src_stage = if self.first_frame {
                    vk::PipelineStageFlags::TOP_OF_PIPE
                } else {
                    vk::PipelineStageFlags::FRAGMENT_SHADER
                };
            self.device.cmd_pipeline_barrier(
                self.command_buffers[image_index_usize],
                fxaa_src_stage,
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
                    clear_value_count: 2,
                    p_clear_values: &[vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    }, vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    }] as *const _,
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
            
// fxaa_image layout transition is handled by render pass implicit
            // end transition (finalLayout=SHADER_READ_ONLY_OPTIMAL), synchronized by
            // subpass dependency (subpass 0 → EXTERNAL). No explicit barrier needed.
            } // end of else (legacy路径)
            
            
            // === Font atlas upload (before UI render pass) ===
            // Move font texture upload OUTSIDE the render pass to avoid layout transitions
            // inside the render pass, which caused the "emoji grid" bug:
            // font texture was temporarily in GENERAL layout during Phase 1 rendering,
            // causing the shader to see raw atlas data (emoji glyphs arranged in grid).
            // Fix: upload BEFORE render pass begins, with proper HOST->SHADER synchronization.
            {
                let ui = self.ui_system.lock();
                let tree = ui.get_widget_tree();
                let mut tree_guard = tree.lock();
                let font_atlas_mutex = ui.get_font_atlas();
                
                // Step 1: Ensure all text is rasterized on-demand (handles CJK etc.)
                {
                    let mut font_atlas_guard = font_atlas_mutex.lock();
                    tree_guard.ensure_text_rasterized(&mut *font_atlas_guard);
                    
                    // If new characters were rasterized, re-upload font texture to GPU
                    if font_atlas_guard.is_atlas_dirty() {
                        let texture_data = font_atlas_guard.get_atlas_texture().to_vec();
                        font_atlas_guard.clear_atlas_dirty();
                        
                        // Upload via map_memory (CPU host write to HOST_COHERENT memory)
                        // Data is immediately visible to GPU after unmap_memory (HOST_COHERENT property).
                        // Pipeline barrier with HOST_WRITE->SHADER_READ ensures GPU sees the data
                        // before any fragment shader reads occur. No layout transition needed --
                        // font texture stays in SHADER_READ_ONLY_OPTIMAL throughout.
                        unsafe {
                            // CPU-side upload: map + copy + unmap (host write)
                            let mem_requirements = self.device.get_image_memory_requirements(self.font_texture);
                            let data_ptr = self.device.map_memory(
                                self.font_texture_memory,
                                0,
                                mem_requirements.size,
                                vk::MemoryMapFlags::empty(),
                            ).expect("Failed to map font texture memory");
                            
                            std::ptr::copy_nonoverlapping(texture_data.as_ptr(), data_ptr as *mut u8, texture_data.len());
                            self.device.unmap_memory(self.font_texture_memory);
                            
                            // GPU-side synchronization: HOST write must be visible before FRAGMENT_SHADER reads
                            // This barrier ensures the GPU sees the CPU-written data before sampling the texture.
                            // No image layout transition -- font texture remains SHADER_READ_ONLY_OPTIMAL.
                            self.device.cmd_pipeline_barrier(
                                self.command_buffers[image_index_usize],
                                vk::PipelineStageFlags::HOST,
                                vk::PipelineStageFlags::FRAGMENT_SHADER,
                                vk::DependencyFlags::empty(),
                                &[vk::MemoryBarrier {
                                    src_access_mask: vk::AccessFlags::HOST_WRITE,
                                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                                    ..Default::default()
                                }],
                                &[] as &[vk::BufferMemoryBarrier],
                                &[] as &[vk::ImageMemoryBarrier],
                            );
                        }
                    }
                }
                
                // Step 2: Now all glyphs are cached, use &FontAtlas for layout/render
                let font_atlas_guard = font_atlas_mutex.lock();
                tree_guard.perform_layout(&*font_atlas_guard);
                
                // Step 2b: After layout, check if PreviewWindow extent changed and
                // queue FBO resize to match.
                {
                    let preview_extent = tree_guard.find_preview_window_extent();
                    if let Some((pw, ph)) = preview_extent {
                        let pw_u = pw as u32;
                        let ph_u = ph as u32;
                        if pw_u != self.offscreen_extent.width || ph_u != self.offscreen_extent.height {
                            if pw_u > 0 && ph_u > 0 {
                                self.pending_offscreen_resize = Some(vk::Extent2D { width: pw_u, height: ph_u });
                            }
                        }
                    }
}
                
                tree_guard.generate_render_data(&*font_atlas_guard); // side effect only: ensure glyphs rasterized before atlas upload
            }
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
            
            let px_range = 4.0;
            // Use extent dimensions for screen_size in push constants.
            // Widget positions and extent share the same coordinate space.
            let screen_width = self.extent.width as f32;
            let screen_height = self.extent.height as f32;
            
            let push_constants = [
                screen_width,
                screen_height,
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
                // Font atlas already uploaded and text already rasterized (done before render pass)
                // Only need layout and render data generation here
                let ui = self.ui_system.lock();
                let tree = ui.get_widget_tree();
                let mut tree_guard = tree.lock();
                let font_atlas_mutex = ui.get_font_atlas();
                let font_atlas_guard = font_atlas_mutex.lock();
                tree_guard.perform_layout(&*font_atlas_guard);
                
                // After layout, check if PreviewWindow extent changed and
                // queue FBO resize to match. This ensures the game preview FBO always
                // tracks the PreviewWindow's actual layout size, even when resized via
                // SplitView drag (where C# callback may read stale dimensions).
                {
                    let preview_extent = tree_guard.find_preview_window_extent();
                    if let Some((pw, ph)) = preview_extent {
                        let pw_u = pw as u32;
                        let ph_u = ph as u32;
                        if pw_u != self.offscreen_extent.width || ph_u != self.offscreen_extent.height {
                            if pw_u > 0 && ph_u > 0 {
                                self.pending_offscreen_resize = Some(vk::Extent2D { width: pw_u, height: ph_u });
                            }
                        }
                    }
                }
                
                tree_guard.generate_render_data(&*font_atlas_guard)
            };
            
            struct RenderBatch {
                vertices: Vec<f32>,
                clip_x: f32,
                clip_y: f32,
                clip_w: f32,
                clip_h: f32,
                layer: u32,
            }
            
            let mut batches: Vec<RenderBatch> = Vec::new();
            let mut current_vertices: Vec<f32> = Vec::new();
            let mut current_clip: Option<(f32, f32, f32, f32)> = None;
            let mut current_layer: u32 = 0;
            
            let mut preview_vertices: Vec<f32> = Vec::new();
            let mut preview_border_vertices: Vec<f32> = Vec::new();
            let mut is_preview_window_context = false;
            
            fn flush_batch(batches: &mut Vec<RenderBatch>, vertices: &mut Vec<f32>, clip: Option<(f32, f32, f32, f32)>, layer: u32) {
                if !vertices.is_empty() {
                    batches.push(RenderBatch {
                        vertices: vertices.clone(),
                        clip_x: clip.map(|c| c.0).unwrap_or(0.0),
                        clip_y: clip.map(|c| c.1).unwrap_or(0.0),
                        clip_w: clip.map(|c| c.2).unwrap_or(1e6),
                        clip_h: clip.map(|c| c.3).unwrap_or(1e6),
                        layer,
                    });
                    vertices.clear();
                }
            }
            
            // 渲染UI控件 - iterate by render_data item to track layer
            for data in render_data.iter() {
                current_layer = data.layer as u32;
                for cmd in &data.draw_commands {
                match cmd {
                    DrawCommand::ClipRect { rect } => {
                        flush_batch(&mut batches, &mut current_vertices, current_clip, current_layer);
                        current_clip = Some((rect.x, rect.y, rect.width, rect.height));
                    }
                    DrawCommand::ClearClip => {
                        flush_batch(&mut batches, &mut current_vertices, current_clip, current_layer);
                        current_clip = None;
                    }
                    DrawCommand::Rect { bounds, width, height, fill_color, stroke_color, stroke_width, .. } => {
                        let x = bounds.x;
                        let y = bounds.y;
                        let w = *width;
                        let h = *height;
                        let r = fill_color.r;
                        let g = fill_color.g;
                        let b = fill_color.b;
                        let a = fill_color.a;
                        
                        // 如果fill_color是透明且是preview window context，跳过填充
                        if !is_preview_window_context || a > 0.0 {
                            current_vertices.extend_from_slice(&[
                                x.round(), y.round(), r, g, b, a, 0.0, 0.0,
                                (x + w).round(), y.round(), r, g, b, a, 0.0, 0.0,
                                x.round(), (y + h).round(), r, g, b, a, 0.0, 0.0,
                                (x + w).round(), y.round(), r, g, b, a, 0.0, 0.0,
                                (x + w).round(), (y + h).round(), r, g, b, a, 0.0, 0.0,
                                x.round(), (y + h).round(), r, g, b, a, 0.0, 0.0,
                            ]);
                        }
                        
                        if let Some(stroke) = stroke_color {
                            if *stroke_width > 0.0 {
                                let sr = stroke.r;
                                let sg = stroke.g;
                                let sb = stroke.b;
                                let sa = stroke.a;
                                let sw = *stroke_width;
                                
                                let target_vertices = if is_preview_window_context {
                                    &mut preview_border_vertices
                                } else {
                                    &mut current_vertices
                                };
                                
                                let xr = x.round();
                                let yr = y.round();
                                let wr = w.round();
                                let hr = h.round();
                                let swr = sw.round();
                                
                                // Top line
                                target_vertices.extend_from_slice(&[
                                    xr, yr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                ]);
                                
                                // Bottom line
                                target_vertices.extend_from_slice(&[
                                    xr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr, yr + hr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr + hr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr, yr + hr, sr, sg, sb, sa, 0.0, 0.0,
                                ]);
                                
                                // Left line
                                target_vertices.extend_from_slice(&[
                                    xr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + swr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + swr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + swr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                ]);
                                
                                // Right line
                                target_vertices.extend_from_slice(&[
                                    xr + wr - swr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr - swr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr + swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                    xr + wr - swr, yr + hr - swr, sr, sg, sb, sa, 0.0, 0.0,
                                ]);
                                
                                // Reset context after border
                                if is_preview_window_context {
                                    is_preview_window_context = false;
                                }
                            }
                        }
                    }
                    DrawCommand::Text { bounds, width, height, font_color, text, font_size, alignment, .. } => {
                        flush_batch(&mut batches, &mut current_vertices, current_clip, current_layer);
                        
                        let text_str = if text.is_empty() {
                            ""
                        } else {
                            std::str::from_utf8(text).unwrap_or("")
                        };
                        
                        let ui_lock = self.ui_system.lock();
                        let font_atlas_guard = ui_lock.get_font_atlas().lock();
                        
                        let glyphs = if alignment.horizontal == hezhou_ui::HorizontalAlignment::Left {
                            let vertical_center = alignment.vertical == hezhou_ui::VerticalAlignment::Center;
                            font_atlas_guard.layout_text_left(
                                0,
                                text_str,
                                *font_size,
                                bounds.x,
                                bounds.y,
                                *height,
                                vertical_center,
                            )
                        } else {
                            font_atlas_guard.layout_text_centered(
                                0,
                                text_str,
                                *font_size,
                                bounds.x,
                                bounds.y,
                                *width,
                                *height,
                            )
                        };

                        for (gx, gy, gw, gh, uv_x, uv_y, uv_w, uv_h, is_color) in glyphs {
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
                            let (r, g, b, a) = if is_color {
                                (1.0, 1.0, 1.0, 1.0)  // Color emoji: white vertex color so atlas RGB passes through shader
                            } else {
                                (font_color.r, font_color.g, font_color.b, font_color.a)
                            };
                            
                            current_vertices.extend_from_slice(&[
                                x, y, r, g, b, a, u0, v0,
                                x + w, y, r, g, b, a, u1, v0,
                                x, y + h, r, g, b, a, u0, v1,
                                x + w, y, r, g, b, a, u1, v0,
                                x + w, y + h, r, g, b, a, u1, v1,
                                x, y + h, r, g, b, a, u0, v1,
                            ]);
                        }
                        
                        flush_batch(&mut batches, &mut current_vertices, current_clip, current_layer);
                    }
                    DrawCommand::Line { start, end, color, width } => {
                        // Render line as a thin quad (2 triangles) oriented along the line direction.
                        // This enables line rendering in TRIANGLE_LIST topology.
                        let dx = end.x - start.x;
                        let dy = end.y - start.y;
                        let len = (dx * dx + dy * dy).sqrt();
                        if len < 0.001 { continue; }
                        // Normalized perpendicular direction for line width
                        let nx = -dy / len * *width / 2.0;
                        let ny = dx / len * *width / 2.0;
                        let r = color.r;
                        let g = color.g;
                        let b = color.b;
                        let a = color.a;
                        // 4 corners of the line quad
                        let x0 = start.x + nx; let y0 = start.y + ny;
                        let x1 = start.x - nx; let y1 = start.y - ny;
                        let x2 = end.x + nx;   let y2 = end.y + ny;
                        let x3 = end.x - nx;   let y3 = end.y - ny;
                        // 2 triangles forming the quad
                        // UV=(0,0) signals shader to use frag_color directly (no texture sampling)
                        // Must match pipeline stride of 8 floats (32 bytes) per vertex
                        let line_vertices: Vec<f32> = vec![
                            x0, y0, r, g, b, a, 0.0, 0.0,
                            x2, y2, r, g, b, a, 0.0, 0.0,
                            x1, y1, r, g, b, a, 0.0, 0.0,
                            x1, y1, r, g, b, a, 0.0, 0.0,
                            x2, y2, r, g, b, a, 0.0, 0.0,
                            x3, y3, r, g, b, a, 0.0, 0.0,
                        ];
                        current_vertices.extend_from_slice(&line_vertices);
                    }
                    DrawCommand::Image { bounds, width, height, texture_id, uv } => {
                        flush_batch(&mut batches, &mut current_vertices, current_clip, current_layer);
                        
                        let x = bounds.x.round();
                        let y = bounds.y.round();
                        let w = width.round();
                        let h = height.round();
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
                        
                        if *texture_id == 1 {
                            preview_vertices.extend_from_slice(&quad_vertices);
                            is_preview_window_context = true;
                        } else {
                            current_vertices.extend_from_slice(&quad_vertices);
                        }
                        
                        flush_batch(&mut batches, &mut current_vertices, current_clip, current_layer);
                    }
                    DrawCommand::Shadow { .. } => {}
                    DrawCommand::SetTransform { .. } => {}
                    DrawCommand::ResetTransform => {}
                    DrawCommand::RectOutline { bounds, width, height, color, stroke_width } => {
                        let x = bounds.x;
                        let y = bounds.y;
                        let w = *width;
                        let h = *height;
                        let r = color.r;
                        let g = color.g;
                        let b = color.b;
                        let a = color.a;
                        let sw = *stroke_width;
                        
                        let line_vertices: Vec<f32> = vec![
                            x, y, r, g, b, a, 0.0, 0.0,
                            x + w, y, r, g, b, a, 0.0, 0.0,
                            x + w, y, r, g, b, a, 0.0, 0.0,
                            x + w, y + h, r, g, b, a, 0.0, 0.0,
                            x + w, y + h, r, g, b, a, 0.0, 0.0,
                            x, y + h, r, g, b, a, 0.0, 0.0,
                            x, y + h, r, g, b, a, 0.0, 0.0,
                            x, y, r, g, b, a, 0.0, 0.0,
                        ];
                        current_vertices.extend_from_slice(&line_vertices);
                    }
                    DrawCommand::Triangle { p1, p2, p3, fill_color } => {
                        let r = fill_color.r;
                        let g = fill_color.g;
                        let b = fill_color.b;
                        let a = fill_color.a;
                        
                        // UV=(0,0) signals shader to use frag_color directly (no texture sampling)
                        // Must match pipeline stride of 8 floats (32 bytes) per vertex
                        let tri_vertices: Vec<f32> = vec![
                            p1.x, p1.y, r, g, b, a, 0.0, 0.0,
                            p2.x, p2.y, r, g, b, a, 0.0, 0.0,
                            p3.x, p3.y, r, g, b, a, 0.0, 0.0,
                        ];
                        current_vertices.extend_from_slice(&tri_vertices);
                    }
                }
                } // inner for cmd loop
                
                // Reset preview window context at widget boundary.
                // When PreviewWindow is NOT selected (no border Rect), is_preview_window_context
                // stays true after DrawCommand::Image, causing subsequent widgets' transparent
                // backgrounds to be skipped and their border strokes to route to preview_border_vertices.
                // This per-widget reset ensures the flag is cleared after each widget's full draw cycle.
                is_preview_window_context = false;
            } // outer for data loop
            
            flush_batch(&mut batches, &mut current_vertices, current_clip, current_layer);
            
            // Split batches into before_preview (layer 0-1) and after_preview (layer 2-3)
            let before_preview_batches: Vec<&RenderBatch> = batches.iter().filter(|b| b.layer <= 1).collect();
            let after_preview_batches: Vec<&RenderBatch> = batches.iter().filter(|b| b.layer >= 2).collect();
            
            let mut all_vertices: Vec<f32> = Vec::new();
            let mut before_preview_ranges: Vec<(usize, usize, f32, f32, f32, f32)> = Vec::new();
            let mut current_offset = 0;
            
            for batch in &before_preview_batches {
                let start = current_offset;
                let end = current_offset + batch.vertices.len();
                before_preview_ranges.push((start, end, batch.clip_x, batch.clip_y, batch.clip_w, batch.clip_h));
                all_vertices.extend_from_slice(&batch.vertices);
                current_offset = end;
            }
            
            // Preview vertices go between before_preview and after_preview
            let preview_vertex_offset = current_offset;
            all_vertices.extend_from_slice(&preview_vertices);
            current_offset += preview_vertices.len();
            
            // Preview border vertices go after preview
            let preview_border_vertex_offset = current_offset;
            all_vertices.extend_from_slice(&preview_border_vertices);
            current_offset += preview_border_vertices.len();
            
            let mut after_preview_ranges: Vec<(usize, usize, f32, f32, f32, f32)> = Vec::new();
            
            for batch in &after_preview_batches {
                let start = current_offset;
                let end = current_offset + batch.vertices.len();
                after_preview_ranges.push((start, end, batch.clip_x, batch.clip_y, batch.clip_w, batch.clip_h));
                all_vertices.extend_from_slice(&batch.vertices);
                current_offset = end;
            }
            
            let vertex_data: &[u8] = bytemuck::cast_slice(&all_vertices);
            
            let vertex_ptr = self.device.map_memory(
                self.vertex_buffer_memories[self.current_frame],
                0,
                vertex_data.len() as vk::DeviceSize,
                vk::MemoryMapFlags::empty()
            ).map_err(|e| format!("Failed to map memory: {}", e))?;
            
            std::ptr::copy_nonoverlapping(vertex_data.as_ptr(), vertex_ptr as *mut u8, vertex_data.len());
            self.device.unmap_memory(self.vertex_buffer_memories[self.current_frame]);
            
            self.device.cmd_bind_vertex_buffers(
                self.command_buffers[image_index_usize],
                0,
                &[self.vertex_buffers[self.current_frame]],
                &[0]
            );
            
            // Phase 1: Draw before_preview batches (layer 0-1: Background + Content)
            for (start, end, clip_x, clip_y, clip_w, clip_h) in &before_preview_ranges {
                if *clip_w < 1e5 {
                    let scissor = vk::Rect2D {
                        offset: vk::Offset2D { 
                            x: clip_x.round() as i32, 
                            y: clip_y.round() as i32 
                        },
                        extent: vk::Extent2D { 
                            width: clip_w.round() as u32, 
                            height: clip_h.round() as u32 
                        },
                    };
                    self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[scissor]);
                } else {
                    let scissor = vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.extent,
                    };
                    self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[scissor]);
                }
                
                let vertex_count = ((end - start) / 8) as u32;
                let first_vertex = (start / 8) as u32;
                
                if vertex_count > 0 {
                    self.device.cmd_draw(
                        self.command_buffers[image_index_usize],
                        vertex_count,
                        1,
                        first_vertex,
                        0
                    );
                }
            }
            
            // Phase 2: Render preview texture quads (between Content and Popup layers)
            if !preview_vertices.is_empty() {
                let preview_offset = (preview_vertex_offset * 4) as u64;
                
                // Bind preview descriptor set
                self.device.cmd_bind_descriptor_sets(
                    self.command_buffers[image_index_usize],
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &[self.preview_descriptor_set],
                    &[]
                );
                
                // Set full-screen scissor to ensure preview quad covers entire preview window area.
                // Without this, Phase 2 inherits Phase 1's last scissor, which might be
                // a restrictive clip rect (e.g., tree view area 0-250px), causing the
                // preview quad to be clipped and leaving the preview window area uncovered.
                // This caused the "emoji grid" flash bug: Phase 1 emoji text showed through
                // the uncovered preview window area during drag.
                let full_scissor = vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.extent,
                };
                self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[full_scissor]);
                
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
                    &[self.vertex_buffers[self.current_frame]],
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
            
            // Phase 2b: Draw preview border vertices (after preview texture)
            if !preview_border_vertices.is_empty() {
                // Ensure full-screen scissor for preview border (same fix as Phase 2)
                let full_scissor_border = vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.extent,
                };
                self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[full_scissor_border]);
                
                let border_offset = (preview_border_vertex_offset * 4) as u64;
                
                self.device.cmd_bind_vertex_buffers(
                    self.command_buffers[image_index_usize],
                    0,
                    &[self.vertex_buffers[self.current_frame]],
                    &[border_offset]
                );
                
                self.device.cmd_draw(
                    self.command_buffers[image_index_usize],
                    (preview_border_vertices.len() / 8) as u32,
                    1,
                    0,
                    0
                );
            }

            // Restore push constants for Phase 3 (after_preview batches need MSDF text rendering)
            // Phase 2 set push_constants to [w, h, 0, 0, 0, 0] (px_range=0 for RGB texture mode).
            // Phase 3 inherits this, causing MSDF text to render with px_range=0 → text collapses.
            // Restore to Phase 1 values: [screen_w, screen_h, 0, 0, px_range=4.0, 0]
            let restore_push_constants = [
                screen_width,
                screen_height,
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
                bytemuck::cast_slice(&restore_push_constants)
            );
            
            // Phase 3: Draw after_preview batches (layer 2-3: Popup + Overlay)
            self.device.cmd_bind_vertex_buffers(
                self.command_buffers[image_index_usize],
                0,
                &[self.vertex_buffers[self.current_frame]],
                &[0]
            );
            
            for (start, end, clip_x, clip_y, clip_w, clip_h) in &after_preview_ranges {
                if *clip_w < 1e5 {
                    let scissor = vk::Rect2D {
                        offset: vk::Offset2D { 
                            x: clip_x.round() as i32, 
                            y: clip_y.round() as i32 
                        },
                        extent: vk::Extent2D { 
                            width: clip_w.round() as u32, 
                            height: clip_h.round() as u32 
                        },
                    };
                    self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[scissor]);
                } else {
                    let scissor = vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: self.extent,
                    };
                    self.device.cmd_set_scissor(self.command_buffers[image_index_usize], 0, &[scissor]);
                }
                
                let vertex_count = ((end - start) / 8) as u32;
                let first_vertex = (start / 8) as u32;
                
                if vertex_count > 0 {
                    self.device.cmd_draw(
                        self.command_buffers[image_index_usize],
                        vertex_count,
                        1,
                        first_vertex,
                        0
                    );
                }
            }
            
            if self.frame_count == 0 {
                self.dfx.lock().get_logger().lock().log(LogLevel::Trace, "Render", &format!("Frame {}: {} vertices + {} preview vertices", self.frame_count, all_vertices.len() / 8, preview_vertices.len() / 8), file!(), line!());
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
            self.frame_counter += 1;
            self.frame_count += 1;
            self.first_frame = false;
            
            Ok(true)
        }
    }
    
pub fn process_events(&mut self) {
        self.glfw.poll_events();
        
        let events: Vec<_> = glfw::flush_messages(&self.event_receiver).collect();
        
        for (_, event) in events {
            match event {
                WindowEvent::MouseButton(button, action, _) => {
                    let x = self.window.get_cursor_pos().0 as f32;
                    let y = self.window.get_cursor_pos().1 as f32;
                    
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
                    let action_raw = match action {
                        glfw::Action::Press => 1,
                        glfw::Action::Release => 0,
                        glfw::Action::Repeat => 2,
                    };
                    
                    // Space 和 S 只在 Press 时处理
                    if action_raw == 1 {  // GLFW Press
                        if key == Key::Space {
                            self.space_pressed = true;
                        }
                        if key == Key::S {
                            self.s_pressed = true;
                        }
                        if key == Key::P {
                            self.p_pressed = true;
                        }
                        if key == Key::F12 {
                            // Take diagnostic screenshot
                            let _ = self.capture_screenshot("screenshots/layout_diagnostic.png");
                        }
                    }
                    
                    // Generic key event handler: dispatch ALL keys with proper modifier state
                    let keycode = convert_glfw_key(key);
                    if keycode != KeyCode::Unknown {
                        let key_action = match action_raw {
                            1 => KeyAction::Press,
                            0 => KeyAction::Release,
                            2 => KeyAction::Repeat,
                            _ => KeyAction::Press,
                        };
                        let key_modifiers = KeyModifiers {
                            shift: mods.contains(glfw::Modifiers::Shift),
                            ctrl: mods.contains(glfw::Modifiers::Control),
                            alt: mods.contains(glfw::Modifiers::Alt),
                        };
                        self.input_handler.lock().on_key_event(&KeyEvent {
                            action: key_action,
                            keycode,
                            modifiers: key_modifiers,
                        }, self.frame_count);
                    }
                }
                WindowEvent::Char(codepoint) => {
                    if codepoint >= ' ' {
                        self.input_handler.lock().on_char_event(&CharEvent {
                            codepoint: codepoint as u32,
                        }, self.frame_count);
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
        
        hezhou_ui::thunk::flush_pending_callbacks();
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
            
            // offscreen_image格式是R8G8B8A8_UNORM（RGBA），像素数据已是RGBA顺序
            // 无需BGRA→RGBA转换（swapchain格式才是BGRA，offscreen不是）
            let pixels: Vec<u8> = data_slice.to_vec();
            
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
    
    /// 截取预览窗(game preview)画面并保存为PNG文件
    ///
    /// 从offscreen_image读取像素数据（即game pass渲染结果 — 3D场景），
    /// 不同于capture_screenshot截取的是swapchain全窗口画面。
    /// offscreen_image当前在SHADER_READ_ONLY_OPTIMAL layout，
    /// 需要先transition到TRANSFER_SRC_OPTIMAL再复制到host buffer。
    pub fn capture_preview_screenshot(&mut self, filepath: &str) -> Result<(), String> {
        unsafe {
            self.device.device_wait_idle()
                .map_err(|e| format!("Failed to wait for device idle: {}", e))?;
            
            // 截图源：使用当前帧实际渲染结果的image
            // rasterization: offscreen_image (经FXAA后)
            // ray_tracing: RayTracePipeline.output_image (compute shader输出)
            let source_image = self.screenshot_source_image;
            let width = self.screenshot_source_extent.width;
            let height = self.screenshot_source_extent.height;
            let buffer_size = (width * height * 4) as usize;
            
            // 创建host可见buffer用于接收像素数据
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
            
            // 分配command buffer用于copy操作
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
            
            // Transition截图源image → TRANSFER_SRC_OPTIMAL
            // 必须考虑first_frame: resize后或初始帧，image可能是UNDEFINED layout
            // ray_tracing模式下output_image的layout是SHADER_READ_ONLY_OPTIMAL
            let (old_layout, src_access, src_stage) = if self.first_frame {
                (vk::ImageLayout::UNDEFINED, vk::AccessFlags::empty(), vk::PipelineStageFlags::TOP_OF_PIPE)
            } else {
                (vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::AccessFlags::SHADER_READ, vk::PipelineStageFlags::FRAGMENT_SHADER)
            };
            let image_barrier = vk::ImageMemoryBarrier {
                old_layout,
                new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: source_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: src_access,
                dst_access_mask: vk::AccessFlags::TRANSFER_READ,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                _marker: std::marker::PhantomData,
            };
            
            self.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier]
            );
            
            // Copy截图源image → buffer
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
                source_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                buffer,
                &[buffer_image_copy]
            );
            
            // Transition截图源image back: TRANSFER_SRC_OPTIMAL → SHADER_READ_ONLY_OPTIMAL
            let image_barrier2 = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: source_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::TRANSFER_READ,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                p_next: std::ptr::null(),
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                _marker: std::marker::PhantomData,
            };
            
            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier2]
            );
            
            self.device.end_command_buffer(command_buffer)
                .map_err(|e| format!("Failed to end command buffer: {}", e))?;
            
            // Submit + wait
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
            
            // 读取像素数据
            let data_ptr = self.device.map_memory(buffer_memory, 0, buffer_size as u64, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map memory: {}", e))?;
            
            let data_slice = std::slice::from_raw_parts(data_ptr as *const u8, buffer_size);
            let pixels: Vec<u8> = data_slice.to_vec();
            
            // offscreen格式是R8G8B8A8_UNORM，像素数据已经是RGBA顺序，无需转换
            
            self.device.unmap_memory(buffer_memory);
            
            // 保存PNG
            let img_buffer: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = 
                image::ImageBuffer::from_raw(width, height, pixels)
                    .expect("Failed to create image buffer");
            
            img_buffer.save(filepath)
                .map_err(|e| format!("Failed to save image: {}", e))?;
            
            // 清理临时资源
            self.device.free_command_buffers(self.command_pool, &command_buffers);
            self.device.destroy_buffer(buffer, None);
            self.device.free_memory(buffer_memory, None);
            
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Info,
                "Screenshot",
                &format!("Preview screenshot saved to {} ({}x{})", filepath, width, height),
                file!(),
                line!()
            );
            
            Ok(())
        }
    }
    
    pub fn set_game_preview_extent(&mut self, width: u32, height: u32) -> Result<(), String> {
        let new_extent = vk::Extent2D { width, height };
        
        if new_extent.width == 0 || new_extent.height == 0 {
            return Err("Invalid preview extent".to_string());
        }
        
        if new_extent.width == self.offscreen_extent.width && new_extent.height == self.offscreen_extent.height {
            self.dfx.lock().get_logger().lock().log(
                LogLevel::Info,
                "Vulkan",
                &format!("Preview extent unchanged: {}x{}", width, height),
                file!(),
                line!()
            );
            return Ok(());
        }
        
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Info,
            "Vulkan",
            &format!("Deferring game preview resize: {}x{} -> {}x{}", 
                self.offscreen_extent.width, self.offscreen_extent.height, width, height),
            file!(),
            line!()
        );
        
        // Store the new extent for deferred recreation at the start of next frame.
        // This avoids a mid-frame device_wait_idle() that causes visible delay/jitter.
        self.pending_offscreen_resize = Some(new_extent);
        
        Ok(())
    }
    
    /// 销毁延迟列表中足够老的Vulkan资源
    /// 
    /// 只销毁帧号 >= frame_counter - DEFERRED_DESTROY_FRAMES 的资源。
    /// 刚退役的资源需要等待足够多的帧，确保GPU和Vulkan驱动完全不再引用它们。
    /// DEFERRED_DESTROY_FRAMES=3 意味着资源至少经过3帧后才被销毁。
/// 运行时安全的延迟销毁 — 每帧在fence wait后调用
    /// 只销毁帧号 <= safe_threshold 的资源（退役3帧以上，GPU不可能还在使用）
    /// 不调用device_wait_idle — fence wait已保证相关帧的GPU工作完成
    unsafe fn flush_deferred_destroy_periodic(&mut self) {
        if self.deferred_destroy_framebuffers.is_empty() 
            && self.deferred_destroy_image_views.is_empty()
            && self.deferred_destroy_images.is_empty()
            && self.deferred_destroy_memories.is_empty() {
            return;
        }
        
        // 只销毁帧号 <= (当前帧 - DEFERRED_DESTROY_FRAMES) 的旧资源
        // 保留较新的资源（可能还在被GPU使用）
        // DEFERRED_DESTROY_FRAMES=3: 超过2帧in-flight + 1帧安全裕度
        // 安全性论证: in_flight_fences.len()=2, wait_for_fences(fence[current_frame])
        // 保证frame[N-2]的GPU工作已完成。资源退役3+帧后，不可能被任何GPU command引用。
        const DEFERRED_DESTROY_FRAMES: u64 = 3;
        let safe_threshold = self.frame_counter.saturating_sub(DEFERRED_DESTROY_FRAMES);
        
        // 分离"可安全销毁"和"仍需保留"的资源
        let safe_fbs: Vec<vk::Framebuffer> = self.deferred_destroy_framebuffers.iter()
            .filter(|(frame, _)| *frame <= safe_threshold)
            .map(|(_, fb)| *fb)
            .collect();
        let safe_ivs: Vec<vk::ImageView> = self.deferred_destroy_image_views.iter()
            .filter(|(frame, _)| *frame <= safe_threshold)
            .map(|(_, iv)| *iv)
            .collect();
        let safe_imgs: Vec<vk::Image> = self.deferred_destroy_images.iter()
            .filter(|(frame, _)| *frame <= safe_threshold)
            .map(|(_, img)| *img)
            .collect();
        let safe_mems: Vec<vk::DeviceMemory> = self.deferred_destroy_memories.iter()
            .filter(|(frame, _)| *frame <= safe_threshold)
            .map(|(_, mem)| *mem)
            .collect();
        
        // 如果没有可安全销毁的资源，直接返回（避免无意义的日志和计算）
        if safe_fbs.is_empty() && safe_ivs.is_empty() && safe_imgs.is_empty() && safe_mems.is_empty() {
            return;
        }
        
        let remaining_fbs: Vec<(u64, vk::Framebuffer)> = self.deferred_destroy_framebuffers.iter()
            .filter(|(frame, _)| *frame > safe_threshold)
            .cloned()
            .collect();
        let remaining_ivs: Vec<(u64, vk::ImageView)> = self.deferred_destroy_image_views.iter()
            .filter(|(frame, _)| *frame > safe_threshold)
            .cloned()
            .collect();
        let remaining_imgs: Vec<(u64, vk::Image)> = self.deferred_destroy_images.iter()
            .filter(|(frame, _)| *frame > safe_threshold)
            .cloned()
            .collect();
        let remaining_mems: Vec<(u64, vk::DeviceMemory)> = self.deferred_destroy_memories.iter()
            .filter(|(frame, _)| *frame > safe_threshold)
            .cloned()
            .collect();
        
        if !safe_fbs.is_empty() || !safe_ivs.is_empty() || !safe_imgs.is_empty() || !safe_mems.is_empty() {
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", 
                &format!("Periodic deferred destroy: destroying {} fbs, {} ivs, {} imgs, {} mems (safe_threshold={}, remaining={}/{}/{}/{})", 
                    safe_fbs.len(), safe_ivs.len(), safe_imgs.len(), safe_mems.len(),
                    safe_threshold,
                    remaining_fbs.len(), remaining_ivs.len(), remaining_imgs.len(), remaining_mems.len()),
                file!(), line!());
        }
        
        // 销毁顺序: framebuffer(引用image_view) → image_view → image → memory
        // 无需device_wait_idle — fence wait + 帧阈值已保证GPU不再引用这些资源
        for fb in &safe_fbs {
            self.device.destroy_framebuffer(*fb, None);
        }
        for iv in &safe_ivs {
            self.device.destroy_image_view(*iv, None);
        }
        for img in &safe_imgs {
            self.device.destroy_image(*img, None);
        }
        for mem in &safe_mems {
            self.device.free_memory(*mem, None);
        }
        
        // 更新延迟列表 — 只保留较新的资源
        self.deferred_destroy_framebuffers = remaining_fbs;
        self.deferred_destroy_image_views = remaining_ivs;
        self.deferred_destroy_images = remaining_imgs;
        self.deferred_destroy_memories = remaining_mems;
    }
    
    /// cleanup时调用的最终销毁 — device_wait_idle + 销毁所有剩余资源
    unsafe fn flush_deferred_destroy(&mut self) {
        if self.deferred_destroy_framebuffers.is_empty() 
            && self.deferred_destroy_image_views.is_empty()
            && self.deferred_destroy_images.is_empty()
            && self.deferred_destroy_memories.is_empty() {
            return;
        }
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", 
            &format!("Final deferred destroy: destroying all remaining {} fbs, {} ivs, {} imgs, {} mems", 
                self.deferred_destroy_framebuffers.len(),
                self.deferred_destroy_image_views.len(),
                self.deferred_destroy_images.len(),
                self.deferred_destroy_memories.len()),
            file!(), line!());
        
        // cleanup时确保GPU完全空闲再销毁所有资源
        self.device.device_wait_idle()
            .expect("Failed to wait for device idle before final deferred flush");
        
        // 销毁顺序: framebuffer → image_view → image → memory
        for (_, fb) in &self.deferred_destroy_framebuffers {
            self.device.destroy_framebuffer(*fb, None);
        }
        for (_, iv) in &self.deferred_destroy_image_views {
            self.device.destroy_image_view(*iv, None);
        }
        for (_, img) in &self.deferred_destroy_images {
            self.device.destroy_image(*img, None);
        }
        for (_, mem) in &self.deferred_destroy_memories {
            self.device.free_memory(*mem, None);
        }
        
        self.deferred_destroy_framebuffers.clear();
        self.deferred_destroy_image_views.clear();
        self.deferred_destroy_images.clear();
        self.deferred_destroy_memories.clear();
}
    
    /// Recreate offscreen FBO resources with a pending extent.
    /// Called at the start of draw_frame after acquiring the swapchain image and
    /// waiting for the in-flight fence — at this point the GPU is idle for this frame
    /// so we can safely destroy and recreate Vulkan objects without a full device_wait_idle.
    unsafe fn apply_pending_offscreen_resize(&mut self) -> Result<(), String> {
        let new_extent = self.pending_offscreen_resize.take()
            .expect("apply_pending_offscreen_resize called without pending extent");
        
        // NOTE: 不在resize中调用flush_deferred_destroy() — 运行时销毁旧Vulkan资源
        // 会触发HEAP_CORRUPTION（即使device_wait_idle后也崩溃）。
        // 旧资源只累积在延迟列表中，在cleanup()时一次性销毁。
        // GPU资源暂时累积但resize不频繁，不会无限增长。
        
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Info,
            "Vulkan",
            &format!("Applying deferred game preview resize: {}x{} -> {}x{}", 
                self.offscreen_extent.width, self.offscreen_extent.height, 
                new_extent.width, new_extent.height),
            file!(),
            line!()
        );
        
        // 将旧offscreen资源推入延迟销毁列表 — 不能立即销毁，
        // 因为Vulkan驱动在GPU使用完framebuffer后可能有延迟的内部清理操作，
        // 立即销毁可能导致ACCESS_VIOLATION/HEAP_CORRUPTION。
        // 延迟销毁在下一帧draw_frame()的wait_for_fences后执行，确保GPU完全不再引用旧资源。
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Deferring old offscreen resources for later destruction...", file!(), line!());
        let retire_frame = self.frame_counter;  // 资源在此帧退役，延迟DEFERRED_DESTROY_FRAMES帧后销毁
        self.deferred_destroy_framebuffers.push((retire_frame, self.offscreen_framebuffer));
        self.deferred_destroy_image_views.push((retire_frame, self.offscreen_image_view));
        self.deferred_destroy_images.push((retire_frame, self.offscreen_image));
        self.deferred_destroy_memories.push((retire_frame, self.offscreen_image_memory));
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Deferring old depth resources for later destruction...", file!(), line!());
        self.deferred_destroy_image_views.push((retire_frame, self.depth_image_view));
        self.deferred_destroy_images.push((retire_frame, self.depth_image));
        self.deferred_destroy_memories.push((retire_frame, self.depth_image_memory));
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Deferring old FXAA resources for later destruction...", file!(), line!());
        self.deferred_destroy_framebuffers.push((retire_frame, self.offscreen_fxaa_framebuffer));
        self.deferred_destroy_image_views.push((retire_frame, self.offscreen_fxaa_image_view));
        self.deferred_destroy_images.push((retire_frame, self.offscreen_fxaa_image));
        self.deferred_destroy_memories.push((retire_frame, self.offscreen_fxaa_image_memory));
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Old FBO resources deferred, creating new...", file!(), line!());
        
        // Create new offscreen image (game output)
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Creating new offscreen image...", file!(), line!());
        let (offscreen_image, offscreen_image_memory) = Self::create_offscreen_image(
            &self.instance, &self.device, self.physical_device, new_extent, self.offscreen_format
        )?;
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Creating new offscreen image_view...", file!(), line!());
        
        let offscreen_image_view = self.device.create_image_view(&vk::ImageViewCreateInfo {
            image: offscreen_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: self.offscreen_format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        }, None).map_err(|e| format!("Failed to create offscreen image view: {}", e))?;
        
        // Create new depth image
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Creating new depth image...", file!(), line!());
        let (depth_image, depth_image_memory) = Self::create_depth_image(
            &self.instance, &self.device, self.physical_device, new_extent
        )?;
        
        let depth_image_view = self.device.create_image_view(&vk::ImageViewCreateInfo {
            image: depth_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::D32_SFLOAT,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        }, None).map_err(|e| format!("Failed to create depth image view: {}", e))?;
        
        let offscreen_framebuffer = self.device.create_framebuffer(&vk::FramebufferCreateInfo {
            render_pass: self.game_render_pass,
            attachment_count: 2,
            p_attachments: &[offscreen_image_view, depth_image_view] as *const _,
            width: new_extent.width,
            height: new_extent.height,
            layers: 1,
            ..Default::default()
        }, None).map_err(|e| format!("Failed to create offscreen framebuffer: {}", e))?;
        
        self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Creating FXAA resources...", file!(), line!());
        
        // Create new FXAA output image
        let (offscreen_fxaa_image, offscreen_fxaa_image_memory) = Self::create_offscreen_image(
            &self.instance, &self.device, self.physical_device, new_extent, self.offscreen_format
        )?;
        
        let offscreen_fxaa_image_view = self.device.create_image_view(&vk::ImageViewCreateInfo {
            image: offscreen_fxaa_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: self.offscreen_format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        }, None).map_err(|e| format!("Failed to create FXAA image view: {}", e))?;
        
        let offscreen_fxaa_framebuffer = self.device.create_framebuffer(&vk::FramebufferCreateInfo {
            render_pass: self.game_render_pass,
            attachment_count: 2,
            p_attachments: &[offscreen_fxaa_image_view, depth_image_view] as *const _,
            width: new_extent.width,
            height: new_extent.height,
            layers: 1,
            ..Default::default()
        }, None).map_err(|e| format!("Failed to create FXAA framebuffer: {}", e))?;
        
        // Update FXAA descriptor set to sample from new offscreen image
        self.device.update_descriptor_sets(
            &[vk::WriteDescriptorSet {
                dst_set: self.fxaa_descriptor_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &vk::DescriptorImageInfo {
                    sampler: self.fxaa_sampler,
                    image_view: offscreen_image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                },
                ..Default::default()
            }],
            &[]
        );
        
        // Update preview descriptor set to point to new offscreen image
        self.device.update_descriptor_sets(
            &[vk::WriteDescriptorSet {
                dst_set: self.preview_descriptor_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &vk::DescriptorImageInfo {
                    sampler: self.preview_sampler,
                    image_view: offscreen_image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                },
                ..Default::default()
            }],
            &[]
        );
        
        // Update struct fields
        self.offscreen_image = offscreen_image;
        self.offscreen_image_memory = offscreen_image_memory;
        self.offscreen_image_view = offscreen_image_view;
        self.offscreen_framebuffer = offscreen_framebuffer;
        self.offscreen_extent = new_extent;
        
        self.depth_image = depth_image;
        self.depth_image_memory = depth_image_memory;
        self.depth_image_view = depth_image_view;
        
        self.offscreen_fxaa_image = offscreen_fxaa_image;
        self.offscreen_fxaa_image_memory = offscreen_fxaa_image_memory;
        self.offscreen_fxaa_image_view = offscreen_fxaa_image_view;
        self.offscreen_fxaa_framebuffer = offscreen_fxaa_framebuffer;
        
        // 同步新offscreen handle到PipelineRegistry（防止resize后pipeline使用失效handle）
        if let Some(ref mut registry) = self.pipeline_registry {
            registry.update_offscreen_resources(
                self.offscreen_image,
                self.offscreen_image_memory,
                self.offscreen_image_view,
                self.offscreen_framebuffer,
                self.depth_image,
                self.depth_image_memory,
                self.depth_image_view,
                self.offscreen_fxaa_image,
                self.offscreen_fxaa_image_memory,
                self.offscreen_fxaa_image_view,
                self.offscreen_fxaa_framebuffer,
            );
        }
        
        // Resize pipeline专属资源（如raytrace的output image）
        // 通过trait resize()方法调用，避免downcast_mut路径
        // pipeline内部将旧资源推入自己的deferred列表（不再立即销毁）
        // 此处drain转移到renderer的统一延迟销毁列表，在cleanup()时由flush_deferred_destroy()统一释放
        let retire_frame = self.frame_counter;
        if let Some(ref mut registry) = self.pipeline_registry {
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Resizing pipeline output resources...", file!(), line!());
            if let Some(pipeline) = registry.active_pipeline_mut() {
                pipeline.resize(new_extent.width, new_extent.height);
                
                // 将pipeline的延迟销毁资源转移到renderer的统一延迟列表
                // 使用as_any_mut()获取RayTracePipeline特有方法
                let (deferred_ivs, deferred_imgs, deferred_mems) = {
                    let any_ref = pipeline.as_any_mut();
                    if let Some(rt_pipeline) = any_ref.downcast_mut::<hezhou_render_pipeline::RayTracePipeline>() {
                        rt_pipeline.drain_deferred_destroys()
                    } else {
                        (Vec::new(), Vec::new(), Vec::new())
                    }
                };
                for iv in deferred_ivs {
                    self.deferred_destroy_image_views.push((retire_frame, iv));
                }
                for img in deferred_imgs {
                    self.deferred_destroy_images.push((retire_frame, img));
                }
                for mem in deferred_mems {
                    self.deferred_destroy_memories.push((retire_frame, mem));
                }
            }
            self.dfx.lock().get_logger().lock().log(LogLevel::Info, "Vulkan", "Pipeline output resources resized", file!(), line!());
        }
        
        self.dfx.lock().get_logger().lock().log(
            LogLevel::Info,
            "Vulkan",
            &format!("Game preview resources recreated: {}x{}", new_extent.width, new_extent.height),
            file!(),
            line!()
        );
        
        // New images start in UNDEFINED layout, so next frame must use UNDEFINED as old_layout
        self.first_frame = true;
        
        Ok(())
    }
    
    pub fn set_camera_params(&mut self, yaw: f32, pitch: f32, x: f32, y: f32, z: f32) {
        self.camera_yaw = yaw;
        self.camera_pitch = pitch;
        self.camera_x = x;
        self.camera_y = y;
        self.camera_z = z;
    }
    
    pub fn set_game_state(&mut self, state: i32) {
        self.game_state = state;
    }
    
    pub fn get_game_state(&self) -> i32 {
        self.game_state
    }
    
    pub fn set_entity_transform(&mut self, px: f32, py: f32, pz: f32, 
                                  rx: f32, ry: f32, rz: f32, rw: f32,
                                  sx: f32, sy: f32, sz: f32) {
        self.entity_position = [px, py, pz];
        self.entity_rotation = [rx, ry, rz, rw];
        self.entity_scale = [sx, sy, sz];
    }
    
    pub fn set_entity_angle(&mut self, angle: f32) {
        self.entity_angle = angle;
    }
    
    pub fn get_entity_angle(&self) -> f32 {
        self.entity_angle
    }
    
    pub fn set_selected_entity(&mut self, entity_id: u64, selected: bool) {
        self.selected_entity_id = entity_id;
        self.is_entity_selected = selected;
        dfx_debug!("Vulkan", "Selected entity: id={}, selected={}", entity_id, selected);
    }
    
    pub fn set_scene(&mut self, scene: *mut hezhou_core::Scene) {
        self.scene_ptr = Some(scene);
    }
    
    /// 加载OBJ模型到GPU mesh buffer
    /// 返回自定义mesh路径字符串（asset://前缀），用于RenderableComponent的mesh_path
    pub fn load_obj_mesh(&mut self, obj_path: &str) -> Result<String, String> {
        // 1. 解析OBJ文件
        let obj_data = crate::mesh_loader::load_obj_file(obj_path)?;
        
        // 2. 计算vertex offset（在primitive数据之后追加）
        let vertex_offset = self.next_custom_mesh_vertex_offset();
        let vertex_data_bytes = obj_data.vertices.len() * std::mem::size_of::<primitive_meshes::MeshVertex>();
        
        // 3. 上传vertex数据到game_mesh_buffer的预留区域
        unsafe {
            let byte_offset = vertex_offset as vk::DeviceSize * std::mem::size_of::<primitive_meshes::MeshVertex>() as vk::DeviceSize;
            let data_ptr = self.device.map_memory(
                self.game_mesh_buffer_memory,
                byte_offset,
                vertex_data_bytes as vk::DeviceSize,
                vk::MemoryMapFlags::empty()
            ).map_err(|e| format!("Failed to map mesh buffer: {}", e))?;
            
            std::ptr::copy_nonoverlapping(
                obj_data.vertices.as_ptr() as *const u8,
                data_ptr as *mut u8,
                vertex_data_bytes,
            );
            self.device.unmap_memory(self.game_mesh_buffer_memory);
        }
        
        // 4. 计算index offset
        let index_offset = self.next_custom_mesh_index_offset();
        let index_data_bytes = obj_data.indices.len() * std::mem::size_of::<u32>();
        
        // 5. 上传index数据到game_index_buffer
        unsafe {
            let byte_offset = index_offset as vk::DeviceSize * std::mem::size_of::<u32>() as vk::DeviceSize;
            let data_ptr = self.device.map_memory(
                self.game_index_buffer_memory,
                byte_offset,
                index_data_bytes as vk::DeviceSize,
                vk::MemoryMapFlags::empty()
            ).map_err(|e| format!("Failed to map index buffer: {}", e))?;
            
            std::ptr::copy_nonoverlapping(
                obj_data.indices.as_ptr() as *const u8,
                data_ptr as *mut u8,
                index_data_bytes,
            );
            self.device.unmap_memory(self.game_index_buffer_memory);
        }
        
        // 6. 存储CustomMeshInfo
        let mesh_path = format!("asset://{}", obj_path);
        self.custom_meshes.insert(mesh_path.clone(), CustomMeshInfo {
            vertex_offset,
            vertex_count: obj_data.vertices.len() as u32,
            index_offset,
            index_count: obj_data.indices.len() as u32,
            has_texture: false,  // 初始无纹理
            texture_descriptor_set: None,
            specular_strength: 0.3,    // 默认高光强度
            ambient_strength: 0.15,    // 默认环境光强度
            shininess: 32.0,           // 默认高光指数
        });
        self.dirty_dynamic_data = true;
        
        Ok(mesh_path)
    }
    
    /// 加载纹理文件到GPU，创建descriptor set并存入texture_cache
    /// path: 纹理文件路径（PNG/JPEG等）
    pub fn load_texture_from_file(&mut self, path: &str) -> Result<(), String> {
        // 如果已缓存，直接返回
        if self.texture_cache.contains_key(path) {
            return Ok(());
        }
        
        // 1. 使用image crate解码纹理文件
        let img = image::open(path)
            .map_err(|e| format!("Failed to load texture '{}': {}", path, e))?;
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();
        let data = rgba_img.as_raw();
        
        // 2. 创建Vulkan image (使用create_texture_2d_simple)
        let (texture_image, texture_memory) = Self::create_texture_2d_simple(
            &self.instance, &self.device, self.physical_device, 
            width, height, data
        )?;
        
        // 3. 转换image layout到SHADER_READ_ONLY_OPTIMAL
        unsafe {
            let transition_cmd = self.device.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                command_pool: self.command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: 1,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate transition command buffer for '{}': {}", path, e))?[0];
            
            self.device.begin_command_buffer(transition_cmd, &vk::CommandBufferBeginInfo::default())
                .map_err(|e| format!("Failed to begin transition command buffer for '{}': {}", path, e))?;
            
            let barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::PREINITIALIZED,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_access_mask: vk::AccessFlags::HOST_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                image: texture_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                ..Default::default()
            };
            
            self.device.cmd_pipeline_barrier(
                transition_cmd,
                vk::PipelineStageFlags::HOST,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
            
            self.device.end_command_buffer(transition_cmd)
                .map_err(|e| format!("Failed to end transition command buffer for '{}': {}", path, e))?;
            
            self.device.queue_submit(self.queue, &[vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &transition_cmd,
                ..Default::default()
            }], vk::Fence::null())
                .map_err(|e| format!("Failed to submit transition for '{}': {}", path, e))?;
            
            self.device.queue_wait_idle(self.queue)
                .map_err(|e| format!("Failed to wait for transition for '{}': {}", path, e))?;
            
            self.device.free_command_buffers(self.command_pool, &[transition_cmd]);
        }
        
        // 4. 创建image view
        let texture_view = unsafe {
            self.device.create_image_view(&vk::ImageViewCreateInfo {
                view_type: vk::ImageViewType::TYPE_2D,
                format: vk::Format::R8G8B8A8_UNORM,
                image: texture_image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create texture view for '{}': {}", path, e))?
        };
        
        // 5. 创建sampler
        let texture_sampler = unsafe {
            self.device.create_sampler(&vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                address_mode_u: vk::SamplerAddressMode::REPEAT,
                address_mode_v: vk::SamplerAddressMode::REPEAT,
                address_mode_w: vk::SamplerAddressMode::REPEAT,
                mip_lod_bias: 0.0,
                max_anisotropy: 1.0,
                compare_op: vk::CompareOp::NEVER,
                min_lod: 0.0,
                max_lod: 0.0,
                border_color: vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
                unnormalized_coordinates: 0,
                ..Default::default()
            }, None).map_err(|e| format!("Failed to create texture sampler for '{}': {}", path, e))?
        };
        
        // 6. 分配descriptor set
        let texture_descriptor_set = unsafe {
            self.device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                descriptor_pool: self.texture_descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &self.texture_descriptor_set_layout,
                ..Default::default()
            }).map_err(|e| format!("Failed to allocate texture descriptor set for '{}': {}", path, e))?[0]
        };
        
        // 7. 更新descriptor set
        unsafe {
            self.device.update_descriptor_sets(
                &[vk::WriteDescriptorSet {
                    dst_set: texture_descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &vk::DescriptorImageInfo {
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        image_view: texture_view,
                        sampler: texture_sampler,
                    },
                    ..Default::default()
                }],
                &[] as &[vk::CopyDescriptorSet],
            );
        }
        
        // 8. 存入texture_cache
        self.texture_cache.insert(path.to_string(), TextureCacheEntry {
            image: texture_image,
            memory: texture_memory,
            view: texture_view,
            sampler: texture_sampler,
            descriptor_set: texture_descriptor_set,
        });
        self.dirty_dynamic_data = true;
        
        Ok(())
    }
    
    /// 计算下一个自定义mesh的vertex offset（primitive数据之后）
    fn next_custom_mesh_vertex_offset(&self) -> u32 {
        // primitive meshes占用前面的空间
        let primitive_vertex_count = self.primitive_ranges.values()
            .map(|r| r.vertex_count)
            .sum::<u32>();
        
        // 自定义mesh追加在后面
        let custom_vertex_count = self.custom_meshes.values()
            .map(|m| m.vertex_count)
            .sum::<u32>();
        
        primitive_vertex_count + custom_vertex_count
    }
    
    /// 计算下一个自定义mesh的index offset
    fn next_custom_mesh_index_offset(&self) -> u32 {
        self.custom_meshes.values()
            .map(|m| m.index_count)
            .sum::<u32>()
    }
    
    fn compute_model_matrix(transform: &hezhou_core::LocalTransform) -> [[f32; 4]; 4] {
        let t = hezhou_core::Mat4::translate(transform.position);
        let r = hezhou_core::Mat4::from_quaternion(transform.rotation);
        let s = hezhou_core::Mat4::scale(transform.scale);
        let model = t * r * s;
        model.data
    }
    
    fn compute_scaled_model_matrix(transform: &hezhou_core::LocalTransform, scale_factor: f32) -> [[f32; 4]; 4] {
        let t = hezhou_core::Mat4::translate(transform.position);
        let r = hezhou_core::Mat4::from_quaternion(transform.rotation);
        let s = hezhou_core::Mat4::scale(hezhou_core::Vec3::new(
            transform.scale.x * scale_factor,
            transform.scale.y * scale_factor,
            transform.scale.z * scale_factor,
        ));
        let model = t * r * s;
        model.data
    }
    
    pub fn cleanup(&mut self) {
        unsafe {
            self.device.device_wait_idle().expect("Failed to wait for device idle");
            
            // Flush deferred destroy — 程序退出时销毁所有延迟列表中的旧Vulkan资源
            self.flush_deferred_destroy();
            
            // Cleanup game preview resources
            self.device.destroy_framebuffer(self.offscreen_framebuffer, None);
            self.device.destroy_image_view(self.offscreen_image_view, None);
            self.device.destroy_image(self.offscreen_image, None);
            self.device.free_memory(self.offscreen_image_memory, None);
            
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_image_memory, None);
            
            self.device.destroy_framebuffer(self.offscreen_fxaa_framebuffer, None);
            self.device.destroy_image_view(self.offscreen_fxaa_image_view, None);
            self.device.destroy_image(self.offscreen_fxaa_image, None);
            self.device.free_memory(self.offscreen_fxaa_image_memory, None);
            
            self.device.destroy_sampler(self.preview_sampler, None);
            
            self.device.destroy_pipeline(self.game_pipeline, None);
            self.device.destroy_pipeline_layout(self.game_pipeline_layout, None);
            self.device.destroy_render_pass(self.game_render_pass, None);
            
            // Cleanup light UBO resources
            self.device.destroy_descriptor_pool(self.light_descriptor_pool, None);
            self.device.destroy_descriptor_set_layout(self.light_descriptor_set_layout, None);
            self.device.destroy_buffer(self.light_ubo, None);
            self.device.free_memory(self.light_ubo_memory, None);
            
            // Cleanup texture descriptor resources
            // Cleanup texture cache resources (per-entity textures)
            for entry in self.texture_cache.values() {
                self.device.destroy_sampler(entry.sampler, None);
                self.device.destroy_image_view(entry.view, None);
                self.device.destroy_image(entry.image, None);
                self.device.free_memory(entry.memory, None);
            }
            self.device.destroy_descriptor_pool(self.texture_descriptor_pool, None);
            self.device.destroy_descriptor_set_layout(self.texture_descriptor_set_layout, None);
            self.device.destroy_sampler(self.default_texture_sampler, None);
            self.device.destroy_image_view(self.default_texture_view, None);
            self.device.destroy_image(self.default_texture_image, None);
            self.device.free_memory(self.default_texture_memory, None);
            
            self.device.destroy_buffer(self.game_mesh_buffer, None);
            self.device.free_memory(self.game_mesh_buffer_memory, None);
            
            self.device.destroy_buffer(self.game_index_buffer, None);
            self.device.free_memory(self.game_index_buffer_memory, None);
            
            self.device.destroy_pipeline(self.fxaa_pipeline, None);
            self.device.destroy_pipeline_layout(self.fxaa_pipeline_layout, None);
            self.device.destroy_descriptor_pool(self.fxaa_descriptor_pool, None);
            self.device.destroy_descriptor_set_layout(self.fxaa_descriptor_set_layout, None);
            self.device.destroy_sampler(self.fxaa_sampler, None);
            
            for vb in &self.vertex_buffers {
                self.device.destroy_buffer(*vb, None);
            }
            for vbm in &self.vertex_buffer_memories {
                self.device.free_memory(*vbm, None);
            }
            
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
