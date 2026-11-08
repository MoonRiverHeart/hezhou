//! 项目级 Build Settings（0.3 task-41）。
//!
//! 文件：`<project_root>/build.settings.json`（JSON，`serde(default)` 兼容旧/缺字段）。
//! 职责：
//! - 加载/推断默认值（无文件时从工程推断 product/start_scene）；
//! - 校验（名称/尺寸/quality/start_scene 存在/targets/version/log_dir）；
//! - 供 Player 导出（dry-run/build）优先使用；
//! - FFI：`build_settings_snapshot_json` / `build_settings_save_json` / `build_settings_validate_json`。
//!
//! 失败安全：读写/解析/校验失败只返回错误码并写日志，不 panic。

use std::ffi::c_char;
use std::path::{Path, PathBuf};

use hezhou_dfx::*;
use serde::{Deserialize, Serialize};

/// 设置文件名（位于工程根目录）。
pub const SETTINGS_FILE_NAME: &str = "build.settings.json";
/// 默认随包字体（相对 `assets/`）。
pub const DEFAULT_FONT_ASSET: &str = "fonts/HarmonyOS_Sans_SC.ttf";
/// 支持的目标平台。
pub const SUPPORTED_TARGETS: [&str; 2] = ["windows", "linux"];
/// 支持的质量档。
pub const SUPPORTED_QUALITIES: [&str; 4] = ["low", "medium", "high", "ultra"];
/// 支持的窗口模式。
pub const SUPPORTED_WINDOW_MODES: [&str; 3] = ["windowed", "borderless_fullscreen", "fullscreen"];

/// 窗口设置。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSettings {
    pub width: u32,
    pub height: u32,
    pub mode: String,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            mode: "windowed".to_string(),
        }
    }
}

/// 项目级构建设置（`build.settings.json`）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildSettings {
    pub product_name: String,
    pub executable_name: String,
    /// 启动场景（相对 `assets/`）。
    pub start_scene: String,
    pub quality: String,
    pub window: WindowSettings,
    /// 目标平台列表（`windows`/`linux`）。
    pub targets: Vec<String>,
    pub version: String,
    pub log_dir: String,
    /// 必检字体（相对 `assets/`）；`null`=不检查。
    pub required_font: Option<String>,
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            product_name: "HezhouGame".to_string(),
            executable_name: "hezhougame".to_string(),
            start_scene: String::new(),
            quality: "high".to_string(),
            window: WindowSettings::default(),
            targets: SUPPORTED_TARGETS.iter().map(|v| v.to_string()).collect(),
            version: "0.1.0".to_string(),
            log_dir: "logs".to_string(),
            required_font: Some(DEFAULT_FONT_ASSET.to_string()),
        }
    }
}

impl BuildSettings {
    /// 可执行文件名清洗：小写、仅保留 ASCII 字母数字/`-`/`_`。
    pub fn sanitized_executable_name(&self) -> String {
        let name: String = self
            .executable_name
            .trim()
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        if name.is_empty() {
            "hezhougame".to_string()
        } else {
            name
        }
    }

    /// 校验错误列表（空=通过）。
    pub fn validate(&self, project_root: &Path) -> Vec<String> {
        let mut errors = Vec::new();
        let product = self.product_name.trim();
        if product.is_empty() {
            errors.push("product_name 不能为空".to_string());
        } else if product.chars().count() > 64 {
            errors.push("product_name 不能超过 64 个字符".to_string());
        }

        let executable = self.executable_name.trim();
        if executable.is_empty() {
            errors.push("executable_name 不能为空".to_string());
        } else if executable.len() > 64
            || !executable
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            errors.push(
                "executable_name 只允许 1~64 个 ASCII 字母数字、`-`、`_`".to_string(),
            );
        }

        if !is_semver_like(&self.version) {
            errors.push(format!(
                "version 必须是 major.minor.patch 数字格式（当前: {}）",
                self.version
            ));
        }

        let quality = self.quality.trim().to_ascii_lowercase();
        if !SUPPORTED_QUALITIES.contains(&quality.as_str()) {
            errors.push(format!(
                "quality 必须是 {:?} 之一（当前: {}）",
                SUPPORTED_QUALITIES, self.quality
            ));
        }

