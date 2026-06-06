use rhi::Rhi;
use rhi::{RhiInitDesc, WindowHandle};
use rhi_vulkan::VulkanRhi;
use core::ui::component::BuildContext;
use core::ui::component::theme::Theme;
use core::ui::layout::layout::LayoutEngine;
use core::ui::layout::text::SimpleTextMeasurer;
use core::ui::layout::text::TextMeasurer;
use core::ui::layout::geometry::Size;
use core::ui::layout::widget::WidgetId;
use core::ui::layout::widget::WidgetTree;
use core::ui::event::types::UIEvent;
use core::ui::event::mouse::{MouseEvent, MouseEventType, MouseButton, Point};
use core::ui::event::modifier::Modifiers;
use core::ui::layout::msdf_measurer::MsdfTextMeasurer;
use crate::draw_utils::{build_draw_commands, hit_test};
use std::sync::Arc;
use std::sync::Mutex;

pub struct AppState {
    rhi: VulkanRhi,
    tree: WidgetTree,
    layout_engine: LayoutEngine<SimpleTextMeasurer>,
    event_handlers: Vec<core::ui::component::EventHandlerEntry>,
    msdf: Arc<Mutex<MsdfTextMeasurer>>,
    texture_uploaded: bool,
}

pub struct Application {
    state: Arc<Mutex<Option<AppState>>>,
    mouse_pos: Arc<Mutex<(f32, f32)>>,
    msdf: Arc<Mutex<MsdfTextMeasurer>>,
}

impl Application {
    pub fn new() -> Self {
        let msdf = Arc::new(Mutex::new(MsdfTextMeasurer::from_system("simhei", "times")));
        Application {
            state: Arc::new(Mutex::new(None)),
            mouse_pos: Arc::new(Mutex::new((0.0, 0.0))),
            msdf,
        }
    }
    
    pub fn run(self, build_fn: impl FnOnce(&mut BuildContext) -> WidgetId + 'static) {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = Arc::new(event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        ).unwrap());
        
        let rhi_desc = RhiInitDesc {
            window_handle: WindowHandle {
                #[cfg(target_os = "windows")]
                hwnd: {
                    use raw_window_handle::HasWindowHandle;
                    match window.window_handle().unwrap().as_raw() {
                        raw_window_handle::RawWindowHandle::Win32(handle) => handle.hwnd.get() as *mut _,
                        _ => panic!("Unsupported"),
                    }
                },
            },
            width: 800,
            height: 600,
        };
        
        let rhi = VulkanRhi::init(&rhi_desc);
        let mut layout_engine = LayoutEngine::new(SimpleTextMeasurer);
        
        let mut ctx = BuildContext::with_msdf(Theme::default(), self.msdf.clone());
        let root_id = build_fn(&mut ctx);
        ctx.set_root(root_id);
        let handlers = ctx.take_event_handlers();
        let mut tree = ctx.build();
        
        let (w, h) = rhi.framebuffer_size();
        layout_engine.calculate_layout(&mut tree, Size::new(w as f32, h as f32));
        
        *self.state.lock().unwrap() = Some(AppState {
            rhi,
            tree,
            layout_engine,
            event_handlers: handlers,
            msdf: self.msdf.clone(),
            texture_uploaded: false,
        });
        
        let state_clone = self.state.clone();
        let window_clone = window.clone();
        let mouse_pos_clone = self.mouse_pos.clone();
        let msdf_clone = self.msdf.clone();
        
        event_loop.run(move |event, window_target| {
            match event {
                winit::event::Event::WindowEvent { event, .. } => match event {
                    winit::event::WindowEvent::CloseRequested => {
                        if let Some(mut app) = state_clone.lock().unwrap().take() {
                            app.rhi.wait_idle();
                            drop(app);
                        }
                        window_target.exit();
                    }
                    winit::event::WindowEvent::Resized(size) => {
                        if let Some(ref mut app) = *state_clone.lock().unwrap() {
                            app.rhi.resize(size.width, size.height);
                        }
                    }
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        *mouse_pos_clone.lock().unwrap() = (position.x as f32, position.y as f32);
                    }
                    winit::event::WindowEvent::MouseInput { state, button, .. } => {
                        if state == winit::event::ElementState::Pressed && button == winit::event::MouseButton::Left {
                            let (mx, my) = *mouse_pos_clone.lock().unwrap();
                            if let Some(ref app) = *state_clone.lock().unwrap() {
                                if let Some(root) = app.tree.root() {
                                    if let Some(hit_id) = hit_test(&app.tree, root, mx, my, 0.0, 0.0) {
                                        let mut target_id = hit_id;
                                        loop {
                                            let mut found = false;
                                            for entry in &app.event_handlers {
                                                if entry.widget_id == target_id {
                                                    let event = UIEvent::Mouse(MouseEvent {
                                                        timestamp: std::time::SystemTime::now(),
                                                        position: Point::new(mx, my),
                                                        button: Some(MouseButton::Left),
                                                        modifiers: Modifiers::default(),
                                                        event_type: MouseEventType::Clicked,
                                                        handled: false,
                                                    });
                                                    (entry.handler)(&event);
                                                    found = true;
                                                    break;
                                                }
                                            }
                                            if found { break; }
                                            if let Some(parent) = app.tree.get(target_id).parent {
                                                target_id = parent;
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        if let Some(ref mut app) = *state_clone.lock().unwrap() {
                            if !app.texture_uploaded && app.tree.root().is_some() {
                                let atlas = msdf_clone.lock().unwrap().font_atlas().clone();
                                app.rhi.upload_texture(&atlas.data, atlas.width, atlas.height);
                                app.texture_uploaded = true;
                            }
                            let (w, h) = app.rhi.framebuffer_size();
                            app.layout_engine.calculate_layout(&mut app.tree, Size::new(w as f32, h as f32));
                            let mut commands = Vec::new();
                            if let Some(root) = app.tree.root() {
                                build_draw_commands(&app.tree, root, &mut commands);
                            }
                            app.rhi.begin_frame();
                            app.rhi.draw(&commands);
                            app.rhi.end_frame();
                        }
                        window_clone.request_redraw();
                    }
                    _ => {}
                },
                winit::event::Event::AboutToWait => window_clone.request_redraw(),
                _ => {}
            }
        }).unwrap();
    }
}