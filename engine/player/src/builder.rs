//! Player 构建器（第一轮：纯 Rust 结构 + dry-run 清单）。
//!
//! 输入：编辑器 cook 产出的资产清单、预编译 C# 程序集、运行时目录。
//! 输出（dry-run）：双平台目标下的完整文件清单（逻辑路径、分类、大小、校验、
//! Unix 权限位、可执行标记）与整体校验指纹；不复制二进制、不依赖真实引擎。
//!
//! 真正的"拷贝 + 写盘"由后续接入 CI/installer 的步骤完成，`write_plan` 只落地
//! 计划元数据（`manifest.json`、`plan.json`、`file-list.txt`），便于人工/CI 审查。

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::config::{ConfigErrors, PlayerBuildConfig};
use crate::cook_bridge::{map_cooked_assets, CookedAssets};
use crate::manifest::{
    validate_script_file_name, ManifestAsset, ManifestError, PlayerManifest, ASSETS_DIR,
    MANIFEST_FILE_NAME, RUNTIME_DIR, SCRIPTS_DIR,
};
use crate::paths::{case_collision_key, normalize_logical_path, to_native_path, PathError};
use crate::platform::TargetPlatform;
use crate::RUNTIME_ENGINE_VERSION;

/// 编辑器 cook 产出的资产清单。
///
/// 与 asset 模块的集成点：cook 产物序列化为 `{ "assets": [ ... ] }`，
/// 每条记录对应 [`ManifestAsset`]（`path` 相对 `assets/`，`kind` 为开放字符串）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CookManifest {
    /// 烘焙后的资产条目。
    #[serde(default)]
    pub assets: Vec<ManifestAsset>,
}

impl CookManifest {
    pub fn new(assets: Vec<ManifestAsset>) -> Self {
        Self { assets }
    }

    /// 从 JSON 字符串解析 cook 清单。
    pub fn from_json_str(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// 从文件读取 cook 清单。
    pub fn load(path: &Path) -> Result<Self, BuildError> {
        let text = fs::read_to_string(path).map_err(|source| BuildError::Io {
            context: format!("读取 cook 清单 {}", path.display()),
            source,
        })?;
        Self::from_json_str(&text).map_err(|source| BuildError::Json {
            context: format!("解析 cook 清单 {}", path.display()),
            source,
        })
    }
}

/// 预编译 C# 程序集输入。
///
/// 与 scripting 模块的集成点：编辑器/CI 用 `mcs -target:library` 产出 `.dll`
/// （不能用 .NET 8 编译），构建器只负责按逻辑路径放入 `scripts/`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptAssembly {
    /// 程序集文件名（可带或不带 `.dll`）。
    pub name: String,
    /// 本机源文件路径；dry-run 时用于读取大小/校验。
    pub source: Option<PathBuf>,
    /// 文件字节数（可由源文件元数据自动填充）。
    pub size_bytes: Option<u64>,
}

impl ScriptAssembly {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: None,
            size_bytes: None,
        }
    }

    /// 带源文件构造，自动读取大小。
    pub fn with_source(name: impl Into<String>, source: impl Into<PathBuf>) -> Self {
        let source = source.into();
        let size_bytes = fs::metadata(&source).ok().map(|meta| meta.len());
        Self {
            name: name.into(),
            source: Some(source),
            size_bytes,
        }
    }

    /// 枚举目录下所有 `.dll`（用于 CI 从 `scripts/bin/Mono` 取产物）。
    pub fn from_dir(dir: &Path) -> Result<Vec<Self>, BuildError> {
        if !dir.is_dir() {
            return Err(BuildError::ScriptsSourceMissing {
                path: dir.to_path_buf(),
            });
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|source| BuildError::Io {
                context: format!("读取脚本目录 {}", dir.display()),
                source,
            })?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .collect();
        entries.sort();

        let mut assemblies = Vec::new();
        for path in entries {
            let is_dll = path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("dll"))
                .unwrap_or(false);
            if !is_dll {
                continue;
            }
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .ok_or_else(|| BuildError::InvalidSourcePath {
                    path: path.display().to_string(),
                    reason: "脚本文件名不是合法 UTF-8".to_string(),
                })?;
            assemblies.push(Self::with_source(name, path));
        }
        Ok(assemblies)
    }
}

/// 运行时目录中的单个文件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFileSource {
    /// 相对 `runtime/` 的逻辑路径。
    pub relative_path: String,
    /// 本机源路径。
    pub source: PathBuf,
    /// 文件字节数。
    pub size_bytes: u64,
}

/// 运行时目录内容（Mono 运行时、Vulkan loader、VC runtime、字体兜底等）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimePayload {
    /// 运行时文件列表（按相对路径排序）。
    pub files: Vec<RuntimeFileSource>,
}

impl RuntimePayload {
    pub fn new(files: Vec<RuntimeFileSource>) -> Self {
        let mut payload = Self { files };
        payload.sort();
        payload
    }

