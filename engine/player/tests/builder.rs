//! 构建器 dry-run 测试：双平台清单、校验、错误路径。

mod common;

use common::{sample_input, TempDir};
use hezhou_player::builder::{
    ArtifactCategory, BuildError, BuildWarning, PlayerBuilder, RuntimeFileSource, RuntimePayload,
    ScriptAssembly,
};
use hezhou_player::config::DEFAULT_FONT_ASSET;
use hezhou_player::manifest::{ManifestAsset, PlayerManifest};
use hezhou_player::platform::TargetPlatform;

#[test]
fn dry_run_produces_windows_and_linux_inventories() {
    let temp = TempDir::new("builder-both");
    let builder = PlayerBuilder::new(sample_input(&temp, TargetPlatform::Windows));
    let plans = builder.plan_both_targets().expect("生成双平台计划");
    assert_eq!(plans.len(), 2);

    let windows = &plans[0];
    let linux = &plans[1];
    assert_eq!(windows.target, TargetPlatform::Windows);
    assert_eq!(linux.target, TargetPlatform::Linux);

    // 可执行文件命名与权限差异。
    let windows_exe = windows.file("demo-game.exe").expect("Windows exe");
    assert!(windows_exe.executable);
    assert_eq!(windows_exe.unix_mode, None);
    assert!(windows.file("demo-game").is_none());

    let linux_exe = linux.file("demo-game").expect("Linux 可执行文件");
    assert!(linux_exe.executable);
    assert_eq!(linux_exe.unix_mode, Some(0o755));
    assert!(linux.file("demo-game.exe").is_none());

    // 目录结构与分类。
    assert_eq!(windows.files_in(ArtifactCategory::Executable).len(), 1);
    assert_eq!(windows.files_in(ArtifactCategory::Asset).len(), 3);
    assert_eq!(windows.files_in(ArtifactCategory::Script).len(), 1);
    assert_eq!(windows.files_in(ArtifactCategory::Runtime).len(), 1);
    assert_eq!(windows.files_in(ArtifactCategory::Manifest).len(), 1);

    assert!(windows.file("assets/scenes/main.scene").is_some());
    assert!(windows.file("scripts/Game.dll").is_some());
    assert!(windows.file("runtime/libmono.so").is_some());
    assert!(windows.file("manifest.json").is_some());

    // 清单内容与配置一致。
    assert_eq!(windows.manifest.start_scene, "scenes/main.scene");
    assert_eq!(windows.manifest.scripts, vec!["Game.dll".to_string()]);
    assert_eq!(windows.manifest.target, TargetPlatform::Windows);
    assert_eq!(linux.manifest.target, TargetPlatform::Linux);
    assert!(windows.manifest.validate().is_ok());

    // 逻辑路径永远使用 `/`，且同一分类内按路径排序。
    for file in &windows.files {
        assert!(!file.relative_path.contains('\\'));
        assert!(!file.relative_path.starts_with('/'));
        assert!(!file.relative_path.contains(".."));
    }
    let assets: Vec<&str> = windows
        .files_in(ArtifactCategory::Asset)
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect();
    let mut expected = assets.clone();
    expected.sort();
    assert_eq!(assets, expected);

    // 校验值：脚本文件来自真实源文件，有 fnv1a64 校验。
    let script = windows.file("scripts/Game.dll").unwrap();
    assert!(
        script
            .checksum
            .as_deref()
            .unwrap_or("")
            .starts_with("fnv1a64:"),
        "脚本校验值缺失：{:?}",
        script.checksum
    );
    assert!(windows.fingerprint.starts_with("fnv1a64:"));
    assert_ne!(windows.fingerprint, linux.fingerprint);

    // 警告：Linux 需要 rpath；可执行文件由 cargo 产出。
    assert!(windows
        .warnings
        .contains(&BuildWarning::ExecutableProducedByCargo));
    assert!(linux.warnings.contains(&BuildWarning::LinuxRpathRequired));
    assert!(!windows
        .warnings
        .contains(&BuildWarning::RuntimePayloadEmpty));
}

#[test]
fn dry_run_is_deterministic() {
    let temp = TempDir::new("builder-deterministic");
    let builder = PlayerBuilder::new(sample_input(&temp, TargetPlatform::Windows));
    let first = builder.plan().expect("第一次计划");
    let second = builder.plan().expect("第二次计划");

    assert_eq!(first.fingerprint, second.fingerprint);
    assert_eq!(first.file_list(), second.file_list());
    assert_eq!(first.files.len(), second.files.len());
}

#[test]
fn fingerprint_reacts_to_asset_and_runtime_content() {
    let temp_a = TempDir::new("builder-fingerprint-a");
    let temp_b = TempDir::new("builder-fingerprint-b");

    let mut input_a = sample_input(&temp_a, TargetPlatform::Linux);
    let mut input_b = sample_input(&temp_b, TargetPlatform::Linux);

    // 同结构、不同运行时内容：dry-run 校验必须不同。
    let changed = temp_b.write("source-runtime/libmono.so", b"different-libmono-content");
    input_b.runtime = RuntimePayload::new(vec![RuntimeFileSource {
        relative_path: "libmono.so".to_string(),
        source: changed,
        size_bytes: b"different-libmono-content".len() as u64,
    }]);

    let plan_a = PlayerBuilder::new(input_a.clone()).plan().unwrap();
    let plan_b = PlayerBuilder::new(input_b).plan().unwrap();
    assert_ne!(plan_a.fingerprint, plan_b.fingerprint);
    assert_ne!(
        plan_a.file("runtime/libmono.so").unwrap().checksum,
        plan_b.file("runtime/libmono.so").unwrap().checksum
    );

    // 同结构同内容（不同临时目录）：指纹一致，保证构建可重复。
    input_a.runtime = RuntimePayload::new(vec![RuntimeFileSource {
        relative_path: "libmono.so".to_string(),
        source: temp_a.write("source-runtime/libmono-copy.so", b"fake-libmono"),
        size_bytes: b"fake-libmono".len() as u64,
    }]);
    let plan_a2 = PlayerBuilder::new(input_a).plan().unwrap();
    assert_eq!(plan_a2.fingerprint, plan_a.fingerprint);
}

