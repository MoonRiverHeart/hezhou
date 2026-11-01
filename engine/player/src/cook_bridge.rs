//! hezhou-asset 真实 cook 输出 → Player 资产/产物映射。
//!
//! 契约来源：`engine/asset/README.md` §6（`format_version = 1`）。
//!
//! # 输入
//!
//! `<cook_out>/cook_manifest.json` + `<cook_out>/imported/*.json` + `<cook_out>/assets/*`，
//! 由 `hezhou_asset::Cooker::plan` / `cook_to_dir` 产出。
//!
//! # 输出映射
//!
//! ```text
//! <cook_out>/cook_manifest.json   -> <PlayerRoot>/assets/cook_manifest.json
//! <cook_out>/imported/<uuid>.json -> <PlayerRoot>/assets/imported/<uuid>.json
//! <cook_out>/assets/<uuid>.<ext>  -> <PlayerRoot>/assets/assets/<uuid>.<ext>
//! ```
//!
//! Player `manifest.json` 的 `assets[]` 与 cook `entries[]` 一一对应：
//! `path == output`，并保留 `asset_id`（AssetId）、`locator`、`dependencies`、
//! `source_hash`、`source_size`，以及 cook 输出文件的 `checksum`（fnv1a64）。
//! 运行时以 AssetId 为键索引，不依赖逻辑路径拼接。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use hezhou_asset::cook::{
    CookEntry, CookManifest, COOK_MANIFEST_FILE, COOK_MANIFEST_FORMAT_VERSION,
};
use hezhou_asset::AssetKind;

use crate::builder::{checksum_file, ArtifactCategory, BuildError, PlannedFile};
use crate::config::PlayerBuildConfig;
use crate::manifest::{ManifestAsset, ASSETS_DIR};
use crate::paths::{case_collision_key, normalize_logical_path, to_native_path};
use crate::platform::TargetPlatform;

/// 真实 cook 输出（manifest + 产物根目录）。
///
/// `output_root` 是 `cook_to_dir` 的输出目录，因此 `imported/`、`assets/` 与
/// `cook_manifest.json` 都在其下。Player 构建按此目录复制产物。
#[derive(Clone, Debug)]
pub struct CookedAssets {
    /// hezhou-asset 的 cook manifest（复用其类型，不另写 schema）。
    pub manifest: CookManifest,
    /// cook 输出根目录。
    pub output_root: PathBuf,
}

impl CookedAssets {
    /// 读取 `<cook_out>/cook_manifest.json`，产物根目录取清单所在目录。
    pub fn load(manifest_path: &Path) -> Result<Self, BuildError> {
        let manifest =
            CookManifest::load(manifest_path).map_err(|error| BuildError::CookManifest {
                path: manifest_path.to_path_buf(),
                message: error.to_string(),
            })?;
        let output_root = manifest_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(Self {
            manifest,
            output_root,
        })
    }

    /// 直接指定清单与产物根目录（测试/上层集成用）。
    pub fn from_parts(manifest: CookManifest, output_root: impl Into<PathBuf>) -> Self {
        Self {
            manifest,
            output_root: output_root.into(),
        }
    }

    /// 预期清单文件路径。
    pub fn manifest_path(&self) -> PathBuf {
        self.output_root.join(COOK_MANIFEST_FILE)
    }

    /// `output` 逻辑路径对应的产出文件本机路径。
    pub fn output_path(&self, output: &str) -> PathBuf {
        self.output_root
            .join(to_native_path(output, TargetPlatform::host()))
    }
}

/// 映射结果，供 `PlayerBuilder::plan_for` 合并计划。
pub(crate) struct CookedPlan {
    /// Player manifest 资产条目（按 path 排序）。
    pub(crate) assets: Vec<ManifestAsset>,
    /// 真实产物文件（cook 资产 + cook_manifest.json 元数据）。
    pub(crate) files: Vec<PlannedFile>,
    /// 启动场景在 Player 包 `assets/` 下的路径（= 对应条目 output）。
    pub(crate) start_scene: String,
    /// 必检字体在 Player 包 `assets/` 下的路径（= 对应条目 output）。
    pub(crate) font: Option<String>,
}

