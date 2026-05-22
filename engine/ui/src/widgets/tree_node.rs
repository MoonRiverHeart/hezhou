use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::thunk::{queue_callback, PendingCallback};

pub struct TreeNode {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: WidgetFlags,
    text: String,
    depth: usize,
    is_expanded: bool,
    has_children: bool,
    is_selected: bool,
    node_height: f32,
    indent_width: f32,
    user_data: u64,
    content_scale: f32,
    hover_color: Color,
    selected_color: Color,
    text_color: Color,
    icon_color: Color,
    font_size: f32,
}

impl TreeNode {
    pub fn new(text: &str) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 100.0, 24.0),
            style: Style::new().with_background(Color::transparent()),
            state: WidgetState::Normal,
            flags: WidgetFlags::default(),
            text: text.to_string(),
            depth: 0,
            is_expanded: false,
            has_children: false,
            is_selected: false,
            node_height: 24.0,
            indent_width: 20.0,
            user_data: 0,
            content_scale: 1.0,
            hover_color: Color::new(0.22, 0.26, 0.30, 1.0),
            selected_color: Color::new(0.18, 0.22, 0.28, 1.0),
            text_color: Color::white(),
            icon_color: Color::white(),
            font_size: 14.0,
        }
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_has_children(mut self, has_children: bool) -> Self {
        self.has_children = has_children;
        self
    }

    pub fn with_user_data(mut self, user_data: u64) -> Self {
        self.user_data = user_data;
        self
    }

    pub fn with_content_scale(mut self, scale: f32) -> Self {
        self.content_scale = scale;
        self.node_height = 24.0 * scale;
        self.indent_width = 20.0 * scale;
        self.font_size = 14.0 * scale;
        self
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.flags.dirty_render = true;
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn set_depth(&mut self, depth: usize) {
        self.depth = depth;
        self.flags.dirty_render = true;
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.is_expanded = expanded;
        self.flags.dirty_render = true;
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }

    pub fn set_has_children(&mut self, has_children: bool) {
        self.has_children = has_children;
        self.flags.dirty_render = true;
    }

    pub fn has_children(&self) -> bool {
        self.has_children
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
        self.flags.dirty_render = true;
    }

    pub fn is_selected(&self) -> bool {
        self.is_selected
    }

    pub fn set_user_data(&mut self, user_data: u64) {
        self.user_data = user_data;
    }

    pub fn user_data(&self) -> u64 {
        self.user_data
    }

    pub fn set_node_height(&mut self, height: f32) {
        self.node_height = height;
        self.layout.height = height;
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn node_height(&self) -> f32 {
        self.node_height
    }

    pub fn set_indent_width(&mut self, width: f32) {
        self.indent_width = width;
        self.flags.dirty_render = true;
    }

    pub fn indent_width(&self) -> f32 {
        self.indent_width
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.node_height = 24.0 * scale;
        self.indent_width = 20.0 * scale;
        self.font_size = 14.0 * scale;
        self.layout.height = self.node_height;
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    pub fn toggle(&mut self) {
        if self.has_children {
            self.is_expanded = !self.is_expanded;
            self.flags.dirty_render = true;
            queue_callback(PendingCallback::TreeNodeToggle { widget_id: self.id.id });
        }
    }

    pub fn select(&mut self) {
        self.is_selected = true;
        self.flags.dirty_render = true;
        queue_callback(PendingCallback::TreeNodeSelect { widget_id: self.id.id, user_data: self.user_data });
    }

    fn draw_expand_icon(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32) {
        let icon_style = Style::new().with_background(self.icon_color);
        
        if self.is_expanded {
            // [-] 折叠图标: 水平线
            let line_y = y + size / 2.0;
            canvas.draw_rect(
                Rect::new(x, line_y - 1.0, size, 2.0),
                &icon_style,
            );
        } else {
            // [+] 展开图标: 水平线 + 垂直线
            let center_x = x + size / 2.0;
            let center_y = y + size / 2.0;
            canvas.draw_rect(
                Rect::new(x, center_y - 1.0, size, 2.0),
                &icon_style,
            );
            canvas.draw_rect(
                Rect::new(center_x - 1.0, y, 2.0, size),
                &icon_style,
            );
        }
    }
}

impl Widget for TreeNode {
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
    }

    fn remove_child(&mut self, child: WidgetId) {
        self.children.retain(|&id| id != child);
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
        "TreeNode"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn flags(&self) -> WidgetFlags {
        self.flags
    }

    fn set_flags(&mut self, flags: WidgetFlags) {
        self.flags = flags;
    }

    fn get_text(&self) -> Option<&str> {
        Some(&self.text)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        // Layout x already accounts for depth indent, so draw at local origin
        let icon_size = self.indent_width * 0.6;

        // 绘制背景: hover=淡蓝, pressed=蓝, selected=淡蓝, normal=透明
        let bg_color = if self.state == WidgetState::Pressed {
            Color::new(0.15, 0.35, 0.55, 1.0)  // 蓝色(点击时)
        } else if self.is_selected {
            self.selected_color  // 淡蓝(选中)
        } else if self.state == WidgetState::Hovered {
            self.hover_color  // 淡蓝(hover)
        } else {
            Color::transparent()
        };

        if bg_color.a > 0.0 {
            let bg_style = Style::new().with_background(bg_color);
            canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &bg_style);
        }

        // 绘制展开/折叠图标
        if self.has_children {
            let icon_x = (self.indent_width - icon_size) / 2.0;
            let icon_y = (height - icon_size) / 2.0;
            self.draw_expand_icon(canvas, icon_x, icon_y, icon_size);
        }

        // 绘制文本
        let text_x = self.indent_width + 4.0 * self.content_scale;
        let text_style = TextStyle::new()
            .with_size(self.font_size)
            .with_color(self.text_color)
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Left,
                vertical: VerticalAlignment::Center,
            });

        canvas.draw_text(
            Rect::new(text_x, 0.0, width - text_x, height),
            &self.text,
            &text_style,
        );
    }

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let (text_width, text_height) = font_atlas.measure_text(0, &self.text, self.font_size);

        let indent = self.depth as f32 * self.indent_width;
        let width = indent + self.indent_width + text_width + 8.0 * self.content_scale;
        let height = self.node_height;

        (width, height)
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                match &event.data {
                    EventData::Touch(touch) => {
                        // Layout x already accounts for depth indent, so touch coordinates
                        // are relative to the node's own origin (no double-indent)
                        
                        // 点击展开图标区域
                        if touch.x >= 0.0 && touch.x < self.indent_width && self.has_children {
                            self.set_state(WidgetState::Pressed);
                            self.toggle();
                            return EventResult::Handled;
                        }
                        
                        // 点击文本区域
                        if touch.x >= self.indent_width {
                            self.set_state(WidgetState::Pressed);
                            self.select();
                            return EventResult::Handled;
                        }
                    }
                    _ => {}
                }
            }

            EventType::TouchEnd => {
                if self.state == WidgetState::Pressed {
                    self.set_state(WidgetState::Hovered);
                    return EventResult::Handled;
                }
            }

            EventType::MouseEnter => {
                if self.state != WidgetState::Pressed {
                    self.set_state(WidgetState::Hovered);
                }
                return EventResult::Handled;
            }

            EventType::MouseLeave => {
                self.set_state(WidgetState::Normal);
                return EventResult::Handled;
            }

            _ => {}
        }

        EventResult::Ignored
    }
}