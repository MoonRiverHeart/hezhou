//! cook 导入资产 bundle（`assets/imported/*.json`）测试：
//! ImportedScene 解析、AssetId→mesh/material/skeleton/animation 查询、
//! 与 cook manifest 的严格一致性校验、坏 JSON/越界/数量不匹配错误、未引用文件告警。

mod common;

use std::path::PathBuf;

use common::{cooked_fixture_dir, TempDir};
use hezhou_asset::cook::{CookEntry, CookManifest};
use hezhou_asset::{
    AssetId, AssetKind, AssetLocator, ImportSourceFormat, ImportedAnimation, ImportedMaterial,
    ImportedMesh, ImportedScene, ImportedSkeleton,
};
use hezhou_player::asset_index::PlayerAssetIndex;
use hezhou_player::builder::{
    BuildSources, CookManifest as PlayerCookManifest, PlayerBuildInput, PlayerBuilder,
    RuntimePayload, ScriptAssembly,
};
use hezhou_player::config::PlayerBuildConfig;
use hezhou_player::cook_bridge::CookedAssets;
use hezhou_player::imported_assets::{
    ImportedAssetBundle, ImportedAssetError,
};
use hezhou_player::manifest::PlayerManifest;
use hezhou_player::platform::TargetPlatform;

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

fn id(text: &str) -> AssetId {
    AssetId::parse(text).expect("AssetId")
}

const PARENT_GLTF: &str = "33333333-3333-4333-8333-333333333333";
const SCENE_ID: &str = "11111111-1111-4111-8111-111111111111";

struct Fixture {
    assets_dir: PathBuf,
    cook: CookManifest,
    manifest: PlayerManifest,
    parent: AssetId,
    mesh: AssetId,
    material: AssetId,
    skeleton: AssetId,
    animation: AssetId,
    output: String,
}

/// 构造合成 cook manifest + imported JSON（4 种导入子资产各 1 个）。
fn make_fixture(temp: &TempDir) -> Fixture {
    let parent = id(PARENT_GLTF);
    let mesh = parent.derive_child(AssetKind::Mesh, "mesh:0");
    let material = parent.derive_child(AssetKind::Material, "material:0");
    let skeleton = parent.derive_child(AssetKind::Skeleton, "skeleton:0");
    let animation = parent.derive_child(AssetKind::Animation, "animation:0");
    let output = format!("imported/{}.json", parent.simple());

    let scene = ImportedScene {
        format: ImportSourceFormat::Gltf,
        generator: None,
        nodes: Vec::new(),
        meshes: vec![ImportedMesh {
            name: Some("HeroMesh".to_string()),
            primitives: Vec::new(),
        }],
        materials: vec![ImportedMaterial {
            name: Some("HeroMaterial".to_string()),
            ..ImportedMaterial::default()
        }],
        textures: Vec::new(),
        skeletons: vec![ImportedSkeleton {
            name: Some("HeroSkeleton".to_string()),
            joints: Vec::new(),
            root_joint: None,
        }],
        animations: vec![ImportedAnimation {
            name: Some("Idle".to_string()),
            channels: Vec::new(),
            duration: 1.0,
        }],
        external_files: Vec::new(),
        warnings: Vec::new(),
    };
    temp.write(
        &format!("assets/{output}"),
        serde_json::to_string(&scene).expect("序列化 ImportedScene").as_bytes(),
    );

    let entry = |id: AssetId, kind: AssetKind, locator: &str| CookEntry {
        id,
        kind,
        locator: AssetLocator::parse(locator).expect("locator"),
        output: output.clone(),
        dependencies: Vec::new(),
        source_size: 0,
        source_hash: "0".repeat(16),
    };
    let cook = CookManifest {
        format_version: 1,
        roots: vec![id(SCENE_ID), parent],
        entries: vec![
            CookEntry {
                id: id(SCENE_ID),
                kind: AssetKind::Scene,
                locator: AssetLocator::parse("assets/scenes/main.scene").unwrap(),
                output: "scenes/main.scene".to_string(),
                dependencies: Vec::new(),
                source_size: 2,
                source_hash: "0".repeat(16),
            },
            CookEntry {
                id: parent,
                kind: AssetKind::Gltf,
                locator: AssetLocator::parse("assets/models/hero.gltf").unwrap(),
                output: output.clone(),
                dependencies: Vec::new(),
                source_size: 0,
                source_hash: "0".repeat(16),
            },
            entry(mesh, AssetKind::Mesh, "assets/models/hero.gltf#mesh:0"),
            entry(
                material,
                AssetKind::Material,
                "assets/models/hero.gltf#material:0",
            ),
            entry(
                skeleton,
                AssetKind::Skeleton,
                "assets/models/hero.gltf#skeleton:0",
            ),
            entry(
                animation,
                AssetKind::Animation,
                "assets/models/hero.gltf#animation:0",
            ),
        ],
    };

    let config = PlayerBuildConfig::new("Imported Fixture", "1.0.0", "scenes/main.scene", host())
        .with_required_font(None::<String>);
    let manifest = PlayerManifest::from_parts(
        &config,
        Vec::new(),
        Vec::new(),
        "scenes/main.scene",
        None,
    );

    Fixture {
        assets_dir: temp.join("assets"),
        cook,
        manifest,
        parent,
        mesh,
        material,
        skeleton,
        animation,
        output,
    }
}

