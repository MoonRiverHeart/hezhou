//! 插件注册表：扫描 `<project>/plugins/<id>/plugin.json` + 启用状态。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::manifest::{PluginCommand, PluginManifest};

/// 启用状态文件名（位于 `<project>/plugins/`）。
pub const ENABLED_FILE_NAME: &str = "enabled.json";

/// 启用状态（`enabled.json`）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EnabledState {
    /// 已启用插件 id（有序）。
    pub enabled: Vec<String>,
}

impl EnabledState {
    /// 读取；不存在返回默认（由 registry 用 enabled_by_default 补齐）。
    pub fn load(plugins_dir: &Path) -> Result<Self, String> {
        let path = plugins_dir.join(ENABLED_FILE_NAME);
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("读取 enabled.json 失败（{}）: {error}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|error| format!("解析 enabled.json 失败（{}）: {error}", path.display()))
    }

    /// 保存（排序去重）。
    pub fn save(&mut self, plugins_dir: &Path) -> Result<(), String> {
        self.enabled.sort();
        self.enabled.dedup();
        let path = plugins_dir.join(ENABLED_FILE_NAME);
        std::fs::create_dir_all(plugins_dir)
            .map_err(|error| format!("创建 plugins 目录失败（{}）: {error}", plugins_dir.display()))?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| format!("序列化 enabled.json 失败: {error}"))?;
        std::fs::write(&path, json)
            .map_err(|error| format!("写入 enabled.json 失败（{}）: {error}", path.display()))
    }

    /// 是否启用。
    pub fn contains(&self, id: &str) -> bool {
        self.enabled.iter().any(|entry| entry == id)
    }
}

/// 注册表中的一个插件。
#[derive(Clone, Debug)]
pub struct PluginEntry {
    /// 清单。
    pub manifest: PluginManifest,
    /// 插件目录。
    pub dir: PathBuf,
}

/// 本地插件注册表。
#[derive(Clone, Debug, Default)]
pub struct PluginRegistry {
    root: PathBuf,
    entries: Vec<PluginEntry>,
    enabled: EnabledState,
}

impl PluginRegistry {
    /// 扫描 `<project>/plugins`（目录不存在 → 空注册表）。
    pub fn scan(project_root: &Path) -> Result<Self, String> {
        let plugins_dir = project_root.join("plugins");
        let mut entries = Vec::new();
        if plugins_dir.is_dir() {
            let read_dir = std::fs::read_dir(&plugins_dir).map_err(|error| {
                format!("读取插件目录失败（{}）: {error}", plugins_dir.display())
            })?;
            for item in read_dir.flatten() {
                let path = item.path();
                if !path.is_dir() {
                    continue;
                }
                let manifest_path = path.join("plugin.json");
                if !manifest_path.is_file() {
                    continue;
                }
                let manifest = PluginManifest::load(&manifest_path)?;
                manifest.validate().map_err(|errors| {
                    format!("插件清单无效（{}）: {}", manifest_path.display(), errors.join("; "))
                })?;
                entries.push(PluginEntry {
                    manifest,
                    dir: path,
                });
            }
        }
        entries.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
        for pair in entries.windows(2) {
            if pair[0].manifest.id == pair[1].manifest.id {
                return Err(format!("重复插件 id: {}", pair[0].manifest.id));
            }
        }
        let has_state_file = plugins_dir.join(ENABLED_FILE_NAME).is_file();
        let mut enabled = if has_state_file {
            EnabledState::load(&plugins_dir)?
        } else {
            EnabledState {
                enabled: entries
                    .iter()
                    .filter(|entry| entry.manifest.enabled_by_default)
                    .map(|entry| entry.manifest.id.clone())
                    .collect(),
            }
        };
        enabled
            .enabled
            .retain(|id| entries.iter().any(|entry| &entry.manifest.id == id));
        Ok(Self {
            root: plugins_dir,
            entries,
            enabled,
        })
    }

