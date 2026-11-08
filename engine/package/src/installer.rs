//! 安装/卸载：复制包目录到 `<project>/packages/<name>/` 并维护 lockfile。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::lockfile::{lock_path, LockedPackage, Lockfile, LOCK_FILE_NAME};
use crate::manifest::{PackageId, Version};
use crate::registry::PackageRegistry;
use crate::resolver::resolve_closure;

/// 一次安装的结果。
#[derive(Clone, Debug, Default)]
pub struct InstallOutcome {
    /// 本次新复制的包（依赖优先顺序）。
    pub installed: Vec<PackageId>,
    /// 已存在且版本一致、跳过的包。
    pub skipped: Vec<PackageId>,
    /// lockfile 路径。
    pub lock_path: PathBuf,
}

impl InstallOutcome {
    /// 是否全部为跳过（无实际复制）。
    pub fn is_noop(&self) -> bool {
        self.installed.is_empty()
    }
}

/// 工程级安装器。
#[derive(Clone, Debug)]
pub struct PackageInstaller {
    project_root: PathBuf,
}

impl PackageInstaller {
    /// 以工程根目录创建安装器。
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// 工程根目录。
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// `<project>/packages`。
    pub fn packages_dir(&self) -> PathBuf {
        self.project_root.join("packages")
    }

    /// `<project>/packages/lock.json`。
    pub fn lockfile_path(&self) -> PathBuf {
        lock_path(&self.project_root)
    }

    /// 读取已安装记录。
    pub fn installed(&self) -> Result<Lockfile, String> {
        Lockfile::load(&self.lockfile_path())
    }

    /// 从本地 registry 安装包及其依赖闭包。
    ///
    /// 失败不修改已有安装（先全部预检；复制单个包时用临时目录原子替换）。
    pub fn install(
        &self,
        registry_dir: &Path,
        package_name: &str,
    ) -> Result<InstallOutcome, String> {
        let registry = PackageRegistry::scan(registry_dir)?;
        let resolved = resolve_closure(&registry, &[package_name.to_string()])
            .map_err(|error| error.to_string())?;
        let mut lock = Lockfile::load(&self.lockfile_path())?;
        let packages_dir = self.packages_dir();

        // 预检 1：已安装版本冲突。
        for package in &resolved {
            if let Some(existing) = lock.find(&package.id.name) {
                if existing.version != package.id.version.to_string() {
                    return Err(format!(
                        "版本冲突: {} 已安装 {}，但解析结果为 {}（请先卸载或调整依赖）",
                        package.id.name, existing.version, package.id.version
                    ));
                }
            }
        }
        // 预检 2：目标目录存在但 lockfile 无记录（非本工具创建，拒绝覆盖）。
        for package in &resolved {
            let target = packages_dir.join(&package.id.name);
            if target.exists() && lock.find(&package.id.name).is_none() {
                return Err(format!(
                    "目标目录已存在但 lockfile 无记录: {}（请手动清理）",
                    target.display()
                ));
            }
        }

        let mut outcome = InstallOutcome {
            lock_path: self.lockfile_path(),
            ..Default::default()
        };
        for package in resolved {
            let target = packages_dir.join(&package.id.name);
            if !target.starts_with(&packages_dir) {
                return Err(format!("安装路径越界: {}", target.display()));
            }
            let already_installed = lock
                .find(&package.id.name)
                .map(|entry| entry.version == package.id.version.to_string())
                .unwrap_or(false);
            let hash = if already_installed && target.is_dir() {
                lock.find(&package.id.name)
                    .map(|entry| entry.hash.clone())
                    .unwrap_or_default()
            } else {
                let manifest = crate::manifest::PackageManifest::load(&package.dir.join("package.json"))?;
                let id = manifest
                    .validate()
                    .map_err(|errors| format!("包清单无效（{}）: {}", package.dir.display(), errors.join("; ")))?;
                if id.name != package.id.name || id.version != package.id.version {
                    return Err(format!("包清单与注册表不一致: {}", package.id));
                }
                copy_package(&package.dir, &target, &manifest.files, &package.id)?;
                hash_directory(&target)?
            };
            let mut dependencies = BTreeMap::new();
            for (dep_name, dep_id) in &package.dependencies {
                dependencies.insert(dep_name.clone(), dep_id.version.to_string());
            }
            let entry = LockedPackage {
                name: package.id.name.clone(),
                version: package.id.version.to_string(),
                hash,
                dependencies,
                path: package.id.name.clone(),
            };
            lock.upsert(entry);
            if already_installed && target.is_dir() {
                outcome.skipped.push(package.id);
            } else {
                outcome.installed.push(package.id);
            }
        }
        lock.save(&self.lockfile_path())?;
        Ok(outcome)
    }

    /// 卸载包（被其他包依赖时拒绝；先检查后删除）。
    pub fn uninstall(&self, name: &str) -> Result<(), String> {
        let mut lock = Lockfile::load(&self.lockfile_path())?;
        let package = lock
            .find(name)
            .cloned()
            .ok_or_else(|| format!("未安装: {name}"))?;
        let dependents = lock.dependents_of(name);
        if !dependents.is_empty() {
            let names: Vec<&str> = dependents.iter().map(|package| package.name.as_str()).collect();
            return Err(format!(
                "无法卸载 {name}: 被依赖 [{}]（先卸载依赖方）",
                names.join(", ")
            ));
        }
        let relative = if package.path.is_empty() {
            package.name.clone()
        } else {
            package.path.clone()
        };
        if !crate::manifest::is_valid_package_name(&relative) {
            return Err(format!("lockfile 记录路径非法: {relative}"));
        }
        let target = self.packages_dir().join(&relative);
        if !target.starts_with(self.packages_dir()) {
            return Err(format!("卸载路径越界: {}", target.display()));
        }
        if target.is_dir() {
            std::fs::remove_dir_all(&target)
                .map_err(|error| format!("删除包目录失败（{}）: {error}", target.display()))?;
        }
        lock.remove(name);
        lock.save(&self.lockfile_path())?;
        Ok(())
    }

