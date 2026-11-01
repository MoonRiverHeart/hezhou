//! 资产数据库：路径 ↔ [`AssetId`]、`.meta`、依赖图与导入设置。
//!
//! # 核心模型
//!
//! - **文件资产**：`assets/` 下一个真实文件，旁边有 `<文件名>.meta` 保存 UUID/类型/导入设置/依赖。
//! - **子资产**：由导入器从源文件产生的资产（glTF 的 Mesh/Material/Texture/Skeleton/Animation）。
//!   其定位符是 `assets/models/hero.gltf#mesh:0`，ID 从父资产 UUID 确定性派生（UUID v5），
//!   同时写入父资产的 `.meta`（`sub_assets` 表）以保证跨机器稳定。
//! - **逻辑路径**：工程相对路径，大小写不敏感比较，统一 `/` 分隔（见 [`crate::path`]）。
//!
//! # 工程移动不丢引用
//!
//! 引用只保存 [`AssetId`]（UUID），不保存路径。工程整体移动目录后重新
//! [`AssetDatabase::scan`]，`.meta` 跟随资产移动，UUID 不变，所有引用自动重新解析。
//! 资产在工程内部移动目录同理。
//!
//! # 用法
//!
//! ```no_run
//! use hezhou_asset::database::AssetDatabase;
//! use hezhou_asset::id::AssetKind;
//!
//! let mut db = AssetDatabase::new("my_project")?;
//! let report = db.scan()?;
//! assert!(report.asset_count > 0);
//!
//! // 路径 -> ID
//! let logical = hezhou_asset::path::LogicalPath::new("assets/models/hero.gltf")?;
//! let id = db.id_of(&logical).unwrap();
//! // 导入 glTF，产生 Mesh/Material/... 子资产
//! let outcome = db.import_gltf(id)?;
//! for sub in &outcome.sub_assets {
//!     println!("{:?} -> {}", sub.kind, sub.id);
//! }
//! # Ok::<(), hezhou_asset::error::AssetError>(())
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AssetError, AssetResult};
use crate::id::{AssetId, AssetKind, AssetKindTag, AssetRef};
use crate::import::ImportedScene;
use crate::meta::{self, meta_path_for, AssetMeta, ImportSettings, SubAssetMeta};
use crate::path::{AssetLocator, LogicalPath};

/// 默认资产根目录（工程相对逻辑路径）。
pub const DEFAULT_ASSETS_ROOT: &str = "assets";

/// 资产数据库中的一条记录（文件资产或子资产）。
#[derive(Clone, Debug, PartialEq)]
pub struct AssetEntry {
    /// 稳定资产 ID。
    pub id: AssetId,
    /// 资产类型。
    pub kind: AssetKind,
    /// 定位符（逻辑路径 + 可选子资产键）。
    pub locator: AssetLocator,
    /// 直接依赖（已排序）。
    pub dependencies: BTreeSet<AssetId>,
    /// 导入设置（子资产继承 `None`，设置保存在父文件 `.meta` 中）。
    pub import_settings: ImportSettings,
}

impl AssetEntry {
    /// 是否是由导入器产生的子资产。
    pub fn is_sub_asset(&self) -> bool {
        self.locator.is_sub_asset()
    }

    /// 源文件逻辑路径。
    pub fn logical_path(&self) -> &LogicalPath {
        self.locator.path()
    }
}

/// 扫描结果统计。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    /// 注册的文件资产数量。
    pub asset_count: usize,
    /// 注册的子资产数量。
    pub sub_asset_count: usize,
    /// 本次新建 `.meta` 的数量。
    pub created_meta_files: usize,
    /// 本次读取已有 `.meta` 的数量。
    pub loaded_meta_files: usize,
    /// 扫描告警（孤立 `.meta`、非法子资产键等）。
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// glTF 导入产生的子资产摘要（供 animation/player/editor 使用）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSubAsset {
    /// 子资产 ID。
    pub id: AssetId,
    /// 子资产类型。
    pub kind: AssetKind,
    /// 子资产键（`mesh:0` / `material:2` / `skeleton:0` / `animation:1`）。
    pub key: String,
    /// 可读名称（来自 glTF 名称字段）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 一次 glTF 导入的结果：中性导入数据 + 已注册的子资产 ID 映射。
