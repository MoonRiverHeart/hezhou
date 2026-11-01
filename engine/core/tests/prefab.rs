//! Prefab：JSON 往返、前向兼容、实例化、覆盖、一层嵌套、循环引用、捕获。

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hezhou_core::{
    BoundsComponent, DirectionalLightComponent, Entity, LocalTransform, Prefab, PrefabEntity,
    PrefabError, PrefabLight, PrefabOverrides, PrefabReference, PrefabRegistry, PrefabRenderable,
    PrefabScriptBinding, PrefabTransform, RenderableComponent, Scene, Vec3,
};

fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target")
        .join("prefab_tests_tmp");
    let _ = std::fs::create_dir_all(&base);
    base.join(format!(
        "hezhou_prefab_{}_{}_{}",
        tag,
        std::process::id(),
        nanos
    ))
}

/// 通用敌人模板：根 + 两个子实体 + 一个孙实体 + 脚本/渲染组件。
fn demo_prefab() -> Prefab {
    let weapon = PrefabEntity::new("Weapon")
        .with_transform(PrefabTransform {
            position: [0.3, 1.2, 0.2],
            ..PrefabTransform::default()
        })
        .with_renderable(PrefabRenderable::new("builtin://cylinder"));

    let body = PrefabEntity::new("Body")
        .with_transform(PrefabTransform {
            position: [0.0, 1.0, 0.0],
            rotation: [0.0, 45.0, 0.0],
            scale: [2.0, 2.0, 2.0],
        })
        .with_renderable(PrefabRenderable {
            material: Some("materials/Enemy.mat".to_string()),
            ..PrefabRenderable::new("builtin://cube")
        })
        .with_children(vec![weapon]);

    let head = PrefabEntity::new("Head")
        .with_transform(PrefabTransform {
            position: [0.0, 2.4, 0.0],
            ..PrefabTransform::default()
        })
        .with_renderable(PrefabRenderable::new("builtin://sphere"));

    let root = PrefabEntity::new("EnemyRoot")
        .with_transform(PrefabTransform {
            position: [1.0, 0.0, 2.0],
            ..PrefabTransform::default()
        })
        .with_renderable(PrefabRenderable::new("builtin://cube"))
        .with_scripts(vec![
            PrefabScriptBinding::new("scripts/Enemy.cs", "Enemy"),
            PrefabScriptBinding {
                enabled: false,
                ..PrefabScriptBinding::new("scripts/Health.cs", "Health")
            },
        ])
        .with_children(vec![body, head]);

    Prefab::new("Enemy", root).with_description("通用敌人模板")
}

fn nested_slot_prefab() -> Prefab {
    let slot = PrefabEntity::new("WeaponSlot")
        .with_transform(PrefabTransform {
            position: [0.5, 1.0, 0.0],
            ..PrefabTransform::default()
        })
        .with_nested_prefab(
            PrefabReference::new("Sword").with_overrides(PrefabOverrides {
                position: Some([1.5, 0.0, 0.0]),
                ..PrefabOverrides::default()
            }),
        );

    Prefab::new(
        "Warrior",
        PrefabEntity::new("WarriorRoot").with_children(vec![slot]),
    )
}

fn sword_prefab() -> Prefab {
    Prefab::new(
        "Sword",
        PrefabEntity::new("SwordRoot")
            .with_renderable(PrefabRenderable::new("builtin://cube"))
            .with_children(vec![PrefabEntity::new("Blade")]),
    )
}

fn find_by_name(scene: &Scene, name: &str) -> Option<Entity> {
    scene
        .get_all_entities()
        .into_iter()
        .find(|entity| scene.get_entity_name(*entity).map(String::as_str) == Some(name))
}

#[test]
fn prefab_json_roundtrip_is_stable() {
    let prefab = demo_prefab();
    let json = prefab.to_json().expect("序列化应成功");
    let parsed = Prefab::from_json(&json).expect("反序列化应成功");

    assert_eq!(parsed, prefab, "Prefab 往返应完全一致");
    assert_eq!(parsed.version, "1.0");
    assert_eq!(parsed.description, "通用敌人模板");
    assert_eq!(
        parsed.to_json().expect("二次序列化应成功"),
        json,
        "两次序列化结果应稳定"
    );
}

