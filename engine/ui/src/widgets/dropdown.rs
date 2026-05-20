use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
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
    on_select: Option<Box<dyn FnMut(usize) + Send + Sync>>,
    content_scale: f32,
}

impl Dropdown {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 200.0, 30.0),
            style: Style::new()
                .with_background(Color::new(0.2, 0.2, 0.2, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 1.0, 4.0),
            state: WidgetState::Normal,
            options: Vec::new(),
            selected_index: 0,
            is_open: false,
            on_select: None,
            content_scale: 1.0,
        }
    }
    
    pub fn set_options(&mut self, options: Vec<String>) {
        self.options = options;
        if self.selected_index >= self.options.len() && !self.options.is_empty() {
            self.selected_index = 0;
        }
    }
    
    pub fn set_selected(&mut self, index: usize) {
        if index < self.options.len() {
            self.selected_index = index;
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
    
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }
    
    pub fn close(&mut self) {
        self.is_open = false;
    }
    
    pub fn select_option(&mut self, index: usize) {
        if index < self.options.len() {
            self.selected_index = index;
            self.is_open = false;
            if let Some(callback) = &mut self.on_select {
                callback(index);
            }
        }
    }
    
    pub fn is_open(&self) -> bool {
        self.is_open
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
    
    fn draw(&mut self, canvas: &mut Canvas) {
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
        
        let text_rect = Rect::new(10.0, 0.0, self.layout.width - 20.0, self.layout.height);
        let display_text = format!("{} v", selected_text);
        canvas.draw_text(text_rect, &display_text, &text_style);
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                if self.state != WidgetState::Disabled {
                    self.set_state(WidgetState::Pressed);
                    return EventResult::Handled;
                }
            }
            
            EventType::TouchEnd => {
                if self.state == WidgetState::Pressed {
                    self.set_state(WidgetState::Normal);
                    self.toggle();
                    return EventResult::Stopped;
                }
            }
            
            EventType::MouseEnter => {
                if self.state != WidgetState::Disabled {
                    self.set_state(WidgetState::Hovered);
                    return EventResult::Handled;
                }
            }
            
            EventType::MouseLeave => {
                if self.state != WidgetState::Disabled && self.state != WidgetState::Pressed {
                    self.set_state(WidgetState::Normal);
                    return EventResult::Handled;
                }
            }
            
            _ => {}
        }
        EventResult::Ignored
    }
}