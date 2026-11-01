//! 资产 cook：把编辑器资产烘焙成 Player 可消费的确定性清单与产物。
//!
//! # 设计目标
//!
//! - **只包含被引用资产**：从根资产（启动场景 / Prefab / 手动指定的 ID）出发计算依赖闭包，
//!   闭包之外的资产不会进入 manifest 与输出目录。
//! - **确定性**：同一份工程、同样的根资产，重复 cook 得到逐字节一致的 `cook_manifest.json`。
//!   manifest 中的条目按 `(逻辑路径, 子资产键, AssetId)` 排序，依赖按 ID 排序，
//!   不依赖 HashMap 迭代顺序、文件扫描顺序或时间戳。
//! - **Player 友好**：`output` 是相对 cook 输出根的 `/` 分隔路径；Player 以 `id` 为键索引。
//!
//! # manifest 格式（`format_version = 1`）
//!
//! ```json
//! {
//!   "format_version": 1,
//!   "roots": ["…uuid…"],
//!   "entries": [
//!     {
//!       "id": "…uuid…",
//!       "kind": "mesh",
//!       "locator": "assets/models/hero.gltf#mesh:0",
//!       "output": "imported/…父文件uuid….json",
//!       "dependencies": ["…uuid…"],
//!       "source_size": 10240,
//!       "source_hash": "9f2c…"
//!     }
//!   ]
//! }
//! ```
//!
//! # 输出布局
//!
//! ```text
//! <output_dir>/
//! ├── cook_manifest.json       # 本清单
//! ├── imported/<uuid>.json     # glTF 导入后的 ImportedScene（几何/骨骼/动画已烘焙）
//! └── assets/<uuid>.<ext>      # 其它资产（纹理/音频/二进制）原样拷贝
//! ```
//!
//! 纹理外部引用在 `imported/*.json` 中会被重写为对应的 `output` 路径，
//! Player 只需按 manifest 查找，不需要知道原始工程目录结构。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::database::AssetDatabase;
use crate::error::{AssetError, AssetResult};
use crate::id::{AssetId, AssetKind};
use crate::import::ImportedImage;
use crate::path::AssetLocator;

/// 当前 manifest 格式版本。
pub const COOK_MANIFEST_FORMAT_VERSION: u32 = 1;
/// manifest 文件名。
pub const COOK_MANIFEST_FILE: &str = "cook_manifest.json";
/// 原样拷贝资产的输出子目录。
pub const COOKED_ASSET_DIR: &str = "assets";
/// glTF 导入产物的输出子目录。
pub const COOKED_IMPORT_DIR: &str = "imported";

/// cook manifest 中的一个条目。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookEntry {
    /// 稳定资产 ID。
    pub id: AssetId,
    /// 资产类型。
    pub kind: AssetKind,
    /// 源定位符（工程相对逻辑路径 + 可选子资产键）。
    pub locator: AssetLocator,
    /// cooked 输出相对路径（`/` 分隔，相对 cook 输出根）。
    pub output: String,
    /// 直接依赖（已排序）。
    pub dependencies: Vec<AssetId>,
    /// 源文件字节数（子资产为父源文件大小）。
    pub source_size: u64,
    /// 源文件内容哈希（FNV-1a 64，十六进制；子资产为父源文件哈希）。
    pub source_hash: String,
}

/// cook manifest。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookManifest {
    /// manifest 格式版本。
    pub format_version: u32,
    /// 本次 cook 的根资产（已排序去重）。
    pub roots: Vec<AssetId>,
    /// 资产条目（已排序）。
    pub entries: Vec<CookEntry>,
}

