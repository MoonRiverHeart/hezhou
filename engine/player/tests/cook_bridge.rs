//! hezhou-asset 真实 cook manifest → Player 包集成测试。

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{cooked_fixture_dir, copy_dir_all, TempDir};
use hezhou_player::builder::{
    ArtifactCategory, BuildError, BuildSources, CookManifest, PlayerBuildInput, PlayerBuilder,
    RuntimePayload, ScriptAssembly,
};
use hezhou_player::config::PlayerBuildConfig;
use hezhou_player::cook_bridge::CookedAssets;
use hezhou_player::manifest::PlayerManifest;
use hezhou_player::platform::TargetPlatform;

const BUILDER_BIN: &str = env!("CARGO_BIN_EXE_hezhou-player-build");
const PLAYER_BIN: &str = env!("CARGO_BIN_EXE_hezhou-player");

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

fn fixture_config(fixture: &Path) -> PlayerBuildConfig {
    let text =
        std::fs::read_to_string(fixture.join("build-config.json")).expect("读取 fixture 构建配置");
    serde_json::from_str(&text).expect("解析 fixture 构建配置")
}

fn fixture_input(fixture: &Path) -> PlayerBuildInput {
    PlayerBuildInput {
        config: fixture_config(fixture),
        cook: CookManifest::default(),
        scripts: ScriptAssembly::from_dir(&fixture.join("scripts")).expect("fixture scripts"),
        runtime: RuntimePayload::from_dir(&fixture.join("runtime")).expect("fixture runtime"),
        cooked: Some(
            CookedAssets::load(&fixture.join("cook_manifest.json")).expect("fixture cook manifest"),
        ),
    }
}

/// 把 cooked fixture 复制到临时目录，便于制造损坏/缺失场景。
fn copy_fixture(temp: &TempDir) -> PathBuf {
    let target = temp.join("cooked");
    copy_dir_all(&cooked_fixture_dir(), &target);
    target
}

#[test]
fn cooked_manifest_maps_assets_and_traceability() {
    let fixture = cooked_fixture_dir();
    let builder = PlayerBuilder::new(fixture_input(&fixture));
    let plan = builder.plan().expect("生成 cooked dry-run 计划");
    let manifest = &plan.manifest;

    assert_eq!(manifest.assets.len(), 4);
    assert_eq!(
        manifest.start_scene,
        "assets/11111111111141118111111111111111.scene"
    );
    assert_eq!(
        manifest.font.as_deref(),
        Some("assets/22222222222242228222222222222222.ttf")
    );

    // 场景：AssetId / locator / output / source 信息一一保留。
    let scene = manifest
        .assets
        .iter()
        .find(|asset| asset.kind == "scene")
        .expect("scene 条目");
    assert_eq!(
        scene.asset_id.as_deref(),
        Some("11111111-1111-4111-8111-111111111111")
    );
    assert_eq!(
        scene.locator.as_deref(),
        Some("assets/scenes/empty.scene")
    );
    assert_eq!(scene.path, scene.output.clone().unwrap());
    assert_eq!(scene.size_bytes, Some(3));
    assert_eq!(scene.source_size, Some(3));
    assert_eq!(scene.source_hash.as_deref(), Some("c735ad1983dc7ddd"));
    assert!(scene
        .checksum
        .as_deref()
        .unwrap_or("")
        .starts_with("fnv1a64:"));

    // glTF：output 在 imported/ 下，依赖保留（texture id）。
    let gltf = manifest
        .assets
        .iter()
        .find(|asset| asset.kind == "gltf")
        .expect("gltf 条目");
    assert!(gltf.path.starts_with("imported/"));
    assert_eq!(
        gltf.dependencies,
        vec!["44444444-4444-4444-8444-444444444444".to_string()]
    );

    // cook_manifest.json 作为 metadata 原样入包。
    assert_eq!(plan.files_in(ArtifactCategory::Asset).len(), 4);
    assert_eq!(plan.files_in(ArtifactCategory::Metadata).len(), 1);
    let metadata = plan
        .file("assets/cook_manifest.json")
        .expect("metadata 文件计划");
    assert_eq!(metadata.category, ArtifactCategory::Metadata);

    manifest.validate().expect("cooked manifest 必须结构合法");
    assert_eq!(plan.fingerprint, builder.plan().unwrap().fingerprint);
}

