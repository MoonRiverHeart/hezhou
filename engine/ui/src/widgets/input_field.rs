use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::font_atlas::FontAtlas;
use crate::types::Point;

pub struct InputField {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    text: String,
    placeholder: String,
    /// all_text = text + placeholder，确保所有可能显示的字符都被预光栅化
    all_text: String,
    cursor_position: usize,
    selection_start: usize,
    selection_end: usize,
    has_selection: bool,
    is_focused: bool,
    on_change: Option<Box<dyn FnMut(&str) + Send + Sync>>,
    content_scale: f32,
    flags: crate::widget::WidgetFlags,
}

impl InputField {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 200.0, 30.0),
            style: Style::new()
                .with_background(Color::new(0.15, 0.15, 0.15, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 1.0, 4.0),
            state: WidgetState::Normal,
            text: String::new(),
            placeholder: String::new(),
            all_text: String::new(),
            cursor_position: 0,
            selection_start: 0,
            selection_end: 0,
            has_selection: false,
            is_focused: false,
            on_change: None,
            content_scale: 1.0,
            flags: crate::widget::WidgetFlags::default(),
        }
    }

    fn rebuild_all_text(&mut self) {
        self.all_text = format!("{}{}", self.text, self.placeholder);
    }
    
    pub fn clear_selection(&mut self) {
        self.selection_start = 0;
        self.selection_end = 0;
        self.has_selection = false;
        self.flags.dirty_render = true;
    }
    
    pub fn select_all(&mut self) {
        self.selection_start = 0;
        self.selection_end = self.text.len();
        self.has_selection = true;
        self.cursor_position = self.text.len();
        self.flags.dirty_render = true;
    }
    
    pub fn has_selection(&self) -> bool {
        self.has_selection
    }
    
    pub fn get_selected_text(&self) -> Option<&str> {
        if self.has_selection && !self.text.is_empty() {
            let start = self.selection_start.min(self.selection_end);
            let end = self.selection_start.max(self.selection_end);
            if start < end && end <= self.text.len() {
                let start_byte = self.text.char_indices().nth(start).map(|(i, _)| i).unwrap_or(0);
                let end_byte = self.text.char_indices().nth(end).map(|(i, _)| i).unwrap_or(self.text.len());
                Some(&self.text[start_byte..end_byte])
            } else {
                None
            }
        } else {
            None
        }
    }
    
    fn extend_selection(&mut self) {
        if !self.has_selection {
            self.selection_start = self.cursor_position;
        }
        self.selection_end = self.cursor_position;
        self.has_selection = true;
        self.flags.dirty_render = true;
    }
    
    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self.rebuild_all_text();
        self
    }
    
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor_position = self.text.len();
        self.rebuild_all_text();
        self.flags.dirty_render = true;
    }
    
    pub fn text(&self) -> &str {
        &self.text
    }
    
    pub fn set_placeholder(&mut self, placeholder: &str) {
        self.placeholder = placeholder.to_string();
        self.rebuild_all_text();
        self.flags.dirty_render = true;
    }
    
    pub fn set_on_change(&mut self, callback: Box<dyn FnMut(&str) + Send + Sync>) {
        self.on_change = Some(callback);
    }
    
    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
    }
    
    pub fn focus(&mut self) {
        self.is_focused = true;
        crate::thunk::ui_set_focused_input_field(self.id.id);
        self.flags.dirty_render = true;
    }
    
    pub fn blur(&mut self) {
        self.is_focused = false;
        crate::thunk::ui_clear_focused_input_field();
        self.flags.dirty_render = true;
    }
    
    pub fn blur_internal(&mut self) {
        self.is_focused = false;
        self.flags.dirty_render = true;
    }
    
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }
    
    fn insert_char(&mut self, c: char) {
        let byte_pos = self.cursor_byte_position();
        self.text.insert(byte_pos, c);
        self.cursor_position += 1;
        self.rebuild_all_text();
        self.trigger_on_change();
        self.flags.dirty_render = true;
    }
    
    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let byte_pos = self.cursor_byte_position();
            self.text.remove(byte_pos);
            self.rebuild_all_text();
            self.trigger_on_change();
            self.flags.dirty_render = true;
        }
    }
    
    fn cursor_byte_position(&self) -> usize {
        self.text.char_indices()
            .nth(self.cursor_position)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }
    
    fn trigger_on_change(&mut self) {
        crate::thunk::queue_callback(crate::thunk::PendingCallback::InputFieldChange {
            widget_id: self.id.id,
            text: self.text.clone(),
        });
        if let Some(callback) = &mut self.on_change {
            callback(&self.text);
        }
    }
    
    fn delete_selection(&mut self) {
        if self.has_selection {
            let start = self.selection_start.min(self.selection_end);
            let end = self.selection_start.max(self.selection_end);
            
            let start_byte = self.text.char_indices().nth(start).map(|(i, _)| i).unwrap_or(0);
            let end_byte = self.text.char_indices().nth(end).map(|(i, _)| i).unwrap_or(self.text.len());
            
            self.text.replace_range(start_byte..end_byte, "");
            self.cursor_position = start;
            self.clear_selection();
            self.rebuild_all_text();
            self.trigger_on_change();
            self.flags.dirty_render = true;
        }
    }
}

