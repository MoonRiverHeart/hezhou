//! 资产数据库 / `.meta` / 依赖图 / 工程移动 集成测试。

mod common;

use std::path::Path;

use hezhou_asset::error::AssetError;
use hezhou_asset::id::{AssetKind, MeshRef, TextureRef};
use hezhou_asset::meta::{meta_path_for, AssetMeta, ImportSettings, TextureImportSettings};
use hezhou_asset::path::LogicalPath;
use hezhou_asset::{AssetDatabase, LogicalPath as ReExportedLogicalPath};

fn temp_project() -> tempfile::TempDir {
    common::tempdir()
}

fn write_asset(project: &Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
    common::write_project_asset(project, relative, bytes)
}

#[test]
fn scan_creates_meta_and_uuid_is_stable_across_reopen() {
    let temp = temp_project();
    let project = temp.path();
    write_asset(project, "assets/models/hero.gltf", b"gltf-placeholder");
    write_asset(project, "assets/audio/hit.wav", b"RIFF");

    let db = AssetDatabase::open(project).unwrap();
    assert_eq!(db.len(), 2);
    let hero_path = LogicalPath::new("assets/models/hero.gltf").unwrap();
    let hero_id = db.id_of(&hero_path).unwrap();
    assert!(meta_path_for(&project.join("assets/models/hero.gltf")).is_file());

    // 重新打开工程：UUID 不变（来自 .meta，而不是随机重建）
    drop(db);
    let db2 = AssetDatabase::open(project).unwrap();
    assert_eq!(db2.id_of(&hero_path).unwrap(), hero_id);
    assert_eq!(db2.logical_path_of(hero_id).unwrap(), &hero_path);

    // 逻辑路径大小写/分隔符不敏感
    let weird = LogicalPath::new("Assets\\Models\\HERO.GLTF").unwrap();
    assert_eq!(db2.id_of(&weird).unwrap(), hero_id);
}

#[test]
fn moving_project_directory_keeps_references() {
    let temp = temp_project();
    let project = temp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    write_asset(&project, "assets/a/hero.png", b"png");
    write_asset(&project, "assets/b/mat.mat", b"mat");

    let mut db = AssetDatabase::open(&project).unwrap();
    let hero_path = LogicalPath::new("assets/a/hero.png").unwrap();
    let mat_path = LogicalPath::new("assets/b/mat.mat").unwrap();
    let hero_id = db.id_of(&hero_path).unwrap();
    let mat_id = db.id_of(&mat_path).unwrap();
    // 材质引用纹理（方向：mat -> hero）
    db.set_dependencies(mat_id, [hero_id]).unwrap();
    let closure_before = db.dependency_closure(&[mat_id]).unwrap();
    assert_eq!(closure_before, vec![hero_id, mat_id]);

    // 整个工程目录改名移动
    let moved = temp.path().join("proj_moved");
    std::fs::rename(&project, &moved).unwrap();

    let db2 = AssetDatabase::open(&moved).unwrap();
    assert_eq!(db2.id_of(&hero_path).unwrap(), hero_id);
    assert_eq!(db2.id_of(&mat_path).unwrap(), mat_id);
    assert_eq!(db2.dependency_closure(&[mat_id]).unwrap(), closure_before);
}

#[test]
fn moving_asset_directory_updates_path_but_keeps_id() {
    let temp = temp_project();
    let project = temp.path();
    write_asset(project, "assets/models/hero.gltf", b"gltf-placeholder");

    let mut db = AssetDatabase::open(project).unwrap();
    let old_path = LogicalPath::new("assets/models/hero.gltf").unwrap();
    let id = db.id_of(&old_path).unwrap();

    // 资产目录整体改名（.meta 跟随移动）
    std::fs::rename(
        project.join("assets/models"),
        project.join("assets/characters"),
    )
    .unwrap();
    let report = db.scan().unwrap();
    assert_eq!(report.asset_count, 1);

    let new_path = LogicalPath::new("assets/characters/hero.gltf").unwrap();
    assert_eq!(db.id_of(&new_path).unwrap(), id);
    assert!(db.id_of(&old_path).is_none(), "旧路径映射应被移除");
    assert_eq!(db.logical_path_of(id).unwrap(), &new_path);
}