#[derive(Clone, Debug)]
pub struct GltfImportOutcome {
    /// 解析出的中性场景数据（动画/渲染模块可直接消费）。
    pub scene: ImportedScene,
    /// 子资产 ID 列表。
    pub sub_assets: Vec<ImportedSubAsset>,
}

impl GltfImportOutcome {
    /// 按类型 + 序号查子资产。
    pub fn sub_asset(&self, kind: AssetKind, index: usize) -> Option<&ImportedSubAsset> {
        let key = format!("{}:{}", kind.as_str(), index);
        self.sub_assets.iter().find(|sub| sub.key == key)
    }

    /// 查找所有指定类型的子资产。
    pub fn sub_assets_of_kind(&self, kind: AssetKind) -> impl Iterator<Item = &ImportedSubAsset> {
        self.sub_assets.iter().filter(move |sub| sub.kind == kind)
    }
}

/// 资产数据库。
#[derive(Clone, Debug)]
pub struct AssetDatabase {
    project_root: PathBuf,
    assets_root: LogicalPath,
    entries: BTreeMap<AssetId, AssetEntry>,
    locators: BTreeMap<AssetLocator, AssetId>,
    warnings: Vec<String>,
}

impl AssetDatabase {
    /// 创建数据库（使用默认 `assets/` 根目录），不扫描磁盘。
    pub fn new(project_root: impl Into<PathBuf>) -> AssetResult<Self> {
        let project_root = project_root.into();
        Self::with_assets_root(project_root, DEFAULT_ASSETS_ROOT)
    }

    /// 创建数据库并指定资产根目录（工程相对逻辑路径，如 `assets`）。
    pub fn with_assets_root(
        project_root: impl Into<PathBuf>,
        assets_root: impl AsRef<str>,
    ) -> AssetResult<Self> {
        let project_root = project_root.into();
        if !project_root.is_absolute() {
            // 统一绝对化，保证 from_fs_path 的 strip_prefix 行为稳定。
            let absolute = std::path::absolute(&project_root)?;
            return Ok(Self {
                project_root: absolute,
                assets_root: LogicalPath::new(assets_root)?,
                entries: BTreeMap::new(),
                locators: BTreeMap::new(),
                warnings: Vec::new(),
            });
        }
        Ok(Self {
            project_root,
            assets_root: LogicalPath::new(assets_root)?,
            entries: BTreeMap::new(),
            locators: BTreeMap::new(),
            warnings: Vec::new(),
        })
    }

    /// 打开工程：创建数据库并执行一次 [`Self::scan`]。
    pub fn open(project_root: impl Into<PathBuf>) -> AssetResult<Self> {
        let mut db = Self::new(project_root)?;
        db.scan()?;
        Ok(db)
    }

    /// 工程根目录（绝对路径）。
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// 资产根目录（工程相对逻辑路径）。
    pub fn assets_root(&self) -> &LogicalPath {
        &self.assets_root
    }

    /// 资产根目录的文件系统路径。
    pub fn assets_root_fs(&self) -> PathBuf {
        self.assets_root.to_fs_path(&self.project_root)
    }

