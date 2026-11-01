//! 资产标识、类型与类型化引用。
//!
//! 设计要点：
//!
//! - [`AssetId`] 是对工程稳定的 UUID。资产文件移动/工程目录改名都不会改变它，
//!   因此引用关系（场景、Prefab、材质、动画）永远通过 ID 而不是路径保存。
//! - [`AssetKind`] 描述资产类型（源文件类型 + 导入产物类型）。
//! - [`AssetRef<K>`] 是带类型的 ID 包装，避免把 Mesh 引用误传给需要 Texture 的接口。
//!
//! 序列化格式：`AssetId` 为 UUID 字符串（如 `"f6f9c1d2-..."`），
//! `AssetRef<K>` 与裸 `AssetId` 完全一致，便于场景/`.meta` JSON 直接读写。

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::error::{AssetError, AssetResult};

/// 资产唯一标识符（UUID）。
/// [`AssetId::nil`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(Uuid);

impl AssetId {
    /// 生成新的随机 ID（UUID v4），用于首次导入时创建资产。
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 用已有 UUID 构造 ID（从 `.meta` 加载时使用）。
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// 全零 ID，表示“无资产”占位。
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// 底层 UUID。
    pub const fn uuid(&self) -> Uuid {
        self.0
    }

    /// 是否为全零 ID。
    pub const fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    /// 解析 UUID 字符串。
    pub fn parse(s: &str) -> AssetResult<Self> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| AssetError::InvalidPath(format!("非法 AssetId {s:?}: {e}")))
    }

    /// 版本 5（SHA-1，确定性）派生 ID。
    ///
    /// 用于从父资产 + 子资产键派生稳定的子资产 ID：
    /// 同一份 glTF 在任意机器、任意工程路径下导入都会得到相同的子资产 UUID。
    pub fn derive_child(&self, kind: AssetKind, key: &str) -> Self {
        let name = format!("{}:{}", kind.as_str(), key);
        Self(Uuid::new_v5(&self.0, name.as_bytes()))
    }

    /// 紧凑字符串形式（无连字符），可用于生成的 cooked 文件名。
    pub fn simple(&self) -> String {
        self.0.simple().to_string()
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::nil()
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<Uuid> for AssetId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<AssetId> for Uuid {
    fn from(value: AssetId) -> Self {
        value.0
    }
}

impl FromStr for AssetId {
    type Err = AssetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// 资产类型。
///
/// 源文件类型（`Gltf`/`Texture`/`Audio`/...）和导入产物类型（`Mesh`/`Material`/
/// `Skeleton`/`Animation`）共用同一个枚举，便于依赖图和 cook manifest 统一描述。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// glTF / GLB 源文件（导入后产生 Mesh/Material/Texture/Skeleton/Animation 子资产）。
    Gltf,
    /// 网格（源文件如 `.obj`，或 glTF 导入产物）。
    Mesh,
    /// 材质。
    Material,
    /// 纹理（源图片，或 glTF 导入产物）。
    Texture,
    /// 骨骼 / 蒙皮。
    Skeleton,
    /// 动画片段。
    Animation,
    /// 音频（wav/ogg/mp3/flac）。
    Audio,
    /// 场景。
    Scene,
    /// Prefab。
    Prefab,
    /// 着色器源码或 SPIR-V。
    Shader,
    /// 字体。
    Font,
    /// 二进制数据。
    Binary,
    /// 未识别类型。
    Other,
}

impl AssetKind {
    /// 稳定的字符串形式（用于 `.meta`/manifest 以及子资产键）。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gltf => "gltf",
            Self::Mesh => "mesh",
            Self::Material => "material",
            Self::Texture => "texture",
            Self::Skeleton => "skeleton",
            Self::Animation => "animation",
            Self::Audio => "audio",
            Self::Scene => "scene",
            Self::Prefab => "prefab",
            Self::Shader => "shader",
            Self::Font => "font",
            Self::Binary => "binary",
            Self::Other => "other",
        }
    }

    /// 由文件扩展名（不含点，大小写不敏感）推断资产类型。
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "gltf" | "glb" => Self::Gltf,
            "obj" | "fbx" | "ply" => Self::Mesh,
            "png" | "jpg" | "jpeg" | "tga" | "bmp" | "hdr" | "ktx2" | "dds" | "webp" => {
                Self::Texture
            }
            "mat" | "material" => Self::Material,
            "wav" | "ogg" | "mp3" | "flac" => Self::Audio,
            "vert" | "frag" | "comp" | "glsl" | "hlsl" | "wgsl" | "spv" => Self::Shader,
            "ttf" | "otf" => Self::Font,
            "scene" | "hzscene" | "hezhou" => Self::Scene,
            "prefab" => Self::Prefab,
            "bin" => Self::Binary,
            _ => Self::Other,
        }
    }

    /// 由字符串名解析资产类型（`as_str` 的逆操作）。
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "gltf" => Self::Gltf,
            "mesh" => Self::Mesh,
            "material" => Self::Material,
            "texture" => Self::Texture,
            "skeleton" => Self::Skeleton,
            "animation" => Self::Animation,
            "audio" => Self::Audio,
            "scene" => Self::Scene,
            "prefab" => Self::Prefab,
            "shader" => Self::Shader,
            "font" => Self::Font,
            "binary" => Self::Binary,
            "other" => Self::Other,
            _ => return None,
        })
    }

    /// 该类型是否只能由导入器产生（不是工程里的源文件）。
    pub const fn is_import_artifact(self) -> bool {
        matches!(
            self,
            Self::Mesh | Self::Material | Self::Skeleton | Self::Animation
        )
    }

    /// 该类型是否能作为源文件出现在 `assets/` 目录里并被 `.meta` 管理。
    ///
    /// 纹理既可能是源文件（`.png`），也可能是 glTF 导入产物，因此也返回 true。
    pub const fn can_be_source_file(self) -> bool {
        !matches!(self, Self::Skeleton | Self::Animation)
    }
}

