use fontdue::Font;
use std::collections::HashMap;
use super::text::{FontAtlas, GlyphInfo};
use crate::ui::layout::geometry::Size;

pub struct MsdfFont {
    primary_font: Font,      // 主字体（中文）
    secondary_font: Font,    // 副字体（英文）
    glyph_cache: HashMap<(char, u32), GlyphInfo>,
    pub atlas: FontAtlas,
    atlas_cursor_x: u32,
    atlas_cursor_y: u32,
    atlas_row_height: u32,
}

impl MsdfFont {
    pub fn new(primary_data: &[u8], secondary_data: &[u8]) -> Self {
        let primary = Font::from_bytes(primary_data, fontdue::FontSettings::default())
            .expect("Failed to load primary font");
        let secondary = Font::from_bytes(secondary_data, fontdue::FontSettings::default())
            .expect("Failed to load secondary font");
        
        MsdfFont {
            primary_font: primary,
            secondary_font: secondary,
            glyph_cache: HashMap::new(),
            atlas: FontAtlas {
                data: vec![0u8; 2048 * 2048 * 4],
                width: 2048,
                height: 2048,
            },
            atlas_cursor_x: 0,
            atlas_cursor_y: 0,
            atlas_row_height: 0,
        }
    }

    /// 按字体文件名搜索系统字体
    pub fn from_system(font_names: &[&str]) -> Self {
        let font_dirs = if cfg!(target_os = "windows") {
            vec![std::path::PathBuf::from("C:/Windows/Fonts")]
        } else if cfg!(target_os = "macos") {
            vec![
                std::path::PathBuf::from("/System/Library/Fonts"),
                std::path::PathBuf::from("/Library/Fonts"),
                std::path::PathBuf::from("~/Library/Fonts"),
            ]
        } else {
            vec![
                std::path::PathBuf::from("/usr/share/fonts"),
                std::path::PathBuf::from("/usr/local/share/fonts"),
            ]
        };
        
        let find_font = |name: &str| -> Vec<u8> {
            for dir in &font_dirs {
                // 不区分大小写搜索
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                            if filename.to_lowercase().contains(&name.to_lowercase()) {
                                if let Ok(data) = std::fs::read(&path) {
                                    return data;
                                }
                            }
                        }
                    }
                }
            }
            panic!("Font not found: {}", name);
        };
        
        let primary_data = find_font(font_names[0]);
        let secondary_data = find_font(font_names[1]);
        
        MsdfFont::new(&primary_data, &secondary_data)
    }
    
    /// 判断是否为中文字符
    fn is_cjk(ch: char) -> bool {
        ('\u{4E00}'..='\u{9FFF}').contains(&ch)   // CJK统一表意文字
        || ('\u{3400}'..='\u{4DBF}').contains(&ch)  // CJK扩展A
        || ('\u{20000}'..='\u{2A6DF}').contains(&ch) // CJK扩展B
        || ('\u{F900}'..='\u{FAFF}').contains(&ch)   // CJK兼容汉字
    }
    
    fn font_for_char(&self, ch: char) -> &Font {
        if Self::is_cjk(ch) {
            &self.primary_font
        } else {
            &self.secondary_font
        }
    }
    
    pub fn rasterize(&self, ch: char, size: f32) -> (fontdue::Metrics, Vec<u8>) {
        self.font_for_char(ch).rasterize(ch, size)
    }
    
    pub fn get_or_create_glyph(&mut self, ch: char, size: u32) -> GlyphInfo {
        let key = (ch, size);
        if !self.glyph_cache.contains_key(&key) {
            let glyph = self.rasterize_glyph(ch, size);
            self.glyph_cache.insert(key, glyph);
        }
        self.glyph_cache[&key].clone()
    }
    
    fn rasterize_glyph(&mut self, ch: char, size: u32) -> GlyphInfo {
        let font = if Self::is_cjk(ch) { &self.primary_font } else { &self.secondary_font };
        let (metrics, bitmap) = font.rasterize(ch, size as f32);
        
        let msdf_size = 32u32;
        let msdf_data = self.generate_msdf(&bitmap, metrics.width as u32, metrics.height as u32, msdf_size);
        
        let atlas_x = self.atlas_cursor_x;
        let atlas_y = self.atlas_cursor_y;
        
        for y in 0..msdf_size {
            for x in 0..msdf_size {
                let src_idx = ((y * msdf_size + x) * 4) as usize;
                let dst_idx = (((atlas_y + y) * self.atlas.width + atlas_x + x) * 4) as usize;
                self.atlas.data[dst_idx..dst_idx+4].copy_from_slice(&msdf_data[src_idx..src_idx+4]);
            }
        }
        
        self.atlas_cursor_x += msdf_size;
        self.atlas_row_height = self.atlas_row_height.max(msdf_size);
        if self.atlas_cursor_x + msdf_size > self.atlas.width {
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y += self.atlas_row_height;
            self.atlas_row_height = 0;
        }
        
        let uv_x0 = atlas_x as f32 / self.atlas.width as f32;
        let uv_y0 = atlas_y as f32 / self.atlas.height as f32;
        let uv_x1 = (atlas_x + msdf_size) as f32 / self.atlas.width as f32;
        let uv_y1 = (atlas_y + msdf_size) as f32 / self.atlas.height as f32;
        
        GlyphInfo {
            character: ch,
            uv: [uv_x0, uv_y0, uv_x1, uv_y1],
            size: Size::new(msdf_size as f32, msdf_size as f32),
            bearing_x: metrics.xmin as f32,
            bearing_y: metrics.ymin as f32,
            advance_x: metrics.advance_width,
            texture_index: 0,
        }
    }
    
    fn generate_msdf(&self, bitmap: &[u8], src_w: u32, src_h: u32, dst_size: u32) -> Vec<u8> {
        let mut msdf = vec![0u8; (dst_size * dst_size * 4) as usize];
        
        if src_w == 0 || src_h == 0 {
            return msdf;
        }
        
        let scale_x = src_w as f32 / dst_size as f32;
        let scale_y = src_h as f32 / dst_size as f32;
        
        for y in 0..dst_size {
            for x in 0..dst_size {
                let src_x = (x as f32 * scale_x) as u32;
                let src_y = (y as f32 * scale_y) as u32;
                let src_idx = (src_y * src_w + src_x) as usize;
                let alpha = if src_idx < bitmap.len() { bitmap[src_idx] } else { 0 };
                
                let dst_idx = ((y * dst_size + x) * 4) as usize;
                msdf[dst_idx] = 255;
                msdf[dst_idx + 1] = 255;
                msdf[dst_idx + 2] = 255;
                msdf[dst_idx + 3] = alpha;
            }
        }
        
        msdf
    }
}