#[test]
fn prefab_json_is_forward_compatible() {
    // 缺少全部可选字段 + 未知字段（模拟未来版本/手写 JSON）。
    let json = r#"{
        "version": "1.0",
        "name": "Loose",
        "future_top_level_field": { "anything": 1 },
        "root": {
            "name": "Root",
            "future_entity_field": "ignored",
            "transform": { "position": [1, 2, 3] },
            "renderable": { "mesh_path": "builtin://cube" },
            "children": [{ "name": "Child" }]
        }
    }"#;

    let prefab = Prefab::from_json(json).expect("应兼容缺失可选字段与未知字段");
    assert_eq!(prefab.name, "Loose");
    assert_eq!(prefab.root.transform.position, [1.0, 2.0, 3.0]);
    assert_eq!(prefab.root.transform.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(prefab.root.transform.scale, [1.0, 1.0, 1.0]);
    assert!(prefab.root.visible, "visible 默认应为 true");
    assert!(prefab.root.scripts.is_empty());
    assert_eq!(prefab.root.children.len(), 1);
}

#[test]
fn prefab_rejects_unsupported_major_version() {
    let json = r#"{
        "version": "2.0",
        "name": "Future",
        "root": { "name": "Root" }
    }"#;
    match Prefab::from_json(json) {
        Err(PrefabError::UnsupportedVersion { found, .. }) => assert_eq!(found, "2.0"),
        other => panic!("应拒绝 2.0 版本，实际: {other:?}"),
    }
}

#[test]
fn instantiate_creates_unique_ids_and_correct_hierarchy() {
    let prefab = demo_prefab();
    let mut scene = Scene::new();

    let first = scene
        .instantiate_prefab(&prefab, None)
        .expect("第一次实例化应成功");
    let second = scene
        .instantiate_prefab(&prefab, None)
        .expect("第二次实例化应成功");

    assert_eq!(first.len(), 4, "根 + Body + Weapon + Head = 4 个实体");
    assert_eq!(second.len(), 4);
    assert_eq!(scene.entity_count(), 8, "两次实例化应创建 8 个独立实体");

    // 两次实例化的实体 ID 不重叠、根实体唯一。
    for entity in &first.entities {
        assert!(
            !second.entities.contains(entity),
            "两次实例化的实体 ID 不应重复"
        );
        assert!(scene.world.entity_exists(*entity));
    }
    assert!(first.root.is_valid() && second.root.is_valid());
    assert_ne!(first.root, second.root);

    // 根作为场景根实体，Body/Head 是根的子实体，Weapon 是 Body 的子实体。
    assert!(scene.get_root_entities().contains(&first.root));
    let body = first
        .entity_at("EnemyRoot/Body")
        .expect("路径应映射到 Body");
    let head = first
        .entity_at("EnemyRoot/Head")
        .expect("路径应映射到 Head");
    let weapon = first
        .entity_at("EnemyRoot/Body/Weapon")
        .expect("路径应映射到 Weapon");
    assert_eq!(scene.get_parent(body), Some(first.root));
    assert_eq!(scene.get_parent(head), Some(first.root));
    assert_eq!(scene.get_parent(weapon), Some(body));
    assert_eq!(scene.get_children(first.root).len(), 2);

    // 路径映射可反查。
    assert_eq!(first.path_of(head), Some("EnemyRoot/Head"));
}