        if !(320..=7680).contains(&self.window.width) {
            errors.push(format!(
                "window.width 必须在 320..=7680（当前: {}）",
                self.window.width
            ));
        }
        if !(240..=4320).contains(&self.window.height) {
            errors.push(format!(
                "window.height 必须在 240..=4320（当前: {}）",
                self.window.height
            ));
        }
        let mode = self.window.mode.trim().to_ascii_lowercase();
        if !SUPPORTED_WINDOW_MODES.contains(&mode.as_str()) {
            errors.push(format!(
                "window.mode 必须是 {:?} 之一（当前: {}）",
                SUPPORTED_WINDOW_MODES, self.window.mode
            ));
        }

        if self.targets.is_empty() {
            errors.push("targets 不能为空（windows/linux）".to_string());
        }
        for target in &self.targets {
            if !SUPPORTED_TARGETS.contains(&target.trim().to_ascii_lowercase().as_str()) {
                errors.push(format!("targets 含不支持的平台: {target}（仅 windows/linux）"));
            }
        }

        let start_scene = self.start_scene.trim();
        if start_scene.is_empty() {
            errors.push("start_scene 不能为空（相对 assets/ 的路径）".to_string());
        } else if !is_safe_relative_path(start_scene) {
            errors.push(format!("start_scene 必须是安全的相对路径: {start_scene}"));
        } else if !project_root.join("assets").join(start_scene).is_file() {
            errors.push(format!(
                "start_scene 不存在: assets/{start_scene}（工程根: {}）",
                project_root.display()
            ));
        }

        let log_dir = self.log_dir.trim();
        if log_dir.is_empty() {
            errors.push("log_dir 不能为空".to_string());
        } else if !is_safe_relative_path(log_dir) {
            errors.push(format!("log_dir 必须是安全的相对路径: {log_dir}"));
        }

        if let Some(font) = self.required_font.as_deref() {
            let font = font.trim();
            if !font.is_empty() && !is_safe_relative_path(font) {
                errors.push(format!("required_font 必须是安全的相对路径: {font}"));
            }
        }

        errors
    }

    /// 序列化为格式化 JSON。
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 从 JSON 解析（缺字段取默认值）。
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

fn is_semver_like(version: &str) -> bool {
    SemVerLite::parse(version).is_some()
}

/// 轻量语义化版本检查（不依赖 player crate 的完整 SemVer，允许 `1.2.3`/`1.2.3-beta`）。
struct SemVerLite;

impl SemVerLite {
    fn parse(version: &str) -> Option<()> {
        let version = version.trim();
        let core = version.split(['-', '+']).next().unwrap_or(version);
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        for part in parts {
            if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
                return None;
            }
            part.parse::<u64>().ok()?;
        }
        Some(())
    }
}

/// 安全相对路径：非空、无绝对前缀、无 `..`、无反斜杠。
fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') || normalized.contains("..") {
        return false;
    }
    !normalized.split('/').any(|segment| segment.is_empty())
}

/// 设置文件路径。
pub fn settings_path(project_root: &Path) -> PathBuf {
    project_root.join(SETTINGS_FILE_NAME)
}

/// 无设置文件时从工程推断默认值。
///
/// - product_name：优先读 `project.json` 的 `name`，否则 `HezhouGame`；
/// - start_scene：优先第一个 `.scene`/`.hzscene` 资产，否则第一个文件资产（并告警），
///   都没有时用 `scenes/main.scene.json`（导出 dry-run 会生成合成条目）。
pub fn infer_defaults(project_root: &Path) -> BuildSettings {
    let mut settings = BuildSettings::default();
    if let Ok(text) = std::fs::read_to_string(project_root.join("project.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(name) = value.get("name").and_then(|name| name.as_str()) {
                if !name.trim().is_empty() {
                    settings.product_name = name.trim().to_string();
                }
            }
        }
    }
    settings.executable_name = settings.sanitized_executable_name();

    if let Some(relative) = first_asset_with_extension(project_root, &["scene", "hzscene"]) {
        settings.start_scene = relative;
    } else if let Some(relative) = first_asset_file(project_root) {
        dfx_warn!(
            "BuildSettings",
            "工程未找到 .scene 资产，start_scene 暂用第一个资产: {}（请在 Build Settings 修改）",
            relative
        );
        settings.start_scene = relative;
    } else {
        settings.start_scene = "scenes/main.scene.json".to_string();
        dfx_warn!(
            "BuildSettings",
            "工程 assets/ 为空，start_scene 使用占位默认值: {}",
            settings.start_scene
        );
    }
    settings
}

