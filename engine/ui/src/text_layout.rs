use crate::font_atlas::FontAtlas;
use crate::types::{Rect, Color};
use crate::style::{TextStyle, Style};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug)]
pub struct GlyphLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_w: f32,
    pub uv_h: f32,
    pub grapheme_index: usize,
    pub advance_x: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct LineLayout {
    pub baseline_y: f32,
    pub x_start: f32,
    pub x_end: f32,
    pub start_grapheme: usize,
    pub end_grapheme: usize,
    pub height: f32,
}

pub struct TextLayoutModel {
    text: String,
    font_size: f32,
    glyphs: Vec<GlyphLayout>,
    lines: Vec<LineLayout>,
    max_bearing_y: f32,
    line_height: f32,
    dirty: bool,
}

impl TextLayoutModel {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            font_size: 16.0,
            glyphs: Vec::new(),
            lines: Vec::new(),
            max_bearing_y: 0.0,
            line_height: 0.0,
            dirty: true,
        }
    }
    
    pub fn set_text(&mut self, text: &str, font_size: f32) {
        self.text = text.to_string();
        self.font_size = font_size;
        self.dirty = true;
    }
    
    pub fn update_layout(&mut self, font_atlas: &FontAtlas, font_index: usize, container_x: f32, container_y: f32, wrap_width: Option<f32>) {
        if !self.dirty && !self.glyphs.is_empty() {
            return;
        }
        
        self.glyphs.clear();
        self.lines.clear();
        
        self.max_bearing_y = font_atlas.get_font_ascent(font_index, self.font_size);
        self.line_height = font_atlas.get_font_line_height(font_index, self.font_size);
        
        let baseline_y = container_y + self.max_bearing_y;
        let mut cursor_x = container_x;
        let mut current_baseline_y = baseline_y;
        let mut line_start_grapheme = 0;
        
        for (grapheme_idx, (_byte_idx, grapheme)) in self.text.grapheme_indices(true).enumerate() {
            if grapheme == "\n" {
                self.lines.push(LineLayout {
                    baseline_y: current_baseline_y,
                    x_start: container_x,
                    x_end: cursor_x,
                    start_grapheme: line_start_grapheme,
                    end_grapheme: grapheme_idx,
                    height: self.line_height,
                });
                
                cursor_x = container_x;
                current_baseline_y += self.line_height;
                line_start_grapheme = grapheme_idx + 1;
                continue;
            }
            
            let mut grapheme_width = 0.0;
            let mut grapheme_chars: Vec<(f32, f32, f32, f32, f32, f32, f32, f32)> = Vec::new();
            
            for c in grapheme.chars() {
                if let Some(info) = font_atlas.get_char_info(font_index, c, self.font_size) {
                    grapheme_width += info.advance_x;
                    if info.width > 0.0 && info.height > 0.0 {
                        grapheme_chars.push((
                            info.bearing_x,
                            info.bearing_y,
                            info.width,
                            info.height,
                            info.uv_x,
                            info.uv_y,
                            info.uv_w,
                            info.uv_h,
                        ));
                    }
                }
            }
            
            if let Some(max_width) = wrap_width {
                if cursor_x + grapheme_width > container_x + max_width && cursor_x > container_x {
                    self.lines.push(LineLayout {
                        baseline_y: current_baseline_y,
                        x_start: container_x,
                        x_end: cursor_x,
                        start_grapheme: line_start_grapheme,
                        end_grapheme: grapheme_idx,
                        height: self.line_height,
                    });
                    
                    cursor_x = container_x;
                    current_baseline_y += self.line_height;
                    line_start_grapheme = grapheme_idx;
                }
            }
            
            for (bearing_x, bearing_y, w, h, uv_x, uv_y, uv_w, uv_h) in &grapheme_chars {
                let gx = (cursor_x + *bearing_x).round();
                let gy = (current_baseline_y - *bearing_y).round();
                
                self.glyphs.push(GlyphLayout {
                    x: gx,
                    y: gy,
                    width: *w,
                    height: *h,
                    uv_x: *uv_x,
                    uv_y: *uv_y,
                    uv_w: *uv_w,
                    uv_h: *uv_h,
                    grapheme_index: grapheme_idx,
                    advance_x: grapheme_width,
                });
            }
            
            cursor_x += grapheme_width;
        }
        
        if line_start_grapheme <= self.text.graphemes(true).count() {
            self.lines.push(LineLayout {
                baseline_y: current_baseline_y,
                x_start: container_x,
                x_end: cursor_x,
                start_grapheme: line_start_grapheme,
                end_grapheme: self.text.graphemes(true).count(),
                height: self.line_height,
            });
        }
        
        self.dirty = false;
    }
    
    pub fn get_total_height(&self) -> f32 {
        if self.lines.is_empty() {
            return 0.0;
        }
        self.lines.last().unwrap().baseline_y - self.lines.first().unwrap().baseline_y + self.line_height + self.max_bearing_y
    }
    
    pub fn get_total_width(&self) -> f32 {
        if self.lines.is_empty() {
            return 0.0;
        }
        let max_x_end: f32 = self.lines.iter().map(|l| l.x_end).fold(0.0f32, |a: f32, b: f32| a.max(b));
        max_x_end - self.lines.first().unwrap().x_start
    }
    
    pub fn get_glyphs(&self) -> &[GlyphLayout] {
        &self.glyphs
    }
    
    pub fn get_lines(&self) -> &[LineLayout] {
        &self.lines
    }
    
    pub fn get_max_bearing_y(&self) -> f32 {
        self.max_bearing_y
    }
    
    pub fn get_line_height(&self) -> f32 {
        self.line_height
    }
    
    pub fn find_line_at_y(&self, y: f32) -> Option<usize> {
        for (idx, line) in self.lines.iter().enumerate() {
            let line_top = line.baseline_y - self.max_bearing_y;
            let line_bottom = line_top + line.height;
            if y >= line_top && y < line_bottom {
                return Some(idx);
            }
        }
        None
    }
    
    pub fn find_grapheme_at_position(&self, x: f32, y: f32) -> usize {
        if self.lines.is_empty() {
            return 0;
        }
        
        if let Some(line_idx) = self.find_line_at_y(y) {
            let line = &self.lines[line_idx];
            
            if x <= line.x_start {
                return line.start_grapheme;
            }
            if x >= line.x_end {
                return line.end_grapheme;
            }
            
            let mut prev_glyph_end_x = line.x_start;
            
            for glyph in &self.glyphs {
                if glyph.grapheme_index >= line.start_grapheme && glyph.grapheme_index < line.end_grapheme {
                    let glyph_start_x = glyph.x;
                    let glyph_end_x = glyph.x + glyph.width;
                    let glyph_center_x = (glyph_start_x + glyph_end_x) / 2.0;
                    
                    if x < glyph_center_x {
                        return glyph.grapheme_index;
                    }
                    
                    prev_glyph_end_x = glyph_end_x;
                }
            }
            
            return line.end_grapheme;
        }
        
        if y < self.lines[0].baseline_y - self.max_bearing_y {
            return 0;
        }
        
        if let Some(last_line) = self.lines.last() {
            if y >= last_line.baseline_y - self.max_bearing_y {
                return last_line.end_grapheme;
            }
        }
        
        0
    }
    
    pub fn get_cursor_position(&self, grapheme_index: usize) -> (f32, f32) {
        if self.lines.is_empty() {
            return (0.0, 0.0);
        }
        
        if grapheme_index == 0 {
            if let Some(first_line) = self.lines.first() {
                return (first_line.x_start, first_line.baseline_y);
            }
        }
        
        for line in &self.lines {
            if grapheme_index > line.start_grapheme && grapheme_index <= line.end_grapheme {
                let mut cursor_x = line.x_start;
                
                for glyph in &self.glyphs {
                    if glyph.grapheme_index >= line.start_grapheme && glyph.grapheme_index < grapheme_index {
                        cursor_x = glyph.x + glyph.advance_x;
                    }
                }
                
                return (cursor_x.round(), line.baseline_y);
            }
        }
        
        if let Some(last_line) = self.lines.last() {
            return (last_line.x_end.round(), last_line.baseline_y);
        }
        
        (0.0, 0.0)
    }
    
    pub fn get_text(&self) -> &str {
        &self.text
    }
    
    pub fn get_font_size(&self) -> f32 {
        self.font_size
    }
}

