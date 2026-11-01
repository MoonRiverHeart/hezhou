//! PBR/Shadow/Post + 天空盒/outline/Gizmo + 蒙皮 draw 的 GPU 冒烟测试（标记 ignored）
//!
//! 需要真实 Vulkan 设备（本机独显/集显均可），不进入常规 CI：
//! `cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored`
//!
//! 覆盖:
//! 1. High 档：完整绘制 Shadow → Game(PBR/HDR) → 天空盒 → 实体 → 选中 outline →
//!    3D Gizmo → 合成 skinned mesh → Bloom → Tonemap，真实提交
//! 2. Low 档：无阴影/无 Bloom 的同一套视觉元素
//! 3. Resize：在两种视口尺寸下分别创建/录制 PBR 通道（模拟窗口 resize 后重建）
//!
//! 若环境无 Vulkan loader/设备，测试打印跳过原因并视为通过（与其他 GPU 测试一致）。

use ash::vk;

use hezhou_render_pipeline::{
    CameraParams, PbrMaterial, RenderQualityPreset, RenderSettings, ShadowFrameData, ViewportExtent,
};
use hezhou_rhi_vulkan::{
    create_host_visible_buffer, create_render_target, destroy_render_target, GpuTarget,
    LegacyPushBase, MeshDraw, PbrDrawItem, PbrForwardPass, SkinnedPipelineState, LEGACY_FAR,
    LEGACY_FOV_DEGREES, LEGACY_NEAR,
};
use hezhou_rhi_vulkan::skinned_mesh::{
    identity_palette_bytes, skinned_vertex_bytes, MAX_PALETTE_BYTES, MAX_PALETTE_JOINTS,
};

const DEFAULT_EXTENT: ViewportExtent = ViewportExtent {
    width: 320,
    height: 180,
};
const RESIZED_EXTENT: ViewportExtent = ViewportExtent {
    width: 640,
    height: 360,
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

    unsafe fn destroy(self) {
        self.device.destroy_command_pool(self.command_pool, None);
        self.device.destroy_device(None);
        self.instance.destroy_instance(None);
        let _ = &self.entry;
    }
}

/// 创建 LDR 输出目标 + D32 深度目标
unsafe fn create_targets(
    headless: &HeadlessVulkan,
    extent: ViewportExtent,
) -> Result<(GpuTarget, GpuTarget), String> {
    let ldr = create_render_target(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        extent,
        vk::Format::B8G8R8A8_UNORM,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        vk::ImageAspectFlags::COLOR,
    )?;
    let depth = create_render_target(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        extent,
        vk::Format::D32_SFLOAT,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::ImageAspectFlags::DEPTH,
    )?;
    Ok((ldr, depth))
}

/// 创建 3 顶点（stride 32）的顶点缓冲，用于实体/outline draw
unsafe fn create_test_vertex_buffer(
    headless: &HeadlessVulkan,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let vertex_bytes: [f32; 24] = [
        // position(3) normal(3) uv(2)
        0.0, 0.5, 0.0, 0.0, 0.0, 1.0, 0.5, 0.5, -0.5, -0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5,
        -0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
    ];
    upload_bytes(
        headless,
        &vertex_bytes,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )
}

/// 创建 3D Gizmo 顶点缓冲（与 renderer `gizmo_vertex_data` 一致）
unsafe fn create_gizmo_buffer(
    headless: &HeadlessVulkan,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let vertices = hezhou_rhi_vulkan::pbr_forward::gizmo_vertex_data();
    upload_bytes(headless, &vertices, vk::BufferUsageFlags::VERTEX_BUFFER)
}

