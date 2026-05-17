use crate::types::*;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureType {
    Tap,
    DoubleTap,
    LongPress,
    Pan,
    Swipe,
    Pinch,
    Rotation,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Gesture {
    pub gesture_type: GestureType,
    pub state: GestureState,
    pub target: WidgetId,
    pub data: GestureData,
}

impl Gesture {
    pub fn new(gesture_type: GestureType, target: WidgetId) -> Self {
        Self {
            gesture_type,
            state: GestureState::Possible,
            target,
            data: GestureData::None,
        }
    }

    pub fn with_data(mut self, data: GestureData) -> Self {
        self.data = data;
        self
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureState {
    Possible,
    Began,
    Changed,
    Ended,
    Cancelled,
    Failed,
}

impl Default for GestureState {
    fn default() -> Self {
        Self::Possible
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub enum GestureData {
    None,
    Tap(TapData),
    Pan(PanData),
    Pinch(PinchData),
    Rotation(RotationData),
    Swipe(SwipeData),
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TapData {
    pub x: f32,
    pub y: f32,
    pub tap_count: u32,
}

impl TapData {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y, tap_count: 1 }
    }

    pub fn with_count(mut self, count: u32) -> Self {
        self.tap_count = count;
        self
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PanData {
    pub start_x: f32,
    pub start_y: f32,
    pub current_x: f32,
    pub current_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
}

impl PanData {
    pub fn new(start_x: f32, start_y: f32) -> Self {
        Self {
            start_x,
            start_y,
            current_x: start_x,
            current_y: start_y,
            velocity_x: 0.0,
            velocity_y: 0.0,
        }
    }

    pub fn update_position(&mut self, x: f32, y: f32) {
        self.current_x = x;
        self.current_y = y;
    }

    pub fn translation(&self) -> Point {
        Point::new(self.current_x - self.start_x, self.current_y - self.start_y)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PinchData {
    pub scale: f32,
    pub velocity: f32,
}

impl PinchData {
    pub fn new(scale: f32) -> Self {
        Self {
            scale,
            velocity: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RotationData {
    pub rotation: f32,
    pub velocity: f32,
}

impl RotationData {
    pub fn new(rotation: f32) -> Self {
        Self {
            rotation,
            velocity: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SwipeData {
    pub direction: SwipeDirection,
    pub velocity: f32,
}

impl SwipeData {
    pub fn new(direction: SwipeDirection, velocity: f32) -> Self {
        Self {
            direction,
            velocity,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Left,
    Right,
    Up,
    Down,
}
