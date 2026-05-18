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
        self.cursor_position = self.text.len();
        self.char_layouts.clear();
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }
    
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
        self.char_layouts.clear();
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let delete_pos = self.cursor_position - 1;
            if delete_pos < self.text.len() && self.is_char_boundary(delete_pos) {
                self.text.remove(delete_pos);
                self.cursor_position = delete_pos;
                self.char_layouts.clear();
                self.layout_dirty = true;
                self.flags.dirty_render = true;
            }
        }
    }
    
    fn is_char_boundary(&self, pos: usize) -> bool {
        if pos == 0 || pos == self.text.len() {
            return true;
        }
        self.text.is_char_boundary(pos)
    }
    
    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            // 找到前一个字符的起始位置
            let prev_char_start = self.text[..self.cursor_position]
                .char_indices()
                .rev()
                .next()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.cursor_position = prev_char_start;
            self.flags.dirty_render = true;
            println!("[TextEdit] Move left: cursor_position={}", self.cursor_position);
        }
    }
    
    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.len() {
            // 找到当前字符并移动到下一个字符的起始位置
            if let Some(c) = self.text[self.cursor_position..].chars().next() {
                self.cursor_position += c.len_utf8();
                self.flags.dirty_render = true;
                println!("[TextEdit] Move right: cursor_position={}", self.cursor_position);
            }
        }
    }
    
    fn move_cursor_up(&mut self) {
        // 找到上一行的相同x位置
        if let Some(current_layout) = self.find_char_layout_at_cursor() {
            let current_x = current_layout.x;
            let current_y = current_layout.y;
            
            // 找到上一行（y更小）
            let prev_line_y = self.char_layouts.iter()
                .filter(|l| l.y < current_y)
                .map(|l| l.y)
                .max_by(|a, b| a.partial_cmp(b).unwrap());
            
            if let Some(prev_y) = prev_line_y {
                // 在上一行找到最接近 current_x 的字符
                let best = self.char_layouts.iter()
                    .filter(|l| l.y == prev_y)
                    .min_by(|a, b| {
                        let dist_a = (a.x - current_x).abs();
                        let dist_b = (b.x - current_x).abs();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });
                
                if let Some(layout) = best {
                    self.cursor_position = layout.byte_index;
                    self.flags.dirty_render = true;
                    println!("[TextEdit] Move up: cursor_position={}", self.cursor_position);
                }
            }
        }
    }
    
    fn move_cursor_down(&mut self) {
        // 找到下一行的相同x位置
        if let Some(current_layout) = self.find_char_layout_at_cursor() {
            let current_x = current_layout.x;
            let current_y = current_layout.y;
            
            // 找到下一行（y更大）
            let next_line_y = self.char_layouts.iter()
                .filter(|l| l.y > current_y)
                .map(|l| l.y)
                .min_by(|a, b| a.partial_cmp(b).unwrap());
            
            if let Some(next_y) = next_line_y {
                // 在下一行找到最接近 current_x 的字符
                let best = self.char_layouts.iter()
                    .filter(|l| l.y == next_y)
                    .min_by(|a, b| {
                        let dist_a = (a.x - current_x).abs();
                        let dist_b = (b.x - current_x).abs();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });
                
                if let Some(layout) = best {
                    self.cursor_position = layout.byte_index;
                    self.flags.dirty_render = true;
                    println!("[TextEdit] Move down: cursor_position={}", self.cursor_position);
                }
            }
        }
    }
    
    fn move_cursor_to_line_start(&mut self) {
        // 找到当前行的起始位置
        if self.cursor_position > 0 {
            let line_start = self.text[..self.cursor_position]
                .match_indices('\n')
                .last()
                .map(|(i, _)| i + 1)
                .unwrap_or(0);
            self.cursor_position = line_start;
        } else {
            self.cursor_position = 0;
        }
        self.flags.dirty_render = true;
        println!("[TextEdit] Move to line start: cursor_position={}", self.cursor_position);
    }
    
    fn move_cursor_to_line_end(&mut self) {
        // 找到当前行的结束位置（下一个\n或文本末尾）
        let line_end = self.text[self.cursor_position..]
            .match_indices('\n')
            .next()
            .map(|(i, _)| self.cursor_position + i)
            .unwrap_or(self.text.len());
        self.cursor_position = line_end;
        self.flags.dirty_render = true;
        println!("[TextEdit] Move to line end: cursor_position={}", self.cursor_position);
    }
    
    fn find_char_layout_at_cursor(&self) -> Option<&CharLayout> {
        // 找到光标前一个字符的布局
        if self.cursor_position == 0 {
            return None;
        }
        
        self.char_layouts.iter()
            .find(|l| l.byte_index == self.cursor_position - 1 || 
                      (l.byte_index < self.cursor_position && 
                       l.byte_index + 1 >= self.cursor_position))
    }
    
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.flags.dirty_render = true;
    }
    
    fn estimate_cursor_x(&self) -> f32 {
        // 估算光标 x 位置（用于验证反向映射）
        let text_start_x = 10.0;
        let char_width = self.text_style.font_size * 0.6;
        
        // 简化计算：cursor_position * char_width
        // 这只是估算，实际渲染时使用 font_atlas 精确值
        text_start_x + self.cursor_position.min(self.text.len()) as f32 * char_width
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
        println!("[Click] Finding cursor position at ({}, {})", click_x, click_y);
        
        if !self.char_layouts.is_empty() {
            println!("[Click] Using precise char_layouts ({} chars)", self.char_layouts.len());
            
            // 找到点击位置最近的字符
            let mut best_pos = 0;
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
            
            println!("[Click] Closest line_y: {}, min_y_distance: {}", closest_line_y, min_y_distance);
            
            // 然后在该行中找到最近的字符（通过 X 坐标）
            for layout in &self.char_layouts {
                // 只考虑同一行的字符
                if layout.y == closest_line_y {
                    let char_center_x = layout.x + layout.width / 2.0;
                    
                    println!("[Click] char {}: x={}, width={}, center_x={}, y={}", 
                             layout.byte_index, layout.x, layout.width, char_center_x, layout.y);
                    
                    let x_distance = (click_x - char_center_x).abs();
                    
                    if x_distance < best_distance {
                        best_distance = x_distance;
                        best_pos = if click_x < char_center_x {
                            layout.byte_index
                        } else {
                            layout.byte_index + 1
                        };
                        println!("[Click]   -> x_distance={}, best_pos={}", x_distance, best_pos);
                    }
                }
            }
            
            // 如果点击超出最后一个字符，放在末尾
            if let Some(last) = self.char_layouts.last() {
                if click_x > last.x + last.width && last.y == closest_line_y {
                    best_pos = self.text.len();
                    println!("[Click] Click beyond last char, pos={}", best_pos);
                }
            }
            
            println!("[Click] Final cursor_position from precise: {}", best_pos);
            return best_pos;
        }
        
        // fallback 使用估算值
        println!("[Click] No char_layouts, using estimation");
        let text_start_x = 10.0;
        let text_start_y = 10.0;
        let line_height = self.text_style.font_size * 1.5;
        let char_width = self.text_style.font_size * 0.6;
        
        let relative_y = click_y - text_start_y;
        let line_index = if relative_y < 0.0 { 0 } else { (relative_y / line_height) as usize };
        
        let mut byte_pos = 0;
        let mut current_line = 0;
        for (i, c) in self.text.char_indices() {
            if c == '\n' {
                current_line += 1;
                if current_line > line_index { break; }
                byte_pos = i + 1;
            }
        }
        
        let relative_x = click_x - text_start_x;
        let estimated_pos = if relative_x <= 0.0 { 0 } else { (relative_x / char_width) as usize };
        
        let result = byte_pos + estimated_pos.min(self.text.len() - byte_pos);
        println!("[Click] Estimated cursor_position: {}", result);
        result
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
            let text_start_x = 10.0;
            let text_start_y = 10.0;
            let font_size = self.text_style.font_size * 2.0;
            
            // 只在布局 dirty 时重新计算
            if self.layout_dirty || self.char_layouts.is_empty() {
                let char_positions = canvas.layout_text_for_cursor(
                    &self.text,
                    font_size,
                    text_start_x,
                    text_start_y,
                );
                
                let max_bearing_y = canvas.get_max_bearing_y(&self.text, font_size);
                
                self.char_layouts = char_positions.iter().map(|(x, baseline_y, advance, char_idx, byte_idx)| {
                    CharLayout {
                        x: *x,
                        y: *baseline_y - max_bearing_y,
                        width: *advance,
                        height: font_size,
                        char_index: *char_idx,
                        byte_index: *byte_idx,
                    }
                }).collect();
                
                self.cached_max_bearing_y = max_bearing_y;
                self.cached_line_height = font_size * 1.5;
                self.layout_dirty = false;
            }
            
            // 计算光标位置（使用缓存的布局）
            let (cursor_x, cursor_y) = if self.cursor_position == 0 {
                (text_start_x, text_start_y)
            } else {
                let mut found_x = text_start_x;
                let mut found_y = text_start_y;
                
                for layout in &self.char_layouts {
                    if layout.byte_index < self.cursor_position {
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
                
                let char_positions = canvas.layout_text_for_cursor(
                    &self.text,
                    font_size,
                    text_start_x,
                    text_start_y,
                );
                
                let max_bearing_y = canvas.get_max_bearing_y(&self.text, font_size);
                
                self.char_layouts = char_positions.iter().map(|(x, baseline_y, advance, char_idx, byte_idx)| {
                    CharLayout {
                        x: *x,
                        y: *baseline_y - max_bearing_y,
                        width: *advance,
                        height: font_size,
                        char_index: *char_idx,
                        byte_index: *byte_idx,
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
                
                // 不调用 update_char_layouts()（使用估算值）
                // 而是依赖 draw() 更新的精确 char_layouts
                
                // 根据点击位置计算cursor_position
                if let EventData::Touch(touch_data) = &event.data {
                    let click_x = touch_data.x;
                    let click_y = touch_data.y;
                    println!("[Click] Click at ({}, {})", click_x, click_y);
                    
                    // 反向映射：像素位置 -> 字符索引
                    // 使用上次 draw() 更新的精确 char_layouts
                    self.cursor_position = self.find_cursor_position_at(click_x, click_y);
                    println!("[Click] cursor_position set to {}", self.cursor_position);
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
                        
                        // 方向键导航
                        if key_data.keycode == KeyCode::Left as u32 {
                            self.move_cursor_left();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Right as u32 {
                            self.move_cursor_right();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Up as u32 {
                            self.move_cursor_up();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Down as u32 {
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