    /// `<project>/plugins`。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 全部插件。
    pub fn entries(&self) -> &[PluginEntry] {
        &self.entries
    }

    /// 启用状态。
    pub fn enabled(&self) -> &EnabledState {
        &self.enabled
    }

    /// 插件是否启用。
    pub fn is_enabled(&self, id: &str) -> bool {
        self.enabled.contains(id)
    }

    /// 设置启用/禁用（持久化 `plugins/enabled.json`）。
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        if !self.entries.iter().any(|entry| entry.manifest.id == id) {
            return Err(format!("未知插件: {id}"));
        }
        let mut state = self.enabled.clone();
        if enabled {
            if !state.contains(id) {
                state.enabled.push(id.to_string());
            }
        } else {
            state.enabled.retain(|entry| entry != id);
        }
        state.save(&self.root)?;
        state.enabled.retain(|entry| {
            self.entries.iter().any(|plugin| &plugin.manifest.id == entry)
        });
        self.enabled = state;
        Ok(())
    }

    /// 启用插件的命令（按插件 id / 命令 id 排序）。
    pub fn enabled_commands(&self) -> Vec<(String, PluginCommand)> {
        let mut commands = Vec::new();
        for entry in &self.entries {
            if !self.enabled.contains(&entry.manifest.id) {
                continue;
            }
            for command in &entry.manifest.commands {
                commands.push((entry.manifest.id.clone(), command.clone()));
            }
        }
        commands.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.id.cmp(&right.1.id))
        });
        commands
    }
}

/// 便捷：读取插件列表 JSON（FFI 用，含 errors 为空）。
pub fn list_json(project_root: &Path) -> String {
    match PluginRegistry::scan(project_root) {
        Ok(registry) => {
            let plugins: Vec<serde_json::Value> = registry
                .entries()
                .iter()
                .map(|entry| {
                    serde_json::json!({
                        "id": entry.manifest.id,
                        "name": entry.manifest.name,
                        "version": entry.manifest.version,
                        "api_version": entry.manifest.api_version,
                        "author": entry.manifest.author,
                        "description": entry.manifest.description,
                        "enabled": registry.is_enabled(&entry.manifest.id),
                        "command_count": entry.manifest.commands.len(),
                        "commands": entry.manifest.commands,
                        "root": entry.dir.to_string_lossy().replace('\\', "/"),
                    })
                })
                .collect();
            serde_json::json!({
                "count": plugins.len(),
                "plugins": plugins,
                "enabled": registry.enabled().enabled,
            })
            .to_string()
        }
        Err(error) => serde_json::json!({ "error": error, "count": 0, "plugins": [] }).to_string(),
    }
}

/// 便捷：启用插件命令 JSON（Command Palette 用）。
pub fn commands_json(project_root: &Path) -> String {
    match PluginRegistry::scan(project_root) {
        Ok(registry) => {
            let commands: Vec<serde_json::Value> = registry
                .enabled_commands()
                .iter()
                .map(|(plugin_id, command)| {
                    serde_json::json!({
                        "plugin_id": plugin_id,
                        "id": command.id,
                        "title": command.title,
                        "shortcut": command.shortcut,
                        "action": command.action,
                    })
                })
                .collect();
            serde_json::json!({ "count": commands.len(), "commands": commands }).to_string()
        }
        Err(error) => serde_json::json!({ "error": error, "count": 0, "commands": [] }).to_string(),
    }
}

/// 设置启用状态（FFI 用）；0=成功。
pub fn set_enabled(project_root: &Path, id: &str, enabled: bool) -> i32 {
    match PluginRegistry::scan(project_root) {
        Ok(mut registry) => match registry.set_enabled(id, enabled) {
            Ok(()) => 0,
            Err(error) => {
                hezhou_dfx_log(&error);
                -1
            }
        },
        Err(error) => {
            hezhou_dfx_log(&error);
            -1
        }
    }
}

