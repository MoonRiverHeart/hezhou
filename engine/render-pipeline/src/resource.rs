//! 管线GPU资源管理
//!
//! PipelineResourceDesc — 管线所需GPU资源类型描述
//! PipelineResources — 管线专属GPU资源容器（buffer/image/descriptor等）
//! ResourceManager — 跨管线切换的资源生命周期管理

use ash::vk;

/// 管线GPU资源需求描述
///
/// 在管线注册时声明需要的资源大小和类型，
/// ResourceManager据此分配GPU资源。
/// 对应UIVulkanRenderer中的gameMeshBuffers/gameIndexBuffers/
/// offscreenImage/depthImage等字段。
#[derive(Clone, Debug)]
pub struct PipelineResourceDesc {
    /// 顶点buffer大小（字节）
    pub vertex_buffer_size: usize,
    /// 索引buffer大小（字节）
    pub index_buffer_size: usize,
    /// 需要的纹理数量
    pub texture_count: usize,
    /// 是否需要depth attachment
    pub needs_depth_attachment: bool,
    /// 是否需要offscreen渲染目标（FBO）
    pub needs_offscreen_target: bool,
    /// push constant大小（字节），必须匹配GLSL std430/std140对齐规则
    pub push_constant_size: usize,
    /// uniform buffer大小（字节）
    pub uniform_buffer_size: usize,
}

impl Default for PipelineResourceDesc {
    fn default() -> Self {
        Self {
            vertex_buffer_size: 0,
            index_buffer_size: 0,
            texture_count: 0,
            needs_depth_attachment: false,
            needs_offscreen_target: false,
            push_constant_size: 0,
            uniform_buffer_size: 0,
        }
    }
}

/// 管线专属GPU资源容器
///
/// 所有字段用Option，因为不同管线需要的资源不同:
/// - ray tracing管线不需要depth attachment
/// - 纯UI管线不需要vertex/index buffer
/// - 后处理管线只需要offscreen target
///
/// 对应UIVulkanRenderer中的GPU资源字段（Lines 2805-2954）:
/// gameMeshBuffers, gameIndexBuffers, offscreenImage/View/Framebuffer,
/// depthImage/View/Memory等。
pub struct PipelineResources {
    /// 顶点buffer
    pub vertex_buffer: Option<vk::Buffer>,
    /// 顶点buffer内存
    pub vertex_buffer_memory: Option<vk::DeviceMemory>,
    /// 索引buffer
    pub index_buffer: Option<vk::Buffer>,
    /// 索引buffer内存
    pub index_buffer_memory: Option<vk::DeviceMemory>,
    /// offscreen渲染目标image
    pub offscreen_image: Option<vk::Image>,
    /// offscreen image内存
    pub offscreen_memory: Option<vk::DeviceMemory>,
    /// offscreen image view
    pub offscreen_view: Option<vk::ImageView>,
    /// offscreen framebuffer
    pub offscreen_framebuffer: Option<vk::Framebuffer>,
    /// depth attachment image
    pub depth_image: Option<vk::Image>,
    /// depth image内存
    pub depth_memory: Option<vk::DeviceMemory>,
    /// depth image view
    pub depth_view: Option<vk::ImageView>,
    /// FXAA offscreen image（后处理抗锯齿输出目标）
    pub fxaa_image: Option<vk::Image>,
    /// FXAA offscreen image内存
    pub fxaa_memory: Option<vk::DeviceMemory>,
    /// FXAA offscreen image view
    pub fxaa_view: Option<vk::ImageView>,
    /// FXAA offscreen framebuffer
    pub fxaa_framebuffer: Option<vk::Framebuffer>,
    /// descriptor sets（用于纹理采样、uniform buffer等）
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    /// uniform buffer（用于MVP矩阵等）
    pub uniform_buffer: Option<vk::Buffer>,
    /// uniform buffer内存
    pub uniform_buffer_memory: Option<vk::DeviceMemory>,
}

