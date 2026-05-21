use crate::canvas::*;
use crate::event::*;
use crate::layout::*;
use crate::style::*;
use crate::types::*;
use crate::widget::*;
use hezhou_dfx::*;

#[repr(i32)]
#[derive(Clone, Copy)]
pub enum DialogAction {
    Ok = 0,
    Cancel = 1,
    Yes = 2,
    No = 3,
    Custom = 100,
}

pub struct DialogButton {
    text: String,
    action: DialogAction,
    id: WidgetId,
}

impl DialogButton {
    pub fn new(text: &str, action: DialogAction) -> Self {
        Self {
            text: text.to_string(),
            action,
            id: WidgetId::new(),
        }
    }
    
    pub fn action_value(&self) -> i32 {
        self.action.action_value()
    }
}

impl DialogAction {
    pub fn action_value(&self) -> i32 {
        match self {
            DialogAction::Ok => 0,
            DialogAction::Cancel => 1,
            DialogAction::Yes => 2,
            DialogAction::No => 3,
            DialogAction::Custom => 100,
        }
    }
}

pub struct Dialog {
    id: WidgetId,
    parent_id: WidgetId,
    children: Vec<WidgetId>,
    layout: Layout,
    style: Style,
    state: WidgetState,
    flags: crate::widget::WidgetFlags,
    title: String,
    content_id: Option<WidgetId>,
    buttons: Vec<DialogButton>,
    is_visible: bool,
    result: Option<i32>,
    title_bar_height: f32,
    button_height: f32,
    hovered_button_index: Option<usize>,
    pressed_button_index: Option<usize>,
    content_scale: f32,
}

impl Dialog {
    pub fn new() -> Self {
        Self {
            id: WidgetId::new(),
            parent_id: WidgetId::invalid(),
            children: Vec::new(),
            layout: Layout::new(0.0, 0.0, 400.0, 300.0),
            style: Style::new()
                .with_background(Color::new(0.18, 0.18, 0.18, 1.0))
                .with_border(Color::new(0.3, 0.3, 0.3, 1.0), 1.0, 8.0),
            state: WidgetState::Normal,
            flags: crate::widget::WidgetFlags::default(),
            title: "Dialog".to_string(),
            content_id: None,
            buttons: Vec::new(),
            is_visible: false,
            result: None,
            title_bar_height: 36.0,
            button_height: 36.0,
            hovered_button_index: None,
            pressed_button_index: None,
            content_scale: 1.0,
        }
    }
    
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }
    
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.layout = Layout::new(
            (crate::thunk::ui_get_screen_size().0 - width) / 2.0,
            (crate::thunk::ui_get_screen_size().1 - height) / 2.0,
            width,
            height,
        );
        self
    }
    
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        self.flags.dirty_render = true;
    }
    
    pub fn set_content(&mut self, content_id: WidgetId) {
        self.content_id = Some(content_id);
        if !self.children.contains(&content_id) {
            self.children.push(content_id);
        }
    }
    
    pub fn add_button(&mut self, text: &str, action: DialogAction) -> WidgetId {
        let button = DialogButton::new(text, action);
        let id = button.id;
        self.buttons.push(button);
        id
    }
    
    pub fn show(&mut self) {
        self.is_visible = true;
        self.result = None;
        self.flags.dirty_render = true;
        dfx_info!("Dialog", "Show: id={}, title={}", self.id.id, self.title);
    }
    
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.flags.dirty_render = true;
        dfx_info!("Dialog", "Hide: id={}", self.id.id);
    }
    
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
    
    pub fn result(&self) -> Option<i32> {
        self.result
    }
    
    pub fn set_content_scale(&mut self, scale: f32) {
        self.content_scale = scale;
    }
    
    fn get_content_rect(&self) -> Rect {
        let scaled_title_height = self.title_bar_height * self.content_scale;
        let scaled_button_height = self.button_height * self.content_scale;
        Rect::new(
            0.0,
            scaled_title_height,
            self.layout.width,
            self.layout.height - scaled_title_height - scaled_button_height,
        )
    }
    
    fn get_button_rect(&self, index: usize) -> Rect {
        let scaled_button_height = self.button_height * self.content_scale;
        let button_count = self.buttons.len();
        if button_count == 0 || index >= button_count {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        
        let button_width = 80.0 * self.content_scale;
        let spacing = 10.0 * self.content_scale;
        let total_width = button_count as f32 * button_width + (button_count - 1) as f32 * spacing;
        let start_x = (self.layout.width - total_width) / 2.0;
        let y = self.layout.height - scaled_button_height - 10.0 * self.content_scale;
        
        Rect::new(
            start_x + index as f32 * (button_width + spacing),
            y,
            button_width,
            scaled_button_height,
        )
    }
    
    fn hit_test_button(&self, local_x: f32, local_y: f32) -> Option<usize> {
        let scaled_button_height = self.button_height * self.content_scale;
        let y_start = self.layout.height - scaled_button_height - 10.0 * self.content_scale;
        
        if local_y < y_start || local_y >= self.layout.height {
            return None;
        }
        
        for i in 0..self.buttons.len() {
            let rect = self.get_button_rect(i);
            if rect.contains(&Point::new(local_x, local_y)) {
                return Some(i);
            }
        }
        None
    }
    
    fn trigger_button_callback(&mut self, index: usize) {
        if index < self.buttons.len() {
            let action_value = self.buttons[index].action_value();
            self.result = Some(action_value);
            crate::thunk::queue_callback(crate::thunk::PendingCallback::DialogResult {
                dialog_id: self.id.id,
                result: action_value,
            });
            dfx_info!("Dialog", "Button clicked: id={}, index={}, action={}", 
                self.id.id, index, action_value);
        }
    }
}

