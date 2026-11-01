//! Player 构建配置与校验。
//!
//! 构建配置描述"要导出一个什么游戏"：启动场景、窗口、版本、目标平台、质量档。
//! 所有路径均为逻辑路径（相对包根目录），由构建器按目标平台转换。

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::paths::{
    is_valid_executable_stem, is_windows_reserved_name, normalize_logical_path, PathError,
};
use crate::platform::TargetPlatform;
use crate::version::SemVer;

/// 默认随包字体（相对 `assets/`）。缺失会导致 UI 文字空白，构建器默认强制检查。
pub const DEFAULT_FONT_ASSET: &str = "fonts/HarmonyOS_Sans_SC.ttf";

/// 窗口宽度下限。
pub const MIN_WINDOW_WIDTH: u32 = 320;
/// 窗口宽度上限。
pub const MAX_WINDOW_WIDTH: u32 = 7680;
/// 窗口高度下限。
pub const MIN_WINDOW_HEIGHT: u32 = 240;
/// 窗口高度上限。
pub const MAX_WINDOW_HEIGHT: u32 = 4320;

/// 质量档。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityTier {
    Low,
    Medium,
    High,
    Ultra,
}

impl QualityTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
        }
    }
}

impl Default for QualityTier {
    fn default() -> Self {
        Self::High
    }
}

/// 窗口模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    /// 普通窗口，可拖动/缩放。
    #[default]
    Windowed,
    /// 无边框全屏（独占桌面）。
    BorderlessFullscreen,
    /// 独占全屏（由渲染后端处理分辨率切换）。
    Fullscreen,
}

impl WindowMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::BorderlessFullscreen => "borderless_fullscreen",
            Self::Fullscreen => "fullscreen",
        }
    }
}

/// 窗口配置。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowConfig {
    /// 窗口标题（显示名，可含中文/空格）。
    pub title: String,
    /// 逻辑宽度（像素，缩放前）。
    pub width: u32,
    /// 逻辑高度（像素，缩放前）。
    pub height: u32,
    /// 窗口模式。
    #[serde(default)]
    pub mode: WindowMode,
}

impl WindowConfig {
    /// 以标题创建默认 1280x720 窗口配置。
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 1280,
            height: 720,
            mode: WindowMode::Windowed,
        }
    }

    /// 设置窗口尺寸。
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// 设置窗口模式。
    pub fn with_mode(mut self, mode: WindowMode) -> Self {
        self.mode = mode;
        self
    }
}

/// Player 构建配置。
///
/// 序列化为 JSON 后即"游戏导出设置"，由编辑器"导出游戏"面板生成、
/// 构建器消费；运行时不直接读取本结构（运行时读 `manifest.json`）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerBuildConfig {
    /// 产品显示名（窗口标题/包名/日志目录的来源）。
    pub product_name: String,
    /// 可执行文件名（不含平台后缀，仅 ASCII 字母数字、`-`、`_`）。
    pub executable_name: String,
    /// 启动场景，相对 `assets/` 的逻辑路径。
    pub start_scene: String,
    /// 游戏版本（`major.minor.patch`）。
    pub version: String,
    /// 目标平台。
    pub target: TargetPlatform,
    /// 质量档。
    pub quality: QualityTier,
    /// 窗口配置。
    pub window: WindowConfig,
    /// 打包内日志目录（运行时优先写用户数据目录，此为兜底）。
    #[serde(default = "default_log_dir")]
    pub log_dir: String,
    /// 必须随包分发的字体（相对 `assets/`）；`null` 表示不强制检查。
    #[serde(default = "default_required_font")]
    pub required_font: Option<String>,
}

fn default_log_dir() -> String {
    "logs".to_string()
}

fn default_required_font() -> Option<String> {
    Some(DEFAULT_FONT_ASSET.to_string())
}

impl PlayerBuildConfig {
    /// 创建一份最小可校验配置；未给出的字段使用合理默认值。
    pub fn new(
        product_name: impl Into<String>,
        version: impl Into<String>,
        start_scene: impl Into<String>,
        target: TargetPlatform,
    ) -> Self {
        let product_name = product_name.into();
        Self {
            executable_name: default_executable_stem(&product_name),
            product_name,
            start_scene: start_scene.into(),
            version: version.into(),
            target,
            quality: QualityTier::High,
            window: WindowConfig::new("Hezhou Game"),
            log_dir: default_log_dir(),
            required_font: default_required_font(),
        }
    }

    /// 设置可执行文件名。
    pub fn with_executable_name(mut self, name: impl Into<String>) -> Self {
        self.executable_name = name.into();
        self
    }

    /// 设置窗口配置。
    pub fn with_window(mut self, window: WindowConfig) -> Self {
        self.window = window;
        self
    }

    /// 设置质量档。
    pub fn with_quality(mut self, quality: QualityTier) -> Self {
        self.quality = quality;
        self
    }

    /// 设置目标平台。
    pub fn with_target(mut self, target: TargetPlatform) -> Self {
        self.target = target;
        self
    }

    /// 设置必检字体；`None` 关闭字体检查。
    pub fn with_required_font(mut self, font: Option<impl Into<String>>) -> Self {
        self.required_font = font.map(Into::into);
        self
    }

