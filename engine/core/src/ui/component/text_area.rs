use super::*;
use crate::ui::layout::widget::TextData;
use crate::ui::event::types::UIEvent;
use crate::ui::event::keyboard::KeyboardEventType;

pub struct TextArea {
    placeholder: String,
    value: String,
    rows: u32,
    on_change: Option<Box<dyn Fn(&str)>>,
    style: Option<Style>,
}

impl TextArea {
    pub fn new(placeholder: impl Into<String>) -> Self {
        TextArea {
            placeholder: placeholder.into(),
            value: String::new(),
            rows: 4,
            on_change: None,
            style: None,
        }
    }
    
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = rows;
        self
    }
    
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
    
    pub fn on_change(mut self, handler: impl Fn(&str) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl Component for TextArea {
    fn build(&mut self, ctx: &mut BuildContext) -> WidgetId {
        let style = self.style.clone().unwrap_or_else(|| ctx.theme.input_style());
        
        let font_size = style.font_size.unwrap_or(ctx.theme.base_font_size);
        let line_height = font_size * 1.5;
        let estimated_height = self.rows as f32 * line_height;
        
        let container_id = ctx.create_node(WidgetType::Container,
            Style::new()
                .padding(style.padding)
                .width(style.width.unwrap_or(300.0))
                .height(estimated_height)
        );
        
        let display_text = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };
        
        let text_data = TextData::new(display_text, font_size);
        let text_id = ctx.create_node(WidgetType::Text(text_data), Style::new());
        ctx.add_child(container_id, text_id);
        
        // 注册键盘事件
        if self.on_change.is_some() {
            ctx.on_event(container_id, move |event| {
                if let UIEvent::Keyboard(keyboard_event) = event {
                    if let KeyboardEventType::TextInput { ref text } = keyboard_event.event_type {
                        // 需要在运行时处理
                    }
                }
            });
        }
        
        container_id
    }
}