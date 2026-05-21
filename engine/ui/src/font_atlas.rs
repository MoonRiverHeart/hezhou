use fontdue::Font;
use std::collections::HashMap;
use hezhou_dfx::{LogLevel, DfxSystem};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::OnceLock;

#[derive(Hash, Eq, PartialEq, Clone)]
struct CharacterKey {
    font_index: usize,
    character: char,
    font_size: u32,
}

pub struct CharacterInfo {
    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_w: f32,
    pub uv_h: f32,
    pub width: f32,
    pub height: f32,
    pub advance_x: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
}

pub struct FontAtlas {
    fonts: Vec<Font>,
    font_data: Vec<Vec<u8>>,
    atlas_texture: Vec<u8>,
    atlas_width: u32,
    atlas_height: u32,
    character_cache: HashMap<CharacterKey, CharacterInfo>,
    current_x: u32,
    current_y: u32,
    row_height: u32,
    cached_font_sizes: Vec<u32>,
    atlas_dirty: bool,
}

const PREDEFINED_FONT_SIZES: [u32; 13] = [48, 36, 32, 28, 24, 22, 20, 18, 16, 15, 14, 13, 12];

impl FontAtlas {
    pub fn new() -> Self {
        Self {
            fonts: Vec::new(),
            font_data: Vec::new(),
            atlas_texture: vec![0u8; 4096 * 4096 * 4],
            atlas_width: 4096,
            atlas_height: 4096,
            character_cache: HashMap::new(),
            current_x: 0,
            current_y: 0,
            row_height: 0,
            cached_font_sizes: PREDEFINED_FONT_SIZES.to_vec(),
            atlas_dirty: true, // dirty on init so initial texture upload happens
        }
    }
    
    pub fn get_nearest_cached_font_size(&self, requested_size: f32) -> f32 {
        let requested = requested_size as u32;
        let mut best_size = PREDEFINED_FONT_SIZES[0];
        let mut best_diff = (best_size as i32 - requested as i32).abs();
        
        for size in PREDEFINED_FONT_SIZES.iter() {
            let diff = (*size as i32 - requested as i32).abs();
            if diff < best_diff {
                best_diff = diff;
                best_size = *size;
            }
        }
        
        best_size as f32
    }
    
    pub fn add_font(&mut self, font_data: &[u8]) -> usize {
        let font = Font::from_bytes(font_data, fontdue::FontSettings::default())
            .expect("Failed to parse font");
        
        self.font_data.push(font_data.to_vec());
        self.fonts.push(font);
        
        self.fonts.len() - 1
    }
    
    pub fn get_font_ascent(&self, font_index: usize, font_size: f32) -> f32 {
        if font_index < self.fonts.len() {
            if let Some(metrics) = self.fonts[font_index].horizontal_line_metrics(font_size) {
                metrics.ascent
            } else {
                font_size * 0.75
            }
        } else {
            font_size * 0.75
        }
    }
    
    pub fn get_font_descent(&self, font_index: usize, font_size: f32) -> f32 {
        if font_index < self.fonts.len() {
            if let Some(metrics) = self.fonts[font_index].horizontal_line_metrics(font_size) {
                metrics.descent
            } else {
                font_size * 0.25
            }
        } else {
            font_size * 0.25
        }
    }
    
    pub fn get_font_height(&self, font_index: usize, font_size: f32) -> f32 {
        self.get_font_ascent(font_index, font_size) - self.get_font_descent(font_index, font_size)
    }
    
    pub fn get_font_line_height(&self, font_index: usize, font_size: f32) -> f32 {
        if font_index < self.fonts.len() {
            if let Some(metrics) = self.fonts[font_index].horizontal_line_metrics(font_size) {
                metrics.ascent - metrics.descent + metrics.line_gap
            } else {
                font_size * 1.2
            }
        } else {
            font_size * 1.2
        }
    }
    
    pub fn prerasterize_chars(&mut self, font_index: usize, chars: &str, sizes: &[f32]) {
        if font_index >= self.fonts.len() {
            return;
        }
        
        for size in sizes {
            for c in chars.chars() {
                self.rasterize_char_direct(font_index, c, *size);
            }
        }
    }
    
