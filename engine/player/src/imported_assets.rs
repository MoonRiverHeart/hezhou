//! cook 导入资产 bundle：解析包内 `assets/imported/*.json`（`hezhou-asset` 的
//! [`hezhou_asset::ImportedScene`]），建立 `AssetId → mesh/material/texture/skeleton/animation`
//! 查询，并与 `cook_manifest`（经 [`PlayerAssetIndex`]）严格核对。
//!
//! # 数据流
//!
//! ```text
//! <PlayerRoot>/assets/cook_manifest.json
//!   └─ gltf entry（output = imported/<uuid>.json） → 读取并解析 ImportedScene
//!        ├─ mesh entry      locator = assets/models/hero.gltf#mesh:0
//!        ├─ material entry  locator = ...#material:0
//!        ├─ skeleton entry  locator = ...#skeleton:0
//!        └─ animation entry locator = ...#animation:0
//! ```
//!
//! 子资产 `AssetId` 由 cook manifest 给出，`locator` 的子资产键（`<kind>:<index>`）用于
//! 定位 `ImportedScene` 数组元素。运行时/未来 C# host 通过 `AssetId` 查询，不依赖路径拼接。
//!
//! # 严格校验（任何不一致 → 可读错误，`--validate`/启动返回 exit 2）
//!
//! - 每个 gltf entry 的 `imported/*.json` 必须存在、JSON 可解析、通过
//!   [`hezhou_asset::validate_imported_scene`]；
//! - 每个子资产 entry 必须有父 gltf entry（同 output），子资产键 kind/index 与 entry.kind 一致、
//!   index 不越界；
//! - 每个 gltf 场景的数组长度必须与 cook manifest 中对应子资产 entry 数量一致
//!   （防止 cook 产物与 manifest 过期/不匹配）。
//!
//! 未被 manifest 引用的 `imported/*.json` 只产生 WARN（忽略），不阻塞启动。
//!
//! # C# 衔接契约（下一轮 scripting）
//!
//! [`ImportedAssetBundle`] 是纯 Rust 只读视图；host 通过 [`crate::host::PlayerHostContext`]
//! 拿到 `&ImportedAssetBundle`。下一轮 C# `Initialize()` 桥接时按以下方式暴露：
//! - 每个子资产以 cook `AssetId`（UUID）为句柄；
//! - `mesh(id)` → 顶点/索引/蒙皮/材质索引（`ImportedMesh`），
//!   `material(id)` → PBR 因子/纹理引用，`skeleton(id)` → joint 层级/IBM，
//!   `animation(id)` → 通道/关键帧；
//! - `scene_of(id)` 给出所属 gltf 场景 AssetId，便于一次性反序列化/生命周期管理；
//! - C# 侧不重建路径/ID 映射，直接复用本 bundle 的 AssetId ↔ 数据索引。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use hezhou_asset::{
    validate_imported_scene, AssetId, AssetKind, ImportedAnimation, ImportedMaterial, ImportedMesh,
    ImportedScene, ImportedSkeleton, ImportedTexture,
};

use crate::asset_index::PlayerAssetIndex;

/// 单个子资产在 `ImportedScene` 中的位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ElementRef {
    scene: AssetId,
    index: usize,
}

/// 已加载的导入场景。
#[derive(Clone, Debug)]
struct LoadedScene {
    scene: ImportedScene,
}

/// 一个 gltf/imported 场景内的元素统计。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportedAssetStats {
    /// 已加载的导入场景（附属于 gltf entry）数量。
    pub scenes: usize,
    /// 场景内 mesh 元素总数。
    pub meshes: usize,
    /// 场景内 material 元素总数。
    pub materials: usize,
    /// 场景内 texture 元素总数。
    pub textures: usize,
    /// 场景内 skeleton 元素总数。
    pub skeletons: usize,
    /// 场景内 animation 元素总数。
    pub animations: usize,
    /// 已按 AssetId 映射的子资产数量（mesh/material/texture/skeleton/animation）。
    pub mapped_sub_assets: usize,
    /// 加载期 WARN 数量（如未引用的 imported 文件）。
    pub warnings: usize,
}

