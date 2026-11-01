//! 方向光 Shadow Map GPU 资源（0.2）
//!
//! 依据 [`crate::render_quality::shadow_pass_desc`] 创建:
//! - `D32_SFLOAT` 深度图（`array_layers = cascade_count`）
//! - 整张 array 的 image view（shader 采样）+ 每层单独 view（framebuffer 用）
//! - 比较采样器（`LESS_OR_EQUAL` + `CLAMP_TO_BORDER`）
//! - 深度专用 render pass + 每层 framebuffer
//!
//! **重要约束**:
//! - 资源销毁必须先 `device_wait_idle()`（与现有 FBO 约定一致，否则可能 DEVICE_LOST）
//! - `destroy()` 由调用方在 GPU 空闲后显式调用；本类型没有 Drop 实现，
//!   避免 device 先于资源销毁导致重复释放
//! - GPU 测试标记 `#[ignore]`，需本机 Vulkan 驱动；不进入常规 CI

use ash::vk;
use hezhou_render_pipeline::RenderSettings;

use crate::render_quality::{shadow_pass_desc, ShadowPassDesc};

/// 阴影图创建/销毁错误
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowMapError {
    /// 错误描述
    pub message: String,
}

impl ShadowMapError {
    /// 构造错误
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ShadowMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shadow map error: {}", self.message)
    }
}

impl std::error::Error for ShadowMapError {}

/// 阴影深度图及其视图/采样器/render pass
pub struct ShadowMapResources {
    /// 单级分辨率
    pub extent: vk::Extent2D,
    /// 深度格式
    pub format: vk::Format,
    /// cascade 层数
    pub cascade_count: u32,
    /// 深度图（array）
    pub image: vk::Image,
    /// 深度图显存
    pub memory: vk::DeviceMemory,
    /// 整张 array 的 image view（shader 采样）
    pub array_view: vk::ImageView,
    /// 每层单独 image view（framebuffer 用）
    pub layer_views: Vec<vk::ImageView>,
    /// 比较采样器
    pub sampler: vk::Sampler,
    /// 深度专用 render pass
    pub render_pass: vk::RenderPass,
    /// 每层 framebuffer
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl ShadowMapResources {
    /// 创建阴影贴图资源
    ///
    /// # Safety
    /// - `device` 必须由 `instance`/`physical_device` 创建且处于合法状态
    /// - 调用方需保证线程安全（Vulkan 外部同步由调用方负责）
    pub unsafe fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        desc: &ShadowPassDesc,
    ) -> Result<Self, ShadowMapError> {
        if desc.cascade_count == 0 {
            return Err(ShadowMapError::new("cascade_count 不能为 0"));
        }
        let mut resources = Self {
            extent: desc.extent,
            format: desc.format,
            cascade_count: desc.cascade_count,
            image: vk::Image::null(),
            memory: vk::DeviceMemory::null(),
            array_view: vk::ImageView::null(),
            layer_views: Vec::new(),
            sampler: vk::Sampler::null(),
            render_pass: vk::RenderPass::null(),
            framebuffers: Vec::new(),
        };
        match resources.create_inner(instance, device, physical_device, desc) {
            Ok(()) => Ok(resources),
            Err(err) => {
                // 部分创建失败时回收已分配资源
                resources.destroy(device);
                Err(err)
            }
        }
    }

    /// 根据渲染设置直接创建（阴影未启用时返回 None）
    ///
    /// # Safety
    /// 同 [`ShadowMapResources::create`]
    pub unsafe fn create_from_settings(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        settings: &RenderSettings,
    ) -> Option<Result<Self, ShadowMapError>> {
        let desc = shadow_pass_desc(settings)?;
        Some(Self::create(instance, device, physical_device, &desc))
    }

