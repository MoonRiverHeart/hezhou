use crate::canvas::*;
use crate::font_atlas::FontAtlas;
use crate::types::*;
use crate::widget::*;
use hezhou_dfx::*;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RenderLayer {
    Background = 0,
    Content = 1,
    Popup = 2,
    Overlay = 3,
}

impl Default for RenderLayer {
    fn default() -> Self {
        RenderLayer::Content
    }
}

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
    layer: RenderLayer,
}

#[repr(C)]
#[derive(Clone)]
pub struct RenderData {
    pub draw_commands: Vec<crate::canvas::DrawCommand>,
    pub bounds: Rect,
    pub z_index: i32,
    pub layer: RenderLayer,
    pub widget_id: u64,
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
    
    pub fn clear(&mut self) {
        if let Some(root_id) = self.root {
            let children = self.children_map.get(&root_id).cloned().unwrap_or_default();
            for child in children {
                self.remove_widget(child);
            }
            self.children_map.insert(root_id, Vec::new());
        } else {
            self.nodes.clear();
            self.parent_map.clear();
            self.children_map.clear();
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
                layer: RenderLayer::default(),
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
                layer: RenderLayer::default(),
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
    
    pub fn set_widget_layer(&mut self, id: WidgetId, layer: RenderLayer) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.layer = layer;
        }
    }
    
