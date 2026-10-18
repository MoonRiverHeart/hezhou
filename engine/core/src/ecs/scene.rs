use crate::ecs::*;
use crate::math::{Vec3, Mat4};
use std::collections::HashMap;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameState {
    Editing = 0,
    Running = 1,
    Paused = 2,
}

impl Default for GameState {
    fn default() -> Self {
        GameState::Editing
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ScriptBinding {
    pub script_path: String,
    pub class_name: String,
    pub enabled: bool,
    pub instance_id: u64,
}

impl Default for ScriptBinding {
    fn default() -> Self {
        Self {
            script_path: String::new(),
            class_name: String::new(),
            enabled: true,
            instance_id: 0,
        }
    }
}

pub struct Scene {
    pub world: World,
    pub state: GameState,
    pub selected_entities: Vec<Entity>,
    pub script_assets: HashMap<u64, ScriptAsset>,
    pub entity_scripts: HashMap<EntityId, Vec<ScriptComponent>>,
    pub entity_bindings: HashMap<EntityId, Vec<ScriptBinding>>,
    pub root_entities: Vec<Entity>,
    pub entity_names: HashMap<EntityId, String>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            state: GameState::Editing,
            selected_entities: Vec::new(),
            script_assets: HashMap::new(),
            entity_scripts: HashMap::new(),
            entity_bindings: HashMap::new(),
            root_entities: Vec::new(),
            entity_names: HashMap::new(),
        }
    }
    
    pub fn create_entity(&mut self) -> Entity {
        let entity = self.world.create_entity();
        self.world.add_component(entity, LocalTransform::new());
        self.world.add_component(entity, SelectionComponent::default());
        self.root_entities.push(entity);
        let default_name = format!("Entity_{}", entity.id);
        self.entity_names.insert(entity.id, default_name);
        entity
    }

    /// 以指定 ID 创建实体；ID 已存在时返回 None。
    pub fn create_entity_with_id(&mut self, id: EntityId) -> Option<Entity> {
        let entity = self.world.create_entity_with_id(id)?;
        self.world.add_component(entity, LocalTransform::new());
        self.world.add_component(entity, SelectionComponent::default());
        self.root_entities.push(entity);
        self.entity_names.insert(entity.id, format!("Entity_{}", entity.id));
        Some(entity)
    }
    
    pub fn create_render_entity(&mut self, mesh_path: String) -> Entity {
        let entity = self.create_entity();
        self.world.add_component(entity, RenderableComponent::new(mesh_path));
        self.world.add_component(entity, BoundsComponent::cube(2.0));
        entity
    }
    
    pub fn create_cube(&mut self) -> Entity {
        self.create_render_entity("builtin://cube".to_string())
    }
    
    pub fn create_plane(&mut self) -> Entity {
        self.create_render_entity("builtin://plane".to_string())
    }
    
    pub fn create_bunny(&mut self) -> Entity {
        self.create_render_entity("builtin://bunny".to_string())
    }
    
    pub fn create_cornell_box(&mut self) -> Entity {
        self.create_render_entity("builtin://cornell_box".to_string())
    }
    
    pub fn create_directional_light(&mut self) -> Entity {
        let entity = self.create_entity();
        self.world.add_component(entity, DirectionalLightComponent::default());
        entity
    }
    
    pub fn attach_script(&mut self, entity: Entity, script: ScriptComponent) {
        if !self.world.entity_exists(entity) {
            return;
        }
        
        self.world.add_component(entity, script.clone());
        
        if !self.entity_scripts.contains_key(&entity.id) {
            self.entity_scripts.insert(entity.id, Vec::new());
        }
        self.entity_scripts.get_mut(&entity.id).unwrap().push(script);
    }
    
    pub fn detach_script(&mut self, entity: Entity) {
        self.world.remove_component::<ScriptComponent>(entity);
        self.entity_scripts.remove(&entity.id);
        self.entity_bindings.remove(&entity.id);
    }
    
    pub fn attach_script_binding(&mut self, entity: Entity, script_path: String, class_name: String) {
        if !self.world.entity_exists(entity) {
            return;
        }
        
        let binding = ScriptBinding {
            script_path: script_path.clone(),
            class_name: class_name.clone(),
            enabled: true,
            instance_id: 0,
        };
        
        if !self.entity_bindings.contains_key(&entity.id) {
            self.entity_bindings.insert(entity.id, Vec::new());
        }
        self.entity_bindings.get_mut(&entity.id).unwrap().push(binding);
        
        let script = ScriptComponent::from_path(&script_path, &class_name);
        self.attach_script(entity, script);
    }
    
