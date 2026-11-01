//! 跨平台逻辑路径规则。
//!
//! Player 内部（清单、构建清单、运行时查找）一律使用**逻辑路径**：
//! - 分隔符固定为 `/`；
//! - 相对路径，禁止 `..` 与盘符/根路径；
//! - 禁止 Windows 非法字符与保留设备名；
//! - 写入磁盘时再按目标平台转换成本机分隔符。
//!
//! 这样同一份资产清单在 Windows/Linux 上含义一致，也避免"大小写不同、
//! 文件名相同"的资产在 Windows 上互相覆盖。

use std::fmt;
use std::path::PathBuf;

use crate::platform::TargetPlatform;

/// 逻辑路径分隔符，始终为 `/`。
pub const LOGICAL_SEPARATOR: char = '/';

/// 逻辑路径总长度上限（字符数）。
pub const MAX_LOGICAL_PATH_LEN: usize = 512;
/// 单个路径段长度上限（字符数）。
pub const MAX_SEGMENT_LEN: usize = 255;

/// 逻辑路径错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathError {
    /// 空路径。
    Empty,
    /// 绝对路径（以 `/` 开头）。
    Absolute,
    /// 包含 `..` 路径段。
    ParentTraversal,
    /// 包含空段（如 `a//b`）。
    EmptySegment,
    /// 包含 Windows 非法字符（`: < > " | ? *` 或控制字符）。
    InvalidChar { ch: char },
    /// 路径段以 `.` 或空格结尾（Windows 会静默截断）。
    InvalidSegmentSuffix { segment: String },
    /// Windows 保留设备名（CON/PRN/AUX/NUL/COM1..9/LPT1..9，含带扩展名形式）。
    ReservedName { name: String },
    /// 超过长度上限。
    TooLong { len: usize, max: usize },
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "逻辑路径为空"),
            Self::Absolute => write!(f, "不允许绝对路径（不能以 `/` 开头）"),
            Self::ParentTraversal => write!(f, "不允许 `..` 路径段（资产不能逃逸包目录）"),
            Self::EmptySegment => write!(f, "路径包含空段"),
            Self::InvalidChar { ch } => write!(f, "路径包含跨平台非法字符 `{ch}`"),
            Self::InvalidSegmentSuffix { segment } => write!(
                f,
                "路径段 `{segment}` 不能以 `.` 或空格结尾（Windows 不兼容）"
            ),
            Self::ReservedName { name } => {
                write!(f, "路径段 `{name}` 是 Windows 保留设备名")
            }
            Self::TooLong { len, max } => write!(f, "路径长度 {len} 超过上限 {max}"),
        }
    }
}

impl std::error::Error for PathError {}

/// 把任意输入规范化为逻辑路径。
///
/// 规则：反斜杠转换为 `/`、丢弃 `.` 段、拒绝空段/`..`/盘符/非法字符/保留名。
pub fn normalize_logical_path(input: &str) -> Result<String, PathError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(PathError::Empty);
    }
    let replaced = trimmed.replace('\\', "/");
    if replaced.starts_with('/') {
        return Err(PathError::Absolute);
    }
    // 允许末尾多余分隔符（如 `assets/`），但中间空段视为错误。
    let without_trailing = replaced.trim_end_matches('/');
    if without_trailing.is_empty() {
        return Err(PathError::Empty);
    }

    let mut segments: Vec<&str> = Vec::new();
    for raw in without_trailing.split('/') {
        if raw.is_empty() {
            return Err(PathError::EmptySegment);
        }
        if raw == "." {
            continue;
        }
        if raw == ".." {
            return Err(PathError::ParentTraversal);
        }
        if raw.chars().count() > MAX_SEGMENT_LEN {
            return Err(PathError::TooLong {
                len: raw.chars().count(),
                max: MAX_SEGMENT_LEN,
            });
        }
        if raw.ends_with('.') || raw.ends_with(' ') {
            return Err(PathError::InvalidSegmentSuffix {
                segment: raw.to_string(),
            });
        }
        for ch in raw.chars() {
            if ch.is_control() || matches!(ch, ':' | '<' | '>' | '"' | '|' | '?' | '*') {
                return Err(PathError::InvalidChar { ch });
            }
        }
        if is_windows_reserved_name(raw) {
            return Err(PathError::ReservedName {
                name: raw.to_string(),
            });
        }
        segments.push(raw);
    }

    if segments.is_empty() {
        return Err(PathError::Empty);
    }
    let normalized = segments.join("/");
    if normalized.chars().count() > MAX_LOGICAL_PATH_LEN {
        return Err(PathError::TooLong {
            len: normalized.chars().count(),
            max: MAX_LOGICAL_PATH_LEN,
        });
    }
    Ok(normalized)
}

/// 拼接逻辑路径并重新规范化。
pub fn join_logical(base: &str, relative: &str) -> Result<String, PathError> {
    let base = normalize_logical_path(base)?;
    let relative = normalize_logical_path(relative)?;
    normalize_logical_path(&format!("{base}/{relative}"))
}

/// 大小写不敏感的比较键，用于检测 Windows 上会互相覆盖的资产。
///
/// 注意：当前只做 Unicode `to_lowercase()`，未做 NFC 规范化；含组合字符的
/// 资产名在后续 asset cooker 中需要额外规范化（见 README TODO）。
pub fn case_collision_key(logical: &str) -> String {
    logical.to_lowercase()
}

/// 转换为目标平台的本机路径分隔符。
pub fn to_native_path(logical: &str, platform: TargetPlatform) -> PathBuf {
    match platform {
        TargetPlatform::Windows => PathBuf::from(logical.replace('/', "\\")),
        TargetPlatform::Linux => PathBuf::from(logical),
    }
}

/// 转换为目标平台的本机路径字符串。
pub fn to_native_string(logical: &str, platform: TargetPlatform) -> String {
    match platform {
        TargetPlatform::Windows => logical.replace('/', "\\"),
        TargetPlatform::Linux => logical.to_string(),
    }
}

/// 取逻辑路径最后一段（文件名）。
pub fn file_name(logical: &str) -> Option<&str> {
    logical.rsplit('/').next().filter(|name| !name.is_empty())
}

/// 取小写扩展名；以 `.` 开头的隐藏文件（如 `.gitignore`）视为无扩展名。
pub fn extension(logical: &str) -> Option<String> {
    let name = file_name(logical)?;
    if name.starts_with('.') {
        return None;
    }
    name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase())
}

/// 判断单个路径段是否为 Windows 保留设备名（含带扩展名形式，如 `CON.txt`）。
pub fn is_windows_reserved_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or(name);
    let upper = base.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6"
            | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6"
            | "LPT7" | "LPT8" | "LPT9"
    )
}

/// 可执行文件名（不含后缀）是否合法：仅 ASCII 字母数字、`-`、`_`。
pub fn is_valid_executable_stem(stem: &str) -> bool {
    !stem.is_empty()
        && stem.chars().count() <= 64
        && stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !is_windows_reserved_name(stem)
}