impl Default for TextLayoutModel {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TextLayoutPresenter {
    model: TextLayoutModel,
    scroll_offset_x: f32,
    scroll_offset_y: f32,
    clip_rect: Option<Rect>,
    container_x: f32,
    container_y: f32,
}

impl TextLayoutPresenter {
    pub fn new() -> Self {
        Self {
            model: TextLayoutModel::new(),
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            clip_rect: None,
            container_x: 0.0,
            container_y: 0.0,
        }
    }
    
    pub fn set_text(&mut self, text: &str, font_size: f32) {
        self.model.set_text(text, font_size);
    }
    
    pub fn set_scroll_offset_y(&mut self, offset: f32) {
        self.scroll_offset_y = offset;
    }
    
    pub fn set_scroll_offset_x(&mut self, offset: f32) {
        self.scroll_offset_x = offset;
    }
    
    pub fn set_clip_rect(&mut self, rect: Rect) {
        self.clip_rect = Some(Rect::new(
            rect.x.round(),
            rect.y.round(),
            rect.width.round(),
            rect.height.round(),
        ));
    }
    
    pub fn clear_clip(&mut self) {
        self.clip_rect = None;
    }
    
    pub fn set_container(&mut self, x: f32, y: f32) {
        self.container_x = x;
        self.container_y = y;
    }
    