    unsafe fn create_inner(
        &mut self,
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        desc: &ShadowPassDesc,
    ) -> Result<(), ShadowMapError> {
        // 1. 深度图
        self.image = device
            .create_image(&desc.image_create_info(), None)
            .map_err(|e| ShadowMapError::new(format!("创建阴影深度图失败: {e}")))?;
        let requirements = device.get_image_memory_requirements(self.image);
        let memory_properties = instance.get_physical_device_memory_properties(physical_device);
        let memory_type = find_memory_type(
            &memory_properties,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        self.memory = device
            .allocate_memory(
                &vk::MemoryAllocateInfo {
                    allocation_size: requirements.size,
                    memory_type_index: memory_type,
                    ..Default::default()
                },
                None,
            )
            .map_err(|e| ShadowMapError::new(format!("分配阴影显存失败: {e}")))?;
        device
            .bind_image_memory(self.image, self.memory, 0)
            .map_err(|e| ShadowMapError::new(format!("绑定阴影显存失败: {e}")))?;

        // 2. array image view
        let array_view_info = desc.array_view_create_info(self.image);
        self.array_view = device
            .create_image_view(&array_view_info, None)
            .map_err(|e| ShadowMapError::new(format!("创建阴影 array view 失败: {e}")))?;

        // 3. 每层 view（framebuffer 需要单层 view）
        for layer in 0..self.cascade_count {
            let layer_view_info = desc.layer_view_create_info(self.image, layer);
            let view = device
                .create_image_view(&layer_view_info, None)
                .map_err(|e| ShadowMapError::new(format!("创建阴影 layer view 失败: {e}")))?;
            self.layer_views.push(view);
        }

        // 4. 比较采样器
        self.sampler = device
            .create_sampler(&desc.sampler_create_info(), None)
            .map_err(|e| ShadowMapError::new(format!("创建阴影采样器失败: {e}")))?;

        // 5. 深度专用 render pass
        self.render_pass = create_depth_render_pass(device, &desc.attachment_description())?;

        // 6. 每层 framebuffer
        for view in &self.layer_views {
            let framebuffer = device
                .create_framebuffer(
                    &vk::FramebufferCreateInfo {
                        render_pass: self.render_pass,
                        attachment_count: 1,
                        p_attachments: view,
                        width: self.extent.width,
                        height: self.extent.height,
                        layers: 1,
                        ..Default::default()
                    },
                    None,
                )
                .map_err(|e| ShadowMapError::new(format!("创建阴影 framebuffer 失败: {e}")))?;
            self.framebuffers.push(framebuffer);
        }

        Ok(())
    }

    /// 销毁全部资源（调用前必须 `device_wait_idle()`）
    ///
    /// # Safety
    /// - GPU 必须已空闲且不再使用这些资源
    /// - 同一资源不可重复销毁（销毁后 handle 置 null）
    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        for framebuffer in self.framebuffers.drain(..) {
            if framebuffer != vk::Framebuffer::null() {
                device.destroy_framebuffer(framebuffer, None);
            }
        }
        for view in self.layer_views.drain(..) {
            if view != vk::ImageView::null() {
                device.destroy_image_view(view, None);
            }
        }
        if self.array_view != vk::ImageView::null() {
            device.destroy_image_view(self.array_view, None);
            self.array_view = vk::ImageView::null();
        }
        if self.sampler != vk::Sampler::null() {
            device.destroy_sampler(self.sampler, None);
            self.sampler = vk::Sampler::null();
        }
        if self.render_pass != vk::RenderPass::null() {
            device.destroy_render_pass(self.render_pass, None);
            self.render_pass = vk::RenderPass::null();
        }
        if self.image != vk::Image::null() {
            device.destroy_image(self.image, None);
            self.image = vk::Image::null();
        }
        if self.memory != vk::DeviceMemory::null() {
            device.free_memory(self.memory, None);
            self.memory = vk::DeviceMemory::null();
        }
    }

    /// 生成 shader 采样用的 descriptor image info（array view + 比较采样器）
    pub fn descriptor_info(&self, layout: vk::ImageLayout) -> vk::DescriptorImageInfo {
        vk::DescriptorImageInfo {
            sampler: self.sampler,
            image_view: self.array_view,
            image_layout: layout,
            ..Default::default()
        }
    }

    /// 取第 cascade 级 framebuffer
    pub fn framebuffer(&self, cascade: u32) -> Option<vk::Framebuffer> {
        self.framebuffers.get(cascade as usize).copied()
    }
}

