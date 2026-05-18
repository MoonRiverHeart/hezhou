use crate::canvas::*;
use crate::font_atlas::FontAtlas;
use crate::types::*;
use crate::widget::*;
use std::collections::HashMap;

pub struct WidgetTree {
    pub root: Option<WidgetId>,
    pub nodes: HashMap<WidgetId, WidgetNode>,
    pub parent_map: HashMap<WidgetId, WidgetId>,
    pub children_map: HashMap<WidgetId, Vec<WidgetId>>,
}

struct WidgetNode {
    widget: Box<dyn Widget>,
    flags: crate::widget::WidgetFlags,
    render_data: Option<RenderData>,
}

#[repr(C)]
#[derive(Clone)]
pub struct RenderData {
    pub draw_commands: Vec<crate::canvas::DrawCommand>,
    pub bounds: Rect,
    pub z_index: i32,
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

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn recenter_widget(&mut self, id: WidgetId, container_width: f32, container_height: f32) {
        if let Some(node) = self.nodes.get_mut(&id) {
            let current_layout = *node.widget.layout();
            let new_x = (container_width - current_layout.width) / 2.0;
            let new_y = (container_height - current_layout.height) / 2.0;
            
            node.widget.set_layout(crate::layout::Layout::new(
                new_x,
                new_y,
                current_layout.width,
                current_layout.height,
            ));
        }
    }

    pub fn set_root(&mut self, widget: Box<dyn Widget>) {
        let id = widget.id();
        self.root = Some(id);
        self.nodes.insert(
            id,
            WidgetNode {
                widget,
                flags: crate::widget::WidgetFlags::default(),
                render_data: None,
            },
        );
        self.children_map.insert(id, Vec::new());
    }