/// 导入资产加载结果摘要（可 Clone，供 `RuntimeReport`/日志）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportedAssetSummary {
    /// 统计。
    pub stats: ImportedAssetStats,
    /// WARN 列表。
    pub warnings: Vec<String>,
}

impl ImportedAssetSummary {
    /// 单行摘要。
    pub fn summary(&self) -> String {
        if self.stats.scenes == 0 {
            return "imported 资产：无（包内没有 gltf/imported 场景）".to_string();
        }
        format!(
            "imported 资产：{} 个导入场景，mesh {} / material {} / texture {} / skeleton {} / animation {}（映射 {} 个子资产，{} 条警告）",
            self.stats.scenes,
            self.stats.meshes,
            self.stats.materials,
            self.stats.textures,
            self.stats.skeletons,
            self.stats.animations,
            self.stats.mapped_sub_assets,
            self.stats.warnings,
        )
    }
}

/// cook 导入资产 bundle（只读）。
#[derive(Clone, Debug)]
pub struct ImportedAssetBundle {
    /// gltf entry AssetId → 已解析场景。
    scenes: BTreeMap<AssetId, LoadedScene>,
    /// 子资产 AssetId → 场景内位置。
    meshes: BTreeMap<AssetId, ElementRef>,
    materials: BTreeMap<AssetId, ElementRef>,
    textures: BTreeMap<AssetId, ElementRef>,
    skeletons: BTreeMap<AssetId, ElementRef>,
    animations: BTreeMap<AssetId, ElementRef>,
    stats: ImportedAssetStats,
    warnings: Vec<String>,
}

