use super::text::{TextMeasurer, TextLayout, FontAtlas};
use super::font::MsdfFont;
use super::geometry::Size;

/// MSDF 文本测量器
pub struct MsdfTextMeasurer {
    font: MsdfFont,
    scale: f32,
}

impl MsdfTextMeasurer {
    pub fn from_files(cn_path: &str, en_path: &str) -> Self {
        MsdfTextMeasurer {
            font: MsdfFont::new(
                &std::fs::read(cn_path).expect("Failed to read CN font"),
                &std::fs::read(en_path).expect("Failed to read EN font"),
            ),
            scale: 1.0,
        }
    }

    /// 从系统字体创建，传入字体文件名关键词
    pub fn from_system(cn_font: &str, en_font: &str) -> Self {
        MsdfTextMeasurer {
            font: MsdfFont::from_system(&[cn_font, en_font]),
            scale: 1.0,
        }
    }
}

impl TextMeasurer for MsdfTextMeasurer {
    fn measure_text(&self, text: &str, font_size: f32, _max_width: f32) -> Size {
        let scale = self.scale_for_size(font_size);
        let mut total_width = 0.0f32;
        let mut max_height = 0.0f32;
        
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, scale);
            total_width += metrics.advance_width;
            max_height = max_height.max(metrics.height as f32);
        }
        
        Size::new(total_width, max_height)
    }
    
    fn layout_text(&mut self, text: &str, font_size: f32, _max_width: f32) -> TextLayout {
        let scale = self.scale_for_size(font_size);
        let mut glyphs = Vec::new();
        let mut total_width = 0.0f32;
        let mut max_height = 0.0f32;
        
        for ch in text.chars() {
            let glyph = self.font.get_or_create_glyph(ch, scale as u32);
            total_width += glyph.advance_x;
            max_height = max_height.max(glyph.size.height);
            glyphs.push(glyph);
        }
        
        TextLayout {
            size: Size::new(total_width, max_height),
            glyphs,
        }
    }
    
    fn font_atlas(&self) -> &FontAtlas {
        &self.font.atlas
    }
    
    fn scale_for_size(&self, font_size: f32) -> f32 {
        font_size * self.scale
    }
}