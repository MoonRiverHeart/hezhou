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
use crate::ui::layout::msdf_measurer::MsdfTextMeasurer;
use crate::ui::layout::text::TextMeasurer;
use crate::ui::layout::text::GlyphInfo;
use crate::ui::event::types::UIEvent;
use theme::Theme;
use std::sync::{Arc, Mutex};

pub struct BuildContext {
    pub theme: Theme,
    pub tree: WidgetTree,
    pub event_handlers: Vec<EventHandlerEntry>,
    pub msdf: Option<Arc<Mutex<MsdfTextMeasurer>>>,
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
            msdf: None,
        }
    }
    
    pub fn with_msdf(theme: Theme, msdf: Arc<Mutex<MsdfTextMeasurer>>) -> Self {
        BuildContext {
            theme,
            tree: WidgetTree::new(),
            event_handlers: Vec::new(),
            msdf: Some(msdf),
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
    
    pub fn take_event_handlers(&mut self) -> Vec<EventHandlerEntry> {
        std::mem::take(&mut self.event_handlers)
    }
    
    pub fn build(self) -> WidgetTree {
        self.tree
    }
    
    pub fn layout_text(&self, text: &str, font_size: f32) -> Option<Vec<GlyphInfo>> {
        if let Some(ref msdf) = self.msdf {
            let mut m = msdf.lock().unwrap();
            Some(m.layout_text(text, font_size, f32::MAX).glyphs)
        } else {
            None
        }
    }
}

pub trait Component {
    fn build(&mut self, ctx: &mut BuildContext) -> WidgetId;
}