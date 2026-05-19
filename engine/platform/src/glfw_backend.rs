use crate::event::*;
use crate::traits::*;
use crate::window::*;
use hezhou_dfx::*;
use glfw::Context;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct GLFWPlatform {
    glfw: Option<glfw::Glfw>,
    window: Option<glfw::PWindow>,
    event_receiver: Option<glfw::GlfwReceiver<(f64, glfw::WindowEvent)>>,
    event_callbacks: Arc<Mutex<Vec<EventCallback>>>,
    running: bool,
    last_mouse_x: f64,
    last_mouse_y: f64,
    content_scale_x: f32,
    content_scale_y: f32,
}

impl GLFWPlatform {
    pub fn new() -> Self {
        Self {
            glfw: None,
            window: None,
            event_receiver: None,
            event_callbacks: Arc::new(Mutex::new(Vec::new())),
            running: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            content_scale_x: 1.0,
            content_scale_y: 1.0,
        }
    }
    
    pub fn get_content_scale(&self) -> (f32, f32) {
        (self.content_scale_x, self.content_scale_y)
    }
}

impl Default for GLFWPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for GLFWPlatform {
    fn name(&self) -> &'static str {
        "GLFW"
    }

    fn init(&mut self) -> Result<(), String> {
        let glfw =
            glfw::init(glfw::fail_on_errors).map_err(|e| format!("GLFW init failed: {}", e))?;
        self.glfw = Some(glfw);
        self.running = true;
        Ok(())
    }

    fn shutdown(&mut self) {
        self.window = None;
        self.glfw = None;
        self.running = false;
    }

    fn create_window(
        &mut self,
        title: &str,
        width: i32,
        height: i32,
    ) -> Result<WindowHandle, String> {
        let glfw = self.glfw.as_mut().ok_or("GLFW not initialized")?;

        let (mut window, events) = glfw
            .create_window(
                width as u32,
                height as u32,
                title,
                glfw::WindowMode::Windowed,
            )
            .expect("Failed to create GLFW window");

        window.set_all_polling(true);
        window.make_current();
        
        let (scale_x, scale_y) = window.get_content_scale();
        self.content_scale_x = scale_x;
        self.content_scale_y = scale_y;
        dfx_info!("GLFW", "Content scale: x={}, y={} (DPI: {})", scale_x, scale_y, scale_x * 96.0);

        let handle = WindowHandle::new(
            NativeWindowType::GLFW,
            window.window_ptr() as usize,
            width,
            height,
        );

        self.window = Some(window);
        self.event_receiver = Some(events);

        Ok(handle)
    }

    fn destroy_window(&mut self, _window: &WindowHandle) {
        self.window = None;
    }

    fn get_window_handle(&self) -> Option<WindowHandle> {
        self.window.as_ref().map(|w| {
            let (width, height) = w.get_size();
            WindowHandle::new(
                NativeWindowType::GLFW,
                w.window_ptr() as usize,
                width,
                height,
            )
        })
    }

    fn set_window_title(&mut self, _window: &WindowHandle, title: &str) {
        if let Some(window) = &mut self.window {
            window.set_title(title);
        }
    }

    fn set_window_size(&mut self, _window: &WindowHandle, width: i32, height: i32) {
        if let Some(window) = &mut self.window {
            window.set_size(width, height);
        }
    }

    fn get_window_size(&self, _window: &WindowHandle) -> (i32, i32) {
        self.window
            .as_ref()
            .map(|w| {
                let (width, height) = w.get_size();
                (width, height)
            })
            .unwrap_or((0, 0))
    }

    fn poll_events(&mut self) -> Vec<PlatformEvent> {
        let mut events = Vec::new();

        if let Some(glfw) = &mut self.glfw {
            glfw.poll_events();

            let time = self.get_time();

            if let Some(receiver) = &self.event_receiver {
                for (_, event) in glfw::flush_messages(&receiver) {
                    let platform_event = GLFWPlatform::convert_glfw_event_static(
                        event,
                        time,
                        &mut self.last_mouse_x,
                        &mut self.last_mouse_y,
                    );
                    events.push(platform_event);
                }
            }

            if let Some(window) = &self.window {
                if window.should_close() {
                    self.running = false;
                    events.push(PlatformEvent {
                        kind: PlatformEventKind::WindowClose,
                        timestamp: (time * 1000.0) as u64,
                        data: PlatformEventData {
                            lifecycle: LifecycleEvent {
                                state: LifecycleState::Destroy,
                            },
                        },
                    });
                }
            }
        }

        for callback in self.event_callbacks.lock().iter() {
            for event in &events {
                callback(event);
            }
        }

        events
    }

    fn wait_events(&mut self) -> Vec<PlatformEvent> {
        if let Some(glfw) = &mut self.glfw {
            glfw.wait_events();
        }
        self.poll_events()
    }

    fn register_event_callback(&mut self, callback: EventCallback) {
        self.event_callbacks.lock().push(callback);
    }

    fn get_time(&self) -> f64 {
        self.glfw.as_ref().map(|g| g.get_time()).unwrap_or(0.0)
    }

    fn sleep(&self, seconds: f64) {
        std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn request_quit(&mut self) {
        self.running = false;
        if let Some(window) = &mut self.window {
            window.set_should_close(true);
        }
    }

    fn get_native_display(&self) -> Option<usize> {
        None
    }
    
    fn get_content_scale(&self) -> (f32, f32) {
        (self.content_scale_x, self.content_scale_y)
    }
}

