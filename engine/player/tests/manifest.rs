//! `manifest.json` 读写与校验测试。

mod common;

use common::TempDir;
use hezhou_player::config::{PlayerBuildConfig, DEFAULT_FONT_ASSET};
use hezhou_player::manifest::{
    ManifestAsset, ManifestError, PlayerManifest, MANIFEST_FILE_NAME,
    MANIFEST_FORMAT_VERSION,
};
use hezhou_player::platform::TargetPlatform;
use hezhou_player::RUNTIME_ENGINE_VERSION;

fn sample_manifest() -> PlayerManifest {
    let config = PlayerBuildConfig::new(
        "Demo Game",
        "1.2.3",
        "scenes/main.scene",
        TargetPlatform::host(),
    );
    PlayerManifest::new(
        &config,
        vec![
            ManifestAsset::new("scenes/main.scene", "scene"),
            ManifestAsset::new(DEFAULT_FONT_ASSET, "font"),
        ],
        vec!["Game.dll".to_string()],
    )
}

#[test]
fn manifest_roundtrip_preserves_fields() {
    let temp = TempDir::new("manifest-roundtrip");
    let manifest = sample_manifest();
    let path = temp.join(MANIFEST_FILE_NAME);

    manifest.save(&path).expect("写入清单");
    let loaded = PlayerManifest::load(&path).expect("读取清单");

    assert_eq!(loaded, manifest);
    assert_eq!(loaded.format_version, MANIFEST_FORMAT_VERSION);
    assert_eq!(loaded.engine_version, RUNTIME_ENGINE_VERSION);
    assert_eq!(loaded.target, TargetPlatform::host());
    assert!(loaded.validate().is_ok());
    assert!(loaded.validate_for_runtime(TargetPlatform::host()).is_ok());
}

#[test]
fn load_from_dir_and_unknown_fields_are_rejected() {
    let temp = TempDir::new("manifest-strict");
    let manifest = sample_manifest();
    manifest.save(&temp.join(MANIFEST_FILE_NAME)).unwrap();
    assert_eq!(
        PlayerManifest::load_from_dir(temp.path()).unwrap(),
        manifest
    );

    // 多出未知字段：严格拒绝，避免不同构建器写出"看似兼容"的清单。
    let mut value: serde_json::Value =
        serde_json::from_str(&manifest.to_json_pretty().unwrap()).unwrap();
    value["unexpected_field"] = serde_json::json!(true);
    let strict_path = temp.join("strict-manifest.json");
    std::fs::write(&strict_path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let error = PlayerManifest::load(&strict_path).expect_err("未知字段必须拒绝");
    assert!(matches!(error, ManifestError::Json { .. }), "实际错误：{error}");
}

#[test]
fn unsupported_format_version_is_rejected() {
    let mut manifest = sample_manifest();
    manifest.format_version = MANIFEST_FORMAT_VERSION + 1;
    let error = manifest.validate().expect_err("格式版本不匹配必须拒绝");
    assert!(matches!(
        error,
        ManifestError::UnsupportedFormat { found, supported }
            if found == MANIFEST_FORMAT_VERSION + 1 && supported == MANIFEST_FORMAT_VERSION
    ));
}

#[test]
fn engine_version_mismatch_is_rejected_at_runtime() {
    let mut manifest = sample_manifest();
    // 结构上合法（三段式），但 major.minor 与运行时不同。
    manifest.engine_version = "9.9.0".to_string();
    assert!(manifest.validate().is_ok());

    let error = manifest
        .validate_for_runtime(TargetPlatform::host())
        .expect_err("引擎版本不兼容必须拒绝");
    assert!(
        matches!(error, ManifestError::EngineVersionMismatch { .. }),
        "实际错误：{error}"
    );
    assert!(error.to_string().contains("不兼容"));
}

#[test]
fn platform_mismatch_is_rejected_at_runtime() {
    let mut manifest = sample_manifest();
    manifest.target = match TargetPlatform::host() {
        TargetPlatform::Windows => TargetPlatform::Linux,
        TargetPlatform::Linux => TargetPlatform::Windows,
    };
    let error = manifest
        .validate_for_runtime(TargetPlatform::host())
        .expect_err("跨平台运行必须拒绝");
    assert!(
        matches!(error, ManifestError::PlatformMismatch { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn missing_start_scene_and_font_are_rejected() {
    let mut manifest = sample_manifest();
    manifest
        .assets
        .retain(|asset| asset.path != "scenes/main.scene");
    let error = manifest.validate().expect_err("缺少启动场景必须拒绝");
    assert!(matches!(error, ManifestError::MissingStartScene { .. }));

    let mut manifest = sample_manifest();
    manifest
        .assets
        .retain(|asset| asset.path != DEFAULT_FONT_ASSET);
    let error = manifest.validate().expect_err("缺少必检字体必须拒绝");
    assert!(matches!(error, ManifestError::MissingFont { .. }));
}

#[test]
fn case_collision_between_assets_is_rejected() {
    let mut manifest = sample_manifest();
    // 与 "scenes/main.scene" 仅大小写不同：Windows 上会互相覆盖。
    manifest
        .assets
        .push(ManifestAsset::new("Scenes/Main.scene", "scene"));
    let error = manifest.validate().expect_err("大小写冲突必须拒绝");
    assert!(
        matches!(error, ManifestError::CaseCollision { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn invalid_script_name_is_rejected() {
    let mut manifest = sample_manifest();
    manifest.scripts = vec!["../evil.dll".to_string()];
    let error = manifest.validate().expect_err("路径逃逸脚本名必须拒绝");
    assert!(
        matches!(
            error,
            ManifestError::InvalidPath {
                field: "scripts[]",
                ..
            }
        ),
        "实际错误：{error}"
    );

    let mut manifest = sample_manifest();
    manifest.scripts = vec!["Game.exe".to_string()];
    assert!(manifest.validate().is_err());
}