/// 把真实 cook 输出映射为 Player 资产清单与文件计划。
///
/// 校验规则：
/// - `format_version <= 1`、entries 非空、`output` 为合法逻辑路径且无大小写冲突；
/// - 每个 `output` 文件必须存在；普通资产（非 glTF/子资产）额外校验
///   `source_size`/`source_hash`（cook 原样拷贝语义，可发现 cook 产物损坏）；
/// - 启动场景/字体必须能在 `entries` 中按 `locator` 找到（工程相对路径匹配）。
pub(crate) fn map_cooked_assets(
    cooked: &CookedAssets,
    config: &PlayerBuildConfig,
    target: TargetPlatform,
) -> Result<CookedPlan, BuildError> {
    let manifest = &cooked.manifest;
    if manifest.format_version > COOK_MANIFEST_FORMAT_VERSION {
        return Err(BuildError::UnsupportedCookFormat {
            found: manifest.format_version,
            supported: COOK_MANIFEST_FORMAT_VERSION,
        });
    }
    if manifest.entries.is_empty() {
        return Err(BuildError::EmptyCookManifest {
            path: cooked.manifest_path(),
        });
    }
    let manifest_source = cooked.manifest_path();
    if !manifest_source.is_file() {
        return Err(BuildError::CookManifest {
            path: manifest_source.clone(),
            message: "cook_manifest.json 不存在".to_string(),
        });
    }

    // 条目输出路径：规范化 + 保留名/冲突检查。
    let mut assets = Vec::with_capacity(manifest.entries.len());
    let mut files = Vec::with_capacity(manifest.entries.len() + 1);
    let mut seen_outputs: HashMap<String, String> = HashMap::new();
    for entry in &manifest.entries {
        let output = normalize_logical_path(&entry.output).map_err(|source| {
            BuildError::InvalidCookedOutput {
                output: entry.output.clone(),
                source,
            }
        })?;
        if output.eq_ignore_ascii_case(COOK_MANIFEST_FILE) {
            return Err(BuildError::ReservedCookedOutput { output });
        }
        let key = case_collision_key(&output);
        if let Some(previous) = seen_outputs.insert(key, output.clone()) {
            return Err(BuildError::CaseCollision {
                first: previous,
                second: output,
            });
        }

        let source = cooked.output_path(&output);
        let (actual_size, actual_checksum) = inspect_cooked_output(&output, &source)?;
        let actual_hash = actual_checksum
            .strip_prefix("fnv1a64:")
            .unwrap_or(actual_checksum.as_str())
            .to_string();

        // cook 对非 glTF/子资产是原样拷贝，source_size/source_hash 应与产物一致；
        // imported/*.json 是重新生成的 JSON，不能用源文件大小/哈希校验。
        if !is_generated_output(entry) {
            if actual_size != entry.source_size {
                return Err(BuildError::CookedSizeMismatch {
                    output: output.clone(),
                    expected: entry.source_size,
                    actual: actual_size,
                });
            }
            if !actual_hash.eq_ignore_ascii_case(&entry.source_hash) {
                return Err(BuildError::CookedChecksumMismatch {
                    output: output.clone(),
                    expected: entry.source_hash.clone(),
                    actual: actual_hash,
                });
            }
        }

        let mut asset = ManifestAsset::new(output.clone(), entry.kind.as_str())
            .with_size(actual_size)
            .with_checksum(actual_checksum.clone())
            .with_cook_metadata(
                entry.id.to_string(),
                entry.locator.to_string(),
                output.clone(),
                entry.dependencies.iter().map(ToString::to_string).collect(),
                entry.source_hash.clone(),
                entry.source_size,
            );
        // 保持与 cook manifest 的 `output` 字段原样一致（便于对照）。
        asset.output = Some(entry.output.clone());
        assets.push(asset);

        files.push(PlannedFile {
            relative_path: format!("{ASSETS_DIR}/{output}"),
            category: ArtifactCategory::Asset,
            size_bytes: Some(actual_size),
            checksum: Some(actual_checksum),
            executable: false,
            unix_mode: target.unix_data_mode(),
            source: Some(source),
        });
    }
    assets.sort_by(|left, right| left.path.cmp(&right.path));

    // cook_manifest.json 原样复制到 Player 包 assets/ 下，供运行时/诊断索引资产。
    let (manifest_size, manifest_checksum) =
        inspect_cooked_output(COOK_MANIFEST_FILE, &manifest_source)?;
    files.push(PlannedFile {
        relative_path: format!("{ASSETS_DIR}/{COOK_MANIFEST_FILE}"),
        category: ArtifactCategory::Metadata,
        size_bytes: Some(manifest_size),
        checksum: Some(manifest_checksum),
        executable: false,
        unix_mode: target.unix_data_mode(),
        source: Some(manifest_source),
    });

    // 启动场景：按工程相对 locator 在 cook 条目中定位，Player manifest 记录 output。
    let start_entry = find_entry_by_project_path(manifest, &config.start_scene).ok_or_else(|| {
        BuildError::MissingStartScene {
            path: config.start_scene.clone(),
        }
    })?;
    if start_entry.locator.is_sub_asset() {
        return Err(BuildError::InvalidStartSceneLocator {
            locator: start_entry.locator.to_string(),
        });
    }
    let start_scene =
        normalize_logical_path(&start_entry.output).map_err(|source| {
            BuildError::InvalidCookedOutput {
                output: start_entry.output.clone(),
                source,
            }
        })?;

    let font = match config.required_font.as_deref() {
        Some(font) => {
            let entry = find_entry_by_project_path(manifest, font).ok_or_else(|| {
                BuildError::MissingFont {
                    path: font.to_string(),
                }
            })?;
            Some(
                normalize_logical_path(&entry.output).map_err(|source| {
                    BuildError::InvalidCookedOutput {
                        output: entry.output.clone(),
                        source,
                    }
                })?,
            )
        }
        None => None,
    };

    Ok(CookedPlan {
        assets,
        files,
        start_scene,
        font,
    })
}

