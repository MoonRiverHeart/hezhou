//! Prefab 资产与实例化（0.2 引擎能力，不含编辑器 UI）。
//!
//! Prefab 是"可复用的实体子树模板"，以 JSON 资产形式保存；项目/场景加载时通过
//! [`Scene::instantiate_prefab`] 展开为普通 ECS 实体（0.2 的项目文件保存展开后的实体，
//! 不保存 prefab 引用）。
//!
//! # JSON 格式
//!
//! ```json
//! {
//!   "version": "1.0",
//!   "name": "Enemy",
//!   "description": "通用敌人模板",
//!   "root": {
//!     "name": "EnemyRoot",
//!     "transform": { "position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1] },
//!     "renderable": { "mesh_path": "builtin://cube", "material": null },
//!     "visible": true,
//!     "scripts": [{ "script_path": "scripts/Enemy.cs", "class_name": "Enemy", "enabled": true }],
//!     "children": []
//!   }
//! }
//! ```
//!
//! - `rotation` 使用**欧拉角（度）**，与 `ProjectEntity.transform` 一致；
//! - 所有可选字段都有默认值，未知字段被忽略（前向兼容）；
//! - `version` 主版本必须为 1，否则加载失败（避免旧引擎误读未来格式）。
//!
//! # 一层嵌套
//!
//! `PrefabEntity.nested_prefab = { "prefab": "<registry key>", "overrides": {...} }`
//! 表示该实体下挂一个其它 Prefab 的根实体（该实体本身作为父级/定位用）。
//! 只允许一层嵌套（根 Prefab + 子 Prefab），更深或循环引用会返回
//! [`PrefabError::NestingTooDeep`] / [`PrefabError::CircularReference`]。
//!
//! # 与 Project / Asset / Editor 的接入契约
//!
//! - **Asset**：Prefab 文件是普通 JSON 资产，按资产路径/AssetId 定位；
//!   `Prefab::load_file` / `save_file` 可直接用于资产管线，不依赖 `hezhou-asset`。
//! - **Project**：Prefab 实例化后就是普通实体，`Project::sync_from_scene` 正常保存；
//!   0.2 项目文件不记录"这是 prefab 实例"（避免 Project 格式升级），
//!   如需在编辑器里显示 prefab 来源，由编辑器自行维护映射。
//! - **Editor**：建议 FFI/命令层提供
//!   `prefab_capture_from_scene(root_entity, name)`、`prefab_instantiate(json, parent, overrides)`、
//!   `scene_manager_switch(scene_name)`；这些 FFI 不在本任务范围内。
//! - **PrefabRegistry**：由资产层/编辑器在加载时把 `prefab.name -> Prefab` 注册进来，
//!   供一层嵌套解析；registry key 建议使用资产稳定路径或 AssetId 字符串。

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ecs::{
    BoundsComponent, DirectionalLightComponent, Entity, LocalTransform, RenderableComponent, Scene,
};
use crate::math::{Quaternion, Vec3};

/// 当前 Prefab JSON 格式版本。
pub const PREFAB_FORMAT_VERSION: &str = "1.0";

/// 支持的主版本号；更高主版本的 Prefab 会拒绝加载。
pub const PREFAB_SUPPORTED_MAJOR_VERSION: u32 = 1;

/// 允许的嵌套层数（1 = 根 Prefab 下可挂一个其它 Prefab）。
pub const MAX_PREFAB_NESTING_DEPTH: usize = 1;

fn default_prefab_version() -> String {
    PREFAB_FORMAT_VERSION.to_string()
}

/// Prefab 资产根。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Prefab {
    /// 格式版本（默认 "1.0"）。
    #[serde(default = "default_prefab_version")]
    pub version: String,
    /// Prefab 名称（同时可作为 [`PrefabRegistry`] 的默认 key）。
    #[serde(default)]
    pub name: String,
    /// 描述/说明，供编辑器展示。
    #[serde(default)]
    pub description: String,
    /// 根实体子树。
    pub root: PrefabEntity,
}

