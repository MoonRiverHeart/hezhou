//! 真实构建测试：复制、清单一致、权限、失败清理、校验和。

mod common;

use std::path::Path;

use common::{sample_build_sources, sample_input, TempDir};
use hezhou_player::builder::{BuildError, BuildSources, PlayerBuilder};
use hezhou_player::manifest::PlayerManifest;
use hezhou_player::platform::TargetPlatform;

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

fn staging_entries(dir: &Path) -> Vec<String> {
    if !dir.exists() {
        return Vec::new();
    }
    std::fs::read_dir(dir)
        .expect("读取输出父目录")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.contains(".staging-"))
        .collect()
}

#[test]
fn real_build_copies_all_categories_with_consistent_manifest() {
    let temp = TempDir::new("real-build-basic");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let plan = builder.plan().expect("dry-run 计划");
    let output_dir = temp.join("dist/player");

    let output = builder
        .build(host(), &sources, &output_dir)
        .expect("真实构建");

    assert_eq!(output.target, host());
    assert_eq!(output.files.len(), plan.files.len());
    assert_eq!(
        output.total_bytes,
        output.files.iter().map(|file| file.size_bytes).sum::<u64>()
    );

    // 每个计划文件都必须存在，且大小/校验和与构建记录一致。
    for planned in &plan.files {
        let destination =
            output_dir.join(hezhou_player::to_native_path(&planned.relative_path, host()));
        assert!(
            destination.is_file(),
            "缺少产物文件：{}",
            destination.display()
        );
        let built = output
            .file(&planned.relative_path)
            .unwrap_or_else(|| panic!("构建记录缺少 {}", planned.relative_path));
        assert_eq!(
            built.size_bytes,
            std::fs::metadata(&destination).unwrap().len()
        );
        assert_eq!(
            Some(built.checksum.clone()),
            hezhou_player::checksum_file(&destination)
        );
        if let Some(planned_size) = planned.size_bytes {
            assert_eq!(planned_size, built.size_bytes, "{} 大小不一致", planned.relative_path);
        }
    }

    // 清单一致：产物 manifest.json 与 dry-run 计划完全相同，且能通过运行时校验。
    let manifest = PlayerManifest::load(&output.manifest_path).expect("读取产物清单");
    assert_eq!(manifest, plan.manifest);
    manifest
        .validate_for_runtime(host())
        .expect("产物清单必须可被当前平台运行时加载");

    // 可执行文件内容来自 cargo 产物。
    let executable_name = plan
        .config
        .executable_stem()
        .to_string()
        + host().executable_suffix();
    let executable_path = output_dir.join(executable_name.clone());
    let source_exe = sources.executable.as_ref().expect("测试 exe 源");
    assert_eq!(
        std::fs::read(&executable_path).unwrap(),
        std::fs::read(source_exe).unwrap()
    );
    assert!(output.file(&executable_name).unwrap().executable);

    // staging 目录不残留。
    assert!(
        staging_entries(&temp.join("dist")).is_empty(),
        "失败/成功后不应残留 staging 目录"
    );
}

#[cfg(unix)]
#[test]
fn built_permissions_follow_plan_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new("real-build-permissions-unix");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let output = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect("真实构建");

    let executable = output_dir_join(&output.output_dir, &output.files[0].relative_path);
    let mode = std::fs::metadata(&executable).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o755, "Linux/Unix 可执行文件必须是 0755");

    let asset = output
        .files
        .iter()
        .find(|file| file.relative_path.starts_with("assets/"))
        .expect("至少一个资产文件");
    let asset_path = output_dir_join(&output.output_dir, &asset.relative_path);
    let asset_mode = std::fs::metadata(&asset_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(asset_mode, 0o644, "Linux/Unix 数据文件必须是 0644");
}

#[cfg(windows)]
#[test]
fn built_permissions_follow_plan_on_windows() {
    let temp = TempDir::new("real-build-permissions-windows");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let output = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect("真实构建");

    // Windows 无 Unix 权限位：计划与构建记录均为 None，但可执行标记为 true。
    let executable = &output.files[0];
    assert_eq!(executable.unix_mode, None);
    assert!(executable.executable);
}

#[cfg(unix)]
fn output_dir_join(root: &Path, logical: &str) -> std::path::PathBuf {
    root.join(hezhou_player::to_native_path(logical, host()))
}

#[test]
fn missing_asset_source_is_rejected_before_writing() {
    let temp = TempDir::new("real-build-missing-asset");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let missing = sources
        .assets_root
        .as_ref()
        .unwrap()
        .join("scenes")
        .join("main.scene");
    std::fs::remove_file(&missing).unwrap();

    let output_dir = temp.join("dist/player");
    let error = builder
        .build(host(), &sources, &output_dir)
        .expect_err("缺资产必须失败");
    assert!(
        matches!(error, BuildError::MissingSource { path: Some(_), .. }),
        "实际错误：{error}"
    );
    assert!(!output_dir.exists(), "失败后不应留下产物目录");
    assert!(staging_entries(&temp.join("dist")).is_empty());
}