    /// 空运行时（第一轮允许，用于纯结构测试）。
    pub fn empty() -> Self {
        Self { files: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn sort(&mut self) {
        self.files
            .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    }

    /// 递归枚举运行时目录（跳过目录符号链接，避免环路）。
    pub fn from_dir(dir: &Path) -> Result<Self, BuildError> {
        if !dir.is_dir() {
            return Err(BuildError::RuntimeSourceMissing {
                path: dir.to_path_buf(),
            });
        }
        let mut files = Vec::new();
        walk_runtime_dir(dir, "", &mut files)?;
        Ok(Self::new(files))
    }
}

fn walk_runtime_dir(
    dir: &Path,
    prefix: &str,
    files: &mut Vec<RuntimeFileSource>,
) -> Result<(), BuildError> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|source| BuildError::Io {
            context: format!("读取运行时目录 {}", dir.display()),
            source,
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();

    for path in entries {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .ok_or_else(|| BuildError::InvalidSourcePath {
                path: path.display().to_string(),
                reason: "运行时文件名不是合法 UTF-8".to_string(),
            })?;
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        // 统一走逻辑路径校验：拒绝非法字符、`..`、保留名。
        let relative = normalize_logical_path(&relative).map_err(|source| {
            BuildError::InvalidRuntimePath {
                path: relative.clone(),
                source,
            }
        })?;

        let metadata = fs::symlink_metadata(&path).map_err(|source| BuildError::Io {
            context: format!("读取运行时文件元数据 {}", path.display()),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            // 符号链接按普通文件处理（记录链接本身大小），不递归目录链接。
            push_runtime_file(&path, relative, metadata.len(), files);
        } else if metadata.is_dir() {
            walk_runtime_dir(&path, &relative, files)?;
        } else if metadata.is_file() {
            push_runtime_file(&path, relative, metadata.len(), files);
        }
    }
    Ok(())
}

fn push_runtime_file(
    path: &Path,
    relative_path: String,
    size_bytes: u64,
    files: &mut Vec<RuntimeFileSource>,
) {
    files.push(RuntimeFileSource {
        relative_path,
        source: path.to_path_buf(),
        size_bytes,
    });
}

/// 构建输入。
#[derive(Clone, Debug)]
pub struct PlayerBuildInput {
    /// 构建配置（`target` 会被双平台计划覆盖）。
    pub config: PlayerBuildConfig,
    /// 简单资产清单（legacy，`{ "assets": [...] }`）；真实 cook 时留空。
    pub cook: CookManifest,
    /// 预编译 C# 程序集。
    pub scripts: Vec<ScriptAssembly>,
    /// 运行时目录内容。
    pub runtime: RuntimePayload,
    /// hezhou-asset 真实 cook 输出（`cook_manifest.json + imported/ + assets/`）。
    ///
    /// `Some` 时以 cook 条目为准（`cook.assets` 被忽略），资产文件直接从
    /// `output_root` 复制并校验；详见 [`crate::cook_bridge`]。
    pub cooked: Option<CookedAssets>,
}

impl PlayerBuildInput {
    /// 最小输入（legacy 资产清单）。
    pub fn new(config: PlayerBuildConfig, cook: CookManifest) -> Self {
        Self {
            config,
            cook,
            scripts: Vec::new(),
            runtime: RuntimePayload::empty(),
            cooked: None,
        }
    }

    /// 使用 hezhou-asset 真实 cook 输出（忽略 legacy `cook.assets`）。
    pub fn with_cooked(mut self, cooked: CookedAssets) -> Self {
        self.cooked = Some(cooked);
        self
    }
}

/// 清单中的文件分类。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactCategory {
    /// 可执行文件。
    Executable,
    /// 烘焙资产。
    Asset,
    /// 预编译 C# 程序集。
    Script,
    /// 运行时文件。
    Runtime,
    /// 资产元数据（如 `assets/cook_manifest.json`）。
    Metadata,
    /// 清单文件。
    Manifest,
}

impl ArtifactCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::Asset => "asset",
            Self::Script => "script",
            Self::Runtime => "runtime",
            Self::Metadata => "metadata",
            Self::Manifest => "manifest",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Executable => 0,
            Self::Asset => 1,
            Self::Script => 2,
            Self::Runtime => 3,
            Self::Metadata => 4,
            Self::Manifest => 5,
        }
    }
}

impl fmt::Display for ArtifactCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// dry-run 计划中的一个文件。
#[derive(Clone, Debug, Serialize)]
pub struct PlannedFile {
    /// 包内逻辑路径（固定 `/`）。
    pub relative_path: String,
    /// 文件分类。
    pub category: ArtifactCategory,
    /// 字节数（未知时 `null`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// 内容校验（`fnv1a64:...`；未知时 `null`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// 是否可执行。
    pub executable: bool,
    /// Unix 权限位（Windows 目标为 `null`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unix_mode: Option<u32>,
    /// 源文件路径（本机构建信息，不写入计划 JSON）。
    #[serde(skip)]
    pub source: Option<PathBuf>,}

/// 构建警告（不阻塞计划生成）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildWarning {
    /// 运行时目录为空：产物依赖系统 Mono/系统库。
    RuntimePayloadEmpty,
    /// 没有预编译脚本：游戏逻辑为空。
    ScriptsEmpty,
    /// 未配置必检字体。
    FontNotConfigured,
    /// Linux 需要链接期 rpath。
    LinuxRpathRequired,
    /// 可执行文件由 cargo 产出，dry-run 不复制二进制。
    ExecutableProducedByCargo,
}

impl BuildWarning {
    pub fn code(self) -> &'static str {
        match self {
            Self::RuntimePayloadEmpty => "runtime_payload_empty",
            Self::ScriptsEmpty => "scripts_empty",
            Self::FontNotConfigured => "font_not_configured",
            Self::LinuxRpathRequired => "linux_rpath_required",
            Self::ExecutableProducedByCargo => "executable_produced_by_cargo",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::RuntimePayloadEmpty => "runtime/ 目录为空：Player 将依赖系统 Mono/系统库，不满足“干净机器可运行”验收",
            Self::ScriptsEmpty => "没有预编译 C# 程序集：Player 不会加载任何游戏逻辑",
            Self::FontNotConfigured => "未配置必检字体：可能依赖系统字体，Windows/Linux 表现可能不一致",
            Self::LinuxRpathRequired => "Linux 构建需要在链接期注入 -Wl,-rpath,$ORIGIN/runtime（CI 集成点）",
            Self::ExecutableProducedByCargo => "可执行文件由 cargo 构建产物提供，dry-run 只记录目标路径",
        }
    }
}

impl fmt::Display for BuildWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code(), self.message())
    }
}

/// dry-run 构建计划。
#[derive(Clone, Debug, Serialize)]
pub struct PlayerBuildPlan {
    /// 目标平台。
    pub target: TargetPlatform,
    /// 目标平台对应的构建配置。
    pub config: PlayerBuildConfig,
    /// 将写入产物根目录的 `manifest.json`。
    pub manifest: PlayerManifest,
    /// 完整文件清单（按分类、路径排序）。
    pub files: Vec<PlannedFile>,
    /// 非阻塞警告。
    pub warnings: Vec<BuildWarning>,
    /// 整体校验指纹（文件清单 + 大小 + 校验值）。
    pub fingerprint: String,
}

impl PlayerBuildPlan {
    /// 按逻辑路径查找文件。
    pub fn file(&self, relative_path: &str) -> Option<&PlannedFile> {
        self.files
            .iter()
            .find(|file| file.relative_path == relative_path)
    }

    /// 某分类下的文件。
    pub fn files_in(&self, category: ArtifactCategory) -> Vec<&PlannedFile> {
        self.files
            .iter()
            .filter(|file| file.category == category)
            .collect()
    }

    /// 已知大小的文件总字节数。
    pub fn total_known_size(&self) -> u64 {
        self.files
            .iter()
            .filter_map(|file| file.size_bytes)
            .sum()
    }

