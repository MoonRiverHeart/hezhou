//! hezhou-package 集成测试：registry 扫描 / 依赖拓扑 / 循环 / 安装+lockfile / 卸载。

use std::path::{Path, PathBuf};

use hezhou_package::{
    resolve_closure, InstallOutcome, Lockfile, PackageInstaller, PackageManifest, PackageRegistry,
};

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("package_tests");
    std::fs::create_dir_all(&base).unwrap();
    base.join(format!("{tag}_{}_{}", std::process::id(), nanos))
}

fn write_package(registry: &Path, name: &str, version: &str, deps: &[(&str, &str)]) -> PathBuf {
    let dir = registry.join(format!("{name}-{version}"));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    let dependencies: std::collections::BTreeMap<&str, &str> = deps.iter().copied().collect();
    let manifest = serde_json::json!({
        "name": name,
        "version": version,
        "display_name": name,
        "description": "test package",
        "dependencies": dependencies,
        "assets": ["assets/data.json"],
        "scripts": ["scripts/demo.cs"],
        "entry": "Demo.Entry",
    });
    std::fs::write(
        dir.join("package.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join("assets").join("data.json"), "{\"v\":1}").unwrap();
    std::fs::write(dir.join("scripts").join("demo.cs"), "// demo").unwrap();
    dir
}

fn setup_registry(tag: &str) -> PathBuf {
    let registry = unique_dir(tag);
    std::fs::create_dir_all(&registry).unwrap();
    write_package(&registry, "pkg-a", "0.1.0", &[("pkg-b", "^0.1.0")]);
    write_package(&registry, "pkg-b", "0.1.0", &[("pkg-c", "1.2.3")]);
    write_package(&registry, "pkg-c", "1.2.3", &[]);
    write_package(&registry, "pkg-c", "1.3.0", &[]);
    write_package(&registry, "pkg-x", "0.1.0", &[("pkg-a", "^0.1.0")]);
    write_package(&registry, "pkg-y", "0.1.0", &[("pkg-z", "*")]);
    registry
}

#[test]
fn registry_scan_and_semver_selection() {
    let registry = setup_registry("scan");
    let index = PackageRegistry::scan(&registry).expect("扫描应成功");
    assert_eq!(index.entries().len(), 6);
    assert_eq!(index.versions_of("pkg-c").len(), 2);
    let exact = index
        .resolve("pkg-c", &hezhou_package::VersionReq::parse("1.2.3").unwrap())
        .expect("精确版本应命中");
    assert_eq!(exact.id.version, hezhou_package::Version::new(1, 2, 3));
    let tilde = index
        .resolve("pkg-c", &hezhou_package::VersionReq::parse("~1.2.0").unwrap())
        .expect("tilde 应命中");
    assert_eq!(tilde.id.version, hezhou_package::Version::new(1, 2, 3));
    let latest = index.latest("pkg-c").expect("latest 应存在");
    assert_eq!(latest.id.version, hezhou_package::Version::new(1, 3, 0));
    let _ = std::fs::remove_dir_all(&registry);
}

#[test]
fn resolves_closure_topologically_and_detects_cycles() {
    let registry = setup_registry("resolve");
    let index = PackageRegistry::scan(&registry).unwrap();
    let resolved = resolve_closure(&index, &["pkg-a".to_string()]).expect("解析应成功");
    let order: Vec<String> = resolved.iter().map(|pkg| pkg.id.name.clone()).collect();
    assert_eq!(order, vec!["pkg-c", "pkg-b", "pkg-a"]);
    let a = resolved.iter().find(|pkg| pkg.id.name == "pkg-a").unwrap();
    assert_eq!(a.dependencies.get("pkg-b").unwrap().version.to_string(), "0.1.0");

    // 循环：pkg-x -> pkg-a -> pkg-b -> pkg-c 无环；人为构造循环 y -> z -> y。
    let loop_registry = unique_dir("cycle");
    std::fs::create_dir_all(&loop_registry).unwrap();
    write_package(&loop_registry, "pkg-y", "0.1.0", &[("pkg-z", "*")]);
    write_package(&loop_registry, "pkg-z", "0.1.0", &[("pkg-y", "*")]);
    let loop_index = PackageRegistry::scan(&loop_registry).unwrap();
    let error = resolve_closure(&loop_index, &["pkg-y".to_string()]).expect_err("应检测循环");
    assert!(error.to_string().contains("循环依赖"), "{error}");

    // 缺失依赖。
    let missing_registry = unique_dir("missing");
    std::fs::create_dir_all(&missing_registry).unwrap();
    write_package(&missing_registry, "pkg-orphan", "0.1.0", &[("pkg-none", "*")]);
    let missing_index = PackageRegistry::scan(&missing_registry).unwrap();
    let error = resolve_closure(&missing_index, &["pkg-orphan".to_string()])
        .expect_err("应检测缺失依赖");
    assert!(error.to_string().contains("缺失依赖"), "{error}");

    let _ = std::fs::remove_dir_all(&registry);
    let _ = std::fs::remove_dir_all(&loop_registry);
    let _ = std::fs::remove_dir_all(&missing_registry);
}

#[test]
fn installs_dependencies_writes_lockfile_and_uninstalls_safely() {
    let registry = setup_registry("install");
    let project = unique_dir("project");
    std::fs::create_dir_all(&project).unwrap();
    let installer = PackageInstaller::new(&project);

    let outcome: InstallOutcome = installer
        .install(&registry, "pkg-a")
        .expect("安装应成功");
    assert_eq!(outcome.installed.len(), 3, "应安装 a/b/c");
    assert!(project.join("packages/pkg-a/package.json").is_file());
    assert!(project.join("packages/pkg-b/assets/data.json").is_file());
    assert!(project.join("packages/pkg-c/scripts/demo.cs").is_file());
    let lock_path = project.join("packages/lock.json");
    assert!(lock_path.is_file());
    let lock = Lockfile::load(&lock_path).unwrap();
    let a = lock.find("pkg-a").expect("lock 应含 pkg-a");
    assert_eq!(a.version, "0.1.0");
    assert!(a.hash.starts_with("fnv1a64:"));
    assert_eq!(a.dependencies.get("pkg-b").map(String::as_str), Some("0.1.0"));

    // 重复安装：幂等（skip），hash 不变。
    let again = installer.install(&registry, "pkg-a").expect("重复安装应成功");
    assert!(again.is_noop(), "重复安装应全部 skip");
    let lock_again = Lockfile::load(&lock_path).unwrap();
    assert_eq!(
        lock_again.find("pkg-a").unwrap().hash,
        a.hash,
        "hash 应稳定"
    );

    // 依赖保护：卸载 pkg-c 被 pkg-b 依赖 → 拒绝。
    let error = installer.uninstall("pkg-c").expect_err("被依赖包应拒绝卸载");
    assert!(error.contains("被依赖"), "{error}");

    // 卸载叶子包 pkg-a（无依赖方）→ 成功；再卸载 pkg-b/pkg-c。
    installer.uninstall("pkg-a").expect("卸载 pkg-a 应成功");
    assert!(!project.join("packages/pkg-a").exists());
    installer.uninstall("pkg-b").expect("卸载 pkg-b 应成功");
    installer.uninstall("pkg-c").expect("卸载 pkg-c 应成功");
    let lock_empty = Lockfile::load(&lock_path).unwrap();
    assert!(lock_empty.is_empty());

    let _ = std::fs::remove_dir_all(&registry);
    let _ = std::fs::remove_dir_all(&project);
}

#[test]
fn rejects_version_conflict_existing_dirs_and_bad_names() {
    let registry = setup_registry("conflict");
    let project = unique_dir("conflict_project");
    std::fs::create_dir_all(&project).unwrap();
    let installer = PackageInstaller::new(&project);
    installer.install(&registry, "pkg-c").expect("先安装 1.3.0");
    // pkg-b 要求 pkg-c 精确 1.2.3 → 与已装 1.3.0 冲突。
    let error = installer.install(&registry, "pkg-b").expect_err("版本冲突应被拒绝");
    assert!(error.contains("版本冲突"), "{error}");

    // lockfile 无记录但目录存在 → 拒绝覆盖。
    let project2 = unique_dir("conflict_project2");
    std::fs::create_dir_all(project2.join("packages/pkg-c")).unwrap();
    let installer2 = PackageInstaller::new(&project2);
    let error = installer2.install(&registry, "pkg-c").expect_err("未知目录应被拒绝");
    assert!(error.contains("lockfile 无记录"), "{error}");

    // 非法包名清单直接拒绝。
    let manifest = PackageManifest::from_json(
        r#"{ "name": "../escape", "version": "1.0.0" }"#,
    )
    .unwrap();
    assert!(manifest.validate().is_err());

    let _ = std::fs::remove_dir_all(&registry);
    let _ = std::fs::remove_dir_all(&project);
    let _ = std::fs::remove_dir_all(&project2);
}

