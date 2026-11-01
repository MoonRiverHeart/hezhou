//! `manifest.json` 数据结构：Player 运行时启动时读取的唯一入口契约。
//!
//! 格式约定（详见 `README.md`）：
//! - `format_version` 是清单结构版本，运行时严格匹配；
//! - `engine_version` 是生成清单的引擎版本，运行时按 major.minor 兼容；
//! - `target` 必须与当前运行平台一致；
//! - 所有路径都是逻辑路径，相对于包根目录。

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::{
    default_executable_stem, ConfigErrors, PlayerBuildConfig, QualityTier, WindowConfig,
};
use crate::paths::{case_collision_key, normalize_logical_path, to_native_path, PathError};
use crate::platform::TargetPlatform;
use crate::version::{SemVer, VersionError};
use crate::RUNTIME_ENGINE_VERSION;

/// 清单文件名（包根目录）。
pub const MANIFEST_FILE_NAME: &str = "manifest.json";
/// 当前支持的清单结构版本。
pub const MANIFEST_FORMAT_VERSION: u32 = 1;

/// Player 包内固定目录名。
pub const ASSETS_DIR: &str = "assets";
/// 脚本（预编译 C# 程序集）目录。
pub const SCRIPTS_DIR: &str = "scripts";
/// 运行时（Mono/Vulkan/VC runtime）目录。
pub const RUNTIME_DIR: &str = "runtime";
/// 日志目录（包内兜底；运行时优先写用户数据目录）。
pub const LOGS_DIR: &str = "logs";

/// 清单中的一条资产记录。
///
/// `path` 相对 `paths.assets`，由 cook 输出决定：
/// - legacy 玩家清单：普通资产相对路径（如 `scenes/main.scene`）；
/// - hezhou-asset cook 产物：等价于 `output`（如 `imported/<uuid>.json`、
///   `assets/<uuid>.png`），因此路径可能以 `assets/` 开头。
///
/// `asset_id`/`locator`/`output`/`dependencies`/`source_*` 只在真实 cook 输出时存在，
/// 用于 Player 运行时按 `AssetId` 索引（见 `engine/asset/README.md` §6）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestAsset {
    /// 相对 `paths.assets` 的逻辑路径（cook 场景下等于 `output`）。
    pub path: String,
    /// 资产类型（开放字符串，默认 `other`；cook 场景下为 `AssetKind` 的 snake_case）。
    #[serde(default = "default_asset_kind")]
    pub kind: String,
    /// 烘焙后字节数（cook 输出文件的实际大小）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// 内容校验值（`fnv1a64:...`，对 cook 输出文件计算）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// hezhou-asset 的稳定 `AssetId`（UUID）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    /// 源定位符（工程相对逻辑路径 + 可选子资产键）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    /// cook 输出相对路径（与 `path` 相同，保留原字段便于对照 cook 契约）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// 直接依赖的 AssetId（已排序，cook manifest 原样保留）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    /// 源文件内容哈希（cook manifest 的 `source_hash`，FNV-1a 64 hex）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,
    /// 源文件字节数（cook manifest 的 `source_size`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_size: Option<u64>,
}

fn default_asset_kind() -> String {
    "other".to_string()
}

impl ManifestAsset {
    pub fn new(path: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: kind.into(),
            size_bytes: None,
            checksum: None,
            asset_id: None,
            locator: None,
            output: None,
            dependencies: Vec::new(),
            source_hash: None,
            source_size: None,
        }
    }

    pub fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// 标记来源 cook 条目（AssetId/定位符/依赖/源信息）。
    pub fn with_cook_metadata(
        mut self,
        asset_id: impl Into<String>,
        locator: impl Into<String>,
        output: impl Into<String>,
        dependencies: Vec<String>,
        source_hash: impl Into<String>,
        source_size: u64,
    ) -> Self {
        self.asset_id = Some(asset_id.into());
        self.locator = Some(locator.into());
        self.output = Some(output.into());
        self.dependencies = dependencies;
        self.source_hash = Some(source_hash.into());
        self.source_size = Some(source_size);
        self
    }
}