    /// 读取任意 registry 并返回包列表（编辑器 UI 用）。
    pub fn registry_summary(&self, registry_dir: &Path) -> Result<String, String> {
        let registry = PackageRegistry::scan(registry_dir)?;
        let packages: Vec<serde_json::Value> = registry
            .entries()
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "name": entry.id.name,
                    "version": entry.id.version.to_string(),
                    "display_name": if entry.manifest.display_name.is_empty() {
                        entry.id.name.clone()
                    } else {
                        entry.manifest.display_name.clone()
                    },
                    "description": entry.manifest.description.clone(),
                    "dependencies": entry.manifest.dependencies.clone(),
                })
            })
            .collect();
        let installed = Lockfile::load(&self.lockfile_path()).unwrap_or_default();
        let value = serde_json::json!({
            "root": registry.root().to_string_lossy().replace('\\', "/"),
            "count": packages.len(),
            "packages": packages,
            "installed": installed.packages.iter().map(|package| package.name.clone()).collect::<Vec<_>>(),
        });
        Ok(value.to_string())
    }

    /// 已安装记录 JSON（不存在时返回空记录）。
    pub fn installed_json(&self) -> String {
        let lock = Lockfile::load(&self.lockfile_path()).unwrap_or_default();
        serde_json::to_string(&lock).unwrap_or_else(|_| "{\"version\":1,\"packages\":[]}".to_string())
    }
}

fn copy_package(
    source: &Path,
    target: &Path,
    whitelist: &[String],
    id: &PackageId,
) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!("包目录不存在: {}", source.display()));
    }
    let manifest_path = source.join("package.json");
    if !manifest_path.is_file() {
        return Err(format!("包缺少 package.json: {}", source.display()));
    }
    let parent = target
        .parent()
        .ok_or_else(|| format!("安装目标无父目录: {}", target.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("创建 packages 目录失败（{}）: {error}", parent.display()))?;
    let temp = parent.join(format!(".{}.tmp-{}", id.name, std::process::id()));
    if temp.exists() {
        let _ = std::fs::remove_dir_all(&temp);
    }
    std::fs::create_dir_all(&temp)
        .map_err(|error| format!("创建临时目录失败（{}）: {error}", temp.display()))?;

    let result = (|| -> Result<(), String> {
        if whitelist.is_empty() {
            copy_tree(source, source, &temp)?;
        } else {
            // 白名单文件 + 清单本身。
            copy_file(&manifest_path, &temp.join("package.json"))?;
            for relative in whitelist {
                if relative.contains("..") || Path::new(relative).is_absolute() {
                    return Err(format!("白名单路径非法: {relative}"));
                }
                let file = source.join(relative);
                if !file.is_file() {
                    return Err(format!("白名单文件缺失: {}（包 {}）", relative, id));
                }
                copy_file(&file, &temp.join(relative))?;
            }
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            if target.exists() {
                std::fs::remove_dir_all(target).map_err(|error| {
                    format!("清理旧安装目录失败（{}）: {error}", target.display())
                })?;
            }
            std::fs::rename(&temp, target)
                .map_err(|error| format!("提交安装目录失败（{}）: {error}", target.display()))?;
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::remove_dir_all(&temp);
            Err(error)
        }
    }
}

fn copy_tree(root: &Path, current: &Path, destination: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(current)
        .map_err(|error| format!("读取目录失败（{}）: {error}", current.display()))?
    {
        let entry = entry.map_err(|error| format!("读取目录项失败: {error}"))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("相对路径错误: {error}"))?;
        let target = destination.join(relative);
        if path.is_dir() {
            copy_tree(root, &path, destination)?;
        } else if path.is_file() {
            copy_file(&path, &target)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("创建目录失败（{}）: {error}", parent.display()))?;
    }
    std::fs::copy(source, target).map_err(|error| {
        format!(
            "复制文件失败（{} -> {}）: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

/// 目录内容指纹：排序后的相对路径 + 内容，fnv1a64。
pub fn hash_directory(directory: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(directory, directory, &mut files)?;
    files.sort();
    let mut hasher: u64 = 0xcbf29ce484222325;
    for (relative, path) in files {
        for byte in relative.as_bytes() {
            hasher ^= *byte as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        hasher ^= 0;
        hasher = hasher.wrapping_mul(0x100000001b3);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("读取文件失败（{}）: {error}", path.display()))?;
        for byte in bytes {
            hasher ^= byte as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("fnv1a64:{hasher:016x}"))
}

fn collect_files(root: &Path, current: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    for entry in std::fs::read_dir(current)
        .map_err(|error| format!("读取目录失败（{}）: {error}", current.display()))?
    {
        let entry = entry.map_err(|error| format!("读取目录项失败: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("相对路径错误: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            out.push((relative, path));
        }
    }
    Ok(())
}

/// 默认 registry 目录候选（cwd/packages/registry、cwd/../packages/registry）。
pub fn default_registry_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    for candidate in [
        cwd.join("packages").join("registry"),
        cwd.join("..").join("packages").join("registry"),
    ] {
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

/// lockfile 文件名常量导出（FFI 文档用）。
pub const LOCK_FILE: &str = LOCK_FILE_NAME;

/// 版本显示辅助。
pub fn format_version(version: &Version) -> String {
    version.to_string()
}

