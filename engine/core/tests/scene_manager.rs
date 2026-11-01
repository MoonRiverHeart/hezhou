//! SceneManager：加载/切换/回退/卸载/从 Project 加载场景。

use hezhou_core::{Project, Scene, SceneError, SceneManager, ScriptAsset, Vec3};

fn scene_with_cubes(count: usize) -> Scene {
    let mut scene = Scene::new();
    for index in 0..count {
        let cube = scene.create_cube();
        scene.set_entity_name(cube, format!("Cube_{index}"));
        scene.set_entity_position(cube, Vec3::new(index as f32, 0.0, 0.0));
    }
    scene
}

#[test]
fn load_switch_and_switch_back_preserve_scene_state() {
    let mut manager = SceneManager::new();
    assert_eq!(manager.scene_count(), 0);
    assert!(manager.current_scene().is_none());

    manager.load_scene("Menu", scene_with_cubes(1));
    manager.load_scene("Level", scene_with_cubes(3));
    assert_eq!(manager.scene_count(), 2);
    assert_eq!(manager.scene_names(), vec!["Level", "Menu"]);

    let first_switch = manager.switch_to("Menu").expect("切换应成功");
    assert_eq!(
        first_switch,
        hezhou_core::SceneSwitch {
            from: None,
            to: "Menu".to_string()
        }
    );
    assert_eq!(manager.current_name(), Some("Menu"));
    assert_eq!(manager.current_scene().unwrap().entity_count(), 1);

    let second_switch = manager.switch_to("Level").expect("切换应成功");
    assert_eq!(
        second_switch,
        hezhou_core::SceneSwitch {
            from: Some("Menu".to_string()),
            to: "Level".to_string()
        }
    );
    assert_eq!(manager.current_scene().unwrap().entity_count(), 3);
    assert_eq!(manager.previous_scene_name(), Some("Menu"));

    // 切换只切当前指针，场景实例保留：切回后实体仍在。
    manager.switch_to("Menu").unwrap();
    assert_eq!(manager.current_scene().unwrap().entity_count(), 1);
    assert_eq!(manager.previous_scene_name(), Some("Level"));

    // switch_back 回到 Level。
    let back = manager.switch_back().expect("回退应成功");
    assert_eq!(back.to, "Level");
    assert_eq!(manager.current_name(), Some("Level"));
    assert_eq!(manager.current_scene().unwrap().entity_count(), 3);

    // 切换到当前场景是幂等的，不写历史。
    let history_before = manager.previous_scene_name().map(str::to_string);
    let same = manager.switch_to("Level").unwrap();
    assert_eq!(same.from, Some("Level".to_string()));
    assert_eq!(manager.previous_scene_name(), history_before.as_deref());
}

#[test]
fn unknown_scene_errors_keep_current_scene() {
    let mut manager = SceneManager::new();
    manager.load_scene("A", scene_with_cubes(1));
    manager.switch_to("A").unwrap();

    assert_eq!(
        manager.switch_to("Missing"),
        Err(SceneError::NotFound("Missing".to_string()))
    );
    assert_eq!(manager.current_name(), Some("A"));

    assert!(matches!(
        manager.unload_scene("Missing"),
        Err(SceneError::NotFound(_))
    ));
    assert_eq!(manager.current_name(), Some("A"));

    let mut empty = SceneManager::new();
    assert_eq!(empty.switch_back(), Err(SceneError::NoPreviousScene));
}

#[test]
fn load_same_name_replaces_scene() {
    let mut manager = SceneManager::new();
    manager.load_scene("Arena", scene_with_cubes(1));
    assert_eq!(manager.scene("Arena").unwrap().entity_count(), 1);

    manager.load_scene("Arena", scene_with_cubes(4));
    assert_eq!(manager.scene_count(), 1, "同名加载应替换而不是追加");
    assert_eq!(manager.scene("Arena").unwrap().entity_count(), 4);
}

