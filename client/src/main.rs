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

fn main() {
    let state = Arc::new(Mutex::new(None::<(VulkanRhi, WidgetTree, LayoutEngine<SimpleTextMeasurer>, Vec<EventHandlerEntry>)>));
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
    
    let mut ctx = BuildContext::new(Theme::default());
    let root_id = VStack::new()
        .spacing(16.0)
        .child(Text::title("标题"))
        .child(Button::new("按钮").on_click(|| println!("按钮被点击了！")))
        .build(&mut ctx);
    ctx.set_root(root_id);

    let event_handlers = ctx.take_event_handlers();
    let tree = ctx.build();
    let layout_engine = LayoutEngine::new(SimpleTextMeasurer);
    
    *state.lock().unwrap() = Some((rhi, tree, layout_engine, event_handlers));
    
    let state_clone = state.clone();
    let window_clone = window.clone();
    let mouse_pos_clone = mouse_pos.clone();
    
    event_loop.run(move |event, window_target| {
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    if let Some((rhi, _, _, _)) = state_clone.lock().unwrap().take() {
                        rhi.wait_idle();
                        drop(rhi);
                    }
                    window_target.exit();
                }

                winit::event::WindowEvent::Resized(size) => {
                    if let Some((ref mut rhi, ref mut tree, _, _)) = *state_clone.lock().unwrap() {
                        rhi.resize(size.width, size.height);
                        if let Some(root) = tree.root() {
                            tree.mark_dirty(root);
                        }
                    }
                }

                winit::event::WindowEvent::CursorMoved { position, .. } => {
                    *mouse_pos_clone.lock().unwrap() = (position.x as f32, position.y as f32);
                }

                winit::event::WindowEvent::MouseInput { state, button, .. } => {
                    if state == winit::event::ElementState::Pressed && button == winit::event::MouseButton::Left {
                        let (mx, my) = *mouse_pos_clone.lock().unwrap();
                        if let Some((_, ref tree, _, ref handlers)) = *state_clone.lock().unwrap() {
                            if let Some(root) = tree.root() {
                                if let Some(hit_id) = hit_test(tree, root, mx, my, 0.0, 0.0) {
                                    let mut target_id = hit_id;
                                    loop {
                                        let mut found = false;
                                        for entry in handlers {
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
                                        if let Some(parent) = tree.get(target_id).parent {
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
                    if let Some((ref mut rhi, ref mut tree, ref mut layout_engine, _)) = *state_clone.lock().unwrap() {
                        let (w, h) = rhi.framebuffer_size();
                        layout_engine.calculate_layout(tree, Size::new(w as f32, h as f32));
                        
                        let mut commands = Vec::new();
                        build_draw_commands(tree, tree.root().unwrap(), &mut commands);
                        
                        rhi.begin_frame();
                        rhi.draw(&commands);
                        rhi.end_frame();
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