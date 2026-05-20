use crate::ecs::Component;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ScriptComponent {
    pub script_asset_id: u64,
    pub script_class_name: String,
    pub enabled: bool,
    pub execution_order: i32,
}

impl Component for ScriptComponent {
    fn type_id() -> crate::ecs::ComponentTypeId { 200 }
    fn type_name() -> &'static str { "ScriptComponent" }
}

impl Default for ScriptComponent {
    fn default() -> Self {
        Self {
            script_asset_id: 0,
            script_class_name: String::new(),
            enabled: true,
            execution_order: 0,
        }
    }
}

impl ScriptComponent {
    pub fn new(script_asset_id: u64, class_name: String) -> Self {
        Self {
            script_asset_id,
            script_class_name: class_name,
            ..Default::default()
        }
    }
    
    pub fn from_path(script_path: &str, class_name: &str) -> Self {
        Self {
            script_asset_id: hash_path(script_path),
            script_class_name: class_name.to_string(),
            ..Default::default()
        }
    }
}

fn hash_path(path: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct ScriptAsset {
    pub id: u64,
    pub path: String,
    pub dll_path: Option<String>,
    pub is_loaded: bool,
    pub last_modified: u64,
}

impl Component for ScriptAsset {
    fn type_id() -> crate::ecs::ComponentTypeId { 201 }
    fn type_name() -> &'static str { "ScriptAsset" }
}

impl ScriptAsset {
    pub fn new(path: String) -> Self {
        Self {
            id: hash_path(&path),
            path,
            ..Default::default()
        }
    }
    
    pub fn rotation_script() -> Self {
        Self::new("scripts/RotationScript.cs".to_string())
    }
}