impl CookManifest {
    /// 读取 manifest 文件。
    pub fn load(path: &Path) -> AssetResult<Self> {
        let text = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&text)?;
        if manifest.format_version > COOK_MANIFEST_FORMAT_VERSION {
            return Err(AssetError::Other(format!(
                "cook manifest 格式版本 {} 高于当前支持 {}",
                manifest.format_version, COOK_MANIFEST_FORMAT_VERSION
            )));
        }
        Ok(manifest)
    }

    /// 序列化为确定性 pretty JSON（末尾带换行）。
    pub fn to_json(&self) -> AssetResult<String> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }

    /// 原子写入 manifest 文件（自动创建父目录）。
    pub fn save(&self, path: &Path) -> AssetResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::meta::write_atomic(path, self.to_json()?.as_bytes())
    }

    /// 按 ID 查条目。
    pub fn entry(&self, id: AssetId) -> Option<&CookEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// 按输出路径查条目。
    pub fn entry_by_output(&self, output: &str) -> Option<&CookEntry> {
        self.entries.iter().find(|entry| entry.output == output)
    }

    /// 所有资产 ID（排序后）。
    pub fn asset_ids(&self) -> Vec<AssetId> {
        let mut ids: Vec<AssetId> = self.entries.iter().map(|entry| entry.id).collect();
        ids.sort_unstable();
        ids
    }
}

/// cook 输出统计。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookOutput {
    /// 输出根目录。
    pub output_dir: PathBuf,
    /// manifest 文件路径。
    pub manifest_path: PathBuf,
    /// 原样拷贝的资产数量。
    pub copied_files: usize,
    /// 写出的导入产物数量（每份 glTF 一个 JSON）。
    pub imported_files: usize,
    /// 写出字节总数。
    pub total_bytes: u64,
}

/// cook 执行器。
pub struct Cooker<'a> {
    db: &'a AssetDatabase,
}

impl<'a> Cooker<'a> {
    /// 创建 cooker。
    pub fn new(db: &'a AssetDatabase) -> Self {
        Self { db }
    }

    /// 计算 cook 计划（不写文件）。
    ///
    /// 只包含 `roots` 依赖闭包内的资产；结果确定性可复现。
    pub fn plan(&self, roots: &[AssetId]) -> AssetResult<CookManifest> {
        let mut sorted_roots: Vec<AssetId> = roots.to_vec();
        sorted_roots.sort_unstable();
        sorted_roots.dedup();
        if sorted_roots.is_empty() {
            return Err(AssetError::Other("cook 根资产列表为空".to_string()));
        }

        let closure = self.db.dependency_closure(&sorted_roots)?;
        let mut entries = Vec::with_capacity(closure.len());
        for id in closure {
            let entry = self
                .db
                .entry(id)
                .ok_or(AssetError::UnknownAsset(id))?
                .clone();

            // 子资产的字节来自父源文件。
            let source_file_id = if entry.is_sub_asset() {
                self.db
                    .id_of_locator(&entry.locator.file_locator())
                    .ok_or(AssetError::UnknownAsset(id))?
            } else {
                id
            };
            let source_fs = self.db.fs_path_of(source_file_id)?;
            let metadata = std::fs::metadata(&source_fs)?;
            let bytes = std::fs::read(&source_fs)?;

            let output = match entry.kind {
                AssetKind::Gltf => {
                    format!("{COOKED_IMPORT_DIR}/{}.json", id.simple())
                }
                _ if entry.is_sub_asset() => {
                    format!("{COOKED_IMPORT_DIR}/{}.json", source_file_id.simple())
                }
                _ => {
                    let extension = entry
                        .locator
                        .path()
                        .extension()
                        .unwrap_or_else(|| "bin".to_string());
                    format!("{COOKED_ASSET_DIR}/{}.{extension}", id.simple())
                }
            };

            entries.push(CookEntry {
                id,
                kind: entry.kind,
                locator: entry.locator.clone(),
                output,
                dependencies: entry.dependencies.iter().copied().collect(),
                source_size: metadata.len(),
                source_hash: fnv1a64_hex(&bytes),
            });
        }

        entries.sort_by(|a, b| a.locator.cmp(&b.locator).then_with(|| a.id.cmp(&b.id)));
        Ok(CookManifest {
            format_version: COOK_MANIFEST_FORMAT_VERSION,
            roots: sorted_roots,
            entries,
        })
    }

