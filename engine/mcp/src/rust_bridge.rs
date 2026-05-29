//! RustBridge — 纯Rust实现EngineBridge trait
//!
//! 可脱离编辑器独立运行，直接调用core crate的Rust API。
//! 能力边界: 读取类 + 简单写入 + 截图/相机(需要渲染器)
//! 无法: create_entity(需C#资产模板逻辑)、run_script(需Mono JIT)、create_widget(需UI.cs封装)
//!
//! ## 线程安全设计
//! Scene/AssetLibrary/Project内部含Box<dyn Any>/Box<dyn System>，不是Send+Sync。
//! RustBridge通过raw pointer持有引用，unsafe impl Send+Sync。
//! 实际Scene访问必须在引擎主线程上(与FFI层使用相同模式: *mut Scene)。
//! TODO: 未来改为channel-based异步请求模式，彻底解决线程安全问题。

use crate::bridge::*;
use anyhow::{Result, bail};
use hezhou_core::ecs::{Entity, Scene, GameState};
use hezhou_core::math::Vec3;
use hezhou_core::{AssetLibrary, Project};

/// Vec3 → [SchemaF32; 3] 转换辅助
fn vec3_to_schema(v: Vec3) -> [SchemaF32; 3] {
    [SchemaF32(v.x), SchemaF32(v.y), SchemaF32(v.z)]
}

/// RustBridge — 持有core crate组件的raw pointer引用
///
/// 独立运行时通过new_empty()创建(只有stub能力)，
/// 编辑器内嵌时从引擎运行时获取raw pointer。
pub struct RustBridge {
    scene: *mut Scene,
    asset_library: *mut AssetLibrary,
    project: *mut Project,
    renderer_available: bool,
}

// Scene/World含Box<dyn Any>和Box<dyn System>，不是Send+Sync。
// 但RustBridge通过raw pointer持有引用，与现有FFI层(*mut Scene)模式一致。
// 实际访问必须在引擎主线程上进行。
// TODO: 改为channel-based模式后可移除unsafe impl
unsafe impl Send for RustBridge {}
unsafe impl Sync for RustBridge {}

impl RustBridge {
    /// 创建空RustBridge (独立MCP server模式，只有stub能力)
    pub fn new_empty() -> Self {
        Self {
            scene: std::ptr::null_mut(),
            asset_library: std::ptr::null_mut(),
            project: std::ptr::null_mut(),
            renderer_available: false,
        }
    }

    /// 从引擎运行时创建RustBridge (编辑器内嵌模式)
    ///
    /// # Safety
    /// scene/asset_library/project指针必须有效，且在RustBridge生命周期内不被释放。
    /// 调用RustBridge方法时，必须在持有这些对象的线程上执行。
    pub unsafe fn from_runtime(
        scene: *mut Scene,
        asset_library: *mut AssetLibrary,
        project: *mut Project,
        renderer_available: bool,
    ) -> Self {
        Self {
            scene,
            asset_library,
            project,
            renderer_available,
        }
    }

    fn scene_ref(&self) -> Result<&Scene> {
        if self.scene.is_null() {
            bail!("Scene not available");
        }
        Ok(unsafe { &*self.scene })
    }

    fn scene_mut(&self) -> Result<&mut Scene> {
        if self.scene.is_null() {
            bail!("Scene not available");
        }
        Ok(unsafe { &mut *self.scene })
    }

    fn al_ref(&self) -> Result<&AssetLibrary> {
        if self.asset_library.is_null() {
            bail!("AssetLibrary not available");
        }
        Ok(unsafe { &*self.asset_library })
    }

    fn project_ref(&self) -> Result<&Project> {
        if self.project.is_null() {
            bail!("Project not available");
        }
        Ok(unsafe { &*self.project })
    }

    fn project_mut(&self) -> Result<&mut Project> {
        if self.project.is_null() {
            bail!("Project not available");
        }
        Ok(unsafe { &mut *self.project })
    }
}

impl EngineBridge for RustBridge {
    // ============================================================
    // 读取类 — [Both] 纯Rust实现
    // ============================================================

