use crate::canvas::*;
use crate::event::*;
use crate::font_atlas::FontAtlas;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use hezhou_dfx::*;

#[derive(Clone, Debug)]
pub struct GridItem {
    pub id: WidgetId,
    pub thumbnail: Option<String>,
    pub label: String,
    pub user_data: u64,
    pub is_selected: bool,
}

impl GridItem {
    pub fn new(label: String, user_data: u64) -> Self {
        Self {
            id: WidgetId::new(),
            thumbnail: None,
            label,
            user_data,
            is_selected: false,
        }
    }
    
    pub fn with_thumbnail(mut self, thumbnail: Option<String>) -> Self {
        self.thumbnail = thumbnail;
        self
    }
}

pub struct GridView {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: WidgetFlags,
    items: Vec<GridItem>,
    cell_size: f32,
    spacing: f32,
    padding: f32,
    columns: usize,
    selected_index: Option<usize>,
    scroll_offset: f32,
    on_item_click: Option<Box<dyn FnMut(usize, u64) + Send + Sync>>,
    content_scale: f32,
    hovered_index: Option<usize>,
    max_scroll_offset: f32,
}

impl GridView {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::zero(),
            style: Style::new()
                .with_background(Color::new(0.15, 0.15, 0.15, 1.0)),
            state: WidgetState::Normal,
            flags: WidgetFlags::default(),
            items: Vec::new(),
            cell_size: 64.0,
            spacing: 8.0,
            padding: 8.0,
            columns: 1,
            selected_index: None,
            scroll_offset: 0.0,
            on_item_click: None,
            content_scale: 1.0,
            hovered_index: None,
            max_scroll_offset: 0.0,
        }
    }
    
    pub fn with_cell_size(mut self, size: f32) -> Self {
        self.cell_size = size;
        self
    }
    
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
    
    pub fn add_item(&mut self, label: String, user_data: u64) -> usize {
        let item = GridItem::new(label, user_data);
        let index = self.items.len();
        self.items.push(item);
        self.calculate_layout();
        self.flags.dirty_render = true;
        index
    }
    
    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            if self.selected_index == Some(index) {
                self.selected_index = None;
            } else if let Some(sel) = self.selected_index {
                if sel > index {
                    self.selected_index = Some(sel - 1);
                }
            }
            self.calculate_layout();
            self.flags.dirty_render = true;
        }
    }
    
    pub fn set_selected(&mut self, index: usize) {
        if index < self.items.len() {
            for (i, item) in self.items.iter_mut().enumerate() {
                item.is_selected = i == index;
            }
            self.selected_index = Some(index);
            self.flags.dirty_render = true;
        }
    }
    
    pub fn clear(&mut self) {
        self.items.clear();
        self.selected_index = None;
        self.scroll_offset = 0.0;
        self.columns = 0;
        self.flags.dirty_render = true;
    }
    
    pub fn set_on_click(&mut self, callback: Box<dyn FnMut(usize, u64) + Send + Sync>) {
        self.on_item_click = Some(callback);
    }
    
    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
        self.cell_size = 64.0 * scale;
        self.spacing = 8.0 * scale;
        self.padding = 8.0 * scale;
        self.calculate_layout();
        self.flags.dirty_render = true;
    }
    
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }
    
    pub fn selected_user_data(&self) -> Option<u64> {
        self.selected_index.and_then(|i| self.items.get(i).map(|item| item.user_data))
    }
    
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
    
    pub fn get_item_user_data(&self, index: usize) -> Option<u64> {
        self.items.get(index).map(|item| item.user_data)
    }
    
    fn calculate_layout(&mut self) {
        let available_width = self.layout.width - 2.0 * self.padding;
        if available_width <= 0.0 {
            self.columns = 0;
            return;
        }
        
        self.columns = ((available_width + self.spacing) / (self.cell_size + self.spacing)).floor() as usize;
        if self.columns < 1 {
            self.columns = 1;
        }
        
        let rows = (self.items.len() + self.columns - 1) / self.columns;
        self.max_scroll_offset = if rows > 0 {
            let total_height = self.padding + rows as f32 * (self.cell_size + self.spacing) - self.spacing + self.padding;
            (total_height - self.layout.height).max(0.0)
        } else {
            0.0
        };
        
        if self.scroll_offset > self.max_scroll_offset {
            self.scroll_offset = self.max_scroll_offset;
        }
    }
    
    fn get_item_position(&self, index: usize) -> (f32, f32) {
        if self.columns == 0 {
            return (0.0, 0.0);
        }
        
        let col = index % self.columns;
        let row = index / self.columns;
        
        let x = self.padding + col as f32 * (self.cell_size + self.spacing);
        let y = self.padding + row as f32 * (self.cell_size + self.spacing) - self.scroll_offset;
        
        (x, y)
    }
    
    fn get_item_at_position(&self, local_x: f32, local_y: f32) -> Option<usize> {
        if self.columns == 0 {
            return None;
        }
        
        let adjusted_y = local_y + self.scroll_offset;
        
        for (i, _item) in self.items.iter().enumerate() {
            let (x, y) = self.get_item_position(i);
            
            if local_x >= x && local_x <= x + self.cell_size &&
               adjusted_y >= y && adjusted_y <= y + self.cell_size {
                return Some(i);
            }
        }
        None
    }
}

