use fontdue::Font;
use std::collections::HashMap;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::OnceLock;
use swash::scale::ScaleContext;
use swash::scale::image::Content;
use swash::scale::{Render, Source, StrikeWith};
use swash::zeno::Format;
use swash::FontRef;

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
    pub is_color: bool,
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
    swash_fonts: Vec<FontRef<'static>>,
    scale_context: ScaleContext,
}

const PREDEFINED_FONT_SIZES: [u32; 13] = [48, 36, 32, 28, 24, 22, 20, 18, 16, 15, 14, 13, 12];

impl FontAtlas {
pub fn new() -> Self {
        Self {
            fonts: Vec::new(),
            font_data: Vec::new(),
            atlas_texture: vec![0u8; 8192 * 8192 * 4],
atlas_width: 8192,
        atlas_height: 8192,
            character_cache: HashMap::new(),
            current_x: 0,
            current_y: 0,
            row_height: 0,
            cached_font_sizes: PREDEFINED_FONT_SIZES.to_vec(),
            atlas_dirty: true, // dirty on init so initial texture upload happens
            swash_fonts: Vec::new(),
            scale_context: ScaleContext::new(),
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
        
        // Create swash FontRef from the same font data (leak for 'static lifetime)
        let static_data: &'static [u8] = Box::leak(font_data.to_vec().into_boxed_slice());
        if let Some(swash_font) = FontRef::from_index(static_data, 0) {
            self.swash_fonts.push(swash_font);
        } else {
            dfx_warn!("FontAtlas", "Failed to create swash FontRef for font index {}", self.fonts.len() - 1);
        }
        
        self.fonts.len() - 1
    }
    
    /// Check if a character is a COLOR glyph (emoji) that should use swash rendering.
    /// Only returns true for characters in known emoji Unicode ranges AND when the font
    /// has COLR/color bitmap tables. CJK and other regular text always use fontdue.
    /// 
    /// Previous bug: checked only if FONT has COLR tables, routing ALL characters (including
    /// CJK like 几何位置渲染运动物理) to swash. This caused metric mismatches between
    /// fontdue's ascent (used in layout baseline) and swash's bearing_y, making some
    /// tab text invisible.
    fn is_color_glyph(&self, font_index: usize, character: char) -> bool {
        if font_index >= self.swash_fonts.len() { return false; }
        let font = &self.swash_fonts[font_index];
        let glyph_id = font.charmap().map(character);
        if glyph_id == 0 { return false; }
        
        // Check if font has COLR table or color bitmap strikes
        let has_colr = font.table(swash::tag_from_bytes(b"COLR")).is_some();
        let has_color_bitmaps = font.color_strikes().next().is_some();
        
        if !has_colr && !has_color_bitmaps {
            return false; // Font doesn't support color glyphs at all
        }
        
        // Font has color tables — now check if THIS CHARACTER is likely an emoji
        // Only route characters in known emoji Unicode ranges to swash
        // CJK (U+4E00-U+9FFF), Latin, etc. always use fontdue for correct metrics
        let cp = character as u32;
        
        // Emoji ranges that typically have COLR/CBDT representations
        const EMOJI_RANGES: [(u32, u32); 9] = [
            (0x1F000, 0x1FFFF), // Emoji blocks (Emoticons 😀, Misc 📁📄, Transport, Supplemental)
            (0x2600,  0x26FF),   // Misc symbols (★●■▶◀)
            (0x2700,  0x27BF),   // Dingbats (✓✗)
            (0x2300,  0x23FF),   // Misc Technical (⌛⏳⏰⏱)
            (0x2B50,  0x2B55),   // Stars ⭐, circles ⭕
            (0x25AA,  0x25FE),   // Small shapes (▪▫◾◼◻◽▪▬)
            (0xFE00,  0xFE0F),   // Variation Selectors (emoji presentation)
            (0xE0020, 0xE007F), // Tags (for compound emoji like 🏴󠁧󠁢󠁥󠁮󠁧)
            (0x200D,  0x200D),   // Zero Width Joiner (compound emoji: 👨‍👩‍👧)
        ];
        
        for (start, end) in &EMOJI_RANGES {
            if cp >= *start && cp <= *end {
                return true; // Character is in an emoji range — try swash for color glyph
            }
        }
        
        // Character is NOT in an emoji range (CJK, Latin, arrows, punctuation, etc.)
        // Use fontdue for correct metrics and 3x supersampling quality
        false
    }
    
    /// Rasterize a color emoji glyph using swash, writing RGBA data into the atlas.
    fn rasterize_color_glyph(&mut self, font_index: usize, character: char, font_size: f32) {
        self.atlas_dirty = true;
        
        let key = CharacterKey {
            font_index,
            character,
            font_size: font_size as u32,
        };
        
        if self.character_cache.contains_key(&key) {
            return;
        }
        
        if font_index >= self.swash_fonts.len() {
            return;
        }
        
        let swash_font = &self.swash_fonts[font_index];
        let glyph_id = swash_font.charmap().map(character);
        if glyph_id == 0 {
            // No glyph mapping — create placeholder
            let info = CharacterInfo {
                uv_x: 0.97,
                uv_y: 0.0,
                uv_w: 0.0,
                uv_h: 0.0,
                width: 0.0,
                height: 0.0,
                advance_x: font_size * 0.5,
                bearing_x: 0.0,
                bearing_y: 0.0,
                is_color: false,
            };
            self.character_cache.insert(key, info);
            return;
        }
        
        let mut scaler = self.scale_context.builder(*swash_font).size(font_size).build();
        
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .render(&mut scaler, glyph_id);
        
        let image = match image {
            Some(img) => img,
            None => {
                // Render failed — create placeholder
                let info = CharacterInfo {
                    uv_x: 0.97,
                    uv_y: 0.0,
                    uv_w: 0.0,
                    uv_h: 0.0,
                    width: 0.0,
                    height: 0.0,
                    advance_x: font_size * 0.5,
                    bearing_x: 0.0,
                    bearing_y: 0.0,
                    is_color: false,
                };
                self.character_cache.insert(key, info);
                return;
            }
        };
        
        let char_width = image.placement.width as u32;
        let char_height = image.placement.height as u32;
        
        if char_width == 0 || char_height == 0 {
            // swash glyph_metrics.advance_width() returns design units, NOT pixels
            // Must scale by font_size / units_per_em to get pixel-scaled value
            // NOTE: image.placement.left/top are already in pixels (not design units)
            let upem = swash_font.metrics(&[]).units_per_em as f32;
            let scale = font_size / upem;
            let advance_x = swash_font.glyph_metrics(&[]).advance_width(glyph_id) * scale;
            let info = CharacterInfo {
                uv_x: 0.97,
                uv_y: 0.0,
                uv_w: 0.0,
                uv_h: 0.0,
                width: 0.0,
                height: 0.0,
                advance_x,
                bearing_x: image.placement.left as f32,
                bearing_y: image.placement.top as f32,
                is_color: false,
            };
            self.character_cache.insert(key, info);
            return;
        }
        
        // Check atlas space
        if self.current_x + char_width > self.atlas_width {
            self.current_x = 0;
            self.current_y += self.row_height;
            self.row_height = 0;
        }
        
        if self.current_y + char_height > self.atlas_height {
            dfx_warn!("FontAtlas", "Atlas FULL! color char='{}' font_idx={} size={}: y={} + h={} > max={}", 
                character, font_index, font_size, self.current_y, char_height, self.atlas_height);
            return;
        }
        
        // Write pixel data to atlas based on content type
        match image.content {
            Content::Color => {
                // RGBA data: 4 bytes per pixel, copy directly
                for y in 0..char_height {
                    for x in 0..char_width {
                        let atlas_x = self.current_x + x;
                        let atlas_y = self.current_y + y;
                        let src_idx = (y as usize * char_width as usize + x as usize) * 4;
                        let dst_idx = atlas_y as usize * self.atlas_width as usize * 4 + atlas_x as usize * 4;
                        
                        if src_idx + 3 < image.data.len() && dst_idx + 3 < self.atlas_texture.len() {
                            self.atlas_texture[dst_idx] = image.data[src_idx];       // R
                            self.atlas_texture[dst_idx + 1] = image.data[src_idx + 1]; // G
                            self.atlas_texture[dst_idx + 2] = image.data[src_idx + 2]; // B
                            self.atlas_texture[dst_idx + 3] = image.data[src_idx + 3]; // A
                        }
                    }
                }
            }
            Content::Mask => {
                // Alpha mask: write as white + alpha (same as fontdue path)
                for y in 0..char_height {
                    for x in 0..char_width {
                        let atlas_x = self.current_x + x;
                        let atlas_y = self.current_y + y;
                        let src_idx = y as usize * char_width as usize + x as usize;
                        let dst_idx = atlas_y as usize * self.atlas_width as usize * 4 + atlas_x as usize * 4;
                        
                        if src_idx < image.data.len() && dst_idx + 3 < self.atlas_texture.len() {
                            let val = image.data[src_idx];
                            self.atlas_texture[dst_idx] = 255;
                            self.atlas_texture[dst_idx + 1] = 255;
                            self.atlas_texture[dst_idx + 2] = 255;
                            self.atlas_texture[dst_idx + 3] = val;
                        }
                    }
                }
            }
            Content::SubpixelMask => {
                // Subpixel mask: skip/ignore, treat as placeholder
                // NOTE: image.placement.left/top are already in pixels (not design units)
                let upem = swash_font.metrics(&[]).units_per_em as f32;
                let scale = font_size / upem;
                let advance_x = swash_font.glyph_metrics(&[]).advance_width(glyph_id) * scale;
                let info = CharacterInfo {
                    uv_x: 0.97,
                    uv_y: 0.0,
                    uv_w: 0.0,
                    uv_h: 0.0,
                    width: 0.0,
                    height: 0.0,
                    advance_x,
                    bearing_x: image.placement.left as f32,
                    bearing_y: image.placement.top as f32,
                    is_color: false,
                };
                self.character_cache.insert(key, info);
                return;
            }
        }
        
        let is_color = image.content == Content::Color;
        // swash glyph_metrics.advance_width() returns design units, NOT pixels
        // Must scale by font_size / units_per_em to match fontdue's pixel-scaled values
        // This is critical: without scaling, emoji advance_x is ~1000 (design units)
        // while fontdue text advance_x is ~10-12 (pixels), causing cursor to jump
        // ~1000 pixels after emoji → all subsequent text pushed off screen
        // NOTE: image.placement.left/top are already in pixels (not design units),
        // so bearing_x/bearing_y should NOT be scaled
        let upem = swash_font.metrics(&[]).units_per_em as f32;
        let scale = font_size / upem;
        let bearing_x = image.placement.left as f32;
        let bearing_y = image.placement.top as f32; // swash: placement.top = distance from baseline to glyph top (positive = above baseline), already in pixels
        let advance_x = swash_font.glyph_metrics(&[]).advance_width(glyph_id) * scale;
        
        let info = CharacterInfo {
            uv_x: self.current_x as f32 / self.atlas_width as f32,
            uv_y: self.current_y as f32 / self.atlas_height as f32,
            uv_w: char_width as f32 / self.atlas_width as f32,
            uv_h: char_height as f32 / self.atlas_height as f32,
            width: char_width as f32,
            height: char_height as f32,
            advance_x,
            bearing_x,
            bearing_y,
            is_color,
        };
        
        self.character_cache.insert(key, info);
        
        self.current_x += char_width + 1;
        self.row_height = self.row_height.max(char_height);
    }

    /// Try to rasterize a character with fallback to emoji font.
    /// If the primary font_index doesn't have the glyph (empty bitmap),
    /// try all other fonts until one produces a non-empty result.
    pub fn rasterize_char_with_fallback(&mut self, primary_font_index: usize, character: char, font_size: f32) {
        // Try primary font first
        let primary_key = CharacterKey {
            font_index: primary_font_index,
            character,
            font_size: font_size as u32,
        };
        
        if self.character_cache.contains_key(&primary_key) {
            return; // Already cached
        }
        
        // Rasterize with primary font
        self.rasterize_char_direct(primary_font_index, character, font_size);
        
        // Check if the result is a placeholder (empty glyph = font doesn't have this char)
        let is_placeholder = self.character_cache.get(&primary_key)
            .map(|info| info.uv_x == 0.97 && info.uv_w == 0.0 && info.width == 0.0 && info.height == 0.0)
            .unwrap_or(false);
        
        if !is_placeholder {
            return; // Primary font has the glyph — done
        }
        
        // Primary font doesn't have this glyph — try fallback fonts
        let font_count = self.fonts.len();
        let mut found_fallback: Option<CharacterInfo> = None;
        
        for fallback_idx in 0..font_count {
            if fallback_idx == primary_font_index {
                continue;
            }
            
            let fallback_key = CharacterKey {
                font_index: fallback_idx,
                character,
                font_size: font_size as u32,
            };
            
            if self.character_cache.contains_key(&fallback_key) {
                // Check if this fallback already has a real glyph
                if let Some(info) = self.character_cache.get(&fallback_key) {
                    if info.uv_x != 0.97 || info.uv_w != 0.0 {
                        found_fallback = Some(CharacterInfo {
                            uv_x: info.uv_x,
                            uv_y: info.uv_y,
                            uv_w: info.uv_w,
                            uv_h: info.uv_h,
                            width: info.width,
                            height: info.height,
                            advance_x: info.advance_x,
                            bearing_x: info.bearing_x,
                            bearing_y: info.bearing_y,
                            is_color: info.is_color,
                        });
                        break;
                    }
                }
                continue;
            }
            
            // Try rasterizing with this fallback font
            self.rasterize_char_direct(fallback_idx, character, font_size);
            
            // Check result
            if let Some(info) = self.character_cache.get(&fallback_key) {
                if info.uv_x != 0.97 || info.uv_w != 0.0 {
                    found_fallback = Some(CharacterInfo {
                        uv_x: info.uv_x,
                        uv_y: info.uv_y,
                        uv_w: info.uv_w,
                        uv_h: info.uv_h,
                        width: info.width,
                        height: info.height,
                        advance_x: info.advance_x,
                        bearing_x: info.bearing_x,
                        bearing_y: info.bearing_y,
                        is_color: info.is_color,
                    });
                    break;
                }
            }
        }
        
        // Replace the placeholder in primary font with fallback glyph data
        if let Some(replacement) = found_fallback {
            self.character_cache.insert(primary_key, replacement);
        }
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
        
        // Check if this is a color glyph — use swash for emoji
        if self.is_color_glyph(font_index, character) {
            self.rasterize_color_glyph(font_index, character, font_size);
            return;
        }
        
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
                    is_color: false,
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
                is_color: false,
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
            dfx_warn!("FontAtlas", "Atlas FULL! char='{}' font_idx={} size={}: y={} + h={} > max={}, cache={}", 
                character, font_index, font_size, self.current_y, char_height, self.atlas_height, self.character_cache.len());
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
            is_color: false,
        };
        
        self.character_cache.insert(key, info);
        
        self.current_x += char_width + 1;
        self.row_height = self.row_height.max(char_height);
    }
    
