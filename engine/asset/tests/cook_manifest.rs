//! cook manifest 集成测试：依赖闭包、确定性、只包含被引用资产、产物写出。

mod common;

use std::path::Path;

use hezhou_asset::cook::{COOK_MANIFEST_FILE, COOK_MANIFEST_FORMAT_VERSION};
use hezhou_asset::error::AssetError;
use hezhou_asset::import::gltf::{import_gltf_file, is_enabled};
use hezhou_asset::meta::GltfImportSettings;
use hezhou_asset::path::LogicalPath;
use hezhou_asset::{
    fnv1a64_hex, validate_imported_scene, AssetDatabase, AssetKind, CookManifest, Cooker,
    ImportedImage, ImportedScene,
};

fn setup_glb_project(project: &Path) -> (AssetDatabase, hezhou_asset::AssetId) {
    common::write_project_asset(project, "assets/models/hero.glb", &common::minimal_glb());
    common::write_project_asset(project, "assets/audio/hit.wav", b"RIFF-unused");
    let mut db = AssetDatabase::open(project).unwrap();
    let hero_id = db
        .id_of(&LogicalPath::new("assets/models/hero.glb").unwrap())
        .unwrap();
    let outcome = db.import_gltf(hero_id).unwrap();
    let mesh_id = outcome.sub_asset(AssetKind::Mesh, 0).unwrap().id;
    (db, mesh_id)
}

#[test]
fn plan_contains_only_referenced_assets_and_is_deterministic() {
    if !is_enabled() {
        eprintln!("跳过：未启用 gltf feature");
        return;
    }
    let temp = common::tempdir();
    let project = temp.path();
    let (db, mesh_id) = setup_glb_project(project);
    let cooker = Cooker::new(&db);

    let manifest = cooker.plan(&[mesh_id]).unwrap();
    assert_eq!(manifest.format_version, COOK_MANIFEST_FORMAT_VERSION);
    assert_eq!(manifest.roots, vec![mesh_id]);

    let locators: Vec<String> = manifest
        .entries
        .iter()
        .map(|entry| entry.locator.to_string())
        .collect();
    // mesh 的依赖闭包：父 glTF + material + texture + skeleton
    assert!(locators.contains(&"assets/models/hero.glb#mesh:0".to_string()));
    assert!(locators.contains(&"assets/models/hero.glb".to_string()));
    assert!(locators.contains(&"assets/models/hero.glb#material:0".to_string()));
    assert!(locators.contains(&"assets/models/hero.glb#texture:0".to_string()));
    assert!(locators.contains(&"assets/models/hero.glb#skeleton:0".to_string()));
    // 未被引用的音频与动画不进包
    assert!(!locators.iter().any(|locator| locator.contains("hit.wav")));
    assert!(!locators
        .iter()
        .any(|locator| locator.contains("#animation:0")));
    assert_eq!(manifest.entries.len(), 5);

    // 确定性：重复 plan 与“重新打开工程后再 plan”逐字节一致
    let json_a = manifest.to_json().unwrap();
    let json_b = cooker.plan(&[mesh_id]).unwrap().to_json().unwrap();
    assert_eq!(json_a, json_b);

    drop(db);
    let reopened = AssetDatabase::open(project).unwrap();
    let json_c = Cooker::new(&reopened)
        .plan(&[mesh_id])
        .unwrap()
        .to_json()
        .unwrap();
    assert_eq!(json_a, json_c, "跨扫描/进程的 manifest 必须一致");

    // manifest 文件 roundtrip
    let manifest_path = temp.path().join("out").join(COOK_MANIFEST_FILE);
    manifest.save(&manifest_path).unwrap();
    let loaded = CookManifest::load(&manifest_path).unwrap();
    assert_eq!(loaded, manifest);
    assert_eq!(std::fs::read_to_string(&manifest_path).unwrap(), json_a);
}

#[test]
fn plan_records_source_metadata_and_dependency_edges() {
    if !is_enabled() {
        return;
    }
    let temp = common::tempdir();
    let project = temp.path();
    let (db, mesh_id) = setup_glb_project(project);
    let gltf_bytes = common::minimal_glb();
    let manifest = Cooker::new(&db).plan(&[mesh_id]).unwrap();

    let mesh_entry = manifest.entry(mesh_id).unwrap();
    assert_eq!(mesh_entry.kind, AssetKind::Mesh);
    assert_eq!(mesh_entry.source_size, gltf_bytes.len() as u64);
    assert_eq!(mesh_entry.source_hash, fnv1a64_hex(&gltf_bytes));
    assert!(!mesh_entry.dependencies.is_empty());

    // 输出路径：同一 glTF 的所有子资产共享一份导入产物，因此本工程只有 1 个唯一输出。
    let mut outputs: Vec<&str> = manifest
        .entries
        .iter()
        .map(|entry| entry.output.as_str())
        .collect();
    outputs.sort_unstable();
    outputs.dedup();
    assert_eq!(outputs.len(), 1, "子资产共享父 glTF 的导入产物");
    assert!(mesh_entry.output.starts_with("imported/"));
    assert!(!mesh_entry.output.contains('\\'));
    // 每个 glTF 子资产都指向父文件的导入产物
    let gltf_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.kind == AssetKind::Gltf)
        .unwrap();
    for entry in &manifest.entries {
        if entry.locator.is_sub_asset() {
            assert_eq!(entry.output, gltf_entry.output);
        }
    }

    // 动画作为根时只需动画 + 父文件
    let animation_id = db
        .entries()
        .find(|entry| entry.kind == AssetKind::Animation)
        .unwrap()
        .id;
    let animation_manifest = Cooker::new(&db).plan(&[animation_id]).unwrap();
    assert_eq!(animation_manifest.entries.len(), 2);
}

