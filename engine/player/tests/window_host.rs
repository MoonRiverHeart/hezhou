//! `window-host` feature 下的窗口渲染 host 测试。
//!
//! 默认 feature 关闭时整个文件不编译（0 测试）；`cargo test -p hezhou-player --features window-host`
//! 会编译本文件。真实打开窗口的测试标记为 `#[ignore]`（CI 无 GPU/显示环境），
//! 本机验证命令见 README §6.3。

#![cfg(feature = "window-host")]

mod common;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use common::{cooked_fixture_dir, TempDir};
use hezhou_player::builder::{
    BuildSources, CookManifest, PlayerBuildInput, PlayerBuilder, RuntimePayload, ScriptAssembly,
};
use hezhou_player::config::PlayerBuildConfig;
use hezhou_player::cook_bridge::CookedAssets;
use hezhou_player::host::{resolve_host, DEFAULT_HOST_NAME};
use hezhou_player::platform::TargetPlatform;
use hezhou_player::window_host::{
    parse_frame_limit, WindowPlayerHost, DEFAULT_FRAME_LIMIT, FRAME_LIMIT_ENV, MAX_FRAME_LIMIT,
    WINDOW_HOST_NAME,
};

const PLAYER_BIN: &str = env!("CARGO_BIN_EXE_hezhou-player");

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

#[test]
fn window_host_is_selected_by_default_and_explicitly() {
    assert_eq!(WINDOW_HOST_NAME, "window");
    assert_eq!(
        resolve_host(DEFAULT_HOST_NAME).unwrap().name(),
        WINDOW_HOST_NAME
    );
    assert_eq!(
        resolve_host("window").unwrap().name(),
        WINDOW_HOST_NAME
    );
    // headless/null 仍可用于无 GPU 环境。
    assert_eq!(resolve_host("headless").unwrap().name(), "default");
    assert_eq!(resolve_host("null").unwrap().name(), "default");
}

#[test]
fn frame_limit_contract() {
    assert_eq!(parse_frame_limit("5"), Some(5));
    assert_eq!(parse_frame_limit("0"), None);
    assert_eq!(parse_frame_limit("junk"), None);
    assert_eq!(parse_frame_limit("999999999"), Some(MAX_FRAME_LIMIT));
    assert_eq!(WindowPlayerHost::with_frame_limit(5).frame_limit(), 5);
    assert_eq!(WindowPlayerHost::new().frame_limit(), DEFAULT_FRAME_LIMIT);
}

/// 真实窗口渲染冒烟：需要显示环境 + Vulkan GPU，CI 默认跳过。
///
/// 手工运行：
/// ```bash
/// cd engine
/// cargo test -p hezhou-player --features window-host --test window_host -- --ignored --nocapture
/// ```
#[ignore = "需要显示环境 + Vulkan GPU；CI 默认跳过，本机手工验证"]
#[test]
fn actual_window_host_renders_fixed_frames() {
    let temp = TempDir::new("window-host-smoke");
    let fixture = cooked_fixture_dir();
    let config_text = std::fs::read_to_string(fixture.join("build-config.json")).unwrap();
    let config: PlayerBuildConfig = serde_json::from_str(&config_text).unwrap();
    let input = PlayerBuildInput {
        config,
        cook: CookManifest::default(),
        scripts: ScriptAssembly::from_dir(&fixture.join("scripts")).unwrap(),
        runtime: RuntimePayload::from_dir(&fixture.join("runtime")).unwrap(),
        cooked: Some(CookedAssets::load(&fixture.join("cook_manifest.json")).unwrap()),
    };
    let builder = PlayerBuilder::new(input);
    let sources = BuildSources {
        executable: Some(PLAYER_BIN.into()),
        assets_root: None,
    };
    let output = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect("构建 fixture Player 包");
    let package = output.output_dir;
    let executable = package.join(host().executable_file_name("cooked-fixture"));
    let log_dir = temp.join("logs");

    let mut args: Vec<OsString> = vec![
        "--run".into(),
        "--host".into(),
        "window".into(),
        "--player-root".into(),
        package.as_os_str().to_owned(),
        "--manifest".into(),
        package.join("manifest.json").into(),
        "--log-dir".into(),
        log_dir.as_os_str().to_owned(),
    ];
    let mut command = Command::new(&executable);
    command.args(std::mem::take(&mut args));
    command.env(FRAME_LIMIT_ENV, "5");
    let result = command.output().expect("运行窗口 host");
    assert!(
        result.status.success(),
        "窗口 host 失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.status.code(), Some(0));

    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("读取 player.log");
    assert!(log.contains("WindowPlayerHost：创建窗口"), "日志：{log}");
    assert!(
        log.contains("窗口与 Vulkan 渲染器创建成功"),
        "日志：{log}"
    );
    assert!(log.contains("内置测试场景：cube entity="), "日志：{log}");
    assert!(log.contains("窗口 host 结束：5 帧"), "日志：{log}");
    assert!(log.contains("达到固定帧数"), "日志：{log}");
}

/// 确保包构建路径在 window-host feature 下也编译（不打开窗口）。
#[allow(dead_code)]
fn _package_path_type(_: PathBuf) {}
