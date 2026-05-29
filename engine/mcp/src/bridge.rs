//! EngineBridge trait — 纯Rust和C# FFI两条路径的统一抽象层
//!
//! 设计原则:
//! 1. 能力边界明确 — 每个方法标注[RustOnly]/[FfiOnly]/[Both]
//! 2. 结构分离 — RustBridge和FfiBridge是两个独立impl，feature gate隔离
//! 3. 纯Rust可脱离 — RustBridge不依赖scripting/Mono，可独立运行
//! 4. FFI利用C# API — FfiBridge复用EditorScript的全部封装逻辑

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use anyhow::Result;

// ============================================================
// JSON Schema兼容类型 — 不生成非标准format(uint64/uint/float)
// JSON Schema draft 2020-12不支持这些format，OpenCode等MCP Host会报警
// 用newtype wrapper替代Rust原始类型，JsonSchema只生成type不生成format
// ============================================================

/// JSON Schema兼容的u64 — 只生成 `"type": "integer"` 不带 `"format": "uint64"`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SchemaU64(pub u64);

impl JsonSchema for SchemaU64 {
    fn schema_name() -> std::borrow::Cow<'static, str> { "uint64".into() }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "integer" })
    }
}

impl From<u64> for SchemaU64 { fn from(v: u64) -> Self { Self(v) } }
impl From<SchemaU64> for u64 { fn from(s: SchemaU64) -> Self { s.0 } }

/// JSON Schema兼容的usize — 只生成 `"type": "integer"` 不带 `"format": "uint"`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SchemaUsize(pub usize);

impl JsonSchema for SchemaUsize {
    fn schema_name() -> std::borrow::Cow<'static, str> { "uint".into() }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "integer" })
    }
}

impl From<usize> for SchemaUsize { fn from(v: usize) -> Self { Self(v) } }
impl From<SchemaUsize> for usize { fn from(s: SchemaUsize) -> Self { s.0 } }

/// JSON Schema兼容的u32 — 只生成 `"type": "integer"` 不带 `"format": "uint32"`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SchemaU32(pub u32);

impl JsonSchema for SchemaU32 {
    fn schema_name() -> std::borrow::Cow<'static, str> { "uint32".into() }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "integer" })
    }
}

impl From<u32> for SchemaU32 { fn from(v: u32) -> Self { Self(v) } }
impl From<SchemaU32> for u32 { fn from(s: SchemaU32) -> Self { s.0 } }

/// JSON Schema兼容的f32 — 只生成 `"type": "number"` 不带 `"format": "float"`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SchemaF32(pub f32);

impl JsonSchema for SchemaF32 {
    fn schema_name() -> std::borrow::Cow<'static, str> { "float".into() }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "number" })
    }
}

impl From<f32> for SchemaF32 { fn from(v: f32) -> Self { Self(v) } }
impl From<SchemaF32> for f32 { fn from(s: SchemaF32) -> Self { s.0 } }

// ============================================================
// 返回结构体 — 使用Schema类型替代原始类型
// ============================================================

/// 实体信息 — MCP tool的统一返回结构
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityInfo {
    pub id: SchemaU64,
    pub name: String,
    pub position: [SchemaF32; 3],
    pub scale: [SchemaF32; 3],
    pub rotation_y_degrees: SchemaF32,
    pub entity_type: String, // "Empty", "Cube", "Plane", "Light" 等
}

/// 场景树节点 — MCP tool的统一返回结构
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SceneTreeNode {
    pub entity_id: SchemaU64,
    pub name: String,
    pub children: Vec<SceneTreeNode>,
}

/// 资产分类信息
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetCategoryInfo {
    pub index: SchemaUsize,
    pub name: String,
    pub asset_count: SchemaUsize,
}

/// 资产详情
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetInfo {
    pub id: SchemaU64,
    pub name: String,
    pub asset_type: String,
    pub description: String,
}

/// 项目信息
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub is_loaded: bool,
    pub entity_count: SchemaUsize,
}

/// 创建实体结果
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateEntityResult {
    pub entity_id: SchemaU64,
    pub name: String,
    pub entity_type: String,
}

/// 创建UI widget结果
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateWidgetResult {
    pub widget_id: SchemaU64,
    pub widget_type: String,
}

/// 运行脚本结果
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunScriptResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

// ============================================================
// MCP规范要求的object根类型包装struct
// MCP规范(2025-03-26)要求tool outputSchema根类型必须是'object'
// 直接返回Vec<T>的schema根类型是'array'，会导致schema验证panic
// 用包装struct把Vec包在object字段中，让schema根类型符合规范
// ============================================================

/// 场景树结果 — 包装Vec<SceneTreeNode>为object根类型
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SceneTreeResult {
    pub nodes: Vec<SceneTreeNode>,
}