    /// 本次会话累计的告警（扫描 + 导入）。
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// 取出并清空告警。
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// 扫描资产目录，建立/刷新路径 → ID 映射与依赖图。
    ///
    /// 行为：
    /// 1. 递归遍历 `assets/`（跳过隐藏文件/目录）；
    /// 2. 每个非 `.meta` 文件：读取 `.meta`，不存在则新建并写盘；
    /// 3. 注册 `.meta` 中记录的子资产（导入产物）；
    /// 4. 孤立 `.meta`（对应资产已删除）产生告警但不报错。
    ///
    /// 该操作是可重复的：移动目录/工程后重新扫描即可恢复引用映射。
    pub fn scan(&mut self) -> AssetResult<ScanReport> {
        let assets_root_fs = self.assets_root_fs();
        std::fs::create_dir_all(&assets_root_fs)?;

        self.entries.clear();
        self.locators.clear();

        let mut report = ScanReport::default();
        let mut files = Vec::new();
        collect_files(&assets_root_fs, &mut files)?;

        let mut asset_files = Vec::new();
        for file in files {
            if meta::is_meta_path(&file) {
                let base = strip_meta_suffix(&file);
                if !base.exists() {
                    report
                        .warnings
                        .push(format!("孤立 meta 文件（资产不存在）: {}", file.display()));
                }
                continue;
            }
            asset_files.push(file);
        }

        for fs_path in asset_files {
            let logical = LogicalPath::from_fs_path(&self.project_root, &fs_path)?;
            let meta_path = meta_path_for(&fs_path);
            let asset_meta = if meta_path.is_file() {
                report.loaded_meta_files += 1;
                AssetMeta::load(&meta_path)?
            } else {
                report.created_meta_files += 1;
                let created = AssetMeta::create(AssetMeta::infer_kind(&fs_path));
                created.save(&meta_path)?;
                created
            };

            report.asset_count += 1;
            let file_locator = AssetLocator::file(logical.clone());
            self.insert_entry(AssetEntry {
                id: asset_meta.uuid,
                kind: asset_meta.kind,
                locator: file_locator,
                dependencies: asset_meta.dependencies.iter().copied().collect(),
                import_settings: asset_meta.import_settings.clone(),
            })?;

            for (key, sub) in &asset_meta.sub_assets {
                let locator = match AssetLocator::sub(logical.clone(), key) {
                    Ok(locator) => locator,
                    Err(error) => {
                        report
                            .warnings
                            .push(format!("meta 中的子资产键非法 {key:?}: {error}"));
                        continue;
                    }
                };
                let mut dependencies: BTreeSet<AssetId> =
                    sub.dependencies.iter().copied().collect();
                // 子资产永远依赖父源文件，保证 cook 闭包包含源数据。
                dependencies.insert(asset_meta.uuid);
                self.insert_entry(AssetEntry {
                    id: sub.uuid,
                    kind: sub.kind,
                    locator,
                    dependencies,
                    import_settings: ImportSettings::None,
                })?;
                report.sub_asset_count += 1;
            }
        }

        self.warnings.extend(report.warnings.iter().cloned());
        Ok(report)
    }

    /// 按 ID 查资产记录。
    pub fn entry(&self, id: AssetId) -> Option<&AssetEntry> {
        self.entries.get(&id)
    }

    /// 按逻辑路径查文件资产记录。
    pub fn entry_by_path(&self, path: &LogicalPath) -> Option<&AssetEntry> {
        self.locators
            .get(&AssetLocator::file(path.clone()))
            .and_then(|id| self.entries.get(id))
    }

    /// 按定位符查资产记录。
    pub fn entry_by_locator(&self, locator: &AssetLocator) -> Option<&AssetEntry> {
        self.locators
            .get(locator)
            .and_then(|id| self.entries.get(id))
    }

    /// 全部资产记录（按 ID 排序）。
    pub fn entries(&self) -> impl Iterator<Item = &AssetEntry> {
        self.entries.values()
    }

    /// 资产数量。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 数据库是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 逻辑路径 → ID。
    pub fn id_of(&self, path: &LogicalPath) -> Option<AssetId> {
        self.locators
            .get(&AssetLocator::file(path.clone()))
            .copied()
    }

    /// 定位符 → ID。
    pub fn id_of_locator(&self, locator: &AssetLocator) -> Option<AssetId> {
        self.locators.get(locator).copied()
    }

    /// ID → 定位符。
    pub fn locator_of(&self, id: AssetId) -> Option<&AssetLocator> {
        self.entries.get(&id).map(|entry| &entry.locator)
    }

    /// ID → 源文件逻辑路径（子资产返回其源文件路径）。
    pub fn logical_path_of(&self, id: AssetId) -> Option<&LogicalPath> {
        self.entries.get(&id).map(|entry| entry.locator.path())
    }

    /// ID → 源文件系统路径。
    pub fn fs_path_of(&self, id: AssetId) -> AssetResult<PathBuf> {
        let entry = self.entries.get(&id).ok_or(AssetError::UnknownAsset(id))?;
        Ok(entry.locator.path().to_fs_path(&self.project_root))
    }

    /// 构造类型化引用并校验实际类型。
    pub fn typed_ref<K: AssetKindTag>(&self, id: AssetId) -> AssetResult<AssetRef<K>> {
        let entry = self.entries.get(&id).ok_or(AssetError::UnknownAsset(id))?;
        let reference = AssetRef::<K>::new(id);
        reference.validate(entry)?;
        Ok(reference)
    }

