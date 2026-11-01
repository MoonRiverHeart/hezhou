//! `.meta` 元数据文件。
//!
//! 每个受资产数据库管理的资产旁边都有一个 `<文件名>.meta`（UTF-8 JSON）：
//!
//! ```text
//! assets/models/hero.gltf
//! assets/models/hero.gltf.meta
//! ```
//!
//! `.meta` 是资产 **身份** 的真正载体：
//!
//! - `uuid`：稳定资产 ID，文件移动、工程改名、重新扫描都不变；
//! - `kind`：资产类型；
//! - `import_settings`：导入设置（glTF/纹理/音频各自的参数）；
//! - `dependencies`：文件级资产依赖（引用其它资产的 UUID）；
//! - `sub_assets`：导入产物（Mesh/Material/Skeleton/Animation 等）的 UUID 表和依赖。
//!
//! 未识别的 JSON 字段会保留在 [`AssetMeta::extra`] 里，保证旧版本编辑器打开新工程
//! 再保存不会丢信息（前向兼容）。
//!
//! 写入使用“临时文件 + rename”的原子写法，避免崩溃时留下半个 JSON。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AssetError, AssetResult};
use crate::id::{AssetId, AssetKind};

/// `.meta` 文件扩展名（完整文件名形如 `hero.gltf.meta`）。
pub const META_EXTENSION: &str = "meta";

/// 当前 `.meta` 格式版本。
pub const META_VERSION: u32 = 1;

/// 判断路径是否是 `.meta` 文件。
pub fn is_meta_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(META_EXTENSION))
        .unwrap_or(false)
}

/// 资产文件对应的 `.meta` 路径（`hero.gltf` -> `hero.gltf.meta`）。
pub fn meta_path_for(asset_path: &Path) -> PathBuf {
    match asset_path.file_name() {
        Some(name) => {
            let mut file_name = name.to_os_string();
            file_name.push(".");
            file_name.push(META_EXTENSION);
            asset_path.with_file_name(file_name)
        }
        None => {
            let mut path = asset_path.to_path_buf();
            path.set_extension(META_EXTENSION);
            path
        }
    }
}

/// glTF/GLB 导入设置。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GltfImportSettings {
    /// 是否导入网格。
    #[serde(default = "default_true")]
    pub import_meshes: bool,
    /// 是否导入材质。
    #[serde(default = "default_true")]
    pub import_materials: bool,
    /// 是否导入纹理。
    #[serde(default = "default_true")]
    pub import_textures: bool,
    /// 是否导入骨骼/蒙皮。
    #[serde(default = "default_true")]
    pub import_skins: bool,
    /// 是否导入动画。
    #[serde(default = "default_true")]
    pub import_animations: bool,
    /// 导入缩放（应用到顶点位置；单位换算用，如 0.01 表示厘米转米）。
    #[serde(default = "default_scale")]
    pub scale: f32,
}

impl Default for GltfImportSettings {
    fn default() -> Self {
        Self {
            import_meshes: true,
            import_materials: true,
            import_textures: true,
            import_skins: true,
            import_animations: true,
            scale: 1.0,
        }
    }
}

/// 纹理过滤模式（与渲染层约定一致，资产层只保存设置）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureFilter {
    /// 线性过滤。
    #[default]
    Linear,
    /// 最近邻过滤。
    Nearest,
}

/// 纹理环绕模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureWrap {
    /// 重复。
    #[default]
    Repeat,
    /// 边缘拉伸。
    Clamp,
    /// 镜像重复。
    Mirror,
}

/// 纹理导入设置。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextureImportSettings {
    /// 是否按 sRGB 色彩空间采样（颜色贴图为 true，法线/遮罩为 false）。
    #[serde(default = "default_true")]
    pub srgb: bool,
    /// 是否生成 mipmap。
    #[serde(default = "default_true")]
    pub generate_mipmaps: bool,
    /// 放大/缩小过滤。
    #[serde(default)]
    pub filter: TextureFilter,
    /// 环绕模式。
    #[serde(default)]
    pub wrap: TextureWrap,
}

