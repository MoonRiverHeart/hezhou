//! 插件清单（`plugin.json`）与 action 白名单。

use std::path::Path;

use serde::{Deserialize, Serialize};

/// 当前编辑器插件 API 版本。
pub const API_VERSION: &str = "0.4";

/// 允许插件命令引用的编辑器内置动作白名单。
pub const ACTION_WHITELIST: [&str; 9] = [
    "new_project",
    "open_project",
    "save",
    "toggle_theme",
    "console",
    "command_palette",
    "profiler_tab",
    "packages_tab",
    "new_scene",
];

/// 插件命令（声明式；action 必须在白名单内）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginCommand {
    /// 命令 id（插件内唯一）。
    pub id: String,
    /// 显示标题。
    pub title: String,
    /// 快捷键提示（仅展示）。
    pub shortcut: String,
    /// 内置动作名（白名单）。
    pub action: String,
}

/// 插件菜单（可选；引用命令 id）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginMenu {
    /// 菜单 id。
    pub id: String,
    /// 菜单标题。
    pub title: String,
    /// 点击执行的命令 id。
    pub command: String,
}

/// 插件清单。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginManifest {
    /// 插件 id（`[a-z0-9][a-z0-9._-]*`）。
    pub id: String,
    /// 显示名。
    pub name: String,
    /// 版本（`major.minor.patch`）。
    pub version: String,
    /// 兼容的编辑器插件 API 版本（如 `0.4`）。
    pub api_version: String,
    /// 描述。
    pub description: String,
    /// 作者。
    pub author: String,
    /// 默认启用（enabled.json 不存在时生效）。
    pub enabled_by_default: bool,
    /// 命令贡献。
    pub commands: Vec<PluginCommand>,
    /// 菜单贡献（可选）。
    pub menus: Vec<PluginMenu>,
    /// 资产声明（相对插件根）。
    pub assets: Vec<String>,
    /// 脚本声明（相对插件根）。
    pub scripts: Vec<String>,
}

impl PluginManifest {
    /// 从 JSON 文本解析。
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str::<Self>(text).map_err(|error| format!("plugin.json 解析失败: {error}"))
    }

    /// 读取 `plugin.json`。
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
        Self::from_json(&text)
    }

    /// 校验清单；失败返回全部可读错误。
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !is_valid_plugin_id(&self.id) {
            errors.push(format!(
                "id 非法: {}（要求小写字母/数字开头，允许 . - _，≤64）",
                self.id
            ));
        }
        if self.name.trim().is_empty() {
            errors.push("name 不能为空".to_string());
        }
        if !is_version_like(&self.version) {
            errors.push(format!("version 必须为 x.y.z: {}", self.version));
        }
        if !is_api_compatible(&self.api_version) {
            errors.push(format!(
                "api_version 不兼容: {}（要求 {}）",
                self.api_version, API_VERSION
            ));
        }
        let mut seen_commands = std::collections::BTreeSet::new();
        for command in &self.commands {
            if !is_valid_command_id(&command.id) {
                errors.push(format!("命令 id 非法: {}", command.id));
            }
            if !seen_commands.insert(command.id.clone()) {
                errors.push(format!("重复命令 id: {}", command.id));
            }
            if command.title.trim().is_empty() {
                errors.push(format!("命令 {} title 不能为空", command.id));
            }
            if !ACTION_WHITELIST.contains(&command.action.trim()) {
                errors.push(format!(
                    "命令 {} 使用未知 action: {}（白名单: {}）",
                    command.id,
                    command.action,
                    ACTION_WHITELIST.join(",")
                ));
            }
        }
        for menu in &self.menus {
            if menu.id.trim().is_empty() || menu.title.trim().is_empty() {
                errors.push("菜单 id/title 不能为空".to_string());
            }
            if !seen_commands.contains(&menu.command) {
                errors.push(format!("菜单 {} 引用未知命令: {}", menu.id, menu.command));
            }
        }
        for (field, values) in [("assets", &self.assets), ("scripts", &self.scripts)] {
            for value in values {
                if !is_safe_relative_path(value) {
                    errors.push(format!("{field} 路径非法（绝对路径/..）: {value}"));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// 插件 id 合法性。
pub fn is_valid_plugin_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    id.len() <= 64
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_'))
}

fn is_valid_command_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_'))
}

fn is_version_like(version: &str) -> bool {
    let parts: Vec<&str> = version.trim().split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_api_compatible(api_version: &str) -> bool {
    let trimmed = api_version.trim();
    trimmed == API_VERSION || trimmed.starts_with(&format!("{API_VERSION}."))
}

fn is_safe_relative_path(path: &str) -> bool {
    let candidate = Path::new(path);
    if path.trim().is_empty() || candidate.is_absolute() {
        return false;
    }
    candidate
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_) | std::path::Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_manifest_and_whitelist() {
        let manifest = PluginManifest::from_json(
            r#"{
                "id": "sample-tools",
                "name": "Sample Tools",
                "version": "0.1.0",
                "api_version": "0.4",
                "author": "Hezhou",
                "commands": [
                    { "id": "toggle-theme", "title": "切换主题", "action": "toggle_theme" },
                    { "id": "console", "title": "打开 Console", "action": "console" }
                ],
                "menus": [ { "id": "tools", "title": "工具", "command": "console" } ],
                "assets": ["assets/readme.txt"],
                "scripts": ["scripts/demo.cs"]
            }"#,
        )
        .unwrap();
        manifest.validate().expect("应通过校验");

        let bad = PluginManifest::from_json(
            r#"{
                "id": "Bad ID",
                "name": "",
                "version": "x",
                "api_version": "9.9",
                "commands": [
                    { "id": "run", "title": "", "action": "exec_shell" },
                    { "id": "run", "title": "dup", "action": "console" }
                ],
                "menus": [ { "id": "m", "title": "菜单", "command": "missing" } ],
                "assets": ["../escape"]
            }"#,
        )
        .unwrap();
        let errors = bad.validate().expect_err("应校验失败");
        let joined = errors.join(";");
        for expected in ["id 非法", "name", "version", "api_version", "未知 action", "重复命令 id", "未知命令", "assets"] {
            assert!(joined.contains(expected), "缺少错误 {expected}: {joined}");
        }
    }
}
