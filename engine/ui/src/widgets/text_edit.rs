use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use hezhou_platform::KeyCode;
use parking_lot::Mutex;
use std::sync::LazyLock;

static CLIPBOARD: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));

pub struct TextEdit {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    text: String,
    text_style: TextStyle,
    cursor_position: usize,
    cursor_visible: bool,
    selection_start: usize,
    selection_end: usize,
    focused: bool,
}

impl TextEdit {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 400.0, 300.0),
            style: Style::new()
                .with_background(Color::new(0.15, 0.15, 0.15, 1.0))
                .with_border(Color::new(0.3, 0.3, 0.3, 1.0), 1.0, 0.0),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            text: String::new(),
            text_style: TextStyle::new().with_size(16.0).with_color(Color::new(0.9, 0.9, 0.9, 1.0)),
            cursor_position: 0,
            cursor_visible: true,
            selection_start: 0,
            selection_end: 0,
            focused: false,
        }
    }
    
    pub fn with_size(width: f32, height: f32) -> Self {
        Self {
            layout: Layout::new(0.0, 0.0, width, height),
            ..Self::new()
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor_position = self.text.len();
        self.flags.dirty_render = true;
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }
    
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
        self.flags.dirty_render = true;
    }
    
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let delete_pos = self.cursor_position - 1;
            if delete_pos < self.text.len() {
                self.text.remove(delete_pos);
                self.cursor_position = delete_pos;
                self.flags.dirty_render = true;
            }
        }
    }
    
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.flags.dirty_render = true;
    }
}

impl Widget for TextEdit {
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
    }

    fn style(&self) -> &Style {
        &self.style
    }
    
    fn set_style(&mut self, style: Style) {
        self.style = style;
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
        "TextEdit"
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
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        if !self.text.is_empty() {
            canvas.draw_text(
                Rect::new(10.0, 10.0, width - 20.0, height - 20.0),
                &self.text,
                &self.text_style,
            );
        }
        
        if self.focused && self.cursor_visible {
            let cursor_x = 10.0 + self.cursor_position as f32 * 8.0;
            canvas.draw_rect(
                Rect::new(cursor_x, 10.0, 2.0, height - 20.0),
                &Style::new().with_background(Color::white()),
            );
        }
    }

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let (text_width, text_height) =
            font_atlas.measure_text(0, &self.text, self.text_style.font_size * 2.0);

        let width = if self.layout.width > 0.0 {
            self.layout.width.max(text_width + 20.0)
        } else {
            text_width + 20.0
        };
        
        let height = if self.layout.height > 0.0 {
            self.layout.height.max(text_height + 20.0)
        } else {
            text_height + 20.0
        };

        (width, height)
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                self.focused = true;
                self.flags.dirty_render = true;
                return EventResult::Handled;
            }
            EventType::KeyDown => {
                if self.focused {
                    if let EventData::Key(key_data) = &event.data {
                        let ctrl_pressed = key_data.modifiers & 2 != 0;
                        
                        if ctrl_pressed {
                            // Ctrl+C: 复制全部文本
                            if key_data.keycode == KeyCode::C as u32 {
                                let mut clipboard = CLIPBOARD.lock();
                                *clipboard = self.text.clone();
                                println!("[TextEdit] Ctrl+C: copied {} chars", self.text.len());
                                return EventResult::Handled;
                            }
                            // Ctrl+V: 粘贴clipboard
                            if key_data.keycode == KeyCode::V as u32 {
                                let clipboard = CLIPBOARD.lock();
                                for c in clipboard.chars() {
                                    self.insert_char(c);
                                }
                                println!("[TextEdit] Ctrl+V: pasted {} chars", clipboard.len());
                                return EventResult::Handled;
                            }
                            // Ctrl+X: 剪切
                            if key_data.keycode == KeyCode::X as u32 {
                                let mut clipboard = CLIPBOARD.lock();
                                *clipboard = self.text.clone();
                                self.text.clear();
                                self.cursor_position = 0;
                                self.flags.dirty_render = true;
                                println!("[TextEdit] Ctrl+X: cut {} chars", clipboard.len());
                                return EventResult::Handled;
                            }
                        }
                        
                        // Unicode字符输入
                        if key_data.unicode_char > 0 && key_data.unicode_char < 128 {
                            let c = char::from_u32(key_data.unicode_char).unwrap_or('\0');
                            if c != '\0' {
                                self.insert_char(c);
                                return EventResult::Handled;
                            }
                        }
                        // Backspace删除
                        if key_data.keycode == KeyCode::Backspace as u32 {
                            self.delete_char();
                            return EventResult::Handled;
                        }
                    }
                }
            }
            _ => {}
        }
        EventResult::Ignored
    }
}