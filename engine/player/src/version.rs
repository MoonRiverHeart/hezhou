//! 版本号解析与运行时兼容性校验。
//!
//! Player 产物与 `hezhou-player` 运行时共享同一套引擎 ABI，因此兼容策略是：
//! **major.minor 必须完全一致，patch 允许运行时版本更高**。这样游戏包可以
//! 在只修复 bug 的运行时补丁上继续运行，但不能跨 0.2/0.3 这类能力边界。

use std::fmt;

use serde::{Deserialize, Serialize};

/// 解析后的语义化版本（`major.minor.patch`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    /// 解析 `major.minor.patch`。不接受前缀 `v`、预发布后缀或空白。
    pub fn parse(input: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = input.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::new(input, "必须是 major.minor.patch 三段"));
        }
        let mut numbers = [0u32; 3];
        for (index, part) in parts.iter().enumerate() {
            if part.is_empty() {
                return Err(VersionError::new(input, "版本段不能为空"));
            }
            if !part.chars().all(|c| c.is_ascii_digit()) {
                return Err(VersionError::new(input, "版本段只能包含 ASCII 数字"));
            }
            numbers[index] = part
                .parse::<u32>()
                .map_err(|_| VersionError::new(input, "版本段超出 u32 范围"))?;
        }
        Ok(Self {
            major: numbers[0],
            minor: numbers[1],
            patch: numbers[2],
        })
    }

    /// 判断清单版本是否可被给定运行时加载（major.minor 相同）。
    pub fn is_compatible_with(&self, runtime: &SemVer) -> bool {
        self.major == runtime.major && self.minor == runtime.minor
    }

    /// 返回 `major.minor`，用于日志与错误提示。
    pub fn major_minor(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 版本解析错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionError {
    /// 原始输入（便于构建日志定位）。
    pub input: String,
    /// 失败原因（中文，直接进入日志）。
    pub reason: &'static str,
}

impl VersionError {
    fn new(input: &str, reason: &'static str) -> Self {
        Self {
            input: input.to_string(),
            reason,
        }
    }
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "版本号 `{}` 非法：{}", self.input, self.reason)
    }
}

impl std::error::Error for VersionError {}
