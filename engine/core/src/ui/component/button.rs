use super::*;
use crate::ui::layout::widget::TextData;
use crate::ui::layout::text::SimpleTextMeasurer;
use crate::ui::layout::text::TextMeasurer;
use crate::ui::layout::geometry::Alignment;
use crate::ui::layout::geometry::EdgeInsets;
use std::sync::{Arc, Mutex};

pub enum ButtonVariant {
    Primary,
    Secondary,
    Text,
}

pub enum ButtonLabel {
    Static(String),
    Dynamic(Arc<Mutex<String>>),
}

pub struct Button {
    label: ButtonLabel,
    variant: ButtonVariant,
    on_click: Option<Box<dyn Fn()>>,
    style: Option<Style>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Button {
            label: ButtonLabel::Static(label.into()),
            variant: ButtonVariant::Primary,
            on_click: None,
            style: None,
        }
    }
    
    pub fn dynamic(label: Arc<Mutex<String>>) -> Self {
        Button {
            label: ButtonLabel::Dynamic(label),
            variant: ButtonVariant::Primary,
            on_click: None,
            style: None,
        }
    }
    
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
    
    fn get_label(&self) -> String {
        match &self.label {
            ButtonLabel::Static(s) => s.clone(),
            ButtonLabel::Dynamic(arc) => arc.lock().unwrap().clone(),
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        let mut s = self.style.unwrap_or_default();
        s.font_size = Some(size);
        self.style = Some(s);
        self
    }
}
impl Component for Button {
    fn build(&mut self, ctx: &mut BuildContext) -> WidgetId {
        let base_style = match self.variant {
            ButtonVariant::Primary => ctx.theme.primary_button_style(),
            ButtonVariant::Secondary => ctx.theme.secondary_button_style(),
            ButtonVariant::Text => ctx.theme.body_text_style(),
        };
        
        let mut style = base_style.clone();
        if let Some(s) = &self.style {
            if let Some(w) = s.width { style.width = Some(w); }
            if let Some(h) = s.height { style.height = Some(h); }
            if s.font_size.is_some() { style.font_size = s.font_size; }
            if s.background_color.is_some() { style.background_color = s.background_color; }
            if s.padding != EdgeInsets::zero() { style.padding = s.padding; }
            if s.border_radius.is_some() { style.border_radius = s.border_radius; }
        }
        
        let font_size = style.font_size.unwrap_or(ctx.theme.base_font_size);
        let label = self.get_label();
        let padding = style.padding;
        
        // 先生成 MSDF 字形，用于计算实际尺寸
        let glyphs = ctx.layout_text(&label, font_size);
        
        let container_width;
        let container_height;
        if let Some(ref g) = glyphs {
            let total_w: f32 = g.iter().map(|g| g.advance_x).sum();
            let max_h: f32 = g.iter().map(|g| g.size.height).fold(0.0, f32::max);
            container_width = total_w + padding.left + padding.right + font_size * 0.6;
            container_height = max_h + padding.top + padding.bottom + font_size * 0.4;
        } else {
            let measurer = SimpleTextMeasurer;
            let text_size = measurer.measure_text(&label, font_size, f32::MAX);
            container_width = text_size.width + padding.left + padding.right + font_size * 0.6;
            container_height = text_size.height + padding.top + padding.bottom + font_size * 0.4;
        }
        
        let container_style = style.width(container_width).height(container_height);
        let container_id = ctx.create_node(WidgetType::Container, container_style);
        
        let text_data = if let Some(g) = glyphs {
            TextData::with_glyphs(&label, font_size, g)
        } else {
            TextData::new(&label, font_size)
        };
        let text_style = Style::new().cross_alignment(Alignment::Center);
        let text_id = ctx.create_node(WidgetType::Text(text_data), text_style);
        ctx.add_child(container_id, text_id);
        
        if let Some(handler) = self.on_click.take() {
            ctx.on_event(container_id, move |_| {
                handler();
            });
        }
        
        container_id
    }
}