    fn rasterize_char(&mut self, font_index: usize, character: char, font_size: f32) {
        // Check if this is a color glyph — use swash for emoji
        if self.is_color_glyph(font_index, character) {
            self.rasterize_color_glyph(font_index, character, font_size);
            return;
        }
        
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
                    is_color: false,
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
                is_color: false,
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
            dfx_warn!("FontAtlas", "Atlas FULL! char='{}' font_idx={} size={}: y={} + h={} > max={}, cache={}", 
                character, font_index, font_size, self.current_y, char_height, self.atlas_height, self.character_cache.len());
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
            is_color: false,
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
    ) -> Vec<(f32, f32, usize, usize, f32, f32, f32, f32, bool)> {
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
            
            match self.get_char_info(font_index, character, font_size) {
                Some(info) => {
                    // 检查是否是placeholder（font不支持此字符，fallback也失败）
                    if info.width == 0.0 && info.height == 0.0 && info.uv_w == 0.0 {
                        // Placeholder glyph (font lacks this character, fallback also failed)
                        // CJK characters like 搜/索 trigger this every frame — log only first occurrence per character
                        static PLACEHOLDER_SEEN_LEFT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                        let char_bits = (character as u32 as u64) | ((font_index as u64) << 32);
                        if PLACEHOLDER_SEEN_LEFT.fetch_or(char_bits, std::sync::atomic::Ordering::Relaxed) & char_bits == 0 {
                            dfx_debug!("FontAtlas", "layout_text_left: placeholder char='{}' font_idx={} size={} — fallback failed (suppressed)", 
                                character, font_index, font_size);
                        }
                        cursor_x += info.advance_x;
                        continue;
                    }
                    
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
                        info.is_color,
                    ));
                    
                    cursor_x += info.advance_x;
                }
                None => {
                    // Missing char info — log only first occurrence per character
                    static MISSING_SEEN_LEFT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                    let char_bits = (character as u32 as u64) | ((font_index as u64) << 32);
                    if MISSING_SEEN_LEFT.fetch_or(char_bits, std::sync::atomic::Ordering::Relaxed) & char_bits == 0 {
                        let cached_size = self.get_nearest_cached_font_size(font_size);
                        dfx_debug!("FontAtlas", "layout_text_left: get_char_info None for char='{}' font_idx={} size={} cached_size={} (suppressed)", 
                            character, font_index, font_size, cached_size as u32);
                    }
                    cursor_x += font_size;
                }
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
    ) -> Vec<(f32, f32, usize, usize, f32, f32, f32, f32, bool)> {
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
                // 检查是否是placeholder（font不支持此字符）
                if info.width == 0.0 && info.height == 0.0 && info.uv_w == 0.0 {
                    // Placeholder glyph — log only first occurrence per character
                    static PLACEHOLDER_SEEN_CENTERED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                    let char_bits = (character as u32 as u64) | ((font_index as u64) << 32);
                    if PLACEHOLDER_SEEN_CENTERED.fetch_or(char_bits, std::sync::atomic::Ordering::Relaxed) & char_bits == 0 {
                        dfx_debug!("FontAtlas", "layout_text_centered: placeholder char='{}' font_idx={} size={} — fallback failed (suppressed)", 
                            character, font_index, font_size);
                    }
                    cursor_x += info.advance_x;
                    continue; // 跳过placeholder，不push到result
                }
                
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
                    info.is_color,
                ));
                
                cursor_x += info.advance_x;
            } else {
                // Missing char info — log only first occurrence per character
                static MISSING_SEEN_CENTERED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let char_bits = (character as u32 as u64) | ((font_index as u64) << 32);
                if MISSING_SEEN_CENTERED.fetch_or(char_bits, std::sync::atomic::Ordering::Relaxed) & char_bits == 0 {
                    dfx_debug!("FontAtlas", "layout_text_centered: get_char_info returned None for char='{}' font_idx={} size={} (suppressed)", 
                        character, font_index, font_size);
                }
                cursor_x += font_size;
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
        
        // Load system emoji font as fallback (Windows: Segoe UI Emoji)
        let emoji_font_candidates = [
            "C:\\Windows\\Fonts\\seguiemj.ttf",   // Segoe UI Emoji
            "C:\\Windows\\Fonts\\Segoe UI Emoji.ttf", // Alternative path
        ];
        
        for emoji_path in &emoji_font_candidates {
            if std::path::Path::new(emoji_path).exists() {
                if let Ok(emoji_data) = std::fs::read(emoji_path) {
                    let emoji_idx = atlas.add_font(&emoji_data);
                    // Pre-cache common emoji ranges
                    let emoji_chars = "😀😁😂🤣😃😄😅😆😉😊😋😎😍😘😗😙😚🙂🤗🤔😐😑😶🙄😏😣😥😮🤐😯😪😫😴😌🤓🤔🤗🤕🤠🤡🤢🤣🤤🤥🤧🤮🤯🤰🤱🤲🤳🤴🤵🤶🤷🤸🤹🤺🤻🤼🤽🤾🤿🙄😂";
                    atlas.prerasterize_chars(emoji_idx, emoji_chars, &sizes);
                    Arc::new(Mutex::new(DfxSystem::new())).lock().get_logger().lock().log(LogLevel::Info, "FontAtlas", &format!("Loaded emoji font (idx={}) from {}", emoji_idx, emoji_path), file!(), line!());
                    break;
                }
            }
        }
        
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
            atlas_guard.rasterize_char_with_fallback(font_index, character, cached_size as f32);
        }
    }
}