impl GLFWPlatform {
    fn convert_glfw_event_static(
        event: glfw::WindowEvent,
        time: f64,
        last_mouse_x: &mut f64,
        last_mouse_y: &mut f64,
    ) -> PlatformEvent {
        let timestamp = (time * 1000.0) as u64;

        match event {
            glfw::WindowEvent::Key(key, _scancode, action, mods) => {
                let keycode = GLFWPlatform::convert_glfw_key(key);
                let key_action = match action {
                    glfw::Action::Press => KeyAction::Press,
                    glfw::Action::Release => KeyAction::Release,
                    glfw::Action::Repeat => KeyAction::Repeat,
                };
                
                let key_modifiers = KeyModifiers {
                    shift: mods.contains(glfw::Modifiers::Shift),
                    ctrl: mods.contains(glfw::Modifiers::Control),
                    alt: mods.contains(glfw::Modifiers::Alt),
                };
                
                dfx_info!("GLFW", "Key event: glfw_key={}, keycode={}, action={}, modifiers(shift={},ctrl={},alt={})", 
                         match key {
                             glfw::Key::Left => "Left",
                             glfw::Key::Right => "Right",
                             glfw::Key::Up => "Up",
                             glfw::Key::Down => "Down",
                             glfw::Key::C => "C",
                             glfw::Key::V => "V",
                             glfw::Key::X => "X",
                             _ => "Other"
                         },
                         keycode as u32,
                         match key_action {
                             KeyAction::Press => "Press",
                             KeyAction::Release => "Release",
                             KeyAction::Repeat => "Repeat",
                         },
                         key_modifiers.shift,
                         key_modifiers.ctrl,
                         key_modifiers.alt);

                PlatformEvent {
                    kind: PlatformEventKind::Key,
                    timestamp,
                    data: PlatformEventData {
                        key: KeyEvent {
                            action: key_action,
                            keycode,
                            modifiers: key_modifiers,
                        },
                    },
                }
            }

            glfw::WindowEvent::Char(codepoint) => {
                PlatformEvent {
                    kind: PlatformEventKind::Char,
                    timestamp,
                    data: PlatformEventData {
                        char_event: CharEvent {
                            codepoint: codepoint as u32,
                        },
                    },
                }
            }

            glfw::WindowEvent::MouseButton(button, action, _mods) => {
                let _mouse_button = match button {
                    glfw::MouseButtonLeft => MouseButton::Left,
                    glfw::MouseButtonRight => MouseButton::Right,
                    glfw::MouseButtonMiddle => MouseButton::Middle,
                    _ => MouseButton::Left,
                };

                let _mouse_action = match action {
                    glfw::Action::Press => MouseAction::Press,
                    glfw::Action::Release => MouseAction::Release,
                    _ => MouseAction::Move,
                };

                PlatformEvent {
                    kind: PlatformEventKind::Mouse,
                    timestamp,
                    data: PlatformEventData {
                        mouse: MouseEvent {
                            action: _mouse_action,
                            button: _mouse_button,
                            x: *last_mouse_x as f32,
                            y: *last_mouse_y as f32,
                            dx: 0.0,
                            dy: 0.0,
                        },
                    },
                }
            }

            glfw::WindowEvent::CursorPos(x, y) => {
                let _dx = x - *last_mouse_x;
                let _dy = y - *last_mouse_y;
                *last_mouse_x = x;
                *last_mouse_y = y;

                PlatformEvent {
                    kind: PlatformEventKind::Mouse,
                    timestamp,
                    data: PlatformEventData {
                        mouse: MouseEvent {
                            action: MouseAction::Move,
                            button: MouseButton::None,
                            x: x as f32,
                            y: y as f32,
                            dx: _dx as f32,
                            dy: _dy as f32,
                        },
                    },
                }
            }

            glfw::WindowEvent::Scroll(x, y) => PlatformEvent {
                kind: PlatformEventKind::Mouse,
                timestamp,
                data: PlatformEventData {
                    mouse: MouseEvent {
                        action: MouseAction::Scroll,
                        button: MouseButton::None,
                        x: *last_mouse_x as f32,
                        y: *last_mouse_y as f32,
                        dx: x as f32,
                        dy: y as f32,
                    },
                },
            },

            glfw::WindowEvent::Size(_width, _height) => PlatformEvent {
                kind: PlatformEventKind::WindowResize,
                timestamp,
                data: PlatformEventData {
                    window: WindowEvent {
                        width: _width,
                        height: _height,
                    },
                },
            },

            glfw::WindowEvent::Close => PlatformEvent {
                kind: PlatformEventKind::WindowClose,
                timestamp,
                data: PlatformEventData {
                    lifecycle: LifecycleEvent {
                        state: LifecycleState::Destroy,
                    },
                },
            },

            _ => PlatformEvent {
                kind: PlatformEventKind::Touch,
                timestamp,
                data: PlatformEventData {
                    touch: TouchEvent {
                        action: TouchAction::Cancel,
                        x: 0.0,
                        y: 0.0,
                        pointer_id: 0,
                    },
                },
            },
        }
    }