/// 实体列表结果 — 包装Vec<EntityInfo>为object根类型
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityListResult {
    pub entities: Vec<EntityInfo>,
}

/// 资产分类列表结果 — 包装Vec<AssetCategoryInfo>为object根类型
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetCategoryListResult {
    pub categories: Vec<AssetCategoryInfo>,
}

/// 资产列表结果 — 包装Vec<AssetInfo>为object根类型
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetListResult {
    pub assets: Vec<AssetInfo>,
}

/// EngineBridge trait — 统一引擎能力抽象层
///
/// 方法标注说明:
/// - [Both] — 纯Rust和FFI都能实现，但结果可能略有差异
/// - [RustOnly] — 只有纯Rust能实现（不依赖Mono/C#）
/// - [FfiOnly] — 只有C# FFI能实现（需要Mono运行时+C#封装逻辑）
pub trait EngineBridge: Send + Sync {
    // ============================================================
    // 读取类操作 — [Both] 纯Rust优先(快、类型安全)
    // ============================================================

    /// [Both] 获取完整场景树结构
    fn get_scene_tree(&self) -> Result<Vec<SceneTreeNode>>;

    /// [Both] 列出所有实体及其属性
    fn list_entities(&self) -> Result<Vec<EntityInfo>>;

    /// [Both] 获取单个实体的详细信息
    fn get_entity_info(&self, entity_id: u64) -> Result<EntityInfo>;

    /// [Both] 获取项目信息
    fn get_project_info(&self) -> Result<ProjectInfo>;

    /// [Both] 获取资产分类列表
    fn list_asset_categories(&self) -> Result<Vec<AssetCategoryInfo>>;

    /// [Both] 获取指定分类下的资产列表
    fn list_assets(&self, category_index: usize) -> Result<Vec<AssetInfo>>;

    // ============================================================
    // 简单写入操作 — [Both] 纯Rust和FFI都能做
    // ============================================================

    /// [Both] 设置实体位置
    fn set_entity_position(&self, entity_id: u64, position: [f32; 3]) -> Result<()>;

    /// [Both] 设置实体缩放
    fn set_entity_scale(&self, entity_id: u64, scale: [f32; 3]) -> Result<()>;

    /// [Both] 绕Y轴旋转实体(角度制，对LLM友好)
    fn rotate_entity(&self, entity_id: u64, angle_degrees: f32) -> Result<()>;

    /// [Both] 设置游戏状态 (Editing=0, Running=1, Paused=2)
    fn set_game_state(&self, state: u32) -> Result<()>;

    /// [Both] 保存项目
    fn project_save(&self) -> Result<()>;

    // ============================================================
    // 截图/视觉反馈 — [RustOnly] 需要渲染器直接操作
    // ============================================================

    /// [RustOnly] 截图保存到指定路径
    fn capture_screenshot(&self, path: &str) -> Result<String>;

    /// [RustOnly] 设置相机参数
    fn set_camera(&self, yaw: f32, pitch: f32, distance: f32, target: [f32; 3]) -> Result<()>;

    // ============================================================
    // 复杂写入操作 — [FfiOnly] 需要C#层封装逻辑
    // ============================================================

    /// [FfiOnly] 创建实体(含资产模板、命名、组件绑定)
    /// FFI路径复用EditorScript的完整Entity创建逻辑
    fn create_entity(
        &self,
        entity_type: &str,
        name: Option<&str>,
        position: Option<[f32; 3]>,
    ) -> Result<CreateEntityResult>;

    /// [FfiOnly] 删除实体
    /// FFI路径走C#的scene_remove_entity，带编辑器状态同步
    fn remove_entity(&self, entity_id: u64) -> Result<()>;

    /// [FfiOnly] 创建UI widget
    /// FFI路径复用UI.cs的完整widget封装(Button/Label/Panel/VStack等)
    fn create_widget(
        &self,
        widget_type: &str,
        parent_id: u64,
        params: serde_json::Value,
    ) -> Result<CreateWidgetResult>;

    /// [FfiOnly] 执行C#脚本
    /// FFI路径通过Mono JIT执行C#代码，复用scripting crate
    fn run_script(&self, script_content: &str, entity_id: Option<u64>) -> Result<RunScriptResult>;

    /// [FfiOnly] 从资产模板创建Entity
    /// FFI路径走AssetLibrary.create_entity_from_template + C#封装
    fn create_entity_from_template(&self, template_id: u64) -> Result<CreateEntityResult>;

    // ============================================================
    // 能力查询 — tool动态注册用
    // ============================================================

    /// 返回当前bridge可用的能力列表
    /// 纯Rust: 返回[RustOnly]+[Both]方法
    /// FFI: 返回[Both]+[FfiOnly]方法(包含更多写入能力)
    fn available_capabilities(&self) -> Vec<String>;
}