use crate::canvas::*;
use crate::event::*;
use crate::font_atlas::FontAtlas;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use hezhou_dfx::*;

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub text: String,
    pub shortcut: Option<String>,
    pub action_id: usize,
    pub is_separator: bool,
    pub is_enabled: bool,
    pub has_submenu: bool,
}

impl MenuItem {
    pub fn new(text: String, shortcut: Option<String>, action_id: usize) -> Self {
        Self {
            text,
            shortcut,
            action_id,
            is_separator: false,
            is_enabled: true,
            has_submenu: false,
        }
    }
    
    pub fn separator() -> Self {
        Self {
            text: String::new(),
            shortcut: None,
            action_id: 0,
            is_separator: true,
            is_enabled: true,
            has_submenu: false,
        }
    }
    
    pub fn with_submenu(mut self) -> Self {
        self.has_submenu = true;
        self
    }
    
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }
}

pub struct PopupMenu {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: WidgetFlags,
    items: Vec<MenuItem>,
    item_height: f32,
    padding: f32,
    min_width: f32,
    on_item_click: Option<Box<dyn FnMut(usize) + Send + Sync>>,
    is_visible: bool,
    position_x: f32,
    position_y: f32,
    hovered_index: Option<usize>,
    content_scale: f32,
    all_text: String,
}

impl PopupMenu {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::zero(),
            style: Style::new()
                .with_background(Color::new(0.2, 0.2, 0.2, 1.0))
                .with_border(Color::new(0.4, 0.4, 0.4, 1.0), 1.0, 0.0),
            state: WidgetState::Normal,
            flags: WidgetFlags::default(),
            items: Vec::new(),
            item_height: 28.0,
            padding: 8.0,
            min_width: 200.0,
            on_item_click: None,
            is_visible: false,
            position_x: 0.0,
            position_y: 0.0,
            hovered_index: None,
            content_scale: 1.0,
            all_text: String::new(),
        }
    }
    
    fn rebuild_all_text(&mut self) {
        self.all_text.clear();
        for item in &self.items {
            if !item.is_separator {
                self.all_text.push_str(&item.text);
                if let Some(shortcut) = &item.shortcut {
                    self.all_text.push_str(shortcut);
                }
                if item.has_submenu {
                    self.all_text.push_str(">");
                }
            }
        }
    }
    
    pub fn add_item(&mut self, text: String, shortcut: Option<String>, action_id: usize) {
        self.items.push(MenuItem::new(text, shortcut, action_id));
        self.rebuild_all_text();
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
    }
    
    pub fn add_separator(&mut self) {
        self.items.push(MenuItem::separator());
        self.rebuild_all_text();
        self.flags.dirty_render = true;
    }
    
    pub fn clear(&mut self) {
        self.items.clear();
        self.rebuild_all_text();
        self.flags.dirty_render = true;
    }
    
    pub fn show(&mut self, x: f32, y: f32) {
        self.position_x = x;
        self.position_y = y;
        self.is_visible = true;
        self.layout.x = x;
        self.layout.y = y;
        self.calculate_size();
        self.flags.dirty_layout = true;
        self.flags.dirty_render = true;
        dfx_info!("PopupMenu", "Show at ({}, {}), size: {}x{}", x, y, self.layout.width, self.layout.height);
    }
    
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.hovered_index = None;
        self.flags.dirty_render = true;
        dfx_info!("PopupMenu", "Hide");
    }
    
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
    
    pub fn set_on_click(&mut self, callback: Box<dyn FnMut(usize) + Send + Sync>) {
        self.on_item_click = Some(callback);
    }
    
    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.item_height = 28.0 * scale;
        self.padding = 8.0 * scale;
        self.min_width = 200.0 * scale;
        self.calculate_size();
        self.flags.dirty_render = true;
    }
    
    fn calculate_size(&mut self) {
        let mut max_text_width: f32 = 0.0;
        let mut max_shortcut_width: f32 = 0.0;
        let font_size = 14.0 * self.content_scale;
        let shortcut_font_size = 12.0 * self.content_scale;
        
        let font_atlas_guard = crate::font_atlas::get_font_atlas().lock();
        
        for item in &self.items {
            if !item.is_separator {
                let (tw, _) = font_atlas_guard.measure_text(0, &item.text, font_size);
                max_text_width = max_text_width.max(tw);
                if let Some(shortcut) = &item.shortcut {
                    let (sw, _) = font_atlas_guard.measure_text(0, shortcut, shortcut_font_size);
                    max_shortcut_width = max_shortcut_width.max(sw);
                }
            }
        }
        
        let width = self.min_width.max(self.padding * 2.0 + max_text_width + 20.0 * self.content_scale + max_shortcut_width);
        
        let mut height = self.padding * 2.0;
        for item in &self.items {
            if item.is_separator {
                height += 9.0 * self.content_scale;
            } else {
                height += self.item_height;
            }
        }
        
        self.layout.width = width;
        self.layout.height = height;
    }
    
    fn get_item_at_position(&self, local_y: f32) -> Option<usize> {
        if !self.is_visible {
            return None;
        }
        
        let mut current_y = self.padding;
        for (i, item) in self.items.iter().enumerate() {
            let item_height = if item.is_separator {
                9.0 * self.content_scale
            } else {
                self.item_height
            };
            
            if local_y >= current_y && local_y < current_y + item_height && !item.is_separator {
                return Some(i);
            }
            current_y += item_height;
        }
        None
    }
}