    /// 落地计划元数据：`manifest.json`、`plan.json`、`file-list.txt`。
    ///
    /// 注意：**不会**复制 exe/assets/scripts/runtime 二进制，真正拷贝留给构建流水线。
    pub fn write_plan(&self, dir: &Path) -> Result<(), BuildError> {
        fs::create_dir_all(dir).map_err(|source| BuildError::Io {
            context: format!("创建计划目录 {}", dir.display()),
            source,
        })?;
        self.manifest
            .save(&dir.join(MANIFEST_FILE_NAME))
            .map_err(BuildError::Manifest)?;

        let plan_json = serde_json::to_string_pretty(self).map_err(|source| BuildError::Json {
            context: "序列化构建计划".to_string(),
            source,
        })?;
        fs::write(dir.join("plan.json"), format!("{plan_json}\n")).map_err(|source| {
            BuildError::Io {
                context: format!("写入计划 {}", dir.join("plan.json").display()),
                source,
            }
        })?;

        fs::write(dir.join("file-list.txt"), self.file_list()).map_err(|source| {
            BuildError::Io {
                context: format!("写入计划 {}", dir.join("file-list.txt").display()),
                source,
            }
        })?;
        Ok(())
    }

    /// 稳定的文件清单文本（供 CI 做 diff/审查）。
    pub fn file_list(&self) -> String {
        let mut lines = String::new();
        lines.push_str("# category\tmode\tsize\tchecksum\tpath\n");
        for file in &self.files {
            lines.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                file.category,
                file.unix_mode
                    .map(|mode| format!("{mode:04o}"))
                    .unwrap_or_else(|| "-".to_string()),
                file.size_bytes
                    .map(|size| size.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                file.checksum.as_deref().unwrap_or("-"),
                file.relative_path,
            ));
        }
        lines
    }

    /// 人类可读的 dry-run 报告。
    pub fn report(&self) -> String {
        let mut out = String::new();
        out.push_str("Player 构建计划（dry-run）\n");
        out.push_str(&format!("  目标平台: {}\n", self.target.display_name()));
        out.push_str(&format!("  产品    : {}\n", self.manifest.product_name));
        out.push_str(&format!(
            "  版本    : {}（引擎 {}）\n",
            self.manifest.version, self.manifest.engine_version
        ));
        out.push_str(&format!("  启动场景: assets/{}\n", self.manifest.start_scene));
        out.push_str(&format!(
            "  窗口    : {}x{} {} \"{}\"\n",
            self.manifest.window.width,
            self.manifest.window.height,
            self.manifest.window.mode.as_str(),
            self.manifest.window.title
        ));
        out.push_str(&format!(
            "  质量档  : {}\n",
            self.manifest.quality.as_str()
        ));
        out.push_str(&format!(
            "  统计    : 资产 {} / 脚本 {} / 运行时 {} / 合计 {} 个文件，已知大小 {}\n",
            self.files_in(ArtifactCategory::Asset).len(),
            self.files_in(ArtifactCategory::Script).len(),
            self.files_in(ArtifactCategory::Runtime).len(),
            self.files.len(),
            human_size(self.total_known_size()),
        ));
        out.push_str("文件清单:\n");
        for file in &self.files {
            out.push_str(&format!(
                "  [{}] {:<56} {:>10}  {:<10} {}\n",
                file.category.as_str(),
                file.relative_path,
                file.size_bytes
                    .map(human_size)
                    .unwrap_or_else(|| "-".to_string()),
                file.unix_mode
                    .map(|mode| format!("{mode:04o}"))
                    .unwrap_or_else(|| "-".to_string()),
                file.checksum.as_deref().unwrap_or("-"),
            ));
        }
        out.push_str(&format!("校验指纹: {}\n", self.fingerprint));
        if self.warnings.is_empty() {
            out.push_str("警告    : 无\n");
        } else {
            out.push_str(&format!("警告    : {} 条\n", self.warnings.len()));
            for warning in &self.warnings {
                out.push_str(&format!("  - {warning}\n"));
            }
        }
        out
    }
}

impl fmt::Display for PlayerBuildPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.report())
    }
}

/// dry-run 构建器。不写产物，只生成计划。
#[derive(Clone, Debug)]
pub struct PlayerBuilder {
    input: PlayerBuildInput,
}

impl PlayerBuilder {
    pub fn new(input: PlayerBuildInput) -> Self {
        Self { input }
    }

    pub fn input(&self) -> &PlayerBuildInput {
        &self.input
    }

    /// 按配置中的目标平台生成计划。
    pub fn plan(&self) -> Result<PlayerBuildPlan, BuildError> {
        self.plan_for(self.input.config.target)
    }

    /// 生成 Windows + Linux 双平台计划（顺序固定，便于 CI 对比）。
    pub fn plan_both_targets(&self) -> Result<Vec<PlayerBuildPlan>, BuildError> {
        TargetPlatform::ALL
            .iter()
            .map(|target| self.plan_for(*target))
            .collect()
    }

