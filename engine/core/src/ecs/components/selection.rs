use crate::ecs::Component;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionLevel {
    None = 0,
    Entity = 1,
    SubMesh = 2,
    Face = 3,
    Edge = 4,
    Vertex = 5,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SelectionComponent {
    pub selected: bool,
    pub level: SelectionLevel,
    pub submesh_index: Option<u32>,
    pub face_indices: Vec<u32>,
    pub edge_indices: Vec<u32>,
    pub vertex_indices: Vec<u32>,
}

impl Component for SelectionComponent {
    fn type_id() -> crate::ecs::ComponentTypeId { 300 }
    fn type_name() -> &'static str { "SelectionComponent" }
}

impl Default for SelectionComponent {
    fn default() -> Self {
        Self {
            selected: false,
            level: SelectionLevel::None,
            submesh_index: None,
            face_indices: Vec::new(),
            edge_indices: Vec::new(),
            vertex_indices: Vec::new(),
        }
    }
}

impl SelectionComponent {
    pub fn entity_level() -> Self {
        Self {
            selected: true,
            level: SelectionLevel::Entity,
            ..Default::default()
        }
    }
    
    pub fn submesh_level(index: u32) -> Self {
        Self {
            selected: true,
            level: SelectionLevel::SubMesh,
            submesh_index: Some(index),
            ..Default::default()
        }
    }
    
    pub fn face_level(indices: Vec<u32>) -> Self {
        Self {
            selected: true,
            level: SelectionLevel::Face,
            face_indices: indices,
            ..Default::default()
        }
    }
    
    pub fn clear(&mut self) {
        self.selected = false;
        self.level = SelectionLevel::None;
        self.submesh_index = None;
        self.face_indices.clear();
        self.edge_indices.clear();
        self.vertex_indices.clear();
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct HighlightSettings {
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub color_a: f32,
    pub outline_width: f32,
    pub pulse_speed: f32,
    pub show_wireframe: bool,
}

impl Component for HighlightSettings {
    fn type_id() -> crate::ecs::ComponentTypeId { 301 }
    fn type_name() -> &'static str { "HighlightSettings" }
}

impl HighlightSettings {
    pub fn orange_highlight() -> Self {
        Self {
            color_r: 1.0,
            color_g: 0.6,
            color_b: 0.3,
            color_a: 0.5,
            outline_width: 0.05,
            pulse_speed: 0.0,
            show_wireframe: false,
        }
    }
    
    pub fn wireframe_highlight() -> Self {
        Self {
            color_r: 1.0,
            color_g: 0.7,
            color_b: 0.4,
            color_a: 0.8,
            outline_width: 0.02,
            pulse_speed: 2.0,
            show_wireframe: true,
        }
    }
}