pub mod theme;
pub mod text;
pub mod button;
pub mod text_input;
pub mod text_area;
pub mod list;
pub mod grid;
pub mod container;

use crate::ui::layout::widget::*;
use crate::ui::layout::style::*;
use crate::ui::event::types::UIEvent;
use crate::ui::event::mouse::MouseEventType;
use crate::ui::event::keyboard::KeyboardEventType;
use theme::Theme;

/// 构建上下文
pub struct BuildContext {
    pub theme: Theme,
    pub tree: WidgetTree,
    pub event_handlers: Vec<EventHandlerEntry>,
}

pub struct EventHandlerEntry {
    pub widget_id: WidgetId,
    pub handler: Box<dyn Fn(&UIEvent)>,
}

impl BuildContext {
    pub fn new(theme: Theme) -> Self {
        BuildContext {
            theme,
            tree: WidgetTree::new(),
            event_handlers: Vec::new(),
        }
    }
    
    pub fn create_node(&mut self, widget_type: WidgetType, style: Style) -> WidgetId {
        self.tree.create_node(widget_type, style)
    }
    
    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        self.tree.add_child(parent, child);
    }
    
    pub fn set_root(&mut self, node_id: WidgetId) {
        self.tree.set_root(node_id);
    }
    
    pub fn on_event(&mut self, widget_id: WidgetId, handler: impl Fn(&UIEvent) + 'static) {
        self.event_handlers.push(EventHandlerEntry {
            widget_id,
            handler: Box::new(handler),
        });
    }
    
    pub fn build(self) -> WidgetTree {
        self.tree
    }
}

pub trait Component {
    fn build(&self, ctx: &mut BuildContext) -> WidgetId;
}

pub fn build_component(ctx: &mut BuildContext, component: &dyn Component) -> WidgetId {
    component.build(ctx)
}