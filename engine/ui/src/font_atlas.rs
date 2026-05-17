use fontdue::Font;
use std::collections::HashMap;
use hezhou_dfx::{LogLevel, DfxSystem};
use parking_lot::Mutex;
use std::sync::Arc;

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
    pub bearing_y: f32,
}

pub struct FontAtlas {
    fonts: Vec<Font>,
    font_data: Vec<Vec<u8>>,
    atlas_texture: Vec<u8>,
    atlas_width: u32,
    atlas_height: u32,
    character_cache: HashMap<CharacterKey, CharacterInfo>,
}

impl FontAtlas {
    pub fn new() -> Self {
        Self {
            fonts: Vec::new(),
            font_data: Vec::new(),
            atlas_texture: vec![0u8; 1024 * 1024 * 4],
            atlas_width: 1024,
            atlas_height: 1024,
            character_cache: HashMap::new(),
        }
    }
    
    pub fn add_font(&mut self, font_data: &[u8]) -> usize {
        let font = Font::from_bytes(font_data, fontdue::FontSettings::default())
            .expect("Failed to parse font");
        
        self.font_data.push(font_data.to_vec());
        self.fonts.push(font);
        
        self.fonts.len() - 1
    }
    
    pub fn prerasterize_chars(&mut self, font_index: usize, chars: &str, sizes: &[f32]) {
        if font_index >= self.fonts.len() {
            return;
        }
        
        for size in sizes {
            for c in chars.chars() {
                self.rasterize_char(font_index, c, *size);
            }
        }
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
        
        let (metrics, bitmap) = self.fonts[font_index].rasterize(character, font_size);
        
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
                advance_x: metrics.advance_width,
                bearing_y: 0.0,
            };
            self.character_cache.insert(key, info);
            return;
        }
        
        static mut CURRENT_X: u32 = 0;
        static mut CURRENT_Y: u32 = 0;
        static mut ROW_HEIGHT: u32 = 0;
        
        unsafe {
            if CURRENT_X + char_width > self.atlas_width {
                CURRENT_X = 0;
                CURRENT_Y += ROW_HEIGHT;
                ROW_HEIGHT = 0;
            }
            
            if CURRENT_Y + char_height > self.atlas_height {
                return;
            }
            
            for y in 0..char_height {
                for x in 0..char_width {
                    let atlas_x = CURRENT_X + x;
                    let atlas_y = CURRENT_Y + y;
                    let src_idx = y as usize * char_width as usize + x as usize;
                    let dst_idx = atlas_y as usize * self.atlas_width as usize * 4 + atlas_x as usize * 4;
                    
                    if src_idx < bitmap.len() && dst_idx + 3 < self.atlas_texture.len() {
                        let val = bitmap[src_idx];
                        self.atlas_texture[dst_idx] = val;
                        self.atlas_texture[dst_idx + 1] = val;
                        self.atlas_texture[dst_idx + 2] = val;
                        self.atlas_texture[dst_idx + 3] = 255;
                    }
                }
            }
            
            let bearing_y = metrics.bounds.height + metrics.bounds.ymin;
            
            let info = CharacterInfo {
                uv_x: CURRENT_X as f32 / self.atlas_width as f32,
                uv_y: CURRENT_Y as f32 / self.atlas_height as f32,
                uv_w: char_width as f32 / self.atlas_width as f32,
                uv_h: char_height as f32 / self.atlas_height as f32,
                width: char_width as f32,
                height: char_height as f32,
                advance_x: metrics.advance_width,
                bearing_y,
            };
            
            self.character_cache.insert(key, info);
            
            CURRENT_X += char_width + 1;
            ROW_HEIGHT = ROW_HEIGHT.max(char_height);
        }
    }
    
    pub fn get_char_info(&self, font_index: usize, character: char, font_size: f32) -> Option<&CharacterInfo> {
        let key = CharacterKey {
            font_index,
            character,
            font_size: font_size as u32,
        };
        
        self.character_cache.get(&key)
    }
    
    pub fn get_atlas_texture(&self) -> &[u8] {
        &self.atlas_texture
    }
    
    pub fn get_atlas_dimensions(&self) -> (u32, u32) {
        (self.atlas_width, self.atlas_height)
    }
    
    pub fn measure_text(&self, font_index: usize, text: &str, font_size: f32) -> (f32, f32) {
        if font_index >= self.fonts.len() || text.is_empty() {
            return (0.0, 0.0);
        }
        
        let mut total_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;
        
        for character in text.chars() {
            if let Some(info) = self.get_char_info(font_index, character, font_size) {
                total_width += info.advance_x;
                max_height = max_height.max(info.height);
            }
        }
        
        (total_width, max_height)
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
        
        let start_x = container_x + (container_width - text_width) / 2.0;
        let start_y = container_y + (container_height - text_height) / 2.0;
        
        let mut result = Vec::new();
        let mut cursor_x = start_x;
        
        for character in text.chars() {
            if let Some(info) = self.get_char_info(font_index, character, font_size) {
                result.push((
                    cursor_x,
                    start_y,
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
    
    pub fn layout_text_left(
        &self,
        font_index: usize,
        text: &str,
        font_size: f32,
        container_x: f32,
        container_y: f32,
        container_height: f32,
    ) -> Vec<(f32, f32, usize, usize, f32, f32, f32, f32)> {
        if font_index >= self.fonts.len() || text.is_empty() {
            return Vec::new();
        }
        
        let (_, text_height) = self.measure_text(font_index, text, font_size);
        
        let start_x = container_x;
        let start_y = container_y + (container_height - text_height) / 2.0;
        
        let mut result = Vec::new();
        let mut cursor_x = start_x;
        
        for character in text.chars() {
            if let Some(info) = self.get_char_info(font_index, character, font_size) {
                result.push((
                    cursor_x,
                    start_y,
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

pub fn create_font_atlas() -> FontAtlas {
    let mut atlas = FontAtlas::new();
    
    let font_path = std::path::Path::new("C:\\Users\\94023\\Documents\\commandline-tools-windows-x64\\command-line-tools\\sdk\\default\\hms\\previewer\\resources\\fonts\\HarmonyOS_Sans_SC.ttf");
    
    if font_path.exists() {
        let font_data = std::fs::read(font_path).expect("Failed to read font file");
        let font_index = atlas.add_font(&font_data);
        
        let test_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_+=[]{}|;:,.<>?/~ Click Me Welcome to Hezhou UI Press SPACE to change text hello 新建打开保存运行项目结构资产管理游戏预览属性编辑选中位置大小状态FPS脚本场景模型纹理新建场景新建脚本新建材质新建文件夹打开场景打开项目打开资源保存场景保存全部另存为就绪未命名TexturesModels";
        let sizes = [32.0, 16.0, 14.0];
        
        atlas.prerasterize_chars(font_index, test_chars, &sizes);
        
        Arc::new(Mutex::new(DfxSystem::new())).lock().get_logger().lock().log(LogLevel::Info, "FontAtlas", &format!("Pre-rasterized {} chars at sizes {:?}", test_chars.len(), sizes), file!(), line!());
    } else {
        Arc::new(Mutex::new(DfxSystem::new())).lock().get_logger().lock().log(LogLevel::Warn, "FontAtlas", &format!("Font file not found at {}", font_path.display()), file!(), line!());
    }
    
    atlas
}