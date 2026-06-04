use super::geometry::Size;

/// 文本测量trait：布局引擎通过这个接口获取文本尺寸
pub trait TextMeasurer {
    /// 测量文本在给定字体大小和最大宽度下的尺寸
    fn measure_text(&self, text: &str, font_size: f32, max_width: f32) -> Size;
}

/// 简单的字符计数测量器（用于测试和演示）
pub struct SimpleTextMeasurer;

impl TextMeasurer for SimpleTextMeasurer {
    fn measure_text(&self, text: &str, font_size: f32, _max_width: f32) -> Size {
        // 简单估算：每个字符宽度约为字体大小的0.6倍
        let char_width = font_size * 0.6;
        let width = text.len() as f32 * char_width;
        let height = font_size * 1.2; // 行高
        
        Size::new(width, height)
    }
}