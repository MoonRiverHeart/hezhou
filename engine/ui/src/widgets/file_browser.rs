use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use hezhou_dfx::*;
use std::fs;
use std::path::Path;

pub struct FileItem {
    name: String,
    path: String,
    is_directory: bool,
}

pub struct FileBrowser {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    current_path: String,
    items: Vec<FileItem>,
    selected_index: Option<usize>,
    filter: Option<String>,
    show_hidden: bool,
    path_bar_height: f32,
    search_bar_height: f32,
    item_height: f32,
    hovered_item_index: Option<usize>,
    pressed_item_index: Option<usize>,
    scroll_offset: f32,
    content_scale: f32,
}

impl FileBrowser {
    pub fn new() -> Self {
        let initial_path = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| ".".to_string());
        
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 300.0, 400.0),
            style: Style::new()
                .with_background(Color::new(0.15, 0.15, 0.15, 1.0))
                .with_border(Color::new(0.3, 0.3, 0.3, 1.0), 1.0, 4.0),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            current_path: initial_path,
            items: Vec::new(),
            selected_index: None,
            filter: None,
            show_hidden: false,
            path_bar_height: 32.0,
            search_bar_height: 28.0,
            item_height: 24.0,
            hovered_item_index: None,
            pressed_item_index: None,
            scroll_offset: 0.0,
            content_scale: 1.0,
        }
    }
    
    pub fn with_initial_path(mut self, path: &str) -> Self {
        self.current_path = path.to_string();
        self.refresh();
        self
    }
    
    pub fn with_layout(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.layout = Layout::new(x, y, width, height);
        self
    }
    
    pub fn set_path(&mut self, path: &str) {
        self.current_path = path.to_string();
        self.selected_index = None;
        self.scroll_offset = 0.0;
        self.refresh();
        dfx_info!("FileBrowser", "SetPath: id={}, path={}", self.id.id, path);
    }
    
    pub fn refresh(&mut self) {
        self.items.clear();
        
        let path = Path::new(&self.current_path);
        if !path.exists() || !path.is_dir() {
            dfx_warn!("FileBrowser", "Path does not exist or is not a directory: {}", self.current_path);
            return;
        }
        
        if let Ok(entries) = fs::read_dir(path) {
            let mut dirs: Vec<FileItem> = Vec::new();
            let mut files: Vec<FileItem> = Vec::new();
            
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }
                
                let entry_path = entry.path().to_string_lossy().into_owned();
                let is_dir = entry.path().is_dir();
                
                if let Some(filter) = &self.filter {
                    if !is_dir {
                        let matches = if filter.starts_with('*') {
                            let ext = filter.trim_start_matches('*');
                            name.ends_with(ext)
                        } else {
                            name.contains(filter)
                        };
                        if !matches {
                            continue;
                        }
                    }
                }
                
                let item = FileItem {
                    name,
                    path: entry_path,
                    is_directory: is_dir,
                };
                
                if is_dir {
                    dirs.push(item);
                } else {
                    files.push(item);
                }
            }
            
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            
            self.items.extend(dirs);
            self.items.extend(files);
        }
        
        self.flags.dirty_render = true;
        dfx_info!("FileBrowser", "Refresh: id={}, items={}", self.id.id, self.items.len());
    }
    
    pub fn set_filter(&mut self, filter: &str) {
        self.filter = Some(filter.to_string());
        self.refresh();
        dfx_info!("FileBrowser", "SetFilter: id={}, filter={}", self.id.id, filter);
    }
    
    pub fn navigate_up(&mut self) {
        let path = Path::new(&self.current_path);
        if let Some(parent) = path.parent() {
            let parent_path = parent.to_string_lossy().into_owned();
            self.set_path(&parent_path);
            dfx_info!("FileBrowser", "NavigateUp: id={}, new_path={}", self.id.id, parent_path);
        }
    }
    
    pub fn navigate_to(&mut self, path: &str) {
        self.set_path(path);
    }
    
    pub fn get_selected_path(&self) -> Option<String> {
        self.selected_index.and_then(|idx| {
            self.items.get(idx).map(|item| item.path.clone())
        })
    }
    
    pub fn get_selected_item(&self) -> Option<&FileItem> {
        self.selected_index.and_then(|idx| self.items.get(idx))
    }
    
    pub fn current_path(&self) -> &str {
        &self.current_path
    }
    
    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
    }
    
    fn get_content_rect(&self) -> Rect {
        let scaled_path_height = self.path_bar_height * self.content_scale;
        let scaled_search_height = self.search_bar_height * self.content_scale;
        Rect::new(
            0.0,
            scaled_path_height,
            self.layout.width,
            self.layout.height - scaled_path_height - scaled_search_height,
        )
    }
    
    fn get_item_rect(&self, index: usize) -> Rect {
        let scaled_path_height = self.path_bar_height * self.content_scale;
        let scaled_item_height = self.item_height * self.content_scale;
        let y = scaled_path_height + index as f32 * scaled_item_height - self.scroll_offset;
        Rect::new(0.0, y, self.layout.width, scaled_item_height)
    }
    
    fn hit_test_item(&self, local_x: f32, local_y: f32) -> Option<usize> {
        let content_rect = self.get_content_rect();
        if !content_rect.contains(&Point::new(local_x, local_y)) {
            return None;
        }
        
        let scaled_path_height = self.path_bar_height * self.content_scale;
        let scaled_item_height = self.item_height * self.content_scale;
        let adjusted_y = local_y - scaled_path_height + self.scroll_offset;
        let index = (adjusted_y / scaled_item_height) as usize;
        
        if index < self.items.len() {
            Some(index)
        } else {
            None
        }
    }
    
    fn trigger_select_callback(&mut self, path: &str) {
        crate::thunk_manager::queue_callback(crate::thunk_manager::PendingCallback::FileBrowserSelect {
            browser_id: self.id.id,
            path: path.to_string(),
        });
    }
    
    fn trigger_double_click_callback(&mut self, path: &str) {
        crate::thunk_manager::queue_callback(crate::thunk_manager::PendingCallback::FileBrowserDoubleClick {
            browser_id: self.id.id,
            path: path.to_string(),
        });
    }
}

