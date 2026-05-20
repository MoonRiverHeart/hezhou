use std::path::PathBuf;
use std::collections::HashMap;
use std::fs;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::math::{Vec3, Quaternion};
use crate::ecs::{Entity, Scene, GameState, LocalTransform, RenderableComponent, BoundsComponent};

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
pub struct ProjectEntity {
    pub id: u64,
    pub name: String,
    pub transform: ProjectEntityTransform,
    pub mesh_type: String,
    pub scripts: Vec<ProjectScriptBinding>,
    pub components: HashMap<String, serde_json::Value>,
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
    project_path: PathBuf,
    is_loaded: bool,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            file: ProjectFile::default(),
            project_path: PathBuf::new(),
            is_loaded: false,
        }
    }
}

impl Project {
    pub fn new(name: String, project_path: PathBuf) -> Self {
        let now = Utc::now();
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
            is_loaded: false,
        }
    }
    
    pub fn create_new(name: String, parent_path: PathBuf) -> Result<Self, ProjectError> {
        let project_path = parent_path.join(&name);
        
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
        project.save()?;
        
        Ok(project)
    }
    
    pub fn load(path: PathBuf) -> Result<Self, ProjectError> {
        let project_file_path = path.join("project.json");
        
        if !project_file_path.exists() {
            return Err(ProjectError::FileNotFound(project_file_path.to_string_lossy().to_string()));
        }
        
        let json = fs::read_to_string(&project_file_path)?;
        let file: ProjectFile = serde_json::from_str(&json)?;
        
        let mut project = Project::new(file.name.clone(), path.clone());
        project.file = file;
        project.is_loaded = true;
        
        Ok(project)
    }
    
    pub fn save(&mut self) -> Result<(), ProjectError> {
        self.file.modified_at = Utc::now();
        
        let project_file_path = self.project_path.join("project.json");
        let json = serde_json::to_string_pretty(&self.file)?;
        fs::write(&project_file_path, json)?;
        
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
        for project_entity in &self.file.entities {
            let entity = scene.create_entity();
            
            let mesh_path = Self::mesh_type_to_path(&project_entity.mesh_type);
            scene.world.add_component(entity, RenderableComponent::new(mesh_path));
            scene.world.add_component(entity, BoundsComponent::cube(2.0));
            
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
            
            for script_binding in &project_entity.scripts {
                scene.attach_script_binding(
                    entity,
                    script_binding.script_path.clone(),
                    script_binding.class_name.clone(),
                );
                scene.set_script_binding_enabled(entity, scene.get_script_binding_count(entity) - 1, script_binding.enabled);
            }
        }
    }
    
    pub fn sync_from_scene(&mut self, scene: &Scene) {
        self.file.entities.clear();
        
        for entity in &scene.root_entities {
            if let Some(name) = scene.get_entity_name(*entity) {
                let transform = scene.world.get_component::<LocalTransform>(*entity);
                let renderable = scene.world.get_component::<RenderableComponent>(*entity);
                
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
                    mesh_type: if let Some(r) = renderable {
                        Self::path_to_mesh_type(&r.mesh_path)
                    } else {
                        "Cube".to_string()
                    },
                    scripts: Self::collect_scripts(scene, *entity),
                    components: HashMap::new(),
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