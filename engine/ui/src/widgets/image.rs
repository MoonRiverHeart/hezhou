use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::thunk::{queue_callback, PendingCallback};

#[derive(Clone, Copy, PartialEq)]
pub enum ImageScaleMode {
    Fill,   // Scale to fill entire bounds (may crop)
    Fit,    // Scale to fit within bounds (may have empty space)
    Center, // Original size, centered
    Crop,   // Scale to fill, crop overflow
}

pub struct Image {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    texture_id: u64,
    uv: Rect,
    scale_mode: ImageScaleMode,
    content_scale: f32,
}

impl Image {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, width, height),
            style: Style::new(),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            texture_id: 0,
            uv: Rect::new(0.0, 0.0, 1.0, 1.0),
            scale_mode: ImageScaleMode::Fit,
            content_scale: 1.0,
        }
    }

    pub fn set_texture_id(&mut self, texture_id: u64) {
        self.texture_id = texture_id;
        self.flags.dirty_render = true;
    }

    pub fn get_texture_id(&self) -> u64 {
        self.texture_id
    }

    pub fn set_uv(&mut self, uv: Rect) {
        self.uv = uv;
        self.flags.dirty_render = true;
    }

    pub fn set_scale_mode(&mut self, mode: ImageScaleMode) {
        self.scale_mode = mode;
        self.flags.dirty_render = true;
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.flags.dirty_render = true;
    }
}

impl Widget for Image {
    fn id(&self) -> WidgetId { self.id }
    fn parent(&self) -> Option<WidgetId> {
        if self.parent_id.is_valid() { Some(self.parent_id) } else { None }
    }
    fn set_parent(&mut self, parent: WidgetId) { self.parent_id = parent; }
    fn children(&self) -> &[WidgetId] { &self.children }
    fn add_child(&mut self, child: WidgetId) { self.children.push(child); }
    fn remove_child(&mut self, child: WidgetId) { self.children.retain(|c| *c != child); }
    fn layout(&self) -> &Layout { &self.layout }
    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }
    fn style(&self) -> &Style { &self.style }
    fn set_style(&mut self, style: Style) {
        self.style = style;
        self.flags.dirty_style = true;
        self.flags.dirty_render = true;
    }
    fn state(&self) -> WidgetState { self.state }
    fn set_state(&mut self, state: WidgetState) {
        self.state = state;
        self.flags.dirty_render = true;
    }
    fn widget_type(&self) -> &'static str { "Image" }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn flags(&self) -> WidgetFlags { self.flags }
    fn set_flags(&mut self, flags: WidgetFlags) { self.flags = flags; }

    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        (self.layout.width, self.layout.height)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        if self.texture_id == 0 {
            // No texture — draw placeholder background
            canvas.draw_rect(
                Rect::new(0.0, 0.0, self.layout.width, self.layout.height),
                &Style::new().with_background(Color::new(0.15, 0.15, 0.18, 1.0)),
            );
            return;
        }

        let width = self.layout.width;
        let height = self.layout.height;

        // Calculate draw rect based on scale_mode
        let (draw_x, draw_y, draw_w, draw_h) = match self.scale_mode {
            ImageScaleMode::Fill | ImageScaleMode::Crop => {
                // Fill entire bounds
                (0.0, 0.0, width, height)
            }
            ImageScaleMode::Fit => {
                // Fit within bounds — maintain aspect ratio (use UV to determine aspect)
                // For now just fill the bounds since we don't know actual image dimensions
                (0.0, 0.0, width, height)
            }
            ImageScaleMode::Center => {
                // Center at original size — use bounds as-is for now
                (0.0, 0.0, width, height)
            }
        };

        canvas.draw_image(
            Rect::new(draw_x, draw_y, draw_w, draw_h),
            self.texture_id,
            self.uv,
        );
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                // Image click — trigger global_click so C# can handle screenshot preview
                self.flags.dirty_render = true;
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}