    pub fn rasterize_char_direct(&mut self, font_index: usize, character: char, font_size: f32) {
        self.atlas_dirty = true;
        if character == ' ' || character == '\t' || character == '\n' || character == '\r' {
            let key = CharacterKey {
                font_index,
                character,
                font_size: font_size as u32,
            };
            
            if !self.character_cache.contains_key(&key) {
                let space_width = font_size * 0.25;
                let info = CharacterInfo {
                    uv_x: 0.97,
                    uv_y: 0.0,
                    uv_w: 0.0,
                    uv_h: 0.0,
                    width: 0.0,
                    height: 0.0,
                    advance_x: space_width,
                    bearing_x: 0.0,
                    bearing_y: 0.0,
                };
                self.character_cache.insert(key, info);
            }
            return;
        }
        
        let key = CharacterKey {
            font_index,
            character,
            font_size: font_size as u32,
        };
        
        if self.character_cache.contains_key(&key) {
            return;
        }
        
        let supersample_scale = 3.0;
        let raster_size = font_size * supersample_scale;
        let (metrics, bitmap) = self.fonts[font_index].rasterize(character, raster_size);
        
        let char_width = metrics.width as u32;
        let char_height = metrics.height as u32;
        
        if char_width == 0 || char_height == 0 || bitmap.is_empty() {
            let info = CharacterInfo {
                uv_x: 0.97,
                uv_y: 0.0,
                uv_w: 0.0,
                uv_h: 0.0,
                width: 0.0,
                height: 0.0,
                advance_x: metrics.advance_width / supersample_scale,
                bearing_x: metrics.bounds.xmin / supersample_scale,
                bearing_y: (metrics.bounds.height + metrics.bounds.ymin) / supersample_scale,
            };
            self.character_cache.insert(key, info);
            return;
        }
        
        if self.current_x + char_width > self.atlas_width {
            self.current_x = 0;
            self.current_y += self.row_height;
            self.row_height = 0;
        }
        
        if self.current_y + char_height > self.atlas_height {
            return;
        }
        
        for y in 0..char_height {
            for x in 0..char_width {
                let atlas_x = self.current_x + x;
                let atlas_y = self.current_y + y;
                let src_idx = y as usize * char_width as usize + x as usize;
                let dst_idx = atlas_y as usize * self.atlas_width as usize * 4 + atlas_x as usize * 4;
                
                if src_idx < bitmap.len() && dst_idx + 3 < self.atlas_texture.len() {
                    let val = bitmap[src_idx];
                    self.atlas_texture[dst_idx] = 255;
                    self.atlas_texture[dst_idx + 1] = 255;
                    self.atlas_texture[dst_idx + 2] = 255;
                    self.atlas_texture[dst_idx + 3] = val;
                }
            }
        }
        
        let bearing_x = metrics.bounds.xmin / supersample_scale;
        let bearing_y = (metrics.bounds.height + metrics.bounds.ymin) / supersample_scale;
        
        let info = CharacterInfo {
            uv_x: self.current_x as f32 / self.atlas_width as f32,
            uv_y: self.current_y as f32 / self.atlas_height as f32,
            uv_w: char_width as f32 / self.atlas_width as f32,
            uv_h: char_height as f32 / self.atlas_height as f32,
            width: char_width as f32 / supersample_scale,
            height: char_height as f32 / supersample_scale,
            advance_x: metrics.advance_width / supersample_scale,
            bearing_x,
            bearing_y,
        };
        
        self.character_cache.insert(key, info);
        
        self.current_x += char_width + 1;
        self.row_height = self.row_height.max(char_height);
    }
    