    /// 执行 cook：写出 manifest、拷贝资产、烘焙 glTF 导入数据。
    pub fn cook_to_dir(
        &self,
        manifest: &CookManifest,
        output_dir: impl AsRef<Path>,
    ) -> AssetResult<CookOutput> {
        if manifest.format_version > COOK_MANIFEST_FORMAT_VERSION {
            return Err(AssetError::Other(format!(
                "cook manifest 格式版本 {} 高于当前支持 {}",
                manifest.format_version, COOK_MANIFEST_FORMAT_VERSION
            )));
        }
        let output_dir = output_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&output_dir)?;
        std::fs::create_dir_all(output_dir.join(COOKED_ASSET_DIR))?;
        std::fs::create_dir_all(output_dir.join(COOKED_IMPORT_DIR))?;

        let mut copied_files = 0usize;
        let mut imported_files = 0usize;
        let mut total_bytes = 0u64;
        let mut written_imports: BTreeSet<AssetId> = BTreeSet::new();

        for entry in &manifest.entries {
            if entry.kind == AssetKind::Gltf || entry.locator.is_sub_asset() {
                // glTF 源文件与它的子资产共享同一份导入产物。
                let import_file_id = if entry.locator.is_sub_asset() {
                    self.db
                        .id_of_locator(&entry.locator.file_locator())
                        .ok_or(AssetError::UnknownAsset(entry.id))?
                } else {
                    entry.id
                };
                if !written_imports.insert(import_file_id) {
                    continue;
                }
                let target = output_dir.join(output_to_fs_path(&entry.output));
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let text = self.render_imported_json(import_file_id, manifest)?;
                total_bytes += text.len() as u64;
                std::fs::write(&target, text)?;
                imported_files += 1;
            } else {
                let source_fs = self.db.fs_path_of(entry.id)?;
                let target = output_dir.join(output_to_fs_path(&entry.output));
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let bytes = std::fs::read(&source_fs)?;
                total_bytes += bytes.len() as u64;
                std::fs::write(&target, bytes)?;
                copied_files += 1;
            }
        }

        let manifest_path = output_dir.join(COOK_MANIFEST_FILE);
        manifest.save(&manifest_path)?;
        total_bytes += std::fs::metadata(&manifest_path)?.len();

        Ok(CookOutput {
            output_dir,
            manifest_path,
            copied_files,
            imported_files,
            total_bytes,
        })
    }

    /// 生成某个 glTF 资产的导入产物 JSON，并重写纹理外部引用为 cooked 输出路径。
    fn render_imported_json(
        &self,
        file_id: AssetId,
        manifest: &CookManifest,
    ) -> AssetResult<String> {
        let entry = self
            .db
            .entry(file_id)
            .ok_or(AssetError::UnknownAsset(file_id))?;
        if entry.kind != AssetKind::Gltf {
            return Err(AssetError::KindMismatch {
                expected: AssetKind::Gltf,
                actual: entry.kind,
            });
        }
        let settings = entry.import_settings.gltf().cloned().unwrap_or_default();
        let fs_path = self.db.fs_path_of(file_id)?;
        let mut scene = crate::import::gltf::import_gltf_file(&fs_path, &settings)?;
        let source_path = entry.locator.path().clone();

        // 把 ImportedImage::ExternalFile 的相对 URI 重写为 cooked 输出路径。
        for texture in &mut scene.textures {
            if let ImportedImage::ExternalFile { uri, .. } = &mut texture.image {
                if let Some(logical) = self.db.resolve_logical_uri(&source_path, uri) {
                    if let Some(target_id) = self.db.id_of(&logical) {
                        if let Some(target_entry) = manifest.entry(target_id) {
                            *uri = target_entry.output.clone();
                        }
                    }
                }
            }
        }

        let mut text = serde_json::to_string_pretty(&scene)?;
        text.push('\n');
        Ok(text)
    }
}

/// manifest 输出路径（`/` 分隔）转文件系统路径。
fn output_to_fs_path(output: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for segment in output.split('/') {
        if !segment.is_empty() {
            path.push(segment);
        }
    }
    path
}

/// FNV-1a 64 位哈希（确定性、无依赖，用于 manifest 内容指纹）。
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// FNV-1a 64 位哈希的十六进制形式（16 字符）。
pub fn fnv1a64_hex(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a64(bytes))
}
