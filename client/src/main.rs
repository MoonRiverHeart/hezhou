use rhi::Rhi;
use rhi::{RhiInitDesc, WindowHandle};
use rhi_vulkan::VulkanRhi;
use core::ui::component::{BuildContext, Component, EventHandlerEntry};
use core::ui::component::theme::Theme;
use core::ui::component::text::Text;
use core::ui::component::button::Button;
use core::ui::component::container::VStack;
use core::ui::layout::layout::LayoutEngine;
use core::ui::layout::text::SimpleTextMeasurer;
use core::ui::layout::geometry::Size;
use core::ui::layout::widget::WidgetTree;
use core::ui::event::types::UIEvent;
use core::ui::event::mouse::{MouseEvent, MouseEventType, MouseButton, Point};
use core::ui::event::modifier::Modifiers;
use client::draw_utils::{build_draw_commands, hit_test};
use std::sync::Arc;
use std::sync::Mutex;
use core::ui::layout::msdf_measurer::MsdfTextMeasurer;
use core::ui::layout::text::TextMeasurer;

struct AppState {
    rhi: VulkanRhi,
    tree: WidgetTree,
    layout_engine: LayoutEngine<SimpleTextMeasurer>,
    event_handlers: Vec<EventHandlerEntry>,
}

fn main() {
    // 在 main() 函数开头加载字体
    let text_measurer = MsdfTextMeasurer::from_system("simhei", "times");

    let button_text = Arc::new(Mutex::new("按钮".to_string()));
    
    let state = Arc::new(Mutex::new(None::<AppState>));
    let mouse_pos = Arc::new(Mutex::new((0.0f32, 0.0f32)));
    
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
    let layout_engine = LayoutEngine::new(SimpleTextMeasurer);
    // let layout_engine = LayoutEngine::new(text_measurer);
    
    *state.lock().unwrap() = Some(AppState { rhi, tree: WidgetTree::new(), layout_engine, event_handlers: vec![] });
    
    let state_clone = state.clone();
    let window_clone = window.clone();
    let mouse_pos_clone = mouse_pos.clone();
    let button_text_clone = button_text.clone();
    
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
                    // 每次重绘时重建UI并布局
                    let (mut new_tree, new_handlers) = build_ui(&button_text_clone);
                    if let Some(ref mut app) = *state_clone.lock().unwrap() {
                        let (w, h) = app.rhi.framebuffer_size();
                        app.layout_engine.calculate_layout(&mut new_tree, Size::new(w as f32, h as f32));
                        
                        let mut commands = Vec::new();
                        if let Some(root) = new_tree.root() {
                            build_draw_commands(&new_tree, root, &mut commands);
                        }
                        
                        app.rhi.begin_frame();
                        app.rhi.draw(&commands);
                        app.rhi.end_frame();
                        
                        app.tree = new_tree;
                        app.event_handlers = new_handlers;
                    }
                    window_clone.request_redraw();
                }
                _ => {}
            },
            winit::event::Event::AboutToWait => {
                window_clone.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}

fn build_ui(button_text: &Arc<Mutex<String>>) -> (WidgetTree, Vec<EventHandlerEntry>) {
    let mut ctx = BuildContext::new(Theme::default());
    let btn_text = button_text.clone();  // clone Arc
    let root_id = VStack::new()
        .spacing(16.0)
        .child(Text::title("标题"))
        .child(Button::dynamic(btn_text).on_click({
            let bt = button_text.clone();  // 再 clone 一个给闭包
            move || {
                *bt.lock().unwrap() = "按钮被点击了！".to_string();
            }
        }))
        .build(&mut ctx);
    ctx.set_root(root_id);
    let handlers = ctx.take_event_handlers();
    (ctx.build(), handlers)
}