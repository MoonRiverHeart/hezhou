//! 本地包注册表：扫描目录中的 `package.json` 并按 name/version 建索引。

use std::path::{Path, PathBuf};

use crate::manifest::{PackageId, PackageManifest, Version, VersionReq};

/// 注册表中的一个包。
#[derive(Clone, Debug)]
pub struct PackageEntry {
    /// 包标识（name@version）。
    pub id: PackageId,
    /// 包目录（只读源）。
    pub dir: PathBuf,
    /// 原始清单。
    pub manifest: PackageManifest,
}

/// 本地包注册表（仅扫描本地目录，不联网）。
#[derive(Clone, Debug, Default)]
pub struct PackageRegistry {
    root: PathBuf,
    entries: Vec<PackageEntry>,
}

impl PackageRegistry {
    /// 扫描 `dir` 下每个子目录的 `package.json`。
    ///
    /// 校验失败/重复包会导致整体失败（避免半可用注册表）。
    pub fn scan(dir: &Path) -> Result<Self, String> {
        if !dir.is_dir() {
            return Err(format!("注册表目录不存在: {}", dir.display()));
        }
        let mut entries = Vec::new();
        let read_dir = std::fs::read_dir(dir)
            .map_err(|error| format!("读取注册表 {} 失败: {error}", dir.display()))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("package.json");
            if !manifest_path.is_file() {
                continue;
            }
            let manifest = PackageManifest::load(&manifest_path)?;
            let id = manifest.validate().map_err(|errors| {
                format!("包清单无效（{}）: {}", manifest_path.display(), errors.join("; "))
            })?;
            if manifest.display_name.is_empty() {
                // display_name 可选：缺省用 name 展示。
            }
            entries.push(PackageEntry {
                id,
                dir: path,
                manifest,
            });
        }
        entries.sort_by(|left, right| {
            left.id
                .name
                .cmp(&right.id.name)
                .then_with(|| left.id.version.cmp(&right.id.version))
        });
        for pair in entries.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(format!("注册表存在重复包: {}", pair[0].id));
            }
        }
        Ok(Self {
            root: dir.to_path_buf(),
            entries,
        })
    }

    /// 注册表根目录。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 全部包（按 name/version 升序）。
    pub fn entries(&self) -> &[PackageEntry] {
        &self.entries
    }

    /// 按 name 列出全部版本（升序）。
    pub fn versions_of(&self, name: &str) -> Vec<&PackageEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.id.name == name)
            .collect()
    }

    /// 解析 name + 版本范围 → 最高匹配版本。
    pub fn resolve(&self, name: &str, requirement: &VersionReq) -> Option<&PackageEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.id.name == name && requirement.matches(&entry.id.version))
            .max_by_key(|entry| entry.id.version)
    }

    /// 最高版本。
    pub fn latest(&self, name: &str) -> Option<&PackageEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.id.name == name)
            .max_by_key(|entry| entry.id.version)
    }
}

/// 版本排序辅助（保持公开以便上层展示）。
pub fn sort_versions_ascending(left: &Version, right: &Version) -> std::cmp::Ordering {
    left.cmp(right)
}
