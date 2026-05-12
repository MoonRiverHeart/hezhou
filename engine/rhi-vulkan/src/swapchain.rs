use ash::vk;
use hezhou_rhi::RhiError;

pub struct SwapchainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub fn choose_swapchain_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats.iter()
        .find(|f| f.format == vk::Format::B8G8R8A8_UNORM && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
        .unwrap_or(&formats[0])
        .clone()
}

pub fn choose_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    present_modes.iter()
        .find(|m| **m == vk::PresentModeKHR::MAILBOX)
        .map(|m| *m)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

pub fn choose_extent(capabilities: &vk::SurfaceCapabilitiesKHR, width: u32, height: u32) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: width.max(capabilities.min_image_extent.width).min(capabilities.max_image_extent.width),
            height: height.max(capabilities.min_image_extent.height).min(capabilities.max_image_extent.height),
        }
    }
}