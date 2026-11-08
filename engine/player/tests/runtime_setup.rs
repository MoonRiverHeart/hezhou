//! 内置 Mono 运行时启动准备测试（task-38）：runtime/mono 探测、环境变量前置、no-op、幂等。

mod common;

use std::path::Path;
use std::sync::Mutex;

use common::TempDir;
use hezhou_player::platform::TargetPlatform;
use hezhou_player::runtime_setup::{
    setup_mono_runtime, setup_mono_runtime_for, MonoRuntimeSetup, MONO_PATH_ENV,
};

/// 环境变量测试串行化（同一测试进程内避免并行竞争）。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_env_guard<R>(f: impl FnOnce() -> R) -> R {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = std::env::var("PATH").ok();
    let mono_path = std::env::var(MONO_PATH_ENV).ok();
    let ld_path = std::env::var("LD_LIBRARY_PATH").ok();
    let result = f();
    restore("PATH", path);
    restore(MONO_PATH_ENV, mono_path);
    restore("LD_LIBRARY_PATH", ld_path);
    result
}

fn restore(name: &str, value: Option<String>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}

fn separator(platform: TargetPlatform) -> char {
    match platform {
        TargetPlatform::Windows => ';',
        TargetPlatform::Linux => ':',
    }
}

fn native_env(platform: TargetPlatform) -> &'static str {
    match platform {
        TargetPlatform::Windows => "PATH",
        TargetPlatform::Linux => "LD_LIBRARY_PATH",
    }
}

fn first_entry(name: &str, platform: TargetPlatform) -> Option<String> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.split(separator(platform)).next().map(str::to_string))
}

#[test]
fn missing_runtime_is_noop() {
    with_env_guard(|| {
        let temp = TempDir::new("runtime-setup-noop");
        let before_path = std::env::var("PATH").ok();
        let before_mono = std::env::var(MONO_PATH_ENV).ok();
        let setup = setup_mono_runtime(temp.path());
        assert!(!setup.found);
        assert!(setup.modified.is_empty());
        assert_eq!(std::env::var("PATH").ok(), before_path);
        assert_eq!(std::env::var(MONO_PATH_ENV).ok(), before_mono);
        assert!(setup.summary().contains("no-op"));
        assert_eq!(
            setup,
            MonoRuntimeSetup::not_found(TargetPlatform::host())
        );
    });
}

#[test]
fn found_runtime_prepends_env_and_is_idempotent() {
    with_env_guard(|| {
        let temp = TempDir::new("runtime-setup-found");
        temp.write("runtime/mono/bin/mono.exe", b"fake-mono");
        temp.write("runtime/mono/lib/libmono-2.0.so", b"fake-libmono");
        let platform = TargetPlatform::host();
        let setup = setup_mono_runtime(temp.path());
        assert!(setup.found, "应检测到 runtime/mono");
        assert_eq!(setup.root.as_deref(), Some(temp.join("runtime/mono").as_path()));
        assert_eq!(
            setup.native_dir.as_deref(),
            Some(temp.join("runtime/mono/bin").as_path())
        );
        assert_eq!(
            setup.managed_dir.as_deref(),
            Some(temp.join("runtime/mono/lib").as_path())
        );
        assert!(setup.summary().contains("内置 Mono 运行时：已找到"));
        assert_eq!(
            first_entry(native_env(platform), platform).as_deref(),
            temp.join("runtime/mono/bin").to_str()
        );
        assert_eq!(
            first_entry(MONO_PATH_ENV, platform).as_deref(),
            temp.join("runtime/mono/lib").to_str()
        );

        let path_after_first = std::env::var(native_env(platform)).unwrap();
        let mono_after_first = std::env::var(MONO_PATH_ENV).unwrap();
        let second = setup_mono_runtime(temp.path());
        assert!(second.found);
        assert!(second.modified.is_empty(), "第二次应为幂等 no-op");
        assert_eq!(std::env::var(native_env(platform)).unwrap(), path_after_first);
        assert_eq!(std::env::var(MONO_PATH_ENV).unwrap(), mono_after_first);
    });
}

#[test]
fn missing_bin_or_lib_falls_back_to_mono_root() {
    with_env_guard(|| {
        let temp = TempDir::new("runtime-setup-flat");
        // 只有 runtime/mono 根目录（没有 bin/lib）。
        temp.write("runtime/mono/readme.txt", b"flat mono");
        let platform = TargetPlatform::host();
        let setup = setup_mono_runtime(temp.path());
        assert!(setup.found);
        let mono_root = temp.join("runtime/mono");
        assert_eq!(setup.native_dir.as_deref(), Some(mono_root.as_path()));
        assert_eq!(setup.managed_dir.as_deref(), Some(mono_root.as_path()));
        assert_eq!(
            first_entry(native_env(platform), platform).as_deref(),
            mono_root.to_str()
        );
    });
}

#[test]
fn explicit_platform_env_names_are_correct() {
    with_env_guard(|| {
        let temp = TempDir::new("runtime-setup-platform");
        temp.write("runtime/mono/bin/mono.exe", b"fake");
        temp.write("runtime/mono/lib/libmono.so", b"fake");

        let windows = setup_mono_runtime_for(temp.path(), TargetPlatform::Windows);
        assert!(windows.found);
        assert!(windows.modified.contains(&"PATH"));
        assert!(windows.modified.contains(&MONO_PATH_ENV));
        let path = std::env::var("PATH").unwrap();
        assert!(
            path.starts_with(temp.join("runtime/mono/bin").to_str().unwrap()),
            "PATH 应以 mono/bin 开头：{path}"
        );

        // 重新 prepend Linux 变量（同一临时目录；Windows 盘符冒号不参与 Linux 列表断言）。
        let linux = setup_mono_runtime_for(temp.path(), TargetPlatform::Linux);
        assert!(linux.found);
        assert!(linux.modified.contains(&"LD_LIBRARY_PATH"));
        let mono_path = std::env::var(MONO_PATH_ENV).unwrap();
        assert!(
            mono_path.starts_with(temp.join("runtime/mono/lib").to_str().unwrap()),
            "MONO_PATH 应以 mono/lib 开头：{mono_path}"
        );
        let ld_path = std::env::var("LD_LIBRARY_PATH").unwrap();
        assert!(
            ld_path.starts_with(temp.join("runtime/mono/lib").to_str().unwrap()),
            "LD_LIBRARY_PATH 应以 mono/lib 开头：{ld_path}"
        );
    });
}

/// 确保测试辅助类型签名被使用（避免未使用导入警告）。
#[allow(dead_code)]
fn _path_type(_: &Path) {}
