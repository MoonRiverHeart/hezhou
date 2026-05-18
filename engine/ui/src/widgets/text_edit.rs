use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use hezhou_platform::KeyCode;
use parking_lot::Mutex;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;
use arboard::Clipboard;

static CLIPBOARD_BACKUP: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));

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
    cursor_grapheme_index: usize,
    cursor_byte_index: usize,
    cursor_visible: bool,
    selection_start: usize,
    selection_end: usize,
    focused: bool,
    char_layouts: Vec<CharLayout>,
    layout_dirty: bool,
    cached_line_height: f32,
    cached_max_bearing_y: f32,
    scroll_offset_y: f32,
    total_content_height: f32,
    scrollbar_dragging: bool,
    scrollbar_drag_start_y: f32,
    scrollbar_drag_start_offset: f32,
}

#[derive(Clone, Copy, Debug)]
struct CharLayout {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    grapheme_index: usize,      // grapheme cluster索引
    grapheme_start_byte: usize, // 该grapheme在String中的起始字节
    grapheme_end_byte: usize,   // 该grapheme结束字节（exclusive）
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
            cursor_grapheme_index: 0,
            cursor_byte_index: 0,
            cursor_visible: true,
            selection_start: 0,
            selection_end: 0,
            focused: false,
            char_layouts: Vec::new(),
            layout_dirty: true,
            cached_line_height: 0.0,
            cached_max_bearing_y: 0.0,
            scroll_offset_y: 0.0,
            total_content_height: 0.0,
            scrollbar_dragging: false,
            scrollbar_drag_start_y: 0.0,
            scrollbar_drag_start_offset: 0.0,
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
        let num_graphemes = self.text.graphemes(true).count();
        self.cursor_grapheme_index = num_graphemes;
        self.cursor_byte_index = self.text.len();
        self.char_layouts.clear();
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn set_font_size(&mut self, size: f32) {
        self.text_style.font_size = size;
        self.char_layouts.clear();
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn get_text_style(&self) -> &TextStyle {
        &self.text_style
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }
    
    fn grapheme_index_to_byte_index(&self, grapheme_index: usize) -> usize {
        self.text.grapheme_indices(true)
            .nth(grapheme_index)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(self.text.len())
    }
    
    fn byte_index_to_grapheme_index(&self, byte_index: usize) -> usize {
        self.text.grapheme_indices(true)
            .position(|(byte_idx, _)| byte_idx == byte_index)
            .unwrap_or(self.text.graphemes(true).count())
    }
    
    fn get_current_grapheme(&self) -> Option<&str> {
        self.text.graphemes(true).nth(self.cursor_grapheme_index)
    }
    
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_byte_index, c);
        self.cursor_byte_index += c.len_utf8();
        self.cursor_grapheme_index = self.byte_index_to_grapheme_index(self.cursor_byte_index);
        self.char_layouts.clear();
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn insert_grapheme(&mut self, grapheme: &str) {
        self.text.insert_str(self.cursor_byte_index, grapheme);
        self.cursor_byte_index += grapheme.len();
        self.cursor_grapheme_index += 1;
        self.char_layouts.clear();
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn delete_char(&mut self) {
        if self.cursor_grapheme_index > 0 {
            // 找到前一个grapheme的起始和结束位置
            let prev_grapheme = self.text.grapheme_indices(true)
                .nth(self.cursor_grapheme_index - 1);
            
            if let Some((start_byte, grapheme_str)) = prev_grapheme {
                let end_byte = start_byte + grapheme_str.len();
                self.text.drain(start_byte..end_byte);
                self.cursor_byte_index = start_byte;
                self.cursor_grapheme_index -= 1;
                self.char_layouts.clear();
                self.layout_dirty = true;
                self.flags.dirty_render = true;
            }
        }
    }
    
    fn move_cursor_left(&mut self) {
        if self.cursor_grapheme_index > 0 {
            self.cursor_grapheme_index -= 1;
            self.cursor_byte_index = self.grapheme_index_to_byte_index(self.cursor_grapheme_index);
            self.flags.dirty_render = true;
            println!("[TextEdit] Move left: grapheme_index={}, byte_index={}", 
                     self.cursor_grapheme_index, self.cursor_byte_index);
        }
    }
    
    fn move_cursor_right(&mut self) {
        let num_graphemes = self.text.graphemes(true).count();
        if self.cursor_grapheme_index < num_graphemes {
            self.cursor_grapheme_index += 1;
            self.cursor_byte_index = self.grapheme_index_to_byte_index(self.cursor_grapheme_index);
            self.flags.dirty_render = true;
            println!("[TextEdit] Move right: grapheme_index={}, byte_index={}", 
                     self.cursor_grapheme_index, self.cursor_byte_index);
        }
    }
    
    fn move_cursor_up(&mut self) {
        if let Some(current_layout) = self.find_char_layout_at_cursor() {
            let current_x = current_layout.x;
            let current_y = current_layout.y;
            
            let prev_line_y = self.char_layouts.iter()
                .filter(|l| l.y < current_y)
                .map(|l| l.y)
                .max_by(|a, b| a.partial_cmp(b).unwrap());
            
            if let Some(prev_y) = prev_line_y {
                let best = self.char_layouts.iter()
                    .filter(|l| l.y == prev_y)
                    .min_by(|a, b| {
                        let dist_a = (a.x - current_x).abs();
                        let dist_b = (b.x - current_x).abs();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });
                
                if let Some(layout) = best {
                    self.cursor_grapheme_index = layout.grapheme_index;
                    self.cursor_byte_index = layout.grapheme_start_byte;
                    self.flags.dirty_render = true;
                    println!("[TextEdit] Move up: grapheme_index={}", self.cursor_grapheme_index);
                }
            }
        }
    }
    
    fn move_cursor_down(&mut self) {
        if let Some(current_layout) = self.find_char_layout_at_cursor() {
            let current_x = current_layout.x;
            let current_y = current_layout.y;
            
            let next_line_y = self.char_layouts.iter()
                .filter(|l| l.y > current_y)
                .map(|l| l.y)
                .min_by(|a, b| a.partial_cmp(b).unwrap());
            
            if let Some(next_y) = next_line_y {
                let best = self.char_layouts.iter()
                    .filter(|l| l.y == next_y)
                    .min_by(|a, b| {
                        let dist_a = (a.x - current_x).abs();
                        let dist_b = (b.x - current_x).abs();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });
                
                if let Some(layout) = best {
                    self.cursor_grapheme_index = layout.grapheme_index;
                    self.cursor_byte_index = layout.grapheme_start_byte;
                    self.flags.dirty_render = true;
                    println!("[TextEdit] Move down: grapheme_index={}", self.cursor_grapheme_index);
                }
            }
        }
    }
    
    fn move_cursor_to_line_start(&mut self) {
        // 找到当前行的起始位置（上一个\n或文本开头）
        let line_start_byte = self.text[..self.cursor_byte_index]
            .match_indices('\n')
            .last()
            .map(|(i, _)| i + 1)
            .unwrap_or(0);
        
        self.cursor_byte_index = line_start_byte;
        self.cursor_grapheme_index = self.byte_index_to_grapheme_index(line_start_byte);
        self.flags.dirty_render = true;
        println!("[TextEdit] Move to line start: grapheme_index={}", self.cursor_grapheme_index);
    }
    
    fn move_cursor_to_line_end(&mut self) {
        // 找到当前行的结束位置（下一个\n或文本末尾）
        let line_end_byte = self.text[self.cursor_byte_index..]
            .match_indices('\n')
            .next()
            .map(|(i, _)| self.cursor_byte_index + i)
            .unwrap_or(self.text.len());
        
        self.cursor_byte_index = line_end_byte;
        self.cursor_grapheme_index = self.byte_index_to_grapheme_index(line_end_byte);
        self.flags.dirty_render = true;
        println!("[TextEdit] Move to line end: grapheme_index={}", self.cursor_grapheme_index);
    }
    
    fn find_char_layout_at_cursor(&self) -> Option<&CharLayout> {
        self.char_layouts.iter()
            .find(|l| l.grapheme_index == self.cursor_grapheme_index)
    }
    
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.flags.dirty_render = true;
    }
    
