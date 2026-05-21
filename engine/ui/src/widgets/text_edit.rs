use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::text_layout::TextLayoutPresenter;
use hezhou_dfx::*;
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
    presenter: TextLayoutPresenter,
    layout_dirty: bool,
    scroll_offset_x: f32,
    scroll_offset_y: f32,
    total_content_width: f32,
    total_content_height: f32,
    v_scrollbar_dragging: bool,
    v_scrollbar_drag_start_y: f32,
    v_scrollbar_drag_start_offset: f32,
    h_scrollbar_dragging: bool,
    h_scrollbar_drag_start_x: f32,
    h_scrollbar_drag_start_offset: f32,
    show_line_numbers: bool,
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
            presenter: TextLayoutPresenter::new(),
            layout_dirty: true,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            total_content_width: 0.0,
            total_content_height: 0.0,
            v_scrollbar_dragging: false,
            v_scrollbar_drag_start_y: 0.0,
            v_scrollbar_drag_start_offset: 0.0,
            h_scrollbar_dragging: false,
            h_scrollbar_drag_start_x: 0.0,
            h_scrollbar_drag_start_offset: 0.0,
            show_line_numbers: false,
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
        self.presenter.set_text(text, self.text_style.font_size);
        let num_graphemes = self.text.graphemes(true).count();
        self.cursor_grapheme_index = num_graphemes;
        self.cursor_byte_index = self.text.len();
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn set_font_size(&mut self, size: f32) {
        self.text_style.font_size = size;
        self.presenter.set_text(&self.text, size);
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }

    pub fn set_show_line_numbers(&mut self, show: bool) {
        self.show_line_numbers = show;
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
        self.presenter.set_text(&self.text, self.text_style.font_size);
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn insert_grapheme(&mut self, grapheme: &str) {
        self.text.insert_str(self.cursor_byte_index, grapheme);
        self.cursor_byte_index += grapheme.len();
        self.cursor_grapheme_index += 1;
        self.presenter.set_text(&self.text, self.text_style.font_size);
        self.layout_dirty = true;
        self.flags.dirty_render = true;
    }
    
    pub fn delete_char(&mut self) {
        if self.cursor_grapheme_index > 0 {
            let prev_grapheme = self.text.grapheme_indices(true)
                .nth(self.cursor_grapheme_index - 1);
            
            if let Some((start_byte, grapheme_str)) = prev_grapheme {
                let end_byte = start_byte + grapheme_str.len();
                self.text.drain(start_byte..end_byte);
                self.cursor_byte_index = start_byte;
                self.cursor_grapheme_index -= 1;
                self.presenter.set_text(&self.text, self.text_style.font_size);
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
            dfx_info!("TextEdit", "Move left: grapheme_index={}, byte_index={}", 
                     self.cursor_grapheme_index, self.cursor_byte_index);
        }
    }
    
    fn move_cursor_right(&mut self) {
        let num_graphemes = self.text.graphemes(true).count();
        if self.cursor_grapheme_index < num_graphemes {
            self.cursor_grapheme_index += 1;
            self.cursor_byte_index = self.grapheme_index_to_byte_index(self.cursor_grapheme_index);
            self.flags.dirty_render = true;
            dfx_info!("TextEdit", "Move right: grapheme_index={}, byte_index={}", 
                     self.cursor_grapheme_index, self.cursor_byte_index);
        }
    }
    
    fn move_cursor_up(&mut self) {
        let (cursor_x, cursor_y) = self.presenter.grapheme_to_pixel(self.cursor_grapheme_index);
        let target_y = cursor_y - self.presenter.get_model().get_line_height();
        
        let new_grapheme = self.presenter.pixel_to_grapheme(cursor_x, target_y);
        self.cursor_grapheme_index = new_grapheme;
        self.cursor_byte_index = self.grapheme_index_to_byte_index(new_grapheme);
        self.flags.dirty_render = true;
    }
    
    fn move_cursor_down(&mut self) {
        let (cursor_x, cursor_y) = self.presenter.grapheme_to_pixel(self.cursor_grapheme_index);
        let target_y = cursor_y + self.presenter.get_model().get_line_height();
        
        let new_grapheme = self.presenter.pixel_to_grapheme(cursor_x, target_y);
        self.cursor_grapheme_index = new_grapheme;
        self.cursor_byte_index = self.grapheme_index_to_byte_index(new_grapheme);
        self.flags.dirty_render = true;
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
        dfx_info!("TextEdit", "Move to line start: grapheme_index={}", self.cursor_grapheme_index);
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
        dfx_info!("TextEdit", "Move to line end: grapheme_index={}", self.cursor_grapheme_index);
    }
    
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.flags.dirty_render = true;
    }
    
    fn find_cursor_position_at(&self, click_x: f32, click_y: f32) -> usize {
        self.presenter.pixel_to_grapheme(click_x, click_y)
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

    fn flags(&self) -> WidgetFlags {
        self.flags
    }

    fn set_flags(&mut self, flags: WidgetFlags) {
        self.flags = flags;
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        let font_size = self.text_style.font_size;
        
        let line_number_width = if self.show_line_numbers { 50.0 } else { 0.0 };
        let text_margin_x = 10.0;
        let text_start_x = if self.show_line_numbers { line_number_width + text_margin_x } else { text_margin_x };
        let scrollbar_width = 12.0;
        let scrollbar_height = 12.0;
        let text_area_width = width - line_number_width - scrollbar_width - 2.0 * text_margin_x;
        let text_area_height = height - scrollbar_height;
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        if self.show_line_numbers {
            let line_number_style = Style::new()
                .with_background(Color::new(0.12, 0.12, 0.12, 1.0));
            canvas.draw_rect(Rect::new(0.0, 0.0, line_number_width, text_area_height), &line_number_style);
        }
        
        if let Some(font_atlas) = canvas.get_font_atlas() {
            if self.layout_dirty {
                self.presenter.set_scroll_offset_y(self.scroll_offset_y);
                self.presenter.set_scroll_offset_x(self.scroll_offset_x);
                self.presenter.set_container(text_start_x, 10.0);
                self.presenter.set_clip_rect(Rect::new(text_start_x, 0.0, text_area_width, text_area_height));
                self.presenter.update_layout(font_atlas, 0, None);
                self.layout_dirty = false;
            }
            
            self.total_content_height = self.presenter.get_total_height();
            self.total_content_width = self.presenter.get_total_width();
        }
        
        let line_height = self.presenter.get_model().get_line_height();
        let max_bearing_y = self.presenter.get_model().get_max_bearing_y();
        
        let max_scroll_y = (self.total_content_height - text_area_height).max(0.0);
        let max_scroll_x = (self.total_content_width - text_area_width).max(0.0);
        self.scroll_offset_y = self.scroll_offset_y.min(max_scroll_y).max(0.0);
        self.scroll_offset_x = self.scroll_offset_x.min(max_scroll_x).max(0.0);
        
        if self.show_line_numbers {
            let line_number_text_style = TextStyle::new()
                .with_size(font_size)
                .with_color(Color::new(0.5, 0.5, 0.5, 1.0));
            
            for (line_idx, line) in self.presenter.get_model().get_lines().iter().enumerate() {
                let line_y = line.baseline_y - max_bearing_y - self.scroll_offset_y;
                if line_y >= 0.0 && line_y < text_area_height {
                    let line_num_str = (line_idx + 1).to_string();
                    canvas.draw_text(
                        Rect::new(5.0, line_y, line_number_width - 10.0, font_size),
                        &line_num_str,
                        &line_number_text_style,
                    );
                }
            }
        }
        
        let num_graphemes = self.text.graphemes(true).count();
        
        self.selection_start = self.selection_start.min(num_graphemes);
        self.selection_end = self.selection_end.min(num_graphemes);
        self.cursor_grapheme_index = self.cursor_grapheme_index.min(num_graphemes);
        
        if self.selection_start != self.selection_end {
            let start = self.selection_start.min(self.selection_end);
            let end = self.selection_start.max(self.selection_end);
            
            for line in self.presenter.get_model().get_lines() {
                if line.end_grapheme >= start && line.start_grapheme < end {
                    let line_y = line.baseline_y - max_bearing_y - self.scroll_offset_y;
                    if line_y >= 0.0 && line_y < text_area_height {
                        let start_x = if line.start_grapheme >= start {
                            line.x_start - self.scroll_offset_x
                        } else {
                            let (x, _) = self.presenter.grapheme_to_pixel(start);
                            x
                        };
                        let end_x = if line.end_grapheme <= end {
                            line.x_end - self.scroll_offset_x
                        } else {
                            let (x, _) = self.presenter.grapheme_to_pixel(end);
                            x
                        };
                        
                        if start_x < text_start_x + text_area_width && end_x > text_start_x {
                            canvas.draw_rect(
                                Rect::new(start_x.max(text_start_x), line_y, (end_x - start_x).min(text_area_width), line_height),
                                &Style::new().with_background(Color::new(0.3, 0.5, 0.8, 0.3)),
                            );
                        }
                    }
                }
            }
        }
        
        if !self.text.is_empty() {
            let text_start_y = 10.0 - self.scroll_offset_y;
            let text_draw_x = text_start_x - self.scroll_offset_x;
            canvas.set_clip_rect(Rect::new(text_start_x, 0.0, text_area_width, text_area_height));
            canvas.draw_text(
                Rect::new(text_draw_x, text_start_y, self.total_content_width, self.total_content_height),
                &self.text,
                &self.text_style,
            );
            canvas.clear_clip();
        }
        
        // Vertical scrollbar
        if self.total_content_height > text_area_height {
            let v_scrollbar_x = width - scrollbar_width;
            let scrollbar_bg_style = Style::new()
                .with_background(Color::new(0.08, 0.08, 0.08, 1.0));
            canvas.draw_rect(Rect::new(v_scrollbar_x, 0.0, scrollbar_width, text_area_height), &scrollbar_bg_style);
            
            let scrollbar_ratio = text_area_height / self.total_content_height;
            let v_scrollbar_h = (text_area_height * scrollbar_ratio).max(30.0);
            let v_scrollbar_y = (self.scroll_offset_y / max_scroll_y) * (text_area_height - v_scrollbar_h);
            
            let scrollbar_style = Style::new()
                .with_background(Color::new(0.3, 0.3, 0.3, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 1.0, 3.0);
            canvas.draw_rect(
                Rect::new(v_scrollbar_x + 1.0, v_scrollbar_y, scrollbar_width - 2.0, v_scrollbar_h),
                &scrollbar_style,
            );
        }
        
        // Horizontal scrollbar
        if self.total_content_width > text_area_width {
            let h_scrollbar_y = text_area_height;
            let scrollbar_bg_style = Style::new()
                .with_background(Color::new(0.08, 0.08, 0.08, 1.0));
            canvas.draw_rect(Rect::new(line_number_width, h_scrollbar_y, width - line_number_width - scrollbar_width, scrollbar_height), &scrollbar_bg_style);
            
            let scrollbar_ratio = text_area_width / self.total_content_width;
            let h_scrollbar_w = (text_area_width * scrollbar_ratio).max(30.0);
            let h_scrollbar_x = line_number_width + (self.scroll_offset_x / max_scroll_x) * (text_area_width - h_scrollbar_w);
            
            let scrollbar_style = Style::new()
                .with_background(Color::new(0.3, 0.3, 0.3, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 1.0, 3.0);
            canvas.draw_rect(
                Rect::new(h_scrollbar_x, h_scrollbar_y + 1.0, h_scrollbar_w, scrollbar_height - 2.0),
                &scrollbar_style,
            );
        }
        
        // Cursor
        if self.focused && self.cursor_visible {
            let (cursor_x, cursor_y) = self.presenter.grapheme_to_pixel(self.cursor_grapheme_index);
            let draw_y = cursor_y - max_bearing_y;
            
            if draw_y >= 0.0 && draw_y < text_area_height && cursor_x >= text_start_x && cursor_x < text_start_x + text_area_width {
                canvas.draw_rect(
                    Rect::new(cursor_x, draw_y, 2.0, max_bearing_y + 4.0),
                    &Style::new().with_background(Color::white()),
                );
            }
        }
    }

    fn measure(&self, font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
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
                    let text_area_height = self.layout.height - 12.0;
                    let text_area_width = self.layout.width - 50.0 - 12.0 - 20.0;
                    
                    let max_scroll_y = (self.total_content_height - text_area_height).max(0.0);
                    let max_scroll_x = (self.total_content_width - text_area_width).max(0.0);
                    
                    self.scroll_offset_y = (self.scroll_offset_y + wheel_data.delta_y * 30.0)
                        .min(max_scroll_y)
                        .max(0.0);
                    self.scroll_offset_x = (self.scroll_offset_x + wheel_data.delta_x * 30.0)
                        .min(max_scroll_x)
                        .max(0.0);
                    
                    self.layout_dirty = true;
                    self.flags.dirty_render = true;
                    return EventResult::Handled;
                }
            }
            EventType::TouchBegin => {
                let width = self.layout.width;
                let height = self.layout.height;
                let line_number_width = 50.0;
                let scrollbar_width = 12.0;
                let scrollbar_height = 12.0;
                let text_area_height = height - scrollbar_height;
                let v_scrollbar_x = width - scrollbar_width;
                let h_scrollbar_y = text_area_height;
                
                if let EventData::Touch(touch_data) = &event.data {
                    let click_x = touch_data.x;
                    let click_y = touch_data.y;
                    
                    // Vertical scrollbar
                    if self.total_content_height > text_area_height && click_x >= v_scrollbar_x && click_y < h_scrollbar_y {
                        let max_scroll_y = (self.total_content_height - text_area_height).max(0.0);
                        let scrollbar_ratio = text_area_height / self.total_content_height;
                        let v_scrollbar_h = (text_area_height * scrollbar_ratio).max(30.0);
                        let v_scrollbar_y = (self.scroll_offset_y / max_scroll_y) * (text_area_height - v_scrollbar_h);
                        
                        if click_y >= v_scrollbar_y && click_y <= v_scrollbar_y + v_scrollbar_h {
                            self.v_scrollbar_dragging = true;
                            self.v_scrollbar_drag_start_y = click_y;
                            self.v_scrollbar_drag_start_offset = self.scroll_offset_y;
                            self.flags.dirty_render = true;
                            return EventResult::Handled;
                        }
                    }
                    
                    // Horizontal scrollbar
                    if self.total_content_width > width - line_number_width - scrollbar_width - 20.0 && click_y >= h_scrollbar_y && click_x >= line_number_width {
                        let text_area_width = width - line_number_width - scrollbar_width - 20.0;
                        let max_scroll_x = (self.total_content_width - text_area_width).max(0.0);
                        let scrollbar_ratio = text_area_width / self.total_content_width;
                        let h_scrollbar_w = (text_area_width * scrollbar_ratio).max(30.0);
                        let h_scrollbar_x = line_number_width + (self.scroll_offset_x / max_scroll_x) * (text_area_width - h_scrollbar_w);
                        
                        if click_x >= h_scrollbar_x && click_x <= h_scrollbar_x + h_scrollbar_w {
                            self.h_scrollbar_dragging = true;
                            self.h_scrollbar_drag_start_x = click_x;
                            self.h_scrollbar_drag_start_offset = self.scroll_offset_x;
                            self.flags.dirty_render = true;
                            return EventResult::Handled;
                        }
                    }
                }
                
                self.focused = true;
                self.cursor_visible = true;
                
                if let EventData::Touch(touch_data) = &event.data {
                    let click_x = touch_data.x;
                    let click_y = touch_data.y;
                    let shift_pressed = touch_data.modifiers & 1 != 0;
                    
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
                        dfx_info!("TextEdit", "Selection: {} to {}", self.selection_start, self.selection_end);
                    } else {
                        self.selection_start = new_grapheme_idx;
                        self.selection_end = new_grapheme_idx;
                        self.cursor_grapheme_index = new_grapheme_idx;
                        self.cursor_byte_index = new_byte_idx;
                    }
                    
                    dfx_info!("Click", "cursor_grapheme_index={}, cursor_byte_index={}", 
                             self.cursor_grapheme_index, self.cursor_byte_index);
                }
                
                self.flags.dirty_render = true;
                return EventResult::Handled;
            }
            EventType::TouchMove => {
                let width = self.layout.width;
                let height = self.layout.height;
                let text_area_height = height - 12.0;
                let text_area_width = width - 50.0 - 12.0 - 20.0;
                
                if self.v_scrollbar_dragging {
                    if let EventData::Touch(touch_data) = &event.data {
                        let max_scroll_y = (self.total_content_height - text_area_height).max(0.0);
                        let scrollbar_ratio = text_area_height / self.total_content_height;
                        let v_scrollbar_h = (text_area_height * scrollbar_ratio).max(30.0);
                        let scrollable_track = text_area_height - v_scrollbar_h;
                        
                        let drag_delta_y = touch_data.y - self.v_scrollbar_drag_start_y;
                        let scroll_delta = (drag_delta_y / scrollable_track) * max_scroll_y;
                        self.scroll_offset_y = (self.v_scrollbar_drag_start_offset + scroll_delta)
                            .min(max_scroll_y)
                            .max(0.0);
                        self.layout_dirty = true;
                        self.flags.dirty_render = true;
                        return EventResult::Handled;
                    }
                }
                
                if self.h_scrollbar_dragging {
                    if let EventData::Touch(touch_data) = &event.data {
                        let max_scroll_x = (self.total_content_width - text_area_width).max(0.0);
                        let scrollbar_ratio = text_area_width / self.total_content_width;
                        let h_scrollbar_w = (text_area_width * scrollbar_ratio).max(30.0);
                        let scrollable_track = text_area_width - h_scrollbar_w;
                        
                        let drag_delta_x = touch_data.x - self.h_scrollbar_drag_start_x;
                        let scroll_delta = (drag_delta_x / scrollable_track) * max_scroll_x;
                        self.scroll_offset_x = (self.h_scrollbar_drag_start_offset + scroll_delta)
                            .min(max_scroll_x)
                            .max(0.0);
                        self.layout_dirty = true;
                        self.flags.dirty_render = true;
                        return EventResult::Handled;
                    }
                }
                
                if self.focused {
                    if let EventData::Touch(touch_data) = &event.data {
                        let click_x = touch_data.x;
                        let click_y = touch_data.y;
                        
                        let num_graphemes = self.text.graphemes(true).count();
                        let new_grapheme_idx = self.find_cursor_position_at(click_x, click_y).min(num_graphemes);
                        let new_byte_idx = self.grapheme_index_to_byte_index(new_grapheme_idx);
                        
                        self.selection_end = new_grapheme_idx;
                        self.cursor_grapheme_index = new_grapheme_idx;
                        self.cursor_byte_index = new_byte_idx;
                        
                        self.flags.dirty_render = true;
                        return EventResult::Handled;
                    }
                }
            }
            EventType::TouchEnd => {
                if self.v_scrollbar_dragging {
                    self.v_scrollbar_dragging = false;
                    self.flags.dirty_render = true;
                    return EventResult::Handled;
                }
                
                if self.h_scrollbar_dragging {
                    self.h_scrollbar_dragging = false;
                    self.flags.dirty_render = true;
                    return EventResult::Handled;
                }
                
                if self.focused {
                    dfx_info!("TextEdit", "TouchEnd, finalizing selection");
                }
            }
            EventType::KeyDown => {
                dfx_info!("TextEdit", "KeyDown received, focused={}", self.focused);
                if self.focused {
                    if let EventData::Key(key_data) = &event.data {
                        dfx_info!("TextEdit", "keycode={}, unicode={}, modifiers={}", 
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
                                        dfx_error!("TextEdit", "Clipboard set error: {:?}", e);
                                        let mut backup = CLIPBOARD_BACKUP.lock();
                                        *backup = text_to_copy.clone();
                                    } else {
                                        dfx_info!("TextEdit", "Ctrl+C: copied {} chars to system clipboard", text_to_copy.len());
                                    }
                                } else {
                                    let mut backup = CLIPBOARD_BACKUP.lock();
                                    *backup = text_to_copy.clone();
                                    dfx_info!("TextEdit", "Ctrl+C: copied {} chars to backup (no system clipboard)", text_to_copy.len());
                                }
                                return EventResult::Handled;
                            }
                            if key_data.keycode == KeyCode::V as u32 {
                                let text_to_paste = if let Ok(mut clipboard) = Clipboard::new() {
                                    clipboard.get_text().unwrap_or_else(|e| {
                                        dfx_error!("TextEdit", "Clipboard get error: {:?}", e);
                                        CLIPBOARD_BACKUP.lock().clone()
                                    })
                                } else {
                                    CLIPBOARD_BACKUP.lock().clone()
                                };
                                
                                dfx_info!("TextEdit", "Ctrl+V: pasting {} chars", text_to_paste.len());
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
                                        dfx_error!("TextEdit", "Clipboard set error: {:?}", e);
                                        let mut backup = CLIPBOARD_BACKUP.lock();
                                        *backup = text_to_cut.clone();
                                    } else {
                                        dfx_info!("TextEdit", "Ctrl+X: cut {} chars to system clipboard", text_to_cut.len());
                                    }
                                } else {
                                    let mut backup = CLIPBOARD_BACKUP.lock();
                                    *backup = text_to_cut.clone();
                                }
                                
                                self.presenter.set_text(&self.text, self.text_style.font_size);
                                self.layout_dirty = true;
                                self.flags.dirty_render = true;
                                return EventResult::Handled;
                            }
                        }
                        
                        // Unicode字符输入
                        if key_data.unicode_char > 0 {
                            let c = char::from_u32(key_data.unicode_char);
                            if let Some(c) = c {
                                dfx_info!("TextEdit", "Inserting char: '{}' (unicode={})", c, key_data.unicode_char);
                                self.insert_char(c);
                                return EventResult::Handled;
                            }
                        }
                        
                        // Backspace删除（删除前一个grapheme）
                        if key_data.keycode == KeyCode::Backspace as u32 {
                            dfx_info!("TextEdit", "Backspace, deleting grapheme");
                            self.delete_char();
                            return EventResult::Handled;
                        }
                        
                        // 方向键导航
                        dfx_info!("TextEdit", "Checking arrow keys: keycode={}, Left={}, Right={}, Up={}, Down={}", 
                                 key_data.keycode, KeyCode::Left as u32, KeyCode::Right as u32, KeyCode::Up as u32, KeyCode::Down as u32);
                        
                        if key_data.keycode == KeyCode::Left as u32 {
                            dfx_info!("TextEdit", "ArrowLeft detected!");
                            self.move_cursor_left();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Right as u32 {
                            dfx_info!("TextEdit", "ArrowRight detected!");
                            self.move_cursor_right();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Up as u32 {
                            dfx_info!("TextEdit", "ArrowUp detected!");
                            self.move_cursor_up();
                            return EventResult::Handled;
                        }
                        if key_data.keycode == KeyCode::Down as u32 {
                            dfx_info!("TextEdit", "ArrowDown detected!");
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
                        
                        dfx_info!("TextEdit", "KeyDown ignored: keycode={}, unicode={}, ctrl={}", 
                            key_data.keycode, key_data.unicode_char, ctrl_pressed);
                    }
                }
            }
            _ => {}
        }
        EventResult::Ignored
    }
}