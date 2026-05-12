use ash::vk;
use ash::khr::surface::Instance;
use hezhou_rhi::RhiError;
use std::ffi::CString;

pub fn create_instance(entry: &ash::Entry, app_name: &str) -> Result<ash::Instance, RhiError> {
    unsafe {
        let app_name_c = CString::new(app_name).unwrap();
        let engine_name_c = CString::new("Hezhou Engine").unwrap();
        
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name_c.as_ptr(),
            application_version: vk::make_api_version(0, 1, 0, 0),
            p_engine_name: engine_name_c.as_ptr(),
            engine_version: vk::make_api_version(0, 1, 0, 0),
            api_version: vk::API_VERSION_1_2,
            ..Default::default()
        };
        
        let extension_names = [
            vk::KHR_SURFACE_NAME.as_ptr(),
            vk::KHR_WIN32_SURFACE_NAME.as_ptr(),
        ];
        
        let instance_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
            ..Default::default()
        };
        
        entry.create_instance(&instance_info, None)
            .map_err(|e| RhiError::DeviceCreationFailed(format!("Instance creation failed: {}", e)))
    }
}

pub fn select_physical_device(instance: &ash::Instance) -> Result<vk::PhysicalDevice, RhiError> {
    unsafe {
        let devices = instance.enumerate_physical_devices()
            .map_err(|e| RhiError::DeviceCreationFailed(format!("Failed to enumerate devices: {}", e)))?;
        
        devices.into_iter().next()
            .ok_or(RhiError::DeviceCreationFailed("No physical device found".to_string()))
    }
}

pub fn find_graphics_queue_family(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> Result<u32, RhiError> {
    unsafe {
        let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
        
        queue_families.iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32)
            .ok_or(RhiError::DeviceCreationFailed("No graphics queue found".to_string()))
    }
}

pub fn create_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
) -> Result<(ash::Device, vk::Queue), RhiError> {
    unsafe {
        let queue_priority = 1.0f32;
        let queue_create_info = vk::DeviceQueueCreateInfo {
            queue_family_index,
            queue_count: 1,
            p_queue_priorities: &queue_priority,
            ..Default::default()
        };
        
        let extension_names = [vk::KHR_SWAPCHAIN_NAME.as_ptr()];
        
        let device_info = vk::DeviceCreateInfo {
            p_queue_create_infos: &queue_create_info,
            queue_create_info_count: 1,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
            ..Default::default()
        };
        
        let device = instance.create_device(physical_device, &device_info, None)
            .map_err(|e| RhiError::DeviceCreationFailed(format!("Device creation failed: {}", e)))?;
        
        let queue = device.get_device_queue(queue_family_index, 0);
        
        Ok((device, queue))
    }
}

pub fn create_command_pool(device: &ash::Device, queue_family_index: u32) -> Result<vk::CommandPool, RhiError> {
    unsafe {
        let pool_info = vk::CommandPoolCreateInfo {
            queue_family_index,
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            ..Default::default()
        };
        
        device.create_command_pool(&pool_info, None)
            .map_err(|e| RhiError::CommandPoolCreationFailed(format!("Command pool creation failed: {}", e)))
    }
}

pub fn create_semaphore(device: &ash::Device) -> Result<vk::Semaphore, RhiError> {
    unsafe {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        device.create_semaphore(&semaphore_info, None)
            .map_err(|e| RhiError::DeviceCreationFailed(format!("Semaphore creation failed: {}", e)))
    }
}

pub fn create_fence(device: &ash::Device, signaled: bool) -> Result<vk::Fence, RhiError> {
    unsafe {
        let fence_info = vk::FenceCreateInfo {
            flags: if signaled { vk::FenceCreateFlags::SIGNALED } else { vk::FenceCreateFlags::empty() },
            ..Default::default()
        };
        device.create_fence(&fence_info, None)
            .map_err(|e| RhiError::DeviceCreationFailed(format!("Fence creation failed: {}", e)))
    }
}

pub fn create_render_pass(device: &ash::Device, format: vk::Format) -> Result<vk::RenderPass, RhiError> {
    unsafe {
        let color_attachment = vk::AttachmentDescription {
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
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
        
        device.create_render_pass(&render_pass_info, None)
            .map_err(|e| RhiError::RenderPassCreationFailed(e.to_string()))
    }
}