use crate::{Widget, WidgetId, Layout, Style, Color, Canvas, Rect, Event, EventResult, DrawCommand, WidgetState, EventType, WidgetFlags};

pub struct PreviewWindow {
    id: WidgetId,
    parent: Option<WidgetId>,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    texture_id: u64,
    selected: bool,
    edit_mode: bool,  // true=Editing(橙色边框), false=Running(蓝色边框)
    flags: WidgetFlags,
}

impl PreviewWindow {
    pub fn new(texture_id: u64) -> Self {
        Self {
            id: WidgetId::new(),
            parent: None,
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 0.0, 0.0),
            style: Style::default(),
            state: WidgetState::Normal,
            texture_id,
            selected: false,
            edit_mode: true,  // 默认Editing模式
            flags: WidgetFlags::default(),
        }
    }
    
    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }
    
    pub fn id(&self) -> WidgetId {
        self.id
    }
    
    pub fn set_texture_id(&mut self, texture_id: u64) {
        self.texture_id = texture_id;
    }
    
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
    
    pub fn is_selected(&self) -> bool {
        self.selected
    }
    
    pub fn set_edit_mode(&mut self, edit_mode: bool) {
        self.edit_mode = edit_mode;
    }
}

impl Widget for PreviewWindow {
    fn id(&self) -> WidgetId {
        self.id
    }
    
    fn parent(&self) -> Option<WidgetId> {
        self.parent
    }
    
    fn set_parent(&mut self, parent: WidgetId) {
        self.parent = Some(parent);
    }
    
    fn children(&self) -> &[WidgetId] {
        &self.children
    }
    
    fn add_child(&mut self, child: WidgetId) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }
    
    fn remove_child(&mut self, child: WidgetId) {
        self.children.retain(|c| *c != child);
    }
    
    fn layout(&self) -> &Layout {
        &self.layout
    }
    
    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }
    
    fn style(&self) -> &Style {
        &self.style
    }
    
    fn set_style(&mut self, style: Style) {
        self.style = style;
    }
    
    fn state(&self) -> WidgetState {
        self.state
    }
    
    fn set_state(&mut self, state: WidgetState) {
        self.state = state;
    }
    
    fn widget_type(&self) -> &'static str {
        "PreviewWindow"
    }
    
    fn draw(&mut self, canvas: &mut Canvas) {
        let bounds = Rect::new(0.0, 0.0, self.layout.width, self.layout.height);
        let uv = Rect::new(0.0, 0.0, 1.0, 1.0);
        canvas.draw_image(bounds, self.texture_id, uv);
        
        if self.selected {
            // Editing模式：橙色边框 (1.0, 0.6, 0.3)
            // Running模式：蓝色边框 (0.2, 0.6, 1.0)
            let border_color = if self.edit_mode {
                Color::new(1.0, 0.6, 0.3, 1.0)  // 淡橙色
            } else {
                Color::new(0.2, 0.6, 1.0, 1.0)  // 蓝色
            };
            
            let border_style = Style::new()
                .with_background(Color::transparent())
                .with_border(border_color, 1.0, 0.0);
            canvas.draw_rect(bounds, &border_style);
        }
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        if event.target.id == self.id.id && event.event_type == EventType::TouchBegin {
            self.selected = true;
            // 点击预览窗时清除InputField焦点 — 互斥关系
            crate::thunk::ui_clear_focused_input_field();
            return EventResult::Handled;
        }
        EventResult::Ignored
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
}

impl Default for PreviewWindow {
    fn default() -> Self {
        Self::new(1)
    }
}