impl Default for TextureImportSettings {
    fn default() -> Self {
        Self {
            srgb: true,
            generate_mipmaps: true,
            filter: TextureFilter::Linear,
            wrap: TextureWrap::Repeat,
        }
    }
}

/// 音频导入设置。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioImportSettings {
    /// 是否以流式方式加载（长音乐建议 true）。
    #[serde(default)]
    pub streaming: bool,
    /// 默认是否循环。
    #[serde(default)]
    pub looping: bool,
    /// 导入时应用的音量增益。
    #[serde(default = "default_scale")]
    pub volume: f32,
}

impl Default for AudioImportSettings {
    fn default() -> Self {
        Self {
            streaming: false,
            looping: false,
            volume: 1.0,
        }
    }
}

/// 资产导入设置。
///
/// JSON 形式（adjacently tagged，便于扩展和手写）：
///
/// ```json
/// { "importer": "gltf", "settings": { "import_animations": false } }
/// { "importer": "none" }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "importer", content = "settings", rename_all = "snake_case")]
pub enum ImportSettings {
    /// glTF/GLB 导入设置。
    Gltf(GltfImportSettings),
    /// 纹理导入设置。
    Texture(TextureImportSettings),
    /// 音频导入设置。
    Audio(AudioImportSettings),
    /// 无特殊导入设置（二进制、着色器、字体等直接拷贝）。
    #[default]
    None,
}

impl ImportSettings {
    /// 根据资产类型返回默认导入设置。
    pub fn default_for_kind(kind: AssetKind) -> Self {
        match kind {
            AssetKind::Gltf => Self::Gltf(GltfImportSettings::default()),
            AssetKind::Texture => Self::Texture(TextureImportSettings::default()),
            AssetKind::Audio => Self::Audio(AudioImportSettings::default()),
            _ => Self::None,
        }
    }

    /// 取 glTF 导入设置（类型不匹配时返回 `None`）。
    pub fn gltf(&self) -> Option<&GltfImportSettings> {
        match self {
            Self::Gltf(settings) => Some(settings),
            _ => None,
        }
    }

    /// 取纹理导入设置。
    pub fn texture(&self) -> Option<&TextureImportSettings> {
        match self {
            Self::Texture(settings) => Some(settings),
            _ => None,
        }
    }

    /// 取音频导入设置。
    pub fn audio(&self) -> Option<&AudioImportSettings> {
        match self {
            Self::Audio(settings) => Some(settings),
            _ => None,
        }
    }
}

/// 单个导入子资产（glTF 的一个 Mesh/Material/Skeleton/Animation/Texture）的元数据。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubAssetMeta {
    /// 稳定子资产 UUID。
    pub uuid: AssetId,
    /// 子资产类型。
    pub kind: AssetKind,
    /// 导入器中的可读名称（不参与身份判定）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 直接依赖（UUID，保存时排序去重）。
    #[serde(default)]
    pub dependencies: Vec<AssetId>,
}

impl SubAssetMeta {
    /// 新建子资产元数据。
    pub fn new(uuid: AssetId, kind: AssetKind, name: Option<String>) -> Self {
        Self {
            uuid,
            kind,
            name,
            dependencies: Vec::new(),
        }
    }
}