    /// 直接依赖（已排序）。
    pub fn dependencies(&self, id: AssetId) -> AssetResult<&BTreeSet<AssetId>> {
        self.entries
            .get(&id)
            .map(|entry| &entry.dependencies)
            .ok_or(AssetError::UnknownAsset(id))
    }

    /// 依赖闭包（含根资产），按 **拓扑顺序** 返回：依赖在前，引用者在后。
    ///
    /// - 结果集合只包含 `roots` 可达的资产（cook “只包含被引用资产”的基础）；
    /// - 顺序确定：根与依赖均按 [`AssetId`] 排序遍历；
    /// - 依赖成环时返回 [`AssetError::DependencyCycle`]。
    pub fn dependency_closure(&self, roots: &[AssetId]) -> AssetResult<Vec<AssetId>> {
        let mut sorted_roots: Vec<AssetId> = roots.to_vec();
        sorted_roots.sort_unstable();
        sorted_roots.dedup();

        let mut state: BTreeMap<AssetId, u8> = BTreeMap::new();
        let mut order = Vec::new();
        let mut path = Vec::new();
        for root in sorted_roots {
            self.visit_dependency(root, &mut state, &mut order, &mut path)?;
        }
        Ok(order)
    }

    fn visit_dependency(
        &self,
        id: AssetId,
        state: &mut BTreeMap<AssetId, u8>,
        order: &mut Vec<AssetId>,
        path: &mut Vec<AssetId>,
    ) -> AssetResult<()> {
        match state.get(&id).copied() {
            Some(2) => return Ok(()),
            Some(1) => {
                path.push(id);
                return Err(AssetError::DependencyCycle(path.clone()));
            }
            _ => {}
        }
        let entry = self.entries.get(&id).ok_or(AssetError::UnknownAsset(id))?;
        state.insert(id, 1);
        path.push(id);
        for dependency in &entry.dependencies {
            self.visit_dependency(*dependency, state, order, path)?;
        }
        path.pop();
        state.insert(id, 2);
        order.push(id);
        Ok(())
    }

    /// 设置资产依赖（写入内存并同步到 `.meta`）。
    pub fn set_dependencies(
        &mut self,
        id: AssetId,
        dependencies: impl IntoIterator<Item = AssetId>,
    ) -> AssetResult<()> {
        let entry = self.entries.get(&id).ok_or(AssetError::UnknownAsset(id))?;
        let locator = entry.locator.clone();
        let new_dependencies: BTreeSet<AssetId> = dependencies.into_iter().collect();

        let mut meta = self.meta_for(id)?;
        if let Some(sub_key) = locator.sub_key() {
            let sub = meta
                .sub_assets
                .get_mut(sub_key)
                .ok_or(AssetError::UnknownAsset(id))?;
            sub.dependencies = new_dependencies.iter().copied().collect();
        } else {
            meta.set_dependencies(new_dependencies.iter().copied());
        }
        meta.save(&meta_path_for(
            &locator.path().to_fs_path(&self.project_root),
        ))?;

        if let Some(entry) = self.entries.get_mut(&id) {
            entry.dependencies = new_dependencies;
        }
        Ok(())
    }

    /// 读取资产（子资产返回父文件）的 `.meta`。
    pub fn meta_for(&self, id: AssetId) -> AssetResult<AssetMeta> {
        let entry = self.entries.get(&id).ok_or(AssetError::UnknownAsset(id))?;
        let fs_path = entry.locator.path().to_fs_path(&self.project_root);
        AssetMeta::load(&meta_path_for(&fs_path))
    }

    /// 确保某个工程内文件已注册为文件资产（必要时创建 `.meta`），返回其 ID。
    pub fn register_file_asset(&mut self, logical_path: &LogicalPath) -> AssetResult<AssetId> {
        if let Some(id) = self.id_of(logical_path) {
            return Ok(id);
        }
        let fs_path = logical_path.to_fs_path(&self.project_root);
        if !fs_path.is_file() {
            return Err(AssetError::UnknownPath(logical_path.clone()));
        }
        let meta_path = meta_path_for(&fs_path);
        let asset_meta = if meta_path.is_file() {
            AssetMeta::load(&meta_path)?
        } else {
            let created = AssetMeta::create(AssetMeta::infer_kind(&fs_path));
            created.save(&meta_path)?;
            created
        };
        self.insert_entry(AssetEntry {
            id: asset_meta.uuid,
            kind: asset_meta.kind,
            locator: AssetLocator::file(logical_path.clone()),
            dependencies: asset_meta.dependencies.iter().copied().collect(),
            import_settings: asset_meta.import_settings.clone(),
        })?;
        Ok(asset_meta.uuid)
    }