    fn convert_glfw_key(key: glfw::Key) -> KeyCode {
        match key {
            glfw::Key::A => KeyCode::A,
            glfw::Key::B => KeyCode::B,
            glfw::Key::C => KeyCode::C,
            glfw::Key::D => KeyCode::D,
            glfw::Key::E => KeyCode::E,
            glfw::Key::F => KeyCode::F,
            glfw::Key::G => KeyCode::G,
            glfw::Key::H => KeyCode::H,
            glfw::Key::I => KeyCode::I,
            glfw::Key::J => KeyCode::J,
            glfw::Key::K => KeyCode::K,
            glfw::Key::L => KeyCode::L,
            glfw::Key::M => KeyCode::M,
            glfw::Key::N => KeyCode::N,
            glfw::Key::O => KeyCode::O,
            glfw::Key::P => KeyCode::P,
            glfw::Key::Q => KeyCode::Q,
            glfw::Key::R => KeyCode::R,
            glfw::Key::S => KeyCode::S,
            glfw::Key::T => KeyCode::T,
            glfw::Key::U => KeyCode::U,
            glfw::Key::V => KeyCode::V,
            glfw::Key::W => KeyCode::W,
            glfw::Key::X => KeyCode::X,
            glfw::Key::Y => KeyCode::Y,
            glfw::Key::Z => KeyCode::Z,
            glfw::Key::Space => KeyCode::Space,
            glfw::Key::Enter => KeyCode::Enter,
            glfw::Key::Escape => KeyCode::Escape,
            glfw::Key::Backspace => KeyCode::Backspace,
            glfw::Key::Tab => KeyCode::Tab,
            glfw::Key::LeftShift | glfw::Key::RightShift => KeyCode::Shift,
            glfw::Key::LeftControl | glfw::Key::RightControl => KeyCode::Ctrl,
            glfw::Key::LeftAlt | glfw::Key::RightAlt => KeyCode::Alt,
            glfw::Key::Left => KeyCode::Left,
            glfw::Key::Right => KeyCode::Right,
            glfw::Key::Up => KeyCode::Up,
            glfw::Key::Down => KeyCode::Down,
            glfw::Key::Home => KeyCode::Home,
            glfw::Key::End => KeyCode::End,
            glfw::Key::Num0 => KeyCode::Num0,
            glfw::Key::Num1 => KeyCode::Num1,
            glfw::Key::Num2 => KeyCode::Num2,
            glfw::Key::Num3 => KeyCode::Num3,
            glfw::Key::Num4 => KeyCode::Num4,
            glfw::Key::Num5 => KeyCode::Num5,
            glfw::Key::Num6 => KeyCode::Num6,
            glfw::Key::Num7 => KeyCode::Num7,
            glfw::Key::Num8 => KeyCode::Num8,
            glfw::Key::Num9 => KeyCode::Num9,
            _ => KeyCode::Unknown,
        }
    }
}
