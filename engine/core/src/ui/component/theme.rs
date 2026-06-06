// component/theme.rs

use crate::ui::layout::style::Style;
use crate::ui::layout::geometry::EdgeInsets;

/// 主题系统，管理全局样式
#[derive(Debug, Clone)]
pub struct Theme {
    /// 主色调
    pub primary_color: (u8, u8, u8),
    /// 次色调
    pub secondary_color: (u8, u8, u8),
    /// 背景色
    pub background_color: (u8, u8, u8),
    /// 文字色
    pub text_color: (u8, u8, u8),
    /// 字体族
    pub font_family: String,
    /// 基础字号
    pub base_font_size: f32,
    /// 基础圆角
    pub border_radius: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            primary_color: (59, 130, 246),
            secondary_color: (107, 114, 128),
            background_color: (255, 255, 255),
            text_color: (31, 41, 55),
            font_family: "system".to_string(),
            base_font_size: 14.0,
            border_radius: 6.0,
        }
    }
}

impl Theme {
    pub fn new() -> Self {
        Theme::default()
    }
    
    /// 主按钮样式
    pub fn primary_button_style(&self) -> Style {
        Style::new()
            .padding(EdgeInsets::symmetric(8.0, 16.0))
            .font_size(self.base_font_size)
            .background(self.primary_color)
            .border_radius(self.border_radius)  // 默认6.0，矩形小圆角
    }
    
    /// 次按钮样式
    pub fn secondary_button_style(&self) -> Style {
        Style::new()
            .padding(EdgeInsets::symmetric(6.0, 12.0))
            .font_size(self.base_font_size * 0.9)
    }
    
    /// 标题文本样式
    pub fn title_text_style(&self) -> Style {
        Style::new()
            .font_size(self.base_font_size * 1.7)
    }
    
    /// 正文文本样式
    pub fn body_text_style(&self) -> Style {
        Style::new()
            .font_size(self.base_font_size)
    }
    
    /// 小文本样式
    pub fn caption_text_style(&self) -> Style {
        Style::new()
            .font_size(self.base_font_size * 0.8)
    }
    
    /// 输入框样式
    pub fn input_style(&self) -> Style {
        Style::new()
            .padding(EdgeInsets::all(8.0))
            .font_size(self.base_font_size)
    }
    
    /// 列表项样式
    pub fn list_item_style(&self) -> Style {
        Style::new()
            .padding(EdgeInsets::symmetric(10.0, 16.0))
            .font_size(self.base_font_size)
    }
}