impl Widget for FileBrowser {
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
    
    fn widget_type(&self) -> &'static str {
        "FileBrowser"
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn draw(&mut self, canvas: &mut Canvas) {
        let width = self.layout.width;
        let height = self.layout.height;
        let scaled_path_height = self.path_bar_height * self.content_scale;
        let scaled_search_height = self.search_bar_height * self.content_scale;
        let scaled_item_height = self.item_height * self.content_scale;
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        let path_bar_style = Style::new()
            .with_background(Color::new(0.2, 0.2, 0.2, 1.0))
            .with_border(Color::new(0.3, 0.3, 0.3, 1.0), 1.0, 0.0);
        canvas.draw_rect(Rect::new(0.0, 0.0, width, scaled_path_height), &path_bar_style);
        
        let up_button_rect = Rect::new(0.0, 0.0, scaled_path_height, scaled_path_height);
        let up_button_style = Style::new()
            .with_background(Color::new(0.25, 0.25, 0.25, 1.0));
        canvas.draw_rect(up_button_rect, &up_button_style);
        
        let arrow_style = TextStyle::new()
            .with_size(16.0 * self.content_scale)
            .with_color(Color::white())
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Center,
                vertical: VerticalAlignment::Center,
            });
        canvas.draw_text(up_button_rect, "↑", &arrow_style);
        
        let path_style = TextStyle::new()
            .with_size(12.0 * self.content_scale)
            .with_color(Color::new(0.8, 0.8, 0.8, 1.0))
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Left,
                vertical: VerticalAlignment::Center,
            });
        let path_text_rect = Rect::new(
            scaled_path_height + 5.0 * self.content_scale,
            0.0,
            width - scaled_path_height - 10.0 * self.content_scale,
            scaled_path_height,
        );
        
        let display_path = if self.current_path.len() > 50 {
            format!("...{}", &self.current_path[self.current_path.len() - 47..])
        } else {
            self.current_path.clone()
        };
        canvas.draw_text(path_text_rect, &display_path, &path_style);
        
        let content_style = Style::new()
            .with_background(Color::new(0.12, 0.12, 0.12, 1.0));
        let content_rect = self.get_content_rect();
        canvas.draw_rect(content_rect, &content_style);
        
        for (i, item) in self.items.iter().enumerate() {
            let item_rect = self.get_item_rect(i);
            
            if item_rect.y < scaled_path_height || item_rect.y + item_rect.height > height - scaled_search_height {
                continue;
            }
            
            let bg_color = if self.selected_index == Some(i) {
                Color::new(0.3, 0.5, 0.7, 1.0)
            } else if self.hovered_item_index == Some(i) {
                Color::new(0.22, 0.22, 0.22, 1.0)
            } else {
                Color::new(0.15, 0.15, 0.15, 1.0)
            };
            
            let item_style = Style::new().with_background(bg_color);
            canvas.draw_rect(item_rect, &item_style);
            
            let icon = if item.is_directory { "📁" } else { "📄" };
            let icon_style = TextStyle::new()
                .with_size(14.0 * self.content_scale)
                .with_color(Color::white())
                .with_alignment(TextAlignment {
                    horizontal: HorizontalAlignment::Left,
                    vertical: VerticalAlignment::Center,
                });
            canvas.draw_text(
                Rect::new(
                    5.0 * self.content_scale,
                    item_rect.y,
                    20.0 * self.content_scale,
                    item_rect.height,
                ),
                icon,
                &icon_style,
            );
            
            let name_style = TextStyle::new()
                .with_size(12.0 * self.content_scale)
                .with_color(Color::white())
                .with_alignment(TextAlignment {
                    horizontal: HorizontalAlignment::Left,
                    vertical: VerticalAlignment::Center,
                });
            canvas.draw_text(
                Rect::new(
                    25.0 * self.content_scale,
                    item_rect.y,
                    item_rect.width - 30.0 * self.content_scale,
                    item_rect.height,
                ),
                &item.name,
                &name_style,
            );
        }
        
        let search_bar_style = Style::new()
            .with_background(Color::new(0.18, 0.18, 0.18, 1.0))
            .with_border(Color::new(0.25, 0.25, 0.25, 1.0), 1.0, 0.0);
        let search_rect = Rect::new(0.0, height - scaled_search_height, width, scaled_search_height);
        canvas.draw_rect(search_rect, &search_bar_style);
        
        let search_label_style = TextStyle::new()
            .with_size(11.0 * self.content_scale)
            .with_color(Color::new(0.6, 0.6, 0.6, 1.0));
        canvas.draw_text(
            Rect::new(5.0 * self.content_scale, search_rect.y, 40.0 * self.content_scale, search_rect.height),
            "搜索:",
            &search_label_style,
        );
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        match event.event_type {
            EventType::TouchBegin => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    let scaled_path_height = self.path_bar_height * self.content_scale;
                    
                    if local_y >= 0.0 && local_y < scaled_path_height && local_x < scaled_path_height {
                        self.navigate_up();
                        return EventResult::Handled;
                    }
                    
                    if let Some(index) = self.hit_test_item(local_x, local_y) {
                        self.pressed_item_index = Some(index);
                        self.set_state(WidgetState::Pressed);
                        return EventResult::Handled;
                    }
                }
            }
            
            EventType::TouchEnd => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if let Some(pressed_index) = self.pressed_item_index {
                        if let Some(index) = self.hit_test_item(local_x, local_y) {
                            if index == pressed_index {
                                self.selected_index = Some(index);
                                if let Some(item) = self.items.get(index) {
                                    let path = item.path.clone();
                                    let is_dir = item.is_directory;
                                    self.trigger_select_callback(&path);
                                    
                                    if is_dir {
                                        self.navigate_to(&path);
                                    }
                                }
                            }
                        }
                        self.pressed_item_index = None;
                        self.set_state(WidgetState::Normal);
                        return EventResult::Stopped;
                    }
                }
            }
            
            EventType::MouseEnter => {
                if let EventData::Touch(touch) = &event.data {
                    self.hovered_item_index = self.hit_test_item(touch.x, touch.y);
                    if self.hovered_item_index.is_some() {
                        self.set_state(WidgetState::Hovered);
                        return EventResult::Handled;
                    }
                }
            }
            
            EventType::MouseLeave => {
                self.hovered_item_index = None;
                self.set_state(WidgetState::Normal);
                return EventResult::Handled;
            }
            
            EventType::TouchMove => {
                if let EventData::Touch(touch) = &event.data {
                    self.hovered_item_index = self.hit_test_item(touch.x, touch.y);
                }
            }
            
            EventType::DoubleClick => {
                if let Some(index) = self.selected_index {
                    if let Some(item) = self.items.get(index) {
                        let path = item.path.clone();
                        let is_dir = item.is_directory;
                        self.trigger_double_click_callback(&path);
                        if is_dir {
                            self.navigate_to(&path);
                        }
                    }
                }
                return EventResult::Handled;
            }
            
            _ => {}
        }
        
        EventResult::Ignored
    }
}

impl Default for FileBrowser {
    fn default() -> Self {
        Self::new()
    }
}

pub type FileBrowserSelectCallback = extern "C" fn(u64, *const std::ffi::c_char);
pub type FileBrowserDoubleClickCallback = extern "C" fn(u64, *const std::ffi::c_char);

pub fn trigger_file_browser_select_callback(browser_id: u64, path: &str) {
    crate::thunk_manager::trigger_file_browser_select_callback(browser_id, path);
}

pub fn trigger_file_browser_double_click_callback(browser_id: u64, path: &str) {
    crate::thunk_manager::trigger_file_browser_double_click_callback(browser_id, path);
}