fn index_of(fixture: &Fixture) -> PlayerAssetIndex {
    PlayerAssetIndex::from_cook_manifest(&fixture.cook, &fixture.manifest).expect("资产索引")
}

#[test]
fn loads_imported_bundle_and_maps_all_sub_asset_kinds() {
    let temp = TempDir::new("imported-mapping");
    let fixture = make_fixture(&temp);
    let index = index_of(&fixture);
    // glTF 子资产与父 entry 共享 output；索引允许共享 output（文件级仍唯一）。
    assert_eq!(index.len(), 6);

    let bundle = ImportedAssetBundle::load(&fixture.assets_dir, &index).expect("加载 bundle");
    assert_eq!(bundle.scene_count(), 1);
    assert!(!bundle.is_empty());
    let stats = bundle.stats();
    assert_eq!(stats.scenes, 1);
    assert_eq!(stats.meshes, 1);
    assert_eq!(stats.materials, 1);
    assert_eq!(stats.textures, 0);
    assert_eq!(stats.skeletons, 1);
    assert_eq!(stats.animations, 1);
    assert_eq!(stats.mapped_sub_assets, 4);
    assert_eq!(stats.warnings, 0);

    // 查询：AssetId → mesh/material/skeleton/animation + 所属场景。
    assert_eq!(
        bundle.mesh(fixture.mesh).unwrap().name.as_deref(),
        Some("HeroMesh")
    );
    assert_eq!(
        bundle.material(fixture.material).unwrap().name.as_deref(),
        Some("HeroMaterial")
    );
    assert_eq!(
        bundle.skeleton(fixture.skeleton).unwrap().name.as_deref(),
        Some("HeroSkeleton")
    );
    assert_eq!(
        bundle.animation(fixture.animation).unwrap().name.as_deref(),
        Some("Idle")
    );
    assert_eq!(bundle.scene(fixture.parent).map(|s| s.meshes.len()), Some(1));
    assert_eq!(bundle.scene_of(fixture.mesh), Some(fixture.parent));
    assert!(bundle.mesh(id(SCENE_ID)).is_none(), "非导入资产不应出现在 bundle");

    // 摘要与统计。
    let summary = bundle.summary();
    assert!(summary.contains("1 个导入场景"), "summary: {summary}");
    assert!(summary.contains("mesh 1 / material 1"), "summary: {summary}");
    assert!(summary.contains("skeleton 1 / animation 1"), "summary: {summary}");
    assert_eq!(bundle.summary_data().stats.mapped_sub_assets, 4);
}