unsafe fn upload_bytes<T>(
    headless: &HeadlessVulkan,
    values: &[T],
    usage: vk::BufferUsageFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let bytes: &[u8] = std::slice::from_raw_parts(
        values.as_ptr() as *const u8,
        std::mem::size_of_val(values),
    );
    let (buffer, memory) = create_host_visible_buffer(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        bytes.len() as u64,
        usage,
    )?;
    let ptr = headless
        .device
        .map_memory(memory, 0, bytes.len() as u64, vk::MemoryMapFlags::empty())
        .map_err(|e| format!("map 顶点缓冲失败: {e}"))?;
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
    headless.device.unmap_memory(memory);
    Ok((buffer, memory))
}

fn camera() -> CameraParams {
    CameraParams {
        position: [0.0, 2.0, 5.0],
        yaw: 0.2,
        pitch: -0.3,
        fov: LEGACY_FOV_DEGREES.to_radians(),
        near: LEGACY_NEAR,
        far: LEGACY_FAR,
    }
}

/// 录制一帧：Shadow（可选）→ HDR pass（天空/实体/outline/Gizmo/合成蒙皮）→ Bloom/Tonemap
unsafe fn record_frame(
    headless: &HeadlessVulkan,
    pass: &PbrForwardPass,
    skinned: &SkinnedPipelineState,
    settings: &RenderSettings,
    extent: ViewportExtent,
    vertex_buffer: vk::Buffer,
    gizmo_buffer: vk::Buffer,
    ldr_target: &GpuTarget,
) -> Result<(), String> {
    let camera = camera();
    let aspect = extent.width as f32 / extent.height as f32;
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

    let model = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    // === 每实体材质（task-22）：3 个实体分别使用不同材质 ===
    // 1) 红色金属（高 metallic / 低 roughness）
    // 2) 蓝色粗糙介电质
    // 3) 无覆盖 → 回退 settings.default_material
    let per_entity_materials: [PbrMaterial; 3] = [
        PbrMaterial::metal([0.9, 0.1, 0.1], 0.15),
        PbrMaterial::dielectric([0.1, 0.2, 0.9], 0.85),
        settings.default_material.sanitized(),
    ];
    let mut draws: Vec<MeshDraw> = Vec::new();
    let mut pbr_items: Vec<PbrDrawItem> = Vec::new();
    for (index, material) in per_entity_materials.iter().enumerate() {
        let mut mesh_model = model;
        // 横向错开，避免三个三角形完全重叠
        mesh_model[3][0] = (index as f32 - 1.0) * 1.2;
        let mesh = MeshDraw::primitive(0, 3, mesh_model);
        draws.push(mesh);
        pbr_items.push(PbrDrawItem::new(mesh, *material));
    }
    assert_ne!(
        pbr_items[0].push_constant().material_params,
        pbr_items[1].push_constant().material_params,
        "两个实体的材质 push constant 必须不同"
    );

    // outline：放大 1.05 的同一 mesh
    let outline_model = [
        [1.05, 0.0, 0.0, 0.0],
        [0.0, 1.05, 0.0, 0.0],
        [0.0, 0.0, 1.05, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let outlines = [MeshDraw::primitive(0, 3, outline_model)];
    let base = LegacyPushBase {
        viewport: [extent.width as f32, extent.height as f32],
        camera_pos: camera.position,
        camera_yaw: camera.yaw,
        camera_pitch: camera.pitch,
    };

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

    // 1. 阴影
    if let Some(shadow_frame) = &shadow {
        pass.record_shadow(cmd, shadow_frame, vertex_buffer, vertex_buffer, &draws);
    }
    // 2. HDR 主 pass：天空盒 → 每实体材质实体 → outline → Gizmo → 合成蒙皮
    pass.begin_scene(cmd, [0.05, 0.05, 0.1, 1.0]);
    pass.record_sky(cmd, [0.16, 0.32, 0.62], [0.72, 0.82, 0.92]);
    pass.record_scene_draws_with_materials(
        cmd,
        vertex_buffer,
        vertex_buffer,
        &pbr_items,
    );
    pass.record_outline(cmd, None, None, vertex_buffer, vertex_buffer, &outlines, &base);
    pass.record_gizmo(cmd, None, None, gizmo_buffer, [0.0, 0.0, 0.0], &base);
    skinned.update_palette_animated(0.5)?;
    let skinned_model = hezhou_rhi_vulkan::pbr_forward::synthetic_skinned_model_matrix(
        camera.position,
        camera.yaw,
        camera.pitch,
        0.5,
    );
    skinned.record_synthetic(cmd, skinned_model, &base);
    pass.end_scene(cmd);
    // 3. Bloom（可选）→ Tonemap
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
    headless
        .device
        .free_command_buffers(headless.command_pool, &[cmd]);
    Ok(())
}

struct FrameResources {
    ldr: GpuTarget,
    depth: GpuTarget,
    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    gizmo_buffer: vk::Buffer,
    gizmo_memory: vk::DeviceMemory,
}

unsafe fn create_frame_resources(
    headless: &HeadlessVulkan,
    extent: ViewportExtent,
) -> Result<FrameResources, String> {
    let (ldr, depth) = create_targets(headless, extent)?;
    let (vertex_buffer, vertex_memory) = create_test_vertex_buffer(headless)?;
    let (gizmo_buffer, gizmo_memory) = create_gizmo_buffer(headless)?;
    Ok(FrameResources {
        ldr,
        depth,
        vertex_buffer,
        vertex_memory,
        gizmo_buffer,
        gizmo_memory,
    })
}

unsafe fn destroy_frame_resources(headless: &HeadlessVulkan, resources: FrameResources) {
    headless.device.destroy_buffer(resources.vertex_buffer, None);
    headless.device.free_memory(resources.vertex_memory, None);
    headless.device.destroy_buffer(resources.gizmo_buffer, None);
    headless.device.free_memory(resources.gizmo_memory, None);
    destroy_render_target(&headless.device, resources.ldr);
    destroy_render_target(&headless.device, resources.depth);
}

/// 创建 PBR 通道 + 蒙皮 pipeline（供 High/Low/resize 三组测试复用）
unsafe fn create_pipeline_pair(
    headless: &HeadlessVulkan,
    extent: ViewportExtent,
    settings: &RenderSettings,
    depth_view: vk::ImageView,
    ldr_view: vk::ImageView,
) -> Result<(PbrForwardPass, SkinnedPipelineState), String> {
    let pass = PbrForwardPass::create(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        headless.command_pool,
        headless.queue,
        extent,
        depth_view,
        ldr_view,
        vk::Format::B8G8R8A8_UNORM,
        settings,
    )?;
    let skinned = SkinnedPipelineState::create(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        headless.command_pool,
        headless.queue,
        pass.hdr_render_pass(),
        extent,
    )?;
    Ok((pass, skinned))
}

#[test]
#[ignore = "需要 Vulkan 设备（GPU 测试）；运行: cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored"]
fn pbr_forward_gpu_high_quality_draw_path() {
    unsafe {
        let Some(headless) = HeadlessVulkan::create() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let resources = create_frame_resources(&headless, DEFAULT_EXTENT)?;
            let settings = RenderSettings::for_preset(RenderQualityPreset::High);
            let (pass, skinned) = create_pipeline_pair(
                &headless,
                DEFAULT_EXTENT,
                &settings,
                resources.depth.view,
                resources.ldr.view,
            )?;
            assert!(pass.shadow_active(), "High 档应启用阴影 pass");
            assert!(pass.bloom_active(), "High 档应启用 Bloom pass");
            assert!(pass.sky_pipeline_ready(), "天空盒 pipeline 应创建");
            assert!(pass.outline_pipeline_ready(), "outline/Gizmo pipeline 应创建");
            assert_eq!(pass.settings().quality.shadow_cascade_count, 3);
            assert_eq!(skinned.skinned_vertex_count(), 8);
            assert_eq!(skinned.skinned_index_count(), 36);

            record_frame(
                &headless,
                &pass,
                &skinned,
                &settings,
                DEFAULT_EXTENT,
                resources.vertex_buffer,
                resources.gizmo_buffer,
                &resources.ldr,
            )?;

            // 清理顺序：先蒙皮（引用 hdr render pass）后 PBR 通道
            skinned.destroy();
            pass.destroy();
            destroy_frame_resources(&headless, resources);
            eprintln!(
                "GPU PBR 冒烟通过: High 档 Shadow→Game(PBR/HDR)+天空盒+outline+Gizmo+合成蒙皮→Bloom→Tonemap"
            );
            Ok(())
        })();
        headless.destroy();
        if let Err(err) = result {
            panic!("GPU PBR High 档冒烟失败: {err}");
        }
    }
}

