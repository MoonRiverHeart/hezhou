use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;

/// Label换行模式
/// None=0: 单行，原有行为
/// Wrap=1: 自动换行，接入TextLayoutModel
/// Truncate=2: 截断+省略号
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapMode {
    None = 0,
    Wrap = 1,
    Truncate = 2,
}

impl Default for WrapMode {
    fn default() -> Self {
        WrapMode::None
    }
}

impl From<u32> for WrapMode {
    fn from(value: u32) -> Self {
        match value {
            1 => WrapMode::Wrap,
            2 => WrapMode::Truncate,
            _ => WrapMode::None,
        }
    }
}

pub struct Label {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    text: String,
    text_style: TextStyle,
    /// 换行模式: None(单行), Wrap(自动换行), Truncate(截断)
    wrap_mode: WrapMode,
}

impl Label {
    pub fn new(text: &str) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 200.0, 30.0),
            style: Style::new(),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            text: text.to_string(),
            text_style: TextStyle::new().with_size(16.0).with_color(Color::white()),
            wrap_mode: WrapMode::None,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.flags.dirty_render = true;
    }

    pub fn set_text_style(&mut self, style: TextStyle) {
        self.text_style = style;
        self.flags.dirty_render = true;
    }
    
    pub fn set_font_size(&mut self, size: f32) {
        self.text_style.font_size = size;
        self.flags.dirty_render = true;
    }

    /// 设置换行模式
    pub fn set_wrap_mode(&mut self, mode: WrapMode) {
        self.wrap_mode = mode;
        self.flags.dirty_render = true;
        self.flags.dirty_layout = true;
    }

    /// 获取换行模式
    pub fn get_wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }

    /// 获取字体大小(用于外部measure_text计算)
    pub fn get_font_size(&self) -> f32 {
        self.text_style.font_size
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }
}

impl Widget for Label {
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
        "Label"
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

        let text_style = TextStyle::new()
            .with_size(self.text_style.font_size)
            .with_color(self.text_style.font_color)
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Left,
                vertical: VerticalAlignment::Center,
            });

        canvas.draw_text(Rect::new(0.0, 0.0, width, height), &self.text, &text_style);
    }

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let (text_width, text_height) =
            font_atlas.measure_text(0, &self.text, self.text_style.font_size);

        let width = if self.layout.width > 0.0 {
            self.layout.width.max(text_width)
        } else {
            text_width
        };
        
        let height = if self.layout.height > 0.0 {
            self.layout.height.max(text_height)
        } else {
            text_height
        };

        (width, height)
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                self.set_state(WidgetState::Pressed);
                return EventResult::Handled;
            }
            EventType::TouchEnd => {
                if self.state == WidgetState::Pressed {
                    self.set_state(WidgetState::Normal);
                    return EventResult::Handled;
                }
            }
            EventType::MouseEnter => {
                self.set_state(WidgetState::Hovered);
                return EventResult::Handled;
            }
            EventType::MouseLeave => {
                if self.state != WidgetState::Pressed {
                    self.set_state(WidgetState::Normal);
                }
                return EventResult::Handled;
            }
            _ => {}
        }
        EventResult::Ignored
    }
}
