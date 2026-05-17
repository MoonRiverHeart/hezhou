#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformEvent {
    pub kind: PlatformEventKind,
    pub timestamp: u64,
    pub data: PlatformEventData,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union PlatformEventData {
    pub touch: TouchEvent,
    pub key: KeyEvent,
    pub char_event: CharEvent,
    pub mouse: MouseEvent,
    pub window: WindowEvent,
    pub lifecycle: LifecycleEvent,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformEventKind {
    Touch,
    Key,
    Char,
    Mouse,
    WindowResize,
    WindowClose,
    Lifecycle,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TouchEvent {
    pub action: TouchAction,
    pub x: f32,
    pub y: f32,
    pub pointer_id: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchAction {
    Begin = 0,
    Move = 1,
    End = 2,
    Cancel = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KeyEvent {
    pub action: KeyAction,
    pub keycode: KeyCode,
    pub modifiers: KeyModifiers,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CharEvent {
    pub codepoint: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Press = 0,
    Release = 1,
    Repeat = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    Unknown = 0,
    A = 1,
    B = 2,
    C = 3,
    D = 4,
    E = 5,
    F = 6,
    G = 7,
    H = 8,
    I = 9,
    J = 10,
    K = 11,
    L = 12,
    M = 13,
    N = 14,
    O = 15,
    P = 16,
    Q = 17,
    R = 18,
    S = 19,
    T = 20,
    U = 21,
    V = 22,
    W = 23,
    X = 24,
    Y = 25,
    Z = 26,
    Num0 = 27,
    Num1 = 28,
    Num2 = 29,
    Num3 = 30,
    Num4 = 31,
    Num5 = 32,
    Num6 = 33,
    Num7 = 34,
    Num8 = 35,
    Num9 = 36,
    Space = 37,
    Enter = 38,
    Escape = 39,
    Backspace = 40,
    Tab = 41,
    Shift = 42,
    Ctrl = 43,
    Alt = 44,
    Left = 45,
    Right = 46,
    Up = 47,
    Down = 48,
    Home = 1001,
    Back = 1002,
    Menu = 1003,
    VolumeUp = 1004,
    VolumeDown = 1005,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MouseEvent {
    pub action: MouseAction,
    pub button: MouseButton,
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
    Press = 0,
    Release = 1,
    Move = 2,
    Scroll = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    None = 3,
    Left = 0,
    Right = 1,
    Middle = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WindowEvent {
    pub width: i32,
    pub height: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LifecycleEvent {
    pub state: LifecycleState,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleState {
    Create = 0,
    Start = 1,
    Resume = 2,
    Pause = 3,
    Stop = 4,
    Destroy = 5,
}

unsafe impl Send for PlatformEvent {}
unsafe impl Send for TouchEvent {}
unsafe impl Send for KeyEvent {}
unsafe impl Send for CharEvent {}
unsafe impl Send for MouseEvent {}
unsafe impl Send for WindowEvent {}
unsafe impl Send for LifecycleEvent {}
