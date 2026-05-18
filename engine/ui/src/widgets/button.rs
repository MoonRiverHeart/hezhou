use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use std::sync::Mutex;

pub struct Button {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    text: String,
    text_style: TextStyle,
    on_click: Option<Box<dyn FnMut() + Send + Sync>>,
}

impl Button {
    pub fn new(text: &str) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 100.0, 40.0),
            style: Style::new()
                .with_background(Color::new(0.2, 0.6, 1.0, 1.0))
                .with_border(Color::transparent(), 0.0, 4.0),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            text: text.to_string(),
            text_style: TextStyle::new()
                .with_size(16.0)
                .with_color(Color::white())
                .with_alignment(TextAlignment {
                    horizontal: crate::types::HorizontalAlignment::Center,
                    vertical: crate::types::VerticalAlignment::Center,
                }),
            on_click: None,
        }
    }

    pub fn set_on_click(&mut self, callback: Box<dyn FnMut() + Send + Sync>) {
        self.on_click = Some(callback);
    }
    
    pub fn set_font_size(&mut self, size: f32) {
        self.text_style.font_size = size;
        self.flags.dirty_render = true;
    }
    
    pub fn with_on_click(mut self, callback: Box<dyn FnMut() + Send + Sync>) -> Self {
        self.on_click = Some(callback);
        self
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.flags.dirty_render = true;
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
        self.flags.dirty_style = true;
        self.flags.dirty_render = true;
    }
    
    pub fn trigger_click(&mut self) {
        if let Some(callback) = &mut self.on_click {
            callback();
        }
    }
}

impl Widget for Button {
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

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let (text_width, text_height) =
            font_atlas.measure_text(0, &self.text, self.text_style.font_size);

        let padding = 15.0;
        let width = text_width + padding * 2.0;
        let height = text_height + padding * 2.0;

        (width, height)
    }

    fn widget_type(&self) -> &'static str {
        "Button"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        let padding = 15.0;

        let current_style = match self.state {
            WidgetState::Hovered => Style::new()
                .with_background(Color::new(0.25, 0.65, 1.0, 1.0))
                .with_border(Color::transparent(), 0.0, 4.0),
            WidgetState::Pressed => Style::new()
                .with_background(Color::new(0.15, 0.55, 0.95, 1.0))
                .with_border(Color::transparent(), 0.0, 4.0),
            WidgetState::Disabled => Style::new()
                .with_background(Color::new(0.5, 0.5, 0.5, 0.3))
                .with_border(Color::transparent(), 0.0, 4.0),
            _ => self.style,
        };

        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &current_style);

        canvas.draw_text(Rect::new(padding, padding, width - padding * 2.0, height - padding * 2.0), &self.text, &self.text_style);
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                if self.state != WidgetState::Disabled {
                    self.set_state(WidgetState::Pressed);
                    return EventResult::Handled;
                }
            }

            EventType::TouchEnd => {
                if self.state == WidgetState::Pressed {
                    self.set_state(WidgetState::Normal);
                    if let Some(callback) = &mut self.on_click {
                        callback();
                    }
                    return EventResult::Stopped;
                }
            }

            EventType::MouseEnter => {
                if self.state != WidgetState::Disabled {
                    self.set_state(WidgetState::Hovered);
                    return EventResult::Handled;
                }
            }

            EventType::MouseLeave => {
                if self.state == WidgetState::Hovered || self.state == WidgetState::Pressed {
                    self.set_state(WidgetState::Normal);
                    return EventResult::Handled;
                }
            }

            _ => {}
        }

        EventResult::Ignored
    }
}
