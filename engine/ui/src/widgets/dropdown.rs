use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use hezhou_dfx::*;
use std::sync::Mutex;

pub struct Dropdown {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    options: Vec<String>,
    selected_index: usize,
    is_open: bool,
    hovered_index: Option<usize>,
    on_select: Option<Box<dyn FnMut(usize) + Send + Sync>>,
    content_scale: f32,
    flags: WidgetFlags,
    all_text: String,
}

impl Dropdown {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 200.0, 30.0),
            style: Style::new()
                .with_background(Color::new(0.25, 0.25, 0.28, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.45, 1.0), 1.0, 4.0),
            state: WidgetState::Normal,
            options: Vec::new(),
            selected_index: 0,
            is_open: false,
            hovered_index: None,
            on_select: None,
            content_scale: 1.0,
            flags: WidgetFlags::default(),
            all_text: String::new(),
        }
    }
    
    pub fn set_options(&mut self, options: Vec<String>) {
        self.options = options;
        if self.selected_index >= self.options.len() && !self.options.is_empty() {
            self.selected_index = 0;
        }
        self.rebuild_all_text();
    }
    
    pub fn set_selected(&mut self, index: usize) {
        if index < self.options.len() {
            self.selected_index = index;
            self.rebuild_all_text();
        }
    }
    
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }
    
    pub fn selected_value(&self) -> Option<&str> {
        self.options.get(self.selected_index).map(|s| s.as_str())
    }
    
    pub fn set_on_select(&mut self, callback: Box<dyn FnMut(usize) + Send + Sync>) {
        self.on_select = Some(callback);
    }
    
    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
    }
    
    fn rebuild_all_text(&mut self) {
        self.all_text.clear();
        for option in &self.options {
            self.all_text.push_str(option);
        }
    }
    
    pub fn toggle(&mut self) {
        if self.options.is_empty() {
            dfx_debug!("Dropdown", "toggle: options empty, skip");
            return;
        }
        self.is_open = !self.is_open;
        dfx_debug!("Dropdown", "toggle: is_open={}, options_count={}, selected={}", self.is_open, self.options.len(), self.selected_index);
        if !self.is_open {
            self.hovered_index = None;
        }
        self.flags.dirty_render = true;
    }
    
    pub fn close(&mut self) {
        dfx_debug!("Dropdown", "close: was_open={}", self.is_open);
        self.is_open = false;
        self.hovered_index = None;
        self.flags.dirty_render = true;
    }
    
    pub fn select_option(&mut self, index: usize) {
        if index < self.options.len() {
            self.selected_index = index;
            self.is_open = false;
            self.hovered_index = None;
            self.rebuild_all_text();
            self.flags.dirty_render = true;
            crate::thunk::queue_callback(crate::thunk::PendingCallback::DropdownSelect {
                widget_id: self.id.id,
                index,
            });
            if let Some(callback) = &mut self.on_select {
                callback(index);
            }
        }
    }
    
    pub fn is_open(&self) -> bool {
        self.is_open
    }
    
    pub fn options_count(&self) -> usize {
        self.options.len()
    }
}

