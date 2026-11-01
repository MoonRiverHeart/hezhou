//! 起始场景加载（`load_start_scene`）测试：
//! Project JSON → Scene、层级/Transform 恢复、缺失/坏 JSON/空场景回退、
//! 与 PlayerAssetIndex 的 start_scene output 对接。

mod common;

use std::path::PathBuf;

use common::{cooked_fixture_dir, TempDir};
use hezhou_core::{DirectionalLightComponent, LocalTransform, Project, Scene, Vec3};
use hezhou_player::asset_index::PlayerAssetIndex;
use hezhou_player::builder::{
    BuildSources, CookManifest, PlayerBuildInput, PlayerBuilder, RuntimePayload, ScriptAssembly,
};
use hezhou_player::config::PlayerBuildConfig;
use hezhou_player::cook_bridge::CookedAssets;
use hezhou_player::manifest::{ManifestAsset, PlayerManifest};
use hezhou_player::platform::TargetPlatform;
use hezhou_player::scene_loader::{
    builtin_test_scene, load_start_scene, StartSceneSource,
};

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

/// 构造 legacy Player 包（manifest + `assets/scenes/main.scene`）。
fn write_legacy_package(temp: &TempDir, scene_json: Option<&str>) -> (PathBuf, PlayerManifest) {
    let config = PlayerBuildConfig::new("Scene Fixture", "1.0.0", "scenes/main.scene", host())
        .with_required_font(None::<String>);
    let manifest = PlayerManifest::new(
        &config,
        vec![ManifestAsset::new("scenes/main.scene", "scene")],
        Vec::new(),
    );
    temp.mkdir("assets/scenes");
    if let Some(json) = scene_json {
        temp.write("assets/scenes/main.scene", json.as_bytes());
    }
    let root = temp.path().to_path_buf();
    manifest.save(&root.join("manifest.json")).unwrap();
    (root, manifest)
}

fn project_json_with_scene() -> String {
    let mut source = Scene::new();
    let cube = source.create_cube();
    source.set_entity_name(cube, "CubeA".to_string());
    source.set_entity_position(cube, Vec3::new(1.0, 2.0, 3.0));
    let child = source.create_cube();
    source.set_entity_name(child, "ChildCube".to_string());
    source.set_parent(child, Some(cube));
    source.set_entity_position(child, Vec3::new(-1.0, 0.5, 2.0));
    let sun = source.create_directional_light();
    source.set_entity_name(sun, "Sun".to_string());
    Project::snapshot_to_json(&source).expect("生成 Project JSON")
}

fn find_by_name<'a>(scene: &'a Scene, name: &str) -> hezhou_core::Entity {
    scene
        .get_all_entities()
        .into_iter()
        .find(|entity| scene.get_entity_name(*entity).map(|n| n == name).unwrap_or(false))
        .unwrap_or_else(|| panic!("场景中找不到实体 {name}"))
}

#[test]
fn loads_project_json_scene_with_entities_and_hierarchy() {
    let temp = TempDir::new("scene-loader-ok");
    let (root, manifest) = write_legacy_package(&temp, Some(&project_json_with_scene()));
    let index = PlayerAssetIndex::load(&root, &manifest).expect("legacy 空索引");
    assert!(index.start_scene().is_none());

    let loaded = load_start_scene(&root, &manifest, &index);
    assert!(
        matches!(loaded.source, StartSceneSource::Project { entities: 3, .. }),
        "实际来源：{:?}",
        loaded.source
    );
    assert!(loaded.warnings.is_empty(), "warnings: {:?}", loaded.warnings);
    assert_eq!(loaded.entity_count(), 3);

    // Transform 恢复。
    let cube = find_by_name(&loaded.scene, "CubeA");
    let transform = loaded
        .scene
        .world
        .get_component::<LocalTransform>(cube)
        .expect("CubeA transform");
    assert_eq!(
        (transform.position.x, transform.position.y, transform.position.z),
        (1.0, 2.0, 3.0)
    );

    // 层级恢复（parent_id → set_parent）。
    let child = find_by_name(&loaded.scene, "ChildCube");
    assert_eq!(loaded.scene.get_parent(child), Some(cube));

    // 方向光恢复。
    let sun = find_by_name(&loaded.scene, "Sun");
    assert!(loaded
        .scene
        .world
        .get_component::<DirectionalLightComponent>(sun)
        .is_some());

    // 来源摘要可读。
    assert!(loaded.summary().contains("Project JSON"));
    assert!(loaded.summary().contains("3 个实体"));
}