/// 选择满足要求的内存类型
fn find_memory_type(
    properties: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    required: vk::MemoryPropertyFlags,
) -> Result<u32, ShadowMapError> {
    for index in 0..properties.memory_type_count {
        let supported = (type_bits & (1 << index)) != 0;
        let flags = properties.memory_types[index as usize].property_flags;
        if supported && flags.contains(required) {
            return Ok(index);
        }
    }
    Err(ShadowMapError::new("找不到满足阴影深度图要求的显存类型"))
}

/// 创建深度专用 render pass
///
/// 单 subpass，只有深度 attachment；
/// finalLayout = SHADER_READ_ONLY_OPTIMAL，供 PBR pass 采样。
unsafe fn create_depth_render_pass(
    device: &ash::Device,
    attachment: &vk::AttachmentDescription,
) -> Result<vk::RenderPass, ShadowMapError> {
    let attachments = [*attachment];
    let depth_reference = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };
    let subpass = vk::SubpassDescription {
        pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
        color_attachment_count: 0,
        p_color_attachments: std::ptr::null(),
        p_depth_stencil_attachment: &depth_reference,
        ..Default::default()
    };
    let dependencies = [
        // 外部 → 深度写入
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dependency_flags: vk::DependencyFlags::BY_REGION,
            ..Default::default()
        },
        // 深度写入 → 片元着色器采样
        vk::SubpassDependency {
            src_subpass: 0,
            dst_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
            src_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            dependency_flags: vk::DependencyFlags::BY_REGION,
            ..Default::default()
        },
    ];
    let create_info = vk::RenderPassCreateInfo {
        attachment_count: 1,
        p_attachments: attachments.as_ptr(),
        subpass_count: 1,
        p_subpasses: &subpass,
        dependency_count: dependencies.len() as u32,
        p_dependencies: dependencies.as_ptr(),
        ..Default::default()
    };
    device
        .create_render_pass(&create_info, None)
        .map_err(|e| ShadowMapError::new(format!("创建阴影 render pass 失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hezhou_render_pipeline::RenderQualityPreset;

    #[test]
    fn error_display() {
        let err = ShadowMapError::new("测试错误");
        assert!(err.to_string().contains("测试错误"));
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn find_memory_type_rejects_unknown_bits() {
        let properties = vk::PhysicalDeviceMemoryProperties {
            memory_type_count: 1,
            memory_types: [vk::MemoryType {
                property_flags: vk::MemoryPropertyFlags::HOST_VISIBLE,
                heap_index: 0,
            }; 32],
            memory_heap_count: 1,
            memory_heaps: [vk::MemoryHeap {
                size: 1024,
                flags: vk::MemoryHeapFlags::empty(),
            }; 16],
        };
        // 只申请 bit 31 → 不匹配
        assert!(find_memory_type(
            &properties,
            1 << 31,
            vk::MemoryPropertyFlags::DEVICE_LOCAL
        )
        .is_err());
        // bit 0 + 目标属性不满足 → 不匹配
        assert!(find_memory_type(
            &properties,
            1,
            vk::MemoryPropertyFlags::DEVICE_LOCAL
        )
        .is_err());
        // bit 0 + HOST_VISIBLE → 匹配
        assert_eq!(
            find_memory_type(&properties, 1, vk::MemoryPropertyFlags::HOST_VISIBLE).unwrap(),
            0
        );
    }

    #[test]
    fn framebuffer_lookup_is_bounds_checked() {
        // 不创建真实 GPU 资源，只验证 inspect 逻辑的边界
        let resources = ShadowMapResources {
            extent: vk::Extent2D {
                width: 1024,
                height: 1024,
            },
            format: vk::Format::D32_SFLOAT,
            cascade_count: 3,
            image: vk::Image::null(),
            memory: vk::DeviceMemory::null(),
            array_view: vk::ImageView::null(),
            layer_views: vec![vk::ImageView::null(); 3],
            sampler: vk::Sampler::null(),
            render_pass: vk::RenderPass::null(),
            framebuffers: vec![vk::Framebuffer::null(); 3],
        };
        assert!(resources.framebuffer(0).is_some());
        assert!(resources.framebuffer(2).is_some());
        assert!(resources.framebuffer(3).is_none());
        let info = resources.descriptor_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        assert_eq!(info.image_layout, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        assert_eq!(info.image_view, resources.array_view);
    }

    /// GPU 冒烟测试 — 需要真实 Vulkan 设备，默认 ignored
    ///
    /// 运行: `cargo test -p hezhou-rhi-vulkan -- --ignored create_shadow_map_on_gpu`
    #[test]
    #[ignore = "需要 Vulkan 设备（GPU 测试）；无驱动/服务器环境请保持 ignored"]
    fn create_shadow_map_on_gpu() {
        unsafe {
            let entry = match ash::Entry::load() {
                Ok(entry) => entry,
                Err(err) => {
                    eprintln!("跳过 GPU 测试: 无法加载 Vulkan loader: {err}");
                    return;
                }
            };
            let app_info = vk::ApplicationInfo {
                p_application_name: b"HezhouShadowMapTest\0".as_ptr() as *const i8,
                application_version: vk::make_api_version(0, 0, 2, 0),
                p_engine_name: b"Hezhou\0".as_ptr() as *const i8,
                engine_version: vk::make_api_version(0, 0, 2, 0),
                api_version: vk::make_api_version(0, 1, 1, 0),
                ..Default::default()
            };
            let instance = match entry.create_instance(
                &vk::InstanceCreateInfo {
                    p_application_info: &app_info,
                    ..Default::default()
                },
                None,
            ) {
                Ok(instance) => instance,
                Err(err) => {
                    eprintln!("跳过 GPU 测试: 创建 Vulkan instance 失败: {err}");
                    return;
                }
            };
            let physical_devices = match instance.enumerate_physical_devices() {
                Ok(devices) if !devices.is_empty() => devices,
                Ok(_) => {
                    eprintln!("跳过 GPU 测试: 没有 Vulkan 物理设备");
                    instance.destroy_instance(None);
                    return;
                }
                Err(err) => {
                    eprintln!("跳过 GPU 测试: 枚举物理设备失败: {err}");
                    instance.destroy_instance(None);
                    return;
                }
            };
            let physical_device = physical_devices[0];
            let properties = instance.get_physical_device_properties(physical_device);
            let families = instance.get_physical_device_queue_family_properties(physical_device);
            let family = match families
                .iter()
                .position(|family| family.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            {
                Some(family) => family as u32,
                None => {
                    eprintln!("跳过 GPU 测试: 物理设备没有图形队列");
                    instance.destroy_instance(None);
                    return;
                }
            };
            let priorities = [1.0f32];
            let device = match instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo {
                    queue_create_info_count: 1,
                    p_queue_create_infos: &vk::DeviceQueueCreateInfo {
                        queue_family_index: family,
                        queue_count: 1,
                        p_queue_priorities: priorities.as_ptr(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                None,
            ) {
                Ok(device) => device,
                Err(err) => {
                    eprintln!("跳过 GPU 测试: 创建 Vulkan device 失败: {err}");
                    instance.destroy_instance(None);
                    return;
                }
            };

            let settings =
                RenderSettings::for_preset(RenderQualityPreset::High);
            let mut shadow = ShadowMapResources::create_from_settings(
                &instance,
                &device,
                physical_device,
                &settings,
            )
            .expect("高配应启用阴影")
            .expect("创建阴影贴图失败");

            assert_eq!(shadow.cascade_count, 3);
            assert_eq!(shadow.layer_views.len(), 3);
            assert_eq!(shadow.framebuffers.len(), 3);
            assert_eq!(shadow.extent.width, 2048);
            assert_eq!(shadow.format, vk::Format::D32_SFLOAT);
            let info = shadow.descriptor_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            assert_eq!(info.image_view, shadow.array_view);
            let device_name = properties
                .device_name_as_c_str()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "<unknown>".to_string());
            eprintln!(
                "GPU 阴影贴图创建成功: device={device_name} {}x{} cascades={} format={:?}",
                shadow.extent.width, shadow.extent.height, shadow.cascade_count, shadow.format
            );

            // 约定：销毁前必须 GPU 空闲
            device.device_wait_idle().expect("device_wait_idle");
            shadow.destroy(&device);
            device.destroy_device(None);
            instance.destroy_instance(None);
        }
    }
}