/// `.meta` 文件内容。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssetMeta {
    /// 格式版本，当前为 [`META_VERSION`]。
    #[serde(default = "default_meta_version")]
    pub meta_version: u32,
    /// 稳定资产 UUID。
    pub uuid: AssetId,
    /// 资产类型。
    pub kind: AssetKind,
    /// 导入设置。
    #[serde(default)]
    pub import_settings: ImportSettings,
    /// 文件级依赖（UUID）。
    #[serde(default)]
    pub dependencies: Vec<AssetId>,
    /// 子资产表：`kind:index` -> 子资产元数据。
    #[serde(default)]
    pub sub_assets: BTreeMap<String, SubAssetMeta>,
    /// 未识别的字段（前向兼容，原样保留）。
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl AssetMeta {
    /// 新建资产元数据（导入设置按类型给默认值）。
    pub fn new(uuid: AssetId, kind: AssetKind) -> Self {
        Self {
            meta_version: META_VERSION,
            uuid,
            kind,
            import_settings: ImportSettings::default_for_kind(kind),
            dependencies: Vec::new(),
            sub_assets: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }

    /// 新建资产元数据并自动生成随机 UUID。
    pub fn create(kind: AssetKind) -> Self {
        Self::new(AssetId::new(), kind)
    }

    /// 读取 `.meta` 文件。
    pub fn load(meta_path: &Path) -> AssetResult<Self> {
        let text = std::fs::read_to_string(meta_path)?;
        let meta: Self = serde_json::from_str(&text)?;
        meta.validate_version()?;
        Ok(meta.normalized())
    }

    /// 从 JSON 字符串解析（同样做版本校验与规范化）。
    pub fn from_json(text: &str) -> AssetResult<Self> {
        let meta: Self = serde_json::from_str(text)?;
        meta.validate_version()?;
        Ok(meta.normalized())
    }

    /// 序列化为确定性的 pretty JSON（依赖排序去重，末尾带换行）。
    pub fn to_json(&self) -> AssetResult<String> {
        let normalized = self.clone().normalized();
        let mut text = serde_json::to_string_pretty(&normalized)?;
        text.push('\n');
        Ok(text)
    }

    /// 原子写入 `.meta` 文件（自动创建父目录）。
    pub fn save(&self, meta_path: &Path) -> AssetResult<()> {
        if let Some(parent) = meta_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = self.to_json()?;
        write_atomic(meta_path, text.as_bytes())
    }

    /// 校验格式版本。
    pub fn validate_version(&self) -> AssetResult<()> {
        if self.meta_version > META_VERSION {
            return Err(AssetError::UnsupportedMetaVersion(self.meta_version));
        }
        Ok(())
    }

    /// 规范化：依赖与子资产依赖排序去重，返回新值。
    ///
    /// 确保同一个工程的 `.meta` 在不同机器/不同扫描顺序下字节一致。
    pub fn normalized(mut self) -> Self {
        sort_dedup_ids(&mut self.dependencies);
        for sub in self.sub_assets.values_mut() {
            sort_dedup_ids(&mut sub.dependencies);
        }
        self
    }

    /// 设置文件级依赖（自动排序去重）。
    pub fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = AssetId>) {
        self.dependencies = dependencies.into_iter().collect();
        sort_dedup_ids(&mut self.dependencies);
    }

    /// 添加一个文件级依赖；已存在时忽略。
    pub fn add_dependency(&mut self, dependency: AssetId) {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
            sort_dedup_ids(&mut self.dependencies);
        }
    }

    /// 由资产文件路径推断默认类型（用于首次扫描时没有 `.meta` 的情况）。
    pub fn infer_kind(asset_path: &Path) -> AssetKind {
        asset_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(AssetKind::from_extension)
            .unwrap_or(AssetKind::Other)
    }
}

