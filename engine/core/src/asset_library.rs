use std::collections::HashMap;
use crate::math::Transform;
use crate::ecs::{Entity, Scene, RenderableComponent, BoundsComponent};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AssetType {
    EntityTemplate = 0,
    Texture = 1,
    Material = 2,
    Script = 3,
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::EntityTemplate
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeshType {
    Cube = 0,
    Sphere = 1,
    Plane = 2,
    Cylinder = 3,
    Cone = 4,
    Custom = 5,
    Bunny = 6,  // 低多边形bunny模型（程序化生成）
    CornellBox = 7,  // Cornell Box场景（程序化生成）
}

impl MeshType {
    pub fn to_mesh_path(&self) -> String {
        match self {
            MeshType::Cube => "builtin://cube",
            MeshType::Sphere => "builtin://sphere",
            MeshType::Plane => "builtin://plane",
            MeshType::Cylinder => "builtin://cylinder",
            MeshType::Cone => "builtin://cone",
            MeshType::Custom => "builtin://cube",
            MeshType::Bunny => "builtin://bunny",
            MeshType::CornellBox => "builtin://cornell_box",
        }.to_string()
    }
    
    pub fn from_index(index: u32) -> Self {
        match index {
            0 => MeshType::Cube,
            1 => MeshType::Sphere,
            2 => MeshType::Plane,
            3 => MeshType::Cylinder,
            4 => MeshType::Cone,
            6 => MeshType::Bunny,
            7 => MeshType::CornellBox,
            _ => MeshType::Custom,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextureType {
    SolidWhite = 0,
    Checker = 1,
    UVTest = 2,
    Custom = 3,  // 自定义纹理（从PNG文件加载，路径存于AssetInfo.description）
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialType {
    Default = 0,
}

/// 材质信息：纹理引用 + 光照参数
#[derive(Clone, Debug)]
pub struct MaterialInfo {
    pub diffuse_texture_id: Option<u64>,  // 纹理资产ID（None=使用面着色）
    pub specular_strength: f32,           // 高光强度 (默认0.3)
    pub ambient_strength: f32,            // 环境光强度 (默认0.15)
    pub shininess: f32,                   // 高光指数 (默认32.0)
}

impl Default for MaterialInfo {
    fn default() -> Self {
        Self {
            diffuse_texture_id: None,
            specular_strength: 0.3,
            ambient_strength: 0.15,
            shininess: 32.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntityTemplate {
    pub name: String,
    pub mesh_type: MeshType,
    pub default_transform: Transform,
    pub default_components: Vec<ComponentTemplate>,
}

impl Default for EntityTemplate {
    fn default() -> Self {
        Self {
            name: String::new(),
            mesh_type: MeshType::Cube,
            default_transform: Transform::new(),
            default_components: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ComponentTemplate {
    pub component_type: String,
    pub properties: HashMap<String, serde_json::Value>,
}

impl Default for ComponentTemplate {
    fn default() -> Self {
        Self {
            component_type: String::new(),
            properties: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AssetInfo {
    pub id: u64,
    pub name: String,
    pub asset_type: AssetType,
    pub category: String,
    pub thumbnail_path: Option<String>,
    pub description: Option<String>,
    pub template_data: Option<EntityTemplate>,
    pub texture_type: Option<TextureType>,
    pub material_type: Option<MaterialType>,
    pub custom_mesh_path: Option<String>,     // 自定义mesh文件路径(asset://前缀)
    pub material_info: Option<MaterialInfo>,  // 材质信息
}

impl Default for AssetInfo {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            asset_type: AssetType::EntityTemplate,
            category: String::new(),
            thumbnail_path: None,
            description: None,
            template_data: None,
            texture_type: None,
            material_type: None,
            custom_mesh_path: None,
            material_info: None,
        }
    }
}

pub struct AssetLibrary {
    assets: HashMap<u64, AssetInfo>,
    categories: HashMap<String, Vec<u64>>,
    category_order: Vec<String>,
    next_id: u64,
}

impl Default for AssetLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLibrary {
    pub fn new() -> Self {
        let mut lib = Self {
            assets: HashMap::new(),
            categories: HashMap::new(),
            category_order: Vec::new(),
            next_id: 1,
        };
        lib.init_system_assets();
        lib
    }
    
    fn init_system_assets(&mut self) {
        self.add_entity_template("Cube", MeshType::Cube, "Basic cube geometry");
        self.add_entity_template("Sphere", MeshType::Sphere, "Basic sphere geometry");
        self.add_entity_template("Plane", MeshType::Plane, "Basic plane geometry");
        self.add_entity_template("Cylinder", MeshType::Cylinder, "Basic cylinder geometry");
        self.add_entity_template("Cone", MeshType::Cone, "Basic cone geometry");
        self.add_entity_template("Bunny", MeshType::Bunny, "Low-poly bunny model (procedural)");
        self.add_entity_template("CornellBox", MeshType::CornellBox, "Classic Cornell Box rendering test scene");
        
        self.add_texture("Default White", TextureType::SolidWhite, "Solid white texture");
        self.add_texture("Checker", TextureType::Checker, "Checker pattern texture");
        self.add_texture("UV Test", TextureType::UVTest, "UV test pattern texture");
        
        self.add_material("Default Material", MaterialType::Default, "Basic default material");
    }
    
    fn next_asset_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    
    fn add_to_category(&mut self, category: &str, asset_id: u64) {
        if !self.categories.contains_key(category) {
            self.categories.insert(category.to_string(), Vec::new());
            self.category_order.push(category.to_string());
        }
        self.categories.get_mut(category).unwrap().push(asset_id);
    }
    
    pub fn add_entity_template(&mut self, name: &str, mesh_type: MeshType, description: &str) -> u64 {
        let id = self.next_asset_id();
        
        let template = EntityTemplate {
            name: name.to_string(),
            mesh_type,
            default_transform: Transform::new(),
            default_components: vec![
                ComponentTemplate {
                    component_type: "Renderable".to_string(),
                    properties: HashMap::new(),
                },
            ],
        };
        
        let asset = AssetInfo {
            id,
            name: name.to_string(),
            asset_type: AssetType::EntityTemplate,
            category: "Primitives".to_string(),
            thumbnail_path: None,
            description: Some(description.to_string()),
            template_data: Some(template),
            texture_type: None,
            material_type: None,
            custom_mesh_path: None,
            material_info: None,
        };
        
        self.assets.insert(id, asset);
        self.add_to_category("Primitives", id);
        id
    }
    
    pub fn add_texture(&mut self, name: &str, texture_type: TextureType, description: &str) -> u64 {
        let id = self.next_asset_id();
        
        let asset = AssetInfo {
            id,
            name: name.to_string(),
            asset_type: AssetType::Texture,
            category: "Textures".to_string(),
            thumbnail_path: None,
            description: Some(description.to_string()),
            template_data: None,
            texture_type: Some(texture_type),
            material_type: None,
            custom_mesh_path: None,
            material_info: None,
        };
        
        self.assets.insert(id, asset);
        self.add_to_category("Textures", id);
        id
    }
    
    pub fn add_material(&mut self, name: &str, material_type: MaterialType, description: &str) -> u64 {
        let id = self.next_asset_id();
        
        let asset = AssetInfo {
            id,
            name: name.to_string(),
            asset_type: AssetType::Material,
            category: "Materials".to_string(),
            thumbnail_path: None,
            description: Some(description.to_string()),
            template_data: None,
            texture_type: None,
            material_type: Some(material_type),
            custom_mesh_path: None,
            material_info: None,
        };
        
        self.assets.insert(id, asset);
        self.add_to_category("Materials", id);
        id
    }
    
    /// 加载自定义纹理资产（从PNG文件路径）
    pub fn add_custom_texture(&mut self, name: &str, file_path: &str, description: &str) -> u64 {
        let id = self.next_asset_id();
        let asset = AssetInfo {
            id,
            name: name.to_string(),
            asset_type: AssetType::Texture,
            category: "Textures".to_string(),
            thumbnail_path: Some(file_path.to_string()),
            description: Some(description.to_string()),
            template_data: None,
            texture_type: Some(TextureType::Custom),
            material_type: None,
            custom_mesh_path: None,
            material_info: None,
        };
        self.assets.insert(id, asset);
        self.add_to_category("Textures", id);
        id
    }
    
    /// 加载自定义mesh资产（从OBJ文件路径）
    pub fn add_custom_mesh(&mut self, name: &str, file_path: &str, description: &str) -> u64 {
        let id = self.next_asset_id();
        let asset = AssetInfo {
            id,
            name: name.to_string(),
            asset_type: AssetType::EntityTemplate,
            category: "Models".to_string(),
            thumbnail_path: Some(file_path.to_string()),
            description: Some(description.to_string()),
            template_data: Some(EntityTemplate {
                name: name.to_string(),
                mesh_type: MeshType::Custom,
                ..Default::default()
            }),
            texture_type: None,
            material_type: None,
            custom_mesh_path: Some(format!("asset://{}", file_path)),
            material_info: None,
        };
        self.assets.insert(id, asset);
        self.add_to_category("Models", id);
        id
    }
    
    /// 创建自定义材质
    pub fn add_custom_material(&mut self, name: &str, diffuse_texture_id: Option<u64>, description: &str) -> u64 {
        let id = self.next_asset_id();
        let asset = AssetInfo {
            id,
            name: name.to_string(),
            asset_type: AssetType::Material,
            category: "Materials".to_string(),
            thumbnail_path: None,
            description: Some(description.to_string()),
            template_data: None,
            texture_type: None,
            material_type: Some(MaterialType::Default),
            custom_mesh_path: None,
            material_info: Some(MaterialInfo {
                diffuse_texture_id,
                specular_strength: 0.3,
                ambient_strength: 0.15,
                shininess: 32.0,
            }),
        };
        self.assets.insert(id, asset);
        self.add_to_category("Materials", id);
        id
    }
    
    pub fn get_category_count(&self) -> usize {
        self.category_order.len()
    }
    
    pub fn get_category_name(&self, index: usize) -> Option<&String> {
        self.category_order.get(index)
    }
    
    pub fn get_asset_count_in_category(&self, category_index: usize) -> usize {
        if let Some(category_name) = self.category_order.get(category_index) {
            self.categories.get(category_name).map(|v| v.len()).unwrap_or(0)
        } else {
            0
        }
    }
    
    pub fn get_asset_by_category_index(&self, category_index: usize, asset_index: usize) -> Option<&AssetInfo> {
        if let Some(category_name) = self.category_order.get(category_index) {
            if let Some(asset_ids) = self.categories.get(category_name) {
                if let Some(asset_id) = asset_ids.get(asset_index) {
                    return self.assets.get(asset_id);
                }
            }
        }
        None
    }
    
    pub fn get_asset(&self, id: u64) -> Option<&AssetInfo> {
        self.assets.get(&id)
    }
    
    pub fn create_entity_from_template(&self, template_id: u64, scene: &mut Scene) -> Option<Entity> {
        let asset = self.assets.get(&template_id)?;
        let template = asset.template_data.as_ref()?;
        
        let entity = scene.create_entity();
        
        let mesh_path = template.mesh_type.to_mesh_path();
        scene.world.add_component(entity, RenderableComponent::new(mesh_path));
        scene.world.add_component(entity, BoundsComponent::cube(2.0));
        
        let name = format!("{}_{}", template.name, entity.id);
        scene.set_entity_name(entity, name);
        
        Some(entity)
    }
    
    pub fn create_entity_with_mesh(&self, mesh_type: MeshType, scene: &mut Scene) -> Entity {
        let entity = scene.create_entity();
        
        let mesh_path = mesh_type.to_mesh_path();
        scene.world.add_component(entity, RenderableComponent::new(mesh_path));
        scene.world.add_component(entity, BoundsComponent::cube(2.0));
        
        let mesh_name = match mesh_type {
            MeshType::Cube => "Cube",
            MeshType::Sphere => "Sphere",
            MeshType::Plane => "Plane",
            MeshType::Cylinder => "Cylinder",
            MeshType::Cone => "Cone",
            MeshType::Custom => "Custom",
            MeshType::Bunny => "Bunny",
            MeshType::CornellBox => "CornellBox",
        };
        let name = format!("{}_{}", mesh_name, entity.id);
        scene.set_entity_name(entity, name);
        
        entity
    }
}

pub static ASSET_LIBRARY: std::sync::OnceLock<std::sync::RwLock<AssetLibrary>> = std::sync::OnceLock::new();

pub fn get_asset_library() -> &'static std::sync::RwLock<AssetLibrary> {
    ASSET_LIBRARY.get_or_init(|| std::sync::RwLock::new(AssetLibrary::new()))
}