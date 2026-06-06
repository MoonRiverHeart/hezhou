use super::*;
use crate::ui::layout::widget::TextData;
use crate::ui::layout::text::SimpleTextMeasurer;
use crate::ui::layout::text::TextMeasurer;
use crate::ui::layout::geometry::Alignment;
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
}

impl Component for Button {
    fn build(&mut self, ctx: &mut BuildContext) -> WidgetId {
        let base_style = match self.variant {
            ButtonVariant::Primary => ctx.theme.primary_button_style(),
            ButtonVariant::Secondary => ctx.theme.secondary_button_style(),
            ButtonVariant::Text => ctx.theme.body_text_style(),
        };
        
        let style = self.style.clone().unwrap_or(base_style);
        let font_size = style.font_size.unwrap_or(ctx.theme.base_font_size);
        let padding = style.padding;
        
        let label = self.get_label();
        let measurer = SimpleTextMeasurer;
        let text_size = measurer.measure_text(&label, font_size, f32::MAX);
        
        let extra_h = font_size * 0.4;
        let extra_w = font_size * 0.6;
        let container_width = text_size.width + padding.left + padding.right + extra_w;
        let container_height = text_size.height + padding.top + padding.bottom + extra_h;
        
        let container_style = style.clone()
            .width(container_width)
            .height(container_height);
        
        let container_id = ctx.create_node(WidgetType::Container, container_style);
        
        let text_data = TextData::new(&label, font_size);
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