impl Widget for Dialog {
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
        "Dialog"
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn draw(&mut self, canvas: &mut Canvas) {
        if !self.is_visible {
            return;
        }
        
        let width = self.layout.width;
        let height = self.layout.height;
        let scaled_title_height = self.title_bar_height * self.content_scale;
        let scaled_button_height = self.button_height * self.content_scale;
        
        if self.style.shadow.is_some() {
            canvas.draw_shadow(Rect::new(0.0, 0.0, width, height), self.style.shadow.as_ref().unwrap());
        }
        
        canvas.draw_rect(Rect::new(0.0, 0.0, width, height), &self.style);
        
        let title_bar_style = Style::new()
            .with_background(Color::new(0.22, 0.22, 0.22, 1.0));
        canvas.draw_rect(Rect::new(0.0, 0.0, width, scaled_title_height), &title_bar_style);
        
        let title_style = TextStyle::new()
            .with_size(16.0 * self.content_scale)
            .with_color(Color::white())
            .with_alignment(TextAlignment {
                horizontal: HorizontalAlignment::Center,
                vertical: VerticalAlignment::Center,
            });
        canvas.draw_text(
            Rect::new(0.0, 0.0, width, scaled_title_height),
            &self.title,
            &title_style,
        );
        
        let content_style = Style::new()
            .with_background(Color::new(0.15, 0.15, 0.15, 1.0));
        let content_rect = self.get_content_rect();
        canvas.draw_rect(content_rect, &content_style);
        
        let button_area_style = Style::new()
            .with_background(Color::new(0.18, 0.18, 0.18, 1.0));
        canvas.draw_rect(
            Rect::new(0.0, height - scaled_button_height - 10.0 * self.content_scale, width, scaled_button_height + 10.0 * self.content_scale),
            &button_area_style,
        );
        
        for (i, button) in self.buttons.iter().enumerate() {
            let rect = self.get_button_rect(i);
            
            let bg_color = if self.pressed_button_index == Some(i) {
                Color::new(0.35, 0.35, 0.35, 1.0)
            } else if self.hovered_button_index == Some(i) {
                Color::new(0.28, 0.28, 0.28, 1.0)
            } else {
                Color::new(0.25, 0.25, 0.25, 1.0)
            };
            
            let button_style = Style::new()
                .with_background(bg_color)
                .with_border(Color::new(0.35, 0.35, 0.35, 1.0), 1.0, 0.0);
            
            canvas.draw_rect(rect, &button_style);
            
            let text_style = TextStyle::new()
                .with_size(14.0 * self.content_scale)
                .with_color(Color::white())
                .with_alignment(TextAlignment {
                    horizontal: HorizontalAlignment::Center,
                    vertical: VerticalAlignment::Center,
                });
            canvas.draw_text(rect, &button.text, &text_style);
        }
    }
    
    fn on_event(&mut self, event: &Event) -> EventResult {
        if !self.is_visible {
            return EventResult::Ignored;
        }
        
        match event.event_type {
            EventType::TouchBegin => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if let Some(index) = self.hit_test_button(local_x, local_y) {
                        self.pressed_button_index = Some(index);
                        self.set_state(WidgetState::Pressed);
                        return EventResult::Handled;
                    }
                }
            }
            
            EventType::TouchEnd => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if let Some(pressed_index) = self.pressed_button_index {
                        if let Some(index) = self.hit_test_button(local_x, local_y) {
                            if index == pressed_index {
                                self.trigger_button_callback(index);
                            }
                        }
                        self.pressed_button_index = None;
                        self.set_state(WidgetState::Normal);
                        return EventResult::Stopped;
                    }
                }
            }
            
            EventType::MouseEnter => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    if let Some(index) = self.hit_test_button(local_x, local_y) {
                        self.hovered_button_index = Some(index);
                        self.set_state(WidgetState::Hovered);
                        return EventResult::Handled;
                    }
                }
            }
            
            EventType::MouseLeave => {
                self.hovered_button_index = None;
                self.set_state(WidgetState::Normal);
                return EventResult::Handled;
            }
            
            EventType::TouchMove => {
                if let EventData::Touch(touch) = &event.data {
                    let local_x = touch.x;
                    let local_y = touch.y;
                    
                    self.hovered_button_index = self.hit_test_button(local_x, local_y);
                }
            }
            
            _ => {}
        }
        
        EventResult::Ignored
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new()
    }
}

pub type DialogResultCallback = extern "C" fn(u64, i32);

pub fn trigger_dialog_result_callback(dialog_id: u64, result: i32) {
    crate::thunk::trigger_dialog_result_callback(dialog_id, result);
}