#[test]
#[ignore = "需要 Vulkan 设备（GPU 测试）；运行: cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored"]
fn pbr_forward_gpu_low_quality_draw_path() {
    unsafe {
        let Some(headless) = HeadlessVulkan::create() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let resources = create_frame_resources(&headless, DEFAULT_EXTENT)?;
            let settings = RenderSettings::for_preset(RenderQualityPreset::Low);
            let (pass, skinned) = create_pipeline_pair(
                &headless,
                DEFAULT_EXTENT,
                &settings,
                resources.depth.view,
                resources.ldr.view,
            )?;
            assert!(!pass.shadow_active(), "Low 档不应启用阴影");
            assert!(!pass.bloom_active(), "Low 档不应启用 Bloom");
            assert!(pass.sky_pipeline_ready());
            assert!(pass.outline_pipeline_ready());

            record_frame(
                &headless,
                &pass,
                &skinned,
                &settings,
                DEFAULT_EXTENT,
                resources.vertex_buffer,
                resources.gizmo_buffer,
                &resources.ldr,
            )?;

            skinned.destroy();
            pass.destroy();
            destroy_frame_resources(&headless, resources);
            eprintln!(
                "GPU PBR 冒烟通过: Low 档 PBR+天空盒+outline+Gizmo+合成蒙皮→Tonemap（无阴影/无 Bloom）"
            );
            Ok(())
        })();
        headless.destroy();
        if let Err(err) = result {
            panic!("GPU PBR Low 档冒烟失败: {err}");
        }
    }
}