impl Widget for InputField {
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
    
    fn measure(&self, _font_atlas: &FontAtlas) -> (f32, f32) {
        (self.layout.width, self.layout.height)
    }
    
    fn widget_type(&self) -> &'static str {
        "InputField"
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
        Some(&self.all_text)
    }
    
    fn draw(&mut self, canvas: &mut Canvas) {
        let global_focused = crate::thunk::ui_get_focused_input_field();
        self.is_focused = global_focused == self.id.id;
        
        let current_style = if self.is_focused {
            Style::new()
                .with_background(Color::new(0.18, 0.18, 0.18, 1.0))
                .with_border(Color::new(0.5, 0.7, 1.0, 1.0), 2.0, 4.0)
        } else {
            match self.state {
                WidgetState::Hovered => Style::new()
                    .with_background(Color::new(0.2, 0.2, 0.2, 1.0))
                    .with_border(Color::new(0.5, 0.5, 0.5, 1.0), 1.0, 4.0),
                _ => self.style,
            }
        };
        
        let rect = Rect::new(0.0, 0.0, self.layout.width, self.layout.height);
        canvas.draw_rect(rect, &current_style);
        
        let font_size = 14.0 * self.content_scale;
        
        if self.has_selection && !self.text.is_empty() {
            let start = self.selection_start.min(self.selection_end);
            let end = self.selection_start.max(self.selection_end);
            
            if let Some(font_atlas) = canvas.get_font_atlas() {
                let chars_before_start: String = self.text.chars().take(start).collect();
                let (start_x, _) = font_atlas.measure_text(0, &chars_before_start, font_size);
                
                let chars_selected: String = self.text.chars().take(end).collect();
                let (end_x, _) = font_atlas.measure_text(0, &chars_selected, font_size);
                
                let selection_rect = Rect::new(
                    8.0 + start_x,
                    6.0,
                    end_x - start_x,
                    self.layout.height - 12.0
                );
                let selection_style = Style::new()
                    .with_background(Color::new(0.2, 0.4, 0.8, 0.5));
                canvas.draw_rect(selection_rect, &selection_style);
            }
        }
        
        let display_text = if self.text.is_empty() && !self.is_focused {
            &self.placeholder
        } else {
            &self.text
        };
        
        let text_color = if self.text.is_empty() && !self.is_focused {
            Color::new(0.5, 0.5, 0.5, 1.0)
        } else {
            Color::white()
        };
        
        let text_style = TextStyle::new()
            .with_size(font_size)
            .with_color(text_color)
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Left,
                vertical: VerticalAlignment::Center,
            });
        
        let text_rect = Rect::new(8.0, 0.0, self.layout.width - 16.0, self.layout.height);
        canvas.draw_text(text_rect, display_text, &text_style);
        
        if self.is_focused {
            if let Some(font_atlas) = canvas.get_font_atlas() {
                let text_width = if self.text.is_empty() {
                    0.0
                } else {
                    let chars_before_cursor: String = self.text.chars()
                        .take(self.cursor_position)
                        .collect();
                    let (w, _) = font_atlas.measure_text(0, &chars_before_cursor, font_size);
                    w
                };
                
                let cursor_x = 8.0 + text_width;
                let cursor_y1 = 6.0;
                let cursor_y2 = self.layout.height - 6.0;
                
                canvas.draw_line(
                    Point::new(cursor_x, cursor_y1),
                    Point::new(cursor_x, cursor_y2),
                    Color::white(),
                    1.0
                );
            }
        }
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                if self.state != WidgetState::Disabled {
                    self.set_state(WidgetState::Pressed);
                    self.focus();
                    self.clear_selection();
                    return EventResult::Handled;
                }
            }
            
            EventType::TouchEnd => {
                if self.state == WidgetState::Pressed {
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
                if self.state == WidgetState::Hovered {
                    self.set_state(WidgetState::Normal);
                    return EventResult::Handled;
                }
            }
            
            EventType::KeyDown => {
                let focused_id = crate::thunk::ui_get_focused_input_field();
                if focused_id != self.id.id {
                    return EventResult::Ignored;
                }
                
                if let EventData::Key(key_data) = &event.data {
                        let keycode = key_data.keycode;
                        let modifiers = key_data.modifiers;
                        let shift = (modifiers & 1) != 0;
                        let ctrl = (modifiers & 2) != 0;
                        
                        if ctrl && keycode == 1 {
                            self.select_all();
                            return EventResult::Handled;
                        }
                        
                        if keycode == 8 {
                            if self.has_selection {
                                self.delete_selection();
                            } else {
                                self.delete_char();
                            }
                            return EventResult::Handled;
                        } else if keycode == 13 {
                            self.blur();
                            return EventResult::Handled;
                        } else if keycode >= 32 && keycode <= 126 {
                            if self.has_selection {
                                self.delete_selection();
                            }
                            self.insert_char(keycode as u8 as char);
                            return EventResult::Handled;
                        } else if key_data.unicode_char > 0 {
                            if let Some(c) = char::from_u32(key_data.unicode_char) {
                                if self.has_selection {
                                    self.delete_selection();
                                }
                                self.insert_char(c);
                                return EventResult::Handled;
                            }
                        }
                        
                        let left_keycode = 80u32;
                        let right_keycode = 79u32;
                        let home_keycode = 74u32;
                        let end_keycode = 77u32;
                        
                        if keycode == left_keycode {
                            if self.cursor_position > 0 {
                                self.cursor_position -= 1;
                                if shift {
                                    self.extend_selection();
                                } else {
                                    self.clear_selection();
                                }
                            }
                            return EventResult::Handled;
                        } else if keycode == right_keycode {
                            if self.cursor_position < self.text.chars().count() {
                                self.cursor_position += 1;
                                if shift {
                                    self.extend_selection();
                                } else {
                                    self.clear_selection();
                                }
                            }
                            return EventResult::Handled;
                        } else if keycode == home_keycode {
                            self.cursor_position = 0;
                            if shift {
                                self.extend_selection();
                            } else {
                                self.clear_selection();
                            }
                            return EventResult::Handled;
                        } else if keycode == end_keycode {
                            self.cursor_position = self.text.chars().count();
                            if shift {
                                self.extend_selection();
                            } else {
                                self.clear_selection();
                            }
                            return EventResult::Handled;
                        }
                    }
                }
            
            _ => {}
        }
        EventResult::Ignored
    }
}