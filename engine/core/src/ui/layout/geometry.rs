/// 二维尺寸
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Size { width, height }
    }
    
    /// 在约束范围内调整尺寸
    pub fn clamp(self, min: Size, max: Size) -> Self {
        Size {
            width: self.width.clamp(min.width, max.width),
            height: self.height.clamp(min.height, max.height),
        }
    }
}

/// 矩形区域（位置+尺寸）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Rect { x, y, width, height }
    }
    
    /// 获取尺寸部分
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
    
    /// 获取位置部分
    pub fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

/// 二维点坐标
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Point { x, y }
    }
}

/// 布局约束：父节点给子节点的空间限制
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Constraints {
    /// 创建"紧"约束：强制子节点为指定尺寸
    pub fn tight(size: Size) -> Self {
        Constraints {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
        }
    }
    
    /// 创建"松"约束：子节点可以在0到指定尺寸之间自由选择
    pub fn loose(size: Size) -> Self {
        Constraints {
            min_width: 0.0,
            max_width: size.width,
            min_height: 0.0,
            max_height: size.height,
        }
    }
    
    /// 无界约束（子节点可以任意大）
    pub fn unbounded() -> Self {
        Constraints {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }
    
    /// 约束一个尺寸到合法范围
    pub fn constrain(&self, size: &mut Size) {
        size.width = size.width.clamp(self.min_width, self.max_width);
        size.height = size.height.clamp(self.min_height, self.max_height);
    }
    
    /// 创建被边距缩小的约束
    pub fn deflate(&self, edge: &EdgeInsets) -> Self {
        let horizontal = edge.left + edge.right;
        let vertical = edge.top + edge.bottom;
        
        Constraints {
            min_width: (self.min_width - horizontal).max(0.0),
            max_width: (self.max_width - horizontal).max(0.0),
            min_height: (self.min_height - vertical).max(0.0),
            max_height: (self.max_height - vertical).max(0.0),
        }
    }
}

/// 边距（内边距或外边距）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub fn all(value: f32) -> Self {
        EdgeInsets {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
    
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        EdgeInsets {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
    
    pub fn zero() -> Self {
        EdgeInsets::all(0.0)
    }
    
    /// 将内边距加到尺寸上
    pub fn inflate(&self, size: &Size) -> Size {
        Size::new(
            size.width + self.left + self.right,
            size.height + self.top + self.bottom,
        )
    }
    
    /// 从尺寸减去内边距
    pub fn deflate(&self, size: &Size) -> Size {
        Size::new(
            (size.width - self.left - self.right).max(0.0),
            (size.height - self.top - self.bottom).max(0.0),
        )
    }
}

/// 布局方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// 对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}