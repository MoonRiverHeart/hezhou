use std::path::PathBuf;
use std::collections::HashMap;
use std::fs;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::math::{Vec3, Quaternion};
use crate::ecs::{
    BoundsComponent, DirectionalLightComponent, Entity, GameState, LocalTransform,
    RenderableComponent, Scene,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectSettings {
    pub game_width: u32,
    pub game_height: u32,
    pub fps: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            game_width: 1920,
            game_height: 1080,
            fps: 60,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectEntityTransform {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for ProjectEntityTransform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectScriptBinding {
    pub script_path: String,
    pub class_name: String,
    pub enabled: bool,
}

impl Default for ProjectScriptBinding {
    fn default() -> Self {
        Self {
            script_path: String::new(),
            class_name: String::new(),
            enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

impl Default for ProjectLight {
    fn default() -> Self {
        let light = DirectionalLightComponent::default();
        Self {
            direction: [light.direction.x, light.direction.y, light.direction.z],
            color: [light.color.x, light.color.y, light.color.z],
            intensity: light.intensity,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectEntity {
    pub id: u64,
    pub name: String,
    pub transform: ProjectEntityTransform,
    /// "Cube"/"Sphere"...；方向光为 "DirectionalLight"；空实体为 "None"。
    pub mesh_type: String,
    pub scripts: Vec<ProjectScriptBinding>,
    pub components: HashMap<String, serde_json::Value>,
    #[serde(default = "default_visible")]
    pub visible: bool,
    /// 仅方向光实体使用；旧版工程文件缺少该字段时为 None。
    #[serde(default)]
    pub light: Option<ProjectLight>,
    /// 父实体的工程 ID；None 表示根实体。旧版工程默认 None。
    #[serde(default)]
    pub parent_id: Option<u64>,
}

fn default_visible() -> bool {
    true
}

impl Default for ProjectEntity {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            transform: ProjectEntityTransform::default(),
            mesh_type: "Cube".to_string(),
            scripts: Vec::new(),
            components: HashMap::new(),
            visible: true,
            light: None,
            parent_id: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum ProjectAssetType {
    Texture = 0,
    Material = 1,
    Mesh = 2,
    Script = 3,
}

impl Default for ProjectAssetType {
    fn default() -> Self {
        ProjectAssetType::Texture
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectAsset {
    pub id: u64,
    pub name: String,
    pub asset_type: ProjectAssetType,
    pub path: String,
}

impl Default for ProjectAsset {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            asset_type: ProjectAssetType::Texture,
            path: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditorState {
    pub selected_entity_id: Option<u64>,
    pub camera_position: [f32; 3],
    pub camera_rotation: [f32; 3],
    pub game_state: String,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_entity_id: None,
            camera_position: [10.0, 10.0, 10.0],
            camera_rotation: [-30.0, 45.0, 0.0],
            game_state: "Editing".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectFile {
    pub version: String,
    pub name: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub settings: ProjectSettings,
    pub entities: Vec<ProjectEntity>,
    pub assets: Vec<ProjectAsset>,
    pub scripts: Vec<String>,
    pub current_state: EditorState,
}

impl Default for ProjectFile {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            name: String::new(),
            path: String::new(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            settings: ProjectSettings::default(),
            entities: Vec::new(),
            assets: Vec::new(),
            scripts: Vec::new(),
            current_state: EditorState::default(),
        }
    }
}

pub struct Project {
    file: ProjectFile,
    /// 工程目录（用于展示、创建子目录）。
    project_path: PathBuf,
    /// 实际工程 JSON 文件路径。加载单个 .json 文件时可能不在工程目录下。
    project_file_path: PathBuf,
    is_loaded: bool,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            file: ProjectFile::default(),
            project_path: PathBuf::new(),
            project_file_path: PathBuf::new(),
            is_loaded: false,
        }
    }
}

impl Project {
    pub fn new(name: String, project_path: PathBuf) -> Self {
        let now = Utc::now();
        let project_file_path = project_path.join("project.json");
        let file = ProjectFile {
            version: "1.0".to_string(),
            name,
            path: project_path.to_string_lossy().to_string(),
            created_at: now,
            modified_at: now,
            settings: ProjectSettings::default(),
            entities: Vec::new(),
            assets: Vec::new(),
            scripts: Vec::new(),
            current_state: EditorState::default(),
        };
        
        Self {
            file,
            project_path,
            project_file_path,
            is_loaded: false,
        }
    }
    
    /// 在 parent_path 下创建以 name 命名的工程目录。
    /// 若调用方传入的是 .../*.json 文件路径，则直接以该文件作为工程文件，
    /// 工程目录取其父目录（兼容旧的“另存为”调用方式）。
    pub fn create_new(name: String, parent_path: PathBuf) -> Result<Self, ProjectError> {
        let is_json_file = parent_path
            .extension()
            .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("json"))
            .unwrap_or(false);

        let (project_path, project_file_path) = if is_json_file {
            let dir = parent_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf();
            (dir, parent_path)
        } else {
            let dir = parent_path.join(&name);
            let file = dir.join("project.json");
            (dir, file)
        };
        
        fs::create_dir_all(&project_path)?;
        fs::create_dir_all(project_path.join("assets"))?;
        fs::create_dir_all(project_path.join("assets/textures"))?;
        fs::create_dir_all(project_path.join("assets/materials"))?;
        fs::create_dir_all(project_path.join("assets/meshes"))?;
        fs::create_dir_all(project_path.join("assets/scripts"))?;
        fs::create_dir_all(project_path.join("scenes"))?;
        fs::create_dir_all(project_path.join("scripts"))?;
        fs::create_dir_all(project_path.join("settings"))?;
        
        let mut project = Project::new(name, project_path);
        project.project_file_path = project_file_path;
        project.is_loaded = true;
        project.save()?;
        
        Ok(project)
    }
    
    /// 加载工程。path 可以是工程目录（读取其中的 project.json），
    /// 也可以直接是 .json 文件路径。
    pub fn load(path: PathBuf) -> Result<Self, ProjectError> {
        let (project_path, project_file_path) = if path.is_dir() {
            let file = path.join("project.json");
            // 兼容旧版本使用 project.project.json 作为文件名的工程。
            let file = if file.exists() {
                file
            } else {
                let legacy = path.join("project.project.json");
                if legacy.exists() { legacy } else { file }
            };
            (path, file)
        } else {
            let dir = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf();
            (dir, path)
        };
        
        if !project_file_path.exists() {
            return Err(ProjectError::FileNotFound(project_file_path.to_string_lossy().to_string()));
        }
        
        let json = fs::read_to_string(&project_file_path)?;
        let file: ProjectFile = serde_json::from_str(&json)?;
        
        let mut project = Project::new(file.name.clone(), project_path);
        project.file = file;
        project.project_file_path = project_file_path;
        project.is_loaded = true;
        
        Ok(project)
    }
    
    pub fn save(&mut self) -> Result<(), ProjectError> {
        self.file.modified_at = Utc::now();
        
        if let Some(parent) = self.project_file_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(&self.file)?;
        fs::write(&self.project_file_path, json)?;
        
        Ok(())
    }
    
    pub fn get_name(&self) -> &str {
        &self.file.name
    }
    
    pub fn get_path(&self) -> &PathBuf {
        &self.project_path
    }
    
    pub fn get_entity_count(&self) -> usize {
        self.file.entities.len()
    }
    
    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }
    
    pub fn sync_to_scene(&self, scene: &mut Scene) {
        // 加载是“替换”而不是“追加”：先清空旧场景，避免重复实体。
        scene.clear();

        // 父子关系需要两阶段：先创建所有实体并建立“工程 ID → 新 Entity”映射，
        // 再设置 parent，避免父实体尚未创建或 ID 变化导致丢层级。
        let mut entity_map: Vec<(u64, Entity)> = Vec::new();

        for project_entity in &self.file.entities {
            // 保留工程中的实体 ID，撤销/重做和脚本绑定才能继续引用同一实体。
            let entity = scene
                .create_entity_with_id(project_entity.id)
                .unwrap_or_else(|| scene.create_entity());

            match project_entity.mesh_type.as_str() {
                "DirectionalLight" => {
                    scene
                        .world
                        .add_component(entity, DirectionalLightComponent::default());
                }
                "None" | "" => {}
                mesh_type => {
                    scene.world.add_component(
                        entity,
                        RenderableComponent::new(Self::mesh_type_to_path(mesh_type)),
                    );
                    scene.world.add_component(entity, BoundsComponent::cube(2.0));
                }
            }
            
            let mut transform = LocalTransform::new();
            transform.position = Vec3::new(
                project_entity.transform.position[0],
                project_entity.transform.position[1],
                project_entity.transform.position[2],
            );
            transform.rotation = Quaternion::from_euler_degrees(
                project_entity.transform.rotation[0],
                project_entity.transform.rotation[1],
                project_entity.transform.rotation[2],
            );
            transform.scale = Vec3::new(
                project_entity.transform.scale[0],
                project_entity.transform.scale[1],
                project_entity.transform.scale[2],
            );
            scene.world.add_component(entity, transform);
            
            scene.set_entity_name(entity, project_entity.name.clone());

            if let Some(light) = &project_entity.light {
                scene.set_light_direction(
                    entity,
                    Vec3::new(light.direction[0], light.direction[1], light.direction[2]),
                );
                scene.set_light_color(
                    entity,
                    Vec3::new(light.color[0], light.color[1], light.color[2]),
                );
                scene.set_light_intensity(entity, light.intensity);
            }

            if !project_entity.visible {
                scene.set_renderable_visible(entity, false);
            }
            
            for script_binding in &project_entity.scripts {
                scene.attach_script_binding(
                    entity,
                    script_binding.script_path.clone(),
                    script_binding.class_name.clone(),
                );
                scene.set_script_binding_enabled(entity, scene.get_script_binding_count(entity) - 1, script_binding.enabled);
            }

            entity_map.push((project_entity.id, entity));
        }

        // 第二阶段：按工程 ID 映射恢复父子层级。
        for project_entity in &self.file.entities {
            if let Some(parent_project_id) = project_entity.parent_id {
                if let Some((_, entity)) = entity_map
                    .iter()
                    .find(|(project_id, _)| *project_id == project_entity.id)
                {
                    if let Some((_, parent)) = entity_map
                        .iter()
                        .find(|(project_id, _)| *project_id == parent_project_id)
                    {
                        scene.set_parent(*entity, Some(*parent));
                    }
                }
            }
        }

        // 工程加载后回到编辑模式，避免直接以运行态启动脚本。
        scene.state = GameState::Editing;
    }
    
    /// 将当前场景序列化为完整快照（含层级、组件、脚本绑定），用于撤销/重做。
    pub fn snapshot_to_json(scene: &Scene) -> Result<String, ProjectError> {
        let mut snapshot = Project::new("__scene_snapshot__".to_string(), PathBuf::from("."));
        snapshot.sync_from_scene(scene);
        serde_json::to_string(&snapshot.file).map_err(ProjectError::from)
    }

    /// 从快照恢复场景。恢复采用“替换”语义，实体 ID 会重新分配；
    /// 调用方需要重建 UI 树和脚本实例。
    pub fn restore_from_json(json: &str, scene: &mut Scene) -> Result<(), ProjectError> {
        let file: ProjectFile = serde_json::from_str(json)?;
        let mut snapshot = Project::new(file.name.clone(), PathBuf::from("."));
        snapshot.file = file;
        snapshot.sync_to_scene(scene);
        Ok(())
    }

    pub fn sync_from_scene(&mut self, scene: &Scene) {
        self.file.entities.clear();
        
        for entity in scene.get_all_entities() {
            if let Some(name) = scene.get_entity_name(entity) {
                let transform = scene.world.get_component::<LocalTransform>(entity);
                let renderable = scene.world.get_component::<RenderableComponent>(entity);
                let light = scene.world.get_component::<DirectionalLightComponent>(entity);
                
                let project_entity = ProjectEntity {
                    id: entity.id,
                    name: name.clone(),
                    transform: if let Some(t) = transform {
                        ProjectEntityTransform {
                            position: [t.position.x, t.position.y, t.position.z],
                            rotation: t.rotation.to_euler_degrees(),
                            scale: [t.scale.x, t.scale.y, t.scale.z],
                        }
                    } else {
                        ProjectEntityTransform::default()
                    },
                    mesh_type: if let Some(ref r) = renderable {
                        Self::path_to_mesh_type(&r.mesh_path)
                    } else if light.is_some() {
                        "DirectionalLight".to_string()
                    } else {
                        "None".to_string()
                    },
                    scripts: Self::collect_scripts(scene, entity),
                    components: HashMap::new(),
                    visible: renderable.as_ref().map(|r| r.visible).unwrap_or(true),
                    light: light.map(|l| ProjectLight {
                        direction: [l.direction.x, l.direction.y, l.direction.z],
                        color: [l.color.x, l.color.y, l.color.z],
                        intensity: l.intensity,
                    }),
                    parent_id: scene.get_parent(entity).map(|p| p.id),
                };
                
                self.file.entities.push(project_entity);
            }
        }
        
        self.file.current_state.selected_entity_id = scene.selected_entities.first().map(|e| e.id);
        self.file.current_state.game_state = match scene.state {
            GameState::Editing => "Editing",
            GameState::Running => "Running",
            GameState::Paused => "Paused",
        }.to_string();
    }
    
    fn mesh_type_to_path(mesh_type: &str) -> String {
        match mesh_type {
            "Cube" => "builtin://cube",
            "Sphere" => "builtin://sphere",
            "Plane" => "builtin://plane",
            "Cylinder" => "builtin://cylinder",
            "Cone" => "builtin://cone",
            "Bunny" => "builtin://bunny",
            "CornellBox" => "builtin://cornell_box",
            _ => mesh_type,
        }.to_string()
    }
    
    fn path_to_mesh_type(path: &str) -> String {
        match path {
            "builtin://cube" => "Cube",
            "builtin://sphere" => "Sphere",
            "builtin://plane" => "Plane",
            "builtin://cylinder" => "Cylinder",
            "builtin://cone" => "Cone",
            "builtin://bunny" => "Bunny",
            "builtin://cornell_box" => "CornellBox",
            _ => path,
        }.to_string()
    }
    
    fn collect_scripts(scene: &Scene, entity: Entity) -> Vec<ProjectScriptBinding> {
        let bindings = scene.entity_bindings.get(&entity.id);
        if let Some(bindings) = bindings {
            bindings.iter().map(|b| ProjectScriptBinding {
                script_path: b.script_path.clone(),
                class_name: b.class_name.clone(),
                enabled: b.enabled,
            }).collect()
        } else {
            Vec::new()
        }
    }
    
    pub fn add_entity(&mut self, entity: ProjectEntity) {
        self.file.entities.push(entity);
    }
    
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.file.entities.retain(|e| e.id != entity_id);
    }
    
    pub fn get_entity(&self, entity_id: u64) -> Option<&ProjectEntity> {
        self.file.entities.iter().find(|e| e.id == entity_id)
    }
    
    pub fn update_entity(&mut self, entity_id: u64, update_fn: impl FnOnce(&mut ProjectEntity)) {
        if let Some(entity) = self.file.entities.iter_mut().find(|e| e.id == entity_id) {
            update_fn(entity);
        }
    }
    
    pub fn add_asset(&mut self, asset: ProjectAsset) {
        self.file.assets.push(asset);
    }
    
    pub fn add_script(&mut self, script_path: String) {
        if !self.file.scripts.contains(&script_path) {
            self.file.scripts.push(script_path);
        }
    }
    
    pub fn get_settings(&self) -> &ProjectSettings {
        &self.file.settings
    }
    
    pub fn set_settings(&mut self, settings: ProjectSettings) {
        self.file.settings = settings;
    }
}

#[derive(Debug)]
pub enum ProjectError {
    FileNotFound(String),
    IoError(std::io::Error),
    JsonError(serde_json::Error),
}

impl From<std::io::Error> for ProjectError {
    fn from(err: std::io::Error) -> Self {
        ProjectError::IoError(err)
    }
}

impl From<serde_json::Error> for ProjectError {
    fn from(err: serde_json::Error) -> Self {
        ProjectError::JsonError(err)
    }
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::FileNotFound(path) => write!(f, "Project file not found: {}", path),
            ProjectError::IoError(err) => write!(f, "IO error: {}", err),
            ProjectError::JsonError(err) => write!(f, "JSON error: {}", err),
        }
    }
}

pub static PROJECT_INSTANCE: std::sync::OnceLock<std::sync::Mutex<Project>> = std::sync::OnceLock::new();

pub fn get_project() -> &'static std::sync::Mutex<Project> {
    PROJECT_INSTANCE.get_or_init(|| std::sync::Mutex::new(Project::default()))
}