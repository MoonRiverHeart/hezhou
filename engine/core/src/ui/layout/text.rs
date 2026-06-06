use super::geometry::Size;

pub struct SimpleTextMeasurer;

/// 字形信息
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// 字符
    pub character: char,
    /// 字形在纹理图集中的UV坐标 (u0, v0, u1, v1)
    pub uv: [f32; 4],
    /// 字形在纹理图集中的尺寸（像素）
    pub size: Size,
    /// 字形偏移（相对于基线起点）
    pub bearing_x: f32,
    pub bearing_y: f32,
    /// 前进宽度（到下一个字符的距离）
    pub advance_x: f32,
    /// MSDF纹理索引
    pub texture_index: u32,
}

/// 字体纹理图集
#[derive(Debug, Clone)]
pub struct FontAtlas {
    /// 纹理图集的像素数据（RGBA）
    pub data: Vec<u8>,
    /// 纹理图集宽度
    pub width: u32,
    /// 纹理图集高度
    pub height: u32,
}

/// 文本布局结果
#[derive(Debug, Clone)]
pub struct TextLayout {
    /// 总尺寸
    pub size: Size,
    /// 每个字符的字形信息
    pub glyphs: Vec<GlyphInfo>,
}

/// 文本测量和字形生成trait
pub trait TextMeasurer {
    fn measure_text(&self, text: &str, font_size: f32, max_width: f32) -> Size;
    fn layout_text(&mut self, text: &str, font_size: f32, max_width: f32) -> TextLayout;
    fn font_atlas(&self) -> &FontAtlas;
    fn scale_for_size(&self, font_size: f32) -> f32;
}

impl TextMeasurer for SimpleTextMeasurer {
    fn measure_text(&self, text: &str, font_size: f32, _max_width: f32) -> Size {
        let char_width = font_size * 0.6;
        let width = text.len() as f32 * char_width;
        let height = font_size * 1.2;
        Size::new(width, height)
    }
    
    fn layout_text(&mut self, text: &str, font_size: f32, _max_width: f32) -> TextLayout {
        let size = self.measure_text(text, font_size, _max_width);
        TextLayout {
            size,
            glyphs: vec![],
        }
    }
    
    fn font_atlas(&self) -> &FontAtlas {
        // 返回空图集，实际使用中不会调用
        unimplemented!()
    }
    
    fn scale_for_size(&self, font_size: f32) -> f32 {
        font_size
    }
}