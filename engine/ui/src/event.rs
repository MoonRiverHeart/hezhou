use crate::types::*;
use std::fmt;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    TouchBegin,
    TouchMove,
    TouchEnd,
    TouchCancel,
    
    Click,
    LongPress,
    DoubleClick,
    
    KeyDown,
    KeyUp,
    
    MouseEnter,
    MouseLeave,
    
    FocusGain,
    FocusLost,
    
    LayoutChanged,
    StyleChanged,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::TouchBegin => write!(f, "TouchBegin"),
            EventType::TouchMove => write!(f, "TouchMove"),
            EventType::TouchEnd => write!(f, "TouchEnd"),
            EventType::TouchCancel => write!(f, "TouchCancel"),
            EventType::Click => write!(f, "Click"),
            EventType::LongPress => write!(f, "LongPress"),
            EventType::DoubleClick => write!(f, "DoubleClick"),
            EventType::KeyDown => write!(f, "KeyDown"),
            EventType::KeyUp => write!(f, "KeyUp"),
            EventType::MouseEnter => write!(f, "MouseEnter"),
            EventType::MouseLeave => write!(f, "MouseLeave"),
            EventType::FocusGain => write!(f, "FocusGain"),
            EventType::FocusLost => write!(f, "FocusLost"),
            EventType::LayoutChanged => write!(f, "LayoutChanged"),
            EventType::StyleChanged => write!(f, "StyleChanged"),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: u64,
    pub target: WidgetId,
    pub bubbles: bool,
    pub cancelable: bool,
    pub stopped: bool,
    pub immediate_stopped: bool,
    pub data: EventData,
}

impl Event {
    pub fn new(event_type: EventType, timestamp: u64) -> Self {
        Self {
            event_type,
            timestamp,
            target: WidgetId::invalid(),
            bubbles: true,
            cancelable: true,
            stopped: false,
            immediate_stopped: false,
            data: EventData::None,
        }
    }
    
    pub fn with_target(mut self, target: WidgetId) -> Self {
        self.target = target;
        self
    }
    
    pub fn with_data(mut self, data: EventData) -> Self {
        self.data = data;
        self
    }
    
    pub fn stop_propagation(&mut self) {
        self.stopped = true;
    }
    
    pub fn stop_immediate_propagation(&mut self) {
        self.immediate_stopped = true;
    }
    
    pub fn prevent_default(&mut self) {
        // 标记事件已被取消
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub enum EventData {
    None,
    Touch(TouchData),
    Key(KeyData),
    Mouse(MouseData),
    Layout(LayoutData),
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TouchData {
    pub x: f32,
    pub y: f32,
    pub pointer_id: u32,
    pub pressure: f32,
}

impl TouchData {
    pub fn new(x: f32, y: f32, pointer_id: u32) -> Self {
        Self {
            x,
            y,
            pointer_id,
            pressure: 1.0,
        }
    }
    
    pub fn with_pressure(mut self, pressure: f32) -> Self {
        self.pressure = pressure;
        self
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KeyData {
    pub keycode: u32,
    pub modifiers: u32,
    pub unicode_char: u32,
}

impl KeyData {
    pub fn new(keycode: u32, modifiers: u32) -> Self {
        Self {
            keycode,
            modifiers,
            unicode_char: 0,
        }
    }
    
    pub fn with_unicode(mut self, unicode_char: u32) -> Self {
        self.unicode_char = unicode_char;
        self
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MouseData {
    pub x: f32,
    pub y: f32,
    pub button: MouseButton,
}

impl MouseData {
    pub fn new(x: f32, y: f32, button: MouseButton) -> Self {
        Self { x, y, button }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LayoutData {
    pub old_bounds: Rect,
    pub new_bounds: Rect,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Ignored,
    Handled,
    Stopped,
    ImmediateStop,
}

impl Default for EventResult {
    fn default() -> Self {
        Self::Ignored
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EventPhase {
    pub capturing: bool,
    pub bubbling: bool,
}

impl EventPhase {
    pub fn capturing() -> Self {
        Self { capturing: true, bubbling: false }
    }
    
    pub fn bubbling() -> Self {
        Self { capturing: false, bubbling: true }
    }
    
    pub fn at_target() -> Self {
        Self { capturing: false, bubbling: false }
    }
}