//! `window-host` feature 下的窗口渲染 host 测试。
//!
//! 默认 feature 关闭时整个文件不编译（0 测试）；`cargo test -p hezhou-player --features window-host`
//! 会编译本文件。真实打开窗口的测试标记为 `#[ignore]`（CI 无 GPU/显示环境），
//! 本机验证命令见 README §6.3。

#![cfg(feature = "window-host")]

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{cooked_fixture_dir, skinned_fixture_dir, TempDir};
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

/// 用指定 cook fixture 构建 Player 包，返回 `(包目录, 可执行文件 stem)`。
fn build_fixture_package(temp: &TempDir, fixture: &Path) -> (PathBuf, String) {
    build_fixture_package_with_scripts(temp, fixture, None)
}

/// 构建 Player 包；`scripts` 为 `Some` 时覆盖 fixture 自带 scripts（mono-host 测试用）。
fn build_fixture_package_with_scripts(
    temp: &TempDir,
    fixture: &Path,
    scripts: Option<Vec<ScriptAssembly>>,
) -> (PathBuf, String) {
    let config_text = std::fs::read_to_string(fixture.join("build-config.json")).unwrap();
    let config: PlayerBuildConfig = serde_json::from_str(&config_text).unwrap();
    let executable_stem = config.executable_stem().to_string();
    let input = PlayerBuildInput {
        config,
        cook: CookManifest::default(),
        scripts: match scripts {
            Some(scripts) => scripts,
            None => ScriptAssembly::from_dir(&fixture.join("scripts")).unwrap(),
        },
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
    (output.output_dir, executable_stem)
}

fn run_window_host(package: &Path, stem: &str, frame_limit: &str, log_dir: &Path) -> std::process::Output {
    run_window_host_with_env(package, stem, frame_limit, log_dir, &[])
}

fn run_window_host_with_env(
    package: &Path,
    stem: &str,
    frame_limit: &str,
    log_dir: &Path,
    extra_env: &[(&str, &str)],
) -> std::process::Output {
    let executable = package.join(host().executable_file_name(stem));
    let args: Vec<OsString> = vec![
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
    command.args(args).env(FRAME_LIMIT_ENV, frame_limit);
    for (key, value) in extra_env {
        command.env(key, value);
    }
    command.output().expect("运行窗口 host")
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
    let (package, stem) = build_fixture_package(&temp, &fixture);
    let log_dir = temp.join("logs");

    let result = run_window_host(&package, &stem, "5", &log_dir);
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
    // 起始场景来自包内 Project JSON（cooked fixture 场景 = 3 个实体：cube/child cube/sun）。
    assert!(
        log.contains("使用起始场景：3 个实体"),
        "日志：{log}"
    );
    assert!(log.contains("来源：Project JSON"), "日志：{log}");
    assert!(log.contains("PBR draw path 激活：true"), "日志：{log}");
    assert!(log.contains("窗口 host 结束：5 帧"), "日志：{log}");
    assert!(log.contains("达到固定帧数"), "日志：{log}");
    // cooked fixture 的 imported 场景没有 joints/weights → 蒙皮回退，窗口仍正常。
    assert!(
        log.contains("包内没有带 joints/weights 的蒙皮 mesh"),
        "日志：{log}"
    );
}

/// 真实窗口 + 真实 Mono 调用 C# PlayerGame（feature = mono-host）：
/// mcs 编译测试 fixture → 打成 Player 包 → 窗口 host 调用 Initialize/Update/Shutdown。
#[cfg(feature = "mono-host")]
#[ignore = "需要显示环境 + Vulkan GPU + Mono；CI 默认跳过，本机手工验证"]
#[test]
fn actual_window_host_calls_csharp_player_game() {
    use common::compile_player_game_fixture;

    let temp = TempDir::new("window-host-mono-smoke");
    let fixture = cooked_fixture_dir();
    let Some(dll) = compile_player_game_fixture(&temp) else {
        return;
    };
    let scripts = vec![ScriptAssembly::with_source("PlayerGame.dll", &dll)];
    let (package, stem) = build_fixture_package_with_scripts(&temp, &fixture, Some(scripts));
    let log_dir = temp.join("logs");
    let mono_log = temp.join("mono-output.log");

    let result = run_window_host_with_env(
        &package,
        &stem,
        "5",
        &log_dir,
        &[("HEZHOU_PLAYER_MONO_LOG", mono_log.to_str().unwrap())],
    );
    assert!(
        result.status.success(),
        "mono 窗口 host 失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.status.code(), Some(0));

    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("读取 player.log");
    assert!(
        log.contains("mono-host：加载 C# 入口 `PlayerGame.dll`"),
        "日志：{log}"
    );
    assert!(
        log.contains("mono-host：Hezhou.PlayerGame.Initialize(context=0x"),
        "日志：{log}"
    );
    assert!(
        log.contains("mono-host：Shutdown 完成（Initialize 1 次，Update 5 次，Shutdown 1 次，错误 0 次"),
        "日志：{log}"
    );
    assert!(log.contains("窗口 host 结束：5 帧"), "日志：{log}");

    let csharp_log = std::fs::read_to_string(&mono_log).expect("读取 C# 调用日志");
    // task-32 起 Player 传入非 0 PlayerCSharpContext；fixture 忽略 context 但记录其数值。
    assert!(
        csharp_log.contains("Initialize context="),
        "C# 日志：{csharp_log}"
    );
    assert!(csharp_log.contains("Update #1"), "C# 日志：{csharp_log}");
    assert!(csharp_log.contains("Update #5"), "C# 日志：{csharp_log}");
    assert!(
        csharp_log.contains("Shutdown updates=5"),
        "C# 日志：{csharp_log}"
    );
}

/// 真实窗口 + 真实 C# PlayerGame 的 PlayerCSharpContext（feature = mono-host）：
/// 编译 `engine/scripts/PlayerGame.cs` → Player 包 → 窗口 host 让 C# 创建/查询 Scene 实体。
#[cfg(feature = "mono-host")]
#[ignore = "需要显示环境 + Vulkan GPU + Mono；CI 默认跳过，本机手工验证"]
#[test]
fn actual_window_host_calls_csharp_scene_context() {
    use common::compile_engine_player_game;

    let temp = TempDir::new("window-host-csharp-context");
    let fixture = cooked_fixture_dir();
    let Some(dll) = compile_engine_player_game(&temp) else {
        return;
    };
    let scripts = vec![ScriptAssembly::with_source("PlayerGame.dll", &dll)];
    let (package, stem) = build_fixture_package_with_scripts(&temp, &fixture, Some(scripts));
    let log_dir = temp.join("logs");
    let mono_log = temp.join("mono-context.log");

    let result = run_window_host_with_env(
        &package,
        &stem,
        "5",
        &log_dir,
        &[("HEZHOU_PLAYER_MONO_LOG", mono_log.to_str().unwrap())],
    );
    assert!(
        result.status.success(),
        "C# context 窗口 host 失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.status.code(), Some(0));

    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("读取 player.log");
    assert!(
        log.contains("mono-host：Scene context 已提供（初始 3 实体，ABI v2"),
        "日志：{log}"
    );
    assert!(
        log.contains("C# Initialize 后 Scene 实体 4"),
        "日志：{log}"
    );
    assert!(
        log.contains("C# Shutdown 后 Scene 实体 3"),
        "日志：{log}"
    );
    assert!(log.contains("窗口 host 结束：5 帧"), "日志：{log}");

    let csharp_log = std::fs::read_to_string(&mono_log).expect("读取 C# 调用日志");
    assert!(
        csharp_log.contains("SceneContext: Initialize 创建 cube="),
        "C# 日志：{csharp_log}"
    );
    assert!(csharp_log.contains("entities 3->4"), "C# 日志：{csharp_log}");
    assert!(
        csharp_log.contains("position=(2.500,0.500,-1.500)"),
        "C# 日志：{csharp_log}"
    );
    assert!(
        csharp_log.contains("rotationSet=0") && csharp_log.contains("scale=(1.500,1.500,1.500)"),
        "C# 日志：{csharp_log}"
    );
    assert!(
        csharp_log.contains("SceneContext: frame=1 entities=4 cube="),
        "C# 日志：{csharp_log}"
    );
    assert!(
        csharp_log.contains("SceneContext: Shutdown entities=3")
            && csharp_log.contains("existsBefore=1")
            && csharp_log.contains("destroyResult=0")
            && csharp_log.contains("existsAfter=0"),
        "C# 日志：{csharp_log}"
    );
}

/// 真实窗口渲染真实蒙皮 fixture：ImportedPrimitive → SkinnedVertex → task-28 上传 +
/// 每帧 palette 更新；需要显示环境 + Vulkan GPU，CI 默认跳过。
#[ignore = "需要显示环境 + Vulkan GPU；CI 默认跳过，本机手工验证"]
#[test]
fn actual_window_host_renders_real_skinned_asset() {
    let temp = TempDir::new("window-host-skinned-smoke");
    let fixture = skinned_fixture_dir();
    let (package, stem) = build_fixture_package(&temp, &fixture);
    let log_dir = temp.join("logs");

    let result = run_window_host(&package, &stem, "10", &log_dir);
    assert!(
        result.status.success(),
        "蒙皮窗口 host 失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.status.code(), Some(0));

    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("读取 player.log");
    assert!(
        log.contains("蒙皮资产：mesh aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa（3 顶点 / 1 三角形）"),
        "日志：{log}"
    );
    assert!(log.contains("skeleton 2 关节"), "日志：{log}");
    assert!(log.contains("animation Spin（1.00s）"), "日志：{log}");
    assert!(
        log.contains("蒙皮网格上传成功：3 顶点 / 1 三角形 / 2 关节"),
        "日志：{log}"
    );
    assert!(
        log.contains("PBR draw path 激活：true"),
        "日志：{log}"
    );
    assert!(
        log.contains("已上传蒙皮网格：1"),
        "日志：{log}"
    );
    assert!(
        log.contains("蒙皮绘制统计：3 顶点 / 1 三角形 / 2 关节；palette 更新 10 次（动画驱动）"),
        "日志：{log}"
    );
    assert!(log.contains("窗口 host 结束：10 帧"), "日志：{log}");
}
