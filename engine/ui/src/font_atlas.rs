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
}

impl FontAtlas {
    pub fn new() -> Self {
        Self {
            fonts: Vec::new(),
            font_data: Vec::new(),
            atlas_texture: vec![0u8; 2048 * 2048 * 4],
            atlas_width: 2048,
            atlas_height: 2048,
            character_cache: HashMap::new(),
            current_x: 0,
            current_y: 0,
            row_height: 0,
        }
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
    
    fn rasterize_char_direct(&mut self, font_index: usize, character: char, font_size: f32) {
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
        
        let supersample_scale = 2.0;
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
        
        let supersample_scale = 2.0;
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

pub fn create_font_atlas() -> FontAtlas {
    let mut atlas = FontAtlas::new();
    
    let font_path = std::path::Path::new("C:\\Users\\94023\\Documents\\commandline-tools-windows-x64\\command-line-tools\\sdk\\default\\hms\\previewer\\resources\\fonts\\HarmonyOS_Sans_SC.ttf");
    
    if font_path.exists() {
        let font_data = std::fs::read(font_path).expect("Failed to read font file");
        let font_index = atlas.add_font(&font_data);
        
        let test_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_+=[]{}|;:,.<>?/~ Click Me Welcome to Hezhou UI Press SPACE to change text hello 新建打开保存运行项目结构资产管理游戏预览属性编辑选中位置大小状态FPS脚本场景模型纹理新建场景新建脚本新建材质新建文件夹打开场景打开项目打开资源保存场景保存全部另存为就绪未命名TexturesModels // NewScript.cs using System Hezhou public class void Start Console WriteLine Update deltaTime logic here script file content read text edit panel button label stack panel rect layout style color background Hot Reload Trigger function method return float int string bool true false null this static private protected public override virtual abstract async await foreach while for if else switch case break continue try catch finally throw new out in ref params get set value object var const readonly namespace import using partial where select from group order by into join let on equals ascend descending take skip distinct aggregate sum count min max average first last single any all contains except intersect union concat reverse zip sequence element at range index substring length trim split join replace contains starts ends index of remove insert pad format parse convert to string int double bool datetime timespan guid guid new guid empty null try parse parse exact compare equals gethashcode tostring from binary to base64 stream reader writer file path directory info drive exception stack trace inner message source data code detail fault invalid operation argument null range type unsupported not implemented object disposed thread state timeout deadlock monitor lock mutex semaphore concurrent queue stack dictionary list array hash set sorted linked observable binding is enabled disabled visible collapsed hidden margin padding width height min max actual desired horizontal vertical alignment center left right top bottom stretch wrap wrap text clip overflow scroll auto fit fill uniform aspect ratio scale transform rotate translate skew matrix vector point size rect bounds location corner edge border corner radius thickness brush solid linear radial gradient color image tile stretch fill uniform aspect ratio opacity visibility hit test visible collapsed hidden render transform layout clip effect drop shadow blur glow outer inner noise mask opacity filter level radius direction offset angle distance spread source destination blend mode mix copy clear source over in out atop xor add saturate multiply screen overlay darken lighten color dodge burn soft hard difference exclusion hue saturation luminosity component mask unmask isolate isolate group knockout luma rgb alpha premultiplied straight apply save restore reset clear fill stroke clip path transform begin end close move line curve quadratic bezier smooth arc rect circle ellipse text font family style weight italic bold normal regular medium light extralight extrabold thin black condensed extended oblique underline strikethrough baseline subscript superscript small caps letter spacing word spacing kerning tracking leading line height paragraph indent hanging first left right tab stop decimal alignment keep together break before after around avoid orphans widows hyphenate minimum maximum consecutive limit zone threshold characters spaces auto manual none column row gap rule style width color span balance fill empty auto balanced consume flexible remaining fit fill proportionally distribute space stretch grow shrink basis direction wrap reverse main start end center justify between around evenly cross start end center stretch baseline content start end center between around evenly even odd stretch auto min max fit fill none collapse separate border box content box padding box margin box fixed sticky relative absolute static transform style flat preserve 3d perspective backface visible hidden translate Z scale Z rotate X Y Z perspective origin flat preserve 3d backface visible hidden pointer events auto none visible painted fill stroke all bounding clip path mask filter opacity transition animation timing function ease linear ease in ease out ease in out step start step end cubic bezier spring frames delay duration iteration count infinite alternate reverse both normal running paused fill mode forwards backwards both none keyframes block from to 0% 100% important media screen print handheld projection tv color monochrome resolution dpi dpcm scan progressive interlace grid width height orientation aspect ratio pixel index color monotone grayscale hue saturation lightness red green blue alpha cyan magenta yellow black cielab hcl hsv hwb named transparent current system accent dark light appearance theme media feature prefers reduced motion data save forced colors dynamic range contrast high standard none active hover focus enabled disabled read only checked indeterminate placeholder value empty valid invalid in range out of range required optional autofill autofilled modal open default placeholder marker selection caret match parent always never internal external global local inherit initial unset revert layer cascade scope container query selector type universal class id attribute pseudo element child descendant sibling adjacent general namespace combinator grouping nesting at rule declaration property value important syntax vendor extension hack prefix moz webkit o ms khtml apple official";
        let sizes = [48.0, 36.0, 32.0, 24.0, 20.0, 18.0, 16.0, 14.0, 12.0];
        
        atlas.prerasterize_chars(font_index, test_chars, &sizes);
        
        Arc::new(Mutex::new(DfxSystem::new())).lock().get_logger().lock().log(LogLevel::Info, "FontAtlas", &format!("Pre-rasterized {} chars at sizes {:?}", test_chars.len(), sizes), file!(), line!());
    } else {
        Arc::new(Mutex::new(DfxSystem::new())).lock().get_logger().lock().log(LogLevel::Warn, "FontAtlas", &format!("Font file not found at {}", font_path.display()), file!(), line!());
    }
    
    atlas
}