#[test]
fn instantiate_preserves_scripts_renderable_and_metadata() {
    let prefab = demo_prefab();
    let mut scene = Scene::new();
    let instance = scene.instantiate_prefab(&prefab, None).unwrap();
    let root = instance.root;

    let transform = scene.world.get_component::<LocalTransform>(root).unwrap();
    assert!((transform.position - Vec3::new(1.0, 0.0, 2.0)).magnitude() < 1e-5);

    let renderable = scene
        .world
        .get_component::<RenderableComponent>(root)
        .expect("根应有 Renderable");
    assert_eq!(renderable.mesh_path, "builtin://cube");
    assert!(renderable.visible);
    assert!(
        scene.world.get_component::<BoundsComponent>(root).is_some(),
        "可渲染实体应带包围盒"
    );

    // 脚本绑定保留，enabled 状态保留。
    assert_eq!(scene.get_script_binding_count(root), 2);
    let first = scene.get_script_binding(root, 0).unwrap();
    assert_eq!(first.script_path, "scripts/Enemy.cs");
    assert_eq!(first.class_name, "Enemy");
    assert!(first.enabled);
    let second = scene.get_script_binding(root, 1).unwrap();
    assert_eq!(second.class_name, "Health");
    assert!(!second.enabled, "禁用的脚本绑定应保持禁用");

    // 材质与名字。
    let body = find_by_name(&scene, "Body").unwrap();
    assert_eq!(
        scene.get_renderable_material(body),
        Some("materials/Enemy.mat".to_string())
    );
    assert_eq!(
        scene.get_entity_name(root).map(String::as_str),
        Some("EnemyRoot")
    );
}

#[test]
fn overrides_apply_to_root_only() {
    let prefab = demo_prefab();
    let mut scene = Scene::new();
    let overrides = PrefabOverrides {
        name: Some("BossRoot".to_string()),
        position: Some([9.0, 1.0, -2.0]),
        rotation: Some([0.0, 90.0, 0.0]),
        scale: Some([3.0, 3.0, 3.0]),
        visible: Some(false),
    };
    let instance = scene
        .instantiate_prefab_with(
            &prefab,
            &hezhou_core::PrefabInstantiateOptions::new().with_overrides(&overrides),
        )
        .expect("带覆盖实例化应成功");
    let root = instance.root;

    let transform = scene.world.get_component::<LocalTransform>(root).unwrap();
    assert!((transform.position - Vec3::new(9.0, 1.0, -2.0)).magnitude() < 1e-5);
    assert!((transform.scale - Vec3::new(3.0, 3.0, 3.0)).magnitude() < 1e-5);
    let euler = transform.rotation.to_euler_degrees();
    assert!((euler[1] - 90.0).abs() < 0.05, "y 轴旋转应生效: {euler:?}");
    assert_eq!(
        scene.get_entity_name(root).map(String::as_str),
        Some("BossRoot")
    );
    assert!(
        !scene
            .world
            .get_component::<RenderableComponent>(root)
            .unwrap()
            .visible,
        "visible 覆盖应生效"
    );

    // 子实体不受根覆盖影响。
    let body = find_by_name(&scene, "Body").unwrap();
    assert_eq!(
        scene.get_entity_name(body).map(String::as_str),
        Some("Body")
    );
    let body_transform = scene.world.get_component::<LocalTransform>(body).unwrap();
    assert!((body_transform.position.y - 1.0).abs() < 1e-5);
}

#[test]
fn instance_can_be_attached_to_parent_entity() {
    let prefab = demo_prefab();
    let mut scene = Scene::new();
    let parent = scene.create_entity();
    scene.set_entity_name(parent, "Arena".to_string());

    let instance = scene
        .instantiate_prefab_with(
            &prefab,
            &hezhou_core::PrefabInstantiateOptions::new().with_parent(parent),
        )
        .expect("挂到父实体应成功");

    assert_eq!(scene.get_parent(instance.root), Some(parent));
    assert!(
        !scene.get_root_entities().contains(&instance.root),
        "挂到父实体后不再是根实体"
    );
    assert!(scene.get_children(parent).contains(&instance.root));

    // 父实体不存在时应报错。
    let missing = Entity::new(999_999);
    let result = scene.instantiate_prefab_with(
        &prefab,
        &hezhou_core::PrefabInstantiateOptions::new().with_parent(missing),
    );
    assert!(matches!(result, Err(PrefabError::InvalidParent(_))));
}