#[test]
#[ignore = "需要 Vulkan 设备（GPU 测试）；运行: cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored"]
fn pbr_forward_gpu_two_real_skinned_meshes() {
    unsafe {
        let Some(headless) = HeadlessVulkan::create() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let resources = create_frame_resources(&headless, DEFAULT_EXTENT)?;
            let settings = RenderSettings::for_preset(RenderQualityPreset::High);
            let (pass, skinned) = create_pipeline_pair(
                &headless,
                DEFAULT_EXTENT,
                &settings,
                resources.depth.view,
                resources.ldr.view,
            )?;
            assert!(pass.sky_pipeline_ready());

            // === 2 个真实蒙皮网格（不同顶点/索引/palette） ===
            let (cube_vertices, cube_indices) =
                hezhou_rhi_vulkan::pbr_forward::synthetic_skinned_cube();
            // 网格 A：8 顶点 / 36 索引，2 关节 palette
            let mesh_a = (cube_vertices.clone(), cube_indices.clone());
            // 网格 B：两个 cube 合并（16 顶点 / 72 索引），3 关节 palette
            let mut vertices_b = cube_vertices.clone();
            for vertex in &mut vertices_b {
                vertex.position[1] += 1.5;
            }
            let mut mesh_b_vertices = cube_vertices.clone();
            mesh_b_vertices.extend(vertices_b);
            let mut mesh_b_indices = cube_indices.clone();
            mesh_b_indices.extend(cube_indices.iter().map(|index| index + 8));
            let mesh_b = (mesh_b_vertices, mesh_b_indices);
            assert_ne!(mesh_a.0.len(), mesh_b.0.len(), "两个网格顶点数不同");
            assert_ne!(mesh_a.1.len(), mesh_b.1.len(), "两个网格索引数不同");

            let (vb_a, vb_a_mem, ib_a, ib_a_mem) = create_skinned_mesh_buffers(
                &headless,
                &mesh_a.0,
                &mesh_a.1,
            )?;
            let (vb_b, vb_b_mem, ib_b, ib_b_mem) = create_skinned_mesh_buffers(
                &headless,
                &mesh_b.0,
                &mesh_b.1,
            )?;
            let (palette_a, palette_a_mem) =
                create_palette_buffer(&headless, &two_joint_palette())?;
            let (palette_b, palette_b_mem) =
                create_palette_buffer(&headless, &three_joint_palette())?;

            let base = LegacyPushBase {
                viewport: [DEFAULT_EXTENT.width as f32, DEFAULT_EXTENT.height as f32],
                camera_pos: camera().position,
                camera_yaw: camera().yaw,
                camera_pitch: camera().pitch,
            };
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
                    image: resources.ldr.image,
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

            // 背景实体（可选）+ 两个真实蒙皮网格
            pass.begin_scene(cmd, [0.05, 0.05, 0.1, 1.0]);
            pass.record_sky(cmd, [0.16, 0.32, 0.62], [0.72, 0.82, 0.92]);
            let mesh = MeshDraw::primitive(0, 3, [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]);
            pass.record_scene_draws_with_materials(
                cmd,
                resources.vertex_buffer,
                resources.vertex_buffer,
                &[PbrDrawItem::new(mesh, settings.default_material.sanitized())],
            );
            let model_a = [
                [0.5, 0.0, 0.0, 0.0],
                [0.0, 0.5, 0.0, 0.0],
                [0.0, 0.0, 0.5, 0.0],
                [-1.2, 0.0, 0.0, 1.0],
            ];
            let model_b = [
                [0.4, 0.0, 0.0, 0.0],
                [0.0, 0.4, 0.0, 0.0],
                [0.0, 0.0, 0.4, 0.0],
                [1.2, 0.0, 0.0, 1.0],
            ];
            skinned.record_mesh(
                cmd, 101, vb_a, ib_a, mesh_a.1.len() as u32, palette_a, model_a, &base,
            );
            skinned.record_mesh(
                cmd, 102, vb_b, ib_b, mesh_b.1.len() as u32, palette_b, model_b, &base,
            );
            pass.end_scene(cmd);
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
            headless
                .device
                .free_command_buffers(headless.command_pool, &[cmd]);
            assert_eq!(
                skinned.entity_palette_set_count(),
                2,
                "两个真实蒙皮网格应各有 palette descriptor"
            );

            // 清理
            skinned.destroy();
            pass.destroy();
            for (buffer, memory) in [
                (vb_a, vb_a_mem),
                (ib_a, ib_a_mem),
                (vb_b, vb_b_mem),
                (ib_b, ib_b_mem),
                (palette_a, palette_a_mem),
                (palette_b, palette_b_mem),
            ] {
                headless.device.destroy_buffer(buffer, None);
                headless.device.free_memory(memory, None);
            }
            destroy_frame_resources(&headless, resources);
            eprintln!(
                "GPU 真实蒙皮网格冒烟通过: 2 个网格（8v/36i+2 关节、16v/72i+3 关节）真实绘制"
            );
            Ok(())
        })();
        headless.destroy();
        if let Err(err) = result {
            panic!("GPU 真实蒙皮网格冒烟失败: {err}");
        }
    }
}