    fn get_scene_tree(&self) -> Result<Vec<SceneTreeNode>> {
        let scene = self.scene_ref()?;
        
        let mut tree = Vec::new();
        for entity in &scene.root_entities {
            let name = scene.entity_names.get(&entity.id)
                .cloned()
                .unwrap_or_else(|| format!("Entity_{}", entity.id));
            
            // 获取子节点
            let children = scene.world.get_children(*entity)
                .into_iter()
                .map(|child| {
                    let child_name = scene.entity_names.get(&child.id)
                        .cloned()
                        .unwrap_or_else(|| format!("Entity_{}", child.id));
                    SceneTreeNode {
                        entity_id: SchemaU64(child.id),
                        name: child_name,
                        children: Vec::new(),
                    }
                })
                .collect();
            
            tree.push(SceneTreeNode {
                entity_id: SchemaU64(entity.id),
                name,
                children,
            });
        }
        Ok(tree)
    }

    fn list_entities(&self) -> Result<Vec<EntityInfo>> {
        let scene = self.scene_ref()?;
        
        let mut result = Vec::new();
        for entity in &scene.root_entities {
            let info = self.get_entity_info(entity.id)?;
            result.push(info);
        }
        Ok(result)
    }

    fn get_entity_info(&self, entity_id: u64) -> Result<EntityInfo> {
        let scene = self.scene_ref()?;
        let entity = Entity::new(entity_id);
        
        if !scene.world.entity_exists(entity) {
            bail!("Entity {} not found", entity_id);
        }
        
        let name = scene.entity_names.get(&entity_id)
            .cloned()
            .unwrap_or_else(|| format!("Entity_{}", entity_id));
        let position = scene.get_entity_position(entity)
            .map(vec3_to_schema)
            .unwrap_or([SchemaF32(0.0), SchemaF32(0.0), SchemaF32(0.0)]);
        let scale = scene.get_entity_scale(entity)
            .map(vec3_to_schema)
            .unwrap_or([SchemaF32(1.0), SchemaF32(1.0), SchemaF32(1.0)]);
        
        // rotation: 从Quaternion提取Y轴旋转角度(度数)
        let rotation_y = scene.get_entity_rotation(entity)
            .map(|q| q.to_euler_degrees()[1]) // [x, y, z] euler angles in degrees
            .unwrap_or(0.0);
        
        // 判断entity_type: 从mesh component或名称推断
        let entity_type = if scene.world.has_component::<hezhou_core::ecs::RenderableComponent>(entity) {
            let mesh_path = scene.world.get_component::<hezhou_core::ecs::RenderableComponent>(entity)
                .map(|r| r.mesh_path.clone())
                .unwrap_or_default();
            if mesh_path.contains("cube") { "Cube" }
            else if mesh_path.contains("plane") { "Plane" }
            else if mesh_path.contains("sphere") { "Sphere" }
            else { "Mesh" }
        } else if scene.world.has_component::<hezhou_core::ecs::DirectionalLightComponent>(entity) {
            "Light"
        } else {
            "Empty"
        };
        
        Ok(EntityInfo {
            id: SchemaU64(entity_id),
            name,
            position,
            scale,
            rotation_y_degrees: SchemaF32(rotation_y),
            entity_type: entity_type.to_string(),
        })
    }

    fn get_project_info(&self) -> Result<ProjectInfo> {
        let project = self.project_ref()?;
        Ok(ProjectInfo {
            name: project.get_name().to_string(),
            path: project.get_path().to_string_lossy().to_string(),
            is_loaded: project.is_loaded(),
            entity_count: SchemaUsize(project.get_entity_count()),
        })
    }

    fn list_asset_categories(&self) -> Result<Vec<AssetCategoryInfo>> {
        let al = self.al_ref()?;
        let count = al.get_category_count();
        let mut result = Vec::new();
        for i in 0..count {
            let name = al.get_category_name(i)
                .cloned()
                .unwrap_or_else(|| format!("Category_{}", i));
            let asset_count = al.get_asset_count_in_category(i);
            result.push(AssetCategoryInfo {
                index: SchemaUsize(i),
                name,
                asset_count: SchemaUsize(asset_count),
            });
        }
        Ok(result)
    }

    fn list_assets(&self, category_index: usize) -> Result<Vec<AssetInfo>> {
        let al = self.al_ref()?;
        let count = al.get_asset_count_in_category(category_index);
        let mut result = Vec::new();
        for j in 0..count {
            let asset = al.get_asset_by_category_index(category_index, j)
                .ok_or_else(|| anyhow::anyhow!("Asset not found at category {} index {}", category_index, j))?;
            result.push(AssetInfo {
                id: SchemaU64(asset.id),
                name: asset.name.clone(),
                asset_type: format!("{:?}", asset.asset_type),
                description: asset.description.clone().unwrap_or_default(),
            });
        }
        Ok(result)
    }

