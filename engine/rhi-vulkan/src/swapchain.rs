use ash::vk;
use ash::khr::surface::Instance as SurfaceLoader;
use ash::khr::swapchain::Device as SwapchainLoader;
use hezhou_rhi::RhiError;

pub struct VulkanSwapchain {
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    format: vk::Format,
    extent: vk::Extent2D,
    swapchain_loader: SwapchainLoader,
}

impl VulkanSwapchain {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        surface: vk::SurfaceKHR,
        render_pass: vk::RenderPass,
        width: u32,
        height: u32,
    ) -> Result<Self, RhiError> {
        unsafe {
            let surface_loader = SurfaceLoader::new(entry, instance);
            
            let formats = surface_loader.get_physical_device_surface_formats(physical_device, surface)
                .map_err(|e| RhiError::SwapchainCreationFailed(e.to_string()))?;
            
            let present_modes = surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
                .map_err(|e| RhiError::SwapchainCreationFailed(e.to_string()))?;
            
            let capabilities = surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
                .map_err(|e| RhiError::SwapchainCreationFailed(e.to_string()))?;
            
            let format = formats.iter()
                .find(|f| f.format == vk::Format::B8G8R8A8_UNORM)
                .map(|f| f.format)
                .unwrap_or(formats[0].format);
            
            let color_space = formats.iter()
                .find(|f| f.format == vk::Format::B8G8R8A8_UNORM)
                .map(|f| f.color_space)
                .unwrap_or(formats[0].color_space);
            
            let present_mode = *present_modes.iter()
                .find(|m| **m == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(&vk::PresentModeKHR::FIFO);
            
            let extent = if capabilities.current_extent.width != u32::MAX {
                capabilities.current_extent
            } else {
                vk::Extent2D {
                    width: width.clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
                    height: height.clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
                }
            };
            
            let mut image_count = capabilities.min_image_count + 1;
            if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
                image_count = capabilities.max_image_count;
            }
            
            let swapchain_loader = SwapchainLoader::new(instance, device);
            
            let swapchain_info = vk::SwapchainCreateInfoKHR {
                surface,
                min_image_count: image_count,
                image_format: format,
                image_color_space: color_space,
                image_extent: extent,
                image_array_layers: 1,
                image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                pre_transform: capabilities.current_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode,
                clipped: vk::TRUE,
                old_swapchain: vk::SwapchainKHR::null(),
                ..Default::default()
            };
            
            let swapchain = swapchain_loader.create_swapchain(&swapchain_info, None)
                .map_err(|e| RhiError::SwapchainCreationFailed(e.to_string()))?;
            
            let images = swapchain_loader.get_swapchain_images(swapchain)
                .map_err(|e| RhiError::SwapchainCreationFailed(e.to_string()))?;
            
            let image_views: Vec<vk::ImageView> = images.iter()
                .map(|image| {
                    let view_info = vk::ImageViewCreateInfo {
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
                    device.create_image_view(&view_info, None)
                        .map_err(|e| RhiError::SwapchainCreationFailed(e.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            let framebuffers: Vec<vk::Framebuffer> = image_views.iter()
                .map(|view| {
                    let fb_info = vk::FramebufferCreateInfo {
                        render_pass,
                        attachment_count: 1,
                        p_attachments: &*view,
                        width: extent.width,
                        height: extent.height,
                        layers: 1,
                        ..Default::default()
                    };
                    device.create_framebuffer(&fb_info, None)
                        .map_err(|e| RhiError::FramebufferCreationFailed(e.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            
            Ok(Self {
                swapchain,
                images,
                image_views,
                framebuffers,
                format,
                extent,
                swapchain_loader,
            })
        }
    }
    
    pub fn swapchain(&self) -> vk::SwapchainKHR {
        self.swapchain
    }
    
    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }
    
    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.image_views
    }
    
    pub fn framebuffers(&self) -> &[vk::Framebuffer] {
        &self.framebuffers
    }
    
    pub fn format(&self) -> vk::Format {
        self.format
    }
    
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
    
    pub fn loader(&self) -> &SwapchainLoader {
        &self.swapchain_loader
    }
    
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            for fb in &self.framebuffers {
                device.destroy_framebuffer(*fb, None);
            }
            for view in &self.image_views {
                device.destroy_image_view(*view, None);
            }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}