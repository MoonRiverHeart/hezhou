use ash::vk;

pub struct VulkanBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: usize,
}

impl VulkanBuffer {
    pub fn new_vertex(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: &ash::Device,
        data: &[f32],
    ) -> Result<Self, hezhou_rhi::RhiError> {
        unsafe {
            let size = (data.len() * 4) as u64;
            
            let buffer_info = vk::BufferCreateInfo {
                size,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            
            let buffer = device.create_buffer(&buffer_info, None)
                .map_err(|e| hezhou_rhi::RhiError::BufferCreationFailed(e.to_string()))?;
            
            let requirements = device.get_buffer_memory_requirements(buffer);
            
            let memory_type_index = {
                let memory_properties = instance.get_physical_device_memory_properties(physical_device);
                let mut index = 0u32;
                for (i, mem_type) in memory_properties.memory_types.iter().enumerate() {
                    let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
                    if suitable && mem_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT) {
                        index = i as u32;
                        break;
                    }
                }
                index
            };
            
            let alloc_info = vk::MemoryAllocateInfo {
                allocation_size: requirements.size,
                memory_type_index,
                ..Default::default()
            };
            
            let memory = device.allocate_memory(&alloc_info, None)
                .map_err(|e| hezhou_rhi::RhiError::OutOfMemory(e.to_string()))?;
            
            device.bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| hezhou_rhi::RhiError::BufferCreationFailed(e.to_string()))?;
            
            let ptr = device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty())
                .map_err(|e| hezhou_rhi::RhiError::MappingFailed(e.to_string()))?;
            
            std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, ptr as *mut u8, data.len() * 4);
            
            device.unmap_memory(memory);
            
            Ok(Self {
                buffer,
                memory,
                size: data.len() * 4,
            })
        }
    }
    
    pub fn buffer(&self) -> vk::Buffer {
        self.buffer
    }
    
    pub fn size(&self) -> usize {
        self.size
    }
    
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.memory, None);
        }
    }
}