#[test]
fn missing_start_scene_is_rejected() {
    let temp = TempDir::new("builder-missing-scene");
    let mut input = sample_input(&temp, TargetPlatform::Windows);
    input
        .cook
        .assets
        .retain(|asset| asset.path != "scenes/main.scene");

    let error = PlayerBuilder::new(input).plan().expect_err("缺少启动场景必须失败");
    assert!(
        matches!(error, BuildError::MissingStartScene { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn missing_font_is_rejected() {
    let temp = TempDir::new("builder-missing-font");
    let mut input = sample_input(&temp, TargetPlatform::Windows);
    input
        .cook
        .assets
        .retain(|asset| asset.path != DEFAULT_FONT_ASSET);

    let error = PlayerBuilder::new(input).plan().expect_err("缺少字体必须失败");
    assert!(
        matches!(error, BuildError::MissingFont { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn case_collision_is_rejected() {
    let temp = TempDir::new("builder-case-collision");
    let mut input = sample_input(&temp, TargetPlatform::Windows);
    input
        .cook
        .assets
        .push(ManifestAsset::new("Scenes/Main.scene", "scene"));

    let error = PlayerBuilder::new(input).plan().expect_err("大小写冲突必须失败");
    assert!(
        matches!(error, BuildError::CaseCollision { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn invalid_script_name_is_rejected() {
    let temp = TempDir::new("builder-script-name");
    let mut input = sample_input(&temp, TargetPlatform::Windows);
    input.scripts = vec![ScriptAssembly::new("../evil")];

    let error = PlayerBuilder::new(input).plan().expect_err("非法脚本名必须失败");
    assert!(
        matches!(error, BuildError::Manifest(_)),
        "实际错误：{error}"
    );
}

#[test]
fn empty_runtime_and_scripts_emit_warnings() {
    let temp = TempDir::new("builder-warnings");
    let mut input = sample_input(&temp, TargetPlatform::Windows);
    input.runtime = RuntimePayload::empty();
    input.scripts.clear();

    let plan = PlayerBuilder::new(input).plan().unwrap();
    assert!(plan.warnings.contains(&BuildWarning::RuntimePayloadEmpty));
    assert!(plan.warnings.contains(&BuildWarning::ScriptsEmpty));
}

#[test]
fn cook_manifest_accepts_assets_prefix() {
    let temp = TempDir::new("builder-prefix");
    let mut input = sample_input(&temp, TargetPlatform::Windows);
    input.cook.assets = vec![
        ManifestAsset::new("assets/scenes/main.scene", "scene"),
        ManifestAsset::new("assets/fonts/HarmonyOS_Sans_SC.ttf", "font"),
    ];

    let plan = PlayerBuilder::new(input).plan().unwrap();
    assert_eq!(plan.manifest.start_scene, "scenes/main.scene");
    assert!(plan.file("assets/scenes/main.scene").is_some());
    assert!(plan.file("assets/assets/scenes/main.scene").is_none());
}

#[test]
fn write_plan_writes_metadata_only() {
    let temp = TempDir::new("builder-write-plan");
    let plan = PlayerBuilder::new(sample_input(&temp, TargetPlatform::Windows))
        .plan()
        .unwrap();
    let out = temp.join("out");
    plan.write_plan(&out).expect("写入计划元数据");

    assert!(out.join("manifest.json").is_file());
    assert!(out.join("plan.json").is_file());
    assert!(out.join("file-list.txt").is_file());
    // dry-run 不复制二进制。
    assert!(!out.join("demo-game.exe").exists());
    assert!(!out.join("assets").exists());

    let manifest = PlayerManifest::load(&out.join("manifest.json")).expect("读取计划清单");
    assert_eq!(manifest.product_name, "Demo Game");
    manifest.validate().expect("计划清单必须合法");

    let file_list = std::fs::read_to_string(out.join("file-list.txt")).unwrap();
    assert!(file_list.contains("assets/scenes/main.scene"));
    assert!(file_list.contains("manifest.json"));

    let plan_json = std::fs::read_to_string(out.join("plan.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&plan_json).unwrap();
    assert_eq!(value["target"], serde_json::json!("windows"));
    assert_eq!(value["fingerprint"], serde_json::json!(plan.fingerprint));
}

#[test]
fn cook_manifest_json_roundtrip_and_unknown_fields_tolerated() {
    let cook = hezhou_player::CookManifest::from_json_str(
        r#"{
            "assets": [
                {"path": "scenes/main.scene", "kind": "scene", "size_bytes": 42}
            ],
            "cooker_version": "0.2.0"
        }"#,
    )
    .expect("解析 cook 清单");
    assert_eq!(cook.assets.len(), 1);
    assert_eq!(cook.assets[0].path, "scenes/main.scene");
    assert_eq!(cook.assets[0].kind, "scene");
    assert_eq!(cook.assets[0].size_bytes, Some(42));
}
