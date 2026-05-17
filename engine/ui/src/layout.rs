use crate::types::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub anchor: Anchor,
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,
}

impl Layout {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            anchor: Anchor::TopLeft,
            margin: EdgeInsets::zero(),
            padding: EdgeInsets::zero(),
        }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    pub fn content_bounds(&self) -> Rect {
        Rect::new(
            self.x + self.padding.left,
            self.y + self.padding.top,
            self.width - self.padding.horizontal(),
            self.height - self.padding.vertical(),
        )
    }

    pub fn set_position(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::zero()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    Absolute,
    Flex,
    Grid,
    Stack,
}

impl Default for LayoutType {
    fn default() -> Self {
        Self::Absolute
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlexLayout {
    pub direction: FlexDirection,
    pub wrap: bool,
    pub justify: FlexJustify,
    pub align: FlexAlign,
    pub gap: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl Default for FlexDirection {
    fn default() -> Self {
        Self::Row
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexJustify {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Default for FlexJustify {
    fn default() -> Self {
        Self::Start
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexAlign {
    Start,
    End,
    Center,
    Stretch,
}

impl Default for FlexAlign {
    fn default() -> Self {
        Self::Stretch
    }
}

impl Default for FlexLayout {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Row,
            wrap: false,
            justify: FlexJustify::Start,
            align: FlexAlign::Stretch,
            gap: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GridLayout {
    pub columns: u32,
    pub rows: u32,
    pub column_gap: f32,
    pub row_gap: f32,
}

impl Default for GridLayout {
    fn default() -> Self {
        Self {
            columns: 1,
            rows: 1,
            column_gap: 0.0,
            row_gap: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StackLayout {
    pub orientation: StackOrientation,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackOrientation {
    Horizontal,
    Vertical,
}

impl Default for StackOrientation {
    fn default() -> Self {
        Self::Vertical
    }
}

impl Default for StackLayout {
    fn default() -> Self {
        Self {
            orientation: StackOrientation::Vertical,
        }
    }
}
