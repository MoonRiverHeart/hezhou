use crate::texture::{Texture, TextureId};

pub type MaterialId = u64;

pub struct Material {
    pub id: MaterialId,
    pub shader_name: String,
    pub textures: Vec<(u32, TextureId)>,
    pub base_color: crate::color::Color,
}

impl Material {
    pub fn new(id: MaterialId) -> Self {
        Self {
            id,
            shader_name: "default".to_string(),
            textures: Vec::new(),
            base_color: crate::color::Color::white(),
        }
    }
    
    pub fn set_shader(&mut self, name: &str) {
        self.shader_name = name.to_string();
    }
    
    pub fn set_texture(&mut self, slot: u32, texture_id: TextureId) {
        self.textures.push((slot, texture_id));
    }
    
    pub fn remove_texture(&mut self, slot: u32) {
        self.textures.retain(|(s, _)| *s != slot);
    }
    
    pub fn get_texture(&self, slot: u32) -> Option<TextureId> {
        self.textures.iter()
            .find(|(s, _)| *s == slot)
            .map(|(_, id)| *id)
    }
    
    pub fn set_base_color(&mut self, color: crate::color::Color) {
        self.base_color = color;
    }
}