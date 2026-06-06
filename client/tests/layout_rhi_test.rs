use client::draw_utils::build_draw_commands;
use core::ui::component::{BuildContext, Component};
use core::ui::component::theme::Theme;
use core::ui::component::text::Text;
use core::ui::component::button::Button;
use core::ui::component::container::VStack;
use core::ui::layout::layout::LayoutEngine;
use core::ui::layout::text::SimpleTextMeasurer;
use core::ui::layout::geometry::Size;

#[test]
fn test_ui_layout_to_draw_commands() {
    let mut ctx = BuildContext::new(Theme::default());
    let root_id = VStack::new()
        .spacing(16.0)
        .child(Text::title("标题"))
        .child(Button::new("按钮"))
        .build(&mut ctx);
    ctx.set_root(root_id);
    let mut tree = ctx.build();
    
    let mut layout_engine = LayoutEngine::new(SimpleTextMeasurer);
    layout_engine.calculate_layout(&mut tree, Size::new(800.0, 600.0));
    
    let mut commands = Vec::new();
    build_draw_commands(&tree, tree.root().unwrap(), &mut commands);
    
    assert!(!commands.is_empty(), "应该生成绘制命令");
    
    let root = tree.get(tree.root().unwrap());
    assert!(root.layout.is_some());
    let layout = root.layout.unwrap();
    assert_eq!(layout.width, 800.0);
    assert_eq!(layout.height, 600.0);
}

#[test]
fn test_rect_command() {
    let cmd = rhi::DrawCommand::rect(0.0, 0.0, 100.0, 50.0, 1.0, 0.0, 0.0, 1.0);
    assert_eq!(cmd.vertices.len(), 4);
    assert_eq!(cmd.indices, Some(vec![0, 1, 2, 2, 3, 0]));
}