use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use std::any::Any;

pub trait Widget: Send + Sync {
    fn id(&self) -> WidgetId;
    fn parent(&self) -> Option<WidgetId>;
    fn set_parent(&mut self, parent: WidgetId);

    fn children(&self) -> &[WidgetId];
    fn add_child(&mut self, child: WidgetId);
    fn remove_child(&mut self, child: WidgetId);

    fn layout(&self) -> &Layout;
    fn set_layout(&mut self, layout: Layout);

    fn style(&self) -> &Style;
    fn set_style(&mut self, style: Style);

    fn state(&self) -> WidgetState;
    fn set_state(&mut self, state: WidgetState);

    fn widget_type(&self) -> &'static str;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        (self.layout().width, self.layout().height)
    }

    fn hit_test(&self, point: Point) -> bool {
        let bounds = Rect::new(
            self.layout().x,
            self.layout().y,
            self.layout().width,
            self.layout().height,
        );
        bounds.contains(&point)
    }

    fn draw(&mut self, canvas: &mut Canvas);
    fn on_event(&mut self, event: &Event) -> EventResult;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
    Focused,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self::Normal
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WidgetFlags {
    pub dirty_layout: bool,
    pub dirty_style: bool,
    pub dirty_render: bool,
    pub dirty_children: bool,
    pub visible: bool,
    pub enabled: bool,
    pub clip_children: bool,
}

impl Default for WidgetFlags {
    fn default() -> Self {
        Self {
            dirty_layout: false,
            dirty_style: false,
            dirty_render: false,
            dirty_children: false,
            visible: true,
            enabled: true,
            clip_children: false,
        }
    }
}
