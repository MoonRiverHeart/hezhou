//! 跨平台逻辑路径。
//!
//! 工程内所有资产都以 **工程相对逻辑路径** 表示，例如：
//!
//! ```text
//! assets/models/hero.gltf
//! assets/textures/hero_albedo.png
//! assets/audio/hit.wav
//! ```
//!
//! 规则（Windows / Linux 行为一致）：
//!
//! 1. 分隔符统一为 `/`；输入里的 `\` 会被转换为 `/`。
//! 2. 大小写不敏感比较：逻辑路径的相等/排序/Hash 都基于 Unicode 小写副本，
//!    避免 Windows（不区分大小写）与 Linux（区分大小写）出现“同一资产两个 ID”。
//!    原始大小写会保留在 [`LogicalPath::as_str`] 中用于显示。
//! 3. `.` 段被忽略，`..` 段只在工程内回退；越出工程根的路径直接报错。
//! 4. 拒绝绝对路径（`/`、`C:\`、UNC）和 Windows 非法字符 `< > : " | ? * #`。
//! 5. 路径相对 **工程根目录**，不是相对 `assets/`。这样 `.meta`、cook 输出和
//!    Player 包都使用同一套路径，工程整体移动目录不需要改任何引用。
//!
//! 子资产（glTF 导入出的 Mesh/Material/Skeleton/Animation）用
//! [`AssetLocator`] 表达：`assets/models/hero.gltf#animation:0`。

use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{AssetError, AssetResult};

/// Windows 上不允许出现在文件名中的字符（逻辑路径统一遵守，保证可跨平台）。
const INVALID_PATH_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*', '#'];

/// 工程相对逻辑路径（大小写不敏感比较）。
#[derive(Clone)]
pub struct LogicalPath {
    normalized: String,
    key: String,
}

impl LogicalPath {
    /// 解析并规范化逻辑路径。
    pub fn new(input: impl AsRef<str>) -> AssetResult<Self> {
        let raw = input.as_ref();
        if raw.is_empty() {
            return Err(AssetError::InvalidPath("逻辑路径为空".to_string()));
        }
        if raw.chars().any(|c| c.is_control()) {
            return Err(AssetError::InvalidPath(format!(
                "逻辑路径包含控制字符: {raw:?}"
            )));
        }

        let replaced = raw.replace('\\', "/");
        if replaced.starts_with('/') {
            return Err(AssetError::InvalidPath(format!(
                "逻辑路径必须是工程相对路径，不能是绝对路径: {raw:?}"
            )));
        }
        // Windows 盘符（规范化后形如 `C:/...`）
        let bytes = replaced.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' {
            return Err(AssetError::InvalidPath(format!(
                "逻辑路径不能包含盘符: {raw:?}"
            )));
        }

