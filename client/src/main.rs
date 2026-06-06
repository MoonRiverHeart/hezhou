use rhi::Rhi;
use rhi::{RhiInitDesc, WindowHandle, DrawCommand, Vertex};
use rhi_vulkan::VulkanRhi;
use core::ui::component::{BuildContext, Component};
use core::ui::component::theme::Theme;
use core::ui::component::text::Text;
use core::ui::component::button::Button;
use core::ui::component::container::VStack;
use core::ui::layout::layout::LayoutEngine;
use core::ui::layout::text::SimpleTextMeasurer;
use core::ui::layout::geometry::Size;
use core::ui::layout::style::Style;
use core::ui::layout::geometry::EdgeInsets;
use core::ui::layout::widget::{WidgetTree, WidgetId, WidgetType};
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

fn build_draw_commands(tree: &WidgetTree, node_id: WidgetId, commands: &mut Vec<DrawCommand>) {
    build_draw_commands_impl(tree, node_id, 0.0, 0.0, commands);
}

fn build_draw_commands_impl(tree: &WidgetTree, node_id: WidgetId, parent_x: f32, parent_y: f32, commands: &mut Vec<DrawCommand>) {
    let node = tree.get(node_id);
    let layout = if let Some(l) = node.layout { l } else { return };
    
    let abs_x = parent_x + layout.x;
    let abs_y = parent_y + layout.y;
    
    match &node.widget_type {
        WidgetType::Container | WidgetType::Row | WidgetType::Column => {
            let margin = node.style.margin;
            let outer_x = abs_x - margin.left;
            let outer_y = abs_y - margin.top;
            let outer_w = layout.width + margin.left + margin.right;
            let outer_h = layout.height + margin.top + margin.bottom;
            
            if outer_w > 0.0 && outer_h > 0.0 {
                commands.push(rect_line(
                    outer_x, outer_y, outer_w, outer_h,
                    0.0, 1.0, 0.0, 1.0,
                ));
            }
            
            let padding = node.style.padding;
            let inner_x = abs_x + padding.left;
            let inner_y = abs_y + padding.top;
            let inner_w = layout.width - padding.left - padding.right;
            let inner_h = layout.height - padding.top - padding.bottom;
            
            if inner_w > 0.0 && inner_h > 0.0 {
                commands.push(rect_line(
                    inner_x, inner_y, inner_w, inner_h,
                    1.0, 0.0, 0.0, 1.0,
                ));
            }
            
            // 打印红框中心
            let has_text_child = node.children.iter().any(|&cid| {
                matches!(tree.get(cid).widget_type, WidgetType::Text(_))
            });
            if has_text_child {
                let red_center_x = inner_x + inner_w / 2.0;
                let red_center_y = inner_y + inner_h / 2.0;
                println!("DEBUG red frame center: ({:.1},{:.1}) inner=({:.1},{:.1},{:.1}x{:.1})",
                    red_center_x, red_center_y, inner_x, inner_y, inner_w, inner_h);
            }
        }
        WidgetType::Text(data) => {
            let font_size = data.font_size;
            let char_width = font_size * 1.0;
            let total_text_width = data.content.len() as f32 * char_width;
            let text_center_x = abs_x + total_text_width / 2.0;
            let text_center_y = abs_y + font_size * 0.6;
            
            println!("DEBUG text center: '{}' center=({:.1},{:.1}) abs_pos=({:.1},{:.1}) total_width={:.1}",
                data.content, text_center_x, text_center_y, abs_x, abs_y, total_text_width);
            
            for i in 0..data.content.len() {
                let x = abs_x + i as f32 * char_width;
                let y = abs_y;
                commands.push(rect_command(
                    x, y, char_width - 1.0, font_size * 1.2,
                    1.0, 1.0, 1.0, 1.0,
                ));
            }
        }
        _ => {}
    }
    
    for &child_id in &node.children {
        build_draw_commands_impl(tree, child_id, abs_x, abs_y, commands);
    }
}

fn rect_command(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32) -> DrawCommand {
    DrawCommand {
        vertices: vec![
            Vertex { position: [x, y], color: [r, g, b, a], uv: [0.0, 0.0] },
            Vertex { position: [x + w, y], color: [r, g, b, a], uv: [1.0, 0.0] },
            Vertex { position: [x + w, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
            Vertex { position: [x, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
        ],
        indices: Some(vec![0, 1, 2, 2, 3, 0]),
        clip_rect: None,
        texture_id: 0,
    }
}

fn rect_line(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32) -> DrawCommand {
    let t = 1.0;
    let vertices = vec![
        Vertex { position: [x, y], color: [r, g, b, a], uv: [0.0, 0.0] },
        Vertex { position: [x + w, y], color: [r, g, b, a], uv: [1.0, 0.0] },
        Vertex { position: [x + w, y + t], color: [r, g, b, a], uv: [1.0, 1.0] },
        Vertex { position: [x, y + t], color: [r, g, b, a], uv: [0.0, 1.0] },
        Vertex { position: [x, y + h - t], color: [r, g, b, a], uv: [0.0, 0.0] },
        Vertex { position: [x + w, y + h - t], color: [r, g, b, a], uv: [1.0, 0.0] },
        Vertex { position: [x + w, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
        Vertex { position: [x, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
        Vertex { position: [x, y], color: [r, g, b, a], uv: [0.0, 0.0] },
        Vertex { position: [x + t, y], color: [r, g, b, a], uv: [1.0, 0.0] },
        Vertex { position: [x + t, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
        Vertex { position: [x, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
        Vertex { position: [x + w - t, y], color: [r, g, b, a], uv: [0.0, 0.0] },
        Vertex { position: [x + w, y], color: [r, g, b, a], uv: [1.0, 0.0] },
        Vertex { position: [x + w, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
        Vertex { position: [x + w - t, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
    ];
    let indices = vec![
        0,1,2, 2,3,0,
        4,5,6, 6,7,4,
        8,9,10, 10,11,8,
        12,13,14, 14,15,12,
    ];
    DrawCommand {
        vertices,
        indices: Some(indices),
        clip_rect: None,
        texture_id: 0,
    }
}