#[test]
fn nested_prefab_instantiates_one_level() {
    let mut registry = PrefabRegistry::new();
    assert!(registry.insert(sword_prefab()));
    assert!(registry.insert(nested_slot_prefab()));
    assert_eq!(registry.len(), 2);
    assert_eq!(registry.keys(), vec!["Sword", "Warrior"]);

    let mut scene = Scene::new();
    let prefab = nested_slot_prefab();
    let instance = scene
        .instantiate_prefab_with(
            &prefab,
            &hezhou_core::PrefabInstantiateOptions::new().with_registry(&registry),
        )
        .expect("一层嵌套应成功");

    // WarriorRoot / WeaponSlot / SwordRoot / Blade = 4 个实体。
    assert_eq!(instance.len(), 4, "实际路径: {:?}", instance.paths);
    let warrior_root = instance.entity_at("WarriorRoot").unwrap();
    let slot = instance.entity_at("WarriorRoot/WeaponSlot").unwrap();
    let sword_root = instance
        .entity_at("WarriorRoot/WeaponSlot/SwordRoot")
        .unwrap();
    let blade = instance
        .entity_at("WarriorRoot/WeaponSlot/SwordRoot/Blade")
        .unwrap();

    assert_eq!(scene.get_parent(slot), Some(warrior_root));
    assert_eq!(scene.get_parent(sword_root), Some(slot));
    assert_eq!(scene.get_parent(blade), Some(sword_root));
    assert_eq!(scene.get_parent(warrior_root), None);

    // 嵌套覆盖：SwordRoot 位置被 override 为 [1.5, 0, 0]（相对 WeaponSlot）。
    let transform = scene
        .world
        .get_component::<LocalTransform>(sword_root)
        .unwrap();
    assert!((transform.position - Vec3::new(1.5, 0.0, 0.0)).magnitude() < 1e-5);
    assert!(
        scene
            .world
            .get_component::<RenderableComponent>(sword_root)
            .is_some(),
        "嵌套 Prefab 根应带自己的渲染组件"
    );
}

#[test]
fn nested_prefab_missing_registry_or_entry_errors() {
    let prefab = nested_slot_prefab();
    let mut scene = Scene::new();

    // 没有 registry。
    let result = scene.instantiate_prefab(&prefab, None);
    assert!(matches!(result, Err(PrefabError::MissingRegistry(_))));

    // registry 里没有对应 key。
    let registry = PrefabRegistry::new();
    let result = scene.instantiate_prefab_with(
        &prefab,
        &hezhou_core::PrefabInstantiateOptions::new().with_registry(&registry),
    );
    assert!(matches!(result, Err(PrefabError::MissingPrefab(_))));
}

#[test]
fn nested_prefab_cycle_is_rejected() {
    // A → B → A
    let mut registry = PrefabRegistry::new();
    registry.insert_with_key(
        "A",
        Prefab::new(
            "A",
            PrefabEntity::new("ARoot").with_nested_prefab(PrefabReference::new("B")),
        ),
    );
    registry.insert_with_key(
        "B",
        Prefab::new(
            "B",
            PrefabEntity::new("BRoot").with_nested_prefab(PrefabReference::new("A")),
        ),
    );
    let mut scene = Scene::new();
    let result = scene.instantiate_prefab_with(
        registry.get("A").unwrap(),
        &hezhou_core::PrefabInstantiateOptions::new().with_registry(&registry),
    );
    match result {
        Err(PrefabError::CircularReference { path }) => {
            assert!(path.contains(&"A".to_string()), "引用栈应包含 A: {path:?}");
        }
        other => panic!("应检测到循环引用，实际: {other:?}"),
    }

    // 自引用 A → A
    let mut self_registry = PrefabRegistry::new();
    self_registry.insert_with_key(
        "Self",
        Prefab::new(
            "Self",
            PrefabEntity::new("SelfRoot").with_nested_prefab(PrefabReference::new("Self")),
        ),
    );
    let result = scene.instantiate_prefab_with(
        self_registry.get("Self").unwrap(),
        &hezhou_core::PrefabInstantiateOptions::new().with_registry(&self_registry),
    );
    assert!(matches!(result, Err(PrefabError::CircularReference { .. })));
}