        let mut segments: Vec<&str> = Vec::new();
        for segment in replaced.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err(AssetError::InvalidPath(format!(
                            "逻辑路径越出工程根目录: {raw:?}"
                        )));
                    }
                }
                _ => {
                    if let Some(bad) = segment.chars().find(|c| INVALID_PATH_CHARS.contains(c)) {
                        return Err(AssetError::InvalidPath(format!(
                            "逻辑路径包含非法字符 {bad:?}: {raw:?}"
                        )));
                    }
                    if segment.ends_with(' ') || segment.ends_with('.') {
                        return Err(AssetError::InvalidPath(format!(
                            "逻辑路径段不能以空格或点结尾: {raw:?}"
                        )));
                    }
                    segments.push(segment);
                }
            }
        }

        if segments.is_empty() {
            return Err(AssetError::InvalidPath(format!(
                "逻辑路径没有有效段: {raw:?}"
            )));
        }

        let normalized = segments.join("/");
        let key = normalized.to_lowercase();
        Ok(Self { normalized, key })
    }

    /// 原始大小写的规范化路径（`/` 分隔）。
    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    /// 大小写不敏感的规范化键（小写）。
    pub fn key(&self) -> &str {
        &self.key
    }

    /// 文件名（含扩展名）。
    pub fn file_name(&self) -> Option<&str> {
        self.normalized.rsplit('/').next()
    }

    /// 扩展名（小写、不含点）。
    pub fn extension(&self) -> Option<String> {
        let name = self.file_name()?;
        let idx = name.rfind('.')?;
        let ext = &name[idx + 1..];
        if ext.is_empty() {
            None
        } else {
            Some(ext.to_ascii_lowercase())
        }
    }

    /// 父目录逻辑路径；工程根下的文件返回 `None`。
    pub fn parent(&self) -> Option<LogicalPath> {
        let idx = self.normalized.rfind('/')?;
        let parent = &self.normalized[..idx];
        if parent.is_empty() {
            return None;
        }
        LogicalPath::new(parent).ok()
    }

    /// 拼接子路径。
    pub fn join(&self, relative: impl AsRef<str>) -> AssetResult<LogicalPath> {
        let rel = relative.as_ref();
        if rel.is_empty() {
            return Ok(self.clone());
        }
        LogicalPath::new(format!("{}/{}", self.normalized, rel))
    }

    /// 替换扩展名（`ext` 不含点）。
    pub fn with_extension(&self, ext: &str) -> AssetResult<LogicalPath> {
        let parent = self.parent();
        let stem = self
            .file_name()
            .map(|name| match name.rfind('.') {
                Some(idx) => &name[..idx],
                None => name,
            })
            .unwrap_or("asset");
        let joined = format!("{stem}.{ext}");
        match parent {
            Some(parent) => parent.join(joined),
            None => LogicalPath::new(joined),
        }
    }

    /// 是否位于 `prefix` 目录之下（大小写不敏感，逐段比较）。
    pub fn is_under(&self, prefix: &LogicalPath) -> bool {
        if self.key == prefix.key {
            return true;
        }
        let mut prefix_key = prefix.key.clone();
        prefix_key.push('/');
        self.key.starts_with(&prefix_key)
    }

    /// 去掉 `prefix` 前缀，返回相对路径。
    pub fn strip_prefix(&self, prefix: &LogicalPath) -> Option<LogicalPath> {
        if !self.is_under(prefix) {
            return None;
        }
        if self.key == prefix.key {
            return None;
        }
        let rest = &self.normalized[prefix.normalized.len() + 1..];
        LogicalPath::new(rest).ok()
    }

    /// 转成文件系统路径（`project_root` + 逻辑路径）。
    pub fn to_fs_path(&self, project_root: &Path) -> PathBuf {
        let mut path = project_root.to_path_buf();
        for segment in self.normalized.split('/') {
            path.push(segment);
        }
        path
    }

    /// 把文件系统路径转换为工程相对逻辑路径。
    ///
    /// - `fs_path` 可以是绝对路径，也可以是相对 `project_root` 的路径。
    /// - 路径比较大小写不敏感（兼容 Windows 盘符/目录大小写差异）。
    pub fn from_fs_path(project_root: &Path, fs_path: &Path) -> AssetResult<Self> {
        let root = absolute_lexical(project_root)?;
        let full = if fs_path.is_absolute() {
            absolute_lexical(fs_path)?
        } else {
            absolute_lexical(&root.join(fs_path))?
        };

        let root_components: Vec<String> = component_strings(&root)?;
        let full_components: Vec<String> = component_strings(&full)?;
        if full_components.len() < root_components.len() {
            return Err(AssetError::InvalidPath(format!(
                "路径 {} 不在工程根 {} 之下",
                full.display(),
                root.display()
            )));
        }
        for (root_part, full_part) in root_components.iter().zip(full_components.iter()) {
            if root_part.to_lowercase() != full_part.to_lowercase() {
                return Err(AssetError::InvalidPath(format!(
                    "路径 {} 不在工程根 {} 之下",
                    full.display(),
                    root.display()
                )));
            }
        }
        let relative: Vec<&str> = full_components[root_components.len()..]
            .iter()
            .map(String::as_str)
            .collect();
        if relative.is_empty() {
            return Err(AssetError::InvalidPath(format!(
                "工程根目录本身不能作为资产路径: {}",
                full.display()
            )));
        }
        LogicalPath::new(relative.join("/"))
    }
}

impl PartialEq for LogicalPath {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for LogicalPath {}

impl PartialOrd for LogicalPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogicalPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl std::hash::Hash for LogicalPath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl fmt::Display for LogicalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.normalized)
    }
}

impl fmt::Debug for LogicalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LogicalPath({:?})", self.normalized)
    }
}

impl Serialize for LogicalPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.normalized)
    }
}

impl<'de> Deserialize<'de> for LogicalPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        LogicalPath::new(raw).map_err(D::Error::custom)
    }
}