    pub fn add_widget(&mut self, widget: Box<dyn Widget>, parent: WidgetId) {
        let id = widget.id();

        self.nodes.insert(
            id,
            WidgetNode {
                widget,
                flags: crate::widget::WidgetFlags::default(),
                render_data: None,
            },
        );

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

    pub fn get_widget_mut(&mut self, id: WidgetId) -> Option<&mut Box<dyn Widget>> {
        self.nodes.get_mut(&id).map(|node| &mut node.widget)
    }

    pub fn get_children(&self, id: WidgetId) -> &[WidgetId] {
        self.children_map
            .get(&id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn get_parent(&self, id: WidgetId) -> Option<WidgetId> {
        self.parent_map.get(&id).copied()
    }

    pub fn hit_test(&self, point: Point) -> Option<WidgetId> {
        self.hit_test_recursive(self.root?, point, 0.0, 0.0)
    }
    
    pub fn get_all_widget_ids(&self) -> Vec<WidgetId> {
        self.nodes.keys().cloned().collect()
    }

    fn hit_test_recursive(&self, id: WidgetId, point: Point, parent_abs_x: f32, parent_abs_y: f32) -> Option<WidgetId> {
        if let Some(node) = self.nodes.get(&id) {
            let layout = *node.widget.as_ref().layout();
            let abs_x = parent_abs_x + layout.x;
            let abs_y = parent_abs_y + layout.y;
            
            let abs_bounds = Rect::new(abs_x, abs_y, layout.width, layout.height);
            if abs_bounds.contains(&point) {
                let children = self.get_children(id);
                for child in children.iter().rev() {
                    if let Some(hit) = self.hit_test_recursive(*child, point, abs_x, abs_y) {
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

pub fn perform_layout(&mut self, font_atlas: &FontAtlas) {
        if let Some(root_id) = self.root {
            let _ = self.measure_and_layout(root_id, font_atlas);
        }
    }
    
    pub fn get_absolute_layout(&self, id: WidgetId) -> Option<crate::layout::Layout> {
        let node = self.nodes.get(&id)?;
        let layout = *node.widget.layout();
        
        if let Some(parent_id) = self.parent_map.get(&id) {
            let parent_layout = self.get_absolute_layout(*parent_id)?;
            Some(crate::layout::Layout::new(
                parent_layout.x + layout.x,
                parent_layout.y + layout.y,
                layout.width,
                layout.height,
            ))
        } else {
            Some(layout)
        }
    }
    
    fn measure_and_layout(&mut self, id: WidgetId, font_atlas: &FontAtlas) -> (f32, f32) {
        let children = self.get_children(id).to_vec();

        let mut child_sizes = Vec::new();
        for &child_id in &children {
            let size = self.measure_and_layout(child_id, font_atlas);
            child_sizes.push(size);
        }

        let widget_type = self
            .nodes
            .get(&id)
            .map(|n| n.widget.widget_type())
            .unwrap_or("");

        let (width, height) = match widget_type {
            "HStack" => {
                let mut total_width: f32 = 0.0;
                let mut max_height: f32 = 0.0;
                let spacing = self
                    .nodes
                    .get(&id)
                    .and_then(|n| {
                        if let Some(hstack) =
                            n.widget.as_any().downcast_ref::<crate::widgets::HStack>()
                        {
                            Some(hstack.spacing)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(8.0);

                for (i, (w, h)) in child_sizes.iter().enumerate() {
                    total_width += w;
                    max_height = max_height.max(*h);
                    if i < children.len() - 1 {
                        total_width += spacing;
                    }
                }

                (total_width, max_height)
            }
            "VStack" => {
                let mut max_width: f32 = 0.0;
                let mut total_height: f32 = 0.0;
                let spacing = self
                    .nodes
                    .get(&id)
                    .and_then(|n| {
                        if let Some(vstack) =
                            n.widget.as_any().downcast_ref::<crate::widgets::VStack>()
                        {
                            Some(vstack.spacing)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(8.0);

                for (i, (w, h)) in child_sizes.iter().enumerate() {
                    max_width = max_width.max(*w);
                    total_height += h;
                    if i < children.len() - 1 {
                        total_height += spacing;
                    }
                }

                (max_width, total_height)
            }
            _ => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.widget.as_mut().measure(font_atlas)
                } else {
                    (0.0, 0.0)
                }
            }
        };

        if let Some(node) = self.nodes.get_mut(&id) {
            let current_layout = *node.widget.layout();
            if current_layout.width == 0.0 || current_layout.height == 0.0 {
                node.widget.set_layout(crate::layout::Layout::new(
                    current_layout.x,
                    current_layout.y,
                    width,
                    height,
                ));
            }
        }

        match widget_type {
            "HStack" => {
                self.layout_hstack_children(id, &children, &child_sizes);
            }
            "VStack" => {
                self.layout_vstack_children(id, &children, &child_sizes);
            }
            _ => {}
        }

        let final_layout = self
            .nodes
            .get(&id)
            .map(|n| n.widget.layout())
            .map(|l| (l.width, l.height))
            .unwrap_or((width, height));

        final_layout
    }

    fn layout_hstack_children(
        &mut self,
        parent_id: WidgetId,
        children: &[WidgetId],
        child_sizes: &[(f32, f32)],
    ) {
        let spacing = self
            .nodes
            .get(&parent_id)
            .and_then(|n| {
                if let Some(hstack) = n.widget.as_any().downcast_ref::<crate::widgets::HStack>() {
                    Some(hstack.spacing)
                } else {
                    None
                }
            })
            .unwrap_or(8.0);

        let mut current_x = 0.0;

        for (i, &child_id) in children.iter().enumerate() {
            let (w, h) = child_sizes[i];

            if let Some(node) = self.nodes.get_mut(&child_id) {
                let child_layout = *node.widget.layout();
                let y = 0.0;

                node.widget.set_layout(crate::layout::Layout::new(
                    current_x,
                    y,
                    w.max(child_layout.width),
                    h.max(child_layout.height),
                ));
            }

            current_x += w + spacing;
        }
    }

    fn layout_vstack_children(
        &mut self,
        parent_id: WidgetId,
        children: &[WidgetId],
        child_sizes: &[(f32, f32)],
    ) {
        let spacing = self
            .nodes
            .get(&parent_id)
            .and_then(|n| {
                if let Some(vstack) = n.widget.as_any().downcast_ref::<crate::widgets::VStack>() {
                    Some(vstack.spacing)
                } else {
                    None
                }
            })
            .unwrap_or(8.0);
        
        let mut current_y = 0.0;

        for (i, &child_id) in children.iter().enumerate() {
            let (w, h) = child_sizes[i];

            if let Some(node) = self.nodes.get_mut(&child_id) {
                let child_layout = *node.widget.layout();
                let x = 0.0;

                node.widget.set_layout(crate::layout::Layout::new(
                    x,
                    current_y,
                    w.max(child_layout.width),
                    h.max(child_layout.height),
                ));
            }

            current_y += h + spacing;
        }
    }

    pub fn generate_render_data(&mut self, font_atlas: &FontAtlas) -> Vec<RenderData> {
        let mut render_data = Vec::new();

        if let Some(root_id) = self.root {
            self.generate_render_data_recursive(root_id, 0.0, 0.0, &mut render_data, font_atlas);
        }

        render_data
    }

    fn generate_render_data_recursive(
        &mut self,
        id: WidgetId,
        parent_abs_x: f32,
        parent_abs_y: f32,
        render_data: &mut Vec<RenderData>,
        font_atlas: &FontAtlas,
    ) {
        if let Some(node) = self.nodes.get_mut(&id) {
            if node.widget.as_ref().state() != WidgetState::Disabled {
                let layout = *node.widget.as_ref().layout();
                let abs_x = parent_abs_x + layout.x;
                let abs_y = parent_abs_y + layout.y;
                
                let mut canvas = Canvas::with_font_atlas(font_atlas as *const FontAtlas, 0);
                node.widget.as_mut().draw(&mut canvas);
                
                let commands = canvas.get_commands().to_vec();
                let absolute_commands: Vec<DrawCommand> = commands
                    .iter()
                    .map(|cmd| Self::offset_draw_command(cmd, abs_x, abs_y))
                    .collect();

                render_data.push(RenderData {
                    draw_commands: absolute_commands,
                    bounds: Rect::new(abs_x, abs_y, layout.width, layout.height),
                    z_index: 0,
                });

                node.render_data = Some(render_data.last().unwrap().clone());
            }
        }

        let layout = self.nodes.get(&id).map(|n| *n.widget.layout()).unwrap_or_default();
        let abs_x = parent_abs_x + layout.x;
        let abs_y = parent_abs_y + layout.y;
        
        let children = self.get_children(id).to_vec();
        for child in children {
            self.generate_render_data_recursive(child, abs_x, abs_y, render_data, font_atlas);
        }
    }
    
    fn offset_draw_command(cmd: &DrawCommand, offset_x: f32, offset_y: f32) -> DrawCommand {
        match cmd {
            DrawCommand::Rect { bounds, width, height, fill_color, stroke_color, stroke_width, border_radius } => {
                DrawCommand::Rect {
                    bounds: Point::new(bounds.x + offset_x, bounds.y + offset_y),
                    width: *width,
                    height: *height,
                    fill_color: *fill_color,
                    stroke_color: *stroke_color,
                    stroke_width: *stroke_width,
                    border_radius: *border_radius,
                }
            }
            DrawCommand::Text { bounds, width, height, text, text_len, font_size, font_color, alignment } => {
                DrawCommand::Text {
                    bounds: Point::new(bounds.x + offset_x, bounds.y + offset_y),
                    width: *width,
                    height: *height,
                    text: *text,
                    text_len: *text_len,
                    font_size: *font_size,
                    font_color: *font_color,
                    alignment: *alignment,
                }
            }
            DrawCommand::Line { start, end, color, width } => {
                DrawCommand::Line {
                    start: Point::new(start.x + offset_x, start.y + offset_y),
                    end: Point::new(end.x + offset_x, end.y + offset_y),
                    color: *color,
                    width: *width,
                }
            }
            DrawCommand::Image { bounds, width, height, texture_id, uv } => {
                DrawCommand::Image {
                    bounds: Point::new(bounds.x + offset_x, bounds.y + offset_y),
                    width: *width,
                    height: *height,
                    texture_id: *texture_id,
                    uv: *uv,
                }
            }
            DrawCommand::Shadow { bounds, shadow } => {
                DrawCommand::Shadow {
                    bounds: Rect::new(
                        bounds.x + offset_x,
                        bounds.y + offset_y,
                        bounds.width,
                        bounds.height,
                    ),
                    shadow: shadow.clone(),
                }
            }
            DrawCommand::ClipRect { rect } => {
                DrawCommand::ClipRect {
                    rect: Rect::new(
                        rect.x + offset_x,
                        rect.y + offset_y,
                        rect.width,
                        rect.height,
                    ),
                }
            }
            DrawCommand::ClearClip => DrawCommand::ClearClip,
            DrawCommand::SetTransform { transform } => DrawCommand::SetTransform { transform: *transform },
            DrawCommand::ResetTransform => DrawCommand::ResetTransform,
        }
    }
}

impl Default for WidgetTree {
    fn default() -> Self {
        Self::new()
    }
}
