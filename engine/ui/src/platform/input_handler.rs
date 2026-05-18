use crate::event::MouseButton;
use crate::*;
use hezhou_platform::*;
use parking_lot::Mutex;
use std::sync::Arc;
use hezhou_platform::KeyCode;

pub struct UIInputHandler {
    event_dispatcher: Arc<Mutex<EventDispatcher>>,
    last_mouse_pos: Point,
    touch_active: bool,
    active_pointer_id: i32,
    screen_width: f32,
    screen_height: f32,
    shift_pressed: bool,
    ctrl_pressed: bool,
}

impl UIInputHandler {
    pub fn new(event_dispatcher: Arc<Mutex<EventDispatcher>>) -> Self {
        Self {
            event_dispatcher,
            last_mouse_pos: Point::zero(),
            touch_active: false,
            active_pointer_id: 0,
            screen_width: 800.0,
            screen_height: 600.0,
            shift_pressed: false,
            ctrl_pressed: false,
        }
    }
    
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    pub fn process_platform_events(&mut self, events: &[PlatformEvent]) {
        for event in events {
            self.process_event(event);
        }
    }

    fn process_event(&mut self, event: &PlatformEvent) {
        match event.kind {
            PlatformEventKind::Touch => {
            }
            PlatformEventKind::Key => {
                unsafe {
                    let key_event = event.data.key;
                    self.on_key_event(&key_event, event.timestamp);
                }
            }
            PlatformEventKind::Char => {
                unsafe {
                    let char_event = event.data.char_event;
                    self.on_char_event(&char_event, event.timestamp);
                }
            }
            PlatformEventKind::Mouse => {
            }
            PlatformEventKind::WindowResize => {
            }
            PlatformEventKind::WindowClose => {
            }
            PlatformEventKind::Lifecycle => {
            }
        }
    }

    pub fn on_touch_event(&mut self, touch: &TouchEvent, timestamp: u64) {
        let x = touch.x;
        let y = self.screen_height - touch.y;
        
        match touch.action {
            TouchAction::Begin => {
                self.touch_active = true;
                self.active_pointer_id = touch.pointer_id;

                let mut event = Event::new(EventType::TouchBegin, timestamp).with_data(
                    EventData::Touch(TouchData::new(x, y, touch.pointer_id as u32)),
                );

                self.event_dispatcher.lock().dispatch_event(&mut event);
            }
            TouchAction::Move => {
                if !self.touch_active || touch.pointer_id != self.active_pointer_id {
                    return;
                }

                let mut event = Event::new(EventType::TouchMove, timestamp).with_data(
                    EventData::Touch(TouchData::new(x, y, touch.pointer_id as u32)),
                );

                self.event_dispatcher.lock().dispatch_event(&mut event);
            }
            TouchAction::End => {
                self.touch_active = false;

                let mut event = Event::new(EventType::TouchEnd, timestamp).with_data(
                    EventData::Touch(TouchData::new(x, y, touch.pointer_id as u32)),
                );

                self.event_dispatcher.lock().dispatch_event(&mut event);
            }
            TouchAction::Cancel => {
                self.touch_active = false;

                let mut event = Event::new(EventType::TouchCancel, timestamp).with_data(
                    EventData::Touch(TouchData::new(x, y, touch.pointer_id as u32)),
                );

                self.event_dispatcher.lock().dispatch_event(&mut event);
            }
        }
    }

    pub fn on_mouse_event(&mut self, mouse: &MouseEvent, timestamp: u64) {
        let ui_button = convert_mouse_button(mouse.button);
        let x = mouse.x;
        let y = mouse.y;
        
        let modifiers = self.get_current_modifiers();

        match mouse.action {
            MouseAction::Press => {
                let mut event = Event::new(EventType::TouchBegin, timestamp)
                    .with_data(EventData::Touch(TouchData::new(x, y, 0).with_modifiers(modifiers)));

                self.event_dispatcher.lock().dispatch_event(&mut event);
            }
            MouseAction::Release => {
                let mut event = Event::new(EventType::TouchEnd, timestamp)
                    .with_data(EventData::Touch(TouchData::new(x, y, 0)));

                self.event_dispatcher.lock().dispatch_event(&mut event);
            }
            MouseAction::Move => {
                let mut event = Event::new(EventType::MouseEnter, timestamp).with_data(
                    EventData::Mouse(MouseData::new(x, y, ui_button)),
                );

                self.event_dispatcher.lock().dispatch_event(&mut event);
            }
            MouseAction::Scroll => {
            }
        }
    }

    pub fn on_key_event(&mut self, key: &KeyEvent, timestamp: u64) {
        println!("[UIInputHandler] Key event: keycode={} (Left={}, Right={}, Up={}, Down={}), action={}", 
                 key.keycode as u32, KeyCode::Left as u32, KeyCode::Right as u32, KeyCode::Up as u32, KeyCode::Down as u32,
                 match key.action {
                     KeyAction::Press => "Press",
                     KeyAction::Release => "Release",
                     KeyAction::Repeat => "Repeat",
                 });
        
        // 追踪 Shift 和 Ctrl 键状态
        if key.keycode == KeyCode::Shift {
            match key.action {
                KeyAction::Press | KeyAction::Repeat => self.shift_pressed = true,
                KeyAction::Release => self.shift_pressed = false,
            }
        }
        if key.keycode == KeyCode::Ctrl {
            match key.action {
                KeyAction::Press | KeyAction::Repeat => self.ctrl_pressed = true,
                KeyAction::Release => self.ctrl_pressed = false,
            }
        }

        let modifiers = convert_key_modifiers(&key.modifiers);

        let mut event = match key.action {
            KeyAction::Press | KeyAction::Repeat => Event::new(EventType::KeyDown, timestamp)
                .with_data(EventData::Key(KeyData::new(key.keycode as u32, modifiers))),
            KeyAction::Release => Event::new(EventType::KeyUp, timestamp)
                .with_data(EventData::Key(KeyData::new(key.keycode as u32, modifiers))),
        };

        self.event_dispatcher.lock().dispatch_event(&mut event);
    }
    
    fn get_current_modifiers(&self) -> u32 {
        let mut flags = 0u32;
        if self.shift_pressed { flags |= 1; }
        if self.ctrl_pressed { flags |= 2; }
        flags
    }

    pub fn on_char_event(&mut self, char_event: &CharEvent, timestamp: u64) {
        let mut event = Event::new(EventType::KeyDown, timestamp)
            .with_data(EventData::Key(KeyData::new(0, 0).with_unicode(char_event.codepoint)));

        self.event_dispatcher.lock().dispatch_event(&mut event);
    }

    pub fn on_resize(&mut self, width: i32, height: i32) {
        self.screen_width = width as f32;
        self.screen_height = height as f32;
    }
}

fn convert_mouse_button(platform_button: hezhou_platform::MouseButton) -> MouseButton {
    match platform_button {
        hezhou_platform::MouseButton::Left => MouseButton::Left,
        hezhou_platform::MouseButton::Right => MouseButton::Right,
        hezhou_platform::MouseButton::Middle => MouseButton::Middle,
        hezhou_platform::MouseButton::None => MouseButton::None,
    }
}

fn convert_key_modifiers(modifiers: &KeyModifiers) -> u32 {
    let mut flags = 0u32;
    if modifiers.shift {
        flags |= 1;
    }
    if modifiers.ctrl {
        flags |= 2;
    }
    if modifiers.alt {
        flags |= 4;
    }
    flags
}

impl Default for UIInputHandler {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(EventDispatcher::default())))
    }
}