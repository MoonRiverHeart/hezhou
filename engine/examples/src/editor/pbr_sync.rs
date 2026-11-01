//! 每实体 PBR 材质同步：core `RenderableComponent.pbr_material` → renderer 覆盖表（task-26）。
//!
//! 调用时机：
//! - 编辑器主循环每帧（保证 Inspector 修改后下一帧生效、删除实体后清理覆盖）；
//! - `editor_sync_pbr_materials_now()` FFI（启动自检/编辑后立即同步，返回 renderer 覆盖数）。
//!
//! 限制（0.2）：仅同步 albedo 纹理路径；其他纹理槽未接入。

use std::collections::BTreeSet;

use hezhou_core::{PbrMaterial, RenderableComponent, Scene};
use hezhou_dfx::*;
use hezhou_rhi_vulkan::UIVulkanRenderer;

/// 上一次已同步的实体集合（用于清理已删除/已清除材质的渲染覆盖）。
static mut SYNCED_IDS: Option<BTreeSet<u64>> = None;

/// 同步场景中所有带 PBR 材质的实体到 renderer；返回 renderer 当前材质覆盖数。
pub(crate) fn sync_pbr_materials(scene: &Scene, renderer: &mut UIVulkanRenderer) -> usize {
    let mut current: BTreeSet<u64> = BTreeSet::new();

    for entity in scene.get_all_entities() {
        let Some(renderable) = scene.world.get_component::<RenderableComponent>(entity) else {
            continue;
        };
        let Some(material) = renderable.pbr_material.as_ref() else {
            continue;
        };
        let material: PbrMaterial = material.clone().sanitized();
        renderer.set_entity_pbr_material_params(
            entity.id,
            material.albedo,
            material.metallic,
            material.roughness,
            material.normal_scale,
            material.ao,
            material.emissive,
            material.emissive_strength,
        );
        renderer.set_entity_pbr_texture(entity.id, material.albedo_texture_path.as_deref());
        current.insert(entity.id);
    }

    unsafe {
        let synced = SYNCED_IDS.get_or_insert_with(BTreeSet::new);
        let stale: Vec<u64> = synced.difference(&current).copied().collect();
        for entity_id in stale {
            renderer.remove_entity_pbr_material(entity_id);
        }
        if *synced != current {
            dfx_info!(
                "PBR",
                "每实体材质同步: {} 个实体（renderer 覆盖 {}）",
                current.len(),
                renderer.pbr_material_override_count()
            );
            *synced = current;
        }
    }

    renderer.pbr_material_override_count()
}

/// FFI：立即同步一次并返回 renderer 材质覆盖数（0=场景/renderer 未初始化）。
#[unsafe(no_mangle)]
pub extern "C" fn editor_sync_pbr_materials_now() -> i32 {
    unsafe {
        let (Some(scene_ptr), Some(renderer_ptr)) = (super::SCENE, super::RENDERER) else {
            return 0;
        };
        sync_pbr_materials(&*scene_ptr, &mut *renderer_ptr) as i32
    }
}

