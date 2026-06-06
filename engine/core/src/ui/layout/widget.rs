use slotmap::{SlotMap, new_key_type};
use super::geometry::{Size, Rect, Axis};
use super::style::Style;
use super::text::GlyphInfo;

new_key_type! {
    pub struct WidgetId;
}

pub enum WidgetType {
    Container,
    Row,
    Column,
    Text(TextData),
    Spacer(Size),
}

pub struct TextData {
    pub content: String,
    pub font_size: f32,
    pub glyphs: Option<Vec<GlyphInfo>>,
}

impl TextData {
    pub fn new(content: impl Into<String>, font_size: f32) -> Self {
        TextData {
            content: content.into(),
            font_size,
            glyphs: None,
        }
    }
    
    pub fn with_glyphs(content: impl Into<String>, font_size: f32, glyphs: Vec<GlyphInfo>) -> Self {
        TextData {
            content: content.into(),
            font_size,
            glyphs: Some(glyphs),
        }
    }
}

pub struct WidgetNode {
    pub id: WidgetId,
    pub widget_type: WidgetType,
    pub parent: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub style: Style,
    pub layout: Option<Rect>,
    pub dirty: bool,
}

impl WidgetNode {
    pub fn new(id: WidgetId, widget_type: WidgetType, style: Style) -> Self {
        WidgetNode {
            id,
            widget_type,
            parent: None,
            children: Vec::new(),
            style,
            layout: None,
            dirty: true,
        }
    }
    
    pub fn is_flex_container(&self) -> bool {
        matches!(self.widget_type, WidgetType::Row | WidgetType::Column)
    }
    
    pub fn main_axis(&self) -> Option<Axis> {
        match self.widget_type {
            WidgetType::Row => Some(Axis::Horizontal),
            WidgetType::Column => Some(Axis::Vertical),
            _ => None,
        }
    }
}

pub struct WidgetTree {
    pub nodes: SlotMap<WidgetId, WidgetNode>,
    root: Option<WidgetId>,
}

impl WidgetTree {
    pub fn new() -> Self {
        WidgetTree {
            nodes: SlotMap::with_key(),
            root: None,
        }
    }
    
    pub fn create_node(&mut self, widget_type: WidgetType, style: Style) -> WidgetId {
        let id = self.nodes.insert_with_key(|id| WidgetNode::new(id, widget_type, style));
        id
    }
    
    pub fn add_child(&mut self, parent_id: WidgetId, child_id: WidgetId) {
        self.nodes[child_id].parent = Some(parent_id);
        self.nodes[parent_id].children.push(child_id);
        self.mark_dirty(parent_id);
    }
    
    pub fn set_root(&mut self, node_id: WidgetId) {
        self.root = Some(node_id);
        self.mark_dirty(node_id);
    }
    
    pub fn get(&self, id: WidgetId) -> &WidgetNode {
        &self.nodes[id]
    }
    
    pub fn get_mut(&mut self, id: WidgetId) -> &mut WidgetNode {
        &mut self.nodes[id]
    }
    
    pub fn mark_dirty(&mut self, id: WidgetId) {
        let mut current = Some(id);
        while let Some(node_id) = current {
            let node = &mut self.nodes[node_id];
            node.dirty = true;
            current = node.parent;
        }
    }
    
    pub fn root(&self) -> Option<WidgetId> {
        self.root
    }
    
    pub fn get_children(&self, id: WidgetId) -> Vec<WidgetId> {
        self.nodes[id].children.clone()
    }
}