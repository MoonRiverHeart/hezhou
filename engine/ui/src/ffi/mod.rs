use crate::{WidgetTree, EventDispatcher};
use parking_lot::Mutex;
use std::sync::Arc;

pub type WidgetTreeHandle = *mut Arc<Mutex<WidgetTree>>;
pub type EventDispatcherHandle = *mut Arc<Mutex<EventDispatcher>>;
pub type ClickCallback = extern "C" fn(u64);

pub mod system;
pub mod event;
pub mod widget;
pub mod text_edit;
pub mod dropdown;
pub mod input_field;
pub mod preview;
pub mod tab;
pub mod tree;
pub mod popup_menu;
pub mod grid_view;
pub mod dialog;
pub mod file_browser;
pub mod misc;

pub use system::*;
pub use event::*;
pub use widget::*;
pub use text_edit::*;
pub use dropdown::*;
pub use input_field::*;
pub use preview::*;
pub use tab::*;
pub use tree::*;
pub use popup_menu::*;
pub use grid_view::*;
pub use dialog::*;
pub use file_browser::*;
pub use misc::*;