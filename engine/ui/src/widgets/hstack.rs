use crate::canvas::*;
use crate::event::*;
use crate::font_atlas::FontAtlas;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use std::sync::Mutex;

pub struct HStack {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: WidgetFlags,
    pub spacing: f32,
    pub padding: EdgeInsets,
}

impl HStack {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::zero(),
            style: Style::new().with_background(Color::transparent()),
            state: WidgetState::Normal,
            flags: WidgetFlags::default(),
            spacing: 8.0,
            padding: EdgeInsets::zero(),
        }
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }
}

impl Widget for HStack {
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
        "HStack"
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
        let bounds = self.layout.bounds();
        canvas.draw_rect(bounds, &self.style);
    }

    fn on_event(&mut self, _event: &Event) -> EventResult {
        EventResult::Ignored
    }
}
