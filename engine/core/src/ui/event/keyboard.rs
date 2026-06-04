use super::modifier::Modifiers;
use std::time::SystemTime;

/// 键盘事件
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardEvent {
    pub timestamp: SystemTime,
    pub key: Key,
    pub modifiers: Modifiers,
    pub event_type: KeyboardEventType,
    pub handled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardEventType {
    /// 按键按下
    Pressed,
    /// 按键释放
    Released,
    /// 重复按键（长按）
    Repeated,
    /// 文字输入（用于输入法）
    TextInput { text: String },
    /// 组合输入开始（IME）
    CompositionStart,
    /// 组合输入更新
    CompositionUpdate { text: String },
    /// 组合输入结束
    CompositionEnd { text: String },
}

/// 键盘按键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Character(char),
    Special(SpecialKey),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpecialKey {
    // 功能键
    Escape, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // 方向键
    Up, Down, Left, Right,
    // 修饰键
    Shift, Control, Alt, Super, Meta,
    // 编辑键
    Enter, Backspace, Tab, Space, Delete, Insert, Home, End, PageUp, PageDown,
    // 数字键盘
    NumLock, Keypad0, Keypad1, Keypad2, Keypad3, Keypad4,
    Keypad5, Keypad6, Keypad7, Keypad8, Keypad9,
    // 其他
    PrintScreen, ScrollLock, Pause, Menu, Help,
}