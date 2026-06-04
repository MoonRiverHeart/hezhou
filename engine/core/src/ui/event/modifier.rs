#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,  // Windows 键 / Command 键
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }
    
    pub fn is_empty(&self) -> bool {
        !self.shift && !self.control && !self.alt && !self.super_key
    }
}