    /// 指定平台生成计划。
    pub fn plan_for(&self, target: TargetPlatform) -> Result<PlayerBuildPlan, BuildError> {
        let mut config = self.input.config.clone();
        config.target = target;
        if let Err(errors) = config.validate() {
            return Err(BuildError::Config(errors));
        }

        // ---- 资产来源：hezhou-asset 真实 cook 输出 或 legacy 简单清单 ----
        let assets: Vec<ManifestAsset>;
        let mut files: Vec<PlannedFile> = Vec::new();
        let start_scene: String;
        let font: Option<String>;

        if let Some(cooked) = &self.input.cooked {
            // 真实 cook：cook entries → Player assets[]（path == output），
            // 文件来源为 cook 输出目录，逐条校验存在/大小/校验和。
            let mapped = map_cooked_assets(cooked, &config, target)?;
            assets = mapped.assets;
            start_scene = mapped.start_scene;
            font = mapped.font;
            files.extend(mapped.files);
        } else {
            let legacy_start_scene = config
                .normalized_start_scene()
                .map_err(BuildError::InvalidStartScene)?;
            let legacy_font = config
                .required_font
                .as_deref()
                .map(normalize_logical_path)
                .transpose()
                .map_err(|source| BuildError::InvalidRuntimePath {
                    path: config.required_font.clone().unwrap_or_default(),
                    source,
                })?;

            let mut legacy_assets: Vec<ManifestAsset> = Vec::new();
            let mut asset_keys: HashMap<String, String> = HashMap::new();
            for legacy in &self.input.cook.assets {
                let relative = normalize_asset_relative(&legacy.path).map_err(|source| {
                    BuildError::InvalidAssetPath {
                        path: legacy.path.clone(),
                        source,
                    }
                })?;
                let key = case_collision_key(&relative);
                if let Some(previous) = asset_keys.insert(key, relative.clone()) {
                    return Err(BuildError::CaseCollision {
                        first: previous,
                        second: relative,
                    });
                }
                let kind = legacy.kind.trim();
                let kind = if kind.is_empty() { "other" } else { kind };
                let mut asset = ManifestAsset::new(relative, kind);
                asset.size_bytes = legacy.size_bytes;
                asset.checksum = legacy.checksum.clone();
                legacy_assets.push(asset);
            }
            legacy_assets.sort_by(|left, right| left.path.cmp(&right.path));

            if !legacy_assets
                .iter()
                .any(|asset| asset.path == legacy_start_scene)
            {
                return Err(BuildError::MissingStartScene {
                    path: format!("{ASSETS_DIR}/{legacy_start_scene}"),
                });
            }
            if let Some(font_path) = &legacy_font {
                if !legacy_assets.iter().any(|asset| asset.path == *font_path) {
                    return Err(BuildError::MissingFont {
                        path: format!("{ASSETS_DIR}/{font_path}"),
                    });
                }
            }

            for asset in &legacy_assets {
                files.push(PlannedFile {
                    relative_path: format!("{ASSETS_DIR}/{}", asset.path),
                    category: ArtifactCategory::Asset,
                    size_bytes: asset.size_bytes,
                    checksum: asset.checksum.clone(),
                    executable: false,
                    unix_mode: target.unix_data_mode(),
                    source: None,
                });
            }
            assets = legacy_assets;
            start_scene = legacy_start_scene;
            font = legacy_font;
        }

        // ---- 预编译脚本 ----
        let mut script_names: Vec<String> = Vec::new();
        let mut scripts: Vec<(String, Option<PathBuf>, Option<u64>)> = Vec::new();
        let mut script_keys: HashMap<String, String> = HashMap::new();
        for script in &self.input.scripts {
            let raw = script.name.trim();
            let file_name = if raw.to_ascii_lowercase().ends_with(".dll") {
                raw.to_string()
            } else {
                format!("{raw}.dll")
            };
            validate_script_file_name(&file_name).map_err(BuildError::Manifest)?;
            let key = case_collision_key(&file_name);
            if let Some(previous) = script_keys.insert(key, file_name.clone()) {
                return Err(BuildError::DuplicateScript {
                    first: previous,
                    second: file_name,
                });
            }
            let checksum_source = script.source.clone();
            let size = script.size_bytes.or_else(|| {
                script
                    .source
                    .as_deref()
                    .and_then(|source| fs::metadata(source).ok())
                    .map(|meta| meta.len())
            });
            scripts.push((file_name.clone(), checksum_source, size));
            script_names.push(file_name);
        }
        scripts.sort_by(|left, right| left.0.cmp(&right.0));
        script_names.sort();

        // ---- 运行时文件 ----
        let mut runtime_files = self.input.runtime.files.clone();
        runtime_files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        // ---- 可执行文件 ----
        let executable_name = target.executable_file_name(config.executable_stem());
        files.push(PlannedFile {
            relative_path: executable_name,
            category: ArtifactCategory::Executable,
            size_bytes: None,
            checksum: None,
            executable: true,
            unix_mode: target.unix_executable_mode(),
            source: None,
        });

        for (name, source, size) in &scripts {
            files.push(PlannedFile {
                relative_path: format!("{SCRIPTS_DIR}/{name}"),
                category: ArtifactCategory::Script,
                size_bytes: *size,
                checksum: source.as_deref().and_then(checksum_file),
                executable: false,
                unix_mode: target.unix_data_mode(),
                source: source.clone(),
            });
        }

        for runtime_file in &runtime_files {
            files.push(PlannedFile {
                relative_path: format!("{RUNTIME_DIR}/{}", runtime_file.relative_path),
                category: ArtifactCategory::Runtime,
                size_bytes: Some(runtime_file.size_bytes),
                checksum: checksum_file(&runtime_file.source),
                executable: false,
                unix_mode: target.unix_data_mode(),
                source: Some(runtime_file.source.clone()),
            });
        }

        // ---- 清单 ----
        let manifest = PlayerManifest::from_parts(&config, assets, script_names, start_scene, font);
        manifest.validate().map_err(BuildError::Manifest)?;
        let manifest_json = manifest.to_json_pretty().map_err(BuildError::Manifest)?;
        files.push(PlannedFile {
            relative_path: MANIFEST_FILE_NAME.to_string(),
            category: ArtifactCategory::Manifest,
            size_bytes: Some(manifest_json.len() as u64),
            checksum: Some(checksum_bytes(manifest_json.as_bytes())),
            executable: false,
            unix_mode: target.unix_data_mode(),
            source: None,
        });

        // 全清单大小写冲突检测（资产/脚本/运行时/清单之间也不能撞名）。
        let mut full_paths: HashMap<String, String> = HashMap::new();
        for file in &files {
            let key = case_collision_key(&file.relative_path);
            if let Some(previous) = full_paths.insert(key, file.relative_path.clone()) {
                return Err(BuildError::CaseCollision {
                    first: previous,
                    second: file.relative_path.clone(),
                });
            }
        }

        files.sort_by(|left, right| {
            left.category
                .rank()
                .cmp(&right.category.rank())
                .then_with(|| left.relative_path.cmp(&right.relative_path))
        });

        // ---- 警告 ----
        let mut warnings = vec![BuildWarning::ExecutableProducedByCargo];
        if self.input.runtime.is_empty() {
            warnings.push(BuildWarning::RuntimePayloadEmpty);
        }
        if !files
            .iter()
            .any(|file| file.category == ArtifactCategory::Script)
        {
            warnings.push(BuildWarning::ScriptsEmpty);
        }
        if config.required_font.is_none() {
            warnings.push(BuildWarning::FontNotConfigured);
        }
        if target == TargetPlatform::Linux {
            warnings.push(BuildWarning::LinuxRpathRequired);
        }

        let fingerprint = compute_fingerprint(target, &files);

        Ok(PlayerBuildPlan {
            target,
            config,
            manifest,
            files,
            warnings,
            fingerprint,
        })
    }
}

