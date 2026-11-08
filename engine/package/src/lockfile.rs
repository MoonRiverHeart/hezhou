//! 安装记录 lockfile（`<project>/packages/lock.json`）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// lockfile 文件名（位于 `<project>/packages/`）。
pub const LOCK_FILE_NAME: &str = "lock.json";

/// 单个已安装包记录。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedPackage {
    /// 包名。
    pub name: String,
    /// 版本（`x.y.z`）。
    pub version: String,
    /// 安装内容指纹（fnv1a64:...，确定性）。
    pub hash: String,
    /// 依赖 name -> version。
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    /// 相对 packages/ 的安装子目录（通常为 name）。
    #[serde(default)]
    pub path: String,
}

/// 安装记录。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lockfile {
    /// 格式版本（当前 1）。
    pub version: u32,
    /// 已安装包（按 name 排序）。
    pub packages: Vec<LockedPackage>,
}

impl Default for Lockfile {
    fn default() -> Self {
        Self {
            version: 1,
            packages: Vec::new(),
        }
    }
}

impl Lockfile {
    /// 从文件加载；不存在视为空记录。
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("读取 lockfile 失败（{}）: {error}", path.display()))?;
        let lock: Self = serde_json::from_str(&text)
            .map_err(|error| format!("解析 lockfile 失败（{}）: {error}", path.display()))?;
        Ok(lock)
    }

    /// 保存（排序 + 稳定 JSON）。
    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        self.packages.sort_by(|left, right| left.name.cmp(&right.name));
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| format!("序列化 lockfile 失败: {error}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("创建 lock 目录失败（{}）: {error}", parent.display()))?;
        }
        std::fs::write(path, json)
            .map_err(|error| format!("写入 lockfile 失败（{}）: {error}", path.display()))
    }

    /// 查找记录。
    pub fn find(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|package| package.name == name)
    }

    /// 插入/替换记录。
    pub fn upsert(&mut self, package: LockedPackage) {
        if let Some(existing) = self
            .packages
            .iter_mut()
            .find(|existing| existing.name == package.name)
        {
            *existing = package;
        } else {
            self.packages.push(package);
        }
    }

    /// 移除记录并返回是否存在。
    pub fn remove(&mut self, name: &str) -> Option<LockedPackage> {
        let index = self
            .packages
            .iter()
            .position(|package| package.name == name)?;
        Some(self.packages.remove(index))
    }

    /// lockfile 是否为空。
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// 依赖 `name` 的已安装包列表。
    pub fn dependents_of(&self, name: &str) -> Vec<&LockedPackage> {
        self.packages
            .iter()
            .filter(|package| package.dependencies.contains_key(name))
            .collect()
    }
}

/// 便捷路径：`<project_root>/packages/lock.json`。
pub fn lock_path(project_root: &Path) -> PathBuf {
    project_root.join("packages").join(LOCK_FILE_NAME)
}
