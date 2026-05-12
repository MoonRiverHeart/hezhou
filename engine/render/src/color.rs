#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    
    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }
    
    pub fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(r, g, b, a)
    }
    
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
    
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
    
    pub fn red() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0)
    }
    
    pub fn green() -> Self {
        Self::new(0.0, 1.0, 0.0, 1.0)
    }
    
    pub fn blue() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }
    
    pub fn yellow() -> Self {
        Self::new(1.0, 1.0, 0.0, 1.0)
    }
    
    pub fn cyan() -> Self {
        Self::new(0.0, 1.0, 1.0, 1.0)
    }
    
    pub fn magenta() -> Self {
        Self::new(1.0, 0.0, 1.0, 1.0)
    }
    
    pub fn gray() -> Self {
        Self::new(0.5, 0.5, 0.5, 1.0)
    }
    
    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
    
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::new(
            a.r + (b.r - a.r) * t,
            a.g + (b.g - a.g) * t,
            a.b + (b.b - a.b) * t,
            a.a + (b.a - a.a) * t,
        )
    }
    
    pub fn to_u8(&self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }
    
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }
}