/// Player 包内目录布局（逻辑路径，可重定位）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePaths {
    pub assets: String,
    pub scripts: String,
    pub runtime: String,
    pub logs: String,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self {
            assets: ASSETS_DIR.to_string(),
            scripts: SCRIPTS_DIR.to_string(),
            runtime: RUNTIME_DIR.to_string(),
            logs: LOGS_DIR.to_string(),
        }
    }
}

/// Player 运行时清单。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerManifest {
    /// 清单结构版本。
    pub format_version: u32,
    /// 生成清单的引擎版本（`major.minor.patch`）。
    pub engine_version: String,
    /// 产品显示名。
    pub product_name: String,
    /// 游戏版本（`major.minor.patch`）。
    pub version: String,
    /// 启动场景，相对 `assets/`。
    pub start_scene: String,
    /// 目标平台。
    pub target: TargetPlatform,
    /// 质量档。
    pub quality: QualityTier,
    /// 窗口配置。
    pub window: WindowConfig,
    /// 包内目录布局。
    pub paths: RuntimePaths,
    /// 必检字体，相对 `assets/`；`null` 表示不检查。
    #[serde(default)]
    pub font: Option<String>,
    /// 预编译 C# 程序集文件名（相对 `scripts/`），如 `Game.dll`。
    #[serde(default)]
    pub scripts: Vec<String>,
    /// 打包资产列表（相对 `assets/`）。
    #[serde(default)]
    pub assets: Vec<ManifestAsset>,
    /// 构建器标识，便于复现构建。
    #[serde(default)]
    pub built_by: String,
}

impl PlayerManifest {
    /// 由构建配置 + cook 结果生成清单。
    ///
    /// 传入的 `assets` 路径应已相对 `assets/`，函数内部仍会做一次规范化校验
    /// （`validate`），保证构建产物契约一致。
    pub fn new(
        config: &PlayerBuildConfig,
        assets: Vec<ManifestAsset>,
        scripts: Vec<String>,
    ) -> Self {
        let start_scene = config
            .normalized_start_scene()
            .unwrap_or_else(|_| config.start_scene.clone());
        let font = config
            .required_font
            .as_deref()
            .and_then(|font| normalize_logical_path(font).ok());
        Self::from_parts(config, assets, scripts, start_scene, font)
    }

    /// 由构建配置 + 已映射资产生成清单。
    ///
    /// `start_scene` / `font` 是**相对 `paths.assets`** 的最终路径：
    /// legacy 输入为配置中的逻辑路径；真实 cook 输入为对应条目的 `output`
    /// （如 `assets/<uuid>.scene`），因此可能带 `assets/` 前缀。
    pub fn from_parts(
        config: &PlayerBuildConfig,
        assets: Vec<ManifestAsset>,
        scripts: Vec<String>,
        start_scene: impl Into<String>,
        font: Option<String>,
    ) -> Self {
        Self {
            format_version: MANIFEST_FORMAT_VERSION,
            engine_version: RUNTIME_ENGINE_VERSION.to_string(),
            product_name: config.product_name.clone(),
            version: config.version.clone(),
            start_scene: start_scene.into(),
            target: config.target,
            quality: config.quality,
            window: config.window.clone(),
            paths: RuntimePaths {
                logs: normalize_logical_path(&config.log_dir)
                    .unwrap_or_else(|_| LOGS_DIR.to_string()),
                ..RuntimePaths::default()
            },
            font,
            scripts,
            assets,
            built_by: format!("hezhou-player-builder {}", RUNTIME_ENGINE_VERSION),
        }
    }

    /// 从目录读取 `manifest.json`。
    pub fn load_from_dir(root: &Path) -> Result<Self, ManifestError> {
        Self::load(&root.join(MANIFEST_FILE_NAME))
    }

