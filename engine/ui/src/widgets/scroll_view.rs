use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::thunk::{queue_callback, PendingCallback};

pub struct ScrollView {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    scroll_offset_y: f32,
    scroll_offset_x: f32,
    content_height: f32,
    content_width: f32,
    v_scrollbar_dragging: bool,
    v_scrollbar_drag_start_y: f32,
    v_scrollbar_drag_start_offset: f32,
    show_v_scrollbar: bool,
    show_h_scrollbar: bool,
    on_scroll: Option<Box<dyn FnMut(f32) + Send + Sync>>,
    content_scale: f32,
}

impl ScrollView {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, width, height),
            style: Style::new()
                .with_background(Color::new(0.16, 0.16, 0.20, 1.0)),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            scroll_offset_y: 0.0,
            scroll_offset_x: 0.0,
            content_height: 0.0,
            content_width: 0.0,
            v_scrollbar_dragging: false,
            v_scrollbar_drag_start_y: 0.0,
            v_scrollbar_drag_start_offset: 0.0,
            show_v_scrollbar: true,
            show_h_scrollbar: false,
            on_scroll: None,
            content_scale: 1.0,
        }
    }

    pub fn set_scroll_offset_y(&mut self, offset: f32) {
        let max_scroll = self.max_scroll_y();
        self.scroll_offset_y = offset.clamp(0.0, max_scroll);
        self.flags.dirty_render = true;
        self.flags.dirty_layout = true;
    }

    pub fn get_scroll_offset_y(&self) -> f32 {
        self.scroll_offset_y
    }

    pub fn set_content_size(&mut self, width: f32, height: f32) {
        self.content_width = width;
        self.content_height = height;
        // Clamp scroll offset
        let max = self.max_scroll_y();
        if self.scroll_offset_y > max {
            self.scroll_offset_y = max;
        }
        self.flags.dirty_render = true;
    }

    pub fn set_show_scrollbars(&mut self, show_v: bool, show_h: bool) {
        self.show_v_scrollbar = show_v;
        self.show_h_scrollbar = show_h;
        self.flags.dirty_render = true;
    }

    pub fn set_on_scroll(&mut self, callback: Box<dyn FnMut(f32) + Send + Sync>) {
        self.on_scroll = Some(callback);
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.flags.dirty_render = true;
    }

    fn max_scroll_y(&self) -> f32 {
        let viewport_h = self.layout.height;
        if self.content_height <= viewport_h {
            0.0
        } else {
            self.content_height - viewport_h
        }
    }

    fn scrollbar_track_height(&self) -> f32 {
        self.layout.height - 4.0 * self.content_scale
    }

    fn scrollbar_thumb_height(&self) -> f32 {
        let viewport_h = self.layout.height;
        let track_h = self.scrollbar_track_height();
        if self.content_height <= 0.0 || viewport_h <= 0.0 {
            track_h
        } else {
            let ratio = viewport_h / self.content_height;
            let thumb = ratio * track_h;
            thumb.max(30.0 * self.content_scale)
        }
    }

    fn scrollbar_thumb_y(&self) -> f32 {
        let max_scroll = self.max_scroll_y();
        let track_h = self.scrollbar_track_height();
        let thumb_h = self.scrollbar_thumb_height();

        if max_scroll <= 0.0 || track_h <= 0.0 {
            2.0 * self.content_scale
        } else {
            let ratio = self.scroll_offset_y / max_scroll;
            2.0 * self.content_scale + ratio * (track_h - thumb_h)
        }
    }

    fn is_scrollbar_thumb_hit(&self, x: f32, y: f32) -> bool {
        if !self.show_v_scrollbar {
            return false;
        }
        let scale = self.content_scale;
        let bar_width = 12.0 * scale;
        let bar_x = self.layout.width - bar_width - 2.0 * scale;

        let thumb_y = self.scrollbar_thumb_y();
        let thumb_h = self.scrollbar_thumb_height();

        x >= bar_x && x <= bar_x + bar_width && y >= thumb_y && y <= thumb_y + thumb_h
    }

    fn is_scrollbar_track_hit(&self, x: f32) -> bool {
        if !self.show_v_scrollbar {
            return false;
        }
        let scale = self.content_scale;
        let bar_width = 12.0 * scale;
        let bar_x = self.layout.width - bar_width - 2.0 * scale;
        x >= bar_x && x <= bar_x + bar_width
    }
}

impl Widget for ScrollView {
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
    fn widget_type(&self) -> &'static str { "ScrollView" }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn flags(&self) -> WidgetFlags { self.flags }
    fn set_flags(&mut self, flags: WidgetFlags) { self.flags = flags; }

    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let w = if self.layout.width > 0.0 { self.layout.width } else { 300.0 };
        let h = if self.layout.height > 0.0 { self.layout.height } else { 400.0 };
        (w, h)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        let scale = self.content_scale;

