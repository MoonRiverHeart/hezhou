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
    char_layouts: Vec<CharLayout>,
}

#[derive(Clone, Copy, Debug)]
struct CharLayout {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    char_index: usize,
    byte_index: usize,
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
            char_layouts: Vec::new(),
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
        self.char_layouts.clear();
        self.flags.dirty_render = true;
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }
    
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
        self.char_layouts.clear();
        self.flags.dirty_render = true;
    }
    
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let delete_pos = self.cursor_position - 1;
            if delete_pos < self.text.len() {
                self.text.remove(delete_pos);
                self.cursor_position = delete_pos;
                self.char_layouts.clear();
                self.flags.dirty_render = true;
            }
        }
    }
    
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.flags.dirty_render = true;
    }
    
    fn update_char_layouts(&mut self) {
        if !self.char_layouts.is_empty() {
            return;
        }
        
        let text_start_x = 10.0;
        let text_start_y = 10.0;
        let line_height = self.text_style.font_size * 1.5;
        let char_width = self.text_style.font_size * 0.6;
        
        let mut cursor_x = text_start_x;
        let mut cursor_y = text_start_y;
        
        for (byte_index, c) in self.text.char_indices() {
            if c == '\n' {
                cursor_x = text_start_x;
                cursor_y += line_height;
                continue;
            }
            
            let char_layout = CharLayout {
                x: cursor_x,
                y: cursor_y,
                width: char_width,
                height: self.text_style.font_size,
                char_index: self.char_layouts.len(),
                byte_index,
            };
            
            self.char_layouts.push(char_layout);
            cursor_x += char_width;
        }
    }
    
    fn find_cursor_position_at(&self, click_x: f32, click_y: f32) -> usize {
        let text_start_x = 10.0;
        let text_start_y = 10.0;
        let line_height = self.text_style.font_size * 1.5;
        let char_width = self.text_style.font_size * 0.6;
        
        // 计算点击的行号
        let relative_y = click_y - text_start_y;
        let line_index = if relative_y < 0.0 {
            0
        } else {
            (relative_y / line_height) as usize
        };
        
        // 找到该行的所有字符
        let mut current_line = 0;
        let mut line_chars: Vec<&CharLayout> = Vec::new();
        
        for layout in &self.char_layouts {
            if layout.y == text_start_y + (current_line as f32 * line_height) {
                line_chars.push(layout);
            } else if layout.y > text_start_y + (current_line as f32 * line_height) {
                current_line += 1;
                if current_line > line_index {
                    break;
                }
                line_chars.clear();
                line_chars.push(layout);
            }
        }
        
        // 如果点击的行没有字符（可能是空行）
        if line_chars.is_empty() {
            // 找到该行的起始位置
            let mut byte_pos = 0;
            let mut current_line = 0;
            for (i, c) in self.text.char_indices() {
                if c == '\n' {
                    current_line += 1;
                    if current_line > line_index {
                        break;
                    }
                    byte_pos = i + 1;
                }
            }
            return byte_pos;
        }
        
        // 在该行中找到最近的字符
        let relative_x = click_x - text_start_x;
        let mut best_pos = 0;
        let mut best_distance = f32::MAX;
        
        for layout in &line_chars {
            let char_center_x = layout.x + layout.width / 2.0;
            let distance = (relative_x - char_center_x).abs();
            
            if distance < best_distance {
                best_distance = distance;
                best_pos = if relative_x < char_center_x {
                    layout.byte_index
                } else {
                    layout.byte_index + 1
                };
            }
        }
        
        best_pos
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
            // 更新字符布局
            self.update_char_layouts();
            
            // 找到光标位置对应的字符
            let (cursor_x, cursor_y) = if self.cursor_position == 0 {
                // 光标在最开始
                (10.0, 10.0)
            } else {
                // 查找光标前一个字符
                let mut found = None;
                for layout in &self.char_layouts {
                    if layout.byte_index + 1 == self.cursor_position || 
                       (layout.byte_index < self.cursor_position && 
                        (layout.char_index + 1 >= self.char_layouts.len() || 
                         self.char_layouts[layout.char_index + 1].byte_index >= self.cursor_position)) {
                        found = Some(*layout);
                        break;
                    }
                }
                
                if let Some(layout) = found {
                    (layout.x + layout.width, layout.y)
                } else {
                    // 找不到，放到最后
                    if let Some(last) = self.char_layouts.last() {
                        (last.x + last.width, last.y)
                    } else {
                        (10.0, 10.0)
                    }
                }
            };
            
            let cursor_height = self.text_style.font_size * 1.2;
            
            canvas.draw_rect(
                Rect::new(cursor_x, cursor_y, 2.0, cursor_height),
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
                println!("[TextEdit] TouchBegin received");
                self.focused = true;
                self.cursor_visible = true;
                
                // 更新字符布局
                self.update_char_layouts();
                
                // 根据点击位置计算cursor_position
                if let EventData::Touch(touch_data) = &event.data {
                    let click_x = touch_data.x;
                    let click_y = touch_data.y;
                    println!("[TextEdit] Click at ({}, {})", click_x, click_y);
                    
                    self.cursor_position = self.find_cursor_position_at(click_x, click_y);
                    println!("[TextEdit] cursor_position set to {}", self.cursor_position);
                }
                
                self.flags.dirty_render = true;
                return EventResult::Handled;
            }
            EventType::KeyDown => {
                println!("[TextEdit] KeyDown received, focused={}", self.focused);
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
                                println!("[TextEdit] Ctrl+V: pasting {} chars", clipboard.len());
                                for c in clipboard.chars() {
                                    self.insert_char(c);
                                }
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
                                println!("[TextEdit] Inserting char: '{}' (unicode={})", c, key_data.unicode_char);
                                self.insert_char(c);
                                return EventResult::Handled;
                            }
                        }
                        // Backspace删除
                        if key_data.keycode == KeyCode::Backspace as u32 {
                            println!("[TextEdit] Backspace, deleting char");
                            self.delete_char();
                            return EventResult::Handled;
                        }
                        
                        println!("[TextEdit] KeyDown ignored: keycode={}, unicode={}, ctrl={}", 
                            key_data.keycode, key_data.unicode_char, ctrl_pressed);
                    }
                }
            }
            _ => {}
        }
        EventResult::Ignored
    }
}