use crate::style::*;
use crate::types::*;
use crate::font_atlas::FontAtlas;
use unicode_segmentation::UnicodeSegmentation;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CharLayoutInfo {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub grapheme_index: usize,
    pub grapheme_start_byte: usize,
    pub grapheme_end_byte: usize,
}

#[repr(C)]
pub struct Canvas {
    commands: Vec<DrawCommand>,
    clip_rect: Option<Rect>,
    transform: Transform,
    opacity: f32,
    font_atlas_ptr: Option<*const FontAtlas>,
    font_index: usize,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            clip_rect: None,
            transform: Transform::identity(),
            opacity: 1.0,
            font_atlas_ptr: None,
            font_index: 0,
        }
    }
    
    pub fn with_font_atlas(font_atlas: *const FontAtlas, font_index: usize) -> Self {
        Self {
            commands: Vec::new(),
            clip_rect: None,
            transform: Transform::identity(),
            opacity: 1.0,
            font_atlas_ptr: Some(font_atlas),
            font_index,
        }
    }
    
    pub fn get_font_atlas(&self) -> Option<&FontAtlas> {
        self.font_atlas_ptr.map(|ptr| unsafe { &*ptr })
    }
    
    pub fn layout_text_for_cursor(&self, text: &str, font_size: f32, container_x: f32, container_y: f32) -> Vec<(f32, f32, f32, usize, usize, usize)> {
        self.layout_text_for_cursor_with_wrap(text, font_size, container_x, container_y, None)
    }
    
    pub fn layout_text_for_cursor_with_wrap(&self, text: &str, font_size: f32, container_x: f32, container_y: f32, wrap_width: Option<f32>) -> Vec<(f32, f32, f32, usize, usize, usize)> {
        if let Some(atlas) = self.get_font_atlas() {
            let max_bearing_y = atlas.get_font_ascent(self.font_index, font_size);
            let line_height = atlas.get_font_line_height(self.font_index, font_size);
            
            let baseline_y = container_y + max_bearing_y;
            
            let mut cursor_x = container_x;
            let mut current_baseline_y = baseline_y;
            let mut results = Vec::new();
            
            for (grapheme_index, (byte_idx, grapheme)) in text.grapheme_indices(true).enumerate() {
                if grapheme == "\n" {
                    results.push((cursor_x, current_baseline_y, 0.0, grapheme_index, byte_idx, byte_idx + 1));
                    cursor_x = container_x;
                    current_baseline_y += line_height;
                    continue;
                }
                
                let mut grapheme_width = 0.0;
                for c in grapheme.chars() {
                    if let Some(info) = atlas.get_char_info(self.font_index, c, font_size) {
                        grapheme_width += info.advance_x;
                    }
                }
                
                if let Some(max_width) = wrap_width {
                    if cursor_x + grapheme_width > container_x + max_width && cursor_x > container_x {
                        cursor_x = container_x;
                        current_baseline_y += line_height;
                    }
                }
                
                let grapheme_end_byte = byte_idx + grapheme.len();
                results.push((cursor_x, current_baseline_y, grapheme_width, grapheme_index, byte_idx, grapheme_end_byte));
                cursor_x += grapheme_width;
            }
            
            results
        } else {
            Vec::new()
        }
    }
    
    pub fn get_max_bearing_y(&self, text: &str, font_size: f32) -> f32 {
        if let Some(atlas) = self.get_font_atlas() {
            atlas.get_font_ascent(self.font_index, font_size)
        } else {
            font_size * 0.75
        }
    }
    
    pub fn get_line_height(&self, font_size: f32) -> f32 {
        if let Some(atlas) = self.get_font_atlas() {
            atlas.get_font_line_height(self.font_index, font_size)
        } else {
            font_size * 1.2
        }
    }
    
    pub fn get_font_height(&self, font_size: f32) -> f32 {
        if let Some(atlas) = self.get_font_atlas() {
            atlas.get_font_height(self.font_index, font_size)
        } else {
            font_size
        }
    }

    pub fn draw_rect(&mut self, bounds: Rect, style: &Style) {
        let final_alpha = style.background_color.a * style.opacity * self.opacity;
        self.commands.push(DrawCommand::Rect {
            bounds: self.transform.transform_point(&bounds.origin()),
            width: bounds.width,
            height: bounds.height,
            fill_color: Color::new(
                style.background_color.r,
                style.background_color.g,
                style.background_color.b,
                final_alpha,
            ),
            stroke_color: if style.border_width > 0.0 && style.border_color.a > 0.0 {
                Some(Color::new(
                    style.border_color.r,
                    style.border_color.g,
                    style.border_color.b,
                    style.border_color.a * style.opacity * self.opacity,
                ))
            } else {
                None
            },
            stroke_width: style.border_width,
            border_radius: style.border_radius,
        });
    }

    pub fn draw_text(&mut self, bounds: Rect, text: &str, style: &TextStyle) {
        self.commands.push(DrawCommand::Text {
            bounds: self.transform.transform_point(&bounds.origin()),
            width: bounds.width,
            height: bounds.height,
            text: text.as_bytes().to_vec(),
            font_size: style.font_size,
            font_color: Color::new(
                style.font_color.r,
                style.font_color.g,
                style.font_color.b,
                style.font_color.a * self.opacity,
            ),
            alignment: style.alignment,
        });
    }

    pub fn draw_image(&mut self, bounds: Rect, texture_id: u64, uv: Rect) {
        self.commands.push(DrawCommand::Image {
            bounds: self.transform.transform_point(&bounds.origin()),
            width: bounds.width,
            height: bounds.height,
            texture_id,
            uv,
        });
    }

    pub fn draw_line(&mut self, start: Point, end: Point, color: Color, width: f32) {
        self.commands.push(DrawCommand::Line {
            start: self.transform.transform_point(&start),
            end: self.transform.transform_point(&end),
            color: Color::new(color.r, color.g, color.b, color.a * self.opacity),
            width,
        });
    }

    pub fn draw_shadow(&mut self, bounds: Rect, shadow: &Shadow) {
        self.commands.push(DrawCommand::Shadow {
            bounds,
            shadow: *shadow,
        });
    }

    pub fn set_clip_rect(&mut self, rect: Rect) {
        self.clip_rect = Some(rect);
        self.commands.push(DrawCommand::ClipRect { rect });
    }

    pub fn clear_clip(&mut self) {
        self.clip_rect = None;
        self.commands.push(DrawCommand::ClearClip);
    }

    pub fn set_transform(&mut self, transform: Transform) {
        self.transform = transform;
        self.commands.push(DrawCommand::SetTransform { transform });
    }

    pub fn reset_transform(&mut self) {
        self.transform = Transform::identity();
        self.commands.push(DrawCommand::ResetTransform);
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity;
    }

    pub fn get_commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Clone)]
pub enum DrawCommand {
    Rect {
        bounds: Point,
        width: f32,
        height: f32,
        fill_color: Color,
        stroke_color: Option<Color>,
        stroke_width: f32,
        border_radius: f32,
    },

    Text {
        bounds: Point,
        width: f32,
        height: f32,
        text: Vec<u8>,
        font_size: f32,
        font_color: Color,
        alignment: TextAlignment,
    },

    Image {
        bounds: Point,
        width: f32,
        height: f32,
        texture_id: u64,
        uv: Rect,
    },

    Line {
        start: Point,
        end: Point,
        color: Color,
        width: f32,
    },

    Shadow {
        bounds: Rect,
        shadow: Shadow,
    },

    ClipRect {
        rect: Rect,
    },

    ClearClip,

    SetTransform {
        transform: Transform,
    },

    ResetTransform,
}
