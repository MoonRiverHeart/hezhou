use super::*;
use crate::ui::layout::widget::TextData;
use crate::ui::layout::geometry::Alignment;

pub struct Text {
    content: String,
    font_size: Option<f32>,
    style: Option<Style>,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Text {
            content: content.into(),
            font_size: None,
            style: None,
        }
    }
    
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
    
    pub fn title(content: impl Into<String>) -> Self {
        Text {
            content: content.into(),
            font_size: Some(18.0),
            style: None,
        }
    }
    
    pub fn body(content: impl Into<String>) -> Self {
        Text {
            content: content.into(),
            font_size: Some(14.0),
            style: None,
        }
    }
    
    pub fn caption(content: impl Into<String>) -> Self {
        Text {
            content: content.into(),
            font_size: Some(12.0),
            style: None,
        }
    }
}

impl Component for Text {
    fn build(&mut self, ctx: &mut BuildContext) -> WidgetId {
        let font_size = self.font_size.unwrap_or(ctx.theme.base_font_size);
        let base_style = ctx.theme.body_text_style()
            .cross_alignment(Alignment::Center);  // 加这行
        let mut style = self.style.clone().unwrap_or(base_style);
        if style.font_size.is_none() {
            style.font_size = Some(font_size);
        }
        
        let text_data = TextData::new(&self.content, font_size);
        let id = ctx.create_node(WidgetType::Text(text_data), style);
        id
    }
}