impl fmt::Display for AssetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 类型化引用所需的类型标签。
///
/// 使用者不需要直接实现该 trait，使用本 crate 提供的 [`Mesh`]、[`Texture`] 等标记类型即可。
pub trait AssetKindTag: 'static {
    /// 引用对应的资产类型。
    const KIND: AssetKind;
}

macro_rules! define_asset_kind_tags {
    ($(
        $(#[$meta:meta])*
        $name:ident => $kind:expr
    ),* $(,)?) => {
        $(
            $(#[$meta])*
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
            pub struct $name;

            impl AssetKindTag for $name {
                const KIND: AssetKind = $kind;
            }
        )*
    };
}

define_asset_kind_tags! {
    /// 网格资产标记。
    Mesh => AssetKind::Mesh,
    /// 材质资产标记。
    Material => AssetKind::Material,
    /// 纹理资产标记。
    Texture => AssetKind::Texture,
    /// 骨骼资产标记。
    Skeleton => AssetKind::Skeleton,
    /// 动画资产标记。
    Animation => AssetKind::Animation,
    /// 音频资产标记。
    Audio => AssetKind::Audio,
    /// 场景资产标记。
    Scene => AssetKind::Scene,
    /// Prefab 资产标记。
    Prefab => AssetKind::Prefab,
    /// 着色器资产标记。
    Shader => AssetKind::Shader,
    /// 字体资产标记。
    Font => AssetKind::Font,
}

/// 类型化资产引用：把 [`AssetId`] 与期望类型绑定。
///
/// 序列化格式与裸 `AssetId` 相同（纯 UUID 字符串），因此可以直接出现在场景 JSON、
/// Prefab JSON 或 `.meta` 的依赖列表里。类型校验发生在
/// [`crate::database::AssetDatabase::typed_ref`] / [`AssetRef::validate`]。
pub struct AssetRef<K: AssetKindTag> {
    id: AssetId,
    _marker: PhantomData<fn() -> K>,
}

impl<K: AssetKindTag> AssetRef<K> {
    /// 由裸 ID 构造引用。
    ///
    /// 该函数不做类型校验（可能拿不到数据库）；需要强校验时用
    /// [`crate::database::AssetDatabase::typed_ref`]。
    pub const fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    /// 裸 ID。
    pub const fn id(&self) -> AssetId {
        self.id
    }

    /// 引用类型。
    pub const fn kind(&self) -> AssetKind {
        K::KIND
    }

    /// 取出裸 ID。
    pub const fn into_id(self) -> AssetId {
        self.id
    }

    /// 校验数据库中的资产类型是否与引用类型一致。
    pub fn validate(&self, entry: &crate::database::AssetEntry) -> AssetResult<()> {
        if entry.kind == K::KIND {
            Ok(())
        } else {
            Err(AssetError::KindMismatch {
                expected: K::KIND,
                actual: entry.kind,
            })
        }
    }
}

impl<K: AssetKindTag> Clone for AssetRef<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: AssetKindTag> Copy for AssetRef<K> {}

impl<K: AssetKindTag> PartialEq for AssetRef<K> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<K: AssetKindTag> Eq for AssetRef<K> {}

impl<K: AssetKindTag> PartialOrd for AssetRef<K> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: AssetKindTag> Ord for AssetRef<K> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<K: AssetKindTag> std::hash::Hash for AssetRef<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<K: AssetKindTag> fmt::Debug for AssetRef<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AssetRef<{}>({})", K::KIND.as_str(), self.id)
    }
}

