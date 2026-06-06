use rhi::Rhi;
use rhi::{RhiInitDesc, WindowHandle};
use rhi_vulkan::VulkanRhi;
use core::ui::component::{BuildContext, Component};
use core::ui::component::theme::Theme;
use core::ui::component::text::Text;
use core::ui::component::button::Button;
use core::ui::component::container::VStack;
use core::ui::layout::layout::LayoutEngine;
use core::ui::layout::text::SimpleTextMeasurer;
use core::ui::layout::geometry::Size;
use core::ui::layout::widget::WidgetTree;
use client::draw_utils::build_draw_commands;
use std::sync::Arc;
use std::sync::Mutex;

fn main() {
    let state = Arc::new(Mutex::new(None::<(VulkanRhi, WidgetTree, LayoutEngine<SimpleTextMeasurer>)>));
    
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
        .child(Button::new("按钮"))
        .build(&mut ctx);
    ctx.set_root(root_id);
    let tree = ctx.build();
    let layout_engine = LayoutEngine::new(SimpleTextMeasurer);
    
    *state.lock().unwrap() = Some((rhi, tree, layout_engine));
    
    let state_clone = state.clone();
    let window_clone = window.clone();
    
    event_loop.run(move |event, window_target| {
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    if let Some((rhi, _, _)) = state_clone.lock().unwrap().take() {
                        rhi.wait_idle();
                        drop(rhi);
                    }
                    window_target.exit();
                }
                winit::event::WindowEvent::Resized(size) => {
                    if let Some((ref mut rhi, ref mut tree, _)) = *state_clone.lock().unwrap() {
                        rhi.resize(size.width, size.height);
                        if let Some(root) = tree.root() {
                            tree.mark_dirty(root);
                        }
                    }
                }
                winit::event::WindowEvent::RedrawRequested => {
                    if let Some((ref mut rhi, ref mut tree, ref mut layout_engine)) = *state_clone.lock().unwrap() {
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