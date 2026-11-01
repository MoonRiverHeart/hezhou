//! 运行时资产索引（`assets/cook_manifest.json`）与 PlayerHost 接口测试。

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{cooked_fixture_dir, write_player_package, TempDir};
use hezhou_asset::{AssetId, AssetKind};
use hezhou_player::asset_index::{AssetIndexError, PlayerAssetIndex};
use hezhou_player::builder::{
    BuildSources, CookManifest, PlayerBuildInput, PlayerBuilder, RuntimePayload, ScriptAssembly,
};
use hezhou_player::cook_bridge::CookedAssets;
use hezhou_player::host::{resolve_host, HostOutcome, DEFAULT_HOST_NAME, HOST_PENDING_EXIT_CODE};
use hezhou_player::manifest::PlayerManifest;
use hezhou_player::platform::TargetPlatform;

const PLAYER_BIN: &str = env!("CARGO_BIN_EXE_hezhou-player");

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

fn fixture_input() -> PlayerBuildInput {
    let fixture = cooked_fixture_dir();
    let config_text = std::fs::read_to_string(fixture.join("build-config.json")).unwrap();
    PlayerBuildInput {
        config: serde_json::from_str(&config_text).unwrap(),
        cook: CookManifest::default(),
        scripts: ScriptAssembly::from_dir(&fixture.join("scripts")).unwrap(),
        runtime: RuntimePayload::from_dir(&fixture.join("runtime")).unwrap(),
        cooked: Some(CookedAssets::load(&fixture.join("cook_manifest.json")).unwrap()),
    }
}

/// 用 cooked fixture 构建真实 Player 包（exe 为当前测试构建的 hezhou-player 二进制）。
fn build_package(temp: &TempDir) -> (PathBuf, PlayerManifest) {
    let builder = PlayerBuilder::new(fixture_input());
    let sources = BuildSources {
        executable: Some(PLAYER_BIN.into()),
        assets_root: None,
    };
    let output = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect("构建 fixture Player 包");
    let manifest = PlayerManifest::load(&output.manifest_path).expect("读取产物清单");
    (output.output_dir, manifest)
}

fn run_player(args: &[OsString]) -> Output {
    Command::new(PLAYER_BIN)
        .args(args)
        .output()
        .expect("启动 hezhou-player 失败")
}

/// 以指定包为 player-root 运行 Player（flags 如 `--validate`/`--run`/`--host default`）。
fn package_args(package: &Path, log_dir: &Path, flags: &[&str]) -> Vec<OsString> {
    let mut args: Vec<OsString> = flags.iter().map(|flag| (*flag).into()).collect();
    args.push("--player-root".into());
    args.push(package.as_os_str().to_owned());
    args.push("--manifest".into());
    args.push(package.join("manifest.json").into());
    args.push("--log-dir".into());
    args.push(log_dir.as_os_str().to_owned());
    args
}

