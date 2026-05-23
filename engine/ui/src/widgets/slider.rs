use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::thunk::{queue_callback, PendingCallback};

pub struct Slider {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    track_height: f32,
    thumb_width: f32,
    dragging: bool,
    drag_start_x: f32,
    drag_start_value: f32,
    on_change: Option<Box<dyn FnMut(f32) + Send + Sync>>,
    content_scale: f32,
}

impl Slider {
    pub fn new(min: f32, max: f32, value: f32) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 100.0, 24.0),
            style: Style::new()
                .with_background(Color::new(0.18, 0.18, 0.22, 1.0)),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            min,
            max,
            value,
            step: 0.0,
            track_height: 8.0,
            thumb_width: 20.0,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_value: 0.0,
            on_change: None,
            content_scale: 1.0,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
        self.flags.dirty_render = true;
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }

    pub fn set_range(&mut self, min: f32, max: f32) {
        self.min = min;
        self.max = max;
        self.value = self.value.clamp(min, max);
        self.flags.dirty_render = true;
    }

    pub fn set_step(&mut self, step: f32) {
        self.step = step;
    }

    pub fn set_on_change(&mut self, callback: Box<dyn FnMut(f32) + Send + Sync>) {
        self.on_change = Some(callback);
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.flags.dirty_render = true;
    }

    fn compute_value_from_x(&self, x: f32) -> f32 {
        let padding = 4.0 * self.content_scale;
        let track_start = padding;
        let track_end = self.layout.width - padding;
        let track_width = track_end - track_start;

        if track_width <= 0.0 {
            return self.min;
        }

        let ratio = (x - track_start) / track_width;
        let raw_value = self.min + ratio * (self.max - self.min);

        if self.step > 0.0 {
            let steps = ((raw_value - self.min) / self.step).round();
            self.min + steps * self.step
        } else {
            raw_value
        }
            .clamp(self.min, self.max)
    }

    fn thumb_center_x(&self) -> f32 {
        let padding = 4.0 * self.content_scale;
        let track_start = padding;
        let track_end = self.layout.width - padding;
        let track_width = track_end - track_start;

        if self.max == self.min || track_width <= 0.0 {
            return track_start;
        }

        let ratio = (self.value - self.min) / (self.max - self.min);
        track_start + ratio * track_width
    }
}

impl Widget for Slider {
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
    fn widget_type(&self) -> &'static str { "Slider" }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn flags(&self) -> WidgetFlags { self.flags }
    fn set_flags(&mut self, flags: WidgetFlags) { self.flags = flags; }

    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let height = (self.track_height * 2.0 + self.thumb_width * 0.5) * self.content_scale;
        (self.layout.width, height)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        let scale = self.content_scale;
        let padding = 4.0 * scale;
        let track_h = self.track_height * scale;
        let thumb_w = self.thumb_width * scale;
        let track_y = (height - track_h) / 2.0;

        // Background
        canvas.draw_rect(
            Rect::new(0.0, 0.0, width, height),
            &Style::new().with_background(Color::new(0.15, 0.15, 0.18, 1.0)),
        );

        // Track (dark rect)
        canvas.draw_rect(
            Rect::new(padding, track_y, width - padding * 2.0, track_h),
            &Style::new().with_background(Color::new(0.2, 0.2, 0.25, 1.0)),
        );

        // Filled portion (bright blue rect from left to thumb)
        let thumb_x = self.thumb_center_x();
        let fill_width = thumb_x - padding;
        if fill_width > 0.0 {
            canvas.draw_rect(
                Rect::new(padding, track_y, fill_width, track_h),
                &Style::new().with_background(Color::new(0.3, 0.6, 0.9, 1.0)),
            );
        }

        // Thumb circle/rect
        let thumb_h = track_h + 4.0 * scale;
        let thumb_y = (height - thumb_h) / 2.0;
        let thumb_color = match self.state {
            WidgetState::Hovered => Color::new(1.0, 1.0, 1.0, 1.0),
            WidgetState::Pressed => Color::new(0.8, 0.8, 0.85, 1.0),
            _ => Color::new(0.9, 0.9, 0.92, 1.0),
        };
        canvas.draw_rect(
            Rect::new(thumb_x - thumb_w / 2.0, thumb_y, thumb_w, thumb_h),
            &Style::new()
                .with_background(thumb_color)
                .with_border(Color::new(0.5, 0.5, 0.55, 1.0), 1.0, 1.0),
        );
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                if self.state == WidgetState::Disabled {
                    return EventResult::Ignored;
                }
                // Check if touch is near thumb or on track
                if let EventData::Touch(touch) = &event.data {
                    let thumb_x = self.thumb_center_x();
                    let scale = self.content_scale;
                    let thumb_w = self.thumb_width * scale;
                    let thumb_hit = (touch.x >= thumb_x - thumb_w && touch.x <= thumb_x + thumb_w)
                        || touch.x >= 4.0 * scale;
                    if thumb_hit {
                        self.dragging = true;
                        self.drag_start_x = touch.x;
                        self.drag_start_value = self.value;
                        self.set_state(WidgetState::Pressed);

                        // Jump value to touch position
                        let new_value = self.compute_value_from_x(touch.x);
                        if new_value != self.value {
                            self.value = new_value;
                            self.flags.dirty_render = true;
                            let widget_id = self.id.id;
                            let val = self.value;
                            queue_callback(PendingCallback::SliderChange { widget_id, value: val });
                            if let Some(callback) = &mut self.on_change {
                                callback(self.value);
                            }
                        }
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }

            EventType::TouchMove => {
                if self.dragging {
                    if let EventData::Touch(touch) = &event.data {
                        let new_value = self.compute_value_from_x(touch.x);
                        if new_value != self.value {
                            self.value = new_value;
                            self.flags.dirty_render = true;
                            let widget_id = self.id.id;
                            let val = self.value;
                            queue_callback(PendingCallback::SliderChange { widget_id, value: val });
                            if let Some(callback) = &mut self.on_change {
                                callback(self.value);
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
                    let delta = -wheel.delta_y * 0.01 * (self.max - self.min);
                    let mut new_value = (self.value + delta).clamp(self.min, self.max);
                    if self.step > 0.0 {
                        let steps = ((new_value - self.min) / self.step).round();
                        new_value = self.min + steps * self.step;
                    }
                    if new_value != self.value {
                        self.value = new_value;
                        self.flags.dirty_render = true;
                        let widget_id = self.id.id;
                        let val = self.value;
                        queue_callback(PendingCallback::SliderChange { widget_id, value: val });
                        if let Some(callback) = &mut self.on_change {
                            callback(self.value);
                        }
                    }
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }

            EventType::MouseEnter => {
                if self.state != WidgetState::Disabled && !self.dragging {
                    self.set_state(WidgetState::Hovered);
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }

            EventType::MouseLeave => {
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