impl Default for PipelineResources {
    fn default() -> Self {
        Self {
            vertex_buffer: None,
            vertex_buffer_memory: None,
            index_buffer: None,
            index_buffer_memory: None,
            offscreen_image: None,
            offscreen_memory: None,
            offscreen_view: None,
            offscreen_framebuffer: None,
            depth_image: None,
            depth_memory: None,
            depth_view: None,
            fxaa_image: None,
            fxaa_memory: None,
            fxaa_view: None,
            fxaa_framebuffer: None,
            descriptor_sets: Vec::new(),
            uniform_buffer: None,
            uniform_buffer_memory: None,
        }
    }
}

/// 待销毁资源记录
///
/// 在管线切换时，旧管线的GPU资源不能立即销毁（GPU可能仍在使用），
/// 必须先device_wait_idle()，然后才能安全销毁。
/// 此结构记录需要延迟销毁的资源列表。
#[derive(Default)]
pub struct PendingDestroyResources {
    /// 待销毁的buffer列表
    pub buffers: Vec<vk::Buffer>,
    /// 待销毁的device memory列表
    pub memories: Vec<vk::DeviceMemory>,
    /// 待销毁的image列表
    pub images: Vec<vk::Image>,
    /// 待销毁的image view列表
    pub image_views: Vec<vk::ImageView>,
    /// 待销毁的framebuffer列表
    pub framebuffers: Vec<vk::Framebuffer>,
}

/// 资源管理器 — 管理跨pipeline切换的资源生命周期
///
/// 在switch_pipeline时必须先device_wait_idle()，
/// 然后销毁旧管线资源，再为新管线分配资源。
///
/// **重要**: 销毁FBO资源前必须调用device_wait_idle()，
/// 否则可能导致VK_ERROR_DEVICE_LOST（已有bug修复经验）。
///
/// ResourceManager只管理资源描述和待销毁列表，
/// 实际的Vulkan分配/销毁操作由rhi-vulkan层完成
/// （因为需要unsafe Vulkan API调用）。
pub struct ResourceManager {
    /// 当前活跃管线的资源描述
    current_desc: PipelineResourceDesc,
    /// Vulkan device handle（用于分配/销毁资源的标识）
    device: vk::Device,
    /// 物理设备内存属性（用于buffer分配的内存类型选择）
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    /// 待销毁资源列表（管线切换时收集，device_wait_idle后批量销毁）
    pending_destroy: PendingDestroyResources,
}

impl ResourceManager {
    /// 创建ResourceManager
    ///
    /// 需要Vulkan device和物理设备内存属性，
    /// 用于后续的buffer/image分配。
    pub fn new(
        device: vk::Device,
        memory_properties: vk::PhysicalDeviceMemoryProperties,
    ) -> Self {
        Self {
            current_desc: PipelineResourceDesc::default(),
            device,
            memory_properties,
            pending_destroy: PendingDestroyResources::default(),
        }
    }

    /// 获取当前资源描述
    pub fn current_desc(&self) -> &PipelineResourceDesc {
        &self.current_desc
    }

    /// 获取Vulkan device handle
    pub fn device(&self) -> vk::Device {
        self.device
    }