/// Prefab 中的一个实体。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct PrefabEntity {
    /// 实体名称（实例化后写入 `Scene::entity_names`）。
    pub name: String,
    /// 说明字段。
    pub description: String,
    /// 本地变换。
    pub transform: PrefabTransform,
    /// 可渲染组件；`None` 表示纯逻辑实体。
    pub renderable: Option<PrefabRenderable>,
    /// 方向光组件；`None` 表示非灯光实体。
    pub light: Option<PrefabLight>,
    /// 脚本绑定（实例化时重建 ScriptComponent + ScriptBinding）。
    pub scripts: Vec<PrefabScriptBinding>,
    /// 是否可见（作用于 renderable.visible）。
    pub visible: bool,
    /// 子实体。
    pub children: Vec<PrefabEntity>,
    /// 一层嵌套：该实体下挂载另一个 Prefab 的根实体。
    pub nested_prefab: Option<PrefabReference>,
}

impl Default for PrefabEntity {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            transform: PrefabTransform::default(),
            renderable: None,
            light: None,
            scripts: Vec::new(),
            visible: true,
            children: Vec::new(),
            nested_prefab: None,
        }
    }
}

impl PrefabEntity {
    /// 构造一个空实体（可见、单位变换）。
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn with_transform(mut self, transform: PrefabTransform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_renderable(mut self, renderable: PrefabRenderable) -> Self {
        self.renderable = Some(renderable);
        self
    }

    pub fn with_scripts(mut self, scripts: Vec<PrefabScriptBinding>) -> Self {
        self.scripts = scripts;
        self
    }

    pub fn with_children(mut self, children: Vec<PrefabEntity>) -> Self {
        self.children = children;
        self
    }

    pub fn with_nested_prefab(mut self, reference: PrefabReference) -> Self {
        self.nested_prefab = Some(reference);
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

/// Prefab 实体的本地变换。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct PrefabTransform {
    /// 本地位置。
    pub position: [f32; 3],
    /// 本地旋转（欧拉角，度）。
    pub rotation: [f32; 3],
    /// 本地缩放。
    pub scale: [f32; 3],
}

impl Default for PrefabTransform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl PrefabTransform {
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            ..Self::default()
        }
    }

    /// 从 ECS 本地变换转换（rotation 转欧拉角度数）。
    pub fn from_local_transform(transform: &LocalTransform) -> Self {
        Self {
            position: [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            rotation: transform.rotation.to_euler_degrees(),
            scale: [transform.scale.x, transform.scale.y, transform.scale.z],
        }
    }

    /// 转为 ECS 本地变换。
    pub fn to_local_transform(self) -> LocalTransform {
        LocalTransform {
            position: Vec3::new(self.position[0], self.position[1], self.position[2]),
            rotation: Quaternion::from_euler_degrees(
                self.rotation[0],
                self.rotation[1],
                self.rotation[2],
            ),
            scale: Vec3::new(self.scale[0], self.scale[1], self.scale[2]),
        }
    }
}

/// 可渲染数据（对应 `RenderableComponent`）。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct PrefabRenderable {
    /// 网格路径（如 `builtin://cube` 或资产路径）。
    pub mesh_path: String,
    /// 材质路径。
    pub material: Option<String>,
    /// 纹理路径。
    pub texture: Option<String>,
    /// 是否投射阴影。
    pub cast_shadow: bool,
    /// 是否接收阴影。
    pub receive_shadow: bool,
}

impl Default for PrefabRenderable {
    fn default() -> Self {
        Self {
            mesh_path: String::new(),
            material: None,
            texture: None,
            cast_shadow: true,
            receive_shadow: true,
        }
    }
}

impl PrefabRenderable {
    pub fn new(mesh_path: impl Into<String>) -> Self {
        Self {
            mesh_path: mesh_path.into(),
            ..Self::default()
        }
    }
}

/// 方向光数据（对应 `DirectionalLightComponent`）。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct PrefabLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

