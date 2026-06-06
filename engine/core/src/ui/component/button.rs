use super::*;
use crate::ui::layout::widget::TextData;
use crate::ui::layout::text::SimpleTextMeasurer;
use crate::ui::layout::text::TextMeasurer;
use crate::ui::layout::geometry::Alignment;
use crate::ui::event::types::UIEvent;
use crate::ui::event::mouse::MouseEventType;

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
    fn build(&mut self, ctx: &mut BuildContext) -> WidgetId {
        let base_style = match self.variant {
            ButtonVariant::Primary => ctx.theme.primary_button_style(),
            ButtonVariant::Secondary => ctx.theme.secondary_button_style(),
            ButtonVariant::Text => ctx.theme.body_text_style(),
        };
        
        let style = self.style.clone().unwrap_or(base_style);
        let font_size = style.font_size.unwrap_or(ctx.theme.base_font_size);
        let padding = style.padding;
        
        // 测量文字尺寸
        let measurer = SimpleTextMeasurer;
        let text_size = measurer.measure_text(&self.label, font_size, f32::MAX);
        
        // Container尺寸 = 文字尺寸 + padding + 额外空间
        let extra_h = font_size * 0.4;
        let extra_w = font_size * 0.6;
        let container_width = text_size.width + padding.left + padding.right + extra_w;
        let container_height = text_size.height + padding.top + padding.bottom + extra_h;
        
        let container_style = style.clone()
            .width(container_width)
            .height(container_height);
        
        let container_id = ctx.create_node(WidgetType::Container, container_style);
        
        let text_data = TextData::new(&self.label, font_size);
        let text_style = Style::new().cross_alignment(Alignment::Center);
        let text_id = ctx.create_node(WidgetType::Text(text_data), text_style);
        ctx.add_child(container_id, text_id);
        
        // 注册点击事件，移出 on_click 所有权
        if let Some(handler) = self.on_click.take() {
            ctx.on_event(container_id, move |event| {
                if let UIEvent::Mouse(mouse_event) = event {
                    if mouse_event.event_type == MouseEventType::Clicked {
                        handler();
                    }
                }
            });
        }
        
        container_id
    }
}