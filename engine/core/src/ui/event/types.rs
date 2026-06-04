use std::time::SystemTime;
use super::{
    window::WindowEvent,
    mouse::MouseEvent,
    keyboard::KeyboardEvent,
    touch::TouchEvent,
    drag::DragEvent,
    widget::WidgetEvent,
    lifecycle::LifecycleEvent,
    modifier::Modifiers,
};

/// UI 事件类型枚举
#[derive(Debug)]
pub enum UIEvent {
    // ========== 窗口事件 ==========
    Window(WindowEvent),
    
    // ========== 鼠标事件 ==========
    Mouse(MouseEvent),
    
    // ========== 键盘事件 ==========
    Keyboard(KeyboardEvent),
    
    // ========== 触摸事件 ==========
    Touch(TouchEvent),
    
    // ========== 拖拽事件 ==========
    Drag(DragEvent),
    
    // ========== 控件事件 ==========
    Widget(WidgetEvent),
    
    // ========== 生命周期事件 ==========
    Lifecycle(LifecycleEvent),
    
    // ========== 自定义事件 ==========
    Custom(String, Box<dyn std::any::Any>),
}

// 手动实现 Clone
impl Clone for UIEvent {
    fn clone(&self) -> Self {
        match self {
            UIEvent::Window(e) => UIEvent::Window(e.clone()),
            UIEvent::Mouse(e) => UIEvent::Mouse(e.clone()),
            UIEvent::Keyboard(e) => UIEvent::Keyboard(e.clone()),
            UIEvent::Touch(e) => UIEvent::Touch(e.clone()),
            UIEvent::Drag(e) => UIEvent::Drag(e.clone()),
            UIEvent::Widget(e) => UIEvent::Widget(e.clone()),
            UIEvent::Lifecycle(e) => UIEvent::Lifecycle(e.clone()),
            UIEvent::Custom(name, _) => UIEvent::Custom(name.clone(), Box::new(())), // 无法克隆 Any，只能占位
        }
    }
}

// 手动实现 PartialEq
impl PartialEq for UIEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (UIEvent::Window(a), UIEvent::Window(b)) => a == b,
            (UIEvent::Mouse(a), UIEvent::Mouse(b)) => a == b,
            (UIEvent::Keyboard(a), UIEvent::Keyboard(b)) => a == b,
            (UIEvent::Touch(a), UIEvent::Touch(b)) => a == b,
            (UIEvent::Drag(a), UIEvent::Drag(b)) => a == b,
            (UIEvent::Widget(a), UIEvent::Widget(b)) => a == b,
            (UIEvent::Lifecycle(a), UIEvent::Lifecycle(b)) => a == b,
            (UIEvent::Custom(a, _), UIEvent::Custom(b, _)) => a == b,
            _ => false,
        }
    }
}

impl UIEvent {
    /// 获取事件发生的时间戳
    pub fn timestamp(&self) -> SystemTime {
        match self {
            UIEvent::Window(e) => e.timestamp,
            UIEvent::Mouse(e) => e.timestamp,
            UIEvent::Keyboard(e) => e.timestamp,
            UIEvent::Touch(e) => e.timestamp,
            UIEvent::Drag(e) => e.timestamp,
            UIEvent::Widget(e) => e.timestamp,
            UIEvent::Lifecycle(e) => e.timestamp,
            UIEvent::Custom(_, _) => SystemTime::now(),
        }
    }
    
    /// 判断事件是否已被处理
    pub fn is_handled(&self) -> bool {
        match self {
            UIEvent::Mouse(e) => e.handled,
            UIEvent::Keyboard(e) => e.handled,
            UIEvent::Widget(e) => e.handled,
            _ => false,
        }
    }
    
    /// 标记事件为已处理
    pub fn set_handled(&mut self) {
        match self {
            UIEvent::Mouse(e) => e.handled = true,
            UIEvent::Keyboard(e) => e.handled = true,
            UIEvent::Widget(e) => e.handled = true,
            _ => {}
        }
    }
}