fn first_asset_with_extension(project_root: &Path, extensions: &[&str]) -> Option<String> {
    let assets_root = project_root.join("assets");
    let mut paths = Vec::new();
    collect_files(&assets_root, &mut paths);
    paths.sort();
    for path in paths {
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| extensions.iter().any(|wanted| ext.eq_ignore_ascii_case(wanted)))
            .unwrap_or(false)
        {
            if let Ok(relative) = path.strip_prefix(&assets_root) {
                return Some(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    None
}

fn first_asset_file(project_root: &Path) -> Option<String> {
    let assets_root = project_root.join("assets");
    let mut paths = Vec::new();
    collect_files(&assets_root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .find(|path| path.extension().and_then(|ext| ext.to_str()) != Some("meta"))
        .and_then(|path| {
            path.strip_prefix(&assets_root)
                .ok()
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        })
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// 读取设置文件；缺失/损坏时推断默认值并告警（返回可用设置）。
pub fn load_or_default(project_root: &Path) -> BuildSettings {
    let path = settings_path(project_root);
    match std::fs::read_to_string(&path) {
        Ok(text) => match BuildSettings::from_json(&text) {
            Ok(settings) => settings,
            Err(error) => {
                dfx_warn!(
                    "BuildSettings",
                    "解析 {} 失败（使用默认推断）: {}",
                    path.display(),
                    error
                );
                infer_defaults(project_root)
            }
        },
        Err(_) => infer_defaults(project_root),
    }
}

/// 保存设置文件（先校验；失败返回错误文本，不写盘）。
pub fn save(project_root: &Path, settings: &BuildSettings) -> Result<PathBuf, String> {
    let errors = settings.validate(project_root);
    if !errors.is_empty() {
        return Err(format!("Build Settings 校验失败: {}", errors.join("; ")));
    }
    let path = settings_path(project_root);
    let json = settings
        .to_pretty_json()
        .map_err(|error| format!("序列化 Build Settings 失败: {error}"))?;
    std::fs::write(&path, json).map_err(|error| format!("写入 {} 失败: {error}", path.display()))?;
    Ok(path)
}

// === FFI ===

fn write_c_string(buffer: *mut c_char, buffer_size: usize, text: &str) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
        *buffer.add(copy_len) = 0;
    }
    copy_len
}

fn current_project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// FFI：读取当前工程 Build Settings JSON（缺失时返回推断默认值）；返回字节数。
#[unsafe(no_mangle)]
pub extern "C" fn build_settings_snapshot_json(
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let root = current_project_root();
    let settings = load_or_default(&root);
    let json = match settings.to_pretty_json() {
        Ok(json) => json,
        Err(error) => {
            dfx_error!("BuildSettings", "序列化快照失败: {}", error);
            unsafe { *buffer = 0; }
            return 0;
        }
    };
    write_c_string(buffer, buffer_size, &json)
}

/// FFI：保存 Build Settings JSON。
///
/// 返回 0=成功；-1=JSON 解析失败；-2=校验失败；-3=写盘失败。错误写日志。
#[unsafe(no_mangle)]
pub extern "C" fn build_settings_save_json(json_ptr: *const c_char) -> i32 {
    if json_ptr.is_null() {
        return -1;
    }
    let json = unsafe { std::ffi::CStr::from_ptr(json_ptr).to_string_lossy().into_owned() };
    let settings = match BuildSettings::from_json(&json) {
        Ok(settings) => settings,
        Err(error) => {
            dfx_error!("BuildSettings", "保存失败：JSON 解析错误: {}", error);
            return -1;
        }
    };
    let root = current_project_root();
    let errors = settings.validate(&root);
    if !errors.is_empty() {
        dfx_error!("BuildSettings", "保存失败：{}", errors.join("; "));
        return -2;
    }
    match save(&root, &settings) {
        Ok(path) => {
            dfx_info!("BuildSettings", "已保存: {}", path.display());
            0
        }
        Err(error) => {
            dfx_error!("BuildSettings", "保存失败: {}", error);
            -3
        }
    }
}

/// FFI：校验 Build Settings JSON；错误文本写入 `buffer`。
///
/// 返回 0=通过；-1=JSON 解析失败；-2=校验失败。`errors` 写入缓冲（通过时为空串）。
#[unsafe(no_mangle)]
pub extern "C" fn build_settings_validate_json(
    json_ptr: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> i32 {
    if json_ptr.is_null() {
        write_c_string(buffer, buffer_size, "JSON 为空");
        return -1;
    }
    let json = unsafe { std::ffi::CStr::from_ptr(json_ptr).to_string_lossy().into_owned() };
    let settings = match BuildSettings::from_json(&json) {
        Ok(settings) => settings,
        Err(error) => {
            let message = format!("JSON 解析失败: {error}");
            write_c_string(buffer, buffer_size, &message);
            return -1;
        }
    };
    let root = current_project_root();
    let errors = settings.validate(&root);
    if errors.is_empty() {
        write_c_string(buffer, buffer_size, "");
        0
    } else {
        let message = errors.join("; ");
        write_c_string(buffer, buffer_size, &message);
        -2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("target")
            .join("build_settings_tmp")
            .join(format!("{tag}_{}_{}", std::process::id(), nanos));
        std::fs::create_dir_all(root.join("assets").join("scenes")).unwrap();
        std::fs::write(root.join("assets").join("scenes").join("main.scene"), "{}").unwrap();
        root
    }

    #[test]
    fn roundtrip_preserves_all_fields() {
        let root = temp_project("roundtrip");
        let mut settings = infer_defaults(&root);
        settings.product_name = "My Game".to_string();
        settings.executable_name = "my-game".to_string();
        settings.start_scene = "scenes/main.scene".to_string();
        settings.quality = "ultra".to_string();
        settings.window.width = 1920;
        settings.window.height = 1080;
        settings.window.mode = "fullscreen".to_string();
        settings.targets = vec!["windows".to_string()];
        settings.version = "1.2.3".to_string();
        settings.log_dir = "user_logs".to_string();
        settings.required_font = None;

        let path = save(&root, &settings).expect("保存应成功");
        assert!(path.is_file());
        let loaded = load_or_default(&root);
        assert_eq!(loaded, settings);
    }

    #[test]
    fn defaults_are_inferred_and_valid() {
        let root = temp_project("defaults");
        let settings = infer_defaults(&root);
        assert!(!settings.product_name.is_empty());
        assert_eq!(settings.start_scene, "scenes/main.scene");
        assert_eq!(settings.quality, "high");
        assert_eq!(settings.window.width, 1280);
        assert!(settings.validate(&root).is_empty(), "{:?}", settings.validate(&root));
    }

    #[test]
    fn validation_rejects_bad_values() {
        let root = temp_project("validate");
        let mut settings = infer_defaults(&root);
        settings.product_name = "".to_string();
        settings.executable_name = "bad name!".to_string();
        settings.version = "abc".to_string();
        settings.quality = "insane".to_string();
        settings.window.width = 10;
        settings.window.height = 10;
        settings.window.mode = "floating".to_string();
        settings.targets = vec!["android".to_string()];
        settings.start_scene = "scenes/missing.scene".to_string();
        settings.log_dir = "../outside".to_string();

        let errors = settings.validate(&root);
        let joined = errors.join("\n");
        for expected in [
            "product_name",
            "executable_name",
            "version",
            "quality",
            "window.width",
            "window.height",
            "window.mode",
            "targets",
            "start_scene 不存在",
            "log_dir",
        ] {
            assert!(joined.contains(expected), "缺少错误: {expected}\n{joined}");
        }
        assert!(save(&root, &settings).is_err());
    }

    #[test]
    fn json_parse_failures_are_reported() {
        assert!(BuildSettings::from_json("{not json").is_err());
        let settings = BuildSettings::from_json("{}").expect("空对象应使用默认值");
        assert_eq!(settings.quality, "high");
        assert_eq!(settings.window.width, 1280);
    }
}