/// 规范化 cook 资产路径：相对 `assets/`，允许调用方误带 `assets/` 前缀。
pub fn normalize_asset_relative(path: &str) -> Result<String, PathError> {
    let normalized = normalize_logical_path(path)?;
    let stripped = normalized
        .strip_prefix(&format!("{ASSETS_DIR}/"))
        .unwrap_or(normalized.as_str());
    if stripped.is_empty() || stripped == ASSETS_DIR {
        return Err(PathError::Empty);
    }
    normalize_logical_path(stripped)
}

/// 构建错误。
#[derive(Debug)]
pub enum BuildError {
    Config(ConfigErrors),
    InvalidStartScene(PathError),
    InvalidAssetPath {
        path: String,
        source: PathError,
    },
    InvalidRuntimePath {
        path: String,
        source: PathError,
    },
    /// hezhou-asset cook manifest 读取/解析失败。
    CookManifest {
        path: PathBuf,
        message: String,
    },
    /// cook manifest 格式版本高于 Player 支持。
    UnsupportedCookFormat {
        found: u32,
        supported: u32,
    },
    /// cook manifest 没有任何条目。
    EmptyCookManifest {
        path: PathBuf,
    },
    /// cook 条目 `output` 非法。
    InvalidCookedOutput {
        output: String,
        source: PathError,
    },
    /// cook 条目 `output` 与保留文件名冲突。
    ReservedCookedOutput {
        output: String,
    },
    /// cook 输出文件缺失。
    MissingCookedOutput {
        output: String,
        path: PathBuf,
    },
    /// cook 原样拷贝资产的产物大小与 `source_size` 不一致。
    CookedSizeMismatch {
        output: String,
        expected: u64,
        actual: u64,
    },
    /// cook 原样拷贝资产的产物哈希与 `source_hash` 不一致。
    CookedChecksumMismatch {
        output: String,
        expected: String,
        actual: String,
    },
    /// 启动场景是子资产条目（Player 只支持文件级场景）。
    InvalidStartSceneLocator {
        locator: String,
    },
    InvalidSourcePath {
        path: String,
        reason: String,
    },
    MissingStartScene {
        path: String,
    },
    MissingFont {
        path: String,
    },
    DuplicateScript {
        first: String,
        second: String,
    },
    CaseCollision {
        first: String,
        second: String,
    },
    Manifest(ManifestError),
    RuntimeSourceMissing {
        path: PathBuf,
    },
    ScriptsSourceMissing {
        path: PathBuf,
    },
    /// 真实构建缺少源文件；`path=None` 表示调用方没有提供来源（如未给 `--exe`）。
    MissingSource {
        logical: String,
        path: Option<PathBuf>,
    },
    /// 源文件大小与计划/清单声明不一致。
    SizeMismatch {
        logical: String,
        expected: u64,
        actual: u64,
    },
    /// 源文件内容校验与计划/清单声明不一致。
    ChecksumMismatch {
        logical: String,
        expected: String,
        actual: String,
    },
    /// 真实构建只允许当前平台（跨平台由 CI 在对应平台执行）。
    CrossPlatformBuildUnsupported {
        target: TargetPlatform,
        host: TargetPlatform,
    },
    /// 输出目录非法（空路径）。
    InvalidOutputDirectory {
        path: PathBuf,
    },
    /// 输出路径已存在但不是目录。
    OutputPathNotDirectory {
        path: PathBuf,
    },
    /// 产物写入后与计划不一致（大小/校验/文件集合）。
    OutputVerification {
        logical: String,
        reason: String,
    },
    Json {
        context: String,
        source: serde_json::Error,
    },
    Io {
        context: String,
        source: std::io::Error,
    },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(errors) => write!(f, "{errors}"),
            Self::InvalidStartScene(source) => write!(f, "start_scene {source}"),
            Self::InvalidAssetPath { path, source } => {
                write!(f, "资产路径 `{path}` 非法：{source}")
            }
            Self::InvalidRuntimePath { path, source } => {
                write!(f, "运行时路径 `{path}` 非法：{source}")
            }
            Self::CookManifest { path, message } => {
                write!(f, "读取 cook manifest {} 失败：{message}", path.display())
            }
            Self::UnsupportedCookFormat { found, supported } => write!(
                f,
                "不支持的 cook manifest 格式版本 {found}（Player 支持 <= {supported}）"
            ),
            Self::EmptyCookManifest { path } => {
                write!(f, "cook manifest 没有资产条目：{}", path.display())
            }
            Self::InvalidCookedOutput { output, source } => {
                write!(f, "cook 输出路径 `{output}` 非法：{source}")
            }
            Self::ReservedCookedOutput { output } => {
                write!(f, "cook 输出路径 `{output}` 与保留文件名冲突")
            }
            Self::MissingCookedOutput { output, path } => write!(
                f,
                "cook 输出 `{output}` 不存在：{}（请先执行 cook_to_dir）",
                path.display()
            ),
            Self::CookedSizeMismatch {
                output,
                expected,
                actual,
            } => write!(
                f,
                "cook 输出 `{output}` 大小 {actual} 与 source_size {expected} 不一致（产物可能损坏/过期）"
            ),
            Self::CookedChecksumMismatch {
                output,
                expected,
                actual,
            } => write!(
                f,
                "cook 输出 `{output}` 哈希 {actual} 与 source_hash {expected} 不一致（产物可能损坏/过期）"
            ),
            Self::InvalidStartSceneLocator { locator } => write!(
                f,
                "启动场景定位符 `{locator}` 指向子资产，Player 只支持文件级场景"
            ),
            Self::InvalidSourcePath { path, reason } => {
                write!(f, "源文件路径 `{path}` 非法：{reason}")
            }
            Self::MissingStartScene { path } => write!(f, "启动场景 `{path}` 不在 cook 资产清单中"),
            Self::MissingFont { path } => write!(f, "必检字体 `{path}` 不在 cook 资产清单中"),
            Self::DuplicateScript { first, second } => write!(
                f,
                "脚本程序集重复（大小写不敏感）：`{first}` 与 `{second}`"
            ),
            Self::CaseCollision { first, second } => write!(
                f,
                "产物路径仅大小写不同（Windows 会互相覆盖）：`{first}` 与 `{second}`"
            ),
            Self::Manifest(source) => write!(f, "清单生成/校验失败：{source}"),
            Self::RuntimeSourceMissing { path } => {
                write!(f, "运行时源目录不存在：{}", path.display())
            }
            Self::ScriptsSourceMissing { path } => {
                write!(f, "脚本源目录不存在：{}", path.display())
            }
            Self::MissingSource { logical, path } => match path {
                Some(path) => write!(
                    f,
                    "产物 `{logical}` 的源文件不存在：{}",
                    path.display()
                ),
                None => write!(
                    f,
                    "缺少产物 `{logical}` 的来源（真实构建需提供 --exe/--assets-dir 或带 source 的输入）"
                ),
            },
            Self::SizeMismatch {
                logical,
                expected,
                actual,
            } => write!(
                f,
                "`{logical}` 大小 {actual} 与计划声明 {expected} 不一致"
            ),
            Self::ChecksumMismatch {
                logical,
                expected,
                actual,
            } => write!(
                f,
                "`{logical}` 校验和 {actual} 与计划声明 {expected} 不一致"
            ),
            Self::CrossPlatformBuildUnsupported { target, host } => write!(
                f,
                "真实构建只能构建当前平台 {host}，请求的是 {target}（跨平台请分别在 CI 平台执行）"
            ),
            Self::InvalidOutputDirectory { path } => {
                write!(f, "输出目录非法（空路径）：{}", path.display())
            }
            Self::OutputPathNotDirectory { path } => {
                write!(f, "输出路径已存在但不是目录：{}", path.display())
            }
            Self::OutputVerification { logical, reason } => {
                write!(f, "产物校验失败 `{logical}`：{reason}")
            }
            Self::Json { context, source } => write!(f, "{context} 失败：{source}"),
            Self::Io { context, source } => write!(f, "{context} 失败：{source}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(errors) => Some(errors),
            Self::InvalidStartScene(source) => Some(source),
            Self::InvalidAssetPath { source, .. } => Some(source),
            Self::InvalidRuntimePath { source, .. } => Some(source),
            Self::Manifest(source) => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// 计算文件内容的 `fnv1a64` 校验值；文件不可读时返回 `None`。
pub fn checksum_file(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Fnv1a64::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(format!("fnv1a64:{:016x}", hasher.finish()))
}

/// 计算字节内容的 `fnv1a64` 校验值。
pub fn checksum_bytes(bytes: &[u8]) -> String {
    let mut hasher = Fnv1a64::new();
    hasher.update(bytes);
    format!("fnv1a64:{:016x}", hasher.finish())
}

fn compute_fingerprint(target: TargetPlatform, files: &[PlannedFile]) -> String {
    let mut hasher = Fnv1a64::new();
    hasher.update(target.as_str().as_bytes());
    hasher.update(b"|");
    for file in files {
        hasher.update(file.category.as_str().as_bytes());
        hasher.update(b"|");
        hasher.update(file.relative_path.as_bytes());
        hasher.update(b"|");
        hasher.update(
            file.size_bytes
                .map(|size| size.to_string())
                .unwrap_or_else(|| "-".to_string())
                .as_bytes(),
        );
        hasher.update(b"|");
        hasher.update(file.checksum.as_deref().unwrap_or("-").as_bytes());
        hasher.update(b"\n");
    }
    format!("fnv1a64:{:016x}", hasher.finish())
}

/// FNV-1a 64 位哈希（无第三方依赖，跨平台结果一致）。
struct Fnv1a64 {
    state: u64,
}

impl Fnv1a64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self {
            state: Self::OFFSET_BASIS,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(Self::PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// 供构建日志展示的引擎版本。
pub fn builder_version() -> &'static str {
    RUNTIME_ENGINE_VERSION
}

// ===========================================================================
// 真实构建（从 dry-run 计划到产物目录）
// ===========================================================================

/// 真实构建的额外输入来源。
///
/// dry-run 只需要元数据；真实复制必须提供可执行文件与资产根目录。
/// 脚本/运行时文件的源路径已记录在 [`ScriptAssembly`] / [`RuntimePayload`] 中。
#[derive(Clone, Debug, Default)]
pub struct BuildSources {
    /// cargo 构建出的 Player 可执行文件（`target/{debug,release}/hezhou-player[.exe]`）。
    pub executable: Option<PathBuf>,
    /// 资产源根目录：`<root>/<asset 相对路径>` 必须存在。
    pub assets_root: Option<PathBuf>,
}

impl BuildSources {
    /// 带可执行文件与资产根目录构造。
    pub fn new(executable: impl Into<PathBuf>, assets_root: impl Into<PathBuf>) -> Self {
        Self {
            executable: Some(executable.into()),
            assets_root: Some(assets_root.into()),
        }
    }
}

/// 已写入产物目录的文件记录。
#[derive(Clone, Debug, Serialize)]
pub struct BuiltFile {
    /// 包内逻辑路径（固定 `/`）。
    pub relative_path: String,
    /// 实际字节数。
    pub size_bytes: u64,
    /// 实际内容校验（`fnv1a64:...`）。
    pub checksum: String,
    /// Unix 权限位（Windows 产物为 `null`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unix_mode: Option<u32>,
    /// 是否可执行。
    pub executable: bool,
}

/// 真实构建结果。
#[derive(Clone, Debug, Serialize)]
pub struct PlayerBuildOutput {
    /// 目标平台（真实构建只允许当前平台）。
    pub target: TargetPlatform,
    /// 产物根目录。
    pub output_dir: PathBuf,
    /// 产物清单路径。
    pub manifest_path: PathBuf,
    /// 已写入文件列表（按计划顺序）。
    pub files: Vec<BuiltFile>,
    /// 已写入总字节数。
    pub total_bytes: u64,
    /// 与 dry-run 计划一致的校验指纹。
    pub fingerprint: String,
}

impl PlayerBuildOutput {
    /// 按逻辑路径查找已写入文件。
    pub fn file(&self, relative_path: &str) -> Option<&BuiltFile> {
        self.files
            .iter()
            .find(|file| file.relative_path == relative_path)
    }

    /// 人类可读的构建报告。
    pub fn report(&self) -> String {
        let mut out = String::new();
        out.push_str("Player 真实构建完成\n");
        out.push_str(&format!("  目标平台: {}\n", self.target.display_name()));
        out.push_str(&format!("  产物目录: {}\n", self.output_dir.display()));
        out.push_str(&format!("  清单    : {}\n", self.manifest_path.display()));
        out.push_str(&format!(
            "  统计    : {} 个文件，共 {}\n",
            self.files.len(),
            human_size(self.total_bytes)
        ));
        out.push_str(&format!("  指纹    : {}\n", self.fingerprint));
        out.push_str("文件:\n");
        for file in &self.files {
            out.push_str(&format!(
                "  {:<58} {:>10}  {:<10} {}\n",
                file.relative_path,
                human_size(file.size_bytes),
                file.unix_mode
                    .map(|mode| format!("{mode:04o}"))
                    .unwrap_or_else(|| "-".to_string()),
                file.checksum,
            ));
        }
        out
    }
}

impl fmt::Display for PlayerBuildOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.report())
    }
}