impl Default for PrefabLight {
    fn default() -> Self {
        let light = DirectionalLightComponent::default();
        Self {
            direction: [light.direction.x, light.direction.y, light.direction.z],
            color: [light.color.x, light.color.y, light.color.z],
            intensity: light.intensity,
        }
    }
}

impl PrefabLight {
    pub fn from_component(light: &DirectionalLightComponent) -> Self {
        Self {
            direction: [light.direction.x, light.direction.y, light.direction.z],
            color: [light.color.x, light.color.y, light.color.z],
            intensity: light.intensity,
        }
    }

    pub fn to_component(self) -> DirectionalLightComponent {
        DirectionalLightComponent {
            direction: Vec3::new(self.direction[0], self.direction[1], self.direction[2]),
            color: Vec3::new(self.color[0], self.color[1], self.color[2]),
            intensity: self.intensity,
        }
    }
}

/// 脚本绑定数据。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct PrefabScriptBinding {
    /// 脚本路径。
    pub script_path: String,
    /// C# 类名。
    pub class_name: String,
    /// 是否启用。
    pub enabled: bool,
}

impl Default for PrefabScriptBinding {
    fn default() -> Self {
        Self {
            script_path: String::new(),
            class_name: String::new(),
            enabled: true,
        }
    }
}

impl PrefabScriptBinding {
    pub fn new(script_path: impl Into<String>, class_name: impl Into<String>) -> Self {
        Self {
            script_path: script_path.into(),
            class_name: class_name.into(),
            enabled: true,
        }
    }
}

/// 实例化时对 Prefab 根的覆盖（基础版，不含 Inspector 全量能力）。
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct PrefabOverrides {
    /// 覆盖根实体名称。
    pub name: Option<String>,
    /// 覆盖根实体位置。
    pub position: Option<[f32; 3]>,
    /// 覆盖根实体旋转（欧拉角，度）。
    pub rotation: Option<[f32; 3]>,
    /// 覆盖根实体缩放。
    pub scale: Option<[f32; 3]>,
    /// 覆盖根实体可见性（仅当根带有 Renderable 时生效）。
    pub visible: Option<bool>,
}

impl PrefabOverrides {
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = Some([position.x, position.y, position.z]);
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 是否没有任何覆盖项。
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.position.is_none()
            && self.rotation.is_none()
            && self.scale.is_none()
            && self.visible.is_none()
    }
}

/// 嵌套 Prefab 引用。
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct PrefabReference {
    /// [`PrefabRegistry`] 的 key（建议使用稳定资产路径/名称）。
    pub prefab: String,
    /// 对嵌套 Prefab 根的覆盖。
    pub overrides: PrefabOverrides,
}

impl PrefabReference {
    pub fn new(prefab: impl Into<String>) -> Self {
        Self {
            prefab: prefab.into(),
            overrides: PrefabOverrides::default(),
        }
    }

    /// 设置嵌套 Prefab 根的覆盖。
    pub fn with_overrides(mut self, overrides: PrefabOverrides) -> Self {
        self.overrides = overrides;
        self
    }
}

/// Prefab 相关错误。
#[derive(Clone, Debug, PartialEq)]
pub enum PrefabError {
    /// JSON 解析失败。
    Json(String),
    /// 文件读写失败。
    Io(String),
    /// 不支持的格式主版本。
    UnsupportedVersion { found: String, supported: String },
    /// 嵌套引用的 Prefab 不在 registry 中。
    MissingPrefab(String),
    /// 有嵌套引用但未提供 registry。
    MissingRegistry(String),
    /// 循环引用（路径为引用栈）。
    CircularReference { path: Vec<String> },
    /// 嵌套层数超限。
    NestingTooDeep { max_depth: usize },
    /// 用不存在的实体做 capture。
    EntityNotFound(u64),
    /// 实例化时指定的父实体不存在。
    InvalidParent(u64),
}