#[test]
fn missing_cooked_output_is_rejected_before_writing() {
    let temp = TempDir::new("cooked-missing-output");
    let fixture = copy_fixture(&temp);
    let output_file = fixture
        .join("assets")
        .join("11111111111141118111111111111111.scene");
    std::fs::remove_file(&output_file).unwrap();

    let builder = PlayerBuilder::new(fixture_input(&fixture));
    let plan_error = builder.plan().expect_err("缺少 cook 输出必须失败");
    assert!(
        matches!(plan_error, BuildError::MissingCookedOutput { .. }),
        "实际错误：{plan_error}"
    );

    // 真实构建同样在写盘前失败，不产生产物目录。
    let build_out = temp.join("dist/player");
    let sources = BuildSources {
        executable: Some(temp.write("source/player-exe", b"fake")),
        assets_root: None,
    };
    let build_error = builder
        .build(host(), &sources, &build_out)
        .expect_err("缺少 cook 输出必须构建失败");
    assert!(matches!(
        build_error,
        BuildError::MissingCookedOutput { .. }
    ));
    assert!(!build_out.exists());
}

#[test]
fn corrupted_cooked_output_is_rejected() {
    // 校验和不一致（大小不变）。
    let temp = TempDir::new("cooked-bad-checksum");
    let fixture = copy_fixture(&temp);
    std::fs::write(
        fixture
            .join("assets")
            .join("44444444444444448444444444444444.png"),
        b"JPG!",
    )
    .unwrap();
    let error = PlayerBuilder::new(fixture_input(&fixture))
        .plan()
        .expect_err("cook 输出校验和不一致必须失败");
    assert!(
        matches!(error, BuildError::CookedChecksumMismatch { .. }),
        "实际错误：{error}"
    );

    // 大小不一致。
    let temp = TempDir::new("cooked-bad-size");
    let fixture = copy_fixture(&temp);
    let scene = fixture
        .join("assets")
        .join("11111111111141118111111111111111.scene");
    std::fs::write(&scene, b"{}\n+").unwrap();
    let error = PlayerBuilder::new(fixture_input(&fixture))
        .plan()
        .expect_err("cook 输出大小不一致必须失败");
    assert!(
        matches!(error, BuildError::CookedSizeMismatch { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn unsupported_cook_format_is_rejected() {
    // 直接反序列化绕过 asset crate 的 load 版本检查，验证 Player 侧也拒绝。
    let fixture = cooked_fixture_dir();
    let text = std::fs::read_to_string(fixture.join("cook_manifest.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["format_version"] = serde_json::json!(2);
    let manifest: hezhou_asset::cook::CookManifest = serde_json::from_value(value).unwrap();
    let mut input = fixture_input(&fixture);
    input.cooked = Some(CookedAssets::from_parts(
        manifest,
        fixture.join("."),
    ));
    let error = PlayerBuilder::new(input)
        .plan()
        .expect_err("format_version=2 必须拒绝");
    assert!(
        matches!(
            error,
            BuildError::UnsupportedCookFormat {
                found: 2,
                supported: 1
            }
        ),
        "实际错误：{error}"
    );

    // 走 CookedAssets::load 时 asset crate 也会拒绝。
    let temp = TempDir::new("cooked-format-load");
    let fixture = copy_fixture(&temp);
    let manifest_path = fixture.join("cook_manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    value["format_version"] = serde_json::json!(2);
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&value).unwrap(),
    )
    .unwrap();
    let error = CookedAssets::load(&manifest_path).expect_err("load 也必须拒绝高版本");
    assert!(matches!(error, BuildError::CookManifest { .. }), "实际错误：{error}");
}

#[test]
fn missing_start_scene_and_font_are_rejected() {
    let fixture = cooked_fixture_dir();

    let mut input = fixture_input(&fixture);
    input.config.start_scene = "scenes/missing.scene".to_string();
    let error = PlayerBuilder::new(input)
        .plan()
        .expect_err("cook 中缺少启动场景必须失败");
    assert!(
        matches!(error, BuildError::MissingStartScene { .. }),
        "实际错误：{error}"
    );

    let mut input = fixture_input(&fixture);
    input.config.required_font = Some("fonts/missing.ttf".to_string());
    let error = PlayerBuilder::new(input)
        .plan()
        .expect_err("cook 中缺少字体必须失败");
    assert!(
        matches!(error, BuildError::MissingFont { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn real_build_copies_cooked_layout() {
    let temp = TempDir::new("cooked-real-build");
    let fixture = cooked_fixture_dir();
    let builder = PlayerBuilder::new(fixture_input(&fixture));
    let sources = BuildSources {
        executable: Some(temp.write("source/player-exe", b"fake-player-executable")),
        assets_root: None,
    };
    let output = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect("真实构建 cooked 包");

    let package = &output.output_dir;
    for relative in [
        "manifest.json",
        "assets/cook_manifest.json",
        "assets/assets/11111111111141118111111111111111.scene",
        "assets/assets/22222222222242228222222222222222.ttf",
        "assets/assets/44444444444444448444444444444444.png",
        "assets/imported/33333333333343338333333333333333.json",
    ] {
        let path = package.join(hezhou_player::to_native_path(relative, host()));
        assert!(path.is_file(), "缺少产物：{relative}");
    }

    // 内容与 cook 输出一致（原样复制）。
    assert_eq!(
        std::fs::read(package.join("assets/cook_manifest.json")).unwrap(),
        std::fs::read(fixture.join("cook_manifest.json")).unwrap()
    );
    assert_eq!(
        std::fs::read(package.join(
            "assets/assets/11111111111141118111111111111111.scene"
        ))
        .unwrap(),
        std::fs::read(
            fixture.join("assets/11111111111141118111111111111111.scene")
        )
        .unwrap()
    );

    let manifest = PlayerManifest::load(&output.manifest_path).expect("读取产物清单");
    assert_eq!(
        manifest.assets.len(),
        4,
        "manifest 资产条目应与 cook entries 一致"
    );
    assert!(manifest.assets.iter().all(|asset| asset.asset_id.is_some()));
    manifest
        .validate_for_runtime(host())
        .expect("产物清单必须可被当前平台运行时加载");
}

#[test]
fn cli_cooked_manifest_builds_package_and_validate_passes() {
    let temp = TempDir::new("cooked-cli");
    let fixture = cooked_fixture_dir();
    let build_out = temp.join("dist");
    let args: Vec<OsString> = vec![
        "--config".into(),
        fixture.join("build-config.json").into(),
        "--cook".into(),
        fixture.join("cook_manifest.json").into(),
        "--scripts-dir".into(),
        fixture.join("scripts").into(),
        "--runtime-dir".into(),
        fixture.join("runtime").into(),
        "--exe".into(),
        PLAYER_BIN.into(),
        "--build-out".into(),
        build_out.clone().into(),
    ];
    let output: Output = Command::new(BUILDER_BIN)
        .args(&args)
        .output()
        .expect("启动 hezhou-player-build");
    assert!(
        output.status.success(),
        "cooked CLI 构建失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let package = build_out.join(host().as_str());
    let executable = package.join(host().executable_file_name("cooked-fixture"));
    assert!(executable.is_file(), "缺少产出的 Player 可执行文件");
    assert!(package.join("assets/cook_manifest.json").is_file());
    assert!(package
        .join("assets/imported/33333333333343338333333333333333.json")
        .is_file());

    let manifest = PlayerManifest::load(&package.join("manifest.json")).unwrap();
    assert_eq!(manifest.assets.len(), 4);
    manifest
        .validate_for_runtime(host())
        .expect("CLI 产物清单必须合法");

    // 复制出的 Player 通过启动校验（资产目录布局 = cook output 布局）。
    let log_dir = temp.join("logs");
    let validate = Command::new(&executable)
        .arg("--validate")
        .arg("--manifest")
        .arg(package.join("manifest.json"))
        .arg("--log-dir")
        .arg(&log_dir)
        .output()
        .expect("运行产物 Player");
    assert!(
        validate.status.success(),
        "产物 Player --validate 失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("产物日志");
    assert!(log.contains("启动检查完成"), "日志内容：{log}");
    assert!(log.contains("Cooked Fixture"), "日志内容：{log}");
}