    fn find_cursor_position_at(&self, click_x: f32, click_y: f32) -> usize {
        println!("[Click] Finding cursor position at ({}, {})", click_x, click_y);
        
        let num_graphemes = self.text.graphemes(true).count();
        
        if !self.char_layouts.is_empty() {
            println!("[Click] Using precise char_layouts ({} graphemes)", self.char_layouts.len());
            
            let max_bearing_y = self.cached_max_bearing_y;
            let font_size = self.text_style.font_size;
            
            let mut best_grapheme_idx = 0;
            let mut best_byte_idx = 0;
            let mut best_distance = f32::MAX;
            
            let mut closest_line_y = 0.0;
            let mut min_y_distance = f32::MAX;
            
            for layout in &self.char_layouts {
                let cursor_draw_y = layout.y - max_bearing_y;
                let cursor_draw_y_end = cursor_draw_y + font_size;
                let line_center_y = (cursor_draw_y + cursor_draw_y_end) / 2.0;
                let y_distance = (click_y - line_center_y).abs();
                
                if y_distance < min_y_distance {
                    min_y_distance = y_distance;
                    closest_line_y = layout.y;
                }
            }
            
            println!("[Click] Closest baseline_y: {}", closest_line_y);
            
            for layout in &self.char_layouts {
                if layout.y == closest_line_y {
                    let grapheme_center_x = layout.x + layout.width / 2.0;
                    
                    let x_distance = (click_x - grapheme_center_x).abs();
                    
                    if x_distance < best_distance {
                        best_distance = x_distance;
                        if click_x < grapheme_center_x {
                            best_grapheme_idx = layout.grapheme_index;
                            best_byte_idx = layout.grapheme_start_byte;
                        } else {
                            best_grapheme_idx = layout.grapheme_index + 1;
                            best_byte_idx = layout.grapheme_end_byte;
                        }
                    }
                }
            }
            
            // 确保不超过文本长度
            best_grapheme_idx = best_grapheme_idx.min(num_graphemes);
            
            println!("[Click] Final grapheme_index={}, byte_index={}", best_grapheme_idx, best_byte_idx);
            return best_grapheme_idx;
        }
        
        println!("[Click] No char_layouts, returning 0");
        0
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
        let font_size = self.text_style.font_size;
        
        let line_number_width = 50.0;
        let text_margin_x = 10.0;
        let text_start_x = line_number_width + text_margin_x;
        let scrollbar_width = 12.0;
        let text_area_width = width - line_number_width - scrollbar_width - 2.0 * text_margin_x;
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        let line_number_style = Style::new()
            .with_background(Color::new(0.12, 0.12, 0.12, 1.0));
        canvas.draw_rect(Rect::new(0.0, 0.0, line_number_width, height), &line_number_style);
        
        let num_lines = self.text.lines().count().max(1);
        let max_bearing_y = if self.cached_max_bearing_y > 0.0 {
            self.cached_max_bearing_y
        } else {
            canvas.get_max_bearing_y(&self.text, font_size)
        };
        let line_height = if self.cached_line_height > 0.0 {
            self.cached_line_height
        } else {
            canvas.get_line_height(font_size)
        };
        
        self.total_content_height = 10.0 + max_bearing_y + num_lines as f32 * line_height;
        
        let max_scroll = (self.total_content_height - height).max(0.0);
        self.scroll_offset_y = self.scroll_offset_y.min(max_scroll).max(0.0);
        
        let line_number_text_style = TextStyle::new()
            .with_size(font_size)
            .with_color(Color::new(0.5, 0.5, 0.5, 1.0));
        
        for line_num in 1..=num_lines {
            let line_y = 10.0 + max_bearing_y + (line_num - 1) as f32 * line_height - self.scroll_offset_y;
            if line_y - max_bearing_y >= 0.0 && line_y - max_bearing_y < height {
                let line_num_str = line_num.to_string();
                canvas.draw_text(
                    Rect::new(5.0, line_y - max_bearing_y, line_number_width - 10.0, font_size),
                    &line_num_str,
                    &line_number_text_style,
                );
            }
        }
        
        let num_graphemes = self.text.graphemes(true).count();
        
        self.selection_start = self.selection_start.min(num_graphemes);
        self.selection_end = self.selection_end.min(num_graphemes);
        self.cursor_grapheme_index = self.cursor_grapheme_index.min(num_graphemes);
        
        if self.selection_start != self.selection_end && !self.char_layouts.is_empty() {
            let start = self.selection_start.min(self.selection_end);
            let end = self.selection_start.max(self.selection_end);
            
            let mut lines: Vec<(f32, f32, f32)> = Vec::new();
            
            for layout in &self.char_layouts {
                if layout.grapheme_index >= start && layout.grapheme_index < end {
                    let y = layout.y - self.scroll_offset_y;
                    
                    if let Some(last_line) = lines.last_mut() {
                        if last_line.0 == y {
                            last_line.2 = layout.x + layout.width.max(1.0);
                        } else {
                            lines.push((y, layout.x, layout.x + layout.width.max(1.0)));
                        }
                    } else {
                        lines.push((y, layout.x, layout.x + layout.width.max(1.0)));
                    }
                }
            }
            
            for (y, start_x, end_x) in &lines {
                let highlight_y = y - max_bearing_y;
                if highlight_y >= 0.0 && highlight_y < height {
                    canvas.draw_rect(
                        Rect::new(*start_x, highlight_y, *end_x - *start_x, font_size),
                        &Style::new().with_background(Color::new(0.3, 0.5, 0.8, 0.3)),
                    );
                }
            }
        }
        
        if !self.text.is_empty() {
            let text_start_y = 10.0 - self.scroll_offset_y;
            canvas.draw_text(
                Rect::new(text_start_x, text_start_y, text_area_width, self.total_content_height),
                &self.text,
                &self.text_style,
            );
        }
        
        if self.total_content_height > height {
            let scrollbar_x = width - scrollbar_width;
            let scrollbar_bg_style = Style::new()
                .with_background(Color::new(0.08, 0.08, 0.08, 1.0));
            canvas.draw_rect(Rect::new(scrollbar_x, 0.0, scrollbar_width, height), &scrollbar_bg_style);
            
            let scrollbar_ratio = height / self.total_content_height;
            let scrollbar_height = (height * scrollbar_ratio).max(30.0);
            let scrollbar_y = (self.scroll_offset_y / max_scroll) * (height - scrollbar_height);
            
            let scrollbar_style = Style::new()
                .with_background(Color::new(0.3, 0.3, 0.3, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 1.0, 3.0);
            canvas.draw_rect(
                Rect::new(scrollbar_x + 1.0, scrollbar_y, scrollbar_width - 2.0, scrollbar_height),
                &scrollbar_style,
            );
        }
        
        if self.focused && self.cursor_visible {
            let text_start_y = 10.0 - self.scroll_offset_y;
            
            if self.layout_dirty || self.char_layouts.is_empty() {
                let wrap_width = Some(text_area_width);
                
                let char_positions = canvas.layout_text_for_cursor_with_wrap(
                    &self.text,
                    font_size,
                    text_start_x,
                    10.0,
                    wrap_width,
                );
                
                let max_bearing_y = canvas.get_max_bearing_y(&self.text, font_size);
                
                self.char_layouts = char_positions.iter().map(|(x, baseline_y, width, grapheme_idx, start_byte, end_byte)| {
                    CharLayout {
                        x: *x,
                        y: *baseline_y,
                        width: *width,
                        height: font_size,
                        grapheme_index: *grapheme_idx,
                        grapheme_start_byte: *start_byte,
                        grapheme_end_byte: *end_byte,
                    }
                }).collect();
                
                self.cached_max_bearing_y = max_bearing_y;
                self.cached_line_height = canvas.get_line_height(font_size);
                self.layout_dirty = false;
            }
            
            let max_bearing_y = self.cached_max_bearing_y;
            
            let (cursor_x, cursor_y) = if self.cursor_grapheme_index == 0 {
                (text_start_x, text_start_y + max_bearing_y)
            } else {
                let mut found_x = text_start_x;
                let mut found_y = text_start_y + max_bearing_y;
                
                for layout in &self.char_layouts {
                    if layout.grapheme_index < self.cursor_grapheme_index {
                        found_x = layout.x + layout.width;
                        found_y = layout.y - self.scroll_offset_y;
                    } else {
                        break;
                    }
                }
                
                (found_x, found_y)
            };
            
            if cursor_y - max_bearing_y >= 0.0 && cursor_y - max_bearing_y < height {
                canvas.draw_rect(
                    Rect::new(cursor_x, cursor_y - max_bearing_y, 2.0, canvas.get_font_height(font_size)),
                    &Style::new().with_background(Color::white()),
                );
            }
        } else {
            if self.layout_dirty && canvas.get_font_atlas().is_some() && !self.text.is_empty() {
                let text_start_y = 10.0;
                let wrap_width = Some(text_area_width);
                
                let char_positions = canvas.layout_text_for_cursor_with_wrap(
                    &self.text,
                    font_size,
                    text_start_x,
                    text_start_y,
                    wrap_width,
                );
                
                let max_bearing_y = canvas.get_max_bearing_y(&self.text, font_size);
                
                self.char_layouts = char_positions.iter().map(|(x, baseline_y, width, grapheme_idx, start_byte, end_byte)| {
                    CharLayout {
                        x: *x,
                        y: *baseline_y,
                        width: *width,
                        height: font_size,
                        grapheme_index: *grapheme_idx,
                        grapheme_start_byte: *start_byte,
                        grapheme_end_byte: *end_byte,
                    }
                }).collect();
                
                self.cached_max_bearing_y = max_bearing_y;
                self.cached_line_height = canvas.get_line_height(font_size);
                self.layout_dirty = false;
            }
        }
    }

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        // 注意：虽然签名是 &self，但 widget_tree 通过 as_mut() 调用
        // 这里不更新 char_layouts（需要 &mut self）
        
        let (text_width, text_height) =
            font_atlas.measure_text(0, &self.text, self.text_style.font_size);

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
            EventType::MouseWheel => {
                if let EventData::Wheel(wheel_data) = &event.data {
                    let max_scroll = (self.total_content_height - self.layout.height).max(0.0);
                    self.scroll_offset_y = (self.scroll_offset_y + wheel_data.delta_y * 30.0)
                        .min(max_scroll)
                        .max(0.0);
                    self.flags.dirty_render = true;
                    return EventResult::Handled;
                }
            }
            EventType::TouchBegin => {
                let width = self.layout.width;
                let height = self.layout.height;
                let scrollbar_width = 12.0;
                let scrollbar_x = width - scrollbar_width;
                
                if let EventData::Touch(touch_data) = &event.data {
                    let click_x = touch_data.x;
                    let click_y = touch_data.y;
                    
                    if self.total_content_height > height && click_x >= scrollbar_x {
                        let max_scroll = (self.total_content_height - height).max(0.0);
                        let scrollbar_ratio = height / self.total_content_height;
                        let scrollbar_height = (height * scrollbar_ratio).max(30.0);
                        let scrollbar_y = (self.scroll_offset_y / max_scroll) * (height - scrollbar_height);
                        
                        if click_y >= scrollbar_y && click_y <= scrollbar_y + scrollbar_height {
                            self.scrollbar_dragging = true;
                            self.scrollbar_drag_start_y = click_y;
                            self.scrollbar_drag_start_offset = self.scroll_offset_y;
                            self.flags.dirty_render = true;
                            return EventResult::Handled;
                        }
                    }
                }
                
                println!("[TextEdit] TouchBegin received");
                self.focused = true;
                self.cursor_visible = true;
                
                if let EventData::Touch(touch_data) = &event.data {
                    let click_x = touch_data.x;
                    let click_y = touch_data.y + self.scroll_offset_y;
                    let shift_pressed = touch_data.modifiers & 1 != 0;
                    println!("[Click] Click at ({}, {}), shift={}", click_x, click_y, shift_pressed);
                    
                    let num_graphemes = self.text.graphemes(true).count();
                    let new_grapheme_idx = self.find_cursor_position_at(click_x, click_y).min(num_graphemes);
                    let new_byte_idx = self.grapheme_index_to_byte_index(new_grapheme_idx);
                    
                    if shift_pressed {
                        if self.selection_start == self.selection_end {
                            self.selection_start = self.cursor_grapheme_index.min(num_graphemes);
                            self.selection_end = new_grapheme_idx;
                        } else {
                            self.selection_end = new_grapheme_idx;
                        }
                        println!("[TextEdit] Selection: {} to {}", self.selection_start, self.selection_end);
                    } else {
                        self.selection_start = new_grapheme_idx;
                        self.selection_end = new_grapheme_idx;
                        self.cursor_grapheme_index = new_grapheme_idx;
                        self.cursor_byte_index = new_byte_idx;
                    }
                    
                    println!("[Click] cursor_grapheme_index={}, cursor_byte_index={}", 
                             self.cursor_grapheme_index, self.cursor_byte_index);
                }
                
                self.flags.dirty_render = true;
                return EventResult::Handled;
            }
            EventType::TouchMove => {
                if self.scrollbar_dragging {
                    if let EventData::Touch(touch_data) = &event.data {
                        let height = self.layout.height;
                        let drag_delta_y = touch_data.y - self.scrollbar_drag_start_y;
                        let max_scroll = (self.total_content_height - height).max(0.0);
                        let scrollbar_ratio = height / self.total_content_height;
                        let scrollbar_height = (height * scrollbar_ratio).max(30.0);
                        let scrollable_track = height - scrollbar_height;
                        
                        let scroll_delta = (drag_delta_y / scrollable_track) * max_scroll;
                        self.scroll_offset_y = (self.scrollbar_drag_start_offset + scroll_delta)
                            .min(max_scroll)
                            .max(0.0);
                        self.flags.dirty_render = true;
                        return EventResult::Handled;
                    }
                }
                
                if self.focused {
                    if let EventData::Touch(touch_data) = &event.data {
                        let click_x = touch_data.x;
                        let click_y = touch_data.y + self.scroll_offset_y;
                        
                        let num_graphemes = self.text.graphemes(true).count();
                        let new_grapheme_idx = self.find_cursor_position_at(click_x, click_y).min(num_graphemes);
                        let new_byte_idx = self.grapheme_index_to_byte_index(new_grapheme_idx);
                        
                        self.selection_end = new_grapheme_idx;
                        self.cursor_grapheme_index = new_grapheme_idx;
                        self.cursor_byte_index = new_byte_idx;
                        
                        println!("[Drag] Selection: {} to {}", self.selection_start, self.selection_end);
                        self.flags.dirty_render = true;
                        return EventResult::Handled;
                    }
                }
            }
            EventType::TouchEnd => {
                if self.scrollbar_dragging {
                    self.scrollbar_dragging = false;
                    self.flags.dirty_render = true;
                    return EventResult::Handled;
                }
                
                if self.focused {
                    println!("[TextEdit] TouchEnd, finalizing selection");
                }
            }
            EventType::KeyDown => {
                println!("[TextEdit] KeyDown received, focused={}", self.focused);
                if self.focused {
                    if let EventData::Key(key_data) = &event.data {
                        println!("[TextEdit] keycode={}, unicode={}, modifiers={}", 
                                 key_data.keycode, key_data.unicode_char, key_data.modifiers);
                        
                        let ctrl_pressed = key_data.modifiers & 2 != 0;
                        
                        if ctrl_pressed {
                            if key_data.keycode == KeyCode::C as u32 {
                                let text_to_copy = if self.selection_start != self.selection_end {
                                    let start_g = self.selection_start.min(self.selection_end);
                                    let end_g = self.selection_start.max(self.selection_end);
                                    let start_byte = self.grapheme_index_to_byte_index(start_g);
                                    let end_byte = self.grapheme_index_to_byte_index(end_g);
                                    self.text[start_byte..end_byte].to_string()
                                } else {
                                    self.text.clone()
                                };
                                
                                if let Ok(mut clipboard) = Clipboard::new() {
                                    if let Err(e) = clipboard.set_text(&text_to_copy) {
                                        println!("[TextEdit] Clipboard set error: {:?}", e);
                                        let mut backup = CLIPBOARD_BACKUP.lock();
                                        *backup = text_to_copy.clone();
                                    } else {
                                        println!("[TextEdit] Ctrl+C: copied {} chars to system clipboard", text_to_copy.len());
                                    }
                                } else {
                                    let mut backup = CLIPBOARD_BACKUP.lock();
                                    *backup = text_to_copy.clone();
                                    println!("[TextEdit] Ctrl+C: copied {} chars to backup (no system clipboard)", text_to_copy.len());
                                }
                                return EventResult::Handled;
                            }
                            if key_data.keycode == KeyCode::V as u32 {
                                let text_to_paste = if let Ok(mut clipboard) = Clipboard::new() {
                                    clipboard.get_text().unwrap_or_else(|e| {
                                        println!("[TextEdit] Clipboard get error: {:?}", e);
                                        CLIPBOARD_BACKUP.lock().clone()
                                    })
                                } else {
                                    CLIPBOARD_BACKUP.lock().clone()
                                };
                                
                                println!("[TextEdit] Ctrl+V: pasting {} chars", text_to_paste.len());
                                for grapheme in text_to_paste.graphemes(true) {
                                    self.insert_grapheme(grapheme);
                                }
                                return EventResult::Handled;
                            }
                            if key_data.keycode == KeyCode::X as u32 {
                                let text_to_cut = if self.selection_start != self.selection_end {
                                    let start_g = self.selection_start.min(self.selection_end);
                                    let end_g = self.selection_start.max(self.selection_end);
                                    let start_byte = self.grapheme_index_to_byte_index(start_g);
                                    let end_byte = self.grapheme_index_to_byte_index(end_g);
                                    let cut_text = self.text[start_byte..end_byte].to_string();
                                    self.text.drain(start_byte..end_byte);
                                    self.cursor_grapheme_index = start_g;
                                    self.cursor_byte_index = start_byte;
                                    cut_text
                                } else {
                                    let cut_text = self.text.clone();
                                    self.text.clear();
                                    self.cursor_grapheme_index = 0;
                                    self.cursor_byte_index = 0;
                                    cut_text
                                };
                                self.selection_start = 0;
                                self.selection_end = 0;
                                
                                if let Ok(mut clipboard) = Clipboard::new() {
                                    if let Err(e) = clipboard.set_text(&text_to_cut) {
                                        println!("[TextEdit] Clipboard set error: {:?}", e);
                                        let mut backup = CLIPBOARD_BACKUP.lock();
                                        *backup = text_to_cut.clone();
                                    } else {
                                        println!("[TextEdit] Ctrl+X: cut {} chars to system clipboard", text_to_cut.len());
                                    }
                                } else {
                                    let mut backup = CLIPBOARD_BACKUP.lock();
                                    *backup = text_to_cut.clone();
                                }
                                
                                self.char_layouts.clear();
                                self.layout_dirty = true;
                                self.flags.dirty_render = true;
                                return EventResult::Handled;
                            }
                        }
                        
                        // Unicode字符输入
                        if key_data.unicode_char > 0 {
                            let c = char::from_u32(key_data.unicode_char);
                            if let Some(c) = c {
                                println!("[TextEdit] Inserting char: '{}' (unicode={})", c, key_data.unicode_char);
                                self.insert_char(c);
                                return EventResult::Handled;
                            }
                        }
                        
                        // Backspace删除（删除前一个grapheme）
                        if key_data.keycode == KeyCode::Backspace as u32 {
                            println!("[TextEdit] Backspace, deleting grapheme");
                            self.delete_char();
                            return EventResult::Handled;
                        }
                        
                        // 方向键导航
                        println!("[TextEdit] Checking arrow keys: keycode={}, Left={}, Right={}, Up={}, Down={}", 
                                 key_data.keycode, KeyCode::Left as u32, KeyCode::Right as u32, KeyCode::Up as u32, KeyCode::Down as u32);
                        
                        if key_data.keycode == KeyCode::Left as u32 {
                            println!("[TextEdit] ArrowLeft detected!");
                            self.move_cursor_left();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Right as u32 {
                            println!("[TextEdit] ArrowRight detected!");
                            self.move_cursor_right();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Up as u32 {
                            println!("[TextEdit] ArrowUp detected!");
                            self.move_cursor_up();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Down as u32 {
                            println!("[TextEdit] ArrowDown detected!");
                            self.move_cursor_down();
                            return EventResult::Handled;
                        }
                        
                        // Home/End键
                        if key_data.keycode == KeyCode::Home as u32 {
                            self.move_cursor_to_line_start();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::End as u32 {
                            self.move_cursor_to_line_end();
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