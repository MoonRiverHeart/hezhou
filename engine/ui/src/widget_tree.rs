use crate::types::*;
use crate::widget::*;
use crate::canvas::*;
use std::collections::HashMap;

pub struct WidgetTree {
    root: Option<WidgetId>,
    nodes: HashMap<WidgetId, WidgetNode>,
    parent_map: HashMap<WidgetId, WidgetId>,
    children_map: HashMap<WidgetId, Vec<WidgetId>>,
}

struct WidgetNode {
    widget: Box<dyn Widget>,
    flags: crate::widget::WidgetFlags,
    render_data: Option<RenderData>,
}

#[repr(C)]
#[derive(Clone)]
pub struct RenderData {
    draw_commands: Vec<crate::canvas::DrawCommand>,
    bounds: Rect,
    z_index: i32,
}

impl WidgetTree {
    pub fn new() -> Self {
        Self {
            root: None,
            nodes: HashMap::new(),
            parent_map: HashMap::new(),
            children_map: HashMap::new(),
        }
    }
    
    pub fn set_root(&mut self, widget: Box<dyn Widget>) {
        let id = widget.id();
        self.root = Some(id);
        self.nodes.insert(id, WidgetNode {
            widget,
            flags: crate::widget::WidgetFlags::default(),
            render_data: None,
        });
        self.children_map.insert(id, Vec::new());
    }
    
    pub fn add_widget(&mut self, widget: Box<dyn Widget>, parent: WidgetId) {
        let id = widget.id();
        
        self.nodes.insert(id, WidgetNode {
            widget,
            flags: crate::widget::WidgetFlags::default(),
            render_data: None,
        });
        
        self.parent_map.insert(id, parent);
        self.children_map.insert(id, Vec::new());
        
        if let Some(children) = self.children_map.get_mut(&parent) {
            children.push(id);
        }
        
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.flags.dirty_children = true;
        }
    }
    
    pub fn remove_widget(&mut self, id: WidgetId) {
        if let Some(parent_id) = self.parent_map.remove(&id) {
            if let Some(children) = self.children_map.get_mut(&parent_id) {
                children.retain(|child| *child != id);
            }
        }
        
        if let Some(children) = self.children_map.remove(&id) {
            for child in children {
                self.remove_widget(child);
            }
        }
        
        self.nodes.remove(&id);
    }
    
    pub fn get_widget(&self, id: WidgetId) -> Option<&dyn Widget> {
        self.nodes.get(&id).map(|node| node.widget.as_ref())
    }
    
    pub fn get_widget_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        self.nodes.get_mut(&id).map(|node| node.widget.as_mut())
    }
    
    pub fn get_children(&self, id: WidgetId) -> &[WidgetId] {
        self.children_map.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    pub fn get_parent(&self, id: WidgetId) -> Option<WidgetId> {
        self.parent_map.get(&id).copied()
    }
    
    pub fn hit_test(&self, point: Point) -> Option<WidgetId> {
        self.hit_test_recursive(self.root?, point)
    }
    
    fn hit_test_recursive(&self, id: WidgetId, point: Point) -> Option<WidgetId> {
        if let Some(node) = self.nodes.get(&id) {
            if node.widget.hit_test(point) {
                for child in self.get_children(id) {
                    if let Some(hit) = self.hit_test_recursive(*child, point) {
                        return Some(hit);
                    }
                }
                return Some(id);
            }
        }
        None
    }
    
    pub fn find_path(&self, target: WidgetId) -> Vec<WidgetId> {
        let mut path = Vec::new();
        let mut current = target;
        
        while current.is_valid() {
            path.push(current);
            current = self.get_parent(current).unwrap_or_default();
        }
        
        path.reverse();
        path
    }
    
    pub fn update_layout(&mut self) {
        if let Some(root_id) = self.root {
            self.update_layout_recursive(root_id);
        }
    }
    
    fn update_layout_recursive(&mut self, id: WidgetId) {
        let children = self.get_children(id).to_vec();
        
        for child in children {
            self.update_layout_recursive(child);
        }
        
        if let Some(node) = self.nodes.get_mut(&id) {
            if node.flags.dirty_layout {
                node.flags.dirty_layout = false;
            }
        }
    }
    
    pub fn generate_render_data(&mut self) -> Vec<RenderData> {
        let mut render_data = Vec::new();
        
        if let Some(root_id) = self.root {
            self.generate_render_data_recursive(root_id, &mut render_data);
        }
        
        render_data
    }
    
    fn generate_render_data_recursive(&mut self, id: WidgetId, render_data: &mut Vec<RenderData>) {
        if let Some(node) = self.nodes.get_mut(&id) {
            if node.widget.state() != WidgetState::Disabled {
                let mut canvas = Canvas::new();
                node.widget.draw(&mut canvas);
                
                render_data.push(RenderData {
                    draw_commands: canvas.get_commands().to_vec(),
                    bounds: node.widget.layout().bounds(),
                    z_index: 0,
                });
                
                node.render_data = Some(render_data.last().unwrap().clone());
            }
        }
        
        let children = self.get_children(id).to_vec();
        for child in children {
            self.generate_render_data_recursive(child, render_data);
        }
    }
}

impl Default for WidgetTree {
    fn default() -> Self {
        Self::new()
    }
}