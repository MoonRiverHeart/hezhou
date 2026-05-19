use crate::{Widget, WidgetId, Layout, Style, Color, Canvas, Rect, Event, EventResult, DrawCommand, WidgetState};

pub struct PreviewWindow {
    id: WidgetId,
    parent: Option<WidgetId>,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    texture_id: u64,
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
    }
    
    fn on_event(&mut self, _event: &Event) -> EventResult {
        EventResult::Ignored
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for PreviewWindow {
    fn default() -> Self {
        Self::new(1)
    }
}