fn hezhou_dfx_log(message: &str) {
    // crate 不依赖 dfx，错误由调用方（FFI）记录；此处保持无副作用。
    let _ = message;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join("plugin_tests");
        std::fs::create_dir_all(&base).unwrap();
        let root = base.join(format!("{tag}_{}_{}", std::process::id(), nanos));
        std::fs::create_dir_all(root.join("plugins")).unwrap();
        root
    }

    fn write_plugin(root: &Path, id: &str, enabled_by_default: bool, commands: &[(&str, &str, &str)]) {
        let dir = root.join("plugins").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        let commands_json: Vec<serde_json::Value> = commands
            .iter()
            .map(|(command_id, title, action)| {
                serde_json::json!({ "id": command_id, "title": title, "action": action })
            })
            .collect();
        let manifest = serde_json::json!({
            "id": id,
            "name": id,
            "version": "0.1.0",
            "api_version": "0.4",
            "author": "test",
            "enabled_by_default": enabled_by_default,
            "commands": commands_json,
        });
        std::fs::write(
            dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn scans_registry_and_defaults_enabled() {
        let root = temp_project("scan");
        write_plugin(&root, "sample-tools", true, &[("toggle-theme", "切换主题", "toggle_theme"), ("console", "Console", "console")]);
        write_plugin(&root, "scene-tools", false, &[("save", "保存", "save")]);
        let registry = PluginRegistry::scan(&root).expect("扫描应成功");
        assert_eq!(registry.entries().len(), 2);
        assert!(registry.is_enabled("sample-tools"));
        assert!(!registry.is_enabled("scene-tools"));
        assert_eq!(registry.enabled_commands().len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn set_enabled_persists_and_filters_commands() {
        let root = temp_project("enable");
        write_plugin(&root, "sample-tools", false, &[("toggle-theme", "切换主题", "toggle_theme")]);
        write_plugin(&root, "scene-tools", false, &[("console", "Console", "console")]);
        let mut registry = PluginRegistry::scan(&root).unwrap();
        assert!(registry.enabled_commands().is_empty());
        registry.set_enabled("sample-tools", true).unwrap();
        registry.set_enabled("scene-tools", true).unwrap();
        assert_eq!(registry.enabled_commands().len(), 2);
        assert!(root.join("plugins").join(ENABLED_FILE_NAME).is_file());
        let mut registry2 = PluginRegistry::scan(&root).unwrap();
        assert!(registry2.is_enabled("sample-tools") && registry2.is_enabled("scene-tools"));
        registry2.set_enabled("sample-tools", false).unwrap();
        let mut registry3 = PluginRegistry::scan(&root).unwrap();
        assert!(!registry3.is_enabled("sample-tools"));
        assert!(registry3.set_enabled("missing", true).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_unknown_action_and_bad_paths() {
        let root = temp_project("reject");
        let dir = root.join("plugins").join("bad-plugin");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("plugin.json"),
            r#"{
                "id": "bad-plugin", "name": "Bad", "version": "0.1.0", "api_version": "0.4",
                "commands": [ { "id": "run", "title": "Run", "action": "exec_shell" } ],
                "assets": ["../escape.txt"]
            }"#,
        )
        .unwrap();
        let error = PluginRegistry::scan(&root).expect_err("未知 action 应拒绝");
        assert!(error.contains("未知 action"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lists_json_contains_commands_and_enabled_state() {
        let root = temp_project("json");
        write_plugin(&root, "sample-tools", true, &[("console", "Console", "console")]);
        let list = list_json(&root);
        assert!(list.contains("sample-tools"));
        assert!(list.contains("\"enabled\":true"));
        let commands = commands_json(&root);
        assert!(commands.contains("\"title\":\"Console\""));
        assert!(commands.contains("\"action\":\"console\""));
        let _ = std::fs::remove_dir_all(&root);
    }
}