/// 校验 cook 输出文件存在，返回 `(实际大小, fnv1a64 校验)`。
fn inspect_cooked_output(output: &str, source: &Path) -> Result<(u64, String), BuildError> {
    if !source.is_file() {
        return Err(BuildError::MissingCookedOutput {
            output: output.to_string(),
            path: source.to_path_buf(),
        });
    }
    let size = std::fs::metadata(source)
        .map_err(|error| BuildError::Io {
            context: format!("读取 cook 输出元数据 {}", source.display()),
            source: error,
        })?
        .len();
    let checksum = checksum_file(source).ok_or_else(|| BuildError::MissingCookedOutput {
        output: output.to_string(),
        path: source.to_path_buf(),
    })?;
    Ok((size, checksum))
}

/// cook 输出是否为重新生成的导入产物（imported/*.json），而非原样拷贝。
fn is_generated_output(entry: &CookEntry) -> bool {
    entry.kind == AssetKind::Gltf || entry.locator.is_sub_asset()
}

/// 按工程相对逻辑路径查找文件级 cook 条目。
///
/// 兼容调用方传 `scenes/main.scene` 或 `assets/scenes/main.scene`；
/// 比较大小写不敏感（asset 契约），忽略子资产条目。
pub fn find_entry_by_project_path<'a>(
    manifest: &'a CookManifest,
    project_relative: &str,
) -> Option<&'a CookEntry> {
    let target = normalize_logical_path(project_relative).ok()?;
    let target_key = target.to_lowercase();
    let target_with_assets = format!("{ASSETS_DIR}/{target_key}");
    manifest.entries.iter().find(|entry| {
        if entry.locator.is_sub_asset() {
            return false;
        }
        let locator_key = entry.locator.path().key();
        locator_key == target_key || locator_key == target_with_assets
    })
}
