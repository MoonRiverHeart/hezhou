use super::text::{TextMeasurer, TextLayout, FontAtlas};
use super::font::MsdfFont;
use super::geometry::Size;

pub struct MsdfTextMeasurer {
    font: MsdfFont,
}

impl MsdfTextMeasurer {
    pub fn from_system(cn_font: &str, en_font: &str) -> Self {
        MsdfTextMeasurer {
            font: MsdfFont::from_system(&[cn_font, en_font]),
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
        let chars: Vec<char> = text.chars().collect();
        let size = font_size as u32;
        let glyphs = self.font.get_or_create_glyphs_parallel(&chars, size);
        
        let total_width: f32 = glyphs.iter().map(|g| g.advance_x).sum();
        let max_height: f32 = glyphs.iter()
            .map(|g| g.size.height)
            .fold(0.0f32, f32::max);
        
        TextLayout {
            size: Size::new(total_width, max_height),
            glyphs,
        }
    }
    
    fn font_atlas(&self) -> &FontAtlas {
        &self.font.atlas
    }
    
    fn scale_for_size(&self, font_size: f32) -> f32 {
        font_size
    }
}