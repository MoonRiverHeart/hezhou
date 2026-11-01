//! PBR 材质 core/Project 集成测试（task-26）。
//!
//! 覆盖：默认值、Scene set/get、`Project::snapshot_to_json` roundtrip、
//! 旧工程 JSON（无 `pbr_material` 字段）兼容、Prefab 捕获/实例化保留材质。

use hezhou_core::{Entity, GameState, PbrMaterial, Project, Scene};

#[test]
fn pbr_material_default_values() {
    let material = PbrMaterial::default();
    assert_eq!(material.albedo, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(material.metallic, 0.0);
    assert_eq!(material.roughness, 0.5);
    assert_eq!(material.normal_scale, 1.0);
    assert_eq!(material.ao, 1.0);
    assert_eq!(material.emissive, [0.0, 0.0, 0.0]);
    assert_eq!(material.emissive_strength, 1.0);
    assert!(material.albedo_texture_path.is_none());
}

#[test]
fn scene_set_get_and_sanitize_pbr_material() {
    let mut scene = Scene::new();
    let cube = scene.create_cube();

    assert!(scene.get_entity_pbr_material(cube).is_none());
    let dirty = PbrMaterial {
        albedo: [f32::NAN, 2.0, -1.0, 1.0],
        metallic: 5.0,
        roughness: -2.0,
        normal_scale: f32::INFINITY,
        ao: 0.5,
        emissive: [-1.0, 2.0, f32::NAN],
        emissive_strength: 100.0,
        albedo_texture_path: Some("assets/tex/albedo.png".to_string()),
    };
    assert!(scene.set_entity_pbr_material(cube, Some(dirty)));

    let stored = scene.get_entity_pbr_material(cube).expect("PBR 材质应已保存");
    assert!(stored.albedo[0].is_finite());
    assert_eq!(stored.albedo[1], 1.0);
    assert_eq!(stored.albedo[2], 0.0);
    assert_eq!(stored.metallic, 1.0);
    assert_eq!(stored.roughness, 0.0);
    assert_eq!(stored.normal_scale, 1.0, "∞ 回退默认值");
    assert_eq!(stored.ao, 0.5);
    assert_eq!(stored.emissive[0], 0.0);
    assert_eq!(stored.emissive[1], 2.0);
    assert!(stored.emissive[2].is_finite());
    assert_eq!(stored.emissive_strength, 64.0);
    assert_eq!(
        stored.albedo_texture_path.as_deref(),
        Some("assets/tex/albedo.png")
    );

    // None 清除
    assert!(scene.set_entity_pbr_material(cube, None));
    assert!(scene.get_entity_pbr_material(cube).is_none());
    // 非渲染实体设置失败
    let empty = scene.create_entity();
    assert!(!scene.set_entity_pbr_material(empty, Some(PbrMaterial::default())));
}

#[test]
fn project_snapshot_roundtrip_preserves_pbr_material() {
    let mut scene = Scene::new();
    let cube = scene.create_cube();
    scene.set_entity_name(cube, "PbrCube".to_string());
    let material = PbrMaterial {
        albedo: [0.8, 0.2, 0.1, 1.0],
        metallic: 0.6,
        roughness: 0.3,
        normal_scale: 1.25,
        ao: 0.9,
        emissive: [0.15, 0.0, 0.3],
        emissive_strength: 2.5,
        albedo_texture_path: Some("assets/tex/rust.png".to_string()),
    };
    assert!(scene.set_entity_pbr_material(cube, Some(material.clone())));

    let json = Project::snapshot_to_json(&scene).expect("场景快照应成功");
    assert!(json.contains("pbr_material"), "快照应包含 pbr_material: {json}");
    assert!(json.contains("assets/tex/rust.png"));

    let mut restored = Scene::new();
    Project::restore_from_json(&json, &mut restored).expect("场景快照恢复应成功");
    let restored_entity = restored
        .get_root_entities()
        .first()
        .copied()
        .expect("恢复后应有根实体");
    let restored_material = restored
        .get_entity_pbr_material(restored_entity)
        .expect("恢复后 PBR 材质应存在");
    assert_eq!(restored_material, material);
    assert_eq!(restored.state, GameState::Editing);
}

#[test]
fn old_project_entity_without_pbr_material_is_compatible() {
    // 旧工程实体 JSON（无 pbr_material 字段）应能反序列化，字段为 None。
    let json = r#"{
        "id": 7,
        "name": "OldCube",
        "transform": {
            "position": [0.0, 0.0, 0.0],
            "rotation": [0.0, 0.0, 0.0],
            "scale": [1.0, 1.0, 1.0]
        },
        "mesh_type": "Cube",
        "scripts": [],
        "components": {},
        "visible": true,
        "light": null,
        "parent_id": null,
        "material": null
    }"#;
    let entity: hezhou_core::ProjectEntity =
        serde_json::from_str(json).expect("旧工程实体 JSON 应兼容");
    assert!(entity.pbr_material.is_none());

    // 完整 ProjectFile JSON（旧格式）也可加载（pbr_material 缺省 None）。
    let project_json = r#"{
        "version": "1.0",
        "name": "OldProject",
        "path": ".",
        "created_at": "2026-10-18T00:00:00Z",
        "modified_at": "2026-10-18T00:00:00Z",
        "settings": { "game_width": 1280, "game_height": 720, "fps": 60 },
        "entities": [{
            "id": 1, "name": "OldCube",
            "transform": { "position": [0,0,0], "rotation": [0,0,0], "scale": [1,1,1] },
            "mesh_type": "Cube", "scripts": [], "components": {},
            "visible": true, "light": null, "parent_id": null, "material": null
        }],
        "assets": [],
        "scripts": [],
        "current_state": {
            "selected_entity_id": null,
            "camera_position": [10.0, 10.0, 10.0],
            "camera_rotation": [-30.0, 45.0, 0.0],
            "game_state": "Editing"
        }
    }"#;
    let mut scene = Scene::new();
    Project::restore_from_json(project_json, &mut scene).expect("旧工程 JSON 应可加载");
    let roots = scene.get_root_entities();
    assert_eq!(roots.len(), 1);
    assert!(scene.get_entity_pbr_material(roots[0]).is_none());
}

#[test]
fn prefab_capture_and_instantiate_preserves_pbr_material() {
    let mut scene = Scene::new();
    let source = scene.create_cube();
    scene.set_entity_name(source, "PrefabSource".to_string());
    let material = PbrMaterial {
        albedo: [0.1, 0.9, 0.4, 1.0],
        metallic: 0.2,
        roughness: 0.7,
        ..PbrMaterial::default()
    };
    assert!(scene.set_entity_pbr_material(source, Some(material.clone())));

    let prefab = hezhou_core::Prefab::capture_from_scene(
        &scene,
        source,
        "PbrPrefab",
    )
    .expect("Prefab 捕获应成功");
    let json = prefab.to_json().expect("Prefab 序列化应成功");
    assert!(json.contains("pbr_material"));

    let mut target = Scene::new();
    let instance = target
        .instantiate_prefab(&prefab, None)
        .expect("Prefab 实例化应成功");
    let instantiated = target
        .get_entity_pbr_material(instance.root)
        .expect("实例化后应保留 PBR 材质");
    assert_eq!(instantiated, material);
}