#[test]
fn kind_mismatch_between_locator_and_entry_is_rejected() {
    let temp = TempDir::new("imported-kind-mismatch");
    let mut fixture = make_fixture(&temp);
    // 把 mesh entry 的 locator 改成 material:0 → locator kind 与 entry.kind 不一致。
    for entry in &mut fixture.cook.entries {
        if entry.id == fixture.mesh {
            entry.locator =
                AssetLocator::parse("assets/models/hero.gltf#material:0").unwrap();
        }
    }
    let index = index_of(&fixture);
    let error = ImportedAssetBundle::load(&fixture.assets_dir, &index)
        .expect_err("kind 不一致必须失败");
    assert!(
        matches!(error, ImportedAssetError::KindMismatch { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn sub_asset_index_out_of_range_is_rejected() {
    let temp = TempDir::new("imported-index-range");
    let mut fixture = make_fixture(&temp);
    for entry in &mut fixture.cook.entries {
        if entry.id == fixture.skeleton {
            entry.locator =
                AssetLocator::parse("assets/models/hero.gltf#skeleton:3").unwrap();
        }
    }
    let index = index_of(&fixture);
    let error = ImportedAssetBundle::load(&fixture.assets_dir, &index)
        .expect_err("越界 index 必须失败");
    assert!(
        matches!(
            error,
            ImportedAssetError::IndexOutOfRange {
                kind: AssetKind::Skeleton,
                index: 3,
                len: 1,
                ..
            }
        ),
        "实际错误：{error}"
    );
}

#[test]
fn cook_manifest_and_imported_scene_count_mismatch_is_rejected() {
    let temp = TempDir::new("imported-count-mismatch");
    let mut fixture = make_fixture(&temp);
    // 删除 animation entry：场景有 1 个 animation，manifest 只有 0 个 → 严格计数失败。
    fixture
        .cook
        .entries
        .retain(|entry| entry.id != fixture.animation);
    let index = index_of(&fixture);
    let error = ImportedAssetBundle::load(&fixture.assets_dir, &index)
        .expect_err("数量不一致必须失败");
    assert!(
        matches!(
            error,
            ImportedAssetError::CountMismatch {
                kind: AssetKind::Animation,
                expected: 0,
                actual: 1,
                ..
            }
        ),
        "实际错误：{error}"
    );
}

#[test]
fn bad_imported_json_is_rejected_with_readable_error() {
    let temp = TempDir::new("imported-bad-json");
    let fixture = make_fixture(&temp);
    let path = fixture
        .assets_dir
        .join(hezhou_player::to_native_path(&fixture.output, host()));
    std::fs::write(&path, b"{ not imported scene").unwrap();
    let index = index_of(&fixture);

    let error = ImportedAssetBundle::load(&fixture.assets_dir, &index)
        .expect_err("坏 JSON 必须失败");
    assert!(
        matches!(error, ImportedAssetError::Parse { .. }),
        "实际错误：{error}"
    );
    assert!(error.to_string().contains("解析 imported 场景"));
}

#[test]
fn unreferenced_imported_files_are_warned_not_fatal() {
    let temp = TempDir::new("imported-orphan");
    let fixture = make_fixture(&temp);
    temp.write("assets/imported/orphan.json", b"{\"junk\":true}");
    let index = index_of(&fixture);

    let bundle = ImportedAssetBundle::load(&fixture.assets_dir, &index).expect("孤儿文件只告警");
    assert_eq!(bundle.scene_count(), 1);
    assert_eq!(bundle.stats().warnings, 1);
    assert!(
        bundle.warnings()[0].contains("未被 cook manifest 引用"),
        "warnings: {:?}",
        bundle.warnings()
    );
}

#[test]
fn empty_index_yields_empty_bundle() {
    let temp = TempDir::new("imported-empty");
    let config = PlayerBuildConfig::new("Legacy", "1.0.0", "scenes/main.scene", host())
        .with_required_font(None::<String>);
    let manifest = PlayerManifest::from_parts(
        &config,
        Vec::new(),
        Vec::new(),
        "scenes/main.scene",
        None,
    );
    let index = PlayerAssetIndex::from_player_manifest(&manifest).expect("空索引");
    let bundle = ImportedAssetBundle::load(temp.path(), &index).expect("空 bundle");
    assert!(bundle.is_empty());
    assert_eq!(bundle.stats().scenes, 0);
    assert!(bundle.summary().contains("无"));
}

#[test]
fn runtime_validates_cooked_imported_bundle_and_logs_stats() {
    // 用 cooked fixture 构建真实包；--validate 应加载 imported bundle 并记录统计。
    let temp = TempDir::new("imported-runtime");
    let fixture = cooked_fixture_dir();
    let config_text = std::fs::read_to_string(fixture.join("build-config.json")).unwrap();
    let input = PlayerBuildInput {
        config: serde_json::from_str(&config_text).unwrap(),
        cook: PlayerCookManifest::default(),
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

    // 通过库 API 加载索引 + bundle，确认与运行时同一路径。
    let manifest = PlayerManifest::load(&output.manifest_path).unwrap();
    let index = PlayerAssetIndex::load(&output.output_dir, &manifest).unwrap();
    let bundle = ImportedAssetBundle::load(&manifest.assets_dir(&output.output_dir), &index)
        .expect("加载 cooked bundle");
    assert_eq!(bundle.scene_count(), 1, "cooked fixture 有 1 个 gltf imported 场景");
    assert_eq!(bundle.stats().meshes, 0);
    assert!(bundle.summary().contains("1 个导入场景"));

    // 篡改 imported JSON → 严格校验失败（可读错误）。
    let imported_path = output.output_dir.join("assets/imported/33333333333343338333333333333333.json");
    std::fs::write(&imported_path, b"broken").unwrap();
    let error = ImportedAssetBundle::load(&manifest.assets_dir(&output.output_dir), &index)
        .expect_err("坏 imported JSON 必须失败");
    assert!(matches!(error, ImportedAssetError::Parse { .. }));
}
