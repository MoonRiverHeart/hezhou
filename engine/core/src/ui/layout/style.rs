use super::geometry::{EdgeInsets, Alignment};

/// Widget样式
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// 固定宽度（如果设置）
    pub width: Option<f32>,
    /// 固定高度（如果设置）
    pub height: Option<f32>,
    /// 弹性增长因子
    pub flex_grow: f32,
    /// 弹性收缩因子
    pub flex_shrink: f32,
    /// 内边距
    pub padding: EdgeInsets,
    /// 外边距
    pub margin: EdgeInsets,
    /// 主轴对齐（用于Flex容器）
    pub main_alignment: MainAlignment,
    /// 交叉轴对齐
    pub cross_alignment: Alignment,
    /// 字体大小
    pub font_size: Option<f32>,
    /// 背景色 (r, g, b)
    pub background_color: Option<(u8, u8, u8)>,
    /// 圆角半径
    pub border_radius: Option<f32>,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            width: None,
            height: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            padding: EdgeInsets::zero(),
            margin: EdgeInsets::zero(),
            main_alignment: MainAlignment::Start,
            cross_alignment: Alignment::Stretch,
            font_size: None,
            background_color: None,
            border_radius: None,
        }
    }
}

impl Style {
    pub fn new() -> Self {
        Style::default()
    }
    
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
    
    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }
    
    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self
    }
    
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }
    
    pub fn margin(mut self, margin: EdgeInsets) -> Self {
        self.margin = margin;
        self
    }
    
    pub fn cross_alignment(mut self, alignment: Alignment) -> Self {
        self.cross_alignment = alignment;
        self
    }
    
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }
    
    pub fn background(mut self, color: (u8, u8, u8)) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn border_radius(mut self, r: f32) -> Self {
        self.border_radius = Some(r);
        self
    }
}

/// 主轴对齐
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}