impl std::fmt::Display for PrefabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(err) => write!(f, "prefab json error: {err}"),
            Self::Io(err) => write!(f, "prefab io error: {err}"),
            Self::UnsupportedVersion { found, supported } => {
                write!(
                    f,
                    "unsupported prefab version {found} (supported {supported})"
                )
            }
            Self::MissingPrefab(key) => write!(f, "nested prefab not found in registry: {key}"),
            Self::MissingRegistry(key) => {
                write!(
                    f,
                    "prefab {key} has nested references but no registry was provided"
                )
            }
            Self::CircularReference { path } => {
                write!(f, "circular prefab reference: {}", path.join(" -> "))
            }
            Self::NestingTooDeep { max_depth } => {
                write!(f, "prefab nesting exceeds max depth {max_depth}")
            }
            Self::EntityNotFound(id) => write!(f, "capture entity not found: {id}"),
            Self::InvalidParent(id) => write!(f, "instantiate parent entity not found: {id}"),
        }
    }
}

impl std::error::Error for PrefabError {}

impl From<serde_json::Error> for PrefabError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl From<std::io::Error> for PrefabError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Prefab 注册表：供一层嵌套解析（key 建议使用稳定资产路径）。
#[derive(Clone, Debug, Default)]
pub struct PrefabRegistry {
    prefabs: HashMap<String, Prefab>,
}

impl PrefabRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 以 `prefab.name` 为 key 插入（空名称时返回 `false`）。
    pub fn insert(&mut self, prefab: Prefab) -> bool {
        let name = prefab.name.trim().to_string();
        if name.is_empty() {
            return false;
        }
        self.insert_with_key(name, prefab);
        true
    }

    /// 以指定 key 插入/替换。
    pub fn insert_with_key(&mut self, key: impl Into<String>, prefab: Prefab) {
        self.prefabs.insert(key.into(), prefab);
    }

    pub fn get(&self, key: &str) -> Option<&Prefab> {
        self.prefabs.get(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.prefabs.contains_key(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<Prefab> {
        self.prefabs.remove(key)
    }

    pub fn len(&self) -> usize {
        self.prefabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prefabs.is_empty()
    }

    /// 全部 key（排序后返回，便于 UI/测试稳定输出）。
    pub fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.prefabs.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }
}

/// Prefab 实例化选项。
#[derive(Clone, Copy, Debug, Default)]
pub struct PrefabInstantiateOptions<'a> {
    /// 挂载父实体；`None` 表示作为根实体。
    pub parent: Option<Entity>,
    /// 应用到 Prefab 根的覆盖。
    pub overrides: Option<&'a PrefabOverrides>,
    /// 嵌套 Prefab 注册表；没有嵌套时可省略。
    pub registry: Option<&'a PrefabRegistry>,
}

impl<'a> PrefabInstantiateOptions<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(mut self, parent: Entity) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_overrides(mut self, overrides: &'a PrefabOverrides) -> Self {
        self.overrides = Some(overrides);
        self
    }

    pub fn with_registry(mut self, registry: &'a PrefabRegistry) -> Self {
        self.registry = Some(registry);
        self
    }
}

/// 一次 Prefab 实例化的结果。
#[derive(Clone, Debug, Default)]
pub struct PrefabInstance {
    /// 根实体。
    pub root: Entity,
    /// 所有新实体（深度优先创建顺序）。
    pub entities: Vec<Entity>,
    /// 与 `entities` 一一对应的层级路径（如 `Root/Weapon`；重名追加 `#2`）。
    pub paths: Vec<String>,
    by_path: HashMap<String, Entity>,
}

impl PrefabInstance {
    /// 实例化出的实体数量。
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    /// 按层级路径取实体。
    pub fn entity_at(&self, path: &str) -> Option<Entity> {
        self.by_path.get(path).copied()
    }

    /// 按实体反查层级路径。
    pub fn path_of(&self, entity: Entity) -> Option<&str> {
        self.entities
            .iter()
            .position(|candidate| *candidate == entity)
            .map(|index| self.paths[index].as_str())
    }

    /// 层级路径 → 实体映射。
    pub fn by_path(&self) -> &HashMap<String, Entity> {
        &self.by_path
    }
}

