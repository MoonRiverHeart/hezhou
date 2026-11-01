//! Player 包内资产索引：读取 `assets/cook_manifest.json`，按 `AssetId` 建立索引。
//!
//! 契约来源：`engine/asset/README.md` §6。Player 构建器（`cook_bridge`）把
//! `cook_manifest.json + imported/ + assets/` 原样放入包内 `assets/`；运行时启动时
//! 加载该清单，建立：
//!
//! - `AssetId -> AssetIndexEntry { output/kind/locator/dependencies/source_* }`
//! - `start_scene` / `font` 对应的 `AssetId`（用 Player manifest 中的 output 匹配）
//!
//! 并校验每个 `output` 文件在包内存在。
//!
//! 兼容：没有 `assets/cook_manifest.json` 的 legacy 包（无 asset_id）会得到空索引，
//! 仅告警，不阻塞 `--validate`；有 asset_id 的 legacy 包会按 Player manifest 建索引。

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};

use hezhou_asset::cook::{CookManifest, COOK_MANIFEST_FILE};
use hezhou_asset::{AssetId, AssetKind, AssetLocator};

use crate::manifest::PlayerManifest;
use crate::paths::{normalize_logical_path, to_native_path, PathError};
use crate::platform::TargetPlatform;

/// 资产索引中的一条记录。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIndexEntry {
    /// 稳定 AssetId。
    pub id: AssetId,
    /// 资产类型。
    pub kind: AssetKind,
    /// 源定位符（工程相对逻辑路径 + 可选子资产键）；legacy 包可能没有。
    pub locator: Option<AssetLocator>,
    /// Player 包内相对 `assets/` 的路径（= cook `output`）。
    pub output: String,
    /// 直接依赖。
    pub dependencies: Vec<AssetId>,
    /// 源文件字节数。
    pub source_size: u64,
    /// 源文件 FNV-1a 64 hex。
    pub source_hash: String,
}

/// Player 启动时可用的资产索引。
#[derive(Clone, Debug)]
pub struct PlayerAssetIndex {
    target: TargetPlatform,
    entries: BTreeMap<AssetId, AssetIndexEntry>,
    /// 小写 output -> AssetId（跨平台大小写安全查询）。
    outputs: HashMap<String, AssetId>,
    start_scene: Option<AssetId>,
    font: Option<AssetId>,
    cook_manifest_path: Option<PathBuf>,
}

impl PlayerAssetIndex {
    /// 从 Player 包根目录加载索引。
    ///
    /// - 有 `assets/cook_manifest.json`：复用 [`CookManifest`] 解析，校验 output 存在，
    ///   并核对 Player manifest 的 assets[] 与索引一致；
    /// - 没有该文件：按 Player manifest 的 `asset_id` 建索引（legacy 兼容）；
    /// - 两者都没有 asset_id：空索引（仅告警，不阻塞 `--validate`）。
    pub fn load(root: &Path, manifest: &PlayerManifest) -> Result<Self, AssetIndexError> {
        let assets_dir = manifest.assets_dir(root);
        let cook_path = assets_dir.join(COOK_MANIFEST_FILE);
        if cook_path.is_file() {
            let cook = CookManifest::load(&cook_path).map_err(|error| {
                AssetIndexError::CookManifest {
                    path: cook_path.clone(),
                    message: error.to_string(),
                }
            })?;
            let mut index = Self::from_cook_manifest(&cook, manifest)?;
            index.cook_manifest_path = Some(cook_path);
            index.verify_outputs(&assets_dir)?;
            index.verify_manifest_consistency(manifest)?;
            Ok(index)
        } else {
            let index = Self::from_player_manifest(manifest)?;
            index.verify_outputs(&assets_dir)?;
            Ok(index)
        }
    }

    /// 由已解析的 cook manifest + Player manifest 建索引（纯函数，测试可用）。
    pub fn from_cook_manifest(
        cook: &CookManifest,
        manifest: &PlayerManifest,
    ) -> Result<Self, AssetIndexError> {
        if cook.entries.is_empty() {
            return Err(AssetIndexError::EmptyManifest {
                context: "cook manifest 没有资产条目".to_string(),
            });
        }
        let mut builder = IndexBuilder::new(manifest.target);
        for entry in &cook.entries {
            let output = normalize_logical_path(&entry.output).map_err(|source| {
                AssetIndexError::InvalidOutputPath {
                    output: entry.output.clone(),
                    source,
                }
            })?;
            builder.push(AssetIndexEntry {
                id: entry.id,
                kind: entry.kind,
                locator: Some(entry.locator.clone()),
                output,
                dependencies: entry.dependencies.clone(),
                source_size: entry.source_size,
                source_hash: entry.source_hash.clone(),
            })?;
        }
        builder.finish(manifest)
    }