    /// 由“源文件逻辑路径 + glTF 相对 URI”解析出工程逻辑路径（不访问磁盘）。
    pub fn resolve_logical_uri(&self, source_path: &LogicalPath, uri: &str) -> Option<LogicalPath> {
        let uri = uri.replace('\\', "/");
        if uri.is_empty() {
            return None;
        }
        let parent = source_path.parent();
        let joined = match parent {
            Some(parent) => parent.join(&uri),
            None => LogicalPath::new(&uri),
        };
        joined.ok()
    }

    /// 导入 glTF/GLB 资产：解析文件、注册子资产、更新依赖与 `.meta`。
    ///
    /// 子资产 ID 规则：
    /// - 键为 `<kind>:<index>`（如 `mesh:0`、`animation:1`）；
    /// - 已有 `.meta` 中的子资产 ID 优先复用（重新导入不丢引用）；
    /// - 新子资产 ID 由父 UUID 通过 UUID v5 确定性派生（跨机器一致）。
    ///
    /// 依赖规则：
    /// - 每个子资产依赖父源文件；
    /// - Mesh 依赖其使用的 Material / Skeleton；
    /// - Material 依赖其引用的 Texture；
    /// - Texture 与源文件依赖外部图片/缓冲区文件（若在工程 `assets/` 内）。
    #[cfg(feature = "gltf")]
    pub fn import_gltf(&mut self, file_id: AssetId) -> AssetResult<GltfImportOutcome> {
        let entry = self
            .entries
            .get(&file_id)
            .ok_or(AssetError::UnknownAsset(file_id))?;
        if entry.kind != AssetKind::Gltf {
            return Err(AssetError::KindMismatch {
                expected: AssetKind::Gltf,
                actual: entry.kind,
            });
        }
        let logical = entry.locator.path().clone();
        let settings = entry.import_settings.gltf().cloned().unwrap_or_default();
        let fs_path = logical.to_fs_path(&self.project_root);
        let scene = crate::import::gltf::import_gltf_file(&fs_path, &settings)?;

        self.register_gltf_scene(file_id, &logical, &scene)
    }

    /// 导入 glTF/GLB 资产（`gltf` feature 未启用时的桩）。
    #[cfg(not(feature = "gltf"))]
    pub fn import_gltf(&mut self, _file_id: AssetId) -> AssetResult<GltfImportOutcome> {
        Err(AssetError::ImporterDisabled("gltf"))
    }

