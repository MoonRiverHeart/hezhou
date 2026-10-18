use hezhou_ui::widget::Widget;
use hezhou_ui::widgets::{Button, Panel};
use hezhou_ui::{Layout, Point, WidgetTree};

#[test]
fn hit_test_returns_child_button_and_none_outside() {
    let mut tree = WidgetTree::new();

    let mut root = Box::new(Panel::new());
    root.set_layout(Layout::new(0.0, 0.0, 800.0, 600.0));
    let root_id = root.id();
    tree.set_root(root);

    let mut button = Box::new(Button::new("Test"));
    let button_id = button.id();
    button.set_layout(Layout::new(20.0, 20.0, 120.0, 40.0));
    tree.add_widget(button, root_id);

    assert_eq!(
        tree.hit_test(Point::new(30.0, 30.0)).map(|id| id.id),
        Some(button_id.id)
    );
    assert_eq!(tree.hit_test(Point::new(5.0, 5.0)).map(|id| id.id), Some(root_id.id));
    assert_eq!(tree.hit_test(Point::new(900.0, 700.0)), None);
}