#[test]
fn unload_current_cleans_state_and_returns_scene() {
    let mut manager = SceneManager::new();
    let mut scene = Scene::new();
    let cube = scene.create_cube();
    scene.set_entity_name(cube, "Temporary".to_string());
    scene.register_script_asset(ScriptAsset::new("scripts/Temp.cs".to_string()));
    scene.load_scripts();
    manager.load_scene("TempLevel", scene);
    manager.switch_to("TempLevel").unwrap();

    let returned = manager.unload_scene("TempLevel").expect("卸载应返回场景");
    assert_eq!(
        manager.current_name(),
        None,
        "卸载当前场景后 current 应清空"
    );
    assert!(!manager.is_loaded("TempLevel"));
    assert!(manager.scene("TempLevel").is_none());
    assert!(
        manager.previous_scene_name().is_none(),
        "历史中的场景也应移除"
    );

    // 卸载清理：回到 Editing、脚本资源标记未加载；实体数据保留给调用方处理。
    assert_eq!(returned.state, hezhou_core::GameState::Editing);
    assert_eq!(returned.entity_count(), 1);
    assert!(returned
        .script_assets
        .values()
        .all(|asset| !asset.is_loaded));
}

#[test]
fn unload_all_returns_scenes_and_clears_manager() {
    let mut manager = SceneManager::new();
    manager.load_scene("B", scene_with_cubes(2));
    manager.load_scene("A", scene_with_cubes(1));
    manager.switch_to("B").unwrap();

    let removed = manager.unload_all();
    assert_eq!(removed.len(), 2);
    assert_eq!(
        removed
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "B"],
        "卸载顺序应按场景名排序，输出稳定"
    );
    assert_eq!(removed[0].1.state, hezhou_core::GameState::Editing);
    assert_eq!(manager.scene_count(), 0);
    assert!(manager.current_scene().is_none());
    assert!(manager.previous_scene_name().is_none());
}

#[test]
fn load_scene_from_project_restores_entities() {
    let mut source = Scene::new();
    let cube = source.create_cube();
    source.set_entity_name(cube, "ProjectCube".to_string());
    source.set_entity_position(cube, Vec3::new(7.0, 0.0, 0.0));

    let mut project = Project::new("Demo".to_string(), std::path::PathBuf::from("."));
    project.sync_from_scene(&source);

    let mut manager = SceneManager::new();
    manager.load_scene_from_project("DemoLevel", &project);
    manager.switch_to("DemoLevel").unwrap();

    let scene = manager.current_scene().unwrap();
    assert_eq!(scene.entity_count(), 1);
    let entity = scene.get_all_entities()[0];
    assert_eq!(
        scene.get_entity_name(entity).map(String::as_str),
        Some("ProjectCube")
    );
    let position = scene.get_entity_position(entity).expect("应有位置");
    assert!((position - Vec3::new(7.0, 0.0, 0.0)).magnitude() < 1e-5);
}

#[test]
fn load_scene_from_project_json_restores_entities() {
    let mut source = Scene::new();
    let cube = source.create_cube();
    source.set_entity_name(cube, "JsonCube".to_string());
    let json = Project::snapshot_to_json(&source).expect("快照应成功");

    let mut manager = SceneManager::new();
    manager
        .load_scene_from_project_json("JsonLevel", &json)
        .expect("从 JSON 加载应成功");
    manager.switch_to("JsonLevel").unwrap();

    let scene = manager.current_scene().unwrap();
    assert_eq!(scene.entity_count(), 1);
    let entity = scene.get_all_entities()[0];
    assert_eq!(
        scene.get_entity_name(entity).map(String::as_str),
        Some("JsonCube")
    );

    // 非法 JSON 应返回 LoadFailed 且不注册场景。
    let mut manager = SceneManager::new();
    let result = manager.load_scene_from_project_json("Bad", "{ not valid json");
    assert!(matches!(result, Err(SceneError::LoadFailed(_))));
    assert!(!manager.is_loaded("Bad"));
}

#[test]
fn current_scene_mut_allows_runtime_updates() {
    let mut manager = SceneManager::new();
    manager.load_scene("Runtime", scene_with_cubes(1));
    manager.switch_to("Runtime").unwrap();

    let scene = manager.current_scene_mut().expect("应有当前场景");
    let cube = scene.get_all_entities()[0];
    scene.set_entity_position(cube, Vec3::new(1.0, 2.0, 3.0));

    let position = manager
        .current_scene()
        .unwrap()
        .get_entity_position(cube)
        .expect("应有位置");
    assert!((position - Vec3::new(1.0, 2.0, 3.0)).magnitude() < 1e-5);
}
