//! PBR/Shadow/Post + 蒙皮 pipeline 的 GPU 冒烟测试（标记 ignored）
//!
//! 这些测试需要真实 Vulkan 设备（本机 NV/AMD/Intel 均可），不进入常规 CI：
//! `cargo test -p hezhou-rhi-vulkan -- --ignored pbr_forward_gpu`
//!
//! 测试内容:
//! 1. High 质量档：创建 PBR/Shadow/Bloom/Tonemap 全部资源 + 蒙皮 pipeline，
//!    上传 joint palette，并真实录制 Shadow→Game(PBR)→Bloom→Tonemap 命令后提交
//! 2. Low 质量档：无阴影/无 Bloom 的 PBR 路径录制
//!
//! 若环境无 Vulkan loader/设备，测试打印跳过原因并视为通过（与其他 GPU 测试一致）。

use ash::vk;

use hezhou_render_pipeline::{
    CameraParams, RenderQualityPreset, RenderSettings, ShadowFrameData, ViewportExtent,
};
use hezhou_rhi_vulkan::{
    create_host_visible_buffer, create_render_target, destroy_render_target, GpuTarget, MeshDraw,
    PbrForwardPass, SkinnedPipelineState, LEGACY_FAR, LEGACY_FOV_DEGREES, LEGACY_NEAR,
};

const TEST_EXTENT: ViewportExtent = ViewportExtent {
    width: 320,
    height: 180,
};

struct HeadlessVulkan {
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
}

impl HeadlessVulkan {
    unsafe fn create() -> Option<Self> {
        let entry = match ash::Entry::load() {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("跳过 GPU 测试: 无法加载 Vulkan loader: {err}");
                return None;
            }
        };
        let app_info = vk::ApplicationInfo {
            p_application_name: b"HezhouPbrForwardTest\0".as_ptr() as *const i8,
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
                eprintln!("跳过 GPU 测试: 创建 instance 失败: {err}");
                return None;
            }
        };
        let physical_devices = match instance.enumerate_physical_devices() {
            Ok(devices) if !devices.is_empty() => devices,
            Ok(_) => {
                eprintln!("跳过 GPU 测试: 没有 Vulkan 物理设备");
                instance.destroy_instance(None);
                return None;
            }
            Err(err) => {
                eprintln!("跳过 GPU 测试: 枚举物理设备失败: {err}");
                instance.destroy_instance(None);
                return None;
            }
        };
        let physical_device = physical_devices[0];
        let families = instance.get_physical_device_queue_family_properties(physical_device);
        let Some(family) = families
            .iter()
            .position(|family| family.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        else {
            eprintln!("跳过 GPU 测试: 物理设备没有图形队列");
            instance.destroy_instance(None);
            return None;
        };
        let priorities = [1.0f32];
        let device = match instance.create_device(
            physical_device,
            &vk::DeviceCreateInfo {
                queue_create_info_count: 1,
                p_queue_create_infos: &vk::DeviceQueueCreateInfo {
                    queue_family_index: family as u32,
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
                eprintln!("跳过 GPU 测试: 创建 device 失败: {err}");
                instance.destroy_instance(None);
                return None;
            }
        };
        let queue = device.get_device_queue(family as u32, 0);
        let command_pool = match device.create_command_pool(
            &vk::CommandPoolCreateInfo {
                queue_family_index: family as u32,
                ..Default::default()
            },
            None,
        ) {
            Ok(pool) => pool,
            Err(err) => {
                eprintln!("跳过 GPU 测试: 创建 command pool 失败: {err}");
                device.destroy_device(None);
                instance.destroy_instance(None);
                return None;
            }
        };
        Some(Self {
            entry,
            instance,
            device,
            physical_device,
            queue,
            command_pool,
        })
    }

    unsafe fn destroy(mut self) {
        self.device.destroy_command_pool(self.command_pool, None);
        self.device.destroy_device(None);
        self.instance.destroy_instance(None);
        let _ = &self.entry;
    }
}

