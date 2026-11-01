//! `PlayerBuildConfig` 校验与默认值测试。

use hezhou_player::config::{
    default_executable_stem, PlayerBuildConfig, QualityTier, WindowConfig, WindowMode,
    DEFAULT_FONT_ASSET,
};
use hezhou_player::platform::TargetPlatform;
use hezhou_player::version::SemVer;

#[test]
fn valid_config_with_defaults_passes() {
    let config = PlayerBuildConfig::new("Demo Game", "1.0.0", "scenes/main.scene", TargetPlatform::Windows);

    assert_eq!(config.executable_name, "demo-game");
    assert_eq!(config.quality, QualityTier::High);
    assert_eq!(config.window.width, 1280);
    assert_eq!(config.window.height, 720);
    assert_eq!(config.window.mode, WindowMode::Windowed);
    assert_eq!(config.required_font.as_deref(), Some(DEFAULT_FONT_ASSET));

    let result = config.validate();
    assert!(result.is_ok(), "合法配置不应报错：{result:?}");
}

#[test]
fn invalid_version_and_scene_are_reported() {
    let mut config = PlayerBuildConfig::new("Demo", "not-a-version", "../escape.scene", TargetPlatform::Linux);
    config.version = "v1.2".to_string();
    config.start_scene = "../escape.scene".to_string();

    let errors = config.validate().expect_err("非法配置必须失败");
    let fields: Vec<&str> = errors.issues.iter().map(|issue| issue.field).collect();
    assert!(fields.contains(&"version"), "缺少 version 问题：{fields:?}");
    assert!(fields.contains(&"start_scene"), "缺少 start_scene 问题：{fields:?}");
    assert!(errors.summary().contains("start_scene"));
}

#[test]
fn invalid_window_and_executable_name_are_reported() {
    let mut config = PlayerBuildConfig::new("Demo", "1.0.0", "scenes/main.scene", TargetPlatform::Windows);
    config.window = WindowConfig::new("Demo").with_size(10, 999_999);
    config.executable_name = "bad name.exe".to_string();

    let errors = config.validate().expect_err("窗口/可执行名非法必须失败");
    let fields: Vec<&str> = errors.issues.iter().map(|issue| issue.field).collect();
    assert!(fields.contains(&"window.width"));
    assert!(fields.contains(&"window.height"));
    assert!(fields.contains(&"executable_name"));
}

#[test]
fn empty_title_and_missing_log_dir_are_reported() {
    let mut config = PlayerBuildConfig::new("Demo", "1.0.0", "scenes/main.scene", TargetPlatform::Windows);
    config.window.title = "   ".to_string();
    config.log_dir = "logs/../..".to_string();

    let errors = config.validate().expect_err("空标题/越界日志目录必须失败");
    let fields: Vec<&str> = errors.issues.iter().map(|issue| issue.field).collect();
    assert!(fields.contains(&"window.title"));
    assert!(fields.contains(&"log_dir"));
}

#[test]
fn executable_name_derivation_is_portable() {
    assert_eq!(default_executable_stem("Demo Game"), "demo-game");
    assert_eq!(default_executable_stem("我的游戏"), "hezhou-game");
    assert_eq!(default_executable_stem("  ---  "), "hezhou-game");
    // Windows 保留设备名必须自动规避，否则构建器无法产出可执行文件。
    assert_eq!(default_executable_stem("CON"), "con-player");
    assert_eq!(default_executable_stem("Com1"), "com1-player");
}

#[test]
fn version_parser_rejects_non_semver() {
    assert!(SemVer::parse("1.0.0").is_ok());
    assert!(SemVer::parse("1.0").is_err());
    assert!(SemVer::parse("1.0.0.0").is_err());
    assert!(SemVer::parse("1.0.x").is_err());
    assert!(SemVer::parse("v1.0.0").is_err());
    assert_eq!(SemVer::parse("1.2.3").unwrap().to_string(), "1.2.3");
}
