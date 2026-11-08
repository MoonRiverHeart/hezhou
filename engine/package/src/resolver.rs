//! 依赖解析：版本范围选择、拓扑排序、循环/缺失检测。

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use crate::manifest::{PackageId, VersionReq};
use crate::registry::PackageRegistry;

/// 解析错误（可读信息）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolveError {
    /// 错误原因。
    pub message: String,
}

impl ResolveError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ResolveError {}

/// 解析后的单个包（按依赖优先顺序）。
#[derive(Clone, Debug)]
pub struct ResolvedPackage {
    /// 包标识。
    pub id: PackageId,
    /// 包目录。
    pub dir: PathBuf,
    /// 依赖 name -> 已解析版本。
    pub dependencies: BTreeMap<String, PackageId>,
}

/// 解析入口包及其依赖闭包（依赖优先拓扑序）。
///
/// 规则：
/// - 同一包只选一个版本（取满足全部请求的最高版本）；
/// - 若已选版本不满足后续请求 → 版本冲突错误；
/// - 缺失包 / 循环依赖 / 版本范围非法 → 可读错误。
pub fn resolve_closure(
    registry: &PackageRegistry,
    roots: &[String],
) -> Result<Vec<ResolvedPackage>, ResolveError> {
    let mut selected: BTreeMap<String, PackageId> = BTreeMap::new();
    let mut order: Vec<PackageId> = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    for root in roots {
        visit(
            registry,
            root,
            &VersionReq::Any,
            &mut selected,
            &mut order,
            &mut stack,
        )?;
    }

    let mut resolved = Vec::with_capacity(order.len());
    for id in order {
        let entry = registry
            .versions_of(&id.name)
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| ResolveError::new(format!("注册表内部错误: 找不到 {}", id)))?;
        let mut dependencies = BTreeMap::new();
        for dep_name in entry.manifest.dependencies.keys() {
            if let Some(selected_dep) = selected.get(dep_name) {
                dependencies.insert(dep_name.clone(), selected_dep.clone());
            }
        }
        resolved.push(ResolvedPackage {
            id,
            dir: entry.dir.clone(),
            dependencies,
        });
    }
    Ok(resolved)
}

fn visit(
    registry: &PackageRegistry,
    name: &str,
    requirement: &VersionReq,
    selected: &mut BTreeMap<String, PackageId>,
    order: &mut Vec<PackageId>,
    stack: &mut Vec<String>,
) -> Result<PackageId, ResolveError> {
    if let Some(existing) = selected.get(name) {
        if requirement.matches(&existing.version) {
            return Ok(existing.clone());
        }
        return Err(ResolveError::new(format!(
            "版本冲突: {} 已选 {}，但被要求 {}",
            name, existing.version, requirement_text(requirement)
        )));
    }
    if stack.iter().any(|entry| entry == name) {
        let mut chain = stack.clone();
        chain.push(name.to_string());
        return Err(ResolveError::new(format!(
            "循环依赖: {}",
            chain.join(" -> ")
        )));
    }

    let entry = registry.resolve(name, requirement).ok_or_else(|| {
        let available: Vec<String> = registry
            .versions_of(name)
            .iter()
            .map(|entry| entry.id.version.to_string())
            .collect();
        if available.is_empty() {
            ResolveError::new(format!("缺失依赖: {name}"))
        } else {
            ResolveError::new(format!(
                "版本不匹配: {name} 需要 {}，可用 [{}]",
                requirement_text(requirement),
                available.join(", ")
            ))
        }
    })?;
    let id = entry.id.clone();
    let dependencies = entry
        .manifest
        .parsed_dependencies()
        .map_err(|error| ResolveError::new(format!("包 {} 依赖范围非法: {error}", entry.id)))?;

    stack.push(name.to_string());
    for (dep_name, dep_req) in dependencies {
        visit(registry, &dep_name, &dep_req, selected, order, stack)?;
    }
    stack.pop();

    selected.insert(name.to_string(), id.clone());
    order.push(id.clone());
    Ok(id)
}

fn requirement_text(requirement: &VersionReq) -> String {
    format!("{requirement:?}")
}
