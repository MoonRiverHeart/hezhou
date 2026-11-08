//! 编辑器声明式插件 FFI（0.4 task-55）。
//!
//! 封装 `hezhou-plugin`：插件列表/启用状态/启用插件命令。失败安全。

use std::ffi::c_char;

use hezhou_dfx::*;
use hezhou_plugin::registry;
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

fn project_root_from(text: &str) -> std::path::PathBuf {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    } else {
        std::path::PathBuf::from(trimmed)
    }
}

/// FFI：插件列表 JSON（含启用状态/命令数）。
#[unsafe(no_mangle)]
pub extern "C" fn plugin_list_json(
    project_root: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let root_text = read_c_string(project_root).unwrap_or_default();
    let root = project_root_from(&root_text);
    let json_text = registry::list_json(&root);
    write_c_string(buffer, buffer_size, &json_text)
}

/// FFI：启用/禁用插件（持久化 plugins/enabled.json）。0=成功，-1=失败。
#[unsafe(no_mangle)]
pub extern "C" fn plugin_set_enabled(
    project_root: *const c_char,
    id: *const c_char,
    enabled: i32,
) -> i32 {
    let Some(id) = read_c_string(id) else {
        return -1;
    };
    if id.trim().is_empty() {
        return -1;
    }
    let root_text = read_c_string(project_root).unwrap_or_default();
    let root = project_root_from(&root_text);
    let rc = registry::set_enabled(&root, id.trim(), enabled != 0);
    if rc == 0 {
        dfx_info!("Plugin", "插件 {} 已{}", id.trim(), if enabled != 0 { "启用" } else { "禁用" });
    } else {
        dfx_warn!("Plugin", "设置插件状态失败: {}", id.trim());
    }
    rc
}

/// FFI：启用插件的命令 JSON（Command Palette 用）。
#[unsafe(no_mangle)]
pub extern "C" fn plugin_commands_json(
    project_root: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let root_text = read_c_string(project_root).unwrap_or_default();
    let root = project_root_from(&root_text);
    let commands: Vec<serde_json::Value> = match hezhou_plugin::PluginRegistry::scan(&root) {
        Ok(registry) => registry
            .enabled_commands()
            .iter()
            .map(|(plugin_id, command)| {
                json!({
                    "plugin_id": plugin_id,
                    "id": command.id,
                    "title": command.title,
                    "shortcut": command.shortcut,
                    "action": command.action,
                })
            })
            .collect(),
        Err(error) => {
            dfx_warn!("Plugin", "读取插件命令失败: {}", error);
            Vec::new()
        }
    };
    let value = json!({ "count": commands.len(), "commands": commands });
    write_c_string(buffer, buffer_size, &value.to_string())
}