impl Widget for PopupMenu {
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
        self.children.push(child);
    }
    
    fn remove_child(&mut self, child: WidgetId) {
        self.children.retain(|&id| id != child);
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
    
    fn widget_type(&self) -> &'static str {
        "PopupMenu"
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
    
    fn measure(&self, _font_atlas: &FontAtlas) -> (f32, f32) {
        (self.layout.width, self.layout.height)
    }
    
    fn draw(&mut self, canvas: &mut Canvas) {
        if !self.is_visible {
            return;
        }
        
        let width = self.layout.width;
        let height = self.layout.height;
        let font_size = 14.0 * self.content_scale;
        let shortcut_font_size = 12.0 * self.content_scale;
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        let mut current_y = self.padding;
        
        for (i, item) in self.items.iter().enumerate() {
            if item.is_separator {
                let sep_y = current_y + 4.0 * self.content_scale;
                canvas.draw_line(
                    Point::new(self.padding, sep_y),
                    Point::new(width - self.padding, sep_y),
                    Color::new(0.4, 0.4, 0.4, 1.0),
                    1.0
                );
                current_y += 9.0 * self.content_scale;
            } else {
                let is_hovered = self.hovered_index == Some(i);
                if is_hovered && item.is_enabled {
                    let hover_style = Style::new()
                        .with_background(Color::new(0.3, 0.3, 0.3, 1.0));
                    canvas.draw_rect(Rect::new(self.padding, current_y, width - self.padding * 2.0, self.item_height), &hover_style);
                }
                
                let text_color = if item.is_enabled {
                    Color::white()
                } else {
                    Color::new(0.5, 0.5, 0.5, 1.0)
                };
                
                let text_style = TextStyle::new()
                    .with_size(font_size)
                    .with_color(text_color)
                    .with_alignment(TextAlignment {
                        horizontal: HorizontalAlignment::Left,
                        vertical: VerticalAlignment::Center,
                    });
                
                canvas.draw_text(
                    Rect::new(self.padding, current_y, width - self.padding * 2.0, self.item_height),
                    &item.text,
                    &text_style
                );
                
                if let Some(shortcut) = &item.shortcut {
                    let shortcut_style = TextStyle::new()
                        .with_size(shortcut_font_size)
                        .with_color(Color::new(0.6, 0.6, 0.6, 1.0))
                        .with_alignment(TextAlignment {
                            horizontal: HorizontalAlignment::Right,
                            vertical: VerticalAlignment::Center,
                        });
                    
                    canvas.draw_text(
                        Rect::new(self.padding, current_y, width - self.padding * 2.0 - 10.0 * self.content_scale, self.item_height),
                        shortcut,
                        &shortcut_style
                    );
                }
                
                if item.has_submenu {
                    let arrow_x = width - self.padding - 10.0 * self.content_scale;
                    let arrow_y = current_y + self.item_height / 2.0;
                    canvas.draw_text(
                        Rect::new(arrow_x, current_y, 10.0 * self.content_scale, self.item_height),
                        ">",
                        &text_style
                    );
                }
                
                current_y += self.item_height;
            }
        }
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        if !self.is_visible {
            return EventResult::Ignored;
        }
        
        match event.event_type {
            EventType::TouchBegin => {
                if let EventData::Touch(touch) = &event.data {
                    // touch coordinates are already relative to this widget
                    // (EventDispatcher converts window coords to local coords in both capturing+bubbling)
                    if touch.x >= 0.0 && touch.x <= self.layout.width &&
                       touch.y >= 0.0 && touch.y <= self.layout.height {
                        if let Some(index) = self.get_item_at_position(touch.y) {
                            if self.items[index].is_enabled {
                                self.hovered_index = Some(index);
                                return EventResult::Handled;
                            }
                        }
                    } else {
                        self.hide();
                        crate::thunk::queue_callback(crate::thunk::PendingCallback::PopupMenuClose { widget_id: self.id.id });
                        return EventResult::Stopped;
                    }
                }
            }
            
            EventType::TouchEnd => {
                if let Some(index) = self.hovered_index {
                    if self.items[index].is_enabled {
                        let action_id = self.items[index].action_id;
                        dfx_info!("PopupMenu", "Item clicked: index={}, action_id={}", index, action_id);
                        
                        crate::thunk::queue_callback(crate::thunk::PendingCallback::PopupMenuClick {
                            widget_id: self.id.id,
                            action_id,
                        });
                        
                        if let Some(callback) = &mut self.on_item_click {
                            callback(action_id);
                        }
                        
                        self.hide();
                        return EventResult::Stopped;
                    }
                }
            }
            
            EventType::MouseEnter => {
                self.state = WidgetState::Hovered;
                return EventResult::Handled;
            }
            
            EventType::MouseLeave => {
                self.hovered_index = None;
                self.state = WidgetState::Normal;
                self.flags.dirty_render = true;
                return EventResult::Handled;
            }
            
            EventType::MouseMove => {
                if let EventData::Touch(touch) = &event.data {
                    // touch coordinates are already relative to this widget
                    let new_hovered = self.get_item_at_position(touch.y);
                    if new_hovered != self.hovered_index {
                        self.hovered_index = new_hovered;
                        self.flags.dirty_render = true;
                        return EventResult::Handled;
                    }
                }
            }
            
            _ => {}
        }
        
        EventResult::Ignored
    }
    
    fn hit_test(&self, point: Point) -> bool {
        if !self.is_visible {
            return false;
        }
        point.x >= self.layout.x && point.x <= self.layout.x + self.layout.width &&
        point.y >= self.layout.y && point.y <= self.layout.y + self.layout.height
    }
    
    fn get_text(&self) -> Option<&str> {
        Some(&self.all_text)
    }
}