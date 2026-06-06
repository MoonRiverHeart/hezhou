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
                data: vec![255u8; 1024 * 1024 * 4], // 初始化为白色(255)，alpha=255表示未使用
                width: 1024,
                height: 1024,
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
        
        // 根据字体大小动态调整 MSDF 分辨率和 spread
        let msdf_size = (size.max(32) as f32 * 0.6).min(32.0) as u32;
        let spread = msdf_size as f32 * 0.2;
        
        let glyph_index = self.get_glyph_index(ch);
        println!("DEBUG msdf glyph: ch='{}' index={}", ch, glyph_index);
        let generator = MsdfGenerator::new(msdf_size, spread);
        let msdf_data = generator.generate(font_data, glyph_index, size as f32);

        let font = if Self::is_cjk(ch) { &self.primary_font } else { &self.secondary_font };
        let (metrics, _) = font.rasterize(ch, size as f32);
        let scale_ratio = msdf_size as f32 / size as f32;
        
        // 检查图集是否还有空间
        if self.atlas_cursor_x + msdf_size > self.atlas.width {
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y += self.atlas_row_height;
            self.atlas_row_height = 0;
        }
        
        // 如果图集满了，重置游标（简单处理，旧缓存失效）
        if self.atlas_cursor_y + msdf_size > self.atlas.height {
            let new_height = self.atlas.height * 2;
            let old_height = self.atlas.height;
            let mut new_data = vec![255u8; (self.atlas.width * new_height * 4) as usize];
            let row_bytes = (self.atlas.width * 4) as usize;
            for y in 0..old_height {
                let src_start = (y as usize) * row_bytes;
                let dst_start = (y as usize) * row_bytes;
                new_data[dst_start..dst_start + row_bytes]
                    .copy_from_slice(&self.atlas.data[src_start..src_start + row_bytes]);
            }
            self.atlas.data = new_data;
            self.atlas.height = new_height;
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y = old_height;
            self.atlas_row_height = 0;
            println!("WARNING: Atlas expanded to {}x{}", self.atlas.width, self.atlas.height);
        }
        
        let atlas_x = self.atlas_cursor_x;
        let atlas_y = self.atlas_cursor_y;
        
        // 填充图集区域为黑色（alpha=0），覆盖旧数据
        for y in 0..msdf_size {
            for x in 0..msdf_size {
                let dst_idx = (((atlas_y + y) * self.atlas.width + atlas_x + x) * 4) as usize;
                if dst_idx + 3 < self.atlas.data.len() {
                    self.atlas.data[dst_idx] = 255;
                    self.atlas.data[dst_idx + 1] = 255;
                    self.atlas.data[dst_idx + 2] = 255;
                    self.atlas.data[dst_idx + 3] = 0; // alpha=0 表示无数据
                }
            }
        }
        
        // 拷贝 MSDF 数据到图集
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
        
        let uv_x0 = atlas_x as f32 / self.atlas.width as f32;
        let uv_y0 = atlas_y as f32 / self.atlas.height as f32;
        let uv_x1 = (atlas_x + msdf_size) as f32 / self.atlas.width as f32;
        let uv_y1 = (atlas_y + msdf_size) as f32 / self.atlas.height as f32;

        let advance_x = if MsdfFont::is_cjk(ch) {
            msdf_size as f32 * 0.8
        } else {
            metrics.advance_width
        };
        
        GlyphInfo {
            character: ch,
            uv: [uv_x0, uv_y0, uv_x1, uv_y1],
            size: Size::new(msdf_size as f32, msdf_size as f32),
            bearing_x: metrics.xmin as f32 * scale_ratio,
            bearing_y: metrics.ymin as f32 * scale_ratio,
            advance_x: advance_x, // 返回原始字体尺寸的 advance
            texture_index: 0,
        }
    }

    /// 并行生成 MSDF 数据，串行写入图集
    pub fn get_or_create_glyphs_parallel(&mut self, chars: &[char], size: u32) -> Vec<GlyphInfo> {
        use rayon::prelude::*;
        
        let needs_generate: Vec<char> = chars.iter()
            .filter(|&&ch| !self.glyph_cache.contains_key(&(ch, size)))
            .copied()
            .collect();
        
        if !needs_generate.is_empty() {
            let primary_data = &self.primary_data;
            let secondary_data = &self.secondary_data;
            let msdf_size = (size.max(32) as f32 * 0.5).min(96.0) as u32;
            let spread = msdf_size as f32 * 0.15;
            
            println!("[perf] msdf_size={}, spread={}", msdf_size, spread);
            
            // 并行生成 MSDF 数据（不需要 Font）
            let t_collect = std::time::Instant::now();
            let msdf_results: Vec<(char, Vec<u8>)> = needs_generate
                .par_iter()
                .map(|&ch| {
                    let font_data = if MsdfFont::is_cjk(ch) { primary_data } else { secondary_data };
                    let face = ttf_parser::Face::parse(font_data, 0).unwrap();
                    let glyph_id = face.glyph_index(ch).map(|g| g.0).unwrap_or(0);
                    let generator = MsdfGenerator::new(msdf_size, spread);
                    let msdf_data = generator.generate(font_data, glyph_id, size as f32);
                    (ch, msdf_data)
                })
                .collect();
            println!("[perf] parallel collect done: {:?} ({} glyphs)", t_collect.elapsed(), msdf_results.len());
            
            // 串行 rasterize + 写入图集
            let t_write = std::time::Instant::now();
            for (ch, msdf_data) in msdf_results {
                let font = if MsdfFont::is_cjk(ch) { &self.primary_font } else { &self.secondary_font };
                let small_size = size.min(64);
                let (metrics_small, _) = font.rasterize(ch, small_size as f32);
                let scale_ratio = size as f32 / small_size as f32;
                let metrics = fontdue::Metrics {
                    xmin: (metrics_small.xmin as f32 * scale_ratio) as i32,
                    ymin: (metrics_small.ymin as f32 * scale_ratio) as i32,
                    width: (metrics_small.width as f32 * scale_ratio) as usize,
                    height: (metrics_small.height as f32 * scale_ratio) as usize,
                    advance_width: metrics_small.advance_width * scale_ratio,
                    advance_height: metrics_small.advance_height * scale_ratio,
                    bounds: metrics_small.bounds,
                };
                let glyph = self.write_to_atlas(ch, size, msdf_size, &msdf_data, metrics);
                self.glyph_cache.insert((ch, size), glyph);
            }
            println!("[perf] write_to_atlas done: {:?}", t_write.elapsed());
        }
        
        chars.iter().map(|&ch| self.glyph_cache[&(ch, size)].clone()).collect()
    }
    
    fn write_to_atlas(&mut self, ch: char, size: u32, msdf_size: u32, msdf_data: &[u8], metrics: fontdue::Metrics) -> GlyphInfo {
        // 图集空间检查（和 rasterize_glyph 中一样）
        if self.atlas_cursor_x + msdf_size > self.atlas.width {
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y += self.atlas_row_height;
            self.atlas_row_height = 0;
        }
        if self.atlas_cursor_y + msdf_size > self.atlas.height {
            let new_height = self.atlas.height * 2;
            let old_height = self.atlas.height;
            let mut new_data = vec![255u8; (self.atlas.width * new_height * 4) as usize];
            let row_bytes = (self.atlas.width * 4) as usize;
            for y in 0..old_height {
                let src_start = (y as usize) * row_bytes;
                let dst_start = (y as usize) * row_bytes;
                new_data[dst_start..dst_start + row_bytes]
                    .copy_from_slice(&self.atlas.data[src_start..src_start + row_bytes]);
            }
            self.atlas.data = new_data;
            self.atlas.height = new_height;
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y = old_height;
            self.atlas_row_height = 0;
            println!("WARNING: Atlas expanded to {}x{}", self.atlas.width, self.atlas.height);
        }
        
        let atlas_x = self.atlas_cursor_x;
        let atlas_y = self.atlas_cursor_y;
        
        // 填充和拷贝
        for y in 0..msdf_size {
            for x in 0..msdf_size {
                let dst_idx = (((atlas_y + y) * self.atlas.width + atlas_x + x) * 4) as usize;
                if dst_idx + 3 < self.atlas.data.len() {
                    self.atlas.data[dst_idx] = 255;
                    self.atlas.data[dst_idx + 1] = 255;
                    self.atlas.data[dst_idx + 2] = 255;
                    self.atlas.data[dst_idx + 3] = 0;
                }
            }
        }
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
        
        let uv_x0 = atlas_x as f32 / self.atlas.width as f32;
        let uv_y0 = atlas_y as f32 / self.atlas.height as f32;
        let uv_x1 = (atlas_x + msdf_size) as f32 / self.atlas.width as f32;
        let uv_y1 = (atlas_y + msdf_size) as f32 / self.atlas.height as f32;
        
        let advance_x = if MsdfFont::is_cjk(ch) {
            msdf_size as f32 * 0.8
        } else {
            msdf_size as f32 * 0.55
        };
        
        GlyphInfo {
            character: ch,
            uv: [uv_x0, uv_y0, uv_x1, uv_y1],
            size: Size::new(msdf_size as f32, msdf_size as f32),
            bearing_x: metrics.xmin as f32,
            bearing_y: metrics.ymin as f32,
            advance_x,
            texture_index: 0,
        }
    }
}