/// 实例化内部上下文（引用栈 + 结果累积）。
struct InstantiateContext<'a> {
    registry: Option<&'a PrefabRegistry>,
    stack: Vec<String>,
    instance: PrefabInstance,
}

impl Prefab {
    /// 构造 Prefab。
    pub fn new(name: impl Into<String>, root: PrefabEntity) -> Self {
        Self {
            version: default_prefab_version(),
            name: name.into(),
            description: String::new(),
            root,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// 从 JSON 字符串解析，并校验格式主版本。
    pub fn from_json(json: &str) -> Result<Self, PrefabError> {
        let prefab: Self = serde_json::from_str(json)?;
        prefab.validate_version()?;
        Ok(prefab)
    }

    /// 序列化为格式化 JSON。
    pub fn to_json(&self) -> Result<String, PrefabError> {
        serde_json::to_string_pretty(self).map_err(PrefabError::from)
    }

    /// 保存到文件（自动创建父目录）。
    pub fn save_file(&self, path: impl AsRef<Path>) -> Result<(), PrefabError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    /// 从文件加载。
    pub fn load_file(path: impl AsRef<Path>) -> Result<Self, PrefabError> {
        let json = std::fs::read_to_string(path.as_ref())?;
        Self::from_json(&json)
    }

    /// 校验格式版本主版本号。
    pub fn validate_version(&self) -> Result<(), PrefabError> {
        let major = self
            .version
            .split('.')
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .unwrap_or(0);
        if major == PREFAB_SUPPORTED_MAJOR_VERSION {
            Ok(())
        } else {
            Err(PrefabError::UnsupportedVersion {
                found: self.version.clone(),
                supported: PREFAB_FORMAT_VERSION.to_string(),
            })
        }
    }

    /// 从场景中的实体子树捕获 Prefab（编辑器"另存为 Prefab"入口）。
    ///
    /// 捕获会展开嵌套实例为普通实体（0.2 不保留来源信息）。
    pub fn capture_from_scene(
        scene: &Scene,
        root: Entity,
        name: impl Into<String>,
    ) -> Result<Self, PrefabError> {
        if !scene.world.entity_exists(root) {
            return Err(PrefabError::EntityNotFound(root.id));
        }
        Ok(Self {
            version: default_prefab_version(),
            name: name.into(),
            description: String::new(),
            root: capture_entity(scene, root),
        })
    }
}

// LocalTransform 的 Default 是零四元数（非单位旋转），不能用 unwrap_or_default 代替 new()。
#[allow(clippy::unwrap_or_default)]
fn capture_entity(scene: &Scene, entity: Entity) -> PrefabEntity {
    let transform = scene
        .world
        .get_component::<LocalTransform>(entity)
        .unwrap_or_else(LocalTransform::new);
    let renderable = scene.world.get_component::<RenderableComponent>(entity);
    let light = scene
        .world
        .get_component::<DirectionalLightComponent>(entity);

    let scripts = scene
        .entity_bindings
        .get(&entity.id)
        .map(|bindings| {
            bindings
                .iter()
                .map(|binding| PrefabScriptBinding {
                    script_path: binding.script_path.clone(),
                    class_name: binding.class_name.clone(),
                    enabled: binding.enabled,
                })
                .collect()
        })
        .unwrap_or_default();

    let visible = renderable
        .as_ref()
        .map(|renderable| renderable.visible)
        .unwrap_or(true);

    // 父子关系来自 HashMap，按实体 ID 排序保证捕获结果稳定。
    let mut child_entities = scene.get_children(entity);
    child_entities.sort_by_key(|child| child.id);

    PrefabEntity {
        name: scene.get_entity_name(entity).cloned().unwrap_or_default(),
        description: String::new(),
        transform: PrefabTransform::from_local_transform(&transform),
        renderable: renderable.map(|renderable| PrefabRenderable {
            mesh_path: renderable.mesh_path,
            material: renderable.material_path,
            texture: renderable.texture_path,
            cast_shadow: renderable.cast_shadow,
            receive_shadow: renderable.receive_shadow,
        }),
        light: light.map(|light| PrefabLight::from_component(&light)),
        scripts,
        visible,
        children: child_entities
            .into_iter()
            .map(|child| capture_entity(scene, child))
            .collect(),
        nested_prefab: None,
    }
}

impl Scene {
    /// 实例化 Prefab（无覆盖、无嵌套 registry 的便捷入口）。
    ///
    /// 返回根实体与实体映射；失败时不会留下部分实体（当前实现会先创建，
    /// 调用方应把 Err 视为配置错误，在编辑器/资产加载阶段拦截）。
    pub fn instantiate_prefab(
        &mut self,
        prefab: &Prefab,
        parent: Option<Entity>,
    ) -> Result<PrefabInstance, PrefabError> {
        self.instantiate_prefab_with(
            prefab,
            &PrefabInstantiateOptions {
                parent,
                ..PrefabInstantiateOptions::default()
            },
        )
    }

    /// 带覆盖/父级/嵌套 registry 的完整实例化入口。
    pub fn instantiate_prefab_with(
        &mut self,
        prefab: &Prefab,
        options: &PrefabInstantiateOptions<'_>,
    ) -> Result<PrefabInstance, PrefabError> {
        prefab.validate_version()?;
        if let Some(parent) = options.parent {
            if !self.world.entity_exists(parent) {
                return Err(PrefabError::InvalidParent(parent.id));
            }
        }

        let mut context = InstantiateContext {
            registry: options.registry,
            stack: vec![prefab.name.clone()],
            instance: PrefabInstance::default(),
        };
        let root =
            self.instantiate_prefab_entity(&prefab.root, options.parent, 0, &mut context, "")?;
        if let Some(overrides) = options.overrides {
            self.apply_prefab_overrides(root, overrides);
        }
        context.instance.root = root;
        Ok(context.instance)
    }

    /// 把 [`PrefabOverrides`] 应用到已存在的实体（编辑器改实例属性时也可复用）。
    pub fn apply_prefab_overrides(&mut self, entity: Entity, overrides: &PrefabOverrides) {
        if !self.world.entity_exists(entity) {
            return;
        }
        if let Some(name) = &overrides.name {
            self.set_entity_name(entity, name.clone());
        }
        if let Some(position) = overrides.position {
            self.set_entity_position(entity, Vec3::new(position[0], position[1], position[2]));
        }
        if let Some(rotation) = overrides.rotation {
            self.set_entity_rotation(
                entity,
                Quaternion::from_euler_degrees(rotation[0], rotation[1], rotation[2]),
            );
        }
        if let Some(scale) = overrides.scale {
            self.set_entity_scale(entity, Vec3::new(scale[0], scale[1], scale[2]));
        }
        if let Some(visible) = overrides.visible {
            self.set_renderable_visible(entity, visible);
        }
    }

    /// 递归实例化一个实体及其子实体/嵌套 Prefab。
    fn instantiate_prefab_entity(
        &mut self,
        spec: &PrefabEntity,
        parent: Option<Entity>,
        prefab_depth: usize,
        context: &mut InstantiateContext<'_>,
        parent_path: &str,
    ) -> Result<Entity, PrefabError> {
        let entity = self.create_entity();
        if let Some(parent) = parent {
            self.set_parent(entity, Some(parent));
        }

        // 名称与层级路径（重名追加 #N，保证稳定可查）。
        let display_name = if spec.name.trim().is_empty() {
            "Entity".to_string()
        } else {
            spec.name.clone()
        };
        if !spec.name.trim().is_empty() {
            self.set_entity_name(entity, spec.name.clone());
        }
        let raw_path = if parent_path.is_empty() {
            display_name
        } else {
            format!("{parent_path}/{display_name}")
        };
        let path = unique_instance_path(&raw_path, &context.instance.by_path);
        context.instance.entities.push(entity);
        context.instance.paths.push(path.clone());
        context.instance.by_path.insert(path.clone(), entity);

        // Transform
        self.world
            .add_component(entity, spec.transform.to_local_transform());

        // Renderable（visible 在实体级统一处理）
        if let Some(renderable) = &spec.renderable {
            let component = RenderableComponent {
                mesh_path: renderable.mesh_path.clone(),
                material_path: renderable.material.clone(),
                texture_path: renderable.texture.clone(),
                visible: spec.visible,
                cast_shadow: renderable.cast_shadow,
                receive_shadow: renderable.receive_shadow,
            };
            self.world.add_component(entity, component);
            self.world.add_component(entity, BoundsComponent::cube(2.0));
        }

        // Directional light
        if let Some(light) = &spec.light {
            self.world.add_component(entity, light.to_component());
        }

        // 脚本绑定：重建 ScriptComponent + ScriptBinding，保留 enabled 状态。
        for script in &spec.scripts {
            if script.script_path.is_empty() && script.class_name.is_empty() {
                continue;
            }
            self.attach_script_binding(
                entity,
                script.script_path.clone(),
                script.class_name.clone(),
            );
            let index = self.get_script_binding_count(entity).saturating_sub(1);
            self.set_script_binding_enabled(entity, index, script.enabled);
        }

        // 普通子实体
        for child in &spec.children {
            self.instantiate_prefab_entity(child, Some(entity), prefab_depth, context, &path)?;
        }

        // 一层嵌套 Prefab：先查引用栈（循环），再查深度，最后递归。
        if let Some(reference) = &spec.nested_prefab {
            let registry = context
                .registry
                .ok_or_else(|| PrefabError::MissingRegistry(reference.prefab.clone()))?;
            let nested = registry
                .get(&reference.prefab)
                .ok_or_else(|| PrefabError::MissingPrefab(reference.prefab.clone()))?;
            let nested_name = nested.name.clone();
            let nested_root_spec = nested.root.clone();
            let nested_overrides = reference.overrides.clone();

            if context
                .stack
                .iter()
                .any(|name| name == &reference.prefab || name == &nested_name)
            {
                let mut path = context.stack.clone();
                path.push(nested_name);
                return Err(PrefabError::CircularReference { path });
            }
            if prefab_depth >= MAX_PREFAB_NESTING_DEPTH {
                return Err(PrefabError::NestingTooDeep {
                    max_depth: MAX_PREFAB_NESTING_DEPTH,
                });
            }

            context.stack.push(nested_name);
            let nested_root = self.instantiate_prefab_entity(
                &nested_root_spec,
                Some(entity),
                prefab_depth + 1,
                context,
                &path,
            )?;
            context.stack.pop();
            self.apply_prefab_overrides(nested_root, &nested_overrides);
        }

        Ok(entity)
    }
}

fn unique_instance_path(base: &str, existing: &HashMap<String, Entity>) -> String {
    if !existing.contains_key(base) {
        return base.to_string();
    }
    let mut index = 2usize;
    loop {
        let candidate = format!("{base}#{index}");
        if !existing.contains_key(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefab_error_display_is_readable() {
        let error = PrefabError::CircularReference {
            path: vec!["A".to_string(), "B".to_string(), "A".to_string()],
        };
        let text = error.to_string();
        assert!(text.contains("A -> B -> A"), "实际: {text}");
    }

    #[test]
    fn transform_roundtrip_between_json_and_local_transform() {
        let prefab_transform = PrefabTransform {
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 90.0, 0.0],
            scale: [2.0, 2.0, 2.0],
        };
        let local = prefab_transform.to_local_transform();
        let back = PrefabTransform::from_local_transform(&local);
        assert!((back.position[0] - 1.0).abs() < 1e-5);
        assert!(
            (back.rotation[1] - 90.0).abs() < 0.05,
            "实际: {:?}",
            back.rotation
        );
        assert!((back.scale[2] - 2.0).abs() < 1e-5);
        assert!(PrefabOverrides::default().is_empty());
    }
}
