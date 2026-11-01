//! 最小 fixture 端到端测试：CLI 真实构建 → 复制出的 Player 通过 `--validate`。

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{fixture_dir, TempDir};
use hezhou_player::manifest::PlayerManifest;
use hezhou_player::platform::TargetPlatform;

const BUILDER_BIN: &str = env!("CARGO_BIN_EXE_hezhou-player-build");
const PLAYER_BIN: &str = env!("CARGO_BIN_EXE_hezhou-player");

fn run_builder(args: &[OsString]) -> Output {
    Command::new(BUILDER_BIN)
        .args(args)
        .output()
        .expect("启动 hezhou-player-build 失败")
}

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

fn fixture_args(fixture: &Path, temp: &TempDir) -> Vec<OsString> {
    vec![
        "--config".into(),
        fixture.join("build-config.json").into(),
        "--cook".into(),
        fixture.join("cook-manifest.json").into(),
        "--assets-dir".into(),
        fixture.join("assets").into(),
        "--scripts-dir".into(),
        fixture.join("scripts").into(),
        "--runtime-dir".into(),
        fixture.join("runtime").into(),
        "--exe".into(),
        PLAYER_BIN.into(),
        "--build-out".into(),
        temp.join("dist").into(),
    ]
}

#[test]
fn cli_builds_minimal_fixture_and_validate_passes() {
    let temp = TempDir::new("fixture-cli-ok");
    let fixture = fixture_dir();
    let output = run_builder(&fixture_args(&fixture, &temp));
    assert!(
        output.status.success(),
        "构建失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let package = temp.join("dist").join(host().as_str());
    let executable = package.join(host().executable_file_name("minimal-fixture"));
    assert!(
        executable.is_file(),
        "缺少可执行文件：{}",
        executable.display()
    );
    assert!(package.join("manifest.json").is_file());
    assert!(package.join("assets/scenes/empty.scene").is_file());
    assert!(package
        .join("assets/fonts/HarmonyOS_Sans_SC.ttf")
        .is_file());
    assert!(package.join("scripts/Game.dll").is_file());
    assert!(package.join("runtime/libmono.so").is_file());

    // 清单与真实文件一致：资产大小、字体、脚本都存在。
    let manifest = PlayerManifest::load(&package.join("manifest.json")).expect("读取清单");
    manifest
        .validate_for_runtime(host())
        .expect("产物清单必须可被当前平台加载");
    let assets_dir = manifest.assets_dir(&package);
    for asset in &manifest.assets {
        let path = assets_dir.join(hezhou_player::to_native_path(&asset.path, host()));
        assert!(path.is_file(), "清单资产缺失：{}", path.display());
        if let Some(size) = asset.size_bytes {
            assert_eq!(std::fs::metadata(&path).unwrap().len(), size);
        }
    }
    let scripts_dir = manifest.scripts_dir(&package);
    for script in &manifest.scripts {
        assert!(scripts_dir.join(script).is_file(), "清单脚本缺失：{script}");
    }

    // 复制出的 Player 自身能通过启动校验（真实端到端冒烟）。
    let log_dir = temp.join("logs");
    let validate = Command::new(&executable)
        .arg("--validate")
        .arg("--manifest")
        .arg(package.join("manifest.json"))
        .arg("--log-dir")
        .arg(&log_dir)
        .output()
        .expect("运行产物 Player 失败");
    assert!(
        validate.status.success(),
        "产物 Player --validate 失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&validate.stdout),
        String::from_utf8_lossy(&validate.stderr)
    );
    let log = std::fs::read_to_string(log_dir.join("player.log")).expect("产物日志");
    assert!(log.contains("启动检查完成"), "日志内容：{log}");
    assert!(log.contains("Minimal Fixture"), "日志内容：{log}");

    // 无 staging 残留。
    let leftovers: Vec<PathBuf> = std::fs::read_dir(temp.join("dist"))
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().contains(".staging-"))
                .unwrap_or(false)
        })
        .collect();
    assert!(leftovers.is_empty(), "残留 staging：{leftovers:?}");
}

#[test]
fn cli_missing_asset_fails_cleanly() {
    let temp = TempDir::new("fixture-cli-missing-asset");
    let fixture = fixture_dir();
    // 用空资产目录替换 fixture 的 assets：启动场景源文件不存在。
    let empty_assets = temp.mkdir("empty-assets");
    let mut args = fixture_args(&fixture, &temp);
    // 替换 --assets-dir 的值为空目录
    let index = args
        .iter()
        .position(|arg| arg == "--assets-dir")
        .expect("参数中有 --assets-dir");
    args[index + 1] = empty_assets.into();

    let output = run_builder(&args);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("构建失败"), "stderr: {stderr}");
    assert!(stderr.contains("源文件不存在"), "stderr: {stderr}");
    assert!(
        !temp.join("dist").join(host().as_str()).exists(),
        "失败后不应留下产物目录"
    );
}

#[test]
fn cli_dry_run_still_works_and_writes_only_plan_metadata() {
    let temp = TempDir::new("fixture-cli-dry-run");
    let fixture = fixture_dir();
    let plan_out = temp.join("plan");
    let args: Vec<OsString> = vec![
        "--config".into(),
        fixture.join("build-config.json").into(),
        "--cook".into(),
        fixture.join("cook-manifest.json").into(),
        "--scripts-dir".into(),
        fixture.join("scripts").into(),
        "--runtime-dir".into(),
        fixture.join("runtime").into(),
        "--plan-out".into(),
        plan_out.clone().into(),
    ];
    let output = run_builder(&args);
    assert!(
        output.status.success(),
        "dry-run 失败\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(plan_out.join("windows/manifest.json").is_file());
    assert!(plan_out.join("linux/manifest.json").is_file());
    // dry-run 不产出真实二进制目录。
    assert!(!plan_out.join("windows/minimal-fixture").exists());
    assert!(!plan_out.join("windows/minimal-fixture.exe").exists());
}
