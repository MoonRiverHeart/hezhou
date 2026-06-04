use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct WindowEvent {
    pub timestamp: SystemTime,
    pub window_id: u32,
    pub event_type: WindowEventType,
    pub handled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEventType {
    /// 窗口创建
    Created,
    /// 窗口关闭请求
    CloseRequested,
    /// 窗口已关闭
    Closed,
    /// 窗口大小改变
    Resized { width: u32, height: u32 },
    /// 窗口位置移动
    Moved { x: i32, y: i32 },
    /// 窗口获得焦点
    FocusGained,
    /// 窗口失去焦点
    FocusLost,
    /// 窗口最小化
    Minimized,
    /// 窗口恢复
    Restored,
    /// 窗口最大化
    Maximized,
    /// 窗口缩放比例改变（高分屏）
    ScaleFactorChanged { scale_factor: f64 },
    /// 窗口重绘请求
    RedrawRequested,
    /// 窗口可见性改变
    VisibilityChanged { visible: bool },
}