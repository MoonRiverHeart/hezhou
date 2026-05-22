use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::thunk::{queue_callback, PendingCallback};

pub struct Checkbox {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    checked: bool,
    text: String,
    text_style: TextStyle,
    on_change: Option<Box<dyn FnMut(bool) + Send + Sync>>,
    content_scale: f32,
}

impl Checkbox {
    pub fn new(text: &str) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 100.0, 24.0),
            style: Style::new()
                .with_background(Color::new(0.18, 0.18, 0.22, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 2.0, 0.0),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            checked: false,
            text: text.to_string(),
            text_style: TextStyle::new()
                .with_size(16.0)
                .with_color(Color::white())
                .with_alignment(TextAlignment {
                    horizontal: crate::types::HorizontalAlignment::Left,
                    vertical: crate::types::VerticalAlignment::Center,
                }),
            on_change: None,
            content_scale: 1.0,
        }
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
        self.flags.dirty_render = true;
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn set_on_change(&mut self, callback: Box<dyn FnMut(bool) + Send + Sync>) {
        self.on_change = Some(callback);
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.flags.dirty_render = true;
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
        self.flags.dirty_render = true;
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.text_style.font_size = 16.0 * scale;
        self.flags.dirty_render = true;
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.text_style.font_size = size;
        self.flags.dirty_render = true;
    }
}

impl Widget for Checkbox {
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

        let checkbox_size = 20.0 * self.content_scale;
        let gap = 8.0 * self.content_scale;
        let width = text_width + checkbox_size + gap;
        let height = text_height.max(checkbox_size);

        (width, height)
    }

    fn widget_type(&self) -> &'static str {
        "Checkbox"
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
        let content_scale = self.content_scale;
        let checkbox_size = 20.0 * content_scale;
        let gap = 8.0 * content_scale;
        let padding_y = (height - checkbox_size) / 2.0;

        // Background panel with hover/pressed state
        let bg_color = if self.checked {
            match self.state {
                WidgetState::Hovered => Color::new(0.20, 0.50, 0.20, 1.0),
                WidgetState::Pressed => Color::new(0.15, 0.45, 0.15, 1.0),
                _ => Color::new(0.15, 0.45, 0.15, 1.0),
            }
        } else {
            match self.state {
                WidgetState::Hovered => Color::new(0.23, 0.23, 0.27, 1.0),
                WidgetState::Pressed => Color::new(0.18, 0.18, 0.22, 1.0),
                _ => Color::new(0.18, 0.18, 0.22, 1.0),
            }
        };

        canvas.draw_rect(
            Rect::new(0.0, 0.0, width, height),
            &Style::new()
                .with_background(bg_color)
                .with_border(Color::transparent(), 0.0, 2.0),
        );

        // Checkbox square
        let cb_x = 4.0 * content_scale;
        let cb_y = padding_y;

        if self.checked {
            // Filled square with green background
            canvas.draw_rect(
                Rect::new(cb_x, cb_y, checkbox_size, checkbox_size),
                &Style::new()
                    .with_background(Color::new(0.15, 0.45, 0.15, 1.0))
                    .with_border(Color::new(0.2, 0.9, 0.2, 1.0), 2.0, 2.0),
            );
            // Checkmark symbol ✓
            canvas.draw_text(
                Rect::new(cb_x, cb_y, checkbox_size, checkbox_size),
                "\u{2713}",
                &TextStyle::new()
                    .with_size(14.0 * content_scale)
                    .with_color(Color::new(0.2, 0.9, 0.2, 1.0))
                    .with_alignment(TextAlignment {
                        horizontal: crate::types::HorizontalAlignment::Center,
                        vertical: crate::types::VerticalAlignment::Center,
                    }),
            );
        } else {
            // Empty square with border
            canvas.draw_rect(
                Rect::new(cb_x, cb_y, checkbox_size, checkbox_size),
                &Style::new()
                    .with_background(Color::new(0.12, 0.12, 0.15, 1.0))
                    .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 2.0, 2.0),
            );
        }

        // Text label to the right of checkbox
        let text_x = cb_x + checkbox_size + gap;
        let text_width_available = width - text_x;
        canvas.draw_text(
            Rect::new(text_x, 0.0, text_width_available, height),
            &self.text,
            &self.text_style,
        );
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
                    self.toggle();
                    let widget_id = self.id.id;
                    let checked = self.checked;
                    // Queue thunk callback — NEVER trigger inside lock
                    queue_callback(PendingCallback::CheckboxChange { widget_id, checked });
                    // Also call direct on_change callback
                    if let Some(callback) = &mut self.on_change {
                        callback(self.checked);
                    }
                    self.set_state(WidgetState::Normal);
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