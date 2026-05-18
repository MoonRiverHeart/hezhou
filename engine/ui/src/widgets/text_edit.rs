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
    cursor_grapheme_index: usize,  // 光标在grapheme cluster的位置
    cursor_byte_index: usize,      // 光标对应的byte位置（用于插入/删除）
    cursor_visible: bool,
    selection_start: usize,
    selection_end: usize,
    focused: bool,
    char_layouts: Vec<CharLayout>,
    layout_dirty: bool,
    cached_max_bearing_y: f32,
    cached_line_height: f32,
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
            cached_max_bearing_y: 0.0,
            cached_line_height: 0.0,
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
        
        if !self.char_layouts.is_empty() {
            println!("[Click] Using precise char_layouts ({} graphemes)", self.char_layouts.len());
            
            let mut best_grapheme_idx = 0;
            let mut best_byte_idx = 0;
            let mut best_distance = f32::MAX;
            
            // 首先找到最近的行（通过 Y 坐标）
            let mut closest_line_y = 0.0;
            let mut min_y_distance = f32::MAX;
            
            for layout in &self.char_layouts {
                let line_y = layout.y;
                let y_distance = (click_y - line_y).abs();
                if y_distance < min_y_distance {
                    min_y_distance = y_distance;
                    closest_line_y = line_y;
                }
            }
            
            println!("[Click] Closest line_y: {}", closest_line_y);
            
            // 然后在该行中找到最近的grapheme（通过 X 坐标）
            for layout in &self.char_layouts {
                if layout.y == closest_line_y {
                    let grapheme_center_x = layout.x + layout.width / 2.0;
                    
                    println!("[Click] grapheme {}: x={}, width={}, center_x={}", 
                             layout.grapheme_index, layout.x, layout.width, grapheme_center_x);
                    
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
                        println!("[Click]   -> x_distance={}, best_grapheme_idx={}", x_distance, best_grapheme_idx);
                    }
                }
            }
            
            println!("[Click] Final grapheme_index={}, byte_index={}", best_grapheme_idx, best_byte_idx);
            return best_grapheme_idx;  // 返回grapheme索引
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
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        // 渲染选择高亮（在文本之前）
        if self.selection_start != self.selection_end && !self.char_layouts.is_empty() {
            let start = self.selection_start.min(self.selection_end);
            let end = self.selection_start.max(self.selection_end);
            
            // 找到选择范围的grapheme，绘制高亮矩形
            for layout in &self.char_layouts {
                if layout.grapheme_index >= start && layout.grapheme_index < end {
                    canvas.draw_rect(
                        Rect::new(layout.x, layout.y, layout.width, layout.height),
                        &Style::new().with_background(Color::new(0.3, 0.5, 0.8, 0.3)),
                    );
                }
            }
        }
        
        if !self.text.is_empty() {
            canvas.draw_text(
                Rect::new(10.0, 10.0, width - 20.0, height - 20.0),
                &self.text,
                &self.text_style,
            );
        }
        
        if self.focused && self.cursor_visible {
            let text_start_x = 10.0;
            let text_start_y = 10.0;
            let font_size = self.text_style.font_size * 2.0;
            
            // 只在布局 dirty 时重新计算
            if self.layout_dirty || self.char_layouts.is_empty() {
                let wrap_width = Some(self.layout.width - 20.0);
                
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
                        y: *baseline_y - max_bearing_y,
                        width: *width,
                        height: font_size,
                        grapheme_index: *grapheme_idx,
                        grapheme_start_byte: *start_byte,
                        grapheme_end_byte: *end_byte,
                    }
                }).collect();
                
                self.cached_max_bearing_y = max_bearing_y;
                self.cached_line_height = font_size * 1.5;
                self.layout_dirty = false;
            }
            
            // 计算光标位置
            let (cursor_x, cursor_y) = if self.cursor_grapheme_index == 0 {
                (text_start_x, text_start_y)
            } else {
                let mut found_x = text_start_x;
                let mut found_y = text_start_y;
                
                for layout in &self.char_layouts {
                    if layout.grapheme_index < self.cursor_grapheme_index {
                        found_x = layout.x + layout.width;
                        found_y = layout.y;
                    } else {
                        break;
                    }
                }
                
                (found_x, found_y)
            };
            
            let cursor_height = font_size * 0.75;
            
            canvas.draw_rect(
                Rect::new(cursor_x, cursor_y, 2.0, cursor_height),
                &Style::new().with_background(Color::white()),
            );
        } else {
            // 即使不显示光标，也需要更新布局（如果 dirty）
            if self.layout_dirty && canvas.get_font_atlas().is_some() && !self.text.is_empty() {
                let text_start_x = 10.0;
                let text_start_y = 10.0;
                let font_size = self.text_style.font_size * 2.0;
                let wrap_width = Some(self.layout.width - 20.0);
                
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
                        y: *baseline_y - max_bearing_y,
                        width: *width,
                        height: font_size,
                        grapheme_index: *grapheme_idx,
                        grapheme_start_byte: *start_byte,
                        grapheme_end_byte: *end_byte,
                    }
                }).collect();
                
                self.cached_max_bearing_y = max_bearing_y;
                self.cached_line_height = font_size * 1.5;
                self.layout_dirty = false;
            }
        }
    }

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        // 注意：虽然签名是 &self，但 widget_tree 通过 as_mut() 调用
        // 这里不更新 char_layouts（需要 &mut self）
        
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
                
                if let EventData::Touch(touch_data) = &event.data {
                    let click_x = touch_data.x;
                    let click_y = touch_data.y;
                    let shift_pressed = touch_data.modifiers & 1 != 0;
                    println!("[Click] Click at ({}, {}), shift={}", click_x, click_y, shift_pressed);
                    
                    let new_grapheme_idx = self.find_cursor_position_at(click_x, click_y);
                    let new_byte_idx = self.grapheme_index_to_byte_index(new_grapheme_idx);
                    
                    if shift_pressed {
                        if self.selection_start == self.selection_end {
                            self.selection_start = self.cursor_grapheme_index;
                            self.selection_end = new_grapheme_idx;
                        } else {
                            self.selection_end = new_grapheme_idx;
                        }
                        println!("[TextEdit] Selection: {} to {}", self.selection_start, self.selection_end);
                    } else {
                        self.selection_start = 0;
                        self.selection_end = 0;
                        self.cursor_grapheme_index = new_grapheme_idx;
                        self.cursor_byte_index = new_byte_idx;
                    }
                    
                    println!("[Click] cursor_grapheme_index={}, cursor_byte_index={}", 
                             self.cursor_grapheme_index, self.cursor_byte_index);
                }
                
                self.flags.dirty_render = true;
                return EventResult::Handled;
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
                                let mut clipboard = CLIPBOARD.lock();
                                if self.selection_start != self.selection_end {
                                    let start_g = self.selection_start.min(self.selection_end);
                                    let end_g = self.selection_start.max(self.selection_end);
                                    let start_byte = self.grapheme_index_to_byte_index(start_g);
                                    let end_byte = self.grapheme_index_to_byte_index(end_g);
                                    *clipboard = self.text[start_byte..end_byte].to_string();
                                    println!("[TextEdit] Ctrl+C: copied selection {} chars", clipboard.len());
                                } else {
                                    *clipboard = self.text.clone();
                                    println!("[TextEdit] Ctrl+C: copied all {} chars", self.text.len());
                                }
                                return EventResult::Handled;
                            }
                            // Ctrl+V: 粘贴clipboard
                            if key_data.keycode == KeyCode::V as u32 {
                                let clipboard = CLIPBOARD.lock();
                                println!("[TextEdit] Ctrl+V: pasting {} chars", clipboard.len());
                                // 粘贴整个clipboard作为一个grapheme序列
                                for grapheme in clipboard.graphemes(true) {
                                    self.insert_grapheme(grapheme);
                                }
                                return EventResult::Handled;
                            }
                            // Ctrl+X: 剪切
                            if key_data.keycode == KeyCode::X as u32 {
                                let mut clipboard = CLIPBOARD.lock();
                                if self.selection_start != self.selection_end {
                                    let start_g = self.selection_start.min(self.selection_end);
                                    let end_g = self.selection_start.max(self.selection_end);
                                    let start_byte = self.grapheme_index_to_byte_index(start_g);
                                    let end_byte = self.grapheme_index_to_byte_index(end_g);
                                    *clipboard = self.text[start_byte..end_byte].to_string();
                                    self.text.drain(start_byte..end_byte);
                                    self.cursor_grapheme_index = start_g;
                                    self.cursor_byte_index = start_byte;
                                } else {
                                    *clipboard = self.text.clone();
                                    self.text.clear();
                                    self.cursor_grapheme_index = 0;
                                    self.cursor_byte_index = 0;
                                }
                                self.selection_start = 0;
                                self.selection_end = 0;
                                self.char_layouts.clear();
                                self.layout_dirty = true;
                                self.flags.dirty_render = true;
                                println!("[TextEdit] Ctrl+X: cut {} chars", clipboard.len());
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