    /// 由 Player manifest（legacy 包，无 cook_manifest.json）建索引。
    pub fn from_player_manifest(manifest: &PlayerManifest) -> Result<Self, AssetIndexError> {
        let mut builder = IndexBuilder::new(manifest.target);
        for asset in &manifest.assets {
            let Some(raw_id) = asset.asset_id.as_deref() else {
                continue;
            };
            let id = AssetId::parse(raw_id).map_err(|error| AssetIndexError::InvalidAssetId {
                raw: raw_id.to_string(),
                message: error.to_string(),
            })?;
            let output = normalize_logical_path(asset.output.as_deref().unwrap_or(&asset.path))
                .map_err(|source| AssetIndexError::InvalidOutputPath {
                    output: asset.path.clone(),
                    source,
                })?;
            let locator = asset
                .locator
                .as_deref()
                .map(AssetLocator::parse)
                .transpose()
                .map_err(|error| AssetIndexError::InvalidLocator {
                    raw: asset.locator.clone().unwrap_or_default(),
                    message: error.to_string(),
                })?;
            let kind = AssetKind::from_name(asset.kind.trim()).unwrap_or(AssetKind::Other);
            builder.push(AssetIndexEntry {
                id,
                kind,
                locator,
                output,
                dependencies: Vec::new(),
                source_size: asset.source_size.unwrap_or(0),
                source_hash: asset.source_hash.clone().unwrap_or_default(),
            })?;
        }
        builder.finish(manifest)
    }

    /// 校验所有索引条目的 `output` 文件存在。
    pub fn verify_outputs(&self, assets_dir: &Path) -> Result<(), AssetIndexError> {
        for entry in self.entries.values() {
            let path = assets_dir.join(to_native_path(&entry.output, self.target));
            if !path.is_file() {
                return Err(AssetIndexError::MissingOutput {
                    output: entry.output.clone(),
                    path,
                });
            }
        }
        Ok(())
    }

    /// 核对 Player manifest 的 assets[] 与 cook 索引一一对应（防止清单过期/被篡改）。
    pub fn verify_manifest_consistency(
        &self,
        manifest: &PlayerManifest,
    ) -> Result<(), AssetIndexError> {
        if self.entries.is_empty() {
            return Ok(());
        }
        let mut seen: HashMap<&str, &AssetIndexEntry> = HashMap::new();
        for asset in &manifest.assets {
            let Some(raw_id) = asset.asset_id.as_deref() else {
                continue;
            };
            let id = AssetId::parse(raw_id).map_err(|error| AssetIndexError::InvalidAssetId {
                raw: raw_id.to_string(),
                message: error.to_string(),
            })?;
            let entry = self.entries.get(&id).ok_or_else(|| {
                AssetIndexError::ManifestIndexMismatch {
                    output: asset.path.clone(),
                    reason: format!("Player manifest 声明 AssetId {id}，但 cook 索引中不存在"),
                }
            })?;
            let asset_output = normalize_logical_path(
                asset.output.as_deref().unwrap_or(&asset.path),
            )
            .unwrap_or_else(|_| asset.path.clone());
            if asset_output != entry.output {
                return Err(AssetIndexError::ManifestIndexMismatch {
                    output: asset.path.clone(),
                    reason: format!(
                        "Player manifest output `{asset_output}` 与 cook 索引 `{}` 不一致",
                        entry.output
                    ),
                });
            }
            if !asset.kind.trim().eq_ignore_ascii_case(entry.kind.as_str()) {
                return Err(AssetIndexError::ManifestIndexMismatch {
                    output: asset.path.clone(),
                    reason: format!(
                        "Player manifest kind `{}` 与 cook 索引 `{}` 不一致",
                        asset.kind,
                        entry.kind.as_str()
                    ),
                });
            }
            seen.insert(entry.output.as_str(), entry);
        }
        for entry in self.entries.values() {
            if !seen.contains_key(entry.output.as_str()) {
                return Err(AssetIndexError::ManifestIndexMismatch {
                    output: entry.output.clone(),
                    reason: "cook 索引中的资产未出现在 Player manifest assets[] 中".to_string(),
                });
            }
        }
        Ok(())
    }