/// 创建 LDR 输出目标 + D32 深度目标
unsafe fn create_targets(headless: &HeadlessVulkan) -> Result<(GpuTarget, GpuTarget), String> {
    let ldr = create_render_target(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        TEST_EXTENT,
        vk::Format::B8G8R8A8_UNORM,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        vk::ImageAspectFlags::COLOR,
    )?;
    let depth = create_render_target(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        TEST_EXTENT,
        vk::Format::D32_SFLOAT,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::ImageAspectFlags::DEPTH,
    )?;
    Ok((ldr, depth))
}

/// 创建 3 顶点（stride 32）的顶点缓冲，用于真实 draw
unsafe fn create_test_vertex_buffer(
    headless: &HeadlessVulkan,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let vertex_bytes: [f32; 24] = [
        // position(3) normal(3) uv(2)
        0.0, 0.5, 0.0, 0.0, 0.0, 1.0, 0.5, 0.5, -0.5, -0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5,
        -0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
    ];
    let size = std::mem::size_of_val(&vertex_bytes) as u64;
    let (buffer, memory) = create_host_visible_buffer(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        size,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )?;
    let ptr = headless
        .device
        .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())
        .map_err(|e| format!("map 顶点缓冲失败: {e}"))?;
    std::ptr::copy_nonoverlapping(
        vertex_bytes.as_ptr() as *const u8,
        ptr as *mut u8,
        size as usize,
    );
    headless.device.unmap_memory(memory);
    Ok((buffer, memory))
}

/// 录制一帧：Shadow → Game(PBR) → Bloom → Tonemap
unsafe fn record_frame(
    headless: &HeadlessVulkan,
    pass: &PbrForwardPass,
    settings: &RenderSettings,
    vertex_buffer: vk::Buffer,
    ldr_target: &GpuTarget,
) -> Result<(), String> {
    let camera = CameraParams {
        position: [0.0, 2.0, 5.0],
        yaw: 0.2,
        pitch: -0.3,
        fov: LEGACY_FOV_DEGREES.to_radians(),
        near: LEGACY_NEAR,
        far: LEGACY_FAR,
    };
    let aspect = TEST_EXTENT.width as f32 / TEST_EXTENT.height as f32;
    let light_direction = [-0.4f32, -1.0, -0.3];

    let shadow = match settings.shadow_config() {
        Some(config) => Some(
            ShadowFrameData::from_camera(&camera, aspect, light_direction, config)
                .map_err(|e| format!("阴影矩阵计算失败: {e}"))?,
        ),
        None => None,
    };

    let view = hezhou_rhi_vulkan::camera_view_matrix(camera.position, camera.yaw, camera.pitch);
    let proj = hezhou_rhi_vulkan::camera_projection_matrix(
        LEGACY_FOV_DEGREES,
        aspect,
        LEGACY_NEAR,
        LEGACY_FAR,
    );
    let light = hezhou_render_pipeline::PbrLight {
        direction: light_direction,
        ..Default::default()
    };
    pass.update_frame_ubo(&light, view, proj, camera.position, shadow.as_ref())?;

    let draws = [MeshDraw::primitive(0, 3, [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])];

    let cmd = headless
        .device
        .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
            command_pool: headless.command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        })
        .map_err(|e| format!("分配 command buffer 失败: {e}"))?[0];
    headless
        .device
        .begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())
        .map_err(|e| format!("begin command buffer 失败: {e}"))?;

    // LDR 目标初始化（renderer 在 draw_frame 中会做同样的 UNDEFINED→COLOR_ATTACHMENT 转换）
    headless.device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[vk::ImageMemoryBarrier {
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: ldr_target.image,
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
        }],
    );

    if let Some(shadow_frame) = &shadow {
        pass.record_shadow(cmd, shadow_frame, vertex_buffer, vertex_buffer, &draws);
    }
    pass.record_scene(
        cmd,
        vertex_buffer,
        vertex_buffer,
        &draws,
        &settings.default_material,
    );
    pass.record_post(cmd);

    headless
        .device
        .end_command_buffer(cmd)
        .map_err(|e| format!("end command buffer 失败: {e}"))?;
    headless
        .device
        .queue_submit(
            headless.queue,
            &[vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &cmd,
                ..Default::default()
            }],
            vk::Fence::null(),
        )
        .map_err(|e| format!("queue submit 失败: {e}"))?;
    headless
        .device
        .queue_wait_idle(headless.queue)
        .map_err(|e| format!("queue wait idle 失败: {e}"))?;
    headless.device.free_command_buffers(headless.command_pool, &[cmd]);
    Ok(())
}

