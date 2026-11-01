//! mono-host 纯逻辑测试（不初始化 Mono）：脚本路径定位/错误安全。
//!
//! 真实 C# 调用测试在 `tests/mono_host_call.rs`（独立进程，避免 Mono domain 多次初始化）。

#![cfg(feature = "mono-host")]

mod common;

use common::TempDir;
use hezhou_player::manifest::PlayerManifest;
use hezhou_player::mono_host::{find_player_game_script, MonoHostError, PlayerGameScript};
use hezhou_player::platform::TargetPlatform;
use hezhou_player::{PlayerBuildConfig, SCRIPTS_DIR};

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

fn manifest_with_scripts(scripts: Vec<&str>) -> PlayerManifest {
    let config = PlayerBuildConfig::new("Mono Fixture", "1.0.0", "scenes/main.scene", host())
        .with_required_font(None::<String>);
    PlayerManifest::from_parts(
        &config,
        Vec::new(),
        scripts.into_iter().map(str::to_string).collect(),
        "scenes/main.scene",
        None,
    )
}

#[test]
fn missing_dll_reports_readable_error_without_loading_mono() {
    let temp = TempDir::new("mono-missing");
    let error = PlayerGameScript::load(&temp.join("nope.dll")).expect_err("不存在的 DLL 必须失败");
    assert!(
        matches!(error, MonoHostError::MissingScript { .. }),
        "实际错误：{error}"
    );
    assert!(error.to_string().contains("脚本 DLL 不存在"));
}

#[test]
fn finds_first_dll_in_manifest_scripts() {
    let temp = TempDir::new("mono-find");
    let root = temp.path();
    let manifest = manifest_with_scripts(vec!["notes.txt", "PlayerGame.dll", "Other.dll"]);
    let found = find_player_game_script(&manifest, root)
        .expect("路径解析")
        .expect("应找到 DLL");
    assert_eq!(found.0, "PlayerGame.dll");
    assert_eq!(found.1, root.join(SCRIPTS_DIR).join("PlayerGame.dll"));
}

#[test]
fn no_dll_returns_none_for_legacy_package() {
    let temp = TempDir::new("mono-none");
    let manifest = manifest_with_scripts(vec!["readme.txt"]);
    assert!(find_player_game_script(&manifest, temp.path())
        .expect("路径解析")
        .is_none());
}

#[test]
fn invalid_script_path_is_rejected() {
    let temp = TempDir::new("mono-invalid");
    let manifest = manifest_with_scripts(vec!["../escape.dll"]);
    let error = find_player_game_script(&manifest, temp.path()).expect_err("非法路径必须失败");
    assert!(
        matches!(error, MonoHostError::InvalidScriptPath { .. }),
        "实际错误：{error}"
    );
}