    /// 把已经解析好的 [`ImportedScene`] 注册进数据库（导入器无关的注册逻辑，测试可用）。
    pub fn register_gltf_scene(
        &mut self,
        file_id: AssetId,
        logical: &LogicalPath,
        scene: &ImportedScene,
    ) -> AssetResult<GltfImportOutcome> {
        let mut meta = self.meta_for(file_id)?;

        let mut pending: BTreeMap<String, PendingSubAsset> = BTreeMap::new();
        for (index, mesh) in scene.meshes.iter().enumerate() {
            ensure_pending(
                &mut pending,
                &meta,
                file_id,
                AssetKind::Mesh,
                index,
                mesh.name.clone(),
            );
        }
        for (index, material) in scene.materials.iter().enumerate() {
            ensure_pending(
                &mut pending,
                &meta,
                file_id,
                AssetKind::Material,
                index,
                material.name.clone(),
            );
        }
        for (index, texture) in scene.textures.iter().enumerate() {
            ensure_pending(
                &mut pending,
                &meta,
                file_id,
                AssetKind::Texture,
                index,
                texture.name.clone(),
            );
        }
        for (index, skeleton) in scene.skeletons.iter().enumerate() {
            ensure_pending(
                &mut pending,
                &meta,
                file_id,
                AssetKind::Skeleton,
                index,
                skeleton.name.clone(),
            );
        }
        for (index, animation) in scene.animations.iter().enumerate() {
            ensure_pending(
                &mut pending,
                &meta,
                file_id,
                AssetKind::Animation,
                index,
                animation.name.clone(),
            );
        }

        // Mesh -> Material / Skeleton
        for (index, mesh) in scene.meshes.iter().enumerate() {
            let key = format!("{}:{}", AssetKind::Mesh.as_str(), index);
            let mut material_ids = Vec::new();
            let mut skeleton_ids = Vec::new();
            for primitive in &mesh.primitives {
                if let Some(material) = primitive.material {
                    if let Some(id) = pending
                        .get(&format!("{}:{}", AssetKind::Material.as_str(), material))
                        .map(|sub| sub.id)
                    {
                        material_ids.push(id);
                    }
                }
                if let Some(skeleton) = primitive.skeleton {
                    if let Some(id) = pending
                        .get(&format!("{}:{}", AssetKind::Skeleton.as_str(), skeleton))
                        .map(|sub| sub.id)
                    {
                        skeleton_ids.push(id);
                    }
                }
            }
            if let Some(sub) = pending.get_mut(&key) {
                sub.deps.extend(material_ids);
                sub.deps.extend(skeleton_ids);
            }
        }

        // Material -> Texture
        for (index, material) in scene.materials.iter().enumerate() {
            let key = format!("{}:{}", AssetKind::Material.as_str(), index);
            let mut texture_ids = Vec::new();
            for texture_ref in [
                material.base_color_texture,
                material.metallic_roughness_texture,
                material.normal_texture,
                material.occlusion_texture,
                material.emissive_texture,
            ]
            .into_iter()
            .flatten()
            {
                if let Some(id) = pending
                    .get(&format!(
                        "{}:{}",
                        AssetKind::Texture.as_str(),
                        texture_ref.texture
                    ))
                    .map(|sub| sub.id)
                {
                    texture_ids.push(id);
                }
            }
            if let Some(sub) = pending.get_mut(&key) {
                sub.deps.extend(texture_ids);
            }
        }

        // Texture -> 外部图片文件
        for (index, texture) in scene.textures.iter().enumerate() {
            if let crate::import::ImportedImage::ExternalFile { uri, .. } = &texture.image {
                let Some(target) = self.resolve_logical_uri(logical, uri) else {
                    continue;
                };
                if let Some(external_id) = self.resolve_external_asset(&target)? {
                    let key = format!("{}:{}", AssetKind::Texture.as_str(), index);
                    if let Some(sub) = pending.get_mut(&key) {
                        sub.deps.insert(external_id);
                    }
                }
            }
        }

        // 源文件 -> 外部缓冲区 / 图片文件
        let mut file_dependencies: BTreeSet<AssetId> = BTreeSet::new();
        for uri in &scene.external_files {
            let Some(target) = self.resolve_logical_uri(logical, uri) else {
                continue;
            };
            if let Some(external_id) = self.resolve_external_asset(&target)? {
                file_dependencies.insert(external_id);
            }
        }

        // 写回 `.meta`：子资产表（复用旧 ID）+ 源文件依赖
        meta.sub_assets = pending
            .iter()
            .map(|(key, sub)| {
                (
                    key.clone(),
                    SubAssetMeta {
                        uuid: sub.id,
                        kind: sub.kind,
                        name: sub.name.clone(),
                        dependencies: sub.deps.iter().copied().collect(),
                    },
                )
            })
            .collect();
        meta.set_dependencies(file_dependencies.iter().copied());
        let fs_path = logical.to_fs_path(&self.project_root);
        meta.save(&meta_path_for(&fs_path))?;

        if let Some(entry) = self.entries.get_mut(&file_id) {
            entry.dependencies = file_dependencies;
        }

        // 清理已不存在的子资产记录
        let stale: Vec<AssetId> = self
            .entries
            .values()
            .filter(|entry| {
                entry.locator.is_sub_asset()
                    && entry.locator.path() == logical
                    && !pending.contains_key(entry.locator.sub_key().unwrap_or(""))
            })
            .map(|entry| entry.id)
            .collect();
        for id in stale {
            if let Some(entry) = self.entries.remove(&id) {
                self.locators.remove(&entry.locator);
            }
        }

        // 注册新子资产
        let mut sub_assets = Vec::with_capacity(pending.len());
        for (key, sub) in &pending {
            let locator = AssetLocator::sub(logical.clone(), key)?;
            self.insert_entry(AssetEntry {
                id: sub.id,
                kind: sub.kind,
                locator,
                dependencies: sub.deps.clone(),
                import_settings: ImportSettings::None,
            })?;
            sub_assets.push(ImportedSubAsset {
                id: sub.id,
                kind: sub.kind,
                key: key.clone(),
                name: sub.name.clone(),
            });
        }
        sub_assets.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.key.cmp(&b.key)));

        Ok(GltfImportOutcome {
            scene: scene.clone(),
            sub_assets,
        })
    }

    /// 把外部引用文件登记为资产（仅工程 `assets/` 内的文件）。
    fn resolve_external_asset(&mut self, logical: &LogicalPath) -> AssetResult<Option<AssetId>> {
        if let Some(id) = self.id_of(logical) {
            return Ok(Some(id));
        }
        if !logical.is_under(&self.assets_root) {
            return Ok(None);
        }
        let fs_path = logical.to_fs_path(&self.project_root);
        if !fs_path.is_file() {
            self.warnings.push(format!(
                "外部引用文件不存在，未登记为资产: {}",
                logical.as_str()
            ));
            return Ok(None);
        }
        self.register_file_asset(logical).map(Some)
    }

    fn insert_entry(&mut self, entry: AssetEntry) -> AssetResult<()> {
        if let Some(&existing) = self.locators.get(&entry.locator) {
            if existing != entry.id {
                return Err(AssetError::DuplicateLocator(
                    entry.locator.to_string(),
                    self.entries
                        .get(&existing)
                        .map(|other| other.locator.to_string())
                        .unwrap_or_default(),
                ));
            }
            self.entries.insert(entry.id, entry);
            return Ok(());
        }
        self.locators.insert(entry.locator.clone(), entry.id);
        self.entries.insert(entry.id, entry);
        Ok(())
    }
}

