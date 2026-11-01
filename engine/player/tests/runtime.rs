//! `hezhou-player` 运行时 CLI 冒烟测试（启动校验、文件日志、失败诊断）。

mod common;

use std::process::{Command, Output};

use common::{write_player_package, TempDir};
use hezhou_player::platform::TargetPlatform;

const PLAYER_BIN: &str = env!("CARGO_BIN_EXE_hezhou-player");

fn run_player(args: &[String]) -> Output {
    Command::new(PLAYER_BIN)
        .args(args)
        .output()
        .expect("启动 hezhou-player 失败")
}

fn arg(value: &str) -> String {
    value.to_string()
}

#[test]
fn version_flag_prints_engine_version() {
    let output = run_player(&[arg("--version")]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hezhou-player"));
    assert!(stdout.contains(hezhou_player::RUNTIME_ENGINE_VERSION));
}

#[test]
fn validate_only_package_succeeds_and_writes_file_log() {
    let temp = TempDir::new("runtime-ok");
    let root = write_player_package(&temp, TargetPlatform::host());
    let log_dir = temp.join("logs");
    let manifest = root.join("manifest.json");

    let output = run_player(&[
        arg("--validate"),
        arg("--manifest"),
        manifest.display().to_string(),
        arg("--log-dir"),
        log_dir.display().to_string(),
    ]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Windows 无终端场景依赖文件日志，必须有启动摘要。
    let log_path = log_dir.join("player.log");
    let log = std::fs::read_to_string(&log_path).expect("读取 player.log");
    assert!(log.contains("hezhou-player"), "日志内容：{log}");
    assert!(log.contains("Demo Game"), "日志内容：{log}");
    assert!(log.contains("启动检查完成"), "日志内容：{log}");
    assert!(log.contains("scenes"), "日志内容：{log}");
}

#[test]
fn broken_manifest_exits_nonzero_and_writes_startup_error_log() {
    let temp = TempDir::new("runtime-broken");
    let manifest = temp.write("manifest.json", b"{ this is not json }");

    let output = run_player(&[
        arg("--validate"),
        arg("--manifest"),
        manifest.display().to_string(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("启动失败"));

    let error_log = temp.join("logs/startup-error.log");
    let content = std::fs::read_to_string(&error_log).expect("读取 startup-error.log");
    assert!(content.contains("启动失败"), "错误日志：{content}");
}

#[test]
fn platform_mismatch_exits_nonzero() {
    let temp = TempDir::new("runtime-platform");
    let opposite = match TargetPlatform::host() {
        TargetPlatform::Windows => TargetPlatform::Linux,
        TargetPlatform::Linux => TargetPlatform::Windows,
    };
    let root = write_player_package(&temp, opposite);
    let log_dir = temp.join("logs");

    let output = run_player(&[
        arg("--validate"),
        arg("--manifest"),
        root.join("manifest.json").display().to_string(),
        arg("--log-dir"),
        log_dir.display().to_string(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("目标平台"), "stderr: {stderr}");
}

#[test]
fn missing_start_scene_file_exits_nonzero() {
    let temp = TempDir::new("runtime-missing-scene");
    let root = write_player_package(&temp, TargetPlatform::host());
    std::fs::remove_file(root.join("assets").join("scenes").join("main.scene")).unwrap();

    let output = run_player(&[
        arg("--validate"),
        arg("--manifest"),
        root.join("manifest.json").display().to_string(),
        arg("--log-dir"),
        temp.join("logs").display().to_string(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("缺少必需文件"));
}
