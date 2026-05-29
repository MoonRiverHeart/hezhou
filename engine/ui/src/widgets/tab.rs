use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use crate::thunk::*;
use hezhou_dfx::*;

pub struct Tab {
    title: String,
    content_id: WidgetId,
    closable: bool,
}

pub struct TabWidget {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    tabs: Vec<Tab>,
    active_index: usize,
    tab_bar_height: f32,
    tab_width: f32,
    hovered_tab_index: Option<usize>,
    pressed_tab_index: Option<usize>,
    content_scale: f32,
    /// 缓存计算好的tab宽度（draw阶段计算，get_tab_rect和hit_test_tab使用）
    cached_tab_width: f32,
    measured_tab_text_widths: Vec<f32>,
    /// 所有tab标题拼接的文本，用于预栅格化字体字形
    all_text: String,
}

impl TabWidget {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 300.0, 400.0),
            style: Style::new()
                .with_background(Color::new(0.15, 0.15, 0.15, 1.0)),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            tabs: Vec::new(),
            active_index: 0,
            tab_bar_height: 30.0,
            tab_width: 100.0,
            hovered_tab_index: None,
            pressed_tab_index: None,
            content_scale: 1.0,
            cached_tab_width: 0.0,
            measured_tab_text_widths: Vec::new(),
            all_text: String::new(),
        }
    }

    pub fn with_layout(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.layout = Layout::new(x, y, width, height);
        self
    }

    pub fn add_tab(&mut self, title: &str, content_id: WidgetId, closable: bool) -> usize {
        let index = self.tabs.len();
        self.tabs.push(Tab {
            title: title.to_string(),
            content_id,
            closable,
        });
        if !self.children.contains(&content_id) {
            self.children.push(content_id);
        }
        self.rebuild_all_text();
        dfx_debug!("TabWidget", "AddTab: index={}, title={}, content_id={}, closable={}", 
            index, title, content_id.id, closable);
        index
    }

    pub fn remove_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            let tab = self.tabs.remove(index);
            self.children.retain(|c| *c != tab.content_id);
            if self.active_index >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_index = self.tabs.len() - 1;
            }
            self.rebuild_all_text();
            dfx_debug!("TabWidget", "RemoveTab: index={}, removed title={}", index, tab.title);
        }
    }

    fn rebuild_all_text(&mut self) {
        self.all_text.clear();
        for tab in &self.tabs {
            self.all_text.push_str(&tab.title);
        }
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = index;
            dfx_debug!("TabWidget", "SetActive: index={}", index);
        }
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn get_content_area(&self) -> Rect {
        Rect::new(
            0.0,
            self.tab_bar_height,
            self.layout.width,
            self.layout.height - self.tab_bar_height,
        )
    }

    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
    }

    pub fn content_scale(&self) -> f32 {
        self.content_scale
    }

    pub fn tab_bar_height(&self) -> f32 {
        self.tab_bar_height
    }

    fn get_tab_rect(&self, index: usize) -> Rect {
        let scaled_height = self.tab_bar_height * self.content_scale;
        let scaled_width = if self.cached_tab_width > 0.0 {
            self.cached_tab_width
        } else {
            let tab_count = self.tabs.len().max(1);
            let max_scaled_width = self.tab_width * self.content_scale;
            f32::min(max_scaled_width, self.layout.width / tab_count as f32)
        };
        let x = index as f32 * scaled_width;
        Rect::new(x, 0.0, scaled_width, scaled_height)
    }

    /// 截断文字并添加省略号（如果超出最大宽度）
    /// 当宽度不足以显示省略号截断时，仍然显示原始文字（溢出比消失好）
    fn truncate_text(text: &str, font_size: f32, text_width: f32) -> String {
        if text_width <= 0.0 {
            return text.to_string();
        }
        let ellipsis_width = font_size * 1.5;
        let max_text_width = text_width - ellipsis_width;
        if max_text_width <= 0.0 {
            return text.to_string();
        }
        let mut result = String::new();
        let mut current_width = 0.0;
        for ch in text.chars() {
            let ch_width = if ch.len_utf8() > 1 { font_size } else { font_size * 0.6 };
            if current_width + ch_width > max_text_width {
                result.push_str("...");
                return result;
            }
            current_width += ch_width;
            result.push(ch);
        }
        result
    }

    fn get_close_button_rect(&self, tab_rect: &Rect) -> Rect {
        let size = 14.0 * self.content_scale;
        let margin = 5.0 * self.content_scale;
        Rect::new(
            tab_rect.x + tab_rect.width - size - margin,
            tab_rect.y + (tab_rect.height - size) / 2.0,
            size,
            size,
        )
    }

    fn hit_test_tab(&self, local_x: f32, local_y: f32) -> Option<usize> {
        if local_y < 0.0 || local_y >= self.tab_bar_height * self.content_scale {
            return None;
        }
        let scaled_width = if self.cached_tab_width > 0.0 {
            self.cached_tab_width
        } else {
            let tab_count = self.tabs.len().max(1);
            f32::min(self.tab_width * self.content_scale, self.layout.width / tab_count as f32)
        };
        let index = (local_x / scaled_width) as usize;
        if index < self.tabs.len() {
            Some(index)
        } else {
            None
        }
    }

    fn hit_test_close_button(&self, index: usize, local_x: f32, local_y: f32) -> bool {
        if index >= self.tabs.len() || !self.tabs[index].closable {
            return false;
        }
        let tab_rect = self.get_tab_rect(index);
        let close_rect = self.get_close_button_rect(&tab_rect);
        close_rect.contains(&Point::new(local_x, local_y))
    }

    fn trigger_select_callback(&mut self, index: usize) {
        queue_callback(PendingCallback::TabSelect { widget_id: self.id.id, index });
    }

    fn trigger_close_callback(&mut self, index: usize) {
        queue_callback(PendingCallback::TabClose { widget_id: self.id.id, index });
    }
}