/// 待注册子资产（导入期间的可变中间状态）。
#[derive(Clone, Debug)]
struct PendingSubAsset {
    id: AssetId,
    kind: AssetKind,
    name: Option<String>,
    deps: BTreeSet<AssetId>,
}

/// 取得（必要时创建）子资产 ID：优先复用 `.meta` 中的旧 ID，否则确定性派生。
fn ensure_pending(
    pending: &mut BTreeMap<String, PendingSubAsset>,
    meta: &AssetMeta,
    file_id: AssetId,
    kind: AssetKind,
    index: usize,
    name: Option<String>,
) -> AssetId {
    let key = format!("{}:{}", kind.as_str(), index);
    if let Some(existing) = pending.get(&key) {
        return existing.id;
    }
    let id = meta
        .sub_assets
        .get(&key)
        .filter(|sub| sub.kind == kind)
        .map(|sub| sub.uuid)
        .unwrap_or_else(|| file_id.derive_child(kind, &key));
    pending.insert(
        key,
        PendingSubAsset {
            id,
            kind,
            name,
            deps: BTreeSet::from([file_id]),
        },
    );
    id
}

/// 递归收集文件（按名称排序，跳过隐藏项），保证扫描确定性。
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> AssetResult<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|a, b| {
        let a_name = a
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let b_name = b
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        a_name.cmp(&b_name).then_with(|| a.cmp(b))
    });

    for path in entries {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// `hero.gltf.meta` -> `hero.gltf`。
fn strip_meta_suffix(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let stripped = file_name
        .strip_suffix(&format!(".{}", meta::META_EXTENSION))
        .unwrap_or(&file_name)
        .to_string();
    path.with_file_name(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_database_has_no_entries() {
        let temp = crate::test_support::tempdir();
        let db = AssetDatabase::new(temp.path()).unwrap();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn collect_files_is_sorted_and_skips_hidden() {
        let temp = crate::test_support::tempdir();
        let root = temp.path();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("b.txt"), b"b").unwrap();
        std::fs::write(root.join("A.txt"), b"a").unwrap();
        std::fs::write(root.join("sub/c.txt"), b"c").unwrap();
        std::fs::write(root.join(".hidden"), b"h").unwrap();
        let mut files = Vec::new();
        collect_files(root, &mut files).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(names, vec!["A.txt", "b.txt", "sub/c.txt"]);
    }
}