impl std::str::FromStr for LogicalPath {
    type Err = AssetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// 资产定位符：工程相对逻辑路径 + 可选的子资产键。
///
/// 子资产键格式固定为 `<kind>:<index>`，例如 `mesh:0`、`animation:2`，
/// 由 glTF 导入器在 [`crate::database::AssetDatabase::import_gltf`] 中生成。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetLocator {
    path: LogicalPath,
    sub_key: Option<String>,
}

impl AssetLocator {
    /// 文件级资产定位符。
    pub fn file(path: LogicalPath) -> Self {
        Self {
            path,
            sub_key: None,
        }
    }

    /// 文件级资产定位符（从字符串解析）。
    pub fn file_str(path: impl AsRef<str>) -> AssetResult<Self> {
        Ok(Self::file(LogicalPath::new(path)?))
    }

    /// 子资产定位符。
    pub fn sub(path: LogicalPath, sub_key: impl AsRef<str>) -> AssetResult<Self> {
        let sub_key = sub_key.as_ref();
        validate_sub_key(sub_key)?;
        Ok(Self {
            path,
            sub_key: Some(sub_key.to_string()),
        })
    }

    /// 源文件逻辑路径。
    pub fn path(&self) -> &LogicalPath {
        &self.path
    }

    /// 子资产键。
    pub fn sub_key(&self) -> Option<&str> {
        self.sub_key.as_deref()
    }

    /// 是否指向子资产。
    pub fn is_sub_asset(&self) -> bool {
        self.sub_key.is_some()
    }

    /// 去掉子资产键后的文件定位符。
    pub fn file_locator(&self) -> AssetLocator {
        Self::file(self.path.clone())
    }
}

impl fmt::Display for AssetLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.sub_key {
            Some(sub) => write!(f, "{}#{sub}", self.path),
            None => write!(f, "{}", self.path),
        }
    }
}

impl Serialize for AssetLocator {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AssetLocator {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(D::Error::custom)
    }
}

impl AssetLocator {
    /// 解析 `path` 或 `path#sub_key`。
    pub fn parse(input: &str) -> AssetResult<Self> {
        match input.split_once('#') {
            Some((path, sub_key)) => {
                let path = LogicalPath::new(path)?;
                Self::sub(path, sub_key)
            }
            None => Ok(Self::file(LogicalPath::new(input)?)),
        }
    }
}

/// 子资产键只允许小写字母、数字、`_` 和 `:`，确保在文件名/URL 中安全。
fn validate_sub_key(sub_key: &str) -> AssetResult<()> {
    if sub_key.is_empty() {
        return Err(AssetError::InvalidPath("子资产键为空".to_string()));
    }
    if !sub_key
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == ':')
    {
        return Err(AssetError::InvalidPath(format!(
            "非法子资产键: {sub_key:?}"
        )));
    }
    Ok(())
}

/// 词法绝对化（不访问文件系统，`std::path::absolute` 会处理 `.`/`..`）。
fn absolute_lexical(path: &Path) -> AssetResult<PathBuf> {
    std::path::absolute(path).map_err(AssetError::Io)
}