    // ============================================================
    // 简单写入 — [Both] 纯Rust实现
    // ============================================================

    fn set_entity_position(&self, entity_id: u64, position: [f32; 3]) -> Result<()> {
        let scene = self.scene_mut()?;
        let entity = Entity::new(entity_id);
        scene.set_entity_position(entity, Vec3::new(position[0], position[1], position[2]));
        Ok(())
    }

    fn set_entity_scale(&self, entity_id: u64, scale: [f32; 3]) -> Result<()> {
        let scene = self.scene_mut()?;
        let entity = Entity::new(entity_id);
        scene.set_entity_scale(entity, Vec3::new(scale[0], scale[1], scale[2]));
        Ok(())
    }

    fn rotate_entity(&self, entity_id: u64, angle_degrees: f32) -> Result<()> {
        let scene = self.scene_mut()?;
        let entity = Entity::new(entity_id);
        // 修改rotation quaternion: 在当前Y轴旋转基础上叠加angle_degrees
        if let Some(mut transform) = scene.world.get_component::<hezhou_core::ecs::LocalTransform>(entity) {
            let euler = transform.rotation.to_euler_degrees(); // [x, y, z] degrees
            transform.rotation = hezhou_core::math::Quaternion::from_euler_degrees(
                euler[0],
                euler[1] + angle_degrees,
                euler[2],
            );
            scene.world.add_component(entity, transform);
        }
        Ok(())
    }

    fn set_game_state(&self, state: u32) -> Result<()> {
        let scene = self.scene_mut()?;
        let game_state = match state {
            0 => GameState::Editing,
            1 => GameState::Running,
            2 => GameState::Paused,
            _ => bail!("Invalid game state: {}", state),
        };
        scene.set_state(game_state);
        Ok(())
    }

    fn project_save(&self) -> Result<()> {
        let project = self.project_mut()?;
        project.save().map_err(|e| anyhow::anyhow!("Project save error: {}", e))?;
        Ok(())
    }

    // ============================================================
    // 截图/视觉反馈 — [RustOnly] 需渲染器(暂未实现)
    // ============================================================

    fn capture_screenshot(&self, _path: &str) -> Result<String> {
        if !self.renderer_available {
            bail!("Screenshot requires renderer (not available in standalone mode)");
        }
        bail!("Screenshot not yet implemented in RustBridge — needs renderer integration via FFI")
    }

    fn set_camera(&self, _yaw: f32, _pitch: f32, _distance: f32, _target: [f32; 3]) -> Result<()> {
        if !self.renderer_available {
            bail!("Camera control requires renderer (not available in standalone mode)");
        }
        bail!("Camera control not yet implemented in RustBridge — needs renderer integration via FFI")
    }

    // ============================================================
    // 复杂写入 — [FfiOnly] RustBridge无法实现
    // ============================================================

    fn create_entity(&self, _entity_type: &str, _name: Option<&str>, _position: Option<[f32; 3]>) -> Result<CreateEntityResult> {
        bail!("create_entity requires FfiBridge (C# scripting layer with asset template logic)")
    }

    fn remove_entity(&self, _entity_id: u64) -> Result<()> {
        bail!("remove_entity requires FfiBridge (C# scripting layer with editor state sync)")
    }

    fn create_widget(&self, _widget_type: &str, _parent_id: u64, _params: serde_json::Value) -> Result<CreateWidgetResult> {
        bail!("create_widget requires FfiBridge (C# UI.cs widget encapsulation)")
    }

    fn run_script(&self, _script_content: &str, _entity_id: Option<u64>) -> Result<RunScriptResult> {
        bail!("run_script requires FfiBridge (Mono JIT runtime)")
    }

    fn create_entity_from_template(&self, _template_id: u64) -> Result<CreateEntityResult> {
        bail!("create_entity_from_template requires FfiBridge (C# AssetLibrary layer)")
    }

    // ============================================================
    // 能力查询
    // ============================================================

    fn available_capabilities(&self) -> Vec<String> {
        let mut caps: Vec<String> = vec![
            "get_scene_tree", "list_entities", "get_entity_info",
            "get_project_info", "list_asset_categories", "list_assets",
            "set_entity_position", "set_entity_scale", "rotate_entity",
            "set_game_state", "project_save",
        ].into_iter().map(String::from).collect();
        if self.renderer_available {
            caps.push("capture_screenshot".to_string());
            caps.push("set_camera".to_string());
        }
        caps
    }
}