use std::time::SystemTime;
use super::mouse::Point;

#[derive(Debug, Clone, PartialEq)]
pub struct TouchEvent {
    pub timestamp: SystemTime,
    pub touches: Vec<Touch>,
    pub event_type: TouchEventType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Touch {
    pub id: u64,
    pub position: Point,
    pub pressure: Option<f32>,
    pub radius: Option<Point>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TouchEventType {
    /// 触摸开始
    Started,
    /// 触摸移动
    Moved,
    /// 触摸结束
    Ended,
    /// 触摸取消
    Cancelled,
}