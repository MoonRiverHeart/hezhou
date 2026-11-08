//! 包清单与版本/版本范围。
//!
//! `package.json` 字段：`name` / `version` / `display_name` / `description` /
//! `dependencies`（name -> version req）/ `assets` / `scripts` / `entry` / `files`（可选白名单）。
//!
//! 版本范围支持最小子集：精确 `1.2.3`、`~1.2.3`、`^1.2.3`、`*`、`1.*`、`1.2.*`。

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// 语义化版本（major.minor.patch；不接受 pre-release/build 元数据）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    /// 主版本。
    pub major: u64,
    /// 次版本。
    pub minor: u64,
    /// 修订号。
    pub patch: u64,
}

impl Version {
    /// 构造版本。
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self { major, minor, patch }
    }

    /// 解析 `major.minor.patch`。
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        let mut parts = trimmed.split('.');
        let major = parts.next().unwrap_or("");
        let minor = parts.next().ok_or_else(|| format!("版本号缺少 minor: {text}"))?;
        let patch = parts.next().ok_or_else(|| format!("版本号缺少 patch: {text}"))?;
        if parts.next().is_some() {
            return Err(format!("版本号格式非法: {text}"));
        }
        let parse_part = |part: &str| -> Result<u64, String> {
            if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
                return Err(format!("版本号段非数字: {part}（{text}）"));
            }
            part.parse::<u64>().map_err(|error| format!("版本号溢出: {error}"))
        };
        Ok(Self {
            major: parse_part(major)?,
            minor: parse_part(minor)?,
            patch: parse_part(patch)?,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 版本范围（最小子集）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionReq {
    /// 任意版本（`*`）。
    Any,
    /// 精确版本。
    Exact(Version),
    /// `~x.y.z`：>=x.y.z 且 <x.(y+1).0。
    Tilde(Version),
    /// `^x.y.z`：>=x.y.z 且 <(x+1).0.0（x=0 时 <0.(y+1).0）。
    Caret(Version),
    /// `x` / `x.*`：主版本匹配。
    MajorWild(u64),
    /// `x.y` / `x.y.*`：major.minor 匹配。
    MinorWild(u64, u64),
}

impl VersionReq {
    /// 解析版本范围。
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("版本范围为空".to_string());
        }
        if trimmed == "*" {
            return Ok(Self::Any);
        }
        let (kind, body) = if let Some(rest) = trimmed.strip_prefix('~') {
            ("tilde", rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix('^') {
            ("caret", rest.trim())
        } else {
            ("exact", trimmed)
        };
        if body.is_empty() {
            return Err(format!("版本范围非法: {text}"));
        }
        if body == "*" {
            return Ok(Self::Any);
        }
        let parts: Vec<&str> = body.split('.').collect();
        let has_wild = parts.iter().any(|part| *part == "*");
        if has_wild {
            if kind != "exact" {
                return Err(format!("~ / ^ 不支持通配: {text}"));
            }
            return match parts.as_slice() {
                ["*"] => Ok(Self::Any),
                [major, "*"] => {
                    let major = parse_number(major, text)?;
                    Ok(Self::MajorWild(major))
                }
                [major, minor, "*"] => {
                    let major = parse_number(major, text)?;
                    let minor = parse_number(minor, text)?;
                    Ok(Self::MinorWild(major, minor))
                }
                _ => Err(format!("版本范围通配位置非法: {text}")),
            };
        }
        match parts.as_slice() {
            [major] => Ok(Self::MajorWild(parse_number(major, text)?)),
            [major, minor] => Ok(Self::MinorWild(
                parse_number(major, text)?,
                parse_number(minor, text)?,
            )),
            [major, minor, patch] => {
                let version = Version::new(
                    parse_number(major, text)?,
                    parse_number(minor, text)?,
                    parse_number(patch, text)?,
                );
                Ok(match kind {
                    "tilde" => Self::Tilde(version),
                    "caret" => Self::Caret(version),
                    _ => Self::Exact(version),
                })
            }
            _ => Err(format!("版本范围格式非法: {text}")),
        }
    }

    /// 判断版本是否满足范围。
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(expected) => version == expected,
            Self::Tilde(base) => {
                version >= base
                    && (version.major, version.minor) == (base.major, base.minor)
            }
            Self::Caret(base) => {
                if version < base {
                    return false;
                }
                if base.major > 0 {
                    version.major == base.major
                } else if base.minor > 0 {
                    version.major == 0 && version.minor == base.minor
                } else {
                    version.major == 0 && version.minor == 0 && version.patch == base.patch
                }
            }
            Self::MajorWild(major) => version.major == *major,
            Self::MinorWild(major, minor) => version.major == *major && version.minor == *minor,
        }
    }
}

fn parse_number(text: &str, original: &str) -> Result<u64, String> {
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!("版本段非数字: {text}（{original}）"));
    }
    text.parse::<u64>().map_err(|error| format!("版本号溢出: {error}"))
}

/// 包唯一标识（name@version）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PackageId {
    /// 包名（`[a-z0-9][a-z0-9._-]{0,63}`）。
    pub name: String,
    /// 版本。
    pub version: Version,
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// 包名合法性（小写字母/数字开头，允许 `.`/`-`/`_`，≤64）。
pub fn is_valid_package_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    name.len() <= 64
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_'))
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

