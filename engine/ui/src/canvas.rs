use crate::types::*;
use crate::style::*;

#[repr(C)]
pub struct Canvas {
    commands: Vec<DrawCommand>,
    clip_rect: Option<Rect>,
    transform: Transform,
    opacity: f32,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            clip_rect: None,
            transform: Transform::identity(),
            opacity: 1.0,
        }
    }
    
    pub fn draw_rect(&mut self, bounds: Rect, style: &Style) {
        self.commands.push(DrawCommand::Rect {
            bounds: self.transform.transform_point(&bounds.origin()),
            width: bounds.width,
            height: bounds.height,
            fill_color: style.background_color.with_alpha(style.opacity * self.opacity),
            stroke_color: if style.border_width > 0.0 {
                Some(style.border_color.with_alpha(style.opacity * self.opacity))
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
            text: text.as_ptr(),
            text_len: text.len(),
            font_size: style.font_size,
            font_color: style.font_color.with_alpha(self.opacity),
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
            color: color.with_alpha(self.opacity),
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
        text: *const u8,
        text_len: usize,
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