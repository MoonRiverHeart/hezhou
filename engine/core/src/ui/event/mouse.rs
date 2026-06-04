use std::time::SystemTime;
use super::modifier::Modifiers;

/// 鼠标事件
#[derive(Debug, Clone, PartialEq)]
pub struct MouseEvent {
    pub timestamp: SystemTime,
    pub position: Point,
    pub button: Option<MouseButton>,
    pub modifiers: Modifiers,
    pub event_type: MouseEventType,
    pub handled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseEventType {
    /// 鼠标移动
    Moved,
    /// 鼠标进入窗口
    Entered,
    /// 鼠标离开窗口
    Exited,
    /// 鼠标按下
    Pressed,
    /// 鼠标释放
    Released,
    /// 鼠标点击（按下+释放）
    Clicked,
    /// 鼠标双击
    DoubleClicked,
    /// 鼠标滚轮
    Wheel { delta_x: f32, delta_y: f32 },
}

/// 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// 位置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}