    pub fn update_layout(&mut self, font_atlas: &FontAtlas, font_index: usize, wrap_width: Option<f32>) {
        let effective_y = self.container_y - self.scroll_offset_y;
        self.model.update_layout(font_atlas, font_index, self.container_x, effective_y, wrap_width);
    }
    
    pub fn get_visible_glyphs(&self) -> Vec<GlyphLayout> {
        if self.clip_rect.is_none() {
            return self.model.get_glyphs().to_vec();
        }
        
        let clip = self.clip_rect.unwrap();
        self.model.get_glyphs()
            .iter()
            .filter(|g| {
                let clip_top = clip.y;
                let clip_bottom = clip.y + clip.height;
                let glyph_top = g.y;
                let glyph_bottom = g.y + g.height;
                
                glyph_bottom > clip_top && glyph_top < clip_bottom &&
                g.x + self.scroll_offset_x >= clip.x && g.x + self.scroll_offset_x + g.width <= clip.x + clip.width
            })
            .cloned()
            .collect()
    }
    
    pub fn pixel_to_grapheme(&self, click_x: f32, click_y: f32) -> usize {
        let effective_x = click_x + self.scroll_offset_x;
        let effective_y = click_y + self.scroll_offset_y;
        self.model.find_grapheme_at_position(effective_x.round(), effective_y.round())
    }
    
pub fn grapheme_to_pixel(&self, grapheme_index: usize) -> (f32, f32) {
        let (x, y) = self.model.get_cursor_position(grapheme_index);
        ((x - self.scroll_offset_x).round(), (y - self.scroll_offset_y).round())
    }
    
    pub fn get_max_scroll_y(&self, container_height: f32) -> f32 {
        (self.model.get_total_height() - container_height).max(0.0)
    }
    
    pub fn get_max_scroll_x(&self, container_width: f32) -> f32 {
        (self.model.get_total_width() - container_width).max(0.0)
    }
    
    pub fn get_scroll_offset_y(&self) -> f32 {
        self.scroll_offset_y
    }
    
    pub fn get_scroll_offset_x(&self) -> f32 {
        self.scroll_offset_x
    }
    
    pub fn get_model(&self) -> &TextLayoutModel {
        &self.model
    }
    
    pub fn get_total_height(&self) -> f32 {
        self.model.get_total_height()
    }
    
    pub fn get_total_width(&self) -> f32 {
        self.model.get_total_width()
    }
}

impl Default for TextLayoutPresenter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TextRendererCommand {
    pub glyphs: Vec<GlyphLayout>,
    pub clip_rect: Option<Rect>,
    pub font_color: Color,
}

pub fn generate_render_command(presenter: &TextLayoutPresenter, style: &TextStyle) -> TextRendererCommand {
    TextRendererCommand {
        glyphs: presenter.get_visible_glyphs(),
        clip_rect: presenter.clip_rect,
        font_color: style.font_color,
    }
}