/// 创建真实蒙皮网格的 vertex/index buffer（host visible）
unsafe fn create_skinned_mesh_buffers(
    headless: &HeadlessVulkan,
    vertices: &[hezhou_animation::SkinnedVertex],
    indices: &[u32],
) -> Result<
    (vk::Buffer, vk::DeviceMemory, vk::Buffer, vk::DeviceMemory),
    String,
> {
    let vertex_bytes = skinned_vertex_bytes(vertices);
    let (vertex_buffer, vertex_memory) = upload_raw(
        headless,
        &vertex_bytes,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )?;
    let index_bytes: Vec<u8> = indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect();
    let (index_buffer, index_memory) = upload_raw(
        headless,
        &index_bytes,
        vk::BufferUsageFlags::INDEX_BUFFER,
    )?;
    Ok((vertex_buffer, vertex_memory, index_buffer, index_memory))
}

/// 创建 16KB palette buffer，并写入给定矩阵（其余填单位矩阵）
unsafe fn create_palette_buffer(
    headless: &HeadlessVulkan,
    matrices: &[[[f32; 4]; 4]],
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let identity = identity_palette_bytes();
    let mut bytes = Vec::with_capacity(MAX_PALETTE_BYTES);
    for _ in 0..MAX_PALETTE_JOINTS {
        bytes.extend_from_slice(&identity);
    }
    // 逐个矩阵写入（palette 顺序即 joint 顺序）
    for (matrix_index, matrix) in matrices.iter().take(MAX_PALETTE_JOINTS).enumerate() {
        let mut offset = matrix_index * 64;
        for column in matrix {
            for value in column {
                bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                offset += 4;
            }
        }
    }
    upload_raw(headless, &bytes, vk::BufferUsageFlags::STORAGE_BUFFER)
}