impl<K: AssetKindTag> fmt::Display for AssetRef<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.id, f)
    }
}

impl<K: AssetKindTag> Serialize for AssetRef<K> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.id.serialize(serializer)
    }
}

impl<'de, K: AssetKindTag> Deserialize<'de> for AssetRef<K> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        AssetId::deserialize(deserializer).map(Self::new)
    }
}

impl<K: AssetKindTag> From<AssetRef<K>> for AssetId {
    fn from(value: AssetRef<K>) -> Self {
        value.id
    }
}

/// 网格引用。
pub type MeshRef = AssetRef<Mesh>;
/// 材质引用。
pub type MaterialRef = AssetRef<Material>;
/// 纹理引用。
pub type TextureRef = AssetRef<Texture>;
/// 骨骼引用。
pub type SkeletonRef = AssetRef<Skeleton>;
/// 动画引用。
pub type AnimationRef = AssetRef<Animation>;
/// 音频引用。
pub type AudioRef = AssetRef<Audio>;
/// 场景引用。
pub type SceneRef = AssetRef<Scene>;
/// Prefab 引用。
pub type PrefabRef = AssetRef<Prefab>;
/// 着色器引用。
pub type ShaderRef = AssetRef<Shader>;
/// 字体引用。
pub type FontRef = AssetRef<Font>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_id_string_roundtrip() {
        let id = AssetId::new();
        let text = id.to_string();
        assert_eq!(AssetId::parse(&text).unwrap(), id);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{text}\""));
        let back: AssetId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn derive_child_is_deterministic_and_kind_separated() {
        let parent = AssetId::new();
        let a = parent.derive_child(AssetKind::Mesh, "mesh:0");
        let b = parent.derive_child(AssetKind::Mesh, "mesh:0");
        assert_eq!(a, b);
        let c = parent.derive_child(AssetKind::Animation, "mesh:0");
        assert_ne!(a, c);
        // 不同父资产派生不同 ID
        let other = AssetId::new().derive_child(AssetKind::Mesh, "mesh:0");
        assert_ne!(a, other);
    }

    #[test]
    fn kind_from_extension_is_case_insensitive() {
        assert_eq!(AssetKind::from_extension("GLB"), AssetKind::Gltf);
        assert_eq!(AssetKind::from_extension("Png"), AssetKind::Texture);
        assert_eq!(AssetKind::from_extension("ogg"), AssetKind::Audio);
        assert_eq!(AssetKind::from_extension("unknown"), AssetKind::Other);
    }

    #[test]
    fn typed_ref_serializes_as_plain_uuid() {
        let id = AssetId::new();
        let mesh_ref = MeshRef::new(id);
        let json = serde_json::to_string(&mesh_ref).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        let back: MeshRef = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id(), id);
        assert_eq!(back.kind(), AssetKind::Mesh);
        // Display 与裸 ID 一致
        assert_eq!(mesh_ref.to_string(), id.to_string());
    }

    #[test]
    fn typed_ref_ordering_follows_id() {
        let a = MeshRef::new(AssetId::new());
        let b = MeshRef::new(AssetId::new());
        let (lo, hi) = if a.id() < b.id() { (a, b) } else { (b, a) };
        assert!(lo < hi);
    }
}
