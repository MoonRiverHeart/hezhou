use core::ui::layout::geometry::*;
use core::ui::layout::style::*;
use core::ui::layout::layout::LayoutEngine;
use core::ui::layout::text::SimpleTextMeasurer;
use core::ui::layout::widget::*;
use core::ui::component::*;
use core::ui::component::theme::Theme;
use core::ui::component::text::Text;
use core::ui::component::button::{Button, ButtonVariant};
use core::ui::component::text_input::TextInput;
use core::ui::component::text_area::TextArea;
use core::ui::component::list::{List, ListItem};
use core::ui::component::grid::{Grid, GridColumn, GridCell};
use core::ui::component::container::{VStack, HStack, ZStack};

fn setup_ctx() -> BuildContext {
    BuildContext::new(Theme::default())
}

fn debug_tree(tree: &WidgetTree, node_id: WidgetId, depth: usize) {
    let node = tree.get(node_id);
    let indent = "  ".repeat(depth);
    
    let type_name = match &node.widget_type {
        WidgetType::Container => "Container",
        WidgetType::Row => "Row",
        WidgetType::Column => "Column",
        WidgetType::Text(t) => &format!("Text({})", t.content),
        WidgetType::Spacer(s) => &format!("Spacer({}x{})", s.width, s.height),
    };
    
    if let Some(layout) = node.layout {
        println!("{}└─ {} at ({:.1}, {:.1}) size {:.1}x{:.1}",
            indent, type_name, layout.x, layout.y, layout.width, layout.height);
    } else {
        println!("{}└─ {} (no layout)", indent, type_name);
    }
    
    for &child_id in &node.children {
        debug_tree(tree, child_id, depth + 1);
    }
}

// ============ Text 测试 ============

#[test]
fn test_text_component() {
    let mut ctx = setup_ctx();
    let text_id = Text::new("Hello World").font_size(16.0).build(&mut ctx);
    ctx.set_root(text_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(400.0, 100.0));
    
    let layout = tree.get(text_id).layout.unwrap();
    assert!(layout.width > 0.0);
    assert!(layout.height > 0.0);
}

#[test]
fn test_text_title_and_body() {
    let mut ctx = setup_ctx();
    let column_id = ctx.create_node(WidgetType::Column, Style::new());
    ctx.set_root(column_id);
    
    let title_id = Text::title("这是标题").build(&mut ctx);
    let body_id = Text::body("这是正文内容").build(&mut ctx);
    let caption_id = Text::caption("这是小字备注").build(&mut ctx);
    
    ctx.add_child(column_id, title_id);
    ctx.add_child(column_id, body_id);
    ctx.add_child(column_id, caption_id);
    
    let mut tree = ctx.build();
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(300.0, 200.0));
    
    println!("\nText hierarchy:");
    debug_tree(&tree, column_id, 0);
    
    let title_layout = tree.get(title_id).layout.unwrap();
    let body_layout = tree.get(body_id).layout.unwrap();
    let caption_layout = tree.get(caption_id).layout.unwrap();
    
    assert!(title_layout.height > body_layout.height);
    assert!(caption_layout.height < body_layout.height);
}

// ============ Button 测试 ============

#[test]
fn test_button_component() {
    let mut ctx = setup_ctx();
    let button_id = Button::new("点击我")
        .variant(ButtonVariant::Primary)
        .build(&mut ctx);
    
    ctx.set_root(button_id);
    let tree = ctx.build();
    
    let button_node = tree.get(button_id);
    assert!(matches!(button_node.widget_type, WidgetType::Container));
    assert_eq!(button_node.children.len(), 1);
    
    let text_id = button_node.children[0];
    let text_node = tree.get(text_id);
    if let WidgetType::Text(ref data) = text_node.widget_type {
        assert_eq!(data.content, "点击我");
    }
}