impl ImportedAssetBundle {
    /// 从 Player 包 `assets/` 目录 + 已加载资产索引解析全部 `imported/*.json`。
    ///
    /// 无 gltf/imported 条目时返回空 bundle（Ok），兼容 legacy/最小包。
    pub fn load(assets_dir: &Path, index: &PlayerAssetIndex) -> Result<Self, ImportedAssetError> {
        // 1) 找出 gltf entry（每个 output = 一个 imported JSON）。
        let mut gltf_by_output: BTreeMap<String, AssetId> = BTreeMap::new();
        for entry in index.entries() {
            if entry.kind != AssetKind::Gltf {
                continue;
            }
            if gltf_by_output
                .insert(entry.output.clone(), entry.id)
                .is_some()
            {
                return Err(ImportedAssetError::DuplicateParent {
                    output: entry.output.clone(),
                });
            }
        }

        // 2) 解析每个被引用的 imported JSON。
        let mut scenes: BTreeMap<AssetId, LoadedScene> = BTreeMap::new();
        for (output, id) in &gltf_by_output {
            let path = assets_dir.join(translate_output(output));
            let text = std::fs::read_to_string(&path).map_err(|error| ImportedAssetError::Read {
                path: path.clone(),
                message: error.to_string(),
            })?;
            let scene: ImportedScene =
                serde_json::from_str(&text).map_err(|error| ImportedAssetError::Parse {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
            validate_imported_scene(&scene).map_err(|error| ImportedAssetError::InvalidScene {
                path: path.clone(),
                message: error.to_string(),
            })?;
            scenes.insert(
                *id,
                LoadedScene {
                    scene,
                },
            );
        }

        // 3) 映射子资产 AssetId（locator 子资产键 + 数量严格校验）。
        let mut bundle = Self {
            scenes,
            meshes: BTreeMap::new(),
            materials: BTreeMap::new(),
            textures: BTreeMap::new(),
            skeletons: BTreeMap::new(),
            animations: BTreeMap::new(),
            stats: ImportedAssetStats::default(),
            warnings: Vec::new(),
        };

        // 每个 output 的每类子资产数量（与 ImportedScene 数组长度核对）。
        let mut expected_counts: BTreeMap<(String, AssetKind), usize> = BTreeMap::new();
        for entry in index.entries() {
            let Some(locator) = &entry.locator else {
                continue;
            };
            let Some(sub_key) = locator.sub_key() else {
                continue;
            };
            let output = entry.output.clone();
            let parent = gltf_by_output
                .get(&output)
                .copied()
                .ok_or_else(|| ImportedAssetError::MissingParent {
                    asset_id: entry.id,
                    output: output.clone(),
                })?;
            let loaded = bundle
                .scenes
                .get(&parent)
                .ok_or_else(|| ImportedAssetError::MissingParent {
                    asset_id: entry.id,
                    output: output.clone(),
                })?;

            let (kind_name, index) = parse_sub_key(sub_key).ok_or_else(|| {
                ImportedAssetError::InvalidSubKey {
                    asset_id: entry.id,
                    raw: sub_key.to_string(),
                }
            })?;
            let element_kind = AssetKind::from_name(kind_name).ok_or_else(|| {
                ImportedAssetError::InvalidSubKey {
                    asset_id: entry.id,
                    raw: sub_key.to_string(),
                }
            })?;
            if !is_imported_element_kind(element_kind) {
                return Err(ImportedAssetError::UnsupportedKind {
                    asset_id: entry.id,
                    kind: element_kind,
                });
            }
            if element_kind != entry.kind {
                return Err(ImportedAssetError::KindMismatch {
                    asset_id: entry.id,
                    expected: element_kind,
                    actual: entry.kind,
                });
            }
            let len = element_len(&loaded.scene, element_kind);
            if index >= len {
                return Err(ImportedAssetError::IndexOutOfRange {
                    asset_id: entry.id,
                    kind: element_kind,
                    index,
                    len,
                });
            }
            *expected_counts
                .entry((output.clone(), element_kind))
                .or_insert(0) += 1;
            let reference = ElementRef {
                scene: parent,
                index,
            };
            let target = match element_kind {
                AssetKind::Mesh => &mut bundle.meshes,
                AssetKind::Material => &mut bundle.materials,
                AssetKind::Texture => &mut bundle.textures,
                AssetKind::Skeleton => &mut bundle.skeletons,
                AssetKind::Animation => &mut bundle.animations,
                _ => unreachable!("已由 is_imported_element_kind 过滤"),
            };
            target.insert(entry.id, reference);
        }

        // 4) 严格核对 cook manifest 子资产数量 == ImportedScene 数组长度。
        for (output, parent) in &gltf_by_output {
            let loaded = &bundle.scenes[parent];
            for kind in [
                AssetKind::Mesh,
                AssetKind::Material,
                AssetKind::Texture,
                AssetKind::Skeleton,
                AssetKind::Animation,
            ] {
                let expected = expected_counts
                    .get(&(output.clone(), kind))
                    .copied()
                    .unwrap_or(0);
                let actual = element_len(&loaded.scene, kind);
                if expected != actual {
                    return Err(ImportedAssetError::CountMismatch {
                        output: output.clone(),
                        kind,
                        expected,
                        actual,
                    });
                }
            }
        }

        // 5) 未被引用的 imported 文件只告警（忽略）。
        let imported_dir = assets_dir.join("imported");
        if imported_dir.is_dir() {
            let referenced: BTreeSet<&str> = gltf_by_output.keys().map(String::as_str).collect();
            if let Ok(entries) = std::fs::read_dir(&imported_dir) {
                let mut files: Vec<PathBuf> = entries
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .filter(|path| path.is_file())
                    .collect();
                files.sort();
                for path in files {
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let logical = format!("imported/{name}");
                    if !referenced.contains(logical.as_str()) {
                        bundle.warnings.push(format!(
                            "imported 目录下文件未被 cook manifest 引用，已忽略：{logical}"
                        ));
                    }
                }
            }
        }

        bundle.stats = ImportedAssetStats {
            scenes: bundle.scenes.len(),
            meshes: bundle.total_elements(AssetKind::Mesh),
            materials: bundle.total_elements(AssetKind::Material),
            textures: bundle.total_elements(AssetKind::Texture),
            skeletons: bundle.total_elements(AssetKind::Skeleton),
            animations: bundle.total_elements(AssetKind::Animation),
            mapped_sub_assets: bundle.meshes.len()
                + bundle.materials.len()
                + bundle.textures.len()
                + bundle.skeletons.len()
                + bundle.animations.len(),
            warnings: bundle.warnings.len(),
        };
        Ok(bundle)
    }

    /// 空 bundle（legacy 包/无导入资产）。
    pub fn empty() -> Self {
        Self {
            scenes: BTreeMap::new(),
            meshes: BTreeMap::new(),
            materials: BTreeMap::new(),
            textures: BTreeMap::new(),
            skeletons: BTreeMap::new(),
            animations: BTreeMap::new(),
            stats: ImportedAssetStats::default(),
            warnings: Vec::new(),
        }
    }

    /// 统计。
    pub fn stats(&self) -> &ImportedAssetStats {
        &self.stats
    }

    /// WARN 列表（未引用文件等）。
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// 是否为空（没有导入场景）。
    pub fn is_empty(&self) -> bool {
        self.scenes.is_empty()
    }

    /// 导入场景数量。
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// 按 gltf entry AssetId 查询整个 `ImportedScene`。
    pub fn scene(&self, id: AssetId) -> Option<&ImportedScene> {
        self.scenes.get(&id).map(|loaded| &loaded.scene)
    }

    /// 按 AssetId 查询 mesh 元素。
    pub fn mesh(&self, id: AssetId) -> Option<&ImportedMesh> {
        let reference = self.meshes.get(&id)?;
        self.scenes
            .get(&reference.scene)?
            .scene
            .meshes
            .get(reference.index)
    }

    /// 按 AssetId 查询 material 元素。
    pub fn material(&self, id: AssetId) -> Option<&ImportedMaterial> {
        let reference = self.materials.get(&id)?;
        self.scenes
            .get(&reference.scene)?
            .scene
            .materials
            .get(reference.index)
    }

    /// 按 AssetId 查询 texture 元素。
    pub fn texture(&self, id: AssetId) -> Option<&ImportedTexture> {
        let reference = self.textures.get(&id)?;
        self.scenes
            .get(&reference.scene)?
            .scene
            .textures
            .get(reference.index)
    }

    /// 按 AssetId 查询 skeleton 元素。
    pub fn skeleton(&self, id: AssetId) -> Option<&ImportedSkeleton> {
        let reference = self.skeletons.get(&id)?;
        self.scenes
            .get(&reference.scene)?
            .scene
            .skeletons
            .get(reference.index)
    }

    /// 按 AssetId 查询 animation 元素。
    pub fn animation(&self, id: AssetId) -> Option<&ImportedAnimation> {
        let reference = self.animations.get(&id)?;
        self.scenes
            .get(&reference.scene)?
            .scene
            .animations
            .get(reference.index)
    }

    /// 子资产所属的 gltf 场景 AssetId（不存在时 None）。
    pub fn scene_of(&self, id: AssetId) -> Option<AssetId> {
        if let Some(reference) = self.meshes.get(&id) {
            return Some(reference.scene);
        }
        if let Some(reference) = self.materials.get(&id) {
            return Some(reference.scene);
        }
        if let Some(reference) = self.textures.get(&id) {
            return Some(reference.scene);
        }
        if let Some(reference) = self.skeletons.get(&id) {
            return Some(reference.scene);
        }
        if let Some(reference) = self.animations.get(&id) {
            return Some(reference.scene);
        }
        None
    }

    /// 所有已映射 mesh 的 AssetId（按 AssetId 排序）。
    pub fn mesh_ids(&self) -> Vec<AssetId> {
        self.meshes.keys().copied().collect()
    }

    /// 所有已映射 skeleton 的 AssetId（按 AssetId 排序）。
    pub fn skeleton_ids(&self) -> Vec<AssetId> {
        self.skeletons.keys().copied().collect()
    }

    /// 所有已映射 animation 的 AssetId（按 AssetId 排序）。
    pub fn animation_ids(&self) -> Vec<AssetId> {
        self.animations.keys().copied().collect()
    }

    /// 遍历所有 mesh：`(mesh AssetId, glTF 场景 AssetId, 场景内 mesh 下标, mesh 数据)`。
    pub fn mesh_entries(&self) -> Vec<(AssetId, AssetId, usize, &ImportedMesh)> {
        self.scenes
            .iter()
            .flat_map(|(scene_id, loaded)| {
                loaded
                    .scene
                    .meshes
                    .iter()
                    .enumerate()
                    .map(move |(index, mesh)| (*scene_id, index, mesh))
            })
            .filter_map(|(scene, index, mesh)| {
                self.mesh_id_for(scene, index)
                    .map(|id| (id, scene, index, mesh))
            })
            .collect()
    }

    /// 场景内 mesh 下标 → AssetId（cook manifest 没声明时 None）。
    pub fn mesh_id_for(&self, scene: AssetId, index: usize) -> Option<AssetId> {
        element_id_for(&self.meshes, scene, index)
    }

    /// 场景内 skeleton 下标 → AssetId。
    pub fn skeleton_id_for(&self, scene: AssetId, index: usize) -> Option<AssetId> {
        element_id_for(&self.skeletons, scene, index)
    }

    /// 场景内 animation 下标 → AssetId。
    pub fn animation_id_for(&self, scene: AssetId, index: usize) -> Option<AssetId> {
        element_id_for(&self.animations, scene, index)
    }

    /// 场景内 texture 下标 → AssetId。
    pub fn texture_id_for(&self, scene: AssetId, index: usize) -> Option<AssetId> {
        element_id_for(&self.textures, scene, index)
    }

    /// 场景内 material 下标 → AssetId。
    pub fn material_id_for(&self, scene: AssetId, index: usize) -> Option<AssetId> {
        element_id_for(&self.materials, scene, index)
    }

    /// 单行摘要。
    pub fn summary(&self) -> String {
        self.summary_data().summary()
    }

    /// 可 Clone 的摘要（供 RuntimeReport）。
    pub fn summary_data(&self) -> ImportedAssetSummary {
        ImportedAssetSummary {
            stats: self.stats.clone(),
            warnings: self.warnings.clone(),
        }
    }

    fn total_elements(&self, kind: AssetKind) -> usize {
        self.scenes
            .values()
            .map(|loaded| element_len(&loaded.scene, kind))
            .sum()
    }
}

/// `imported/<file>` 逻辑路径转本机相对路径（`/` 分段）。
fn translate_output(output: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for segment in output.split('/') {
        if !segment.is_empty() {
            path.push(segment);
        }
    }
    path
}

fn parse_sub_key(key: &str) -> Option<(&str, usize)> {
    let (kind, index) = key.split_once(':')?;
    let index = index.parse::<usize>().ok()?;
    Some((kind, index))
}

fn is_imported_element_kind(kind: AssetKind) -> bool {
    matches!(
        kind,
        AssetKind::Mesh
            | AssetKind::Material
            | AssetKind::Texture
            | AssetKind::Skeleton
            | AssetKind::Animation
    )
}

fn element_len(scene: &ImportedScene, kind: AssetKind) -> usize {
    match kind {
        AssetKind::Mesh => scene.meshes.len(),
        AssetKind::Material => scene.materials.len(),
        AssetKind::Texture => scene.textures.len(),
        AssetKind::Skeleton => scene.skeletons.len(),
        AssetKind::Animation => scene.animations.len(),
        _ => 0,
    }
}

/// 在 `AssetId → ElementRef` 表中反查「场景 + 下标」对应的 AssetId。
fn element_id_for(
    map: &BTreeMap<AssetId, ElementRef>,
    scene: AssetId,
    index: usize,
) -> Option<AssetId> {
    map.iter()
        .find(|(_, reference)| reference.scene == scene && reference.index == index)
        .map(|(id, _)| *id)
}

/// 导入资产 bundle 错误（均为可读、可定位）。
#[derive(Debug)]
pub enum ImportedAssetError {
    /// 读取 imported JSON 失败。
    Read { path: PathBuf, message: String },
    /// imported JSON 解析失败。
    Parse { path: PathBuf, message: String },
    /// ImportedScene 内部数据不一致（`validate_imported_scene` 失败）。
    InvalidScene { path: PathBuf, message: String },
    /// 同一 output 有多个 gltf entry。
    DuplicateParent { output: String },
    /// 子资产 entry 找不到父 gltf entry。
    MissingParent { asset_id: AssetId, output: String },
    /// locator 子资产键格式非法。
    InvalidSubKey { asset_id: AssetId, raw: String },
    /// 子资产键 kind 与 entry.kind 不一致。
    KindMismatch {
        asset_id: AssetId,
        expected: AssetKind,
        actual: AssetKind,
    },
    /// 子资产不是 glTF 导入元素类型。
    UnsupportedKind { asset_id: AssetId, kind: AssetKind },
    /// 子资产 index 越界。
    IndexOutOfRange {
        asset_id: AssetId,
        kind: AssetKind,
        index: usize,
        len: usize,
    },
    /// cook manifest 子资产数量与 ImportedScene 数组长度不一致。
    CountMismatch {
        output: String,
        kind: AssetKind,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for ImportedAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, message } => {
                write!(f, "读取 imported 资产 {} 失败：{message}", path.display())
            }
            Self::Parse { path, message } => {
                write!(f, "解析 imported 场景 {} 失败：{message}", path.display())
            }
            Self::InvalidScene { path, message } => {
                write!(f, "imported 场景 {} 数据不一致：{message}", path.display())
            }
            Self::DuplicateParent { output } => {
                write!(f, "cook manifest 中 output `{output}` 有多个 gltf entry")
            }
            Self::MissingParent { asset_id, output } => write!(
                f,
                "子资产 {asset_id} 找不到父 gltf entry（output `{output}`）"
            ),
            Self::InvalidSubKey { asset_id, raw } => {
                write!(f, "子资产 {asset_id} 的 locator 子资产键非法：`{raw}`")
            }
            Self::KindMismatch {
                asset_id,
                expected,
                actual,
            } => write!(
                f,
                "子资产 {asset_id} 的 locator kind `{}` 与 entry.kind `{}` 不一致",
                expected.as_str(),
                actual.as_str()
            ),
            Self::UnsupportedKind { asset_id, kind } => write!(
                f,
                "子资产 {asset_id} 的 kind `{}` 不是 glTF 导入元素",
                kind.as_str()
            ),
            Self::IndexOutOfRange {
                asset_id,
                kind,
                index,
                len,
            } => write!(
                f,
                "子资产 {asset_id} 的 {} index {index} 越界（场景有 {len} 个元素）",
                kind.as_str()
            ),
            Self::CountMismatch {
                output,
                kind,
                expected,
                actual,
            } => write!(
                f,
                "cook manifest 与 imported 场景不一致：`{output}` 的 {} 数量 manifest={expected}、场景={actual}",
                kind.as_str()
            ),
        }
    }
}

impl std::error::Error for ImportedAssetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sub_key_roundtrip() {
        assert_eq!(parse_sub_key("mesh:0"), Some(("mesh", 0)));
        assert_eq!(parse_sub_key("animation:12"), Some(("animation", 12)));
        assert_eq!(parse_sub_key("mesh"), None);
        assert_eq!(parse_sub_key("mesh:x"), None);
        assert_eq!(parse_sub_key(":0"), Some(("", 0)));
    }

    #[test]
    fn summary_of_empty_bundle_is_readable() {
        let bundle = ImportedAssetBundle::empty();
        assert!(bundle.is_empty());
        assert!(bundle.summary().contains("无"));
        assert_eq!(bundle.stats(), &ImportedAssetStats::default());
    }
}
