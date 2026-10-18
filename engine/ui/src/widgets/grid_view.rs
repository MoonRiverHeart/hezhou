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
    all_text: String,
}

impl GridView {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::zero(),
            style: Style::new()
                .with_background(crate::theme::colors().panel),
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
            all_text: String::new(),
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
        self.rebuild_all_text();
        self.calculate_layout();
        self.flags.dirty_render = true;
        index
    }
    
    fn rebuild_all_text(&mut self) {
        self.all_text.clear();
        for item in &self.items {
            self.all_text.push_str(&item.label);
        }
    }
    
    /// 设置网格项缩略图。`icon:<id>` 使用 SVG 矢量图标作为缩略图。
    pub fn set_item_thumbnail(&mut self, index: usize, thumbnail: Option<String>) {
        if let Some(item) = self.items.get_mut(index) {
            item.thumbnail = thumbnail;
            self.flags.dirty_render = true;
        }
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
            self.rebuild_all_text();
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
        self.all_text.clear();
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
    
    fn get_text(&self) -> Option<&str> {
        Some(&self.all_text)
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
        
        // Clip content to viewport bounds
        canvas.set_clip_rect(Rect::new(0.0, 0.0, width, height));
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        let font_size = 12.0 * self.content_scale;
        let text_style = TextStyle::new()
            .with_size(font_size)
            .with_color(crate::theme::colors().text_primary)
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
                crate::theme::colors().selection_bg
            } else if is_hovered {
                crate::theme::colors().hover
            } else {
                crate::theme::colors().canvas
            };
            
            let cell_style = Style::new().with_background(bg_color);
            canvas.draw_rect(Rect::new(x, y, self.cell_size, self.cell_size), &cell_style);
            
            let thumb_height = self.cell_size - 20.0 * self.content_scale;
            let thumb_color = crate::theme::colors().scrollbar_thumb;
            let thumb_style = Style::new().with_background(thumb_color);
            canvas.draw_rect(Rect::new(x + 4.0 * self.content_scale, y + 4.0 * self.content_scale, 
                                        self.cell_size - 8.0 * self.content_scale, thumb_height - 8.0 * self.content_scale), &thumb_style);

            let pixel_thumb = item.thumbnail.as_ref().and_then(|t| t.strip_prefix("pixels:"));
            if let Some(hex) = pixel_thumb {
                let tx = x + 4.0 * self.content_scale;
                let ty = y + 4.0 * self.content_scale;
                let tw = self.cell_size - 8.0 * self.content_scale;
                let th = thumb_height - 8.0 * self.content_scale;
                let cw = tw / 8.0;
                let ch = th / 8.0;
                if hex.len() >= 8 * 8 * 6 {
                    for py in 0..8usize {
                        for px in 0..8usize {
                            let off = (py * 8 + px) * 6;
                            let r = u8::from_str_radix(&hex[off..off + 2], 16).unwrap_or(0) as f32 / 255.0;
                            let g = u8::from_str_radix(&hex[off + 2..off + 4], 16).unwrap_or(0) as f32 / 255.0;
                            let b = u8::from_str_radix(&hex[off + 4..off + 6], 16).unwrap_or(0) as f32 / 255.0;
                            let st = Style::new().with_background(Color::new(r, g, b, 1.0));
                            canvas.draw_rect(Rect::new(tx + px as f32 * cw, ty + py as f32 * ch, cw + 0.5, ch + 0.5), &st);
                        }
                    }
                }
            } else {
            let texture_kind = item.thumbnail.as_ref().and_then(|t| t.strip_prefix("tex:"));
            if let Some(kind) = texture_kind {
                let tx = x + 4.0 * self.content_scale;
                let ty = y + 4.0 * self.content_scale;
                let tw = self.cell_size - 8.0 * self.content_scale;
                let th = thumb_height - 8.0 * self.content_scale;
                if kind == "checker" {
                    let cells = 5usize;
                    let cw = tw / cells as f32;
                    let ch = th / cells as f32;
                    for cy in 0..cells {
                        for cx in 0..cells {
                            let light = (cx + cy) % 2 == 0;
                            let c = if light { Color::new(0.85, 0.85, 0.85, 1.0) } else { Color::new(0.25, 0.25, 0.25, 1.0) };
                            let st = Style::new().with_background(c);
                            canvas.draw_rect(Rect::new(tx + cx as f32 * cw, ty + cy as f32 * ch, cw, ch), &st);
                        }
                    }
                } else if kind == "uv" {
                    let h = th / 3.0;
                    let st_r = Style::new().with_background(Color::new(0.9, 0.2, 0.2, 1.0));
                    let st_g = Style::new().with_background(Color::new(0.2, 0.9, 0.3, 1.0));
                    let st_b = Style::new().with_background(Color::new(0.2, 0.4, 0.95, 1.0));
                    canvas.draw_rect(Rect::new(tx, ty, tw, h), &st_r);
                    canvas.draw_rect(Rect::new(tx, ty + h, tw, h), &st_g);
                    canvas.draw_rect(Rect::new(tx, ty + h * 2.0, tw, h), &st_b);
                } else {
                    let st = Style::new().with_background(Color::new(0.95, 0.95, 0.95, 1.0));
                    canvas.draw_rect(Rect::new(tx, ty, tw, th), &st);
                }
            } else {
            let fallback_icon = match item.label.as_str() {
                "Cube" => Some(5u32),
                "Sphere" => Some(21u32),
                "Plane" => Some(14u32),
                "Cylinder" => Some(6u32),
                "CornellBox" => Some(9u32),
                _ => None,
            };
            let icon_id = item.thumbnail
                .as_ref()
                .and_then(|t| t.strip_prefix("icon:"))
                .and_then(|v| v.parse::<u32>().ok())
                .or(fallback_icon);
            if let Some(icon_id) = icon_id {
                        if let Some(def) = crate::icons::icon(icon_id) {
                            let pad = 10.0 * self.content_scale;
                            let area_x = x + pad;
                            let area_y = y + pad;
                            let area_w = self.cell_size - pad * 2.0;
                            let area_h = thumb_height - pad * 2.0;
                            let color = crate::theme::colors().accent;
                            for &(x1, y1, x2, y2) in def.segments {
                                canvas.draw_line(
                                    Point::new(area_x + x1 * area_w, area_y + y1 * area_h),
                                    Point::new(area_x + x2 * area_w, area_y + y2 * area_h),
                                    color,
                                    2.0 * self.content_scale,
                                );
                        }
                    }
                }
            }
            }
            
            canvas.draw_text(
                Rect::new(x, y, self.cell_size, self.cell_size),
                &item.label,
                &text_style
            );
            
            if is_selected {
                let border_color = crate::theme::colors().accent;
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
            
            let scrollbar_style = Style::new().with_background(crate::theme::colors().scrollbar_thumb);
            canvas.draw_rect(Rect::new(width - scrollbar_width, scrollbar_y, scrollbar_width, scrollbar_height), &scrollbar_style);
        }
        
        canvas.clear_clip();
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