/// staged 输出文件的来源（预检后确定）。
struct PreparedCopy<'a> {
    plan_file: &'a PlannedFile,
    /// 源文件；`None` 表示由构建器生成（`manifest.json`）。
    source: Option<PathBuf>,
    /// 生成的内容（仅清单）。
    generated: Option<Vec<u8>>,
    size_bytes: u64,
    checksum: String,
}

/// staging 目录守卫：未提交时自动清理，保证失败不留半成品。
struct StagingGuard {
    path: PathBuf,
    committed: bool,
}

impl StagingGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

impl PlayerBuilder {
    /// 按 plan 真实复制 exe / assets / scripts / runtime / manifest 到 `output_dir`。
    ///
    /// 行为保证：
    /// - 只允许构建当前平台（跨平台构建由 CI 在对应平台执行）；
    /// - 先完整预检（源文件存在、大小、`fnv1a64` 校验和），再写 staging 目录；
    /// - 成功后原子替换 `output_dir`；失败自动清理 staging，不破坏既有产物；
    /// - 写盘后校验目标文件大小/校验和，并核对文件集合与计划一致；
    /// - Linux 下按计划的 `unix_mode` 设置权限（exe 0755，数据 0644）。
    pub fn build(
        &self,
        target: TargetPlatform,
        sources: &BuildSources,
        output_dir: &Path,
    ) -> Result<PlayerBuildOutput, BuildError> {
        let host = TargetPlatform::host();
        if target != host {
            return Err(BuildError::CrossPlatformBuildUnsupported { target, host });
        }
        if output_dir.as_os_str().is_empty() {
            return Err(BuildError::InvalidOutputDirectory {
                path: output_dir.to_path_buf(),
            });
        }
        if output_dir.exists() && !output_dir.is_dir() {
            return Err(BuildError::OutputPathNotDirectory {
                path: output_dir.to_path_buf(),
            });
        }

        // 1) 生成计划（复用 dry-run 逻辑，双平台 plan_for 仍可用）。
        let plan = self.plan_for(target)?;

        // 2) 完整预检：所有源文件存在、大小/校验和匹配。失败时不创建任何目录。
        let prepared = prepare_copies(&plan, sources)?;

        // 3) staging 目录（与 output 同父目录，保证 rename 同卷原子）。
        let parent = output_parent(output_dir);
        fs::create_dir_all(parent).map_err(|source| BuildError::Io {
            context: format!("创建输出父目录 {}", parent.display()),
            source,
        })?;
        let staging_path = staging_path(output_dir);
        fs::create_dir_all(&staging_path).map_err(|source| BuildError::Io {
            context: format!("创建 staging 目录 {}", staging_path.display()),
            source,
        })?;
        let mut guard = StagingGuard::new(staging_path.clone());

        // 4) 复制 + 单文件校验 + 权限。
        let mut built_files = Vec::with_capacity(prepared.len());
        for copy in &prepared {
            let destination = copy_one(copy, &staging_path, target)?;
            built_files.push(BuiltFile {
                relative_path: copy.plan_file.relative_path.clone(),
                size_bytes: copy.size_bytes,
                checksum: copy.checksum.clone(),
                unix_mode: copy.plan_file.unix_mode,
                executable: copy.plan_file.executable,
            });
            // 保留 destination 变量仅用于调试信息（如未来失败日志）。
            let _ = destination;
        }

        // 5) 核对产物文件集合与计划完全一致（防止漏拷/多拷）。
        verify_tree(&staging_path, &plan)?;

        // 6) 提交：替换旧产物目录（如果存在），rename staging。
        if output_dir.exists() {
            fs::remove_dir_all(output_dir).map_err(|source| BuildError::Io {
                context: format!("清理旧产物目录 {}", output_dir.display()),
                source,
            })?;
        }
        // Windows 上目录 rename 可能被杀毒/索引服务短暂占用（PermissionDenied），
        // 重试几次；仍失败才返回错误（staging guard 会清理临时目录）。
        rename_with_retry(&staging_path, output_dir).map_err(|source| BuildError::Io {
            context: format!(
                "提交产物目录 {} -> {}",
                staging_path.display(),
                output_dir.display()
            ),
            source,
        })?;
        guard.commit();

        let total_bytes = built_files.iter().map(|file| file.size_bytes).sum();
        Ok(PlayerBuildOutput {
            target,
            output_dir: output_dir.to_path_buf(),
            manifest_path: output_dir.join(MANIFEST_FILE_NAME),
            files: built_files,
            total_bytes,
            fingerprint: plan.fingerprint,
        })
    }
}