        // Viewport background
        canvas.draw_rect(
            Rect::new(0.0, 0.0, width, height),
            &Style::new().with_background(Color::new(0.16, 0.16, 0.20, 1.0)),
        );

        // Vertical scrollbar
        if self.show_v_scrollbar && self.content_height > height {
            let bar_width = 12.0 * scale;
            let bar_x = width - bar_width - 2.0 * scale;
            let track_h = self.scrollbar_track_height();
            let thumb_y = self.scrollbar_thumb_y();
            let thumb_h = self.scrollbar_thumb_height();

            // Track background
            canvas.draw_rect(
                Rect::new(bar_x, 2.0 * scale, bar_width, track_h),
                &Style::new().with_background(Color::new(0.2, 0.2, 0.25, 1.0)),
            );

            // Thumb
            let thumb_color = if self.v_scrollbar_dragging {
                Color::new(0.55, 0.55, 0.6, 1.0)
            } else {
                Color::new(0.4, 0.4, 0.45, 1.0)
            };
            canvas.draw_rect(
                Rect::new(bar_x, thumb_y, bar_width, thumb_h),
                &Style::new()
                    .with_background(thumb_color)
                    .with_border(Color::new(0.5, 0.5, 0.55, 1.0), 1.0, 1.0),
            );
        }
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::MouseWheel => {
                if let EventData::Wheel(wheel) = &event.data {
                    let max_scroll = self.max_scroll_y();
                    if max_scroll > 0.0 {
                        let delta = -wheel.delta_y * 30.0 * self.content_scale;
                        let new_offset = (self.scroll_offset_y + delta).clamp(0.0, max_scroll);
                        if new_offset != self.scroll_offset_y {
                            self.scroll_offset_y = new_offset;
                            self.flags.dirty_render = true;
                            self.flags.dirty_layout = true;
                            let widget_id = self.id.id;
                            let offset = self.scroll_offset_y;
                            queue_callback(PendingCallback::ScrollViewScroll { widget_id, offset_y: offset });
                            if let Some(callback) = &mut self.on_scroll {
                                callback(self.scroll_offset_y);
                            }
                        }
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }

            EventType::TouchBegin => {
                if let EventData::Touch(touch) = &event.data {
                    // Check scrollbar thumb hit first
                    if self.is_scrollbar_thumb_hit(touch.x, touch.y) {
                        self.v_scrollbar_dragging = true;
                        self.v_scrollbar_drag_start_y = touch.y;
                        self.v_scrollbar_drag_start_offset = self.scroll_offset_y;
                        return EventResult::Handled;
                    }
                    // Check scrollbar track hit — jump to position
                    if self.is_scrollbar_track_hit(touch.x) {
                        let track_h = self.scrollbar_track_height();
                        let thumb_h = self.scrollbar_thumb_height();
                        let max_scroll = self.max_scroll_y();
                        let track_start = 2.0 * self.content_scale;
                        let ratio = ((touch.y - track_start - thumb_h / 2.0) / (track_h - thumb_h)).clamp(0.0, 1.0);
                        let new_offset = ratio * max_scroll;
                        self.scroll_offset_y = new_offset;
                        self.flags.dirty_render = true;
                        self.flags.dirty_layout = true;
                        let widget_id = self.id.id;
                        let offset = self.scroll_offset_y;
                        queue_callback(PendingCallback::ScrollViewScroll { widget_id, offset_y: offset });
                        if let Some(callback) = &mut self.on_scroll {
                            callback(self.scroll_offset_y);
                        }
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }

            EventType::TouchMove => {
                if self.v_scrollbar_dragging {
                    if let EventData::Touch(touch) = &event.data {
                        let track_h = self.scrollbar_track_height();
                        let thumb_h = self.scrollbar_thumb_height();
                        let max_scroll = self.max_scroll_y();

                        let delta_y = touch.y - self.v_scrollbar_drag_start_y;
                        let scroll_range = track_h - thumb_h;
                        if scroll_range > 0.0 && max_scroll > 0.0 {
                            let scroll_delta = delta_y * (max_scroll / scroll_range);
                            let new_offset = (self.v_scrollbar_drag_start_offset + scroll_delta).clamp(0.0, max_scroll);
                            if new_offset != self.scroll_offset_y {
                                self.scroll_offset_y = new_offset;
                                self.flags.dirty_render = true;
                                self.flags.dirty_layout = true;
                                let widget_id = self.id.id;
                                let offset = self.scroll_offset_y;
                                queue_callback(PendingCallback::ScrollViewScroll { widget_id, offset_y: offset });
                                if let Some(callback) = &mut self.on_scroll {
                                    callback(self.scroll_offset_y);
                                }
                            }
                        }
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }

            EventType::TouchEnd => {
                if self.v_scrollbar_dragging {
                    self.v_scrollbar_dragging = false;
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }
}