#[test]
fn typed_refs_validate_kind() {
    let temp = temp_project();
    let project = temp.path();
    write_asset(project, "assets/meshes/quad.obj", b"obj");
    write_asset(project, "assets/textures/albedo.png", b"png");
    let db = AssetDatabase::open(project).unwrap();

    let mesh_id = db
        .id_of(&LogicalPath::new("assets/meshes/quad.obj").unwrap())
        .unwrap();
    let texture_id = db
        .id_of(&LogicalPath::new("assets/textures/albedo.png").unwrap())
        .unwrap();

    let mesh_ref: MeshRef = db.typed_ref(mesh_id).unwrap();
    assert_eq!(mesh_ref.id(), mesh_id);
    let texture_ref: TextureRef = db.typed_ref(texture_id).unwrap();
    assert_eq!(texture_ref.kind(), AssetKind::Texture);

    let error = db.typed_ref::<hezhou_asset::Mesh>(texture_id).unwrap_err();
    match error {
        AssetError::KindMismatch { expected, actual } => {
            assert_eq!(expected, AssetKind::Mesh);
            assert_eq!(actual, AssetKind::Texture);
        }
        other => panic!("期望 KindMismatch，实际 {other:?}"),
    }
}

#[test]
fn import_settings_roundtrip_through_scan() {
    let temp = temp_project();
    let project = temp.path();
    let png = write_asset(project, "assets/tex/albedo.png", b"png");
    let mut meta = AssetMeta::create(AssetKind::Texture);
    meta.import_settings = ImportSettings::Texture(TextureImportSettings {
        srgb: false,
        generate_mipmaps: false,
        ..Default::default()
    });
    meta.save(&meta_path_for(&png)).unwrap();

    let db = AssetDatabase::open(project).unwrap();
    let id = db
        .id_of(&LogicalPath::new("assets/tex/albedo.png").unwrap())
        .unwrap();
    let settings = db.entry(id).unwrap().import_settings.clone();
    let texture = settings.texture().expect("应为纹理导入设置");
    assert!(!texture.srgb);
    assert!(!texture.generate_mipmaps);
}

#[test]
fn dependency_closure_is_topological_and_cycle_is_rejected() {
    let temp = temp_project();
    let project = temp.path();
    for name in ["assets/a.bin", "assets/b.bin", "assets/c.bin"] {
        write_asset(project, name, name.as_bytes());
    }
    let mut db = AssetDatabase::open(project).unwrap();
    let a = db
        .id_of(&LogicalPath::new("assets/a.bin").unwrap())
        .unwrap();
    let b = db
        .id_of(&LogicalPath::new("assets/b.bin").unwrap())
        .unwrap();
    let c = db
        .id_of(&LogicalPath::new("assets/c.bin").unwrap())
        .unwrap();

    db.set_dependencies(b, [a]).unwrap();
    db.set_dependencies(c, [b]).unwrap();

    let closure = db.dependency_closure(&[c, a]).unwrap();
    assert_eq!(closure, vec![a, b, c], "依赖在前，引用者在后");
    assert_eq!(db.dependency_closure(&[c, a]).unwrap(), closure);

    // 成环检测
    db.set_dependencies(a, [c]).unwrap();
    let error = db.dependency_closure(&[c]).unwrap_err();
    assert!(matches!(error, AssetError::DependencyCycle(_)));
}

#[test]
fn orphan_meta_produces_warning() {
    let temp = temp_project();
    let project = temp.path();
    let png = project.join("assets/tex/gone.png");
    std::fs::create_dir_all(png.parent().unwrap()).unwrap();
    let meta = AssetMeta::create(AssetKind::Texture);
    meta.save(&meta_path_for(&png)).unwrap();

    let db = AssetDatabase::open(project).unwrap();
    assert!(db.is_empty());
    assert!(
        db.warnings()
            .iter()
            .any(|warning| warning.contains("孤立 meta")),
        "告警: {:?}",
        db.warnings()
    );
}

#[test]
fn re_exported_types_are_usable_from_crate_root() {
    let path: ReExportedLogicalPath = ReExportedLogicalPath::new("assets/x.bin").unwrap();
    assert_eq!(path.as_str(), "assets/x.bin");
}
