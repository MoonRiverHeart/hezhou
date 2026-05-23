use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::thunk::{queue_callback, PendingCallback};

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitOrientation {
    Horizontal = 0, // left/right
    Vertical = 1,   // top/bottom
}

pub struct SplitView {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    pub split_ratio: f32,
    pub min_ratio: f32,
    pub max_ratio: f32,
    pub divider_thickness: f32,
    pub orientation: SplitOrientation,
    dragging: bool,
    drag_start_x: f32,
    drag_start_y: f32,
    drag_start_ratio: f32,
    divider_hovered: bool,
    on_ratio_change: Option<Box<dyn FnMut(f32) + Send + Sync>>,
    pub content_scale: f32,
}

impl SplitView {
    pub fn new(width: f32, height: f32, orientation: SplitOrientation) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, width, height),
            style: Style::new()
                .with_background(Color::new(0.18, 0.18, 0.22, 1.0)),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            split_ratio: 0.5,
            min_ratio: 0.1,
            max_ratio: 0.9,
            divider_thickness: 4.0,
            orientation,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_ratio: 0.0,
            divider_hovered: false,
            on_ratio_change: None,
            content_scale: 1.0,
        }
    }

    pub fn set_split_ratio(&mut self, ratio: f32) {
        self.split_ratio = ratio.clamp(self.min_ratio, self.max_ratio);
        self.flags.dirty_render = true;
        self.flags.dirty_layout = true;
    }

    pub fn get_split_ratio(&self) -> f32 {
        self.split_ratio
    }

    pub fn set_min_ratio(&mut self, min: f32) {
        self.min_ratio = min;
        self.split_ratio = self.split_ratio.clamp(self.min_ratio, self.max_ratio);
        self.flags.dirty_render = true;
    }

    pub fn set_max_ratio(&mut self, max: f32) {
        self.max_ratio = max;
        self.split_ratio = self.split_ratio.clamp(self.min_ratio, self.max_ratio);
        self.flags.dirty_render = true;
    }

    pub fn set_on_ratio_change(&mut self, callback: Box<dyn FnMut(f32) + Send + Sync>) {
        self.on_ratio_change = Some(callback);
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.flags.dirty_render = true;
    }

    fn divider_position(&self) -> f32 {
        let scale = self.content_scale;
        let thickness = self.divider_thickness * scale;
        match self.orientation {
            SplitOrientation::Horizontal => {
                self.layout.width * self.split_ratio - thickness / 2.0
            }
            SplitOrientation::Vertical => {
                self.layout.height * self.split_ratio - thickness / 2.0
            }
        }
    }

    fn is_in_divider_area(&self, x: f32, y: f32) -> bool {
        let scale = self.content_scale;
        let padding = 4.0 * scale;
        let thickness = self.divider_thickness * scale;
        match self.orientation {
            SplitOrientation::Horizontal => {
                let div_x = self.divider_position();
                x >= div_x - padding && x <= div_x + thickness + padding
            }
            SplitOrientation::Vertical => {
                let div_y = self.divider_position();
                y >= div_y - padding && y <= div_y + thickness + padding
            }
        }
    }

    fn compute_ratio_from_position(&self, x: f32, y: f32) -> f32 {
        match self.orientation {
            SplitOrientation::Horizontal => {
                if self.layout.width <= 0.0 {
                    return self.split_ratio;
                }
                (x / self.layout.width).clamp(self.min_ratio, self.max_ratio)
            }
            SplitOrientation::Vertical => {
                if self.layout.height <= 0.0 {
                    return self.split_ratio;
                }
                (y / self.layout.height).clamp(self.min_ratio, self.max_ratio)
            }
        }
    }
}