    fn rasterize_char(&mut self, font_index: usize, character: char, font_size: f32) {
        if character == ' ' || character == '\t' || character == '\n' || character == '\r' {
            let key = CharacterKey {
                font_index,
                character,
                font_size: font_size as u32,
            };
            
            if !self.character_cache.contains_key(&key) {
                let space_width = font_size * 0.25;
                let info = CharacterInfo {
                    uv_x: 0.97,
                    uv_y: 0.0,
                    uv_w: 0.0,
                    uv_h: 0.0,
                    width: 0.0,
                    height: 0.0,
                    advance_x: space_width,
                    bearing_x: 0.0,
                    bearing_y: 0.0,
                };
                self.character_cache.insert(key, info);
            }
            return;
        }
        
        let key = CharacterKey {
            font_index,
            character,
            font_size: font_size as u32,
        };
        
        if self.character_cache.contains_key(&key) {
            return;
        }
        
        let supersample_scale = 3.0;
        let raster_size = font_size * supersample_scale;
        let (metrics, bitmap) = self.fonts[font_index].rasterize(character, raster_size);
        
        let char_width = metrics.width as u32;
        let char_height = metrics.height as u32;
        
        if char_width == 0 || char_height == 0 || bitmap.is_empty() {
            let info = CharacterInfo {
                uv_x: 0.97,
                uv_y: 0.0,
                uv_w: 0.0,
                uv_h: 0.0,
                width: 0.0,
                height: 0.0,
                advance_x: metrics.advance_width / supersample_scale,
                bearing_x: metrics.bounds.xmin / supersample_scale,
                bearing_y: (metrics.bounds.height + metrics.bounds.ymin) / supersample_scale,
            };
            self.character_cache.insert(key, info);
            return;
        }
        
        if self.current_x + char_width > self.atlas_width {
            self.current_x = 0;
            self.current_y += self.row_height;
            self.row_height = 0;
        }
        
        if self.current_y + char_height > self.atlas_height {
            return;
        }
        
        for y in 0..char_height {
            for x in 0..char_width {
                let atlas_x = self.current_x + x;
                let atlas_y = self.current_y + y;
                let src_idx = y as usize * char_width as usize + x as usize;
                let dst_idx = atlas_y as usize * self.atlas_width as usize * 4 + atlas_x as usize * 4;
                
                if src_idx < bitmap.len() && dst_idx + 3 < self.atlas_texture.len() {
                    let val = bitmap[src_idx];
                    self.atlas_texture[dst_idx] = 255;
                    self.atlas_texture[dst_idx + 1] = 255;
                    self.atlas_texture[dst_idx + 2] = 255;
                    self.atlas_texture[dst_idx + 3] = val;
                }
            }
        }
        
        let bearing_x = metrics.bounds.xmin / supersample_scale;
        let bearing_y = (metrics.bounds.height + metrics.bounds.ymin) / supersample_scale;
        
        let info = CharacterInfo {
            uv_x: self.current_x as f32 / self.atlas_width as f32,
            uv_y: self.current_y as f32 / self.atlas_height as f32,
            uv_w: char_width as f32 / self.atlas_width as f32,
            uv_h: char_height as f32 / self.atlas_height as f32,
            width: char_width as f32 / supersample_scale,
            height: char_height as f32 / supersample_scale,
            advance_x: metrics.advance_width / supersample_scale,
            bearing_x,
            bearing_y,
        };
        
        self.character_cache.insert(key, info);
        
        self.current_x += char_width + 1;
        self.row_height = self.row_height.max(char_height);
    }
    
    pub fn get_char_info(&self, font_index: usize, character: char, font_size: f32) -> Option<&CharacterInfo> {
        let cached_size = self.get_nearest_cached_font_size(font_size);
        let key = CharacterKey {
            font_index,
            character,
            font_size: cached_size as u32,
        };
        
        self.character_cache.get(&key)
    }
    
    pub fn get_atlas_texture(&self) -> &[u8] {
        &self.atlas_texture
    }
    
    pub fn is_atlas_dirty(&self) -> bool {
        self.atlas_dirty
    }
    
    pub fn clear_atlas_dirty(&mut self) {
        self.atlas_dirty = false;
    }
    
    pub fn get_atlas_dimensions(&self) -> (u32, u32) {
        (self.atlas_width, self.atlas_height)
    }
    
    pub fn measure_text(&self, font_index: usize, text: &str, font_size: f32) -> (f32, f32) {
        if font_index >= self.fonts.len() || text.is_empty() {
            return (0.0, 0.0);
        }
        
        let mut total_width: f32 = 0.0;
        let mut max_bearing_y: f32 = 0.0;
        let mut max_glyph_bottom: f32 = 0.0;
        
        for character in text.chars() {
            if let Some(info) = self.get_char_info(font_index, character, font_size) {
                total_width += info.advance_x;
                max_bearing_y = max_bearing_y.max(info.bearing_y);
                let glyph_bottom = info.height - info.bearing_y;
                max_glyph_bottom = max_glyph_bottom.max(glyph_bottom);
            }
        }
        
        let total_height = max_bearing_y + max_glyph_bottom;
        
        (total_width, total_height)
    }
    
    pub fn layout_text_left(
        &self,
        font_index: usize,
        text: &str,
        font_size: f32,
        container_x: f32,
        container_y: f32,
        container_height: f32,
        vertical_center: bool,
    ) -> Vec<(f32, f32, usize, usize, f32, f32, f32, f32)> {
        if font_index >= self.fonts.len() || text.is_empty() {
            return Vec::new();
        }
        
        let max_bearing_y = self.get_font_ascent(font_index, font_size);
        
        let (_, text_height) = self.measure_text(font_index, text, font_size);
        
        let baseline_y = if vertical_center {
            container_y + (container_height - text_height) / 2.0 + max_bearing_y
        } else {
            container_y + max_bearing_y
        };
        
        let mut result = Vec::new();
        let mut cursor_x = container_x;
        let mut cursor_y = baseline_y;
        let line_height = self.get_font_line_height(font_index, font_size);
        
        for character in text.chars() {
            if character == '\n' {
                cursor_x = container_x;
                cursor_y += line_height;
                continue;
            }
            
            if let Some(info) = self.get_char_info(font_index, character, font_size) {
                let char_x = cursor_x + info.bearing_x;
                let char_y = cursor_y - info.bearing_y;
                
                result.push((
                    char_x,
                    char_y,
                    info.width as usize,
                    info.height as usize,
                    info.uv_x,
                    info.uv_y,
                    info.uv_w,
                    info.uv_h,
                ));
                
                cursor_x += info.advance_x;
            }
        }
        
        result
    }
    