/// 预检所有输入源，返回可复制的文件列表。任何问题在写盘前报错。
fn prepare_copies<'a>(
    plan: &'a PlayerBuildPlan,
    sources: &BuildSources,
) -> Result<Vec<PreparedCopy<'a>>, BuildError> {
    let assets_prefix = format!("{}/", plan.manifest.paths.assets.trim_end_matches('/'));
    let mut prepared = Vec::with_capacity(plan.files.len());

    for file in &plan.files {
        if file.category == ArtifactCategory::Manifest {
            let bytes = plan
                .manifest
                .to_json_pretty()
                .map_err(BuildError::Manifest)?
                .into_bytes();
            let checksum = checksum_bytes(&bytes);
            if let Some(expected) = &file.checksum {
                if expected != &checksum {
                    return Err(BuildError::ChecksumMismatch {
                        logical: file.relative_path.clone(),
                        expected: expected.clone(),
                        actual: checksum,
                    });
                }
            }
            prepared.push(PreparedCopy {
                plan_file: file,
                source: None,
                generated: Some(bytes.clone()),
                size_bytes: bytes.len() as u64,
                checksum,
            });
            continue;
        }

        // 优先使用计划中记录的来源（真实 cook 输出 / 脚本 / 运行时 / 元数据）；
        // legacy 资产清单没有源路径，需要调用方提供 `assets_root`。
        let source = if let Some(source) = file.source.clone() {
            source
        } else {
            match file.category {
                ArtifactCategory::Executable => sources.executable.clone().ok_or_else(|| {
                    BuildError::MissingSource {
                        logical: file.relative_path.clone(),
                        path: None,
                    }
                })?,
                ArtifactCategory::Asset => {
                    let relative = file
                        .relative_path
                        .strip_prefix(&assets_prefix)
                        .ok_or_else(|| BuildError::OutputVerification {
                            logical: file.relative_path.clone(),
                            reason: format!("资产路径必须以 `{assets_prefix}` 开头"),
                        })?;
                    let root =
                        sources
                            .assets_root
                            .clone()
                            .ok_or_else(|| BuildError::MissingSource {
                                logical: file.relative_path.clone(),
                                path: None,
                            })?;
                    root.join(to_native_path(relative, plan.target))
                }
                other => {
                    return Err(BuildError::MissingSource {
                        logical: format!("{}（分类 {}）", file.relative_path, other.as_str()),
                        path: None,
                    });
                }
            }
        };

        if !source.is_file() {
            return Err(BuildError::MissingSource {
                logical: file.relative_path.clone(),
                path: Some(source),
            });
        }
        let actual_size = fs::metadata(&source)
            .map_err(|source_error| BuildError::Io {
                context: format!("读取源文件元数据 {}", source.display()),
                source: source_error,
            })?
            .len();
        if let Some(expected) = file.size_bytes {
            if expected != actual_size {
                return Err(BuildError::SizeMismatch {
                    logical: file.relative_path.clone(),
                    expected,
                    actual: actual_size,
                });
            }
        }
        let checksum = checksum_file(&source).ok_or_else(|| BuildError::MissingSource {
            logical: file.relative_path.clone(),
            path: Some(source.clone()),
        })?;
        if let Some(expected) = &file.checksum {
            // cook 清单可能使用其它算法（如 sha256），只对比本 crate 能计算的格式。
            if expected.starts_with("fnv1a64:") && expected != &checksum {
                return Err(BuildError::ChecksumMismatch {
                    logical: file.relative_path.clone(),
                    expected: expected.clone(),
                    actual: checksum,
                });
            }
        }
        prepared.push(PreparedCopy {
            plan_file: file,
            source: Some(source),
            generated: None,
            size_bytes: actual_size,
            checksum,
        });
    }

    Ok(prepared)
}