impl Widget for SplitView {
    fn id(&self) -> WidgetId { self.id }
    fn parent(&self) -> Option<WidgetId> {
        if self.parent_id.is_valid() { Some(self.parent_id) } else { None }
    }
    fn set_parent(&mut self, parent: WidgetId) { self.parent_id = parent; }
    fn children(&self) -> &[WidgetId] { &self.children }
    fn add_child(&mut self, child: WidgetId) { self.children.push(child); }
    fn remove_child(&mut self, child: WidgetId) { self.children.retain(|c| *c != child); }
    fn layout(&self) -> &Layout { &self.layout }
    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }
    fn style(&self) -> &Style { &self.style }
    fn set_style(&mut self, style: Style) {
        self.style = style;
        self.flags.dirty_style = true;
        self.flags.dirty_render = true;
    }
    fn state(&self) -> WidgetState { self.state }
    fn set_state(&mut self, state: WidgetState) {
        self.state = state;
        self.flags.dirty_render = true;
    }
    fn widget_type(&self) -> &'static str { "SplitView" }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn flags(&self) -> WidgetFlags { self.flags }
    fn set_flags(&mut self, flags: WidgetFlags) { self.flags = flags; }

    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        (self.layout.width, self.layout.height)
    }

    fn hit_test(&self, point: Point) -> bool {
        let bounds = Rect::new(0.0, 0.0, self.layout.width, self.layout.height);
        if !bounds.contains(&point) {
            return false;
        }
        self.is_in_divider_area(point.x, point.y)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        let scale = self.content_scale;
        let thickness = self.divider_thickness * scale;

        // Background
        canvas.draw_rect(
            Rect::new(0.0, 0.0, width, height),
            &Style::new().with_background(Color::new(0.18, 0.18, 0.22, 1.0)),
        );

        // Divider bar
        let divider_color = if self.dragging {
            Color::new(0.5, 0.5, 0.55, 1.0)
        } else if self.divider_hovered {
            Color::new(0.5, 0.5, 0.55, 1.0)
        } else {
            Color::new(0.35, 0.35, 0.4, 1.0)
        };

        match self.orientation {
            SplitOrientation::Horizontal => {
                let div_x = self.divider_position();
                canvas.draw_rect(
                    Rect::new(div_x, 0.0, thickness, height),
                    &Style::new().with_background(divider_color),
                );
            }
            SplitOrientation::Vertical => {
                let div_y = self.divider_position();
                canvas.draw_rect(
                    Rect::new(0.0, div_y, width, thickness),
                    &Style::new().with_background(divider_color),
                );
            }
        }
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                if self.state == WidgetState::Disabled {
                    return EventResult::Ignored;
                }
                if let EventData::Touch(touch) = &event.data {
                    if self.is_in_divider_area(touch.x, touch.y) {
                        self.dragging = true;
                        self.drag_start_x = touch.x;
                        self.drag_start_y = touch.y;
                        self.drag_start_ratio = self.split_ratio;
                        self.set_state(WidgetState::Pressed);
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }

            EventType::TouchMove => {
                if self.dragging {
                    if let EventData::Touch(touch) = &event.data {
                        let new_ratio = self.compute_ratio_from_position(touch.x, touch.y);
                        if new_ratio != self.split_ratio {
                            self.split_ratio = new_ratio;
                            self.flags.dirty_render = true;
                            self.flags.dirty_layout = true;
                            let widget_id = self.id.id;
                            let ratio = self.split_ratio;
                            queue_callback(PendingCallback::SplitViewRatioChange { widget_id, ratio });
                            if let Some(callback) = &mut self.on_ratio_change {
                                callback(self.split_ratio);
                            }
                        }
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }

            EventType::TouchEnd => {
                if self.dragging {
                    self.dragging = false;
                    self.set_state(WidgetState::Normal);
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }

            EventType::MouseWheel => {
                if let EventData::Wheel(wheel) = &event.data {
                    let delta = -wheel.delta_y * 0.02;
                    let new_ratio = (self.split_ratio + delta).clamp(self.min_ratio, self.max_ratio);
                    if new_ratio != self.split_ratio {
                        self.split_ratio = new_ratio;
                        self.flags.dirty_render = true;
                        self.flags.dirty_layout = true;
                        let widget_id = self.id.id;
                        let ratio = self.split_ratio;
                        queue_callback(PendingCallback::SplitViewRatioChange { widget_id, ratio });
                        if let Some(callback) = &mut self.on_ratio_change {
                            callback(self.split_ratio);
                        }
                    }
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }

            EventType::MouseEnter => {
                if let EventData::Touch(touch) = &event.data {
                    if self.is_in_divider_area(touch.x, touch.y) {
                        self.divider_hovered = true;
                        if self.state != WidgetState::Disabled && !self.dragging {
                            self.set_state(WidgetState::Hovered);
                        }
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }

            EventType::MouseLeave => {
                self.divider_hovered = false;
                if self.state == WidgetState::Hovered {
                    self.set_state(WidgetState::Normal);
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }
}