impl Widget for GridView {
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
        self.calculate_layout();
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
        "GridView"
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
        let width = self.layout.width;
        let height = self.layout.height;
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        let font_size = 12.0 * self.content_scale;
        let text_style = TextStyle::new()
            .with_size(font_size)
            .with_color(Color::white())
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Center,
                vertical: VerticalAlignment::Bottom,
            });
        
        for (i, item) in self.items.iter().enumerate() {
            let (x, y) = self.get_item_position(i);
            
            if y > height || y + self.cell_size < 0.0 {
                continue;
            }
            
            let is_hovered = self.hovered_index == Some(i);
            let is_selected = item.is_selected;
            
            let bg_color = if is_selected {
                Color::new(0.2, 0.4, 0.6, 0.8)
            } else if is_hovered {
                Color::new(0.25, 0.25, 0.25, 0.8)
            } else {
                Color::new(0.2, 0.2, 0.2, 0.5)
            };
            
            let cell_style = Style::new().with_background(bg_color);
            canvas.draw_rect(Rect::new(x, y, self.cell_size, self.cell_size), &cell_style);
            
            let thumb_height = self.cell_size - 20.0 * self.content_scale;
            let thumb_color = Color::new(0.3, 0.3, 0.3, 0.6);
            let thumb_style = Style::new().with_background(thumb_color);
            canvas.draw_rect(Rect::new(x + 4.0 * self.content_scale, y + 4.0 * self.content_scale, 
                                        self.cell_size - 8.0 * self.content_scale, thumb_height - 8.0 * self.content_scale), &thumb_style);
            
            canvas.draw_text(
                Rect::new(x, y, self.cell_size, self.cell_size),
                &item.label,
                &text_style
            );
            
            if is_selected {
                let border_color = Color::new(0.3, 0.6, 1.0, 1.0);
                canvas.draw_rect_outline(
                    Rect::new(x, y, self.cell_size, self.cell_size),
                    border_color,
                    2.0 * self.content_scale
                );
            }
        }
        
        if self.max_scroll_offset > 0.0 {
            let scroll_ratio = self.scroll_offset / self.max_scroll_offset;
            let scrollbar_height = 20.0 * self.content_scale;
            let scrollbar_width = 6.0 * self.content_scale;
            let scrollbar_y = scroll_ratio * (height - scrollbar_height);
            
            let scrollbar_style = Style::new().with_background(Color::new(0.5, 0.5, 0.5, 0.5));
            canvas.draw_rect(Rect::new(width - scrollbar_width, scrollbar_y, scrollbar_width, scrollbar_height), &scrollbar_style);
        }
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
EventType::TouchBegin => {
                if let EventData::Touch(touch) = &event.data {
                    // touch coordinates are already relative to this widget
                    // (EventDispatcher converts window coords to local coords in both capturing+bubbling)
                    if touch.x >= 0.0 && touch.x <= self.layout.width &&
                       touch.y >= 0.0 && touch.y <= self.layout.height {
                        if let Some(index) = self.get_item_at_position(touch.x, touch.y) {
                            self.hovered_index = Some(index);
                            return EventResult::Handled;
                        }
                    }
                }
            }
            
            EventType::TouchEnd => {
                if let Some(index) = self.hovered_index {
                    if index < self.items.len() {
                        let user_data = self.items[index].user_data;
                        dfx_info!("GridView", "Item clicked: index={}, user_data={}", index, user_data);
                        
                        self.set_selected(index);
                        
                        crate::thunk::queue_callback(crate::thunk::PendingCallback::GridViewClick {
                            widget_id: self.id.id,
                            index,
                            user_data,
                        });
                        
                        if let Some(callback) = &mut self.on_item_click {
                            callback(index, user_data);
                        }
                        
                        self.hovered_index = None;
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
                    let new_hovered = self.get_item_at_position(touch.x, touch.y);
                    if new_hovered != self.hovered_index {
                        self.hovered_index = new_hovered;
                        self.flags.dirty_render = true;
                        return EventResult::Handled;
                    }
                }
            }
            
            EventType::MouseWheel => {
                if let EventData::Wheel(wheel) = &event.data {
                    let delta = wheel.delta_y * 20.0;
                    self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, self.max_scroll_offset);
                    self.flags.dirty_render = true;
                    return EventResult::Handled;
                }
            }
            
            _ => {}
        }
        
        EventResult::Ignored
    }
}

impl Default for GridView {
    fn default() -> Self {
        Self::new()
    }
}