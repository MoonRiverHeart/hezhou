//! # hezhou-asset — Hezhou 引擎级资产管线（0.2）
//!
//! 不绑定任何具体游戏：只提供“资产身份 / 元数据 / 依赖图 / 导入 / cook”这一层能力，
//! 由编辑器、Player、动画、渲染、音频等模块按需消费。
//!
//! ## 能力概览
//!
//! | 模块 | 能力 |
//! |---|---|
//! | [`id`] | [`AssetId`]（稳定 UUID）、[`AssetKind`]（资产类型）、[`AssetRef<K>`]（类型化引用） |
//! | [`path`] | 跨平台逻辑路径 [`LogicalPath`]（大小写不敏感、`/` 分隔、工程相对）与 [`AssetLocator`]（含子资产键） |
//! | [`meta`] | `.meta` JSON 读写：UUID、类型、导入设置、依赖、子资产表、前向兼容字段 |
//! | [`database`] | [`AssetDatabase`]：路径 ↔ ID、扫描、依赖图、glTF 导入注册 |
//! | [`import`] | 导入数据契约 + glTF/GLB 导入器（Mesh/Material/Texture/Skeleton/Animation） |
//! | [`cook`] | 依赖闭包 → 确定性 manifest + Player 产物 |
//! | [`math`] | 导入契约用的轻量数学类型（Vec2/3/4、Quat、Mat4、Trs、Aabb） |
//!
//! ## 核心保证
//!
//! 1. **引用稳定**：场景/Prefab/材质里保存的是 [`AssetId`]（UUID），不是路径。
//!    资产或整个工程移动目录后，重新扫描 `.meta` 即可恢复所有引用。
//! 2. **跨平台一致**：逻辑路径大小写不敏感比较、统一 `/`；Windows 与 Linux 得到同一份
//!    路径键与同一份 cook manifest。
//! 3. **确定性**：同一工程的扫描、依赖闭包、cook 产物在任意机器上逐字节一致。
//! 4. **只 cook 被引用资产**：从根资产做依赖闭包，未引用资产不进入 Player 包。
//!
//! ## 快速开始
//!
//! ```no_run
//! use hezhou_asset::{AssetDatabase, Cooker, LogicalPath};
//!
//! // 1. 打开工程（自动扫描 assets/、补齐 .meta）
//! let mut db = AssetDatabase::open("my_project")?;
//!
//! // 2. 路径 -> 稳定 ID
//! let hero_path = LogicalPath::new("assets/models/hero.gltf")?;
//! let hero_id = db.id_of(&hero_path).unwrap();
//!
//! // 3. 导入 glTF（产生 Mesh/Material/Texture/Skeleton/Animation 子资产）
//! let outcome = db.import_gltf(hero_id)?;
//! let mesh_id = outcome.sub_asset(hezhou_asset::AssetKind::Mesh, 0).unwrap().id;
//!
//! // 4. cook：只包含 mesh 及其依赖（父 glTF、材质、纹理、外部图片）
//! let manifest = Cooker::new(&db).plan(&[mesh_id])?;
//! assert!(manifest.entry(mesh_id).is_some());
//! # Ok::<(), hezhou_asset::error::AssetError>(())
//! ```
//!
//! ## Player 集成契约
//!
//! 详见 crate 根目录的 `README.md`。要点：
//!
//! - Player 只需要 `cook_manifest.json` + `imported/*.json` + `assets/*`；
//! - 运行时以 [`AssetId`] 为键查 manifest，加载 `output` 指向的文件；
//! - `imported/*.json` 是 [`import::ImportedScene`] 的 JSON 形式（几何已烘焙、纹理路径已重写）；
//! - 版本字段 `format_version` 用于 Player 启动时校验。
//!
//! ## feature
//!
//! - `gltf`（默认启用）：启用 [`import::gltf`] 真实 glTF/GLB 解析器。
//!   关闭后 `import` 模块保留完整数据契约与同签名桩函数，数据库/cook 仍可编译，
//!   调用导入时返回 [`error::AssetError::ImporterDisabled`]。
//! - `animation-bridge`（默认关闭）：启用 [`animation_bridge`]，把 [`import::ImportedScene`]
//!   适配成 `hezhou_animation::gltf::GltfAsset`（再调 `into_runtime()` 得到运行时动画数据）。
//!   默认关闭是为了让 asset 与 animation 两个 crate 的构建互不牵制。
//!
//! ## 非目标（0.2 首轮）
//!
//! - 不做 FBX/OBJ 导入、不做纹理压缩/平台变体烘焙、不做资源热重载 watcher；
//! - 不做 Addressables/分包、不做 Prefab 序列化（由上层场景模块实现）；
//! - 不修改引擎其它 crate（core/render/editor/Player 的接入由对应模块负责人完成）。

#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod cook;
pub mod database;
pub mod error;
pub mod id;
pub mod import;
pub mod math;
pub mod meta;
pub mod path;

/// 与 `hezhou-animation` 的 glTF 契约桥接（feature = `animation-bridge`）。
#[cfg(feature = "animation-bridge")]
pub mod animation_bridge;

#[cfg(feature = "animation-bridge")]
pub use animation_bridge::{imported_scene_to_gltf_asset, GltfAssetBridge};

pub use cook::{fnv1a64, fnv1a64_hex, CookEntry, CookManifest, CookOutput, Cooker};
pub use database::{
    AssetDatabase, AssetEntry, GltfImportOutcome, ImportedSubAsset, ScanReport, DEFAULT_ASSETS_ROOT,
};
pub use error::{AssetError, AssetResult};
pub use id::{
    Animation, AnimationRef, AssetId, AssetKind, AssetKindTag, AssetRef, Audio, AudioRef, Font,
    FontRef, Material, MaterialRef, Mesh, MeshRef, Prefab, PrefabRef, Scene, SceneRef, Shader,
    ShaderRef, Skeleton, SkeletonRef, Texture, TextureRef,
};
pub use import::{
    validate_imported_scene, AlphaMode, AnimationInterpolation, AnimationProperty,
    ImportSourceFormat, ImportedAnimation, ImportedAnimationChannel, ImportedImage, ImportedJoint,
    ImportedMaterial, ImportedMesh, ImportedNode, ImportedPrimitive, ImportedSampler,
    ImportedScene, ImportedSkeleton, ImportedTexture, ImportedTextureRef, PrimitiveMode,
    TextureFilterMode, TextureWrapMode,
};
pub use math::{Aabb, Mat4, Quat, Trs, Vec2, Vec3, Vec4};
pub use meta::{
    is_meta_path, meta_path_for, AssetMeta, AudioImportSettings, GltfImportSettings,
    ImportSettings, SubAssetMeta, TextureFilter, TextureImportSettings, TextureWrap,
    META_EXTENSION, META_VERSION,
};
pub use path::{AssetLocator, LogicalPath};

/// 单元测试公共工具。
///
/// 不使用系统临时目录：部分 Windows 环境下 `%TEMP%` 不可写（沙箱/ACL），
/// 统一使用 crate 的 `target/test-tmp` 作为测试工作区。
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;

    /// 创建自动清理的临时目录（位于 `target/test-tmp`）。
    pub fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp");
        std::fs::create_dir_all(&base).expect("创建测试临时根目录失败");
        tempfile::tempdir_in(&base).expect("创建测试临时目录失败")
    }
}
