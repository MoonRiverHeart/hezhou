use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hezhou_core::ecs::{DirectionalLightComponent, LocalTransform, RenderableComponent};
use hezhou_core::math::Vec3;
use hezhou_core::{GameState, Project, Scene};

fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target")
        .join("project_roundtrip_tmp");
    let _ = std::fs::create_dir_all(&base);
    base.join(format!("hezhou_{}_{}_{}", tag, std::process::id(), nanos))
}

fn find_entity(scene: &Scene, name: &str) -> u64 {
    for entity in scene.get_root_entities() {
        if let Some(id) = find_entity_recursive(scene, *entity, name) {
            return id;
        }
    }
    panic!("entity not found: {}", name);
}

fn find_entity_recursive(scene: &Scene, entity: hezhou_core::Entity, name: &str) -> Option<u64> {
    if scene.get_entity_name(entity).map(|n| n.as_str()) == Some(name) {
        return Some(entity.id);
    }
    for child in scene.get_children(entity) {
        if let Some(id) = find_entity_recursive(scene, child, name) {
            return Some(id);
        }
    }
    None
}

#[test]
fn project_roundtrip_preserves_entity_kinds_and_properties() {
    let temp_root = unique_temp_dir("project_roundtrip");
    std::fs::create_dir_all(&temp_root).unwrap();

    let mut scene = Scene::new();

    let cube = scene.create_cube();
    scene.set_entity_name(cube, "Cube_A".to_string());
    scene.set_entity_position(cube, Vec3::new(1.0, 2.0, 3.0));
    scene.set_entity_scale(cube, Vec3::new(2.0, 2.0, 2.0));
    scene.set_renderable_visible(cube, false);
    scene.set_renderable_material(cube, Some("Default Material".to_string()));
    scene.attach_script_binding(
        cube,
        "scripts/RotatingEntity.cs".to_string(),
        "RotatingEntity".to_string(),
    );
    scene.set_script_binding_enabled(cube, 0, false);

    let empty = scene.create_entity();
    scene.set_entity_name(empty, "Empty_A".to_string());
    scene.set_entity_position(empty, Vec3::new(-1.0, 0.5, 0.0));
    scene.set_parent(empty, Some(cube));

    let light = scene.create_directional_light();
    scene.set_entity_name(light, "Sun".to_string());
    scene.set_light_direction(light, Vec3::new(0.2, -1.0, 0.3));
    scene.set_light_color(light, Vec3::new(1.0, 0.8, 0.6));
    scene.set_light_intensity(light, 2.5);

    let mut project = Project::create_new("RoundtripProject".to_string(), temp_root.clone())
        .expect("create project");
    project.sync_from_scene(&scene);
    project.save().expect("save project");

    let loaded = Project::load(project.get_path().clone()).expect("load project dir");
    assert!(loaded.is_loaded());
    assert_eq!(loaded.get_name(), "RoundtripProject");
    assert_eq!(loaded.get_entity_count(), 3);

    // 目标场景先放一个实体，确认加载会替换而不是追加。
    let mut target = Scene::new();
    target.create_cube();
    loaded.sync_to_scene(&mut target);

    assert_eq!(target.entity_count(), 3);
    assert_eq!(target.get_root_entities().len(), 2);
    assert_eq!(target.state, GameState::Editing);

    let cube_id = find_entity(&target, "Cube_A");
    let cube_entity = target
        .get_root_entities()
        .iter()
        .copied()
        .find(|e| e.id == cube_id)
        .unwrap();
    let transform = target
        .world
        .get_component::<LocalTransform>(cube_entity)
        .expect("cube transform");
    assert!((transform.position.x - 1.0).abs() < 0.001);
    assert!((transform.position.y - 2.0).abs() < 0.001);
    assert!((transform.position.z - 3.0).abs() < 0.001);
    assert!((transform.scale.x - 2.0).abs() < 0.001);

    let renderable = target
        .world
        .get_component::<RenderableComponent>(cube_entity)
        .expect("cube renderable");
    assert_eq!(renderable.mesh_path, "builtin://cube");
    assert!(!renderable.visible);
    assert_eq!(renderable.material_path.as_deref(), Some("Default Material"));
    assert_eq!(target.get_script_binding_count(cube_entity), 1);
    assert!(!target.get_script_binding(cube_entity, 0).unwrap().enabled);

    let empty_id = find_entity(&target, "Empty_A");
    let empty_entity = hezhou_core::Entity::new(empty_id);
    assert!(target
        .world
        .get_component::<RenderableComponent>(empty_entity)
        .is_none());
    assert!(target
        .world
        .get_component::<DirectionalLightComponent>(empty_entity)
        .is_none());
    assert_eq!(target.get_parent(empty_entity), Some(cube_entity));

    let light_id = find_entity(&target, "Sun");
    let light_entity = target
        .get_root_entities()
        .iter()
        .copied()
        .find(|e| e.id == light_id)
        .unwrap();
    let light_component = target
        .world
        .get_component::<DirectionalLightComponent>(light_entity)
        .expect("directional light");
    assert!((light_component.intensity - 2.5).abs() < 0.001);
    assert!((light_component.color.y - 0.8).abs() < 0.001);
    assert!(target
        .world
        .get_component::<RenderableComponent>(light_entity)
        .is_none());

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn project_load_accepts_direct_json_file_and_create_new_file_path() {
    let temp_root = unique_temp_dir("project_file");
    std::fs::create_dir_all(&temp_root).unwrap();

    let project_file = temp_root.join("standalone.project.json");
    let mut project = Project::create_new("Standalone".to_string(), project_file.clone())
        .expect("create new file project");
    project.save().expect("save file project");
    assert!(project_file.exists(), "project file should be created at requested path");

    let loaded = Project::load(project_file.clone()).expect("load direct json file");
    assert!(loaded.is_loaded());
    assert_eq!(loaded.get_name(), "Standalone");

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn scene_snapshot_restore_preserves_entity_ids_and_hierarchy() {
    let mut scene = Scene::new();
    let parent = scene.create_cube();
    let child = scene.create_entity();
    scene.set_entity_name(parent, "Parent".to_string());
    scene.set_entity_name(child, "Child".to_string());
    scene.set_entity_position(parent, Vec3::new(3.0, 4.0, 5.0));
    scene.set_parent(child, Some(parent));

    let json = hezhou_core::Project::snapshot_to_json(&scene).expect("snapshot");
    let mut restored = Scene::new();
    hezhou_core::Project::restore_from_json(&json, &mut restored).expect("restore");

    let restored_parent = find_entity(&restored, "Parent");
    let restored_child = find_entity(&restored, "Child");
    assert_eq!(restored_parent, parent.id);
    assert_eq!(restored_child, child.id);
    assert_eq!(
        restored.get_parent(hezhou_core::Entity::new(restored_child)),
        Some(hezhou_core::Entity::new(parent.id))
    );
}