#[test]
fn test_button_variants() {
    let mut ctx = setup_ctx();
    let column_id = ctx.create_node(WidgetType::Column, Style::new());
    ctx.set_root(column_id);
    
    let primary_id = Button::new("主要按钮").variant(ButtonVariant::Primary).build(&mut ctx);
    let secondary_id = Button::new("次要按钮").variant(ButtonVariant::Secondary).build(&mut ctx);
    let text_btn_id = Button::new("文本按钮").variant(ButtonVariant::Text).build(&mut ctx);
    
    ctx.add_child(column_id, primary_id);
    ctx.add_child(column_id, secondary_id);
    ctx.add_child(column_id, text_btn_id);
    
    let mut tree = ctx.build();
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(200.0, 200.0));
    
    assert!(tree.get(primary_id).layout.is_some());
    assert!(tree.get(secondary_id).layout.is_some());
    assert!(tree.get(text_btn_id).layout.is_some());
}

// ============ VStack / HStack 测试 ============

#[test]
fn test_vstack_component() {
    let mut ctx = setup_ctx();
    let stack_id = VStack::new()
        .spacing(10.0)
        .child(Text::body("第一行"))
        .child(Text::body("第二行"))
        .child(Text::body("第三行"))
        .build(&mut ctx);
    
    ctx.set_root(stack_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(300.0, 200.0));
    
    println!("\nVStack layout:");
    debug_tree(&tree, stack_id, 0);
    
    let children = tree.get_children(stack_id);
    assert_eq!(children.len(), 5); // 3 text + 2 spacer
    
    let child0 = tree.get(children[0]).layout.unwrap();
    let child1 = tree.get(children[1]).layout.unwrap();
    let child2 = tree.get(children[2]).layout.unwrap();
    
    assert!(child1.y > child0.y + child0.height - 0.01);
    assert!(child2.y > child1.y + child1.height - 0.01);
}

#[test]
fn test_hstack_component() {
    let mut ctx = setup_ctx();
    let stack_id = HStack::new()
        .spacing(8.0)
        .child(Text::body("左"))
        .child(Text::body("中"))
        .child(Text::body("右"))
        .build(&mut ctx);
    
    ctx.set_root(stack_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(400.0, 50.0));
    
    println!("\nHStack layout:");
    debug_tree(&tree, stack_id, 0);
    
    let children = tree.get_children(stack_id);
    assert_eq!(children.len(), 5);
    
    let child0 = tree.get(children[0]).layout.unwrap();
    let child2 = tree.get(children[2]).layout.unwrap();
    assert!(child2.x > child0.x + child0.width - 0.01);
}

// ============ List 测试 ============

#[test]
fn test_list_component() {
    let mut ctx = setup_ctx();
    let list_id = List::new()
        .item(ListItem::new(Text::body("项目A")))
        .item(ListItem::new(Text::body("项目B")))
        .item(ListItem::new(Text::body("项目C")))
        .build(&mut ctx);
    
    ctx.set_root(list_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(300.0, 400.0));
    
    let children = tree.get_children(list_id);
    assert_eq!(children.len(), 3);
    
    for &child_id in &children {
        assert!(tree.get(child_id).layout.is_some());
    }
}

// ============ Grid 测试 ============

#[test]
fn test_grid_component() {
    let mut ctx = setup_ctx();
    let grid_id = Grid::new(vec![
        GridColumn::flex(1.0),
        GridColumn::flex(1.0),
        GridColumn::flex(1.0),
    ])
    .row_gap(8.0)
    .column_gap(8.0)
    .cell(GridCell::new(Text::body("A1")))
    .cell(GridCell::new(Text::body("B1")))
    .cell(GridCell::new(Text::body("C1")))
    .cell(GridCell::new(Text::body("A2")))
    .cell(GridCell::new(Text::body("B2")))
    .cell(GridCell::new(Text::body("C2")))
    .build(&mut ctx);
    
    ctx.set_root(grid_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(600.0, 200.0));
    
    println!("\nGrid layout:");
    debug_tree(&tree, grid_id, 0);
    
    let children = tree.get_children(grid_id);
    assert_eq!(children.len(), 3); // Row1, Spacer, Row2
    
    let row1_id = children[0];
    let row1_children = tree.get_children(row1_id);
    assert_eq!(row1_children.len(), 3);
}