#[test]
fn nested_prefab_depth_is_limited() {
    // A → B → C：只允许一层嵌套。
    let mut registry = PrefabRegistry::new();
    registry.insert_with_key(
        "A",
        Prefab::new(
            "A",
            PrefabEntity::new("ARoot").with_nested_prefab(PrefabReference::new("B")),
        ),
    );
    registry.insert_with_key(
        "B",
        Prefab::new(
            "B",
            PrefabEntity::new("BRoot").with_nested_prefab(PrefabReference::new("C")),
        ),
    );
    registry.insert_with_key("C", Prefab::new("C", PrefabEntity::new("CRoot")));

    let mut scene = Scene::new();
    let result = scene.instantiate_prefab_with(
        registry.get("A").unwrap(),
        &hezhou_core::PrefabInstantiateOptions::new().with_registry(&registry),
    );
    assert!(
        matches!(result, Err(PrefabError::NestingTooDeep { max_depth: 1 })),
        "应限制为一层嵌套，实际: {result:?}"
    );
}

#[test]
fn capture_from_scene_roundtrips_subtree() {
    let mut scene = Scene::new();
    let root = scene.create_entity();
    scene.set_entity_name(root, "CapturedRoot".to_string());
    scene.set_entity_position(root, Vec3::new(4.0, 5.0, 6.0));
    scene
        .world
        .add_component(root, RenderableComponent::new("builtin://cube".to_string()));
    scene.world.add_component(root, BoundsComponent::cube(2.0));
    scene.attach_script_binding(root, "scripts/Player.cs".to_string(), "Player".to_string());
    let child = scene.create_entity();
    scene.set_entity_name(child, "Child".to_string());
    scene.set_parent(child, Some(root));
    scene
        .world
        .add_component(child, DirectionalLightComponent::default());

    let prefab = Prefab::capture_from_scene(&scene, root, "Captured").expect("捕获应成功");
    assert_eq!(prefab.name, "Captured");
    assert_eq!(prefab.root.name, "CapturedRoot");
    assert_eq!(prefab.root.transform.position, [4.0, 5.0, 6.0]);
    assert!(prefab.root.renderable.is_some());
    assert_eq!(prefab.root.scripts.len(), 1);
    assert_eq!(prefab.root.scripts[0].class_name, "Player");
    assert_eq!(prefab.root.children.len(), 1);
    assert_eq!(prefab.root.children[0].name, "Child");
    assert!(prefab.root.children[0].light.is_some());
    assert_eq!(
        prefab.root.children[0].light.unwrap(),
        PrefabLight::default()
    );

    // 捕获的 Prefab 可以再实例化到新场景。
    let mut target = Scene::new();
    let instance = target.instantiate_prefab(&prefab, None).unwrap();
    assert_eq!(instance.len(), 2);
    let restored_root = instance.root;
    assert_eq!(
        target.get_entity_name(restored_root).map(String::as_str),
        Some("CapturedRoot")
    );
    assert_eq!(target.get_script_binding_count(restored_root), 1);
    let restored_child = instance.entity_at("CapturedRoot/Child").unwrap();
    assert_eq!(target.get_parent(restored_child), Some(restored_root));
}

#[test]
fn save_and_load_prefab_file() {
    let dir = unique_temp_dir("save_load");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("enemy.prefab.json");

    let prefab = demo_prefab();
    prefab.save_file(&path).expect("保存应成功");
    assert!(path.exists());
    let loaded = Prefab::load_file(&path).expect("加载应成功");
    assert_eq!(loaded, prefab);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn capture_missing_entity_returns_error() {
    let scene = Scene::new();
    let result = Prefab::capture_from_scene(&scene, Entity::new(42), "Ghost");
    assert!(matches!(result, Err(PrefabError::EntityNotFound(42))));
}