#[test]
fn loads_index_and_queries_by_id_kind_locator() {
    let temp = TempDir::new("index-queries");
    let (package, manifest) = build_package(&temp);
    let index = PlayerAssetIndex::load(&package, &manifest).expect("加载资产索引");

    assert_eq!(index.len(), 4);
    assert!(!index.is_empty());
    assert!(index.has_cook_manifest());
    assert!(index
        .cook_manifest_path()
        .map(|path| path.is_file())
        .unwrap_or(false));
    assert_eq!(
        index.summary(),
        "资产索引 4 条（来源：包内 cook_manifest.json）"
    );

    // 按 AssetId 查询 scene。
    let scene_id = AssetId::parse("11111111-1111-4111-8111-111111111111").unwrap();
    let scene = index.entry(scene_id).expect("scene 条目");
    assert_eq!(
        scene.output,
        "assets/11111111111141118111111111111111.scene"
    );
    assert_eq!(scene.kind, AssetKind::Scene);
    assert_eq!(
        scene.locator.as_ref().unwrap().to_string(),
        "assets/scenes/empty.scene"
    );

    // start_scene / font 解析。
    assert_eq!(index.start_scene().unwrap().id, scene_id);
    let font = index.font().expect("font 条目");
    assert_eq!(font.kind, AssetKind::Font);
    assert!(font.output.ends_with(".ttf"));

    // 按 kind 查询。
    assert_eq!(index.by_kind(AssetKind::Scene).len(), 1);
    assert_eq!(index.by_kind(AssetKind::Texture).len(), 1);
    assert_eq!(index.by_kind(AssetKind::Gltf).len(), 1);
    assert_eq!(index.by_kind(AssetKind::Font).len(), 1);
    assert!(index.by_kind(AssetKind::Audio).is_empty());

    // 按 locator / output 查询（大小写不敏感）。
    let texture = index
        .entry_by_locator("assets/textures/hero.png")
        .expect("texture locator");
    assert_eq!(texture.kind, AssetKind::Texture);
    assert_eq!(index.by_locator("ASSETS/Textures/Hero.PNG").len(), 1);
    assert!(index.entry_by_output(&scene.output).is_some());
    assert!(index.entry_by_output(&scene.output.to_uppercase()).is_some());
    assert!(index.entry_by_output("assets/not-exist.bin").is_none());

    // glTF 依赖保留（texture AssetId）。
    let gltf = index.by_kind(AssetKind::Gltf)[0];
    assert_eq!(
        gltf.dependencies,
        vec![AssetId::parse("44444444-4444-4444-8444-444444444444").unwrap()]
    );
}

#[test]
fn missing_output_is_rejected() {
    let temp = TempDir::new("index-missing-output");
    let (package, manifest) = build_package(&temp);
    std::fs::remove_file(
        package
            .join("assets")
            .join("assets")
            .join("44444444444444448444444444444444.png"),
    )
    .unwrap();

    let error = PlayerAssetIndex::load(&package, &manifest).expect_err("缺少 output 必须失败");
    assert!(
        matches!(
            error,
            AssetIndexError::MissingOutput { ref output, .. } if output.contains("4444")
        ),
        "实际错误：{error}"
    );

    // 运行时同样以可读错误退出 2（缺失的是非启动场景资产，layout 校验能通过）。
    let log_dir = temp.join("logs");
    let output = run_player(&package_args(&package, &log_dir, &["--validate"]));
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("资产索引 output"));
}

#[test]
fn bad_cook_manifest_json_is_rejected_with_readable_error() {
    let temp = TempDir::new("index-bad-json");
    let (package, manifest) = build_package(&temp);
    std::fs::write(package.join("assets/cook_manifest.json"), b"{ not json").unwrap();

    let error = PlayerAssetIndex::load(&package, &manifest).expect_err("坏 JSON 必须失败");
    assert!(
        matches!(error, AssetIndexError::CookManifest { .. }),
        "实际错误：{error}"
    );
    assert!(error.to_string().contains("读取资产索引"));

    let log_dir = temp.join("logs");
    let output = run_player(&package_args(&package, &log_dir, &["--validate"]));
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("读取资产索引"));
}