    pub fn remove_script_binding(&mut self, entity: Entity, index: usize) {
        if let Some(bindings) = self.entity_bindings.get_mut(&entity.id) {
            if index < bindings.len() {
                bindings.remove(index);
            }
        }
        
        if let Some(scripts) = self.entity_scripts.get_mut(&entity.id) {
            if index < scripts.len() {
                scripts.remove(index);
            }
        }
    }
    
    pub fn get_script_binding_count(&self, entity: Entity) -> usize {
        self.entity_bindings.get(&entity.id).map(|v| v.len()).unwrap_or(0)
    }
    
    pub fn get_script_binding(&self, entity: Entity, index: usize) -> Option<&ScriptBinding> {
        self.entity_bindings.get(&entity.id).and_then(|v| v.get(index))
    }
    
    pub fn set_script_binding_enabled(&mut self, entity: Entity, index: usize, enabled: bool) {
        if let Some(bindings) = self.entity_bindings.get_mut(&entity.id) {
            if let Some(binding) = bindings.get_mut(index) {
                binding.enabled = enabled;
            }
        }
        if let Some(scripts) = self.entity_scripts.get_mut(&entity.id) {
            if let Some(script) = scripts.get_mut(index) {
                script.enabled = enabled;
            }
        }
    }
    
    pub fn set_script_binding_instance_id(&mut self, entity: Entity, index: usize, instance_id: u64) {
        if let Some(bindings) = self.entity_bindings.get_mut(&entity.id) {
            if let Some(binding) = bindings.get_mut(index) {
                binding.instance_id = instance_id;
            }
        }
    }
    
    pub fn register_script_asset(&mut self, asset: ScriptAsset) {
        self.script_assets.insert(asset.id, asset);
    }
    
    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
        