    /// 按 AssetId 查询。
    pub fn entry(&self, id: AssetId) -> Option<&AssetIndexEntry> {
        self.entries.get(&id)
    }

    /// 按包内 output 路径查询（大小写不敏感）。
    pub fn entry_by_output(&self, output: &str) -> Option<&AssetIndexEntry> {
        let key = normalize_logical_path(output)
            .map(|normalized| normalized.to_lowercase())
            .unwrap_or_else(|_| output.to_lowercase());
        self.outputs
            .get(&key)
            .and_then(|id| self.entries.get(id))
    }

    /// 按 kind 查询（按 output 排序，保证确定性）。
    pub fn by_kind(&self, kind: AssetKind) -> Vec<&AssetIndexEntry> {
        let mut entries: Vec<&AssetIndexEntry> = self
            .entries
            .values()
            .filter(|entry| entry.kind == kind)
            .collect();
        entries.sort_by(|left, right| left.output.cmp(&right.output));
        entries
    }

    /// 按 locator 精确查询（大小写不敏感；`assets/...` 与 `...#mesh:0` 形式均可）。
    pub fn by_locator(&self, locator: &str) -> Vec<&AssetIndexEntry> {
        self.entries
            .values()
            .filter(|entry| {
                entry
                    .locator
                    .as_ref()
                    .map(|candidate| {
                        candidate
                            .to_string()
                            .eq_ignore_ascii_case(locator.trim())
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    /// 按 locator 查单条（同路径可能有多个子资产时返回第一个，按 AssetId 排序）。
    pub fn entry_by_locator(&self, locator: &str) -> Option<&AssetIndexEntry> {
        self.by_locator(locator).into_iter().next()
    }

    /// 遍历所有条目（按 AssetId 排序）。
    pub fn entries(&self) -> impl Iterator<Item = &AssetIndexEntry> {
        self.entries.values()
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空索引（legacy 包）。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 是否从包内 `cook_manifest.json` 加载。
    pub fn has_cook_manifest(&self) -> bool {
        self.cook_manifest_path.is_some()
    }

    /// `cook_manifest.json` 路径（若有）。
    pub fn cook_manifest_path(&self) -> Option<&Path> {
        self.cook_manifest_path.as_deref()
    }

    /// 启动场景条目（由 Player manifest.start_scene 匹配 output）。
    pub fn start_scene(&self) -> Option<&AssetIndexEntry> {
        self.start_scene.and_then(|id| self.entries.get(&id))
    }

    /// 必检字体条目（由 Player manifest.font 匹配 output）。
    pub fn font(&self) -> Option<&AssetIndexEntry> {
        self.font.and_then(|id| self.entries.get(&id))
    }

    /// 启动日志用的一行摘要。
    pub fn summary(&self) -> String {
        format!(
            "资产索引 {} 条（来源：{}）",
            self.entries.len(),
            if self.has_cook_manifest() {
                "包内 cook_manifest.json"
            } else {
                "Player manifest（legacy/无 cook 清单）"
            }
        )
    }
}

/// 索引构建器：统一处理重复 ID/output 与 start_scene/font 解析。
struct IndexBuilder {
    target: TargetPlatform,
    entries: BTreeMap<AssetId, AssetIndexEntry>,
    outputs: HashMap<String, AssetId>,
}

impl IndexBuilder {
    fn new(target: TargetPlatform) -> Self {
        Self {
            target,
            entries: BTreeMap::new(),
            outputs: HashMap::new(),
        }
    }

    fn push(&mut self, entry: AssetIndexEntry) -> Result<(), AssetIndexError> {
        if self.entries.contains_key(&entry.id) {
            return Err(AssetIndexError::DuplicateAssetId { id: entry.id });
        }
        // glTF 子资产（mesh/material/...）与父 glTF 共用同一个 imported/*.json output；
        // 只有文件级条目才要求 output 唯一（子资产用 locator 的 `kind:index` 区分）。
        let is_sub_asset = entry
            .locator
            .as_ref()
            .map(|locator| locator.is_sub_asset())
            .unwrap_or(false);
        let key = entry.output.to_lowercase();
        if !is_sub_asset {
            if self.outputs.contains_key(&key) {
                return Err(AssetIndexError::DuplicateOutput {
                    output: entry.output.clone(),
                });
            }
            self.outputs.insert(key, entry.id);
        }
        self.entries.insert(entry.id, entry);
        Ok(())
    }

    fn finish(self, manifest: &PlayerManifest) -> Result<PlayerAssetIndex, AssetIndexError> {
        let start_scene = match self.resolve_output(&manifest.start_scene) {
            Some(id) => Some(id),
            None if self.entries.is_empty() => None, // legacy 无 asset_id 包：允许空索引
            None => {
                return Err(AssetIndexError::MissingStartScene {
                    output: manifest.start_scene.clone(),
                })
            }
        };
        let font = match manifest.font.as_deref() {
            None => None,
            Some(font) => match self.resolve_output(font) {
                Some(id) => Some(id),
                None if self.entries.is_empty() => None,
                None => {
                    return Err(AssetIndexError::MissingFont {
                        output: font.to_string(),
                    })
                }
            },
        };
        Ok(PlayerAssetIndex {
            target: self.target,
            entries: self.entries,
            outputs: self.outputs,
            start_scene,
            font,
            cook_manifest_path: None,
        })
    }

    fn resolve_output(&self, output: &str) -> Option<AssetId> {
        let key = normalize_logical_path(output)
            .map(|normalized| normalized.to_lowercase())
            .unwrap_or_else(|_| output.to_lowercase());
        self.outputs.get(&key).copied()
    }
}

/// 资产索引错误。
#[derive(Debug)]
pub enum AssetIndexError {
    /// 读取/解析 `cook_manifest.json` 失败（坏 JSON、格式版本过高等）。
    CookManifest { path: PathBuf, message: String },
    /// cook manifest 无条目。
    EmptyManifest { context: String },
    /// cook `output` 不是合法逻辑路径。
    InvalidOutputPath { output: String, source: PathError },
    /// 重复 AssetId。
    DuplicateAssetId { id: AssetId },
    /// 两个文件级条目 output 仅大小写不同/完全相同（子资产共享 output 不算）。
    DuplicateOutput { output: String },
    /// 索引条目对应的包内文件不存在。
    MissingOutput { output: String, path: PathBuf },
    /// Player manifest.start_scene 在索引中找不到。
    MissingStartScene { output: String },
    /// Player manifest.font 在索引中找不到。
    MissingFont { output: String },
    /// Player manifest 中 asset_id 非法。
    InvalidAssetId { raw: String, message: String },
    /// Player manifest 中 locator 非法。
    InvalidLocator { raw: String, message: String },
    /// Player manifest assets[] 与 cook 索引不一致。
    ManifestIndexMismatch { output: String, reason: String },
}

impl fmt::Display for AssetIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CookManifest { path, message } => write!(
                f,
                "读取资产索引 {} 失败：{message}",
                path.display()
            ),
            Self::EmptyManifest { context } => write!(f, "资产索引为空：{context}"),
            Self::InvalidOutputPath { output, source } => {
                write!(f, "资产索引 output `{output}` 非法：{source}")
            }
            Self::DuplicateAssetId { id } => write!(f, "资产索引中 AssetId {id} 重复"),
            Self::DuplicateOutput { output } => {
                write!(f, "资产索引中 output `{output}` 重复（大小写不敏感）")
            }
            Self::MissingOutput { output, path } => write!(
                f,
                "资产索引 output `{output}` 缺失：{}",
                path.display()
            ),
            Self::MissingStartScene { output } => write!(
                f,
                "启动场景 `{output}` 不在资产索引中（cook 清单与 Player manifest 不匹配）"
            ),
            Self::MissingFont { output } => write!(
                f,
                "必检字体 `{output}` 不在资产索引中（cook 清单与 Player manifest 不匹配）"
            ),
            Self::InvalidAssetId { raw, message } => {
                write!(f, "Player manifest asset_id `{raw}` 非法：{message}")
            }
            Self::InvalidLocator { raw, message } => {
                write!(f, "Player manifest locator `{raw}` 非法：{message}")
            }
            Self::ManifestIndexMismatch { output, reason } => {
                write!(f, "资产索引与 Player manifest 不一致（{output}）：{reason}")
            }
        }
    }
}

impl std::error::Error for AssetIndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidOutputPath { source, .. } => Some(source),
            _ => None,
        }
    }
}
