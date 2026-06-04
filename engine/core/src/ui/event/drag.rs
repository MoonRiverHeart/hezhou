use super::mouse::Point;
use super::mouse::MouseButton;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct DragEvent {
    pub timestamp: SystemTime,
    pub start_position: Point,
    pub current_position: Point,
    pub delta: Point,
    pub button: MouseButton,
    pub data: Option<String>,
    pub event_type: DragEventType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragEventType {
    /// 开始拖拽
    Started,
    /// 拖拽中
    Dragging,
    /// 拖拽结束
    Ended,
    /// 拖拽进入目标
    Entered,
    /// 拖拽离开目标
    Exited,
    /// 拖拽放下
    Dropped,
}