impl Widget for TabWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn parent(&self) -> Option<WidgetId> {
        if self.parent_id.is_valid() {
            Some(self.parent_id)
        } else {
            None
        }
    }

    fn set_parent(&mut self, parent: WidgetId) {
        self.parent_id = parent;
    }

    fn children(&self) -> &[WidgetId] {
        &self.children
    }

    fn add_child(&mut self, child: WidgetId) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }

    fn remove_child(&mut self, child: WidgetId) {
        self.children.retain(|c| *c != child);
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }

    fn style(&self) -> &Style {
        &self.style
    }

    fn set_style(&mut self, style: Style) {
        self.style = style;
        self.flags.dirty_style = true;
        self.flags.dirty_render = true;
    }

    fn state(&self) -> WidgetState {
        self.state
    }

    fn set_state(&mut self, state: WidgetState) {
        self.state = state;
        self.flags.dirty_render = true;
    }

    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        let w = if self.layout.width > 0.0 { self.layout.width } else { 300.0 };
        let h = if self.layout.height > 0.0 { self.layout.height } else { 400.0 };
        (w, h)
    }

    fn widget_type(&self) -> &'static str {
        "TabWidget"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_text(&self) -> Option<&str> {
        Some(&self.all_text)
    }

    fn flags(&self) -> WidgetFlags {
        self.flags
    }

    fn set_flags(&mut self, flags: WidgetFlags) {
        self.flags = flags;
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        let scaled_tab_bar_height = self.tab_bar_height * self.content_scale;
        let font_size = 14.0 * self.content_scale;
        let tab_padding_x = 10.0 * self.content_scale;
        let tab_count = self.tabs.len().max(1);
        let max_scaled_width = self.tab_width * self.content_scale;
        // 自适应：均分宽度，但保证至少能显示2个中文字
        let min_tab_width = 2.0 * font_size + tab_padding_x;
        let evenly = width / tab_count as f32;
        let scaled_tab_width = if evenly >= min_tab_width {
            f32::min(max_scaled_width, evenly)
        } else {
            f32::max(evenly, min_tab_width)
        };
        // 缓存tab宽度供get_tab_rect和hit_test_tab使用
        self.cached_tab_width = scaled_tab_width;

        let tab_bar_style = Style::new()
            .with_background(Color::new(0.15, 0.15, 0.15, 1.0));
        canvas.draw_rect(Rect::new(0.0, 0.0, width, scaled_tab_bar_height), &tab_bar_style);

        for (i, tab) in self.tabs.iter().enumerate() {
            let tab_rect = self.get_tab_rect(i);
            
            let tab_bg_color = if i == self.active_index {
                Color::new(0.25, 0.25, 0.25, 1.0)
            } else if self.hovered_tab_index == Some(i) {
                Color::new(0.22, 0.22, 0.22, 1.0)
            } else {
                Color::new(0.2, 0.2, 0.2, 1.0)
            };

            let tab_style = Style::new()
                .with_background(tab_bg_color)
                .with_border(Color::new(0.3, 0.3, 0.3, 1.0), 1.0, 0.0);
            canvas.draw_rect(tab_rect, &tab_style);

            // Dog-ear fold on top-right corner of tab
            let fold_size = 6.0 * self.content_scale;
            let fold_x = tab_rect.x + tab_rect.width;
            let fold_y = tab_rect.y;
            let fold_shadow_color = if i == self.active_index {
                Color::new(0.18, 0.18, 0.18, 1.0)
            } else {
                Color::new(0.15, 0.15, 0.15, 1.0)
            };
            let fold_color = if i == self.active_index {
                Color::new(0.35, 0.35, 0.35, 1.0)
            } else {
                Color::new(0.28, 0.28, 0.28, 1.0)
            };
            canvas.draw_triangle(
                Point::new(fold_x + 1.0 * self.content_scale, fold_y + 1.0 * self.content_scale),
                Point::new(fold_x - fold_size + 1.0 * self.content_scale, fold_y + 1.0 * self.content_scale),
                Point::new(fold_x + 1.0 * self.content_scale, fold_y + fold_size + 1.0 * self.content_scale),
                fold_shadow_color,
            );
            canvas.draw_triangle(
                Point::new(fold_x, fold_y),
                Point::new(fold_x - fold_size, fold_y),
                Point::new(fold_x, fold_y + fold_size),
                fold_color,
            );

            // 文字自适应：根据可用宽度截断文字并添加省略号
            let text_available_width = if tab.closable {
                tab_rect.width - 24.0 * self.content_scale - 5.0 * self.content_scale
            } else {
                tab_rect.width - tab_padding_x
            };
            
            let display_text = Self::truncate_text(&tab.title, font_size, text_available_width);

            // ===== 一次性诊断：输出每个tab的draw参数 =====
            static TAB_DIAG_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let tab_diag = TAB_DIAG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if tab_diag < 30 {
                dfx_info!("TabDiag", "tab[{}] title='{}' display='{}' tab_rect=({:.1},{:.1},{:.1},{:.1}) text_avail_w={:.1} font_size={:.1}",
                    i, tab.title, display_text, tab_rect.x, tab_rect.y, tab_rect.width, tab_rect.height, text_available_width, font_size);
            }

            let text_style = TextStyle::new()
                .with_size(font_size)
                .with_color(Color::white())
                .with_alignment(TextAlignment {
                    horizontal: HorizontalAlignment::Center,
                    vertical: VerticalAlignment::Center,
                });

            let text_rect = Rect::new(
                tab_rect.x + 5.0 * self.content_scale,
                tab_rect.y,
                text_available_width,
                tab_rect.height,
            );
            canvas.draw_text(text_rect, &display_text, &text_style);

            if tab.closable {
                let close_rect = self.get_close_button_rect(&tab_rect);
                let close_style = Style::new()
                    .with_background(Color::new(0.4, 0.4, 0.4, 1.0));
                canvas.draw_rect(close_rect, &close_style);

                let x_size = 8.0 * self.content_scale;
                let center_x = close_rect.x + close_rect.width / 2.0;
                let center_y = close_rect.y + close_rect.height / 2.0;
                let x_color = Color::new(0.8, 0.8, 0.8, 1.0);
                canvas.draw_line(
                    Point::new(center_x - x_size / 2.0, center_y - x_size / 2.0),
                    Point::new(center_x + x_size / 2.0, center_y + x_size / 2.0),
                    x_color, 1.5 * self.content_scale,
                );
                canvas.draw_line(
                    Point::new(center_x + x_size / 2.0, center_y - x_size / 2.0),
                    Point::new(center_x - x_size / 2.0, center_y + x_size / 2.0),
                    x_color, 1.5 * self.content_scale,
                );
            }
        }

        let content_style = Style::new()
            .with_background(Color::new(0.12, 0.12, 0.12, 1.0));
        let content_rect = Rect::new(0.0, scaled_tab_bar_height, width, height - scaled_tab_bar_height);
        canvas.draw_rect(content_rect, &content_style);
    }

    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if let Some(index) = self.hit_test_tab(local_x, local_y) {
                        if self.hit_test_close_button(index, local_x, local_y) {
                            self.pressed_tab_index = Some(index);
                            dfx_debug!("TabWidget", "TouchBegin on close button: index={}", index);
                        } else {
                            self.pressed_tab_index = Some(index);
                            self.set_state(WidgetState::Pressed);
                            dfx_debug!("TabWidget", "TouchBegin on tab: index={}", index);
                        }
                        return EventResult::Handled;
                    }
                }
            }

            EventType::TouchEnd => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if let Some(pressed_index) = self.pressed_tab_index {
                        if let Some(index) = self.hit_test_tab(local_x, local_y) {
                            if index == pressed_index {
                                if self.hit_test_close_button(index, local_x, local_y) {
                                    self.trigger_close_callback(index);
                                    dfx_debug!("TabWidget", "TouchEnd close button clicked: index={}", index);
                                } else if index != self.active_index {
                                    self.set_active(index);
                                    self.trigger_select_callback(index);
                                    dfx_debug!("TabWidget", "TouchEnd tab selected: index={}", index);
                                }
                            }
                        }
                        self.pressed_tab_index = None;
                        self.set_state(WidgetState::Normal);
                        return EventResult::Stopped;
                    }
                }
            }

            EventType::MouseEnter => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if let Some(index) = self.hit_test_tab(local_x, local_y) {
                        self.hovered_tab_index = Some(index);
                        self.set_state(WidgetState::Hovered);
                        return EventResult::Handled;
                    }
                }
            }

            EventType::MouseLeave => {
                self.hovered_tab_index = None;
                self.set_state(WidgetState::Normal);
                return EventResult::Handled;
            }

            EventType::TouchMove => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if local_y < self.tab_bar_height * self.content_scale {
                        if let Some(index) = self.hit_test_tab(local_x, local_y) {
                            self.hovered_tab_index = Some(index);
                        } else {
                            self.hovered_tab_index = None;
                        }
                    } else {
                        self.hovered_tab_index = None;
                    }
                }
            }

            _ => {}
        }

        EventResult::Ignored
    }
}

impl Default for TabWidget {
    fn default() -> Self {
        Self::new()
    }
}