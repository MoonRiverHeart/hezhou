//! 编辑器包管理器 FFI（0.4 task-52）。
//!
//! 封装 `hezhou-package`：本地 registry 列表、安装（写 lockfile）、已安装列表、卸载。
//! 失败安全：错误写入可读 JSON / 返回码，不 panic。

use std::ffi::c_char;
use std::path::PathBuf;

use hezhou_dfx::*;
use hezhou_package::{default_registry_dir, PackageInstaller};
use serde_json::json;

fn read_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { std::ffi::CStr::from_ptr(ptr).to_str().ok().map(str::to_string) }
}

fn write_c_string(buffer: *mut c_char, buffer_size: usize, text: &str) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
        *buffer.add(copy_len) = 0;
    }
    copy_len
}

fn project_root_from(text: &str) -> PathBuf {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        PathBuf::from(trimmed)
    }
}

fn registry_from(text: &str) -> Result<PathBuf, String> {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        let path = PathBuf::from(trimmed);
        if path.is_dir() {
            return Ok(path);
        }
        return Err(format!("registry 目录不存在: {}", path.display()));
    }
    default_registry_dir().ok_or_else(|| "未找到默认 registry（packages/registry）".to_string())
}

/// FFI：列出本地 registry 包 + 已安装状态。
///
/// 写入 JSON：`{root,count,packages:[{name,version,display_name,description,dependencies}],installed:[...]}`；
/// `registry_dir` 为空时使用默认 `packages/registry`。
#[unsafe(no_mangle)]
pub extern "C" fn package_registry_list_json(
    registry_dir: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let requested = read_c_string(registry_dir).unwrap_or_default();
    let registry = match registry_from(&requested) {
        Ok(path) => path,
        Err(error) => {
            dfx_warn!("Package", "registry 不可用: {}", error);
            let value = json!({ "error": error, "count": 0, "packages": [], "installed": [] });
            return write_c_string(buffer, buffer_size, &value.to_string());
        }
    };
    let project_root = project_root_from("");
    let installer = PackageInstaller::new(&project_root);
    match installer.registry_summary(&registry) {
        Ok(json_text) => write_c_string(buffer, buffer_size, &json_text),
        Err(error) => {
            dfx_warn!("Package", "扫描 registry 失败: {}", error);
            let value = json!({ "error": error, "count": 0, "packages": [], "installed": [] });
            write_c_string(buffer, buffer_size, &value.to_string())
        }
    }
}

/// FFI：从本地 registry 安装包（含依赖闭包）并写 lockfile。
///
/// 返回 0=成功；-1=失败（失败原因写入 buffer JSON）。
#[unsafe(no_mangle)]
pub extern "C" fn package_install_json(
    project_root: *const c_char,
    registry_dir: *const c_char,
    name: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> i32 {
    if buffer.is_null() || buffer_size == 0 {
        return -1;
    }
    let Some(name) = read_c_string(name) else {
        write_c_string(buffer, buffer_size, "{\"error\":\"包名为空\"}");
        return -1;
    };
    if name.trim().is_empty() {
        write_c_string(buffer, buffer_size, "{\"error\":\"包名为空\"}");
        return -1;
    }
    let project_text = read_c_string(project_root).unwrap_or_default();
    let registry_text = read_c_string(registry_dir).unwrap_or_default();
    let project = project_root_from(&project_text);
    let registry = match registry_from(&registry_text) {
        Ok(path) => path,
        Err(error) => {
            dfx_warn!("Package", "安装失败: {}", error);
            write_c_string(buffer, buffer_size, &json!({ "error": error }).to_string());
            return -1;
        }
    };
    let installer = PackageInstaller::new(&project);
    match installer.install(&registry, name.trim()) {
        Ok(outcome) => {
            let installed: Vec<String> = outcome
                .installed
                .iter()
                .map(|id| id.to_string())
                .collect();
            let skipped: Vec<String> = outcome
                .skipped
                .iter()
                .map(|id| id.to_string())
                .collect();
            let value = json!({
                "ok": true,
                "installed": installed,
                "skipped": skipped,
                "noop": outcome.is_noop(),
                "lock_path": outcome.lock_path.to_string_lossy().replace('\\', "/"),
                "project_root": project.to_string_lossy().replace('\\', "/"),
            });
            dfx_info!(
                "Package",
                "安装完成: {}（新增 {}，跳过 {}）",
                name.trim(),
                installed.len(),
                skipped.len()
            );
            write_c_string(buffer, buffer_size, &value.to_string());
            0
        }
        Err(error) => {
            dfx_warn!("Package", "安装失败 {}: {}", name.trim(), error);
            write_c_string(buffer, buffer_size, &json!({ "ok": false, "error": error }).to_string());
            -1
        }
    }
}

/// FFI：读取已安装列表（lockfile JSON；不存在返回空记录）。
#[unsafe(no_mangle)]
pub extern "C" fn package_installed_json(
    project_root: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let project_text = read_c_string(project_root).unwrap_or_default();
    let installer = PackageInstaller::new(project_root_from(&project_text));
    write_c_string(buffer, buffer_size, &installer.installed_json())
}

/// FFI：卸载包。返回 0=成功；-1=失败（未安装/被依赖/IO 错误，原因写日志）。
#[unsafe(no_mangle)]
pub extern "C" fn package_uninstall(
    project_root: *const c_char,
    name: *const c_char,
) -> i32 {
    let Some(name) = read_c_string(name) else {
        return -1;
    };
    if name.trim().is_empty() {
        return -1;
    }
    let project_text = read_c_string(project_root).unwrap_or_default();
    let installer = PackageInstaller::new(project_root_from(&project_text));
    match installer.uninstall(name.trim()) {
        Ok(()) => {
            dfx_info!("Package", "已卸载: {}", name.trim());
            0
        }
        Err(error) => {
            dfx_warn!("Package", "卸载失败 {}: {}", name.trim(), error);
            -1
        }
    }
}