#[test]
fn missing_scene_file_falls_back_to_builtin() {
    let temp = TempDir::new("scene-loader-missing");
    let (root, manifest) = write_legacy_package(&temp, None);
    let index = PlayerAssetIndex::load(&root, &manifest).expect("legacy 空索引");

    let loaded = load_start_scene(&root, &manifest, &index);
    assert!(loaded.source.is_fallback());
    assert!(matches!(
        loaded.source,
        StartSceneSource::BuiltinFallback { .. }
    ));
    assert_eq!(loaded.entity_count(), 2, "内置回退 = cube + light");
    assert!(!loaded.warnings.is_empty());
    assert!(loaded.warnings[0].contains("回退内置测试场景"));
    assert!(loaded.summary().contains("内置测试场景"));
}

#[test]
fn bad_json_falls_back_to_builtin() {
    let temp = TempDir::new("scene-loader-bad-json");
    let (root, manifest) = write_legacy_package(&temp, Some("{ not a project json"));
    let index = PlayerAssetIndex::load(&root, &manifest).expect("legacy 空索引");

    let loaded = load_start_scene(&root, &manifest, &index);
    assert!(loaded.source.is_fallback());
    assert_eq!(loaded.entity_count(), 2);
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("Project JSON")),
        "warnings: {:?}",
        loaded.warnings
    );
}

#[test]
fn empty_project_scene_falls_back_to_builtin() {
    let temp = TempDir::new("scene-loader-empty");
    // 合法 Project JSON，但 0 实体。
    let empty = Project::snapshot_to_json(&Scene::new()).unwrap();
    let (root, manifest) = write_legacy_package(&temp, Some(&empty));
    let index = PlayerAssetIndex::load(&root, &manifest).expect("legacy 空索引");

    let loaded = load_start_scene(&root, &manifest, &index);
    assert!(loaded.source.is_fallback());
    assert_eq!(loaded.entity_count(), 2);
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("没有实体")),
        "warnings: {:?}",
        loaded.warnings
    );
}

#[test]
fn builtin_test_scene_has_cube_and_light() {
    let scene = builtin_test_scene();
    let entities = scene.get_all_entities();
    assert_eq!(entities.len(), 2);
    let lights = entities
        .iter()
        .filter(|entity| {
            scene
                .world
                .get_component::<DirectionalLightComponent>(**entity)
                .is_some()
        })
        .count();
    assert_eq!(lights, 1);
}

#[test]
fn index_start_scene_output_drives_loader_path() {
    // 用 cooked fixture 构建真实包，验证索引 start_scene output 与 loader 路径一致。
    let temp = TempDir::new("scene-loader-cooked");
    let fixture = cooked_fixture_dir();
    let config_text = std::fs::read_to_string(fixture.join("build-config.json")).unwrap();
    let input = PlayerBuildInput {
        config: serde_json::from_str(&config_text).unwrap(),
        cook: CookManifest::default(),
        scripts: ScriptAssembly::from_dir(&fixture.join("scripts")).unwrap(),
        runtime: RuntimePayload::from_dir(&fixture.join("runtime")).unwrap(),
        cooked: Some(CookedAssets::load(&fixture.join("cook_manifest.json")).unwrap()),
    };
    let builder = PlayerBuilder::new(input);
    let sources = BuildSources {
        executable: Some(temp.write("source/player-exe", b"fake-exe")),
        assets_root: None,
    };
    let output = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect("构建 cooked 包");
    let manifest = PlayerManifest::load(&output.manifest_path).unwrap();
    let index = PlayerAssetIndex::load(&output.output_dir, &manifest).unwrap();
    let start_entry = index.start_scene().expect("索引 start_scene");

    let loaded = load_start_scene(&output.output_dir, &manifest, &index);
    match &loaded.source {
        StartSceneSource::Project { path, entities } => {
            assert_eq!(*entities, 3, "cooked fixture 场景应有 3 个实体");
            let expected = manifest
                .assets_dir(&output.output_dir)
                .join(hezhou_player::to_native_path(&start_entry.output, host()));
            assert_eq!(path, &expected, "loader 应使用索引 output 路径");
            assert_eq!(loaded.entity_count(), 3);
        }
        other => panic!("应加载 Project JSON，实际：{other:?}"),
    }
}

