#[repr(C)]
pub struct TouchEvent {
    pub action: TouchAction,
    pub x: f32,
    pub y: f32,
    pub timestamp: u64,
    pub pointer_id: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchAction {
    Down = 0,
    Move = 1,
    Up = 2,
    Cancel = 3,
}

#[repr(C)]
pub struct KeyEvent {
    pub action: KeyAction,
    pub keycode: i32,
    pub modifiers: i32,
    pub timestamp: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Press = 0,
    Release = 1,
}

#[repr(C)]
pub struct SizeEvent {
    pub width: i32,
    pub height: i32,
}

#[repr(C)]
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

unsafe impl Send for TouchEvent {}
unsafe impl Send for KeyEvent {}
unsafe impl Send for SizeEvent {}
unsafe impl Send for LifecycleEvent {}