/// 把路径拆成字符串分量；遇到非 UTF-8 分量报错。
fn component_strings(path: &Path) -> AssetResult<Vec<String>> {
    let mut out = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                let text = prefix.as_os_str().to_str().ok_or_else(|| {
                    AssetError::InvalidPath(format!("路径前缀不是 UTF-8: {}", path.display()))
                })?;
                out.push(text.to_string());
            }
            Component::RootDir => {
                // 根目录不参与大小写比较，单独保留为分隔语义
                out.push(String::from("/"));
            }
            Component::CurDir => {}
            Component::ParentDir => {
                // `absolute` 已词法规范化，理论上不会出现；保守回退。
                out.pop();
            }
            Component::Normal(part) => {
                let text = part.to_str().ok_or_else(|| {
                    AssetError::InvalidPath(format!("路径分量不是 UTF-8: {}", path.display()))
                })?;
                out.push(text.to_string());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators_and_case_are_normalized() {
        let win = LogicalPath::new("assets\\Models\\Hero.gltf").unwrap();
        let unix = LogicalPath::new("assets/models/hero.gltf").unwrap();
        assert_eq!(win, unix);
        assert_eq!(win.key(), "assets/models/hero.gltf");
        // 原始大小写保留用于显示
        assert_eq!(win.as_str(), "assets/Models/Hero.gltf");
    }

    #[test]
    fn redundant_segments_are_collapsed() {
        let p = LogicalPath::new("./assets//models/../models/hero.gltf/").unwrap();
        assert_eq!(p.as_str(), "assets/models/hero.gltf");
    }

    #[test]
    fn absolute_and_escaping_paths_rejected() {
        assert!(LogicalPath::new("/etc/passwd").is_err());
        assert!(LogicalPath::new("C:\\Windows\\system.ini").is_err());
        assert!(LogicalPath::new("c:/windows").is_err());
        assert!(LogicalPath::new("assets/../../secret.txt").is_err());
        assert!(LogicalPath::new("assets/hero?.gltf").is_err());
        assert!(LogicalPath::new("assets/hero.gltf#mesh:0").is_err());
        assert!(LogicalPath::new("").is_err());
    }

    #[test]
    fn parent_extension_and_join() {
        let p = LogicalPath::new("assets/models/hero.GLTF").unwrap();
        assert_eq!(p.parent().unwrap().as_str(), "assets/models");
        assert_eq!(p.extension().as_deref(), Some("gltf"));
        assert_eq!(
            p.with_extension("meta").unwrap().as_str(),
            "assets/models/hero.meta"
        );
        assert_eq!(
            p.join("sub/bone.json").unwrap().as_str(),
            "assets/models/hero.GLTF/sub/bone.json"
        );
        // 工程根级文件没有父路径
        assert!(LogicalPath::new("assets").unwrap().parent().is_none());
    }

    #[test]
    fn is_under_and_strip_prefix() {
        let root = LogicalPath::new("assets").unwrap();
        let deep = LogicalPath::new("Assets/Models/Hero.gltf").unwrap();
        assert!(deep.is_under(&root));
        assert_eq!(
            deep.strip_prefix(&root).unwrap().as_str(),
            "Models/Hero.gltf"
        );
        assert!(!root.is_under(&deep));
        // 前缀按段比较，不能只比较字符串前缀
        assert!(!LogicalPath::new("assets2/x.png").unwrap().is_under(&root));
    }

    #[test]
    fn locator_roundtrip_string() {
        let loc = AssetLocator::parse("assets/models/hero.gltf#animation:2").unwrap();
        assert_eq!(loc.path().as_str(), "assets/models/hero.gltf");
        assert_eq!(loc.sub_key(), Some("animation:2"));
        assert_eq!(loc.to_string(), "assets/models/hero.gltf#animation:2");
        let json = serde_json::to_string(&loc).unwrap();
        assert_eq!(json, "\"assets/models/hero.gltf#animation:2\"");
        let back: AssetLocator = serde_json::from_str(&json).unwrap();
        assert_eq!(back, loc);

        let file_loc = AssetLocator::file_str("Assets/Hero.gltf").unwrap();
        assert!(!file_loc.is_sub_asset());
        assert_eq!(file_loc.to_string(), "Assets/Hero.gltf");
        assert_eq!(file_loc.file_locator(), file_loc);
    }

    #[test]
    fn invalid_sub_key_rejected() {
        let path = LogicalPath::new("assets/hero.gltf").unwrap();
        assert!(AssetLocator::sub(path.clone(), "Mesh:0").is_err());
        assert!(AssetLocator::sub(path, "mesh 0").is_err());
    }

    #[test]
    fn fs_path_roundtrip_with_mixed_separators() {
        let temp = crate::test_support::tempdir();
        let root = temp.path();
        let logical = LogicalPath::new("assets/models/hero.gltf").unwrap();
        let fs = logical.to_fs_path(root);
        assert_eq!(
            LogicalPath::from_fs_path(root, &fs).unwrap(),
            logical,
            "绝对路径往返"
        );
        // 相对工程根路径
        assert_eq!(
            LogicalPath::from_fs_path(root, Path::new("assets\\models\\hero.gltf")).unwrap(),
            logical
        );
        // 大小写不同（模拟 Windows 盘符/目录大小写差异）
        let upper = root.join("Assets").join("Models").join("Hero.gltf");
        assert_eq!(LogicalPath::from_fs_path(root, &upper).unwrap(), logical);
        // 工程外的路径
        assert!(LogicalPath::from_fs_path(root, Path::new("../outside.txt")).is_err());
        // 工程根本身
        assert!(LogicalPath::from_fs_path(root, root).is_err());
    }

    #[test]
    fn logical_path_serializes_human_readable() {
        let p = LogicalPath::new("assets/Models/Hero.gltf").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"assets/Models/Hero.gltf\"");
        let back: LogicalPath = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
