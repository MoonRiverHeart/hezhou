use ash::vk;
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct VulkanContext {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    graphics_queue_family_index: u32,
    allocator: RwLock<gpu_allocator::vulkan::Allocator>,
    next_handle: RwLock<u64>,
    buffers: RwLock<HashMap<u64, (vk::Buffer, gpu_allocator::vulkan::Allocation)>>,
    textures: RwLock<HashMap<u64, (vk::Image, vk::ImageView, gpu_allocator::vulkan::Allocation)>>,
    shaders: RwLock<HashMap<u64, vk::ShaderModule>>,
    pipelines: RwLock<HashMap<u64, vk::Pipeline>>,
    render_passes: RwLock<HashMap<u64, vk::RenderPass>>,
    framebuffers: RwLock<HashMap<u64, vk::Framebuffer>>,
    command_pools: RwLock<HashMap<u64, vk::CommandPool>>,
}

impl VulkanContext {
    pub fn new() -> crate::RhiResult<Self> {
        unsafe {
            let entry = ash::Entry::load()
                .map_err(|e| crate::RhiError::DeviceCreationFailed(e.to_string()))?;
            
            let app_info = vk::ApplicationInfo {
                api_version: vk::API_VERSION_1_2,
                ..Default::default()
            };
            
            let layer_names: Vec<*const i8> = vec![];
            let extension_names: Vec<*const i8> = vec![];
            
            let instance_create_info = vk::InstanceCreateInfo {
                application_info: &app_info,
                enabled_layer_names: &layer_names.as_ptr(),
                enabled_extension_names: &extension_names.as_ptr(),
                ..Default::default()
            };
            
            let instance = entry.create_instance(&instance_create_info, None)
                .map_err(|e| crate::RhiError::DeviceCreationFailed(e.to_string()))?;
            
            let physical_devices = instance.enumerate_physical_devices()
                .map_err(|e| crate::RhiError::DeviceCreationFailed(e.to_string()))?;
            
            let physical_device = physical_devices.into_iter().next()
                .ok_or(crate::RhiError::DeviceCreationFailed("No physical device found"))?;
            
            let queue_family_properties = instance.get_physical_device_queue_family_properties(physical_device);
            let graphics_queue_family_index = queue_family_properties
                .iter()
                .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .unwrap_or(0) as u32;
            
            let queue_create_info = vk::DeviceQueueCreateInfo {
                queue_family_index: graphics_queue_family_index,
                queue_count: 1,
                p_queue_priorities: &[1.0f32].as_ptr(),
                ..Default::default()
            };
            
            let extension_names: Vec<*const i8> = vec![vk::KHR_SWAPCHAIN_EXTENSION_NAME.as_ptr()];
            
            let device_create_info = vk::DeviceCreateInfo {
                queue_create_infos: &[queue_create_info].as_ptr(),
                queue_create_info_count: 1,
                enabled_extension_names: &extension_names.as_ptr(),
                ..Default::default()
            };
            
            let device = instance.create_device(physical_device, &device_create_info, None)
                .map_err(|e| crate::RhiError::DeviceCreationFailed(e.to_string()))?;
            
            let graphics_queue = device.get_device_queue(graphics_queue_family_index, 0);
            
            let allocator_desc = gpu_allocator::vulkan::AllocatorCreateDesc {
                physical_device,
                device: device.clone(),
                instance: instance.clone(),
                buffer_device_address: false,
                debug_settings: gpu_allocator::vulkan::AllocatorDebugSettings::default(),
                allocation_sizes: gpu_allocator::vulkan::AllocationSizes::default(),
            };
            
            let allocator = gpu_allocator::vulkan::Allocator::new(&allocator_desc)
                .map_err(|e| crate::RhiError::DeviceCreationFailed(e.to_string()))?;
            
            Ok(Self {
                entry,
                instance,
                physical_device,
                device,
                graphics_queue,
                graphics_queue_family_index,
                allocator: RwLock::new(allocator),
                next_handle: RwLock::new(1),
                buffers: RwLock::new(HashMap::new()),
                textures: RwLock::new(HashMap::new()),
                shaders: RwLock::new(HashMap::new()),
                pipelines: RwLock::new(HashMap::new()),
                render_passes: RwLock::new(HashMap::new()),
                framebuffers: RwLock::new(HashMap::new()),
                command_pools: RwLock::new(HashMap::new()),
            })
        }
    }
    
    pub fn next_handle(&self) -> u64 {
        let mut next = self.next_handle.write();
        let handle = *next;
        *next += 1;
        handle
    }
    
    pub fn device(&self) -> &ash::Device {
        &self.device
    }
    
    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }
    
    pub fn graphics_queue_family_index(&self) -> u32 {
        self.graphics_queue_family_index
    }
    
    pub fn allocator(&self) -> &RwLock<gpu_allocator::vulkan::Allocator> {
        &self.allocator
    }
    
    pub fn store_buffer(&self, handle: u64, buffer: vk::Buffer, allocation: gpu_allocator::vulkan::Allocation) {
        self.buffers.write().insert(handle, (buffer, allocation));
    }
    
    pub fn get_buffer(&self, handle: u64) -> Option<vk::Buffer> {
        self.buffers.read().get(&handle).map(|(b, _)| *b)
    }
    
    pub fn remove_buffer(&self, handle: u64) -> Option<(vk::Buffer, gpu_allocator::vulkan::Allocation)> {
        self.buffers.write().remove(&handle)
    }
    
    pub fn store_shader(&self, handle: u64, shader: vk::ShaderModule) {
        self.shaders.write().insert(handle, shader);
    }
    
    pub fn get_shader(&self, handle: u64) -> Option<vk::ShaderModule> {
        self.shaders.read().get(&handle).copied()
    }
    
    pub fn remove_shader(&self, handle: u64) -> Option<vk::ShaderModule> {
        self.shaders.write().remove(&handle)
    }
    
    pub fn store_render_pass(&self, handle: u64, pass: vk::RenderPass) {
        self.render_passes.write().insert(handle, pass);
    }
    
    pub fn get_render_pass(&self, handle: u64) -> Option<vk::RenderPass> {
        self.render_passes.read().get(&handle).copied()
    }
    
    pub fn remove_render_pass(&self, handle: u64) -> Option<vk::RenderPass> {
        self.render_passes.write().remove(&handle)
    }
    
    pub fn store_command_pool(&self, handle: u64, pool: vk::CommandPool) {
        self.command_pools.write().insert(handle, pool);
    }
    
    pub fn get_command_pool(&self, handle: u64) -> Option<vk::CommandPool> {
        self.command_pools.read().get(&handle).copied()
    }
    
    pub fn remove_command_pool(&self, handle: u64) -> Option<vk::CommandPool> {
        self.command_pools.write().remove(&handle)
    }
    
    pub fn destroy(&self) {
        unsafe {
            for (_, (buffer, _)) in self.buffers.read().iter() {
                self.device.destroy_buffer(*buffer, None);
            }
            for (_, shader) in self.shaders.read().iter() {
                self.device.destroy_shader_module(*shader, None);
            }
            for (_, pass) in self.render_passes.read().iter() {
                self.device.destroy_render_pass(*pass, None);
            }
            for (_, pool) in self.command_pools.read().iter() {
                self.device.destroy_command_pool(*pool, None);
            }
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        self.destroy();
    }
}