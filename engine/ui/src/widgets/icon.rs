use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::icons;

/// SVG 图标控件。图标以矢量线段绘制，不依赖位图，天然适配 DPI。
pub struct IconWidget {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: WidgetFlags,
    icon_id: u32,
    color: Color,
    stroke_width: f32,
}

impl IconWidget {
    pub fn new(icon_id: u32) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 16.0, 16.0),
            style: Style::new(),
            state: WidgetState::Normal,
            flags: WidgetFlags::default(),
            icon_id,
            color: crate::theme::colors().text_secondary,
            stroke_width: 1.5,
        }
    }

    pub fn set_icon(&mut self, icon_id: u32) {
        self.icon_id = icon_id;
        self.flags.dirty_render = true;
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
        self.flags.dirty_render = true;
    }

    pub fn set_stroke_width(&mut self, width: f32) {
        self.stroke_width = width;
        self.flags.dirty_render = true;
    }
}

impl Widget for IconWidget {
    fn id(&self) -> WidgetId { self.id }
    fn parent(&self) -> Option<WidgetId> { Some(self.parent_id) }
    fn set_parent(&mut self, parent: WidgetId) { self.parent_id = parent; }
    fn children(&self) -> &[WidgetId] { &self.children }
    fn add_child(&mut self, child: WidgetId) { self.children.push(child); }
    fn remove_child(&mut self, child: WidgetId) { self.children.retain(|c| *c != child); }
    fn layout(&self) -> &Layout { &self.layout }
    fn set_layout(&mut self, layout: Layout) { self.layout = layout; }
    fn style(&self) -> &Style { &self.style }
    fn set_style(&mut self, style: Style) { self.style = style; }
    fn state(&self) -> WidgetState { self.state }
    fn set_state(&mut self, state: WidgetState) { self.state = state; }
    fn flags(&self) -> WidgetFlags { self.flags }
    fn set_flags(&mut self, flags: WidgetFlags) { self.flags = flags; }
    fn widget_type(&self) -> &'static str { "Icon" }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        (self.layout.width, self.layout.height)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let Some(def) = icons::icon(self.icon_id) else { return; };
        let w = self.layout.width;
        let h = self.layout.height;
        let scale = crate::thunk::ui_get_content_scale();
        let stroke = (self.stroke_width * scale).max(1.0);
        for &(x1, y1, x2, y2) in def.segments {
            canvas.draw_line(
                Point::new(x1 * w, y1 * h),
                Point::new(x2 * w, y2 * h),
                self.color,
                stroke,
            );
        }
    }

    fn on_event(&mut self, _event: &Event) -> EventResult { EventResult::Ignored }
}