    pub fn layout_text_centered(
        &self,
        font_index: usize,
        text: &str,
        font_size: f32,
        container_x: f32,
        container_y: f32,
        container_width: f32,
        container_height: f32,
    ) -> Vec<(f32, f32, usize, usize, f32, f32, f32, f32)> {
        if font_index >= self.fonts.len() || text.is_empty() {
            return Vec::new();
        }
        
        let (text_width, text_height) = self.measure_text(font_index, text, font_size);
        
        let max_bearing_y = self.get_font_ascent(font_index, font_size);
        
        let text_top = container_y + (container_height - text_height) / 2.0;
        let baseline_y = text_top + max_bearing_y;
        let start_x = container_x + (container_width - text_width) / 2.0;
        
        let mut result = Vec::new();
        let mut cursor_x = start_x;
        
        for character in text.chars() {
            if let Some(info) = self.get_char_info(font_index, character, font_size) {
                let char_x = cursor_x + info.bearing_x;
                let char_y = baseline_y - info.bearing_y;
                
                result.push((
                    char_x,
                    char_y,
                    info.width as usize,
                    info.height as usize,
                    info.uv_x,
                    info.uv_y,
                    info.uv_w,
                    info.uv_h,
                ));
                
                cursor_x += info.advance_x;
            }
        }
        
        result
    }
}

impl Default for FontAtlas {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_FONT_ATLAS: OnceLock<parking_lot::Mutex<FontAtlas>> = OnceLock::new();

pub fn get_font_atlas() -> &'static parking_lot::Mutex<FontAtlas> {
    GLOBAL_FONT_ATLAS.get_or_init(|| {
        let mut atlas = FontAtlas::new();
        
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        
        let local_font = exe_dir.join("fonts/HarmonyOS_Sans_SC.ttf");
        let dev_font_path = "C:\\Users\\94023\\Documents\\commandline-tools-windows-x64\\command-line-tools\\sdk\\default\\hms\\previewer\\resources\\fonts\\HarmonyOS_Sans_SC.ttf";
        
        let font_path: std::path::PathBuf = if local_font.exists() {
            local_font
        } else if std::path::Path::new(dev_font_path).exists() {
            std::path::PathBuf::from(dev_font_path)
        } else {
            Arc::new(Mutex::new(DfxSystem::new())).lock().get_logger().lock().log(LogLevel::Warn, "FontAtlas", &format!("Font file not found (checked {} and {})", local_font.display(), dev_font_path), file!(), line!());
            return parking_lot::Mutex::new(atlas);
        };
        
        let font_data = std::fs::read(font_path).expect("Failed to read font file");
        let font_index = atlas.add_font(&font_data);
        
        // Only pre-cache ASCII + basic symbols for fast startup.
        // All other characters (including CJK) will be rasterized on-demand.
        let ascii_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_+=[]{}|;:,.<>?/~ _\"'\\";
        let sizes = [48.0, 36.0, 32.0, 28.0, 24.0, 22.0, 20.0, 18.0, 16.0, 15.0, 14.0, 13.0, 12.0];
        
        atlas.prerasterize_chars(font_index, ascii_chars, &sizes);
        
        Arc::new(Mutex::new(DfxSystem::new())).lock().get_logger().lock().log(LogLevel::Info, "FontAtlas", &format!("Pre-rasterized {} ASCII chars at sizes {:?}", ascii_chars.len(), sizes), file!(), line!());
        
        parking_lot::Mutex::new(atlas)
    })
}

/// Ensure all characters in `text` at `font_size` are rasterized in the atlas cache.
/// Call this before rendering text to guarantee all glyphs are available.
/// This is the key function that enables on-demand rasterization of any character,
/// including CJK characters that were not in the startup precache list.
pub fn ensure_chars_rasterized(font_index: usize, text: &str, font_size: f32) {
    let atlas = get_font_atlas();
    let mut atlas_guard = atlas.lock();
    
    let cached_size = atlas_guard.get_nearest_cached_font_size(font_size) as u32;
    
    for character in text.chars() {
        let key = CharacterKey {
            font_index,
            character,
            font_size: cached_size,
        };
        
        // Only rasterize if not already cached
        if !atlas_guard.character_cache.contains_key(&key) {
            atlas_guard.rasterize_char_direct(font_index, character, cached_size as f32);
        }
    }
}

