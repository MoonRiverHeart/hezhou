use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;

pub struct ListItem {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: WidgetFlags,
    text: String,
    text_style: TextStyle,
    border_color: Color,
    show_border: bool,
}

impl ListItem {
    pub fn new(text: &str) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 100.0, 30.0),
            style: Style::new().with_background(Color::transparent()),
            state: WidgetState::Normal,
            flags: WidgetFlags::default(),
            text: text.to_string(),
            text_style: TextStyle::new().with_size(14.0).with_color(Color::white()),
            border_color: Color::new(0.3, 0.3, 0.3, 0.3),
            show_border: false,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.flags.dirty_render = true;
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn set_border_color(&mut self, color: Color) {
        self.border_color = color;
        self.flags.dirty_render = true;
    }

    pub fn set_show_border(&mut self, show: bool) {
        self.show_border = show;
        self.flags.dirty_render = true;
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.text_style.font_size = size;
        self.flags.dirty_render = true;
    }
}

impl Widget for ListItem {
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
        "ListItem"
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

        if self.style.background_color.a > 0.0 {
            canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        }

        if self.show_border {
            let is_horizontal = width > height * 2.0;
            
            if is_horizontal {
                let line_width = 1.0;
                canvas.draw_rect(
                    Rect::new(width - line_width, 0.0, line_width, height),
                    &Style::new().with_background(self.border_color),
                );
            } else {
                let line_height = 1.0;
                canvas.draw_rect(
                    Rect::new(0.0, height - line_height, width, line_height),
                    &Style::new().with_background(self.border_color),
                );
            }
        }

        let text_style = TextStyle::new()
            .with_size(self.text_style.font_size)
            .with_color(self.text_style.font_color)
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Left,
                vertical: VerticalAlignment::Center,
            });
        
        let padding = 8.0;
        canvas.draw_text(
            Rect::new(padding, 0.0, width - padding * 2.0, height),
            &self.text,
            &text_style,
        );
    }

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let (text_width, text_height) =
            font_atlas.measure_text(0, &self.text, self.text_style.font_size);

        let width = if self.layout.width > 0.0 {
            self.layout.width
        } else {
            text_width + 16.0
        };

        let height = if self.layout.height > 0.0 {
            self.layout.height
        } else {
            text_height + 4.0
        };

        (width, height)
    }

    fn on_event(&mut self, _event: &Event) -> EventResult {
        EventResult::Ignored
    }
}