unsafe fn upload_raw(
    headless: &HeadlessVulkan,
    bytes: &[u8],
    usage: vk::BufferUsageFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let (buffer, memory) = create_host_visible_buffer(
        &headless.instance,
        &headless.device,
        headless.physical_device,
        bytes.len() as u64,
        usage,
    )?;
    let ptr = headless
        .device
        .map_memory(memory, 0, bytes.len() as u64, vk::MemoryMapFlags::empty())
        .map_err(|e| format!("map 上传 buffer 失败: {e}"))?;
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
    headless.device.unmap_memory(memory);
    Ok((buffer, memory))
}

/// 2 关节 palette：单位阵 + 绕 Y 轴旋转 0.3（列主序 mat4）
fn two_joint_palette() -> Vec<[[f32; 4]; 4]> {
    let c = 0.3f32.cos();
    let s = 0.3f32.sin();
    vec![
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        [
            [c, 0.0, -s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    ]
}

/// 3 关节 palette：单位阵 + 两个不同旋转/平移
fn three_joint_palette() -> Vec<[[f32; 4]; 4]> {
    let mut palette = two_joint_palette();
    palette.push([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.5, 0.0, 1.0],
    ]);
    palette
}

#[test]
#[ignore = "需要 Vulkan 设备（GPU 测试）；运行: cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored"]
fn pbr_forward_gpu_resize_recreate() {
    unsafe {
        let Some(headless) = HeadlessVulkan::create() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let settings = RenderSettings::for_preset(RenderQualityPreset::High);
            for extent in [DEFAULT_EXTENT, RESIZED_EXTENT] {
                let resources = create_frame_resources(&headless, extent)?;
                let (pass, skinned) = create_pipeline_pair(
                    &headless,
                    extent,
                    &settings,
                    resources.depth.view,
                    resources.ldr.view,
                )?;
                assert_eq!(pass.extent.width, extent.width);
                assert_eq!(pass.extent.height, extent.height);
                record_frame(
                    &headless,
                    &pass,
                    &skinned,
                    &settings,
                    extent,
                    resources.vertex_buffer,
                    resources.gizmo_buffer,
                    &resources.ldr,
                )?;
                skinned.destroy();
                pass.destroy();
                destroy_frame_resources(&headless, resources);
                eprintln!(
                    "GPU PBR resize 冒烟通过: {}x{} 创建/录制/销毁",
                    extent.width, extent.height
                );
            }
            Ok(())
        })();
        headless.destroy();
        if let Err(err) = result {
            panic!("GPU PBR resize 冒烟失败: {err}");
        }
    }
}
