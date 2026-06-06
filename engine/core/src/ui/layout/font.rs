use fontdue::Font;
use ttf_parser::Face;
use std::collections::HashMap;
use super::text::{FontAtlas, GlyphInfo};
use super::msdf::MsdfGenerator;
use crate::ui::layout::geometry::Size;

pub struct MsdfFont {
    primary_font: Font,
    secondary_font: Font,
    primary_data: Vec<u8>,
    secondary_data: Vec<u8>,
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
            primary_data: primary_data.to_vec(),
            secondary_data: secondary_data.to_vec(),
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
    
    pub fn from_file(path: &str) -> Self {
        let data = std::fs::read(path).expect("Failed to read font file");
        Self::new(&data, &data)
    }
    
    pub fn from_system(font_names: &[&str]) -> Self {
        let font_dirs = if cfg!(target_os = "windows") {
            vec![std::path::PathBuf::from("C:/Windows/Fonts")]
        } else if cfg!(target_os = "macos") {
            vec![
                std::path::PathBuf::from("/System/Library/Fonts"),
                std::path::PathBuf::from("/Library/Fonts"),
            ]
        } else {
            vec![
                std::path::PathBuf::from("/usr/share/fonts"),
                std::path::PathBuf::from("/usr/local/share/fonts"),
            ]
        };
        
        let find_font = |name: &str| -> Vec<u8> {
            for dir in &font_dirs {
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
    
    pub fn is_cjk(ch: char) -> bool {
        ('\u{4E00}'..='\u{9FFF}').contains(&ch)
        || ('\u{3400}'..='\u{4DBF}').contains(&ch)
        || ('\u{F900}'..='\u{FAFF}').contains(&ch)
        || ('\u{3000}'..='\u{303F}').contains(&ch)
        || ('\u{FF00}'..='\u{FFEF}').contains(&ch)
    }
    
    pub fn rasterize(&self, ch: char, size: f32) -> (fontdue::Metrics, Vec<u8>) {
        if Self::is_cjk(ch) {
            self.primary_font.rasterize(ch, size)
        } else {
            self.secondary_font.rasterize(ch, size)
        }
    }
    
    fn get_glyph_index(&self, ch: char) -> u16 {
        let data = if Self::is_cjk(ch) { &self.primary_data } else { &self.secondary_data };
        let face = Face::parse(data, 0).unwrap();
        face.glyph_index(ch).map(|g| g.0).unwrap_or(0)
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
        let font_data = if Self::is_cjk(ch) {
            &self.primary_data
        } else {
            &self.secondary_data
        };
        
        let glyph_index = self.get_glyph_index(ch);
        let generator = MsdfGenerator::new(32, 8.0);
        let msdf_data = generator.generate(font_data, glyph_index, size as f32);
        
        let font = if Self::is_cjk(ch) { &self.primary_font } else { &self.secondary_font };
        let (metrics, _) = font.rasterize(ch, size as f32);
        
        let msdf_size = 32u32;
        let atlas_x = self.atlas_cursor_x;
        let atlas_y = self.atlas_cursor_y;
        
        for y in 0..msdf_size {
            for x in 0..msdf_size {
                let src_idx = ((y * msdf_size + x) * 4) as usize;
                let dst_idx = (((atlas_y + y) * self.atlas.width + atlas_x + x) * 4) as usize;
                if dst_idx + 3 < self.atlas.data.len() && src_idx + 3 < msdf_data.len() {
                    self.atlas.data[dst_idx..dst_idx+4].copy_from_slice(&msdf_data[src_idx..src_idx+4]);
                }
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
}