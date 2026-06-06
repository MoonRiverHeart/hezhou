use std::ffi::CString;
use ash::{vk, Entry, Instance, Device};
use ash::khr::{surface, swapchain};
use rhi::{RhiInitDesc, RenderResult, WindowHandle};

pub struct VulkanContext {
    entry: Entry,
    instance: Instance,
    device: Device,
    surface: vk::SurfaceKHR,
    surface_loader: surface::Instance,
    swapchain_loader: swapchain::Device,
    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    render_pass: vk::RenderPass,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
    framebuffer_width: u32,
    framebuffer_height: u32,
    swapchain_dirty: bool,
    frame_count: usize,
}

impl VulkanContext {
    pub fn new(desc: &RhiInitDesc) -> Self {
        let entry = unsafe { Entry::load().unwrap() };
        let instance = Self::create_instance(&entry, desc);
        let surface = Self::create_surface(&entry, &instance, &desc.window_handle);
        let surface_loader = surface::Instance::new(&entry, &instance);
        let physical_device = Self::pick_physical_device(&instance, &surface_loader, surface);
        let queue_family_index = Self::find_queue_family_index(&instance, physical_device, &surface_loader, surface);
        let (device, graphics_queue, present_queue) = 
            Self::create_device(&instance, physical_device, queue_family_index);
        
        let swapchain_loader = swapchain::Device::new(&instance, &device);
        
        let (swapchain, swapchain_images, swapchain_image_views, _format, _extent) =
            Self::create_swapchain(
                &instance, &device, physical_device,
                &surface_loader, &swapchain_loader,
                surface, desc.width, desc.height,
            );
        
        let render_pass = Self::create_render_pass(&device);
        
        let framebuffers = Self::create_framebuffers(
            &device, &swapchain_image_views, render_pass,
            desc.width, desc.height,
        );
        
        let command_pool = Self::create_command_pool(&device, queue_family_index);
        
        let image_count = swapchain_images.len();
        let command_buffers = Self::create_command_buffers(&device, command_pool, image_count as u32);
        
        let mut image_available_semaphores = Vec::with_capacity(image_count);
        let mut render_finished_semaphores = Vec::with_capacity(image_count);
        let mut in_flight_fences = Vec::with_capacity(image_count);
        
        for _ in 0..image_count {
            image_available_semaphores.push(
                unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() }
            );
            render_finished_semaphores.push(
                unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() }
            );
            in_flight_fences.push(
                unsafe {
                    device.create_fence(
                        &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                        None,
                    ).unwrap()
                }
            );
        }
        
        VulkanContext {
            entry, instance, device, surface, surface_loader, swapchain_loader,
            physical_device, graphics_queue, present_queue,
            swapchain, swapchain_images, swapchain_image_views, framebuffers,
            render_pass, command_pool, command_buffers,
            image_available_semaphores, render_finished_semaphores, in_flight_fences,
            current_frame: 0,
            framebuffer_width: desc.width, framebuffer_height: desc.height,
            swapchain_dirty: false,
            frame_count: 0,
        }
    }
    
    pub fn begin_frame(&mut self) {
        // 用帧计数器索引，而非swapchain图片索引
        let frame_idx = self.frame_count % self.in_flight_fences.len();
        
        // 等待这一帧的fence
        unsafe {
            self.device.wait_for_fences(
                &[self.in_flight_fences[frame_idx]],
                true,
                u64::MAX,
            ).unwrap();
            self.device.reset_fences(&[self.in_flight_fences[frame_idx]]).unwrap();
        }
        
        if self.swapchain_dirty {
            self.recreate_swapchain();
            self.swapchain_dirty = false;
        }
        
        // 获取下一个swapchain图片
        let result = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[frame_idx],
                vk::Fence::null(),
            )
        };
        
        match result {
            Ok((image_index, _)) => {
                self.current_frame = image_index as usize;
                self.frame_count = self.frame_count.wrapping_add(1);
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain();
            }
            Err(_) => {}
        }
    }
    
    pub fn end_frame(&mut self) -> RenderResult {
        let frame_idx = (self.frame_count.wrapping_sub(1)) % self.in_flight_fences.len();
        
        let cmd_buffer = self.command_buffers[self.current_frame];
        
        let wait_semaphores = [self.image_available_semaphores[frame_idx]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[frame_idx]];
        let cmd_buffers = [cmd_buffer];
        
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&cmd_buffers)
            .signal_semaphores(&signal_semaphores);
        
        unsafe {
            self.device.queue_submit(
                self.graphics_queue,
                &[submit_info],
                self.in_flight_fences[frame_idx],
            ).unwrap();
        }
        
        let swapchains = [self.swapchain];
        let image_indices = [self.current_frame as u32];
        
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        
        unsafe { self.swapchain_loader.queue_present(self.present_queue, &present_info).ok(); }
        
        RenderResult { frame_completed: true }
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.framebuffer_width = width;
        self.framebuffer_height = height;
        self.swapchain_dirty = true;
    }
    
    pub fn framebuffer_size(&self) -> (u32, u32) {
        (self.framebuffer_width, self.framebuffer_height)
    }
    
    pub fn wait_idle(&self) {
        unsafe { self.device.device_wait_idle().unwrap(); }
    }
    
    pub fn device(&self) -> &Device { &self.device }
    pub fn physical_device(&self) -> vk::PhysicalDevice { self.physical_device }
    pub fn instance(&self) -> &Instance { &self.instance }
    pub fn command_pool(&self) -> vk::CommandPool { self.command_pool }
    pub fn render_pass(&self) -> vk::RenderPass { self.render_pass }
    pub fn framebuffers(&self) -> &[vk::Framebuffer] { &self.framebuffers }
    pub fn current_command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffers[self.current_frame]
    }
    pub fn current_image_index(&self) -> usize { self.current_frame }
    pub fn framebuffer_width(&self) -> u32 { self.framebuffer_width }
    pub fn framebuffer_height(&self) -> u32 { self.framebuffer_height }
    pub fn graphics_queue(&self) -> vk::Queue { self.graphics_queue }
    
    fn create_instance(entry: &Entry, _desc: &RhiInitDesc) -> Instance {
        let app_name = CString::new("RHI Vulkan").unwrap();
        let engine_name = CString::new("No Engine").unwrap();
        
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);
        
        let layer_names: Vec<CString> = if cfg!(debug_assertions) {
            vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()]
        } else {
            vec![]
        };
        let layer_ptrs: Vec<*const i8> = layer_names.iter().map(|n| n.as_ptr()).collect();
        
        let extension_names = vec![
            surface::NAME.as_ptr(),
            ash::khr::win32_surface::NAME.as_ptr(),
        ];
        
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&layer_ptrs)
            .enabled_extension_names(&extension_names);
        
        unsafe { entry.create_instance(&create_info, None).unwrap() }
    }
    
    fn create_surface(entry: &Entry, instance: &Instance, window_handle: &WindowHandle) -> vk::SurfaceKHR {
        let create_info = vk::Win32SurfaceCreateInfoKHR::default()
            .hwnd(window_handle.hwnd as isize)
            .hinstance(0);
        unsafe {
            ash::khr::win32_surface::Instance::new(entry, instance)
                .create_win32_surface(&create_info, None).unwrap()
        }
    }
    
    fn pick_physical_device(instance: &Instance, surface_loader: &surface::Instance, surface: vk::SurfaceKHR) -> vk::PhysicalDevice {
        let devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        devices.into_iter().find(|&device| {
            let queue_families = unsafe { instance.get_physical_device_queue_family_properties(device) };
            queue_families.iter().enumerate().any(|(i, props)| {
                props.queue_flags.contains(vk::QueueFlags::GRAPHICS) &&
                unsafe { surface_loader.get_physical_device_surface_support(device, i as u32, surface).unwrap() }
            })
        }).expect("No suitable physical device")
    }
    
    fn find_queue_family_index(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        surface_loader: &surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> u32 {
        let queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        queue_families.iter().enumerate()
            .find(|(i, props)| {
                props.queue_flags.contains(vk::QueueFlags::GRAPHICS) &&
                unsafe { surface_loader.get_physical_device_surface_support(physical_device, *i as u32, surface).unwrap() }
            })
            .map(|(i, _)| i as u32)
            .expect("No suitable queue family")
    }
    
    fn create_device(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> (Device, vk::Queue, vk::Queue) {
        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);
        
        let queue_create_infos = [queue_create_info];
        let device_extensions = vec![swapchain::NAME.as_ptr()];
        
        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions);
        
        let device = unsafe { instance.create_device(physical_device, &create_info, None).unwrap() };
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        
        (device, queue, queue)
    }
    
    fn create_swapchain(
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
        surface_loader: &surface::Instance,
        swapchain_loader: &swapchain::Device,
        surface: vk::SurfaceKHR,
        width: u32,
        height: u32,
    ) -> (vk::SwapchainKHR, Vec<vk::Image>, Vec<vk::ImageView>, vk::Format, vk::Extent2D) {
        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface).unwrap()
        };
        
        let format = vk::Format::B8G8R8A8_SRGB;
        let extent = vk::Extent2D {
            width: width.clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
            height: height.clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
        };
        
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(capabilities.min_image_count.max(2))
            .image_format(format)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true);
        
        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None).unwrap() };
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };
        
        let image_views: Vec<vk::ImageView> = images.iter().map(|&image| {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0, level_count: 1,
                    base_array_layer: 0, layer_count: 1,
                });
            unsafe { device.create_image_view(&create_info, None).unwrap() }
        }).collect();
        
        (swapchain, images, image_views, format, extent)
    }
    
    fn create_render_pass(device: &Device) -> vk::RenderPass {
        let color_attachment = vk::AttachmentDescription::default()
            .format(vk::Format::B8G8R8A8_SRGB)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
        
        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        
        let color_attachments = [color_attachment_ref];
        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);
        
        let attachments = [color_attachment];
        let subpasses = [subpass];
        
        let create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses);
        
        unsafe { device.create_render_pass(&create_info, None).unwrap() }
    }
    
    fn create_framebuffers(
        device: &Device,
        image_views: &[vk::ImageView],
        render_pass: vk::RenderPass,
        width: u32,
        height: u32,
    ) -> Vec<vk::Framebuffer> {
        image_views.iter().map(|&view| {
            let attachments = [view];
            let create_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(width).height(height).layers(1);
            unsafe { device.create_framebuffer(&create_info, None).unwrap() }
        }).collect()
    }
    
    fn create_command_pool(device: &Device, queue_family_index: u32) -> vk::CommandPool {
        let create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        unsafe { device.create_command_pool(&create_info, None).unwrap() }
    }
    
    fn create_command_buffers(device: &Device, command_pool: vk::CommandPool, count: u32) -> Vec<vk::CommandBuffer> {
        let create_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);
        unsafe { device.allocate_command_buffers(&create_info).unwrap() }
    }
    
    fn recreate_swapchain(&mut self) {
        unsafe { self.device.device_wait_idle().unwrap(); }
        
        for &fb in &self.framebuffers {
            unsafe { self.device.destroy_framebuffer(fb, None); }
        }
        for &iv in &self.swapchain_image_views {
            unsafe { self.device.destroy_image_view(iv, None); }
        }
        unsafe { self.swapchain_loader.destroy_swapchain(self.swapchain, None); }
        
        let (sc, imgs, ivs, _, extent) = Self::create_swapchain(
            &self.instance, &self.device, self.physical_device,
            &self.surface_loader, &self.swapchain_loader,
            self.surface, self.framebuffer_width, self.framebuffer_height,
        );
        
        self.swapchain = sc;
        self.swapchain_images = imgs;
        self.swapchain_image_views = ivs;
        self.framebuffer_width = extent.width;   // 加这行
        self.framebuffer_height = extent.height; // 加这行
        
        self.framebuffers = Self::create_framebuffers(
            &self.device, &self.swapchain_image_views, self.render_pass,
            self.framebuffer_width, self.framebuffer_height,
        );
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            for i in 0..self.swapchain_images.len() {
                self.device.destroy_semaphore(self.image_available_semaphores[i], None);
                self.device.destroy_semaphore(self.render_finished_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }
            self.device.destroy_command_pool(self.command_pool, None);
            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }
            for &iv in &self.swapchain_image_views {
                self.device.destroy_image_view(iv, None);
            }
            self.device.destroy_render_pass(self.render_pass, None);
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}