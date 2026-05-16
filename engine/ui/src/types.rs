use std::os::raw::c_void;

#[repr(C)]
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
    
    pub fn transparent() -> Self {
        Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }
    }
    
    pub fn black() -> Self {
        Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
    }
    
    pub fn white() -> Self {
        Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
    }
    
    pub fn red() -> Self {
        Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }
    }
    
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
    
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }
}

#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
    
    pub fn distance(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::zero()
    }
}

#[repr(C)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
    
    pub fn zero() -> Self {
        Self { width: 0.0, height: 0.0 }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::zero()
    }
}

#[repr(C)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
    
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }
    }
    
    pub fn from_points(origin: Point, size: Size) -> Self {
        Self {
            x: origin.x,
            y: origin.y,
            width: size.width,
            height: size.height,
        }
    }
    
    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x && point.x <= self.x + self.width &&
        point.y >= self.y && point.y <= self.y + self.height
    }
    
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width &&
        self.x + self.width > other.x &&
        self.y < other.y + other.height &&
        self.y + self.height > other.y
    }
    
    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }
    
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
    
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::zero()
    }
}

#[repr(C)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }
    
    pub fn all(value: f32) -> Self {
        Self { left: value, top: value, right: value, bottom: value }
    }
    
    pub fn zero() -> Self {
        Self { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 }
    }
    
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }
    
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

impl Default for EdgeInsets {
    fn default() -> Self {
        Self::zero()
    }
}

#[repr(C)]
pub struct Transform {
    pub matrix: [f32; 9],
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0,
            ],
        }
    }
    
    pub fn translate(x: f32, y: f32) -> Self {
        Self {
            matrix: [
                1.0, 0.0, x,
                0.0, 1.0, y,
                0.0, 0.0, 1.0,
            ],
        }
    }
    
    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            matrix: [
                sx, 0.0, 0.0,
                0.0, sy, 0.0,
                0.0, 0.0, 1.0,
            ],
        }
    }
    
    pub fn rotate(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            matrix: [
                cos, -sin, 0.0,
                sin, cos, 0.0,
                0.0, 0.0, 1.0,
            ],
        }
    }
    
    pub fn multiply(&self, other: &Transform) -> Transform {
        let mut result = [0.0f32; 9];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    result[i * 3 + j] += self.matrix[i * 3 + k] * other.matrix[k * 3 + j];
                }
            }
        }
        Transform { matrix: result }
    }
    
    pub fn transform_point(&self, point: &Point) -> Point {
        Point {
            x: self.matrix[0] * point.x + self.matrix[1] * point.y + self.matrix[2],
            y: self.matrix[3] * point.x + self.matrix[4] * point.y + self.matrix[5],
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[repr(C)]
pub struct Shadow {
    pub color: Color,
    pub offset: Point,
    pub blur_radius: f32,
}

impl Shadow {
    pub fn new(color: Color, offset: Point, blur_radius: f32) -> Self {
        Self { color, offset, blur_radius }
    }
    
    pub fn default_shadow() -> Self {
        Self {
            color: Color::new(0.0, 0.0, 0.0, 0.5),
            offset: Point::new(2.0, 2.0),
            blur_radius: 4.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId {
    pub id: u64,
}

impl WidgetId {
    pub fn new() -> Self {
        use uuid::Uuid;
        Self {
            id: Uuid::new_v4().as_u128() as u64,
        }
    }
    
    pub fn from_raw(id: u64) -> Self {
        Self { id }
    }
    
    pub fn invalid() -> Self {
        Self { id: 0 }
    }
    
    pub fn is_valid(&self) -> bool {
        self.id != 0
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::invalid()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Default for Anchor {
    fn default() -> Self {
        Self::TopLeft
    }
}

#[repr(C)]
pub struct TextAlignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

impl Default for TextAlignment {
    fn default() -> Self {
        Self {
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Top,
        }
    }
}