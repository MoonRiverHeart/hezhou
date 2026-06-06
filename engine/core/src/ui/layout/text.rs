use super::geometry::Size;

#[derive(Debug, Clone)]
pub struct GlyphInfo {
    pub character: char,
    pub uv: [f32; 4],
    pub size: Size,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance_x: f32,
    pub texture_index: u32,
}

#[derive(Debug, Clone)]
pub struct FontAtlas {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct TextLayout {
    pub size: Size,
    pub glyphs: Vec<GlyphInfo>,
}

pub trait TextMeasurer {
    fn measure_text(&self, text: &str, font_size: f32, max_width: f32) -> Size;
    fn layout_text(&mut self, text: &str, font_size: f32, max_width: f32) -> TextLayout;
    fn font_atlas(&self) -> &FontAtlas;
    fn scale_for_size(&self, font_size: f32) -> f32;
}

pub struct SimpleTextMeasurer;

impl SimpleTextMeasurer {
    pub fn new() -> Self {
        SimpleTextMeasurer
    }
}

impl TextMeasurer for SimpleTextMeasurer {
    fn measure_text(&self, text: &str, font_size: f32, _max_width: f32) -> Size {
        let char_width = font_size * 0.6;
        let width = text.len() as f32 * char_width;
        let height = font_size * 1.2;
        Size::new(width, height)
    }
    
    fn layout_text(&mut self, text: &str, font_size: f32, max_width: f32) -> TextLayout {
        TextLayout {
            size: self.measure_text(text, font_size, max_width),
            glyphs: vec![],
        }
    }
    
    fn font_atlas(&self) -> &FontAtlas {
        // 返回空图集占位
        unimplemented!()
    }
    
    fn scale_for_size(&self, font_size: f32) -> f32 {
        font_size
    }
}