#[test]
fn test_grid_fixed_column() {
    let mut ctx = setup_ctx();
    let grid_id = Grid::new(vec![
        GridColumn::fixed(100.0),
        GridColumn::flex(1.0),
    ])
    .cell(GridCell::new(Text::body("侧栏")))
    .cell(GridCell::new(Text::body("主内容")))
    .build(&mut ctx);
    
    ctx.set_root(grid_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(500.0, 100.0));
    
    let row_id = tree.get_children(grid_id)[0];
    let row_children = tree.get_children(row_id);
    
    let sidebar = tree.get(row_children[0]).layout.unwrap();
    let main = tree.get(row_children[1]).layout.unwrap();
    
    assert!((sidebar.width - 100.0).abs() < 1.0);
    assert!(main.width > 350.0);
}

// ============ 嵌套组件测试 ============

#[test]
fn test_nested_components() {
    let mut ctx = setup_ctx();
    let page_id = VStack::new()
        .spacing(16.0)
        .child(Text::title("用户注册"))
        .child(
            VStack::new()
                .spacing(8.0)
                .child(Text::body("用户名"))
                .child(TextInput::new("请输入用户名"))
        )
        .child(
            VStack::new()
                .spacing(8.0)
                .child(Text::body("密码"))
                .child(TextInput::new("请输入密码"))
        )
        .child(
            HStack::new()
                .spacing(12.0)
                .child(Button::new("注册").variant(ButtonVariant::Primary))
                .child(Button::new("取消").variant(ButtonVariant::Secondary))
        )
        .build(&mut ctx);
    
    ctx.set_root(page_id);
    let mut tree = ctx.build();
    
    println!("\n=== 注册表单布局 ===");
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(400.0, 600.0));
    debug_tree(&tree, page_id, 0);
    
    fn verify_all_layouts(tree: &WidgetTree, node_id: WidgetId) -> usize {
        let node = tree.get(node_id);
        assert!(node.layout.is_some(), "节点缺少布局");
        let mut count = 1;
        for &child_id in &node.children {
            count += verify_all_layouts(tree, child_id);
        }
        count
    }
    
    let total_nodes = verify_all_layouts(&tree, page_id);
    println!("总节点数: {}", total_nodes);
    assert!(total_nodes > 10);
}

// ============ TextInput / TextArea 测试 ============

#[test]
fn test_text_input_with_value() {
    let mut ctx = setup_ctx();
    let input_id = TextInput::new("请输入").value("已有内容").build(&mut ctx);
    ctx.set_root(input_id);
    let tree = ctx.build();
    
    let text_id = tree.get(input_id).children[0];
    let text_node = tree.get(text_id);
    if let WidgetType::Text(ref data) = text_node.widget_type {
        assert_eq!(data.content, "已有内容");
    }
}

#[test]
fn test_text_area_rows() {
    let mut ctx = setup_ctx();
    let area_id = TextArea::new("请输入多行文本").rows(5).build(&mut ctx);
    ctx.set_root(area_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(400.0, 300.0));
    
    let layout = tree.get(area_id).layout.unwrap();
    assert!(layout.height > 50.0);
}

// ============ ZStack 测试 ============

#[test]
fn test_zstack_component() {
    let mut ctx = setup_ctx();
    let stack_id = ZStack::new()
        .child(Text::body("背景"))
        .child(Text::title("前景"))
        .build(&mut ctx);
    
    ctx.set_root(stack_id);
    let mut tree = ctx.build();
    
    let mut engine = LayoutEngine::new(SimpleTextMeasurer);
    engine.calculate_layout(&mut tree, Size::new(300.0, 100.0));
    
    let node = tree.get(stack_id);
    assert!(node.layout.is_some());
    assert_eq!(node.children.len(), 2);
}

// ============ 主题系统测试 ============

#[test]
fn test_theme_styles() {
    let theme = Theme::default();
    
    let btn_style = theme.primary_button_style();
    assert!(btn_style.padding.left > 0.0);
    assert!(btn_style.font_size.is_some());
    
    let title_style = theme.title_text_style();
    let body_style = theme.body_text_style();
    assert!(title_style.font_size.unwrap() > body_style.font_size.unwrap());
}