#[test]
fn cook_writes_import_artifact_and_manifest() {
    if !is_enabled() {
        return;
    }
    let temp = common::tempdir();
    let project = temp.path();
    let (db, mesh_id) = setup_glb_project(project);
    let cooker = Cooker::new(&db);
    let manifest = cooker.plan(&[mesh_id]).unwrap();
    let out_dir = temp.path().join("cooked_player");

    let output = cooker.cook_to_dir(&manifest, &out_dir).unwrap();
    assert_eq!(output.imported_files, 1, "一个 glTF 只写一份导入产物");
    assert!(out_dir.join(COOK_MANIFEST_FILE).is_file());
    let manifest_text = std::fs::read_to_string(out_dir.join(COOK_MANIFEST_FILE)).unwrap();
    assert_eq!(manifest_text, manifest.to_json().unwrap());

    // 导入产物可反序列化回 ImportedScene，且几何/骨骼/动画完整
    let gltf_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.kind == AssetKind::Gltf)
        .unwrap();
    let imported_path = out_dir.join(&gltf_entry.output);
    assert!(imported_path.is_file(), "缺少 {}", imported_path.display());
    let scene: ImportedScene =
        serde_json::from_str(&std::fs::read_to_string(&imported_path).unwrap()).unwrap();
    validate_imported_scene(&scene).unwrap();
    assert_eq!(scene.meshes[0].primitives[0].indices, vec![0, 1, 2]);
    assert_eq!(scene.skeletons[0].joints.len(), 2);
    assert_eq!(scene.animations[0].channels.len(), 2);
    assert!(output.total_bytes > 0);
}

#[test]
fn external_texture_uri_is_rewritten_to_cooked_output() {
    if !is_enabled() {
        return;
    }
    let temp = common::tempdir();
    let project = temp.path();
    let (gltf_path, _, _) = common::minimal_external_gltf(project);
    // 直接导入一次，确认基础数据
    let scene = import_gltf_file(&gltf_path, &GltfImportSettings::default()).unwrap();
    assert!(matches!(
        scene.textures[0].image,
        ImportedImage::ExternalFile { .. }
    ));

    let mut db = AssetDatabase::open(project).unwrap();
    let gltf_id = db
        .id_of(&LogicalPath::new("assets/models/model.gltf").unwrap())
        .unwrap();
    let outcome = db.import_gltf(gltf_id).unwrap();
    let mesh_id = outcome.sub_asset(AssetKind::Mesh, 0).unwrap().id;
    let cooker = Cooker::new(&db);
    let manifest = cooker.plan(&[mesh_id]).unwrap();

    // 外部图片与外部 bin 都进入闭包（只打被引用资产）
    let image_id = db
        .id_of(&LogicalPath::new("assets/models/albedo.png").unwrap())
        .unwrap();
    let image_entry = manifest.entry(image_id).expect("外部图片必须进入 manifest");
    assert_eq!(image_entry.kind, AssetKind::Texture);
    assert!(image_entry.output.starts_with("assets/"));

    let out_dir = temp.path().join("out");
    cooker.cook_to_dir(&manifest, &out_dir).unwrap();
    assert!(out_dir.join(&image_entry.output).is_file());

    // 导入产物里的纹理 URI 被改写为 cooked 相对路径
    let gltf_entry = manifest.entry(gltf_id).unwrap();
    let cooked_scene: ImportedScene =
        serde_json::from_str(&std::fs::read_to_string(out_dir.join(&gltf_entry.output)).unwrap())
            .unwrap();
    match &cooked_scene.textures[0].image {
        ImportedImage::ExternalFile { uri, .. } => {
            assert_eq!(
                uri, &image_entry.output,
                "纹理 URI 应重写为 cooked 输出路径"
            );
        }
        other => panic!("期望外部图片，实际 {other:?}"),
    }
}

#[test]
fn plan_reports_missing_roots_and_cycles() {
    let temp = common::tempdir();
    let project = temp.path();
    common::write_project_asset(project, "assets/a.bin", b"a");
    common::write_project_asset(project, "assets/b.bin", b"b");
    let mut db = AssetDatabase::open(project).unwrap();
    let a = db
        .id_of(&LogicalPath::new("assets/a.bin").unwrap())
        .unwrap();
    let b = db
        .id_of(&LogicalPath::new("assets/b.bin").unwrap())
        .unwrap();
    let cooker = Cooker::new(&db);

    assert!(cooker.plan(&[hezhou_asset::AssetId::new()]).is_err());
    assert!(cooker.plan(&[]).is_err());

    db.set_dependencies(a, [b]).unwrap();
    db.set_dependencies(b, [a]).unwrap();
    let error = Cooker::new(&db).plan(&[a]).unwrap_err();
    match error {
        AssetError::DependencyCycle(path) => assert!(!path.is_empty()),
        other => panic!("期望 DependencyCycle，实际 {other:?}"),
    }
}