    /// 获取物理设备内存属性
    pub fn memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        self.memory_properties
    }

    /// 更新资源描述（在管线切换时调用）
    pub fn update_desc(&mut self, desc: PipelineResourceDesc) {
        self.current_desc = desc;
    }

    /// 根据资源描述创建PipelineResources容器
    ///
    /// 注意: 实际的Vulkan分配逻辑（vkCreateBuffer、vkAllocateMemory等）
    /// 需要在rhi-vulkan crate中实现（需要unsafe Vulkan API调用）。
    /// 这里只创建容器结构，字段用null handle占位。
    pub fn allocate_resources(&self, desc: &PipelineResourceDesc) -> PipelineResources {
        let mut resources = PipelineResources::default();

        // buffer和image的实际分配由rhi-vulkan层完成
        // 这里只根据desc设置哪些字段需要分配（null handle占位）
        if desc.vertex_buffer_size > 0 {
            resources.vertex_buffer = Some(vk::Buffer::null());
            resources.vertex_buffer_memory = Some(vk::DeviceMemory::null());
        }

        if desc.index_buffer_size > 0 {
            resources.index_buffer = Some(vk::Buffer::null());
            resources.index_buffer_memory = Some(vk::DeviceMemory::null());
        }

        if desc.uniform_buffer_size > 0 {
            resources.uniform_buffer = Some(vk::Buffer::null());
            resources.uniform_buffer_memory = Some(vk::DeviceMemory::null());
        }

        if desc.needs_offscreen_target {
            resources.offscreen_image = Some(vk::Image::null());
            resources.offscreen_memory = Some(vk::DeviceMemory::null());
            resources.offscreen_view = Some(vk::ImageView::null());
            resources.offscreen_framebuffer = Some(vk::Framebuffer::null());
        }

        if desc.needs_depth_attachment {
            resources.depth_image = Some(vk::Image::null());
            resources.depth_memory = Some(vk::DeviceMemory::null());
            resources.depth_view = Some(vk::ImageView::null());
        }

        resources.descriptor_sets = vec![vk::DescriptorSet::null(); desc.texture_count.max(1)];

        resources
    }

    /// 将旧管线资源标记为待销毁
    ///
    /// 在switch_pipeline时调用，将旧管线的GPU资源收集到待销毁列表。
    /// **必须在device_wait_idle()之后才能实际销毁**，
    /// 否则GPU可能仍在使用这些资源→VK_ERROR_DEVICE_LOST。
    pub fn mark_for_destroy(&mut self, old_resources: PipelineResources) {
        // 收集所有非null的资源handle到待销毁列表
        if let Some(buf) = old_resources.vertex_buffer {
            if buf != vk::Buffer::null() {
                self.pending_destroy.buffers.push(buf);
            }
        }
        if let Some(mem) = old_resources.vertex_buffer_memory {
            if mem != vk::DeviceMemory::null() {
                self.pending_destroy.memories.push(mem);
            }
        }
        if let Some(buf) = old_resources.index_buffer {
            if buf != vk::Buffer::null() {
                self.pending_destroy.buffers.push(buf);
            }
        }
        if let Some(mem) = old_resources.index_buffer_memory {
            if mem != vk::DeviceMemory::null() {
                self.pending_destroy.memories.push(mem);
            }
        }
        if let Some(buf) = old_resources.uniform_buffer {
            if buf != vk::Buffer::null() {
                self.pending_destroy.buffers.push(buf);
            }
        }
        if let Some(mem) = old_resources.uniform_buffer_memory {
            if mem != vk::DeviceMemory::null() {
                self.pending_destroy.memories.push(mem);
            }
        }
        if let Some(img) = old_resources.offscreen_image {
            if img != vk::Image::null() {
                self.pending_destroy.images.push(img);
            }
        }
        if let Some(mem) = old_resources.offscreen_memory {
            if mem != vk::DeviceMemory::null() {
                self.pending_destroy.memories.push(mem);
            }
        }
        if let Some(view) = old_resources.offscreen_view {
            if view != vk::ImageView::null() {
                self.pending_destroy.image_views.push(view);
            }
        }
        if let Some(fb) = old_resources.offscreen_framebuffer {
            if fb != vk::Framebuffer::null() {
                self.pending_destroy.framebuffers.push(fb);
            }
        }
        if let Some(img) = old_resources.depth_image {
            if img != vk::Image::null() {
                self.pending_destroy.images.push(img);
            }
        }
        if let Some(mem) = old_resources.depth_memory {
            if mem != vk::DeviceMemory::null() {
                self.pending_destroy.memories.push(mem);
            }
        }
        if let Some(view) = old_resources.depth_view {
            if view != vk::ImageView::null() {
                self.pending_destroy.image_views.push(view);
            }
        }
    }

    /// 获取待销毁资源列表（供rhi-vulkan层在device_wait_idle后批量销毁）
    ///
    /// 返回后清空内部列表，避免重复销毁。
    pub fn take_pending_destroy(&mut self) -> PendingDestroyResources {
        std::mem::take(&mut self.pending_destroy)
    }

    /// 检查是否有待销毁资源
    pub fn has_pending_destroy(&self) -> bool {
        !self.pending_destroy.buffers.is_empty()
            || !self.pending_destroy.memories.is_empty()
            || !self.pending_destroy.images.is_empty()
            || !self.pending_destroy.image_views.is_empty()
            || !self.pending_destroy.framebuffers.is_empty()
    }
}