/// 包清单（`package.json`）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PackageManifest {
    /// 包名（唯一键）。
    pub name: String,
    /// 版本（`major.minor.patch`）。
    pub version: String,
    /// 显示名。
    pub display_name: String,
    /// 描述。
    pub description: String,
    /// 依赖：name -> version req（原始字符串）。
    pub dependencies: BTreeMap<String, String>,
    /// 资产路径（相对包根）。
    pub assets: Vec<String>,
    /// 脚本路径（相对包根）。
    pub scripts: Vec<String>,
    /// 入口脚本/类名（可选）。
    pub entry: Option<String>,
    /// 文件白名单（可选；为空复制整包）。
    pub files: Vec<String>,
}

impl PackageManifest {
    /// 从 JSON 文本解析（未知字段忽略）。
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str::<Self>(text).map_err(|error| format!("package.json 解析失败: {error}"))
    }

    /// 读取 `package.json` 文件。
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
        Self::from_json(&text)
    }

    /// 校验清单；成功返回 `PackageId`，失败返回全部错误。
    pub fn validate(&self) -> Result<PackageId, Vec<String>> {
        let mut errors = Vec::new();
        if !is_valid_package_name(&self.name) {
            errors.push(format!(
                "name 非法: {}（要求小写字母/数字开头，允许 . - _，≤64）",
                self.name
            ));
        }
        let version = match Version::parse(&self.version) {
            Ok(version) => Some(version),
            Err(error) => {
                errors.push(format!("version 非法: {error}"));
                None
            }
        };
        for (dep_name, dep_req) in &self.dependencies {
            if !is_valid_package_name(dep_name) {
                errors.push(format!("依赖 name 非法: {dep_name}"));
            }
            if let Err(error) = VersionReq::parse(dep_req) {
                errors.push(format!("依赖 {dep_name} 版本范围非法: {error}"));
            }
        }
        for (field, values) in [
            ("assets", &self.assets),
            ("scripts", &self.scripts),
            ("files", &self.files),
        ] {
            for value in values {
                if !is_safe_relative_path(value) {
                    errors.push(format!("{field} 路径非法（绝对路径/..）: {value}"));
                }
            }
        }
        if let Some(entry) = &self.entry {
            if !is_safe_relative_path(entry) {
                errors.push(format!("entry 路径非法: {entry}"));
            }
        }
        match version {
            Some(version) if errors.is_empty() => Ok(PackageId {
                name: self.name.clone(),
                version,
            }),
            Some(_) => Err(errors),
            None => Err(errors),
        }
    }

    /// 解析后的依赖范围（校验失败返回错误）。
    pub fn parsed_dependencies(&self) -> Result<BTreeMap<String, VersionReq>, String> {
        let mut parsed = BTreeMap::new();
        for (name, req) in &self.dependencies {
            parsed.insert(name.clone(), VersionReq::parse(req)?);
        }
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_matches_version_reqs() {
        assert!(VersionReq::parse("*").unwrap().matches(&Version::new(9, 9, 9)));
        assert!(VersionReq::parse("1.2.3").unwrap().matches(&Version::new(1, 2, 3)));
        assert!(!VersionReq::parse("1.2.3").unwrap().matches(&Version::new(1, 2, 4)));
        let tilde = VersionReq::parse("~1.2.3").unwrap();
        assert!(tilde.matches(&Version::new(1, 2, 9)));
        assert!(!tilde.matches(&Version::new(1, 3, 0)));
        let caret = VersionReq::parse("^1.2.3").unwrap();
        assert!(caret.matches(&Version::new(1, 9, 0)));
        assert!(!caret.matches(&Version::new(2, 0, 0)));
        let caret0 = VersionReq::parse("^0.2.3").unwrap();
        assert!(caret0.matches(&Version::new(0, 2, 9)));
        assert!(!caret0.matches(&Version::new(0, 3, 0)));
        assert!(VersionReq::parse("1.*").unwrap().matches(&Version::new(1, 7, 2)));
        assert!(VersionReq::parse("1.2.*").unwrap().matches(&Version::new(1, 2, 9)));
    }

    #[test]
    fn manifest_validation_reports_errors() {
        let manifest = PackageManifest::from_json(
            r#"{
                "name": "demo-pkg",
                "version": "0.1.0",
                "display_name": "Demo",
                "description": "demo",
                "dependencies": { "mathx": "^0.1.0" },
                "assets": ["assets/data.json"],
                "scripts": ["scripts/demo.cs"],
                "entry": "Demo.Class"
            }"#,
        )
        .unwrap();
        let id = manifest.validate().expect("应通过校验");
        assert_eq!(id.name, "demo-pkg");
        assert_eq!(id.version, Version::new(0, 1, 0));

        let bad = PackageManifest::from_json(
            r#"{ "name": "Bad Name", "version": "x", "dependencies": { "dep": "~bad" }, "assets": ["../escape"] }"#,
        )
        .unwrap();
        let errors = bad.validate().expect_err("应校验失败");
        let joined = errors.join(";");
        assert!(joined.contains("name"), "{joined}");
        assert!(joined.contains("version"), "{joined}");
        assert!(joined.contains("assets"), "{joined}");
    }
}

