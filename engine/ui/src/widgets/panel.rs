use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;

pub struct Panel {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    layout_type: crate::layout::LayoutType,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 300.0, 200.0),
            style: Style::new()
                .with_background(Color::new(1.0, 1.0, 1.0, 1.0))
                .with_border(Color::new(0.8, 0.8, 0.8, 1.0), 1.0, 8.0),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            layout_type: crate::layout::LayoutType::Absolute,
        }
    }

    pub fn with_layout_type(mut self, layout_type: crate::layout::LayoutType) -> Self {
        self.layout_type = layout_type;
        self
    }

    pub fn add_child_widget(&mut self, child: Box<dyn Widget>) {
        let child_id = child.id();
        self.children.push(child_id);
    }
}

impl Widget for Panel {
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
        self.children.retain(|c| *c != child);
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
        "Panel"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let bounds = self.layout.bounds();

        if self.style.shadow.is_some() {
            canvas.draw_shadow(bounds, self.style.shadow.as_ref().unwrap());
        }

        canvas.draw_rect(bounds, &self.style);
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        EventResult::Ignored
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}