    /// 启动场景的规范化逻辑路径。
    pub fn normalized_start_scene(&self) -> Result<String, PathError> {
        normalize_logical_path(&self.start_scene)
    }

    /// 规范化后的可执行文件名（去空白）。
    pub fn executable_stem(&self) -> &str {
        self.executable_name.trim()
    }

    /// 校验配置；一次性返回所有问题，便于构建日志一次列全。
    pub fn validate(&self) -> Result<(), ConfigErrors> {
        let mut issues = Vec::new();

        validate_product_name(&self.product_name, &mut issues);

        let stem = self.executable_stem();
        if stem.is_empty() {
            issues.push(ConfigIssue::new("executable_name", "不能为空"));
        } else if !is_valid_executable_stem(stem) {
            issues.push(ConfigIssue::new(
                "executable_name",
                "只允许 1~64 个 ASCII 字母数字、`-`、`_`，且不能是 Windows 保留设备名",
            ));
        }

        if let Err(error) = self.normalized_start_scene() {
            issues.push(ConfigIssue::new(
                "start_scene",
                format!("启动场景路径非法：{error}"),
            ));
        }

        if let Err(error) = SemVer::parse(&self.version) {
            issues.push(ConfigIssue::new("version", error.to_string()));
        }

        validate_window(&self.window, &mut issues);

        if let Err(error) = normalize_logical_path(&self.log_dir) {
            issues.push(ConfigIssue::new(
                "log_dir",
                format!("日志目录路径非法：{error}"),
            ));
        }

        if let Some(font) = &self.required_font {
            if let Err(error) = normalize_logical_path(font) {
                issues.push(ConfigIssue::new(
                    "required_font",
                    format!("字体路径非法：{error}"),
                ));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(ConfigErrors { issues })
        }
    }
}

/// 校验产品名（中文/空格允许，但禁止路径分隔符与控制字符）。
pub(crate) fn validate_product_name(product_name: &str, issues: &mut Vec<ConfigIssue>) {
    let trimmed = product_name.trim();
    if trimmed.is_empty() {
        issues.push(ConfigIssue::new("product_name", "不能为空"));
        return;
    }
    if trimmed.chars().count() > 64 {
        issues.push(ConfigIssue::new(
            "product_name",
            "长度不能超过 64 个字符",
        ));
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        issues.push(ConfigIssue::new(
            "product_name",
            "不能包含路径分隔符、控制字符或 Windows 非法字符",
        ));
    }
    if is_windows_reserved_name(trimmed) {
        issues.push(ConfigIssue::new(
            "product_name",
            "不能是 Windows 保留设备名",
        ));
    }
}

/// 校验窗口配置。
pub(crate) fn validate_window(window: &WindowConfig, issues: &mut Vec<ConfigIssue>) {
    let title = window.title.trim();
    if title.is_empty() {
        issues.push(ConfigIssue::new("window.title", "不能为空"));
    } else {
        if title.chars().count() > 128 {
            issues.push(ConfigIssue::new(
                "window.title",
                "长度不能超过 128 个字符",
            ));
        }
        if title.chars().any(char::is_control) {
            issues.push(ConfigIssue::new("window.title", "不能包含控制字符"));
        }
    }
    if !(MIN_WINDOW_WIDTH..=MAX_WINDOW_WIDTH).contains(&window.width) {
        issues.push(ConfigIssue::new(
            "window.width",
            format!("必须在 {MIN_WINDOW_WIDTH}..={MAX_WINDOW_WIDTH} 之间"),
        ));
    }
    if !(MIN_WINDOW_HEIGHT..=MAX_WINDOW_HEIGHT).contains(&window.height) {
        issues.push(ConfigIssue::new(
            "window.height",
            format!("必须在 {MIN_WINDOW_HEIGHT}..={MAX_WINDOW_HEIGHT} 之间"),
        ));
    }
}

/// 由产品名派生合法的可执行文件名（不含平台后缀）。
///
/// 非 ASCII 字符与非法字符折叠为 `-`，结果为空时回退 `hezhou-game`；
/// 若与 Windows 保留名冲突则追加 `-player`。
pub fn default_executable_stem(product_name: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;
    for ch in product_name.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else if matches!(ch, '-' | '_') {
            ch
        } else {
            '-'
        };
        if mapped == '-' {
            if !out.is_empty() && !last_was_dash {
                out.push('-');
                last_was_dash = true;
            }
        } else {
            out.push(mapped);
            last_was_dash = false;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        return "hezhou-game".to_string();
    }
    if is_windows_reserved_name(&trimmed) {
        format!("{trimmed}-player")
    } else {
        trimmed
    }
}

/// 单条配置问题。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigIssue {
    /// 配置字段名。
    pub field: &'static str,
    /// 中文问题描述。
    pub message: String,
}

impl ConfigIssue {
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// 一组配置问题。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigErrors {
    pub issues: Vec<ConfigIssue>,
}

impl ConfigErrors {
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// 单行摘要，用于日志。
    pub fn summary(&self) -> String {
        self.issues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "配置校验失败（{} 项）：{}", self.issues.len(), self.summary())
    }
}

impl std::error::Error for ConfigErrors {}