impl Widget for Dropdown {
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
    }
    
    fn style(&self) -> &Style {
        &self.style
    }
    
    fn set_style(&mut self, style: Style) {
        self.style = style;
    }
    
    fn state(&self) -> WidgetState {
        self.state
    }
    
    fn set_state(&mut self, state: WidgetState) {
        self.state = state;
    }
    
    fn measure(&self, _font_atlas: &crate::font_atlas::FontAtlas) -> (f32, f32) {
        (self.layout.width, self.layout.height)
    }
    
    fn widget_type(&self) -> &'static str {
        "Dropdown"
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn flags(&self) -> WidgetFlags {
        self.flags
    }

    fn set_flags(&mut self, flags: WidgetFlags) {
        self.flags = flags;
    }
    
    fn get_text(&self) -> Option<&str> {
        Some(&self.all_text)
    }
    
    fn draw(&mut self, canvas: &mut Canvas) {
        dfx_debug!("Dropdown", "draw: id={}, layout=({:.0},{:.0},{:.0},{:.0}) is_open={} options={} selected={}", 
            self.id.id, self.layout.x, self.layout.y, self.layout.width, self.layout.height,
            self.is_open, self.options.len(), self.selected_index);
        let selected_text: String = if self.options.is_empty() {
            "(empty)".to_string()
        } else {
            self.options.get(self.selected_index).unwrap_or(&String::new()).to_string()
        };
        
        let current_style = match self.state {
            WidgetState::Hovered => Style::new()
                .with_background(Color::new(0.25, 0.25, 0.25, 1.0))
                .with_border(Color::new(0.5, 0.5, 0.5, 1.0), 1.0, 4.0),
            WidgetState::Pressed => Style::new()
                .with_background(Color::new(0.15, 0.15, 0.15, 1.0))
                .with_border(Color::new(0.3, 0.3, 0.3, 1.0), 1.0, 4.0),
            _ => self.style,
        };
        
        let rect = Rect::new(0.0, 0.0, self.layout.width, self.layout.height);
        canvas.draw_rect(rect, &current_style);
        
        let text_style = TextStyle::new()
            .with_size(14.0 * self.content_scale)
            .with_color(Color::white())
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Left,
                vertical: VerticalAlignment::Center,
            });
        
        let text_rect = Rect::new(10.0, 0.0, self.layout.width - 30.0, self.layout.height);
        canvas.draw_text(text_rect, &selected_text, &text_style);
        
        // Draw dropdown arrow (▼) on the right side
        let arrow_size = 10.0 * self.content_scale;
        let arrow_x = self.layout.width - arrow_size - 8.0;
        let arrow_y_center = self.layout.height / 2.0;
        let arrow_half = arrow_size / 2.0;
        
        // ▼ triangle using 3 lines
        let arrow_style = Style::new().with_background(Color::new(0.7, 0.7, 0.7, 1.0));
        canvas.draw_line(
            Point::new(arrow_x, arrow_y_center - arrow_half * 0.4),
            Point::new(arrow_x + arrow_size, arrow_y_center - arrow_half * 0.4),
            Color::new(0.7, 0.7, 0.7, 1.0),
            1.5 * self.content_scale,
        );
        canvas.draw_line(
            Point::new(arrow_x + arrow_size, arrow_y_center - arrow_half * 0.4),
            Point::new(arrow_x + arrow_half, arrow_y_center + arrow_half * 0.6),
            Color::new(0.7, 0.7, 0.7, 1.0),
            1.5 * self.content_scale,
        );
        canvas.draw_line(
            Point::new(arrow_x + arrow_half, arrow_y_center + arrow_half * 0.6),
            Point::new(arrow_x, arrow_y_center - arrow_half * 0.4),
            Color::new(0.7, 0.7, 0.7, 1.0),
            1.5 * self.content_scale,
        );
        
        // === Popup选项列表渲染（is_open=true时） ===
        if self.is_open && !self.options.is_empty() {
            let option_height = self.layout.height;
            let popup_y_start = self.layout.height;
            let popup_width = self.layout.width;
            let font_size = 14.0 * self.content_scale;
            
            for (i, option) in self.options.iter().enumerate() {
                let option_y = popup_y_start + i as f32 * option_height;
                
                // 选项背景：选中项蓝色、hover灰色、普通深灰
                let bg_color = if i == self.selected_index {
                    Color::new(0.2, 0.4, 0.8, 1.0)
                } else if self.hovered_index == Some(i) {
                    Color::new(0.3, 0.3, 0.35, 1.0)
                } else {
                    Color::new(0.25, 0.25, 0.28, 1.0)
                };
                
                let option_style = Style::new().with_background(bg_color);
                canvas.draw_rect(Rect::new(0.0, option_y, popup_width, option_height), &option_style);
                
                // 选项文本
                let text_style = TextStyle::new()
                    .with_size(font_size)
                    .with_color(Color::white())
                    .with_alignment(TextAlignment {
                        horizontal: HorizontalAlignment::Left,
                        vertical: VerticalAlignment::Center,
                    });
                
                canvas.draw_text(
                    Rect::new(10.0, option_y, popup_width - 30.0, option_height),
                    option,
                    &text_style,
                );
            }
            
            // Popup边框
            let popup_total_height = self.options.len() as f32 * option_height;
            canvas.draw_rect_outline(
                Rect::new(0.0, popup_y_start, popup_width, popup_total_height),
                Color::new(0.4, 0.4, 0.45, 1.0),
                1.0 * self.content_scale,
            );
        }
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        if self.state == WidgetState::Disabled {
            return EventResult::Ignored;
        }
        
        match event.event_type {
            EventType::TouchBegin => {
                if let EventData::Touch(touch) = &event.data {
                    dfx_debug!("Dropdown", "TouchBegin: is_open={}, state={:?}, touch=({:.1},{:.1}), layout=({:.0},{:.0})", 
                        self.is_open, self.state, touch.x, touch.y, self.layout.width, self.layout.height);
                    
                    if self.is_open {
                        // Popup打开状态：检查点击位置
                        let option_height = self.layout.height;
                        let popup_y_start = self.layout.height;
                        
                        if touch.x >= 0.0 && touch.x <= self.layout.width {
                            // 检查是否点击了popup选项
                            for i in 0..self.options.len() {
                                let option_y = popup_y_start + i as f32 * option_height;
                                if touch.y >= option_y && touch.y < option_y + option_height {
                                    self.hovered_index = Some(i);
                                    self.set_state(WidgetState::Pressed);
                                    self.flags.dirty_render = true;
                                    dfx_debug!("Dropdown", "TouchBegin: hovered option {}", i);
                                    return EventResult::Handled;
                                }
                            }
                            
                            // 点击在Dropdown按钮区域 → 保持Pressed状态
                            if touch.y >= 0.0 && touch.y < self.layout.height {
                                self.set_state(WidgetState::Pressed);
                                dfx_debug!("Dropdown", "TouchBegin: on button area, set Pressed");
                                return EventResult::Handled;
                            }
                        }
                        
                        // 点击在popup外部 → 关闭popup
                        dfx_debug!("Dropdown", "TouchBegin: outside popup, closing");
                        self.is_open = false;
                        self.hovered_index = None;
                        self.set_state(WidgetState::Normal);
                        self.flags.dirty_render = true;
                        return EventResult::Stopped;
                    } else {
                        // 关闭状态：hit_test已确保事件在widget范围内，无需坐标检查
                        // 参照Button模式：直接设Pressed
                        self.set_state(WidgetState::Pressed);
                        dfx_debug!("Dropdown", "TouchBegin: closed state, set Pressed");
                        return EventResult::Handled;
                    }
                }
                // 无坐标数据 → fallback
                self.set_state(WidgetState::Pressed);
                dfx_debug!("Dropdown", "TouchBegin: no touch data, fallback Pressed");
                return EventResult::Handled;
            }
            
            EventType::TouchEnd => {
                dfx_debug!("Dropdown", "TouchEnd: is_open={}, state={:?}, hovered_index={}", 
                    self.is_open, self.state, self.hovered_index.map(|i| i.to_string()).unwrap_or_else(|| "None".to_string()));
                
                if self.is_open {
                    if let Some(index) = self.hovered_index {
                        // 选中hovered选项
                        dfx_debug!("Dropdown", "TouchEnd: select option {}", index);
                        self.select_option(index);
                        return EventResult::Stopped;
                    }
                    // TouchEnd无hovered_index → 关闭popup
                    dfx_debug!("Dropdown", "TouchEnd: no hovered, closing popup");
                    self.is_open = false;
                    self.hovered_index = None;
                    self.set_state(WidgetState::Normal);
                    self.flags.dirty_render = true;
                    return EventResult::Stopped;
                } else {
                    if self.state == WidgetState::Pressed {
                        self.set_state(WidgetState::Normal);
                        dfx_debug!("Dropdown", "TouchEnd: closed+Pressed → toggle");
                        self.toggle();
                        return EventResult::Stopped;
                    }
                }
                return EventResult::Ignored;
            }
            
            EventType::MouseEnter => {
                dfx_debug!("Dropdown", "MouseEnter: is_open={}", self.is_open);
                self.set_state(WidgetState::Hovered);
                return EventResult::Handled;
            }
            
            EventType::MouseLeave => {
                dfx_debug!("Dropdown", "MouseLeave: is_open={}, state={:?}", self.is_open, self.state);
                self.hovered_index = None;
                if self.state != WidgetState::Pressed {
                    self.set_state(WidgetState::Normal);
                }
                self.flags.dirty_render = true;
                return EventResult::Handled;
            }
            
            EventType::MouseMove => {
                if self.is_open {
                    // MouseMove事件数据是EventData::Mouse，坐标已由dispatch_bubbling转换为局部坐标
                    if let EventData::Mouse(mouse) = &event.data {
                        let option_height = self.layout.height;
                        let popup_y_start = self.layout.height;
                        
                        let mut new_hovered: Option<usize> = None;
                        if mouse.x >= 0.0 && mouse.x <= self.layout.width {
                            for i in 0..self.options.len() {
                                let option_y = popup_y_start + i as f32 * option_height;
                                if mouse.y >= option_y && mouse.y < option_y + option_height {
                                    new_hovered = Some(i);
                                    break;
                                }
                            }
                        }
                        
                        if new_hovered != self.hovered_index {
                            self.hovered_index = new_hovered;
                            self.flags.dirty_render = true;
                            return EventResult::Handled;
                        }
                    }
                }
                return EventResult::Ignored;
            }
            
            _ => {}
        }
        EventResult::Ignored
    }
    
    fn hit_test(&self, point: Point) -> bool {
        let bounds = if self.is_open && !self.options.is_empty() {
            let popup_height = self.options.len() as f32 * self.layout.height;
            Rect::new(
                self.layout.x,
                self.layout.y,
                self.layout.width,
                self.layout.height + popup_height,
            )
        } else {
            Rect::new(
                self.layout.x,
                self.layout.y,
                self.layout.width,
                self.layout.height,
            )
        };
        bounds.contains(&point)
    }
}