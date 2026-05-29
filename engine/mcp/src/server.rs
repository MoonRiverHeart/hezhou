//! Hezhou MCP Server — rmcp声明式tool定义
//!
//! 所有tool通过EngineBridge trait访问引擎能力。
//! 根据bridge实现(RustBridge/FfiBridge)，tool的实际能力不同。
//!
//! rmcp v1.7.0宏用法:
//! - #[tool_router(server_handler)] 在 inherent impl block 上 → 自动生成ServerHandler impl
//! - #[tool] 标记每个tool handler方法
//! - Parameters<T> 解构输入参数
//! - Result<Json<T>, rmcp::ErrorData> 返回结构化输出

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    schemars, tool, tool_router,
    ErrorData,
};
use serde::Deserialize;
use schemars::JsonSchema;
use std::sync::Arc;

use crate::bridge::{EngineBridge, SchemaU64, SchemaU32, SchemaF32, SchemaUsize};

// ============================================================
// Tool参数定义 — JSON Schema自动生成
// 使用Schema* wrapper类型替代Rust原始类型，避免非标准format警告
// ============================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSceneTreeRequest {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListEntitiesRequest {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEntityInfoRequest {
    pub entity_id: SchemaU64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetEntityPositionRequest {
    pub entity_id: SchemaU64,
    pub position: [SchemaF32; 3],
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetEntityScaleRequest {
    pub entity_id: SchemaU64,
    pub scale: [SchemaF32; 3],
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RotateEntityRequest {
    pub entity_id: SchemaU64,
    pub angle_degrees: SchemaF32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetGameStateRequest {
    /// 0=Editing, 1=Running, 2=Paused
    pub state: SchemaU32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectSaveRequest {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CaptureScreenshotRequest {
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetCameraRequest {
    pub yaw: SchemaF32,
    pub pitch: SchemaF32,
    pub distance: SchemaF32,
    pub target: [SchemaF32; 3],
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateEntityRequest {
    /// "Cube", "Plane", "Light", "Empty"
    pub entity_type: String,
    pub name: Option<String>,
    pub position: Option<[SchemaF32; 3]>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveEntityRequest {
    pub entity_id: SchemaU64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWidgetRequest {
    /// "Button", "Label", "Panel", "VStack", "HStack", "InputField" 等
    pub widget_type: String,
    pub parent_id: SchemaU64,
    /// Widget创建参数(JSON格式) — 各widget类型的参数不同
    /// schemars对serde_json::Value的schema描述为"任意JSON值"
    #[schemars(schema_with = "json_value_schema")]
    pub params: serde_json::Value,
}

/// 为serde_json::Value生成简单schema描述，避免schemars默认的递归schema
fn json_value_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::Schema::from(true) // true = "any type" in JSON Schema draft 2020-12
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunScriptRequest {
    pub script_content: String,
    pub entity_id: Option<SchemaU64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateEntityFromTemplateRequest {
    pub template_id: SchemaU64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetProjectInfoRequest {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListAssetCategoriesRequest {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListAssetsRequest {
    pub category_index: SchemaUsize,
}

// ============================================================
// MCP Server定义 — #[tool_router(server_handler)] 自动生成ServerHandler
// ============================================================

/// Hezhou MCP Server — 持有EngineBridge引用
#[derive(Clone)]
pub struct HezhouMcpServer {
    bridge: Arc<dyn EngineBridge>,
    capabilities: Vec<String>,
}

impl HezhouMcpServer {
    pub fn new(bridge: Arc<dyn EngineBridge>) -> Self {
        let capabilities = bridge.available_capabilities();
        Self { bridge, capabilities }
    }

    fn tool_available(&self, tool_name: &str) -> bool {
        self.capabilities.contains(&tool_name.to_string())
    }
}

/// #[tool_router(server_handler)] — inherent impl block上
/// server_handler flag 自动生成 ServerHandler impl
#[tool_router(server_handler)]
impl HezhouMcpServer {
    // ============================================================
    // 读取类tool — [Both]
    // ============================================================

    #[tool(description = "获取当前场景的完整树结构，包含所有实体及其层级关系")]
    async fn get_scene_tree(
        &self,
        _params: Parameters<GetSceneTreeRequest>,
    ) -> Result<Json<crate::bridge::SceneTreeResult>, ErrorData> {
        let nodes = self.bridge.get_scene_tree()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(crate::bridge::SceneTreeResult { nodes }))
    }

    #[tool(description = "列出场景中所有实体及其基本信息(名称、位置、缩放、类型)")]
    async fn list_entities(
        &self,
        _params: Parameters<ListEntitiesRequest>,
    ) -> Result<Json<crate::bridge::EntityListResult>, ErrorData> {
        let entities = self.bridge.list_entities()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(crate::bridge::EntityListResult { entities }))
    }

    #[tool(description = "获取单个实体的详细信息")]
    async fn get_entity_info(
        &self,
        Parameters(GetEntityInfoRequest { entity_id }): Parameters<GetEntityInfoRequest>,
    ) -> Result<Json<crate::bridge::EntityInfo>, ErrorData> {
        let info = self.bridge.get_entity_info(entity_id.0)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(info))
    }

    #[tool(description = "获取项目信息(名称、路径、Entity数量)")]
    async fn get_project_info(
        &self,
        _params: Parameters<GetProjectInfoRequest>,
    ) -> Result<Json<crate::bridge::ProjectInfo>, ErrorData> {
        let info = self.bridge.get_project_info()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(info))
    }

    #[tool(description = "获取资产库的所有分类")]
    async fn list_asset_categories(
        &self,
        _params: Parameters<ListAssetCategoriesRequest>,
    ) -> Result<Json<crate::bridge::AssetCategoryListResult>, ErrorData> {
        let categories = self.bridge.list_asset_categories()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(crate::bridge::AssetCategoryListResult { categories }))
    }

    #[tool(description = "获取指定资产分类下的所有资产")]
    async fn list_assets(
        &self,
        Parameters(ListAssetsRequest { category_index }): Parameters<ListAssetsRequest>,
    ) -> Result<Json<crate::bridge::AssetListResult>, ErrorData> {
        let assets = self.bridge.list_assets(category_index.0)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(crate::bridge::AssetListResult { assets }))
    }

    // ============================================================
    // 简单写入 — [Both]
    // ============================================================

    #[tool(description = "设置实体的3D位置坐标")]
    async fn set_entity_position(
        &self,
        Parameters(SetEntityPositionRequest { entity_id, position }): Parameters<SetEntityPositionRequest>,
    ) -> Result<String, ErrorData> {
        let pos = [position[0].0, position[1].0, position[2].0];
        self.bridge.set_entity_position(entity_id.0, pos)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(format!("Entity {} position set to {:?}", entity_id.0, pos))
    }

    #[tool(description = "设置实体的3D缩放")]
    async fn set_entity_scale(
        &self,
        Parameters(SetEntityScaleRequest { entity_id, scale }): Parameters<SetEntityScaleRequest>,
    ) -> Result<String, ErrorData> {
        let s = [scale[0].0, scale[1].0, scale[2].0];
        self.bridge.set_entity_scale(entity_id.0, s)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(format!("Entity {} scale set to {:?}", entity_id.0, s))
    }

    #[tool(description = "绕Y轴旋转实体(角度制)")]
    async fn rotate_entity(
        &self,
        Parameters(RotateEntityRequest { entity_id, angle_degrees }): Parameters<RotateEntityRequest>,
    ) -> Result<String, ErrorData> {
        self.bridge.rotate_entity(entity_id.0, angle_degrees.0)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(format!("Entity {} rotated by {} degrees", entity_id.0, angle_degrees.0))
    }

    #[tool(description = "设置游戏运行状态: 0=Editing, 1=Running, 2=Paused")]
    async fn set_game_state(
        &self,
        Parameters(SetGameStateRequest { state }): Parameters<SetGameStateRequest>,
    ) -> Result<String, ErrorData> {
        self.bridge.set_game_state(state.0)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let name = match state.0 { 0 => "Editing", 1 => "Running", 2 => "Paused", _ => "Unknown" };
        Ok(format!("Game state set to {}", name))
    }

    #[tool(description = "保存当前项目到文件")]
    async fn project_save(
        &self,
        _params: Parameters<ProjectSaveRequest>,
    ) -> Result<String, ErrorData> {
        self.bridge.project_save()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok("Project saved successfully".to_string())
    }

    // ============================================================
    // 截图/视觉反馈 — [RustOnly]
    // ============================================================

    #[tool(description = "截取当前编辑器画面并保存到指定路径")]
    async fn capture_screenshot(
        &self,
        Parameters(CaptureScreenshotRequest { path }): Parameters<CaptureScreenshotRequest>,
    ) -> Result<String, ErrorData> {
        if !self.tool_available("capture_screenshot") {
            return Err(ErrorData::internal_error(
                "capture_screenshot requires renderer access".to_string(), None
            ));
        }
        let result = self.bridge.capture_screenshot(&path)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(format!("Screenshot saved to {}", result))
    }

    #[tool(description = "设置编辑器相机参数(视角、距离、目标点)")]
    async fn set_camera(
        &self,
        Parameters(SetCameraRequest { yaw, pitch, distance, target }): Parameters<SetCameraRequest>,
    ) -> Result<String, ErrorData> {
        if !self.tool_available("set_camera") {
            return Err(ErrorData::internal_error(
                "set_camera requires renderer access".to_string(), None
            ));
        }
        let t = [target[0].0, target[1].0, target[2].0];
        self.bridge.set_camera(yaw.0, pitch.0, distance.0, t)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(format!("Camera set: yaw={}, pitch={}, dist={}, target={:?}", yaw.0, pitch.0, distance.0, t))
    }

    // ============================================================
    // 复杂写入 — [FfiOnly]
    // ============================================================

    #[tool(description = "在场景中创建新实体(Cube/Plane/Light/Empty)")]
    async fn create_entity(
        &self,
        Parameters(CreateEntityRequest { entity_type, name, position }): Parameters<CreateEntityRequest>,
    ) -> Result<Json<crate::bridge::CreateEntityResult>, ErrorData> {
        if !self.tool_available("create_entity") {
            return Err(ErrorData::internal_error(
                "create_entity requires FfiBridge".to_string(), None
            ));
        }
        let pos = position.map(|p| [p[0].0, p[1].0, p[2].0]);
        let result = self.bridge.create_entity(&entity_type, name.as_deref(), pos)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(result))
    }

    #[tool(description = "删除场景中的实体")]
    async fn remove_entity(
        &self,
        Parameters(RemoveEntityRequest { entity_id }): Parameters<RemoveEntityRequest>,
    ) -> Result<String, ErrorData> {
        if !self.tool_available("remove_entity") {
            return Err(ErrorData::internal_error(
                "remove_entity requires FfiBridge".to_string(), None
            ));
        }
        self.bridge.remove_entity(entity_id.0)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(format!("Entity {} removed", entity_id.0))
    }

    #[tool(description = "创建UI widget(Button/Label/Panel/VStack/HStack/InputField等)")]
    async fn create_widget(
        &self,
        Parameters(CreateWidgetRequest { widget_type, parent_id, params }): Parameters<CreateWidgetRequest>,
    ) -> Result<Json<crate::bridge::CreateWidgetResult>, ErrorData> {
        if !self.tool_available("create_widget") {
            return Err(ErrorData::internal_error(
                "create_widget requires FfiBridge".to_string(), None
            ));
        }
        let result = self.bridge.create_widget(&widget_type, parent_id.0, params)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(result))
    }

    #[tool(description = "执行C#脚本代码", annotations(destructive_hint = true))]
    async fn run_script(
        &self,
        Parameters(RunScriptRequest { script_content, entity_id }): Parameters<RunScriptRequest>,
    ) -> Result<Json<crate::bridge::RunScriptResult>, ErrorData> {
        if !self.tool_available("run_script") {
            return Err(ErrorData::internal_error(
                "run_script requires FfiBridge (Mono JIT runtime)".to_string(), None
            ));
        }
        let eid = entity_id.map(|e| e.0);
        let result = self.bridge.run_script(&script_content, eid)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(result))
    }

    #[tool(description = "从资产库模板创建Entity")]
    async fn create_entity_from_template(
        &self,
        Parameters(CreateEntityFromTemplateRequest { template_id }): Parameters<CreateEntityFromTemplateRequest>,
    ) -> Result<Json<crate::bridge::CreateEntityResult>, ErrorData> {
        if !self.tool_available("create_entity_from_template") {
            return Err(ErrorData::internal_error(
                "create_entity_from_template requires FfiBridge".to_string(), None
            ));
        }
        let result = self.bridge.create_entity_from_template(template_id.0)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust_bridge::RustBridge;

    #[test]
    fn test_tool_router_has_all_tools() {
        let bridge: Arc<dyn EngineBridge> = Arc::new(RustBridge::new_empty());
        let _server = HezhouMcpServer::new(bridge);
        let router = HezhouMcpServer::tool_router();
        let tools = router.list_all();
        
        for tool in &tools {
            eprintln!("Tool: {} — {:?}", tool.name, tool.description);
        }
        
        eprintln!("\nTotal tools registered: {}", tools.len());
        assert_eq!(tools.len(), 18, "应该注册18个tool");
    }
}