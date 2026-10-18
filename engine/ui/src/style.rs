use crate::theme;
use crate::types::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub background_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub border_radius: f32,
    pub opacity: f32,
    pub shadow: Option<Shadow>,
}

impl Style {
    pub fn new() -> Self {
        Self {
            background_color: Color::transparent(),
            border_color: Color::transparent(),
            border_width: 0.0,
            border_radius: 0.0,
            opacity: 1.0,
            shadow: None,
        }
    }

    pub fn with_background(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    pub fn with_border(mut self, color: Color, width: f32, radius: f32) -> Self {
        self.border_color = color;
        self.border_width = width;
        self.border_radius = radius;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TextStyle {
    pub font_size: f32,
    pub font_color: Color,
    pub font_weight: FontWeight,
    pub alignment: TextAlignment,
}

impl TextStyle {
    pub fn new() -> Self {
        Self {
            font_size: 14.0,
            font_color: theme::colors().text_primary,
            font_weight: FontWeight::Normal,
            alignment: TextAlignment::default(),
        }
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.font_color = color;
        self
    }

    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }

    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Light,
    Normal,
    Medium,
    Bold,
    Heavy,
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::Normal
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ButtonStyle {
    pub normal: Style,
    pub hovered: Style,
    pub pressed: Style,
    pub disabled: Style,
}

impl ButtonStyle {
    pub fn new() -> Self {
        let t = theme::colors();
        Self {
            normal: Style::new().with_background(t.button_primary_bg),
            hovered: Style::new().with_background(t.button_primary_hover),
            pressed: Style::new().with_background(t.button_primary_pressed),
            disabled: Style::new().with_background(t.button_secondary_bg).with_opacity(0.6),
        }
    }

    pub fn secondary() -> Self {
        let t = theme::colors();
        Self {
            normal: Style::new().with_background(t.button_secondary_bg),
            hovered: Style::new().with_background(t.button_secondary_hover),
            pressed: Style::new().with_background(t.button_secondary_pressed),
            disabled: Style::new().with_background(t.button_secondary_bg).with_opacity(0.5),
        }
    }

    pub fn material() -> Self {
        let t = theme::colors();
        Self {
            normal: Style::new()
                .with_background(t.button_primary_bg)
                .with_border(Color::transparent(), 0.0, 4.0),
            hovered: Style::new()
                .with_background(t.button_primary_hover)
                .with_border(Color::transparent(), 0.0, 4.0),
            pressed: Style::new()
                .with_background(t.button_primary_pressed)
                .with_border(Color::transparent(), 0.0, 4.0),
            disabled: Style::new()
                .with_background(t.button_secondary_bg)
                .with_border(Color::transparent(), 0.0, 4.0),
        }
    }
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self::new()
    }
}
