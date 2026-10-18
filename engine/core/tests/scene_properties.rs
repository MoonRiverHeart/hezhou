use hezhou_core::ecs::property::{
    get_entity_property_descriptors, get_entity_property_value, set_entity_property_value,
    PropertyValue,
};
use hezhou_core::ecs::{DirectionalLightComponent, LocalTransform, RenderableComponent};
use hezhou_core::math::Vec3;
use hezhou_core::{Entity, GameState, Scene};

#[test]
fn scene_clear_removes_entities_names_bindings_and_selection() {
    let mut scene = Scene::new();
    let cube = scene.create_cube();
    scene.set_entity_name(cube, "OldCube".to_string());
    scene.select_entity(cube);
    scene.attach_script_binding(
        cube,
        "scripts/RotatingEntity.cs".to_string(),
        "RotatingEntity".to_string(),
    );

    scene.state = GameState::Running;
    scene.clear();

    assert_eq!(scene.entity_count(), 0);
    assert_eq!(scene.root_entities.len(), 0);
    assert_eq!(scene.selected_entities.len(), 0);
    assert!(scene.get_entity_name(cube).is_none());
    assert_eq!(scene.get_script_binding_count(cube), 0);
    assert_eq!(scene.state, GameState::Editing);

    // clear 保留 next_entity_id，避免旧 UI 引用与新实体 ID 冲突。
    let next = scene.create_entity();
    assert!(next.id > cube.id);
}

#[test]
fn scene_hierarchy_roundtrip_and_root_bookkeeping() {
    let mut scene = Scene::new();
    let parent = scene.create_entity();
    let child_a = scene.create_entity();
    let child_b = scene.create_entity();

    scene.set_parent(child_a, Some(parent));
    scene.set_parent(child_b, Some(parent));

    assert_eq!(scene.get_parent(child_a), Some(parent));
    assert_eq!(scene.get_children(parent).len(), 2);
    assert!(!scene.root_entities.contains(&child_a));
    assert!(!scene.root_entities.contains(&child_b));

    let children = scene.get_children(parent);
    let ids: Vec<u64> = children.iter().map(|e| e.id).collect();
    assert!(ids.contains(&child_a.id));
    assert!(ids.contains(&child_b.id));

    scene.set_parent(child_a, None);
    assert_eq!(scene.get_parent(child_a), None);
    assert!(scene.root_entities.contains(&child_a));
    assert_eq!(scene.get_children(parent).len(), 1);

    // 删除父实体时，剩余子实体应被提升为根，不能成为“孤儿”。
    scene.remove_entity(parent);
    assert_eq!(scene.get_parent(child_b), None);
    assert!(scene.root_entities.contains(&child_b));
    assert!(!scene.root_entities.contains(&parent));
}

#[test]
fn additive_selection_supports_multi_select_and_toggle() {
    let mut scene = Scene::new();
    let a = scene.create_entity();
    let b = scene.create_entity();
    let c = scene.create_entity();

    scene.add_to_selection(a);
    scene.add_to_selection(b);
    assert_eq!(scene.get_selected_entity_ids(), vec![a.id, b.id]);
    assert!(scene.is_entity_selected(a));
    assert!(scene.is_entity_selected(b));

    // 重复 add 不产生重复项
    scene.add_to_selection(a);
    assert_eq!(scene.get_selected_entity_ids().len(), 2);

    scene.remove_from_selection(a);
    assert!(!scene.is_entity_selected(a));
    assert_eq!(scene.get_selected_entity_ids(), vec![b.id]);

    scene.toggle_selection(c);
    assert!(scene.is_entity_selected(c));
    scene.toggle_selection(b);
    assert!(!scene.is_entity_selected(b));
    assert_eq!(scene.get_selected_entity_ids(), vec![c.id]);

    scene.clear_selection();
    assert!(scene.get_selected_entity_ids().is_empty());
}

#[test]
fn property_reflection_handles_transforms_lights_and_read_only_fields() {
    let mut scene = Scene::new();
    let cube = scene.create_cube();
    let entity = Entity::new(cube.id);

    assert_eq!(get_entity_property_descriptors().len(), 10);

    assert!(set_entity_property_value(
        &mut scene,
        entity,
        "position",
        &PropertyValue::Float3 {
            x: 1.5,
            y: -2.0,
            z: 3.25
        }
    ));
    match get_entity_property_value(&scene, entity, "position") {
        Some(PropertyValue::Float3 { x, y, z }) => {
            assert!((x - 1.5).abs() < 0.001);
            assert!((y + 2.0).abs() < 0.001);
            assert!((z - 3.25).abs() < 0.001);
        }
        other => panic!("unexpected position value: {:?}", other),
    }

    assert!(set_entity_property_value(
        &mut scene,
        entity,
        "visible",
        &PropertyValue::Bool(false)
    ));
    let renderable = scene
        .world
        .get_component::<RenderableComponent>(entity)
        .expect("renderable");
    assert!(!renderable.visible);

    // 只读属性不可写
    assert!(!set_entity_property_value(
        &mut scene,
        entity,
        "id",
        &PropertyValue::Int(999)
    ));
    // 未知属性不可读写
    assert!(!set_entity_property_value(
        &mut scene,
        entity,
        "not_a_property",
        &PropertyValue::Float(1.0)
    ));
    assert!(get_entity_property_value(&scene, entity, "not_a_property").is_none());

    let light = scene.create_directional_light();
    let light_entity = Entity::new(light.id);
    assert!(set_entity_property_value(
        &mut scene,
        light_entity,
        "light_intensity",
        &PropertyValue::Float(4.0)
    ));
    let component = scene
        .world
        .get_component::<DirectionalLightComponent>(light_entity)
        .expect("light component");
    assert!((component.intensity - 4.0).abs() < 0.001);
}

#[test]
fn transform_helpers_reject_missing_entities_without_panicking() {
    let mut scene = Scene::new();
    let missing = Entity::new(99999);
    scene.set_entity_position(missing, Vec3::new(1.0, 2.0, 3.0));
    scene.set_entity_scale(missing, Vec3::one());
    assert!(scene.get_entity_position(missing).is_none());
    assert!(scene
        .world
        .get_component::<LocalTransform>(missing)
        .is_none());
}
