//! 跨平台逻辑路径规则测试。

use hezhou_player::paths::{
    case_collision_key, extension, file_name, is_windows_reserved_name, join_logical,
    normalize_logical_path, to_native_path, PathError,
};
use hezhou_player::platform::TargetPlatform;

#[test]
fn normalize_accepts_backslashes_and_dot_segments() {
    assert_eq!(
        normalize_logical_path(r"assets\scenes\.\main.scene").unwrap(),
        "assets/scenes/main.scene"
    );
    assert_eq!(
        normalize_logical_path("assets/scenes/main.scene/").unwrap(),
        "assets/scenes/main.scene"
    );
    assert_eq!(
        normalize_logical_path("  assets//").is_err(),
        false,
        "末尾多余分隔符应被容忍"
    );
}

#[test]
fn normalize_rejects_unsafe_paths() {
    assert_eq!(normalize_logical_path(""), Err(PathError::Empty));
    assert_eq!(normalize_logical_path("   "), Err(PathError::Empty));
    assert_eq!(
        normalize_logical_path("/etc/passwd"),
        Err(PathError::Absolute)
    );
    assert_eq!(
        normalize_logical_path("assets/../../secret.json"),
        Err(PathError::ParentTraversal)
    );
    assert_eq!(
        normalize_logical_path("assets//main.scene"),
        Err(PathError::EmptySegment)
    );
    assert_eq!(
        normalize_logical_path("assets/co:lon"),
        Err(PathError::InvalidChar { ch: ':' })
    );
    assert_eq!(
        normalize_logical_path("assets/name."),
        Err(PathError::InvalidSegmentSuffix {
            segment: "name.".to_string()
        })
    );
    assert_eq!(
        normalize_logical_path("assets/CON.txt"),
        Err(PathError::ReservedName {
            name: "CON.txt".to_string()
        })
    );
}

#[test]
fn join_logical_revalidates_combined_path() {
    assert_eq!(
        join_logical("assets", "scenes//main.scene").is_err(),
        true
    );
    assert_eq!(
        join_logical("assets", "scenes/main.scene").unwrap(),
        "assets/scenes/main.scene"
    );
    assert_eq!(join_logical("assets/scenes/main.scene", "../x").is_err(), true);
}

#[test]
fn case_key_and_helpers_are_stable() {
    assert_eq!(
        case_collision_key("Assets/Scenes/Main.scene"),
        "assets/scenes/main.scene"
    );
    assert_eq!(file_name("assets/scenes/main.scene"), Some("main.scene"));
    assert_eq!(extension("assets/scenes/main.SCENE"), Some("scene".to_string()));
    assert_eq!(extension("assets/.gitignore"), None);
    assert!(is_windows_reserved_name("com1"));
    assert!(is_windows_reserved_name("NUL"));
    assert!(!is_windows_reserved_name("console"));
}

#[test]
fn native_conversion_matches_target_platform() {
    assert_eq!(
        to_native_path("assets/scenes/main.scene", TargetPlatform::Windows),
        std::path::PathBuf::from(r"assets\scenes\main.scene")
    );
    assert_eq!(
        to_native_path("assets/scenes/main.scene", TargetPlatform::Linux),
        std::path::PathBuf::from("assets/scenes/main.scene")
    );
}

#[test]
fn executable_stem_rules_match_platform_naming() {
    use hezhou_player::paths::is_valid_executable_stem;

    assert!(is_valid_executable_stem("hezhou-game_2"));
    assert!(!is_valid_executable_stem(""));
    assert!(!is_valid_executable_stem("has space"));
    assert!(!is_valid_executable_stem("has/slash"));
    assert!(!is_valid_executable_stem("CON"));
    assert!(!is_valid_executable_stem(&"a".repeat(65)));

    assert_eq!(
        TargetPlatform::Windows.executable_file_name("hezhou-game"),
        "hezhou-game.exe"
    );
    assert_eq!(
        TargetPlatform::Linux.executable_file_name("hezhou-game"),
        "hezhou-game"
    );
}