#[test]
fn manifest_index_mismatch_is_rejected() {
    let temp = TempDir::new("index-mismatch");
    let (package, mut manifest) = build_package(&temp);
    manifest
        .assets
        .retain(|asset| !asset.path.contains("4444"));
    manifest.save(&package.join("manifest.json")).unwrap();

    let error = PlayerAssetIndex::load(&package, &manifest).expect_err("清单与索引不一致必须失败");
    assert!(
        matches!(error, AssetIndexError::ManifestIndexMismatch { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn default_host_registry_and_outcomes() {
    assert_eq!(DEFAULT_HOST_NAME, "default");
    assert_eq!(HOST_PENDING_EXIT_CODE, 3);

    let host = resolve_host("default").expect("default host");
    assert_eq!(host.name(), "default");
    assert_eq!(resolve_host("headless").unwrap().name(), "default");
    assert!(resolve_host("vulkan-host").is_err());
    let unknown_error = match resolve_host("vulkan-host") {
        Ok(_) => panic!("未知 host 不应解析成功"),
        Err(error) => error,
    };
    assert!(unknown_error.to_string().contains("未知 host"));

    assert_eq!(HostOutcome::Running { frames: 3 }.exit_code(), 0);
    assert_eq!(
        HostOutcome::Pending {
            reason: "todo".to_string()
        }
        .exit_code(),
        3
    );
    assert!(HostOutcome::Pending {
        reason: "todo".to_string()
    }
    .is_pending());
}

#[test]
fn run_flag_loads_index_and_reports_host_pending() {
    let temp = TempDir::new("index-run");
    let (package, _manifest) = build_package(&temp);
    let log_dir = temp.join("logs");

    let output = run_player(&package_args(&package, &log_dir, &["--run"]));
    assert_eq!(
        output.status.code(),
        Some(HOST_PENDING_EXIT_CODE),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("等待接入"), "stdout: {stdout}");
    assert!(stdout.contains("资产索引 4 条"), "stdout: {stdout}");

    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("读取 player.log");
    assert!(log.contains("资产索引 4 条"), "日志：{log}");
    assert!(
        log.contains("资产索引启动场景：AssetId 1111"),
        "日志：{log}"
    );
    assert!(log.contains("等待 game host 接入"), "日志：{log}");
    assert!(
        log.contains("PlayerHost `default` 返回退出码 3"),
        "日志：{log}"
    );
    assert!(log.contains("下一轮接入顺序：platform"), "日志：{log}");
}

#[test]
fn host_flag_selects_host_and_unknown_host_fails() {
    let temp = TempDir::new("index-host-flag");
    let (package, _manifest) = build_package(&temp);
    let log_dir = temp.join("logs");

    // --host default 隐含 --run，返回 3。
    let output = run_player(&package_args(&package, &log_dir, &["--host", "default"]));
    assert_eq!(output.status.code(), Some(HOST_PENDING_EXIT_CODE));

    // 未知 host：可读错误 + 退出 2。
    let output = run_player(&package_args(&package, &log_dir, &["--host", "vulkan-host"]));
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("未知 host"));
}

#[test]
fn validate_still_passes_with_index() {
    let temp = TempDir::new("index-validate");
    let (package, _manifest) = build_package(&temp);
    let log_dir = temp.join("logs");

    let output = run_player(&package_args(&package, &log_dir, &["--validate"]));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("读取 player.log");
    assert!(log.contains("资产索引 4 条"), "日志：{log}");
    assert!(
        log.contains("启动检查完成（资产索引已加载）"),
        "日志：{log}"
    );
}

#[test]
fn legacy_package_without_cook_manifest_uses_empty_index() {
    let temp = TempDir::new("index-legacy");
    let root = write_player_package(&temp, host());
    let manifest = PlayerManifest::load(&root.join("manifest.json")).unwrap();
    let index = PlayerAssetIndex::load(&root, &manifest).expect("legacy 包允许空索引");
    assert!(index.is_empty());
    assert!(!index.has_cook_manifest());
    assert!(index.start_scene().is_none());

    let log_dir = temp.join("logs");
    let output = run_player(&package_args(&root, &log_dir, &["--run"]));
    assert_eq!(output.status.code(), Some(HOST_PENDING_EXIT_CODE));
    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("读取 player.log");
    assert!(log.contains("资产索引为空"), "日志：{log}");
    assert!(log.contains("legacy"), "日志：{log}");
}

#[test]
fn index_rejects_duplicate_outputs() {
    let temp = TempDir::new("index-dup");
    let (package, manifest) = build_package(&temp);
    let text = std::fs::read_to_string(package.join("assets/cook_manifest.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["entries"][1]["output"] = value["entries"][0]["output"].clone();
    let cook: hezhou_asset::cook::CookManifest = serde_json::from_value(value).unwrap();

    let error = PlayerAssetIndex::from_cook_manifest(&cook, &manifest)
        .expect_err("重复 output 必须失败");
    assert!(
        matches!(error, AssetIndexError::DuplicateOutput { .. }),
        "实际错误：{error}"
    );
}