/// 原子写文件：先写 `<path>.tmp`，再 rename 覆盖。
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> AssetResult<()> {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "asset.json".to_string());
    let tmp = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    {
        use std::io::Write as _;
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn sort_dedup_ids(ids: &mut Vec<AssetId>) {
    ids.sort_unstable();
    ids.dedup();
}

fn default_meta_version() -> u32 {
    META_VERSION
}

fn default_true() -> bool {
    true
}

fn default_scale() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::AssetId;

    fn sample_meta() -> AssetMeta {
        let mut meta = AssetMeta::new(AssetId::new(), AssetKind::Gltf);
        meta.set_dependencies([AssetId::new(), AssetId::new()]);
        meta.sub_assets.insert(
            "mesh:0".to_string(),
            SubAssetMeta::new(AssetId::new(), AssetKind::Mesh, Some("Hero".to_string())),
        );
        meta
    }

    #[test]
    fn meta_roundtrip_json_and_file() {
        let temp = crate::test_support::tempdir();
        let meta = sample_meta();
        let meta_path = meta_path_for(&temp.path().join("hero.gltf"));
        meta.save(&meta_path).unwrap();

        let loaded = AssetMeta::load(&meta_path).unwrap();
        assert_eq!(loaded, meta);

        // 再次保存字节完全一致（确定性）
        let text = std::fs::read_to_string(&meta_path).unwrap();
        let mut again = temp.path().join("again.meta");
        again.set_extension("meta");
        loaded.save(&again).unwrap();
        assert_eq!(std::fs::read_to_string(&again).unwrap(), text);
    }

    #[test]
    fn meta_path_for_appends_extension() {
        let p = meta_path_for(Path::new("assets/models/hero.gltf"));
        assert!(p.ends_with("hero.gltf.meta"), "{}", p.display());
        assert!(is_meta_path(&p));
        assert!(!is_meta_path(Path::new("assets/hero.gltf")));
    }

    #[test]
    fn unrecognized_fields_are_preserved() {
        let text = r#"{
            "meta_version": 1,
            "uuid": "f6f9c1d2-1111-4222-8333-444455556666",
            "kind": "gltf",
            "custom_tool_data": { "note": "keep me" }
        }"#;
        let meta = AssetMeta::from_json(text).unwrap();
        assert!(meta.extra.contains_key("custom_tool_data"));
        let out = meta.to_json().unwrap();
        assert!(out.contains("custom_tool_data"));
        assert!(out.contains("keep me"));
    }

    #[test]
    fn future_meta_version_is_rejected() {
        let text = r#"{
            "meta_version": 99,
            "uuid": "f6f9c1d2-1111-4222-8333-444455556666",
            "kind": "gltf"
        }"#;
        let err = AssetMeta::from_json(text).unwrap_err();
        assert!(matches!(err, AssetError::UnsupportedMetaVersion(99)));
    }

    #[test]
    fn default_import_settings_depend_on_kind() {
        assert!(matches!(
            AssetMeta::create(AssetKind::Gltf).import_settings,
            ImportSettings::Gltf(_)
        ));
        assert!(matches!(
            AssetMeta::create(AssetKind::Texture).import_settings,
            ImportSettings::Texture(_)
        ));
        assert!(matches!(
            AssetMeta::create(AssetKind::Audio).import_settings,
            ImportSettings::Audio(_)
        ));
        assert!(matches!(
            AssetMeta::create(AssetKind::Binary).import_settings,
            ImportSettings::None
        ));
    }

    #[test]
    fn import_settings_json_shape() {
        let settings = ImportSettings::Gltf(GltfImportSettings {
            import_animations: false,
            ..Default::default()
        });
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            json,
            r#"{"importer":"gltf","settings":{"import_meshes":true,"import_materials":true,"import_textures":true,"import_skins":true,"import_animations":false,"scale":1.0}}"#
        );
        let back: ImportSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, settings);
    }

    #[test]
    fn missing_optional_fields_get_defaults() {
        let text = r#"{
            "uuid": "f6f9c1d2-1111-4222-8333-444455556666",
            "kind": "texture"
        }"#;
        let meta = AssetMeta::from_json(text).unwrap();
        assert_eq!(meta.meta_version, META_VERSION);
        assert!(matches!(meta.import_settings, ImportSettings::None));
        assert!(meta.dependencies.is_empty());
    }

    #[test]
    fn dependencies_are_sorted_and_deduplicated() {
        let mut meta = AssetMeta::create(AssetKind::Mesh);
        let ids: Vec<AssetId> = (0..5).map(|_| AssetId::new()).collect();
        let mut all = ids.clone();
        all.extend(ids.iter().copied());
        meta.set_dependencies(all.clone());
        let mut expected = ids.clone();
        expected.sort_unstable();
        assert_eq!(meta.dependencies, expected);
    }

    #[test]
    fn infer_kind_from_path() {
        assert_eq!(
            AssetMeta::infer_kind(Path::new("assets/hero.glb")),
            AssetKind::Gltf
        );
        assert_eq!(
            AssetMeta::infer_kind(Path::new("assets/albedo.PNG")),
            AssetKind::Texture
        );
        assert_eq!(
            AssetMeta::infer_kind(Path::new("assets/hit.ogg")),
            AssetKind::Audio
        );
        assert_eq!(
            AssetMeta::infer_kind(Path::new("assets/unknown.xyz")),
            AssetKind::Other
        );
    }
}
