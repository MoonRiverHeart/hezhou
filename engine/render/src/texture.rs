pub type TextureId = u64;

#[repr(C)]
pub enum TextureFormat {
    RGBA8 = 0,
    RGB8 = 1,
    RGBA16F = 2,
    RGBA32F = 3,
}

pub struct Texture {
    pub id: TextureId,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub data: Vec<u8>,
}

impl Texture {
    pub fn new(id: TextureId) -> Self {
        Self {
            id,
            width: 0,
            height: 0,
            format: TextureFormat::RGBA8,
            data: Vec::new(),
        }
    }
    
    pub fn create_2d(id: TextureId, width: u32, height: u32, format: TextureFormat) -> Self {
        let size = Self::calculate_size(width, height, &format);
        Self {
            id,
            width,
            height,
            format,
            data: vec![0u8; size],
        }
    }
    
    pub fn from_data(id: TextureId, width: u32, height: u32, format: TextureFormat, data: Vec<u8>) -> Self {
        Self {
            id,
            width,
            height,
            format,
            data,
        }
    }
    
    fn calculate_size(width: u32, height: u32, format: &TextureFormat) -> usize {
        let bytes_per_pixel = match format {
            TextureFormat::RGBA8 => 4,
            TextureFormat::RGB8 => 3,
            TextureFormat::RGBA16F => 8,
            TextureFormat::RGBA32F => 16,
        };
        (width * height * bytes_per_pixel) as usize
    }
    
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }
    
    pub fn get_data(&self) -> &[u8] {
        &self.data
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        let size = Self::calculate_size(width, height, &self.format);
        self.width = width;
        self.height = height;
        self.data.resize(size, 0);
    }
}