#[test]
fn missing_executable_source_is_rejected() {
    let temp = TempDir::new("real-build-missing-exe");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = BuildSources {
        executable: None,
        assets_root: Some(temp.join("source-assets")),
    };
    let error = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect_err("缺 exe 必须失败");
    assert!(
        matches!(error, BuildError::MissingSource { path: None, .. }),
        "实际错误：{error}"
    );
    assert!(!temp.join("dist/player").exists());
}

#[test]
fn size_mismatch_is_rejected() {
    let temp = TempDir::new("real-build-size-mismatch");
    let mut input = sample_input(&temp, host());
    for asset in &mut input.cook.assets {
        if asset.path == "scenes/main.scene" {
            asset.size_bytes = Some(999);
        }
    }
    let builder = PlayerBuilder::new(input);
    let sources = sample_build_sources(&temp);
    let error = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect_err("大小不一致必须失败");
    assert!(
        matches!(
            error,
            BuildError::SizeMismatch {
                expected: 999,
                actual: 42,
                ..
            }
        ),
        "实际错误：{error}"
    );
    assert!(!temp.join("dist/player").exists());
}

#[test]
fn checksum_mismatch_is_rejected() {
    let temp = TempDir::new("real-build-checksum-mismatch");
    let mut input = sample_input(&temp, host());
    for asset in &mut input.cook.assets {
        if asset.path == "scenes/main.scene" {
            asset.checksum = Some("fnv1a64:0000000000000000".to_string());
        }
    }
    let builder = PlayerBuilder::new(input);
    let sources = sample_build_sources(&temp);
    let error = builder
        .build(host(), &sources, &temp.join("dist/player"))
        .expect_err("校验和不一致必须失败");
    assert!(
        matches!(error, BuildError::ChecksumMismatch { .. }),
        "实际错误：{error}"
    );
    assert!(!temp.join("dist/player").exists());
}

#[test]
fn cross_platform_build_is_rejected() {
    let temp = TempDir::new("real-build-cross-platform");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let other = match host() {
        TargetPlatform::Windows => TargetPlatform::Linux,
        TargetPlatform::Linux => TargetPlatform::Windows,
    };
    let error = builder
        .build(other, &sources, &temp.join("dist/player"))
        .expect_err("跨平台真实构建必须拒绝");
    assert!(
        matches!(error, BuildError::CrossPlatformBuildUnsupported { .. }),
        "实际错误：{error}"
    );
    assert!(!temp.join("dist/player").exists());
}

#[test]
fn successful_build_replaces_existing_output() {
    let temp = TempDir::new("real-build-replace");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let output_dir = temp.join("dist/player");
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(output_dir.join("stale.txt"), b"stale").unwrap();

    builder
        .build(host(), &sources, &output_dir)
        .expect("真实构建");
    assert!(!output_dir.join("stale.txt").exists(), "旧产物应被整体替换");
    assert!(output_dir.join("manifest.json").is_file());
    assert!(staging_entries(&temp.join("dist")).is_empty());
}

#[test]
fn failed_build_keeps_existing_output_intact() {
    let temp = TempDir::new("real-build-preserve");
    let good_builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let output_dir = temp.join("dist/player");
    good_builder
        .build(host(), &sources, &output_dir)
        .expect("首次构建成功");
    let manifest_before = std::fs::read(output_dir.join("manifest.json")).unwrap();

    // 第二次用大小不一致的 cook 清单构建：必须失败且不破坏已有产物。
    let mut bad_input = sample_input(&temp, host());
    for asset in &mut bad_input.cook.assets {
        if asset.path == "scenes/main.scene" {
            asset.size_bytes = Some(7);
        }
    }
    let bad_builder = PlayerBuilder::new(bad_input);
    bad_builder
        .build(host(), &sources, &output_dir)
        .expect_err("第二次构建应失败");

    assert_eq!(
        std::fs::read(output_dir.join("manifest.json")).unwrap(),
        manifest_before,
        "失败构建不应改动既有产物"
    );
    assert!(output_dir.join("manifest.json").is_file());
    assert!(staging_entries(&temp.join("dist")).is_empty());
}

#[test]
fn output_path_that_is_a_file_is_rejected() {
    let temp = TempDir::new("real-build-output-file");
    let builder = PlayerBuilder::new(sample_input(&temp, host()));
    let sources = sample_build_sources(&temp);
    let output_file = temp.write("dist-file", b"not-a-directory");

    let error = builder
        .build(host(), &sources, &output_file)
        .expect_err("输出路径是文件必须失败");
    assert!(
        matches!(error, BuildError::OutputPathNotDirectory { .. }),
        "实际错误：{error}"
    );
}
