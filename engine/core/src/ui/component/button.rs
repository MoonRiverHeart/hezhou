use super::*;
use crate::ui::layout::widget::TextData;
use crate::ui::event::types::UIEvent;
use crate::ui::event::mouse::MouseEventType;
use crate::ui::layout::geometry::Alignment;

pub enum ButtonVariant {
    Primary,
    Secondary,
    Text,
}

pub struct Button {
    label: String,
    variant: ButtonVariant,
    on_click: Option<Box<dyn Fn()>>,
    style: Option<Style>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Button {
            label: label.into(),
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
}

impl Component for Button {
    fn build(&self, ctx: &mut BuildContext) -> WidgetId {
        let base_style = match self.variant {
            ButtonVariant::Primary => ctx.theme.primary_button_style(),
            ButtonVariant::Secondary => ctx.theme.secondary_button_style(),
            ButtonVariant::Text => ctx.theme.body_text_style(),
        };
        
        let style = self.style.clone().unwrap_or(base_style);
       let container_id = ctx.create_node(WidgetType::Container, 
            style.clone().cross_alignment(Alignment::Stretch)  // 让Container被Column拉伸
        );
        
        let font_size = style.font_size.unwrap_or(ctx.theme.base_font_size);
        let text_data = TextData::new(&self.label, font_size);
        let text_style = Style::new()
            .cross_alignment(Alignment::Center);  // 交叉轴居中
        let text_id = ctx.create_node(WidgetType::Text(text_data), text_style);
        ctx.add_child(container_id, text_id);
        
        if self.on_click.is_some() {
            ctx.on_event(container_id, move |event| {
                if let UIEvent::Mouse(mouse_event) = event {
                    if mouse_event.event_type == MouseEventType::Clicked {
                    }
                }
            });
        }
        
        container_id
    }
}