        match state {
            GameState::Running => {
                self.load_scripts();
            }
            GameState::Paused => {
            }
            GameState::Editing => {
                self.unload_scripts();
            }
        }
    }
    
    pub fn load_scripts(&mut self) {
        for (_, asset) in &mut self.script_assets {
            asset.is_loaded = true;
        }
    }
    
    pub fn unload_scripts(&mut self) {
        for (_, asset) in &mut self.script_assets {
            asset.is_loaded = false;
        }
    }
    
    pub fn select_entity(&mut self, entity: Entity) {
        if !self.world.entity_exists(entity) {
            return;
        }
        
        self.clear_selection();
        
        if let Some(mut selection) = self.world.get_component::<SelectionComponent>(entity) {
            selection.selected = true;
            selection.level = SelectionLevel::Entity;
            self.world.add_component(entity, selection);
        }
        
        self.selected_entities.push(entity);
    }
    
    /// 追加选择：保留已有选中项，供 Ctrl+Click 多选使用。
    pub fn add_to_selection(&mut self, entity: Entity) {
        if !self.world.entity_exists(entity) || self.selected_entities.contains(&entity) {
            return;
        }
        if let Some(mut selection) = self.world.get_component::<SelectionComponent>(entity) {
            selection.selected = true;
            selection.level = SelectionLevel::Entity;
            self.world.add_component(entity, selection);
        }
        self.selected_entities.push(entity);
    }

    /// 从多选集合移除。
    pub fn remove_from_selection(&mut self, entity: Entity) {
        if !self.selected_entities.contains(&entity) {
            return;
        }
        if let Some(mut selection) = self.world.get_component::<SelectionComponent>(entity) {
            selection.clear();
            self.world.add_component(entity, selection);
        }
        self.selected_entities.retain(|e| *e != entity);
    }

    /// 切换单个实体的选中状态。
    pub fn toggle_selection(&mut self, entity: Entity) {
        if self.selected_entities.contains(&entity) {
            self.remove_from_selection(entity);
        } else {
            self.add_to_selection(entity);
        }
    }

    pub fn get_selected_entity_ids(&self) -> Vec<u64> {
        self.selected_entities.iter().map(|e| e.id).collect()
    }

    /// 热重载后旧 C# 实例已失效，把所有 binding 的 instance_id 清 0，
    /// 让 C# 侧 RecreateAllScriptInstances 重新创建。
    pub fn reset_script_instance_ids(&mut self) {
        for bindings in self.entity_bindings.values_mut() {
            for binding in bindings.iter_mut() {
                binding.instance_id = 0;
            }
        }
    }

    pub fn select_submesh(&mut self, entity: Entity, index: u32) {
        if !self.world.entity_exists(entity) {
            return;
        }
        
        self.clear_selection();
        
        if let Some(mut selection) = self.world.get_component::<SelectionComponent>(entity) {
            selection.selected = true;
            selection.level = SelectionLevel::SubMesh;
            selection.submesh_index = Some(index);
            self.world.add_component(entity, selection);
        }
        
        self.selected_entities.push(entity);
    }
    
    pub fn clear_selection(&mut self) {
        for entity in &self.selected_entities {
            if let Some(mut selection) = self.world.get_component::<SelectionComponent>(*entity) {
                selection.clear();
                self.world.add_component(*entity, selection);
            }
        }
        self.selected_entities.clear();
    }
    
    pub fn get_selected_entities(&self) -> &Vec<Entity> {
        &self.selected_entities
    }
    
    pub fn is_entity_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }
    
    pub fn pick_entity(&self, origin: Vec3, direction: Vec3) -> Option<Entity> {
        let mut closest_entity: Option<Entity> = None;
        let mut closest_distance: f32 = f32::MAX;
        
        for entity in &self.root_entities {
            if let Some(bounds) = self.world.get_component::<BoundsComponent>(*entity) {
                if let Some(transform) = self.world.get_component::<LocalTransform>(*entity) {
                    let world_bounds = Self::transform_bounds(bounds, &transform);
                    
                    if let Some(distance) = world_bounds.intersects_ray(origin, direction) {
                        if distance < closest_distance && distance > 0.0 {
                            closest_distance = distance;
                            closest_entity = Some(*entity);
                        }
                    }
                }
            }
        }
        
        closest_entity
    }
    
    fn transform_bounds(bounds: BoundsComponent, transform: &LocalTransform) -> BoundsComponent {
        let model = Mat4::translate(transform.position)
            * Mat4::from_quaternion(transform.rotation)
            * Mat4::scale(transform.scale);
        
        let corners = [
            Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
            Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
            Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
            Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
            Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
            Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
            Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
            Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
        ];
        
        let mut transformed_corners = Vec::new();
        for corner in corners {
            let tc = model.transform_point(corner);
            transformed_corners.push(tc);
        }
        
        let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        
        for tc in transformed_corners {
            min.x = min.x.min(tc.x);
            min.y = min.y.min(tc.y);
            min.z = min.z.min(tc.z);
            max.x = max.x.max(tc.x);
            max.y = max.y.max(tc.y);
            max.z = max.z.max(tc.z);
        }
        
        BoundsComponent::from_min_max(min, max)
    }
    
    pub fn update(&mut self, delta_time: f32) {
        if self.state == GameState::Running {
            self.world.update(delta_time);
        }
    }
    
    pub fn entity_count(&self) -> usize {
        self.world.entity_count()
    }

    /// 清空场景中的实体与场景级元数据（名称、脚本绑定、选中状态）。
    /// 用于加载项目前重置场景，避免旧实体残留导致重复。
    pub fn clear(&mut self) {
        self.world.clear();
        self.selected_entities.clear();
        self.entity_scripts.clear();
        self.entity_bindings.clear();
        self.entity_names.clear();
        self.root_entities.clear();
        self.state = GameState::Editing;
    }
    
    pub fn remove_entity(&mut self, entity: Entity) {
        if !self.world.entity_exists(entity) {
            return;
        }
        
        // 删除父实体时把子实体提升为根，避免子实体因 parent_map 指向已删除实体
        // 而无法出现在 root_entities / UI 层级树中。
        for child in self.get_children(entity) {
            self.world.set_parent(child, None);
            if !self.root_entities.contains(&child) {
                self.root_entities.push(child);
            }
        }

        self.root_entities.retain(|e| *e != entity);
        self.selected_entities.retain(|e| *e != entity);
        self.entity_scripts.remove(&entity.id);
        self.entity_bindings.remove(&entity.id);
        self.entity_names.remove(&entity.id);
        self.world.destroy_entity(entity);
    }
    
    pub fn set_parent(&mut self, entity: Entity, parent: Option<Entity>) {
        if !self.world.entity_exists(entity) {
            return;
        }
        if let Some(p) = parent {
            if !self.world.entity_exists(p) {
                return;
            }

            // 拒绝自环和祖先环：A→B→…→A 会让层级树无法遍历。
            let mut cursor = Some(p);
            while let Some(current) = cursor {
                if current == entity {
                    return;
                }
                cursor = self.get_parent(current);
            }

            // Remove from root_entities if becoming a child
            self.root_entities.retain(|e| *e != entity);
        } else {
            // Add back to root_entities if unparenting
            if !self.root_entities.contains(&entity) {
                self.root_entities.push(entity);
            }
        }
        self.world.set_parent(entity, parent);
    }
    
    pub fn get_parent(&self, entity: Entity) -> Option<Entity> {
        self.world.get_parent(entity)
    }
    
    pub fn get_children(&self, parent: Entity) -> Vec<Entity> {
        self.world.get_children(parent)
    }
    
    pub fn get_root_entities(&self) -> &Vec<Entity> {
        &self.root_entities
    }

    /// 所有实体（含子实体），用于工程保存/快照/UI 层级遍历。
    pub fn get_all_entities(&self) -> Vec<Entity> {
        self.world.all_entities()
    }
    
    pub fn get_entity_position(&self, entity: Entity) -> Option<Vec3> {
        self.world.get_component::<LocalTransform>(entity).map(|t| t.position)
    }
    
    pub fn get_entity_rotation(&self, entity: Entity) -> Option<crate::math::Quaternion> {
        self.world.get_component::<LocalTransform>(entity).map(|t| t.rotation)
    }
    
    pub fn get_entity_scale(&self, entity: Entity) -> Option<Vec3> {
        self.world.get_component::<LocalTransform>(entity).map(|t| t.scale)
    }
    
    pub fn set_entity_position(&mut self, entity: Entity, pos: Vec3) {
        if self.world.entity_exists(entity) {
            if let Some(mut transform) = self.world.get_component::<LocalTransform>(entity) {
                transform.position = pos;
                self.world.add_component(entity, transform);
            }
        }
    }
    
    pub fn set_entity_rotation(&mut self, entity: Entity, rotation: crate::math::Quaternion) {
        if self.world.entity_exists(entity) {
            if let Some(mut transform) = self.world.get_component::<LocalTransform>(entity) {
                transform.rotation = rotation;
                self.world.add_component(entity, transform);
            }
        }
    }
    
    pub fn set_entity_scale(&mut self, entity: Entity, scale: Vec3) {
        if self.world.entity_exists(entity) {
            if let Some(mut transform) = self.world.get_component::<LocalTransform>(entity) {
                transform.scale = scale;
                self.world.add_component(entity, transform);
            }
        }
    }
    
    pub fn set_entity_name(&mut self, entity: Entity, name: String) {
        if self.world.entity_exists(entity) {
            self.entity_names.insert(entity.id, name);
        }
    }
    
    pub fn get_entity_name(&self, entity: Entity) -> Option<&String> {
        self.entity_names.get(&entity.id)
    }
    
    // RenderableComponent helpers
    pub fn set_renderable_visible(&mut self, entity: Entity, visible: bool) -> bool {
        if !self.world.entity_exists(entity) { return false; }
        if let Some(mut renderable) = self.world.get_component::<RenderableComponent>(entity) {
            renderable.visible = visible;
            // Re-add the modified component
            self.world.add_component(entity, renderable);
            true
        } else {
            false
        }
    }
    
    pub fn set_renderable_material(&mut self, entity: Entity, material_path: Option<String>) -> bool {
        if !self.world.entity_exists(entity) { return false; }
        if let Some(mut renderable) = self.world.get_component::<RenderableComponent>(entity) {
            renderable.material_path = material_path;
            self.world.add_component(entity, renderable);
            true
        } else {
            false
        }
    }

    pub fn get_renderable_material(&self, entity: Entity) -> Option<String> {
        self.world.get_component::<RenderableComponent>(entity)
            .and_then(|r| r.material_path)
    }

    // DirectionalLightComponent helpers
    pub fn set_light_direction(&mut self, entity: Entity, direction: Vec3) -> bool {
        if !self.world.entity_exists(entity) { return false; }
        if let Some(mut light) = self.world.get_component::<DirectionalLightComponent>(entity) {
            light.direction = direction;
            self.world.add_component(entity, light);
            true
        } else {
            false
        }
    }
    
    pub fn set_light_color(&mut self, entity: Entity, color: Vec3) -> bool {
        if !self.world.entity_exists(entity) { return false; }
        if let Some(mut light) = self.world.get_component::<DirectionalLightComponent>(entity) {
            light.color = color;
            self.world.add_component(entity, light);
            true
        } else {
            false
        }
    }
    
    pub fn set_light_intensity(&mut self, entity: Entity, intensity: f32) -> bool {
        if !self.world.entity_exists(entity) { return false; }
        if let Some(mut light) = self.world.get_component::<DirectionalLightComponent>(entity) {
            light.intensity = intensity;
            self.world.add_component(entity, light);
            true
        } else {
            false
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}