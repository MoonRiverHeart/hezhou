//! 版本解析与兼容性测试。

use hezhou_player::version::SemVer;

#[test]
fn parses_three_segment_versions() {
    let version = SemVer::parse("1.2.3").expect("解析 1.2.3");
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 2);
    assert_eq!(version.patch, 3);
    assert_eq!(version.to_string(), "1.2.3");
    assert_eq!(version.major_minor(), "1.2");
}

#[test]
fn rejects_malformed_versions() {
    for input in ["", "1", "1.2", "1.2.3.4", "1.2.x", "v1.2.3", "1.2.3-rc1", "1.2.3 ",
        " 1.2.3", "-1.2.3", "1..3", "1.2.", "99999999999.0.0"] {
        assert!(SemVer::parse(input).is_err(), "`{input}` 不应解析成功");
    }
}

#[test]
fn compatibility_requires_same_major_minor() {
    let runtime = SemVer::parse("0.2.5").unwrap();
    assert!(SemVer::parse("0.2.0").unwrap().is_compatible_with(&runtime));
    assert!(SemVer::parse("0.2.5").unwrap().is_compatible_with(&runtime));
    assert!(SemVer::parse("0.2.9").unwrap().is_compatible_with(&runtime));
    // patch 不同不算不兼容（运行时补丁可加载旧包），但 major.minor 必须一致。
    assert!(!SemVer::parse("0.3.0").unwrap().is_compatible_with(&runtime));
    assert!(!SemVer::parse("1.2.0").unwrap().is_compatible_with(&runtime));
    assert!(!SemVer::parse("0.1.5").unwrap().is_compatible_with(&runtime));
}

#[test]
fn ordering_follows_semver_precedence() {
    let mut versions = vec![
        SemVer::parse("0.2.10").unwrap(),
        SemVer::parse("0.2.2").unwrap(),
        SemVer::parse("0.1.99").unwrap(),
    ];
    versions.sort();
    assert_eq!(
        versions,
        vec![
            SemVer::parse("0.1.99").unwrap(),
            SemVer::parse("0.2.2").unwrap(),
            SemVer::parse("0.2.10").unwrap(),
        ]
    );
}