    /// 从文件读取清单。
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Io {
            context: format!("读取清单 {}", path.display()),
            source,
        })?;
        let manifest: Self = serde_json::from_str(&text).map_err(|source| ManifestError::Json {
            context: format!("解析清单 {}", path.display()),
            source,
        })?;
        Ok(manifest)
    }

    /// 序列化为稳定的 pretty JSON（字段顺序由结构体定义，资产已由构建器排序）。
    pub fn to_json_pretty(&self) -> Result<String, ManifestError> {
        serde_json::to_string_pretty(self)
            .map(|mut json| {
                json.push('\n');
                json
            })
            .map_err(|source| ManifestError::Json {
                context: "序列化清单".to_string(),
                source,
            })
    }

    /// 写入清单文件（自动创建父目录）。
    pub fn save(&self, path: &Path) -> Result<(), ManifestError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| ManifestError::Io {
                    context: format!("创建清单目录 {}", parent.display()),
                    source,
                })?;
            }
        }
        let json = self.to_json_pretty()?;
        std::fs::write(path, json).map_err(|source| ManifestError::Io {
            context: format!("写入清单 {}", path.display()),
            source,
        })
    }

    /// 结构校验（构建期与运行期共用）：格式版本、字段、路径、重复/大小写冲突。
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.format_version != MANIFEST_FORMAT_VERSION {
            return Err(ManifestError::UnsupportedFormat {
                found: self.format_version,
                supported: MANIFEST_FORMAT_VERSION,
            });
        }

        // 复用构建配置校验（产品名/版本/启动场景/窗口/字体/日志目录）。
        let config = PlayerBuildConfig {
            product_name: self.product_name.clone(),
            executable_name: default_executable_stem(&self.product_name),
            start_scene: self.start_scene.clone(),
            version: self.version.clone(),
            target: self.target,
            quality: self.quality,
            window: self.window.clone(),
            log_dir: self.paths.logs.clone(),
            required_font: self.font.clone(),
        };
        if let Err(errors) = config.validate() {
            return Err(ManifestError::InvalidConfig(errors));
        }

        if let Err(source) = SemVer::parse(&self.engine_version) {
            return Err(ManifestError::InvalidVersion {
                field: "engine_version",
                source,
            });
        }

        for (field, value) in [
            ("paths.assets", &self.paths.assets),
            ("paths.scripts", &self.paths.scripts),
            ("paths.runtime", &self.paths.runtime),
            ("paths.logs", &self.paths.logs),
        ] {
            if let Err(source) = normalize_logical_path(value) {
                return Err(ManifestError::InvalidPath { field, source });
            }
        }

        // 资产路径：相对 paths.assets、规范化、无重复、无大小写冲突（Windows 会覆盖）。
        // cook 产出的 `imported/*.json`、`assets/*` 以 `assets/` 开头是合法的（路径即 output）。
        let mut seen: HashMap<String, &str> = HashMap::new();
        for asset in &self.assets {
            let normalized = normalize_logical_path(&asset.path).map_err(|source| {
                ManifestError::InvalidPath {
                    field: "assets[].path",
                    source,
                }
            })?;
            if asset.asset_id.as_deref() == Some("") {
                return Err(ManifestError::EmptyAssetId {
                    path: normalized.clone(),
                });
            }
            if let Some(output) = &asset.output {
                normalize_logical_path(output).map_err(|source| ManifestError::InvalidPath {
                    field: "assets[].output",
                    source,
                })?;
            }
            let key = case_collision_key(&normalized);
            if let Some(previous) = seen.get(&key) {
                return Err(ManifestError::CaseCollision {
                    first: (*previous).to_string(),
                    second: normalized,
                });
            }
            seen.insert(key, &asset.path);
        }

        // 启动场景必须在资产列表内。
        let start_scene = self.normalized_start_scene()?;
        let start_scene_packaged = self.assets.iter().any(|asset| {
            normalize_logical_path(&asset.path)
                .map(|path| path == start_scene)
                .unwrap_or(false)
        });
        if !start_scene_packaged {
            return Err(ManifestError::MissingStartScene {
                path: start_scene,
            });
        }

        // 字体必须随包。
        if let Some(font) = &self.font {
            let font = normalize_logical_path(font).map_err(|source| {
                ManifestError::InvalidPath {
                    field: "font",
                    source,
                }
            })?;
            let font_packaged = self.assets.iter().any(|asset| {
                normalize_logical_path(&asset.path)
                    .map(|path| path == font)
                    .unwrap_or(false)
            });
            if !font_packaged {
                return Err(ManifestError::MissingFont { path: font });
            }
        }

        // 脚本文件名：单段、`.dll`、无重复/大小写冲突。
        let mut script_keys: HashMap<String, &str> = HashMap::new();
        for script in &self.scripts {
            validate_script_file_name(script)?;
            let key = case_collision_key(script);
            if let Some(previous) = script_keys.get(&key) {
                return Err(ManifestError::CaseCollision {
                    first: (*previous).to_string(),
                    second: script.clone(),
                });
            }
            script_keys.insert(key, script);
        }

        Ok(())
    }

    /// 运行时校验：结构 + 目标平台 + 引擎版本兼容。
    pub fn validate_for_runtime(&self, host: TargetPlatform) -> Result<(), ManifestError> {
        self.validate()?;
        if self.target != host {
            return Err(ManifestError::PlatformMismatch {
                manifest: self.target,
                host,
            });
        }
        let manifest_version =
            SemVer::parse(&self.engine_version).map_err(|source| ManifestError::InvalidVersion {
                field: "engine_version",
                source,
            })?;
        let runtime_version =
            SemVer::parse(RUNTIME_ENGINE_VERSION).expect("crate 版本号必须是合法 semver");
        if !manifest_version.is_compatible_with(&runtime_version) {
            return Err(ManifestError::EngineVersionMismatch {
                manifest: manifest_version.to_string(),
                runtime: runtime_version.to_string(),
            });
        }
        Ok(())
    }

    /// 规范化后的启动场景（相对 assets/）。
    pub fn normalized_start_scene(&self) -> Result<String, ManifestError> {
        normalize_logical_path(&self.start_scene).map_err(|source| ManifestError::InvalidPath {
            field: "start_scene",
            source,
        })
    }

    /// 包根目录下的清单路径。
    pub fn manifest_path(root: &Path) -> PathBuf {
        root.join(MANIFEST_FILE_NAME)
    }

    /// 资产目录本机路径。
    pub fn assets_dir(&self, root: &Path) -> PathBuf {
        root.join(to_native_path(&self.paths.assets, self.target))
    }

    /// 脚本目录本机路径。
    pub fn scripts_dir(&self, root: &Path) -> PathBuf {
        root.join(to_native_path(&self.paths.scripts, self.target))
    }

    /// 运行时目录本机路径。
    pub fn runtime_dir(&self, root: &Path) -> PathBuf {
        root.join(to_native_path(&self.paths.runtime, self.target))
    }

    /// 启动场景本机路径。
    pub fn start_scene_path(&self, root: &Path) -> PathBuf {
        let scene = self
            .normalized_start_scene()
            .unwrap_or_else(|_| self.start_scene.clone());
        self.assets_dir(root)
            .join(to_native_path(&scene, self.target))
    }

    /// 字体本机路径（若配置）。
    pub fn font_path(&self, root: &Path) -> Option<PathBuf> {
        self.font.as_deref().and_then(|font| {
            normalize_logical_path(font)
                .ok()
                .map(|font| self.assets_dir(root).join(to_native_path(&font, self.target)))
        })
    }
}

