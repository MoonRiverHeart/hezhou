pub mod dispatcher;
pub mod drag;
pub mod keyboard;
pub mod lifecycle;
pub mod modifier;
pub mod mouse;
pub mod touch;
pub mod types;
pub mod widget;
pub mod window;


pub use mouse::{MouseEvent, MouseEventType, MouseButton};
pub use keyboard::{KeyboardEvent, KeyboardEventType, Key, SpecialKey};
pub use modifier::Modifiers;
pub use mouse::Point;
pub use dispatcher::{EventQueue, EventDispatcher};
pub use dispatcher::EventHandler;
pub use types::UIEvent;  // 假设 UIEvent 在 types.rs 中