/// 复制单个文件并校验目标大小/校验和，返回目标路径。
fn copy_one(
    copy: &PreparedCopy,
    staging_root: &Path,
    target: TargetPlatform,
) -> Result<PathBuf, BuildError> {
    let destination = staging_root.join(to_native_path(&copy.plan_file.relative_path, target));
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| BuildError::Io {
            context: format!("创建产物子目录 {}", parent.display()),
            source,
        })?;
    }

    if let Some(bytes) = &copy.generated {
        fs::write(&destination, bytes).map_err(|source| BuildError::Io {
            context: format!("写入生成文件 {}", destination.display()),
            source,
        })?;
    } else if let Some(source) = &copy.source {
        fs::copy(source, &destination).map_err(|source_error| BuildError::Io {
            context: format!(
                "复制 {} -> {} 失败",
                source.display(),
                destination.display()
            ),
            source: source_error,
        })?;
    } else {
        return Err(BuildError::OutputVerification {
            logical: copy.plan_file.relative_path.clone(),
            reason: "既无源文件也无生成内容".to_string(),
        });
    }

    apply_permissions(&destination, copy.plan_file.unix_mode)?;

    let actual_size = fs::metadata(&destination)
        .map_err(|source| BuildError::Io {
            context: format!("读取产物元数据 {}", destination.display()),
            source,
        })?
        .len();
    if actual_size != copy.size_bytes {
        return Err(BuildError::OutputVerification {
            logical: copy.plan_file.relative_path.clone(),
            reason: format!(
                "产物大小 {} 与计划 {} 不一致",
                actual_size, copy.size_bytes
            ),
        });
    }
    let actual_checksum = checksum_file(&destination).ok_or_else(|| BuildError::OutputVerification {
        logical: copy.plan_file.relative_path.clone(),
        reason: "无法读取产物内容做校验".to_string(),
    })?;
    if actual_checksum != copy.checksum {
        return Err(BuildError::OutputVerification {
            logical: copy.plan_file.relative_path.clone(),
            reason: format!(
                "产物校验和 {} 与预期 {} 不一致",
                actual_checksum, copy.checksum
            ),
        });
    }
    Ok(destination)
}

/// 按计划设置 Unix 权限位（Windows 无 Unix 权限，跳过）。
fn apply_permissions(path: &Path, unix_mode: Option<u32>) -> Result<(), BuildError> {
    #[cfg(unix)]
    {
        if let Some(mode) = unix_mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|source| {
                BuildError::Io {
                    context: format!("设置文件权限 {} {:04o}", path.display(), mode),
                    source,
                }
            })?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (path, unix_mode);
    }
    Ok(())
}

/// 核对 staging 目录内文件集合与计划完全一致。
fn verify_tree(staging_root: &Path, plan: &PlayerBuildPlan) -> Result<(), BuildError> {
    let mut actual = Vec::new();
    collect_tree(staging_root, "", &mut actual).map_err(|source| BuildError::Io {
        context: format!("遍历 staging 目录 {}", staging_root.display()),
        source,
    })?;
    actual.sort();

    let mut expected: Vec<String> = plan
        .files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect();
    expected.sort();

    if actual != expected {
        return Err(BuildError::OutputVerification {
            logical: "<tree>".to_string(),
            reason: format!(
                "产物文件集合与计划不一致：计划 {} 个，实际 {} 个（多出/缺少请见清单）",
                expected.len(),
                actual.len()
            ),
        });
    }
    Ok(())
}

fn collect_tree(
    dir: &Path,
    prefix: &str,
    out: &mut Vec<String>,
) -> Result<(), std::io::Error> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in entries {
        let name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            collect_tree(&path, &relative, out)?;
        } else if metadata.is_file() {
            out.push(relative);
        }
    }
    Ok(())
}

fn output_parent(output_dir: &Path) -> &Path {
    output_dir
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn staging_path(output_dir: &Path) -> PathBuf {
    let name = output_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "player".to_string());
    let counter = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    output_parent(output_dir).join(format!(
        ".{name}.staging-{}-{counter}",
        std::process::id()
    ))
}

/// 目录 rename 的小重试封装（Windows 杀毒/索引服务可能短暂占用刚写入的目录）。
fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last_error = None;
    for attempt in 0u64..5 {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
                }
            }
        }
    }
    Err(last_error.expect("至少尝试过一次 rename"))
}