#[test]
#[ignore = "需要 Vulkan 设备（GPU 测试）；运行: cargo test -p hezhou-rhi-vulkan -- --ignored pbr_forward_gpu"]
fn pbr_forward_gpu_high_quality_draw_path() {
    unsafe {
        let Some(headless) = HeadlessVulkan::create() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let (ldr, depth) = create_targets(&headless)?;
            let settings = RenderSettings::for_preset(RenderQualityPreset::High);
            let pass = PbrForwardPass::create(
                &headless.instance,
                &headless.device,
                headless.physical_device,
                headless.command_pool,
                headless.queue,
                TEST_EXTENT,
                depth.view,
                ldr.view,
                vk::Format::B8G8R8A8_UNORM,
                &settings,
            )?;
            assert!(pass.shadow_active(), "High 档应启用阴影 pass");
            assert!(pass.bloom_active(), "High 档应启用 Bloom pass");
            assert_eq!(pass.settings().quality.shadow_cascade_count, 3);

            let skinned = SkinnedPipelineState::create(
                &headless.instance,
                &headless.device,
                headless.physical_device,
                headless.command_pool,
                headless.queue,
                pass.hdr_render_pass(),
                TEST_EXTENT,
            )?;
            assert_ne!(skinned.pipeline(), vk::Pipeline::null());

            // 蒙皮 palette 上传（2 关节 = 128B）
            let palette = hezhou_animation::JointPalette::identity(2);
            let uploaded = skinned.upload_joint_palette(&palette.to_le_bytes())?;
            assert_eq!(uploaded, 128);

            let (vertex_buffer, vertex_memory) = create_test_vertex_buffer(&headless)?;
            record_frame(&headless, &pass, &settings, vertex_buffer, &ldr)?;

            // 清理（调用方保证 GPU 空闲：record_frame 内已 queue_wait_idle）
            pass.destroy();
            skinned.destroy();
            headless.device.destroy_buffer(vertex_buffer, None);
            headless.device.free_memory(vertex_memory, None);
            destroy_render_target(&headless.device, ldr);
            destroy_render_target(&headless.device, depth);
            eprintln!("GPU PBR 冒烟通过: High 档 Shadow→Game(PBR)→Bloom→Tonemap + skinned pipeline 已执行");
            Ok(())
        })();
        headless.destroy();
        if let Err(err) = result {
            panic!("GPU PBR High 档冒烟失败: {err}");
        }
    }
}

#[test]
#[ignore = "需要 Vulkan 设备（GPU 测试）；运行: cargo test -p hezhou-rhi-vulkan -- --ignored pbr_forward_gpu"]
fn pbr_forward_gpu_low_quality_draw_path() {
    unsafe {
        let Some(headless) = HeadlessVulkan::create() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let (ldr, depth) = create_targets(&headless)?;
            let settings = RenderSettings::for_preset(RenderQualityPreset::Low);
            let pass = PbrForwardPass::create(
                &headless.instance,
                &headless.device,
                headless.physical_device,
                headless.command_pool,
                headless.queue,
                TEST_EXTENT,
                depth.view,
                ldr.view,
                vk::Format::B8G8R8A8_UNORM,
                &settings,
            )?;
            assert!(!pass.shadow_active(), "Low 档不应启用阴影");
            assert!(!pass.bloom_active(), "Low 档不应启用 Bloom");

            let (vertex_buffer, vertex_memory) = create_test_vertex_buffer(&headless)?;
            record_frame(&headless, &pass, &settings, vertex_buffer, &ldr)?;

            pass.destroy();
            headless.device.destroy_buffer(vertex_buffer, None);
            headless.device.free_memory(vertex_memory, None);
            destroy_render_target(&headless.device, ldr);
            destroy_render_target(&headless.device, depth);
            eprintln!("GPU PBR 冒烟通过: Low 档 PBR→Tonemap 路径已执行（无阴影/无 Bloom）");
            Ok(())
        })();
        headless.destroy();
        if let Err(err) = result {
            panic!("GPU PBR Low 档冒烟失败: {err}");
        }
    }
}