/// 校验脚本文件名（单段、`.dll`、允许 ASCII 字母数字与 `. _ -`）。
pub fn validate_script_file_name(name: &str) -> Result<(), ManifestError> {
    let normalized = normalize_logical_path(name).map_err(|source| ManifestError::InvalidPath {
        field: "scripts[]",
        source,
    })?;
    if normalized.contains('/') {
        return Err(ManifestError::InvalidScriptName {
            name: name.to_string(),
            reason: "脚本名必须是单段文件名（相对 scripts/）",
        });
    }
    let lower = normalized.to_ascii_lowercase();
    if !lower.ends_with(".dll") {
        return Err(ManifestError::InvalidScriptName {
            name: name.to_string(),
            reason: "必须是 .dll 预编译程序集",
        });
    }
    let stem = &normalized[..normalized.len() - 4];
    if stem.is_empty()
        || !stem
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        || stem.ends_with('.')
    {
        return Err(ManifestError::InvalidScriptName {
            name: name.to_string(),
            reason: "程序集名只允许 ASCII 字母数字、`.`、`_`、`-`",
        });
    }
    Ok(())
}

/// 清单相关错误。
#[derive(Debug)]
pub enum ManifestError {
    Io {
        context: String,
        source: std::io::Error,
    },
    Json {
        context: String,
        source: serde_json::Error,
    },
    UnsupportedFormat {
        found: u32,
        supported: u32,
    },
    InvalidConfig(ConfigErrors),
    InvalidVersion {
        field: &'static str,
        source: VersionError,
    },
    InvalidPath {
        field: &'static str,
        source: PathError,
    },
    InvalidAssetPath {
        path: String,
        reason: &'static str,
    },
    EmptyAssetId {
        path: String,
    },
    InvalidScriptName {
        name: String,
        reason: &'static str,
    },
    MissingStartScene {
        path: String,
    },
    MissingFont {
        path: String,
    },
    CaseCollision {
        first: String,
        second: String,
    },
    EngineVersionMismatch {
        manifest: String,
        runtime: String,
    },
    PlatformMismatch {
        manifest: TargetPlatform,
        host: TargetPlatform,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context} 失败：{source}"),
            Self::Json { context, source } => write!(f, "{context} 失败：{source}"),
            Self::UnsupportedFormat { found, supported } => write!(
                f,
                "不支持的清单格式版本 {found}（运行时支持 {supported}）"
            ),
            Self::InvalidConfig(errors) => write!(f, "{errors}"),
            Self::InvalidVersion { field, source } => write!(f, "{field} {source}"),
            Self::InvalidPath { field, source } => write!(f, "{field} {source}"),
            Self::InvalidAssetPath { path, reason } => {
                write!(f, "资产路径 `{path}` 非法：{reason}")
            }
            Self::EmptyAssetId { path } => {
                write!(f, "资产 `{path}` 的 asset_id 为空字符串")
            }
            Self::InvalidScriptName { name, reason } => {
                write!(f, "脚本文件名 `{name}` 非法：{reason}")
            }
            Self::MissingStartScene { path } => {
                write!(f, "启动场景 `{path}` 不在资产清单中")
            }
            Self::MissingFont { path } => write!(f, "必检字体 `{path}` 不在资产清单中"),
            Self::CaseCollision { first, second } => write!(
                f,
                "资产/脚本路径仅大小写不同（Windows 会互相覆盖）：`{first}` 与 `{second}`"
            ),
            Self::EngineVersionMismatch { manifest, runtime } => write!(
                f,
                "清单引擎版本 {manifest} 与运行时 {runtime} 不兼容（要求 major.minor 一致）"
            ),
            Self::PlatformMismatch { manifest, host } => write!(
                f,
                "清单目标平台是 {manifest}，当前运行平台是 {host}"
            ),
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::InvalidConfig(errors) => Some(errors),
            Self::InvalidVersion { source, .. } => Some(source),
            Self::InvalidPath { source, .. } => Some(source),
            _ => None,
        }
    }
}

