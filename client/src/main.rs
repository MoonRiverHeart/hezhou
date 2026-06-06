use rhi::Rhi;
use rhi::{RhiInitDesc, WindowHandle};
use rhi_vulkan::VulkanRhi;
use core::ui::component::{BuildContext, Component, EventHandlerEntry};
use core::ui::component::theme::Theme;
use core::ui::layout::layout::LayoutEngine;
use core::ui::layout::text::SimpleTextMeasurer;
use core::ui::layout::text::TextMeasurer;
use core::ui::layout::geometry::{Size, EdgeInsets, Alignment};
use core::ui::layout::style::{Style, MainAlignment};
use core::ui::layout::widget::{WidgetTree, WidgetType, TextData};
use core::ui::event::types::UIEvent;
use core::ui::event::mouse::{MouseEvent, MouseEventType, MouseButton, Point};
use core::ui::event::modifier::Modifiers;
use core::ui::layout::msdf_measurer::MsdfTextMeasurer;
use client::draw_utils::{build_draw_commands, hit_test};
use std::sync::Arc;
use std::sync::Mutex;

struct AppState {
    rhi: VulkanRhi,
    tree: WidgetTree,
    layout_engine: LayoutEngine<SimpleTextMeasurer>,
    event_handlers: Vec<EventHandlerEntry>,
    msdf: Arc<Mutex<MsdfTextMeasurer>>,
    texture_uploaded: bool,
    needs_rebuild: bool,
}

fn main() {
    let msdf = Arc::new(Mutex::new(MsdfTextMeasurer::from_system("simhei", "times")));
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
    let mut layout_engine = LayoutEngine::new(SimpleTextMeasurer);
    
    // 初始构建 UI
    let (mut init_tree, init_handlers) = {
        let mut m = msdf.lock().unwrap();
        build_ui(&button_text, &mut m)
    };
    let (w, h) = rhi.framebuffer_size();
    layout_engine.calculate_layout(&mut init_tree, Size::new(w as f32, h as f32));
    
    *state.lock().unwrap() = Some(AppState {
        rhi,
        tree: init_tree,
        layout_engine,
        event_handlers: init_handlers,
        msdf: msdf.clone(),
        texture_uploaded: false,
        needs_rebuild: false,
    });
    
    let state_clone = state.clone();
    let window_clone = window.clone();
    let mouse_pos_clone = mouse_pos.clone();
    let button_text_clone = button_text.clone();
    let msdf_clone = msdf.clone();
    
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
                        app.needs_rebuild = true;
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
                                    let mut clicked = false;
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
                                                clicked = true;
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
                                    // 点击后标记需要重建
                                    if clicked {
                                        if let Some(ref mut app) = *state_clone.lock().unwrap() {
                                            app.needs_rebuild = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                winit::event::WindowEvent::RedrawRequested => {
                    if let Some(ref mut app) = *state_clone.lock().unwrap() {
                        // 按需重建 UI
                        if app.needs_rebuild {
                            let (mut new_tree, new_handlers) = {
                                let mut m = msdf_clone.lock().unwrap();
                                build_ui(&button_text_clone, &mut m)
                            };
                            let (w, h) = app.rhi.framebuffer_size();
                            app.layout_engine.calculate_layout(&mut new_tree, Size::new(w as f32, h as f32));
                            app.tree = new_tree;
                            app.event_handlers = new_handlers;
                            app.needs_rebuild = false;
                        }
                        
                        if !app.texture_uploaded {
                            let atlas = msdf_clone.lock().unwrap().font_atlas().clone();
                            app.rhi.upload_texture(&atlas.data, atlas.width, atlas.height);
                            app.texture_uploaded = true;
                        }
                        
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
            winit::event::Event::AboutToWait => {
                window_clone.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}

fn build_ui(
    button_text: &Arc<Mutex<String>>,
    msdf: &mut MsdfTextMeasurer,
) -> (WidgetTree, Vec<EventHandlerEntry>) {
    let start = std::time::Instant::now();
    
    let mut ctx = BuildContext::new(Theme::default());
    println!("[perf] BuildContext created: {:?}", start.elapsed());
    
    let root_id = ctx.create_node(
        WidgetType::Column,
        Style::new().cross_alignment(Alignment::Center),
    );
    ctx.tree.get_mut(root_id).style.main_alignment = MainAlignment::Center;
    ctx.set_root(root_id);
    
    // 标题
    let t0 = std::time::Instant::now();
    let title_glyphs = msdf.layout_text("标题", 240.0, f32::MAX).glyphs;
    println!("[perf] 标题 layout_text (240px): {:?}", t0.elapsed());
    
    let title_data = TextData::with_glyphs("标题", 240.0, title_glyphs);
    let title_id = ctx.create_node(
        WidgetType::Text(title_data),
        Style::new().cross_alignment(Alignment::Center),
    );
    ctx.add_child(root_id, title_id);
    
    // 间距
    let spacer_id = ctx.create_node(
        WidgetType::Spacer(Size::new(0.0, 16.0)),
        Style::default(),
    );
    ctx.add_child(root_id, spacer_id);
    
    // 按钮
    let t1 = std::time::Instant::now();
    let btn_label = button_text.lock().unwrap().clone();
    let btn_glyphs = msdf.layout_text(&btn_label, 160.0, f32::MAX).glyphs;
    println!("[perf] 按钮 layout_text (160px): {:?}", t1.elapsed());
    
    let btn_data = TextData::with_glyphs(&btn_label, 160.0, btn_glyphs);
    
    let padding = EdgeInsets::symmetric(8.0, 40.0);
    let font_size = 80.0;
    let text_size = msdf.measure_text(&btn_label, font_size, f32::MAX);
    let container_w = text_size.width + padding.left + padding.right + font_size * 0.6;
    let container_h = text_size.height + padding.top + padding.bottom + font_size * 0.4;
    
    let btn_container_id = ctx.create_node(
        WidgetType::Container,
        Style::new()
            .width(container_w)
            .height(container_h)
            .padding(padding)
            .background(ctx.theme.primary_color),
    );
    
    let btn_text_id = ctx.create_node(
        WidgetType::Text(btn_data),
        Style::new().cross_alignment(Alignment::Center),
    );
    ctx.add_child(btn_container_id, btn_text_id);
    ctx.add_child(root_id, btn_container_id);
    
    // 测试文字
    let t2 = std::time::Instant::now();
    let test_text = "你好，world！";
    let test_size = 360.0;
    let test_glyphs = msdf.layout_text(test_text, test_size, f32::MAX).glyphs;
    println!("[perf] 测试文字 layout_text (360px, {} chars): {:?}", test_text.chars().count(), t2.elapsed());
    
    let test_data = TextData::with_glyphs(test_text, test_size, test_glyphs);
    let test_id = ctx.create_node(
        WidgetType::Text(test_data),
        Style::new().cross_alignment(Alignment::Center),
    );
    ctx.add_child(root_id, test_id);
    
    // 注册点击事件
    let bt = button_text.clone();
    ctx.on_event(btn_container_id, move |_| {
        *bt.lock().unwrap() = "被点击了".to_string();
    });
    
    let t3 = std::time::Instant::now();
    let handlers = ctx.take_event_handlers();
    let tree = ctx.build();
    println!("[perf] ctx.build: {:?}", t3.elapsed());
    println!("[perf] TOTAL build_ui: {:?}", start.elapsed());
    
    (tree, handlers)
}