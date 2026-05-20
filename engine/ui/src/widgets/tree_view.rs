use crate::canvas::*;
use crate::event::*;
use crate::font_atlas::FontAtlas;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::widgets::TreeNode;

pub struct TreeView {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: WidgetFlags,
    root_nodes: Vec<WidgetId>,
    selected_node: Option<WidgetId>,
    node_height: f32,
    indent_width: f32,
    content_scale: f32,
    scroll_offset: f32,
    node_parent_map: std::collections::HashMap<WidgetId, WidgetId>,
}

impl TreeView {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 200.0, 300.0),
            style: Style::new().with_background(Color::new(0.15, 0.15, 0.15, 1.0)),
            state: WidgetState::Normal,
            flags: WidgetFlags::default(),
            root_nodes: Vec::new(),
            selected_node: None,
            node_height: 24.0,
            indent_width: 20.0,
            content_scale: 1.0,
            scroll_offset: 0.0,
            node_parent_map: std::collections::HashMap::new(),
        }
    }

    pub fn with_layout(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.layout = Layout::new(x, y, width, height);
        self
    }

    pub fn with_content_scale(mut self, scale: f32) -> Self {
        self.content_scale = scale;
        self.node_height = 24.0 * scale;
        self.indent_width = 20.0 * scale;
        self
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.node_height = 24.0 * scale;
        self.indent_width = 20.0 * scale;
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn add_root_node(&mut self, node_id: WidgetId) {
        self.root_nodes.push(node_id);
        self.children.push(node_id);
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn add_child_node(&mut self, parent_id: WidgetId, child_id: WidgetId) {
        self.node_parent_map.insert(child_id, parent_id);
        self.children.push(child_id);
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn remove_node(&mut self, node_id: WidgetId) {
        self.root_nodes.retain(|&id| id != node_id);
        self.children.retain(|&id| id != node_id);
        self.node_parent_map.remove(&node_id);
        
        if self.selected_node == Some(node_id) {
            self.selected_node = None;
        }
        
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn set_selected(&mut self, node_id: WidgetId) {
        self.selected_node = Some(node_id);
        self.flags.dirty_render = true;
    }

    pub fn clear_selection(&mut self) {
        self.selected_node = None;
        self.flags.dirty_render = true;
    }

    pub fn selected_node(&self) -> Option<WidgetId> {
        self.selected_node
    }

    pub fn expand_node(&mut self, node_id: WidgetId) {
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn collapse_node(&mut self, node_id: WidgetId) {
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn root_nodes(&self) -> &[WidgetId] {
        &self.root_nodes
    }

    pub fn get_parent_node(&self, node_id: WidgetId) -> Option<WidgetId> {
        self.node_parent_map.get(&node_id).copied()
    }

    fn collect_visible_nodes(&self, tree: &crate::WidgetTree) -> Vec<(WidgetId, f32)> {
        let mut result = Vec::new();
        let mut current_y = 0.0;

        for root_id in &self.root_nodes {
            self.collect_visible_nodes_recursive(*root_id, tree, &mut result, &mut current_y, 0);
        }

        result
    }

    fn collect_visible_nodes_recursive(
        &self,
        node_id: WidgetId,
        tree: &crate::WidgetTree,
        result: &mut Vec<(WidgetId, f32)>,
        current_y: &mut f32,
        depth: usize,
    ) {
        if let Some(widget) = tree.get_widget(node_id) {
            if let Some(node) = widget.as_any().downcast_ref::<TreeNode>() {
                result.push((node_id, *current_y));
                *current_y += self.node_height;

                if node.is_expanded() {
                    for child_id in node.children() {
                        self.collect_visible_nodes_recursive(*child_id, tree, result, current_y, depth + 1);
                    }
                }
            }
        }
    }

    fn update_node_positions(&mut self, tree: &mut crate::WidgetTree) {
        let visible_nodes = self.collect_visible_nodes(tree);

        for (node_id, y) in visible_nodes {
            if let Some(widget) = tree.get_widget_mut(node_id) {
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    let indent = node.depth() as f32 * self.indent_width;
                    let width = self.layout.width - indent;
                    node.set_layout(Layout::new(indent, y - self.scroll_offset, width, self.node_height));
                    
                    if self.selected_node == Some(node_id) {
                        node.set_selected(true);
                    } else {
                        node.set_selected(false);
                    }
                }
            }
        }
    }

    pub fn layout_nodes(&mut self, tree: &mut crate::WidgetTree) {
        self.update_node_positions(tree);
    }
}

impl Widget for TreeView {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn parent(&self) -> Option<WidgetId> {
        if self.parent_id.is_valid() {
            Some(self.parent_id)
        } else {
            None
        }
    }

    fn set_parent(&mut self, parent: WidgetId) {
        self.parent_id = parent;
    }

    fn children(&self) -> &[WidgetId] {
        &self.children
    }

    fn add_child(&mut self, child: WidgetId) {
        self.children.push(child);
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    fn remove_child(&mut self, child: WidgetId) {
        self.children.retain(|&id| id != child);
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    fn style(&self) -> &Style {
        &self.style
    }

    fn set_style(&mut self, style: Style) {
        self.style = style;
        self.flags.dirty_style = true;
        self.flags.dirty_render = true;
    }

    fn state(&self) -> WidgetState {
        self.state
    }

    fn set_state(&mut self, state: WidgetState) {
        self.state = state;
        self.flags.dirty_render = true;
    }

    fn widget_type(&self) -> &'static str {
        "TreeView"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn measure(&self, _font_atlas: &FontAtlas) -> (f32, f32) {
        (self.layout.width, self.layout.height)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;

        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                match &event.data {
                    EventData::Touch(touch) => {
                        let adjusted_y = touch.y + self.scroll_offset;
                        
                        for root_id in &self.root_nodes {
                            if self.handle_touch_recursive(*root_id, touch.x, adjusted_y, event.timestamp) {
                                return EventResult::Handled;
                            }
                        }
                    }
                    _ => {}
                }
            }

            _ => {}
        }

        EventResult::Ignored
    }
}

impl TreeView {
    fn handle_touch_recursive(&self, _node_id: WidgetId, _x: f32, _y: f32, _timestamp: u64) -> bool {
        false
    }
}