    pub fn get_widget_layer(&self, id: WidgetId) -> RenderLayer {
        self.nodes.get(&id).map(|node| node.layer).unwrap_or_default()
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
            // Skip hidden Dialog and its children
            if node.widget.as_ref().widget_type() == "Dialog" {
                if let Some(dialog) = node.widget.as_ref().as_any().downcast_ref::<crate::widgets::Dialog>() {
                    if !dialog.is_visible() {
                        return None;
                    }
                }
            }
            
            let layout = *node.widget.as_ref().layout();
            let abs_x = parent_abs_x + layout.x;
            let abs_y = parent_abs_y + layout.y;
            
            let abs_bounds = Rect::new(abs_x, abs_y, layout.width, layout.height);
            if abs_bounds.contains(&point) {
                // For TreeView widgets, only hit-test visible (expanded) nodes
                let children: Vec<WidgetId> = {
                    if node.widget.as_ref().widget_type() == "TreeView" {
                        if let Some(tree_view) = node.widget.as_ref().as_any().downcast_ref::<crate::widgets::TreeView>() {
                            tree_view.get_visible_nodes(self)
                        } else {
                            self.get_children(id).to_vec()
                        }
                    } else {
                        self.get_children(id).to_vec()
                    }
                };
                
                // Sort children by render layer (highest first: Overlay→Popup→Content→Background)
                // so that Popup layer widgets are hit-tested before Content layer widgets
                // covering the same screen area. This ensures menus and dialogs are clickable
                // even when content widgets overlap them.
                let mut sorted_children: Vec<WidgetId> = children;
                sorted_children.sort_by(|a, b| {
                    let layer_a = self.get_widget_layer(*a) as i32;
                    let layer_b = self.get_widget_layer(*b) as i32;
                    layer_b.cmp(&layer_a) // Higher layer first
                });
                
                // Skip invisible PopupMenu in hit_test
                for child in sorted_children.iter() {
                    let child_node = self.nodes.get(child);
                    if let Some(cn) = child_node {
                        if cn.widget.as_ref().widget_type() == "PopupMenu" {
                            if let Some(popup) = cn.widget.as_ref().as_any().downcast_ref::<crate::widgets::PopupMenu>() {
                                if !popup.is_visible() {
                                    continue; // Skip hidden popup menus
                                }
                            }
                        }
                    }
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

    /// Ensure all text from widgets is rasterized in the font atlas cache.
    /// This must be called BEFORE perform_layout/generate_render_data to guarantee
    /// all glyphs (including CJK characters not in the startup precache) are available.
    pub fn ensure_text_rasterized(&self, font_atlas: &mut FontAtlas) {
        let font_index = 0; // Default font index
        let sizes: [f32; 13] = [48.0, 36.0, 32.0, 28.0, 24.0, 22.0, 20.0, 18.0, 16.0, 15.0, 14.0, 13.0, 12.0];
        
        for node in self.nodes.values() {
            if let Some(text) = node.widget.as_ref().get_text() {
                font_atlas.prerasterize_chars(font_index, text, &sizes);
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
            "HStack" | "List" => {
                // HStack和Horizontal List使用相同的布局逻辑
                let is_horizontal = if widget_type == "List" {
                    self.nodes.get(&id)
                        .and_then(|n| n.widget.as_any().downcast_ref::<crate::widgets::List>())
                        .map(|l| l.orientation == crate::widgets::list::ListOrientation::Horizontal)
                        .unwrap_or(true)
                } else {
                    true
                };
                
                let (spacing, padding) = if widget_type == "HStack" {
                    self.nodes.get(&id)
                        .and_then(|n| {
                            if let Some(hstack) = n.widget.as_any().downcast_ref::<crate::widgets::HStack>() {
                                Some((hstack.spacing, hstack.padding))
                            } else {
                                None
                            }
                        })
                        .unwrap_or((8.0, crate::types::EdgeInsets::zero()))
                } else {
                    self.nodes.get(&id)
                        .and_then(|n| n.widget.as_any().downcast_ref::<crate::widgets::List>())
                        .map(|l| (l.spacing, crate::types::EdgeInsets::zero()))
                        .unwrap_or((0.0, crate::types::EdgeInsets::zero()))
                };

                if is_horizontal {
                    let mut total_width: f32 = 0.0;
                    let mut max_height: f32 = 0.0;
                    
                    for (i, (w, h)) in child_sizes.iter().enumerate() {
                        total_width += w;
                        max_height = max_height.max(*h);
                        if i < children.len() - 1 {
                            total_width += spacing;
                        }
                    }
                    
                    total_width += padding.left + padding.right;
                    max_height += padding.top + padding.bottom;
                    
                    (total_width, max_height)
                } else {
                    // Vertical List
                    let mut max_width: f32 = 0.0;
                    let mut total_height: f32 = 0.0;
                    
                    for (i, (w, h)) in child_sizes.iter().enumerate() {
                        max_width = max_width.max(*w);
                        total_height += h;
                        if i < children.len() - 1 {
                            total_height += spacing;
                        }
                    }
                    
                    max_width += padding.left + padding.right;
                    total_height += padding.top + padding.bottom;
                    
                    (max_width, total_height)
                }
            }
            "VStack" => {
                let mut max_width: f32 = 0.0;
                let mut total_height: f32 = 0.0;
                let (spacing, padding) = self
                    .nodes
                    .get(&id)
                    .and_then(|n| {
                        if let Some(vstack) =
                            n.widget.as_any().downcast_ref::<crate::widgets::VStack>()
                        {
                            Some((vstack.spacing, vstack.padding))
                        } else {
                            None
                        }
                    })
                    .unwrap_or((8.0, crate::types::EdgeInsets::zero()));

                for (i, (w, h)) in child_sizes.iter().enumerate() {
                    max_width = max_width.max(*w);
                    total_height += h;
                    if i < children.len() - 1 {
                        total_height += spacing;
                    }
                }
                
                max_width += padding.left + padding.right;
                total_height += padding.top + padding.bottom;

                (max_width, total_height)
            }
            "SplitView" => {
                // SplitView is self-measured: returns its own layout dimensions
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.widget.as_mut().measure(font_atlas)
                } else {
                    (0.0, 0.0)
                }
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
            // HStack and VStack auto-size from children every frame (children sizes can change)
            // Other widgets auto-size only on first frame (when width/height == 0)
            let auto_size_from_children = widget_type == "HStack" || widget_type == "VStack";
            if auto_size_from_children || current_layout.width == 0.0 || current_layout.height == 0.0 {
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
            "List" => {
                self.layout_list_children(id, &children, &child_sizes);
            }
            "TreeView" => {
                self.layout_tree_view_children(id, font_atlas);
            }
            "Dialog" => {
                self.layout_dialog_children(id);
            }
            "SplitView" => {
                self.layout_split_view_children(id);
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
        let (spacing, padding) = self
            .nodes
            .get(&parent_id)
            .and_then(|n| {
                if let Some(hstack) = n.widget.as_any().downcast_ref::<crate::widgets::HStack>() {
                    Some((hstack.spacing, hstack.padding))
                } else {
                    None
                }
            })
            .unwrap_or((8.0, crate::types::EdgeInsets::zero()));

        let parent_width = self.nodes.get(&parent_id)
            .map(|n| n.widget.layout().width)
            .unwrap_or(0.0);
        let parent_height = self.nodes.get(&parent_id)
            .map(|n| n.widget.layout().height)
            .unwrap_or(0.0);
        
        let content_width = parent_width - padding.left - padding.right;
        let content_height = parent_height - padding.top - padding.bottom;

        // Collect flex_expand flags
        let flex_flags: Vec<bool> = children.iter().map(|&child_id| {
            self.nodes.get(&child_id)
                .map(|n| n.widget.flags().flex_expand)
                .unwrap_or(false)
        }).collect();
        let cross_flags: Vec<bool> = children.iter().map(|&child_id| {
            self.nodes.get(&child_id)
                .map(|n| n.widget.flags().cross_axis_fill)
                .unwrap_or(false)
        }).collect();

        // Calculate fixed children total width + spacing
        let flex_count = flex_flags.iter().filter(|&f| *f).count();
        let mut fixed_total: f32 = 0.0;
        for (i, (w, _)) in child_sizes.iter().enumerate() {
            if !flex_flags[i] {
                fixed_total += *w;
                if i < children.len() - 1 {
                    fixed_total += spacing;
                }
            }
        }
        // flex children also need spacing between them and fixed children
        if flex_count > 0 && children.len() > 1 {
            // total spacing = (children.len() - 1) * spacing, already accounted in fixed_total for non-flex gaps
            // add spacing for flex children positions
            let total_spacing = (children.len() - 1) as f32 * spacing;
            let remaining = content_width - fixed_total - total_spacing;
            let flex_extra = if flex_count > 0 { remaining / flex_count as f32 } else { 0.0 };
            
            let mut current_x = padding.left;
            for (i, &child_id) in children.iter().enumerate() {
                let (w, h) = child_sizes[i];
                let child_width = if flex_flags[i] { w + flex_extra } else { w };
                let child_height = if cross_flags[i] { content_height } else { h };
                
                if let Some(node) = self.nodes.get_mut(&child_id) {
                    node.widget.set_layout(crate::layout::Layout::new(
                        current_x,
                        padding.top,
                        child_width,
                        child_height,
                    ));
                }
                
                current_x += child_width + spacing;
            }
        } else {
            // No flex children — use original logic
            let mut current_x = padding.left;
            for (i, &child_id) in children.iter().enumerate() {
                let (w, h) = child_sizes[i];
                let child_height = if cross_flags[i] { content_height } else { h };
                
                // Skip effectively hidden children (zero-size) — they don't occupy layout space
                if w == 0.0 && h == 0.0 {
                    if let Some(node) = self.nodes.get_mut(&child_id) {
                        node.widget.set_layout(crate::layout::Layout::new(
                            current_x,
                            padding.top,
                            0.0,
                            0.0,
                        ));
                    }
                    continue;
                }
                
                if let Some(node) = self.nodes.get_mut(&child_id) {
                    let child_layout = *node.widget.layout();
                    node.widget.set_layout(crate::layout::Layout::new(
                        current_x,
                        padding.top,
                        w.max(child_layout.width),
                        child_height,
                    ));
                }
                
                current_x += w + spacing;
            }
        }
    }

    fn layout_vstack_children(
        &mut self,
        parent_id: WidgetId,
        children: &[WidgetId],
        child_sizes: &[(f32, f32)],
    ) {
        let (spacing, padding) = self
            .nodes
            .get(&parent_id)
            .and_then(|n| {
                if let Some(vstack) = n.widget.as_any().downcast_ref::<crate::widgets::VStack>() {
                    Some((vstack.spacing, vstack.padding))
                } else {
                    None
                }
            })
            .unwrap_or((8.0, crate::types::EdgeInsets::zero()));
        
        let parent_width = self.nodes.get(&parent_id)
            .map(|n| n.widget.layout().width)
            .unwrap_or(0.0);
        let parent_height = self.nodes.get(&parent_id)
            .map(|n| n.widget.layout().height)
            .unwrap_or(0.0);
        
        let content_width = parent_width - padding.left - padding.right;
        let content_height = parent_height - padding.top - padding.bottom;

        // Collect flex_expand and cross_axis_fill flags
        let flex_flags: Vec<bool> = children.iter().map(|&child_id| {
            self.nodes.get(&child_id)
                .map(|n| n.widget.flags().flex_expand)
                .unwrap_or(false)
        }).collect();
        let cross_flags: Vec<bool> = children.iter().map(|&child_id| {
            self.nodes.get(&child_id)
                .map(|n| n.widget.flags().cross_axis_fill)
                .unwrap_or(false)
        }).collect();

        let flex_count = flex_flags.iter().filter(|&f| *f).count();
        
        if flex_count > 0 {
            // Calculate remaining space for flex_expand children
            let mut fixed_total: f32 = 0.0;
            for (i, (_, h)) in child_sizes.iter().enumerate() {
                if !flex_flags[i] {
                    fixed_total += *h;
                    if i < children.len() - 1 {
                        fixed_total += spacing;
                    }
                }
            }
            let total_spacing = (children.len() - 1) as f32 * spacing;
            let remaining = content_height - fixed_total - total_spacing;
            let flex_extra = remaining / flex_count as f32;
            
            let mut current_y = padding.top;
            for (i, &child_id) in children.iter().enumerate() {
                let (w, h) = child_sizes[i];
                let child_width = if cross_flags[i] { content_width } else { w };
                let child_height = if flex_flags[i] { h + flex_extra } else { h };
                
                if let Some(node) = self.nodes.get_mut(&child_id) {
                    node.widget.set_layout(crate::layout::Layout::new(
                        padding.left,
                        current_y,
                        child_width,
                        child_height,
                    ));
                }
                
                current_y += child_height + spacing;
            }
        } else {
            // No flex children — use original logic with cross_axis_fill support
            let mut current_y = padding.top;
            for (i, &child_id) in children.iter().enumerate() {
                let (w, h) = child_sizes[i];
                let child_width = if cross_flags[i] { content_width } else { w };
                
                if let Some(node) = self.nodes.get_mut(&child_id) {
                    let child_layout = *node.widget.layout();
                    node.widget.set_layout(crate::layout::Layout::new(
                        padding.left,
                        current_y,
                        child_width.max(child_layout.width),
                        h.max(child_layout.height),
                    ));
                }
                
                current_y += h + spacing;
            }
        }
    }

    fn layout_list_children(
        &mut self,
        parent_id: WidgetId,
        children: &[WidgetId],
        child_sizes: &[(f32, f32)],
    ) {
        let list_info = self
            .nodes
            .get(&parent_id)
            .and_then(|n| n.widget.as_any().downcast_ref::<crate::widgets::List>())
            .map(|l| (l.spacing, l.orientation))
            .unwrap_or((0.0, crate::widgets::list::ListOrientation::Horizontal));
        
        let spacing = list_info.0;
        let is_horizontal = list_info.1 == crate::widgets::list::ListOrientation::Horizontal;
        
        if is_horizontal {
            let mut current_x = 0.0;
            
            for (i, &child_id) in children.iter().enumerate() {
                let (w, h) = child_sizes[i];
                
                if let Some(node) = self.nodes.get_mut(&child_id) {
                    let child_layout = *node.widget.layout();
                    
                    node.widget.set_layout(crate::layout::Layout::new(
                        current_x,
                        child_layout.y,
                        w.max(child_layout.width),
                        h.max(child_layout.height),
                    ));
                }
                
                current_x += w + spacing;
            }
        } else {
            let mut current_y = 0.0;
            
            for (i, &child_id) in children.iter().enumerate() {
                let (w, h) = child_sizes[i];
                
                if let Some(node) = self.nodes.get_mut(&child_id) {
                    let child_layout = *node.widget.layout();
                    
                    node.widget.set_layout(crate::layout::Layout::new(
                        child_layout.x,
                        current_y,
                        w.max(child_layout.width),
                        h.max(child_layout.height),
                    ));
                }
                
                current_y += h + spacing;
            }
        }
    }

    fn layout_tree_view_children(&mut self, id: WidgetId, _font_atlas: &FontAtlas) {
        let tree_view_info = {
            if let Some(node) = self.nodes.get(&id) {
                if let Some(tree_view) = node.widget.as_any().downcast_ref::<crate::widgets::TreeView>() {
                    let root_nodes = tree_view.root_nodes().to_vec();
                    let node_height = tree_view.node_height();
                    let indent_width = tree_view.indent_width();
                    let tree_layout = *tree_view.layout();
                    (root_nodes, node_height, indent_width, tree_layout)
                } else {
                    return;
                }
            } else {
                return;
            }
        };
        
        let (root_nodes, node_height, indent_width, tree_layout) = tree_view_info;
        
        let mut y_offset = 0.0f32;
        for root_id in root_nodes {
            self.layout_tree_nodes_recursive(root_id, &mut y_offset, node_height, indent_width, tree_layout.width, id);
        }
        
        if let Some(node) = self.nodes.get_mut(&id) {
            if let Some(tree_view) = node.widget.as_any_mut().downcast_mut::<crate::widgets::TreeView>() {
                let current_layout = *tree_view.layout();
                tree_view.set_layout(crate::layout::Layout::new(
                    current_layout.x,
                    current_layout.y,
                    current_layout.width,
                    y_offset.max(current_layout.height),
                ));
            }
        }
    }

    fn layout_split_view_children(&mut self, id: WidgetId) {
        let split_info = {
            if let Some(node) = self.nodes.get(&id) {
                if let Some(split_view) = node.widget.as_any().downcast_ref::<crate::widgets::SplitView>() {
                    (split_view.split_ratio, split_view.divider_thickness, split_view.content_scale, split_view.orientation, *split_view.layout())
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        let (ratio, divider_thickness, content_scale, orientation, layout) = split_info;
        let thickness = divider_thickness * content_scale;
        let children = self.get_children(id).to_vec();

        if children.len() < 2 {
            return;
        }

        match orientation {
            crate::widgets::SplitOrientation::Horizontal => {
                let first_width = layout.width * ratio - thickness / 2.0;
                let second_width = layout.width * (1.0 - ratio) - thickness / 2.0;
                let second_x = layout.width * ratio + thickness / 2.0;

                if let Some(node) = self.nodes.get_mut(&children[0]) {
                    let child_layout = *node.widget.layout();
                    node.widget.set_layout(crate::layout::Layout::new(
                        0.0,
                        0.0,
                        first_width.max(child_layout.width),
                        layout.height,
                    ));
                }
                if let Some(node) = self.nodes.get_mut(&children[1]) {
                    let child_layout = *node.widget.layout();
                    node.widget.set_layout(crate::layout::Layout::new(
                        second_x,
                        0.0,
                        second_width.max(child_layout.width),
                        layout.height,
                    ));
                }
            }
            crate::widgets::SplitOrientation::Vertical => {
                let first_height = layout.height * ratio - thickness / 2.0;
                let second_height = layout.height * (1.0 - ratio) - thickness / 2.0;
                let second_y = layout.height * ratio + thickness / 2.0;

                if let Some(node) = self.nodes.get_mut(&children[0]) {
                    let child_layout = *node.widget.layout();
                    node.widget.set_layout(crate::layout::Layout::new(
                        0.0,
                        0.0,
                        layout.width,
                        first_height.max(child_layout.height),
                    ));
                }
                if let Some(node) = self.nodes.get_mut(&children[1]) {
                    let child_layout = *node.widget.layout();
                    node.widget.set_layout(crate::layout::Layout::new(
                        0.0,
                        second_y,
                        layout.width,
                        second_height.max(child_layout.height),
                    ));
                }
            }
        }
    }

    fn layout_dialog_children(&mut self, id: WidgetId) {
        // Get Dialog properties: content_id, title_bar_height, button_height, content_scale
        let dialog_info = {
            if let Some(node) = self.nodes.get(&id) {
                if let Some(dialog) = node.widget.as_any().downcast_ref::<crate::widgets::Dialog>() {
                    (dialog.content_id, dialog.title_bar_height, dialog.button_height, dialog.content_scale, *dialog.layout())
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        let (content_id, title_bar_height, button_height, content_scale, dialog_layout) = dialog_info;
        let scaled_title_height = title_bar_height * content_scale;
        let scaled_button_height = button_height * content_scale;
        // Button area has 10px padding above buttons
        let button_area_height = scaled_button_height + 10.0 * content_scale;

        // Position the content widget inside the content_rect area
        // Content rect: (0, scaled_title_height, dialog_width, dialog_height - scaled_title_height - button_area_height)
        if let Some(content_widget_id) = content_id {
            let content_y = scaled_title_height;
            let content_width = dialog_layout.width;
            let content_height = dialog_layout.height - scaled_title_height - button_area_height;

            if let Some(node) = self.nodes.get_mut(&content_widget_id) {
                node.widget.set_layout(crate::layout::Layout::new(
                    0.0,
                    content_y,
                    content_width,
                    content_height,
                ));
            }
        }
    }
    
    fn layout_tree_nodes_recursive(
        &mut self,
        node_id: WidgetId,
        y_offset: &mut f32,
        node_height: f32,
        indent_width: f32,
        container_width: f32,
        tree_view_id: WidgetId,
    ) {
        let (depth, is_expanded) = {
            if let Some(node) = self.nodes.get(&node_id) {
                if let Some(tree_node) = node.widget.as_any().downcast_ref::<crate::widgets::TreeNode>() {
                    (tree_node.depth(), tree_node.is_expanded())
                } else {
                    return;
                }
            } else {
                return;
            }
        };
        
        let child_ids: Vec<WidgetId> = {
            if let Some(node) = self.nodes.get(&tree_view_id) {
                if let Some(tree_view) = node.widget.as_any().downcast_ref::<crate::widgets::TreeView>() {
                    tree_view.get_node_children(node_id).to_vec()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };
        
        let x = depth as f32 * indent_width;
        let width = container_width - x;
        
        if let Some(node) = self.nodes.get_mut(&node_id) {
            if let Some(tree_node) = node.widget.as_any_mut().downcast_mut::<crate::widgets::TreeNode>() {
                tree_node.set_layout(crate::layout::Layout::new(x, *y_offset, width, node_height));
            }
        }
        
        *y_offset += node_height;
        
        if is_expanded {
            for child_id in child_ids {
                self.layout_tree_nodes_recursive(child_id, y_offset, node_height, indent_width, container_width, tree_view_id);
            }
        }
    }

    pub fn generate_render_data(&mut self, font_atlas: &FontAtlas) -> Vec<RenderData> {
        let mut render_data = Vec::new();
        
        if let Some(root_id) = self.root {
            self.generate_render_data_recursive(root_id, 0.0, 0.0, &mut render_data, font_atlas);
        }
        
        render_data.sort_by_key(|r| r.layer);
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
            // Skip hidden Dialog and its children entirely
            if node.widget.as_ref().widget_type() == "Dialog" {
                if let Some(dialog) = node.widget.as_ref().as_any().downcast_ref::<crate::widgets::Dialog>() {
                    if !dialog.is_visible() {
                        return;
                    }
                }
            }
            
            if node.widget.as_ref().state() != WidgetState::Disabled {
                let layout = *node.widget.as_ref().layout();
                let abs_x = parent_abs_x + layout.x;
                let abs_y = parent_abs_y + layout.y;
                let layer = node.layer;
                
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
                    layer,
                    widget_id: id.id,
                });

                node.render_data = Some(render_data.last().unwrap().clone());
            }
        }

        let layout = self.nodes.get(&id).map(|n| *n.widget.layout()).unwrap_or_default();
        let abs_x = parent_abs_x + layout.x;
        let abs_y = parent_abs_y + layout.y;
        
        // For TreeView widgets, only render visible (expanded) nodes
        let children: Vec<WidgetId> = {
            let node = self.nodes.get(&id);
            if let Some(n) = node {
                if n.widget.as_ref().widget_type() == "TreeView" {
                    if let Some(tree_view) = n.widget.as_ref().as_any().downcast_ref::<crate::widgets::TreeView>() {
                        tree_view.get_visible_nodes(self)
                    } else {
                        self.get_children(id).to_vec()
                    }
                } else {
                    self.get_children(id).to_vec()
                }
            } else {
                Vec::new()
            }
        };
        
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
            DrawCommand::Text { bounds, width, height, text, font_size, font_color, alignment } => {
                DrawCommand::Text {
                    bounds: Point::new(bounds.x + offset_x, bounds.y + offset_y),
                    width: *width,
                    height: *height,
                    text: text.clone(),
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
            DrawCommand::RectOutline { bounds, width, height, color, stroke_width } => {
                DrawCommand::RectOutline {
                    bounds: Point::new(bounds.x + offset_x, bounds.y + offset_y),
                    width: *width,
                    height: *height,
                    color: *color,
                    stroke_width: *stroke_width,
                }
            }
        }
    }
    
    pub fn debug_print_tree(&self) {
        if let Some(root) = self.root {
            self.print_node(root, 0);
        }
    }

    fn print_node(&self, id: WidgetId, depth: usize) {
        let indent = "  ".repeat(depth);
        if let Some(node) = self.nodes.get(&id) {
            let layout = node.widget.layout();
            let widget_type = node.widget.widget_type();
            let layer = node.layer;
            dfx_info!("UITree", "{}[{}] id={} pos=({:.0},{:.0}) size=({:.0},{:.0}) layer={}", 
                indent, widget_type, id.id, layout.x, layout.y, layout.width, layout.height, layer as i32);
            
            if let Some(children) = self.children_map.get(&id) {
                for &child_id in children {
                    self.print_node(child_id, depth + 1);
                }
            }
        } else {
            dfx_info!("UITree", "{}[UNKNOWN] id={}", indent, id.id);
        }
    }

    pub fn root(&self) -> Option<WidgetId> {
        self.root
    }

    pub fn dump_node_to_string(&self, id: WidgetId, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);
        if let Some(node) = self.nodes.get(&id) {
            let layout = node.widget.layout();
            let widget_type = node.widget.widget_type();
            let layer = node.layer;
            output.push_str(&format!(
                "{}[{}] id={} pos=({:.0},{:.0}) size=({:.0},{:.0}) layer={}\n",
                indent, widget_type, id.id, layout.x, layout.y, layout.width, layout.height, layer as i32
            ));
            if let Some(children) = self.children_map.get(&id) {
                for &child_id in children {
                    self.dump_node_to_string(child_id, depth + 1, output);
                }
            }
        }
    }
}

impl Default for WidgetTree {
    fn default() -> Self {
        Self::new()
    }
}
