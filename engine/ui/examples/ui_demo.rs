use hezhou_ui::widgets::*;
use hezhou_ui::*;

fn main() {
    println!("=== UI System Demo ===");

    let mut ui_system = UISystem::new();
    let mut tree = ui_system.get_widget_tree();

    let panel = Box::new(Panel::new().with_layout_type(LayoutType::Absolute));

    tree.lock().set_root(panel);

    let panel_id = tree.lock().root.unwrap();

    let button1 = Box::new(Button::new("Click Me").with_on_click(Box::new(|| {
        println!("Button clicked!");
    })));

    let button1_id = button1.id();
    tree.lock().add_widget(button1, panel_id);

    if let Some(btn) = tree.lock().get_widget_mut(button1_id) {
        btn.as_mut()
            .set_layout(Layout::new(20.0, 20.0, 120.0, 40.0));
    }

    let label = Box::new(Label::new("Hello UI!"));
    let label_id = label.id();
    tree.lock().add_widget(label, panel_id);

    if let Some(lbl) = tree.lock().get_widget_mut(label_id) {
        lbl.as_mut()
            .set_layout(Layout::new(20.0, 80.0, 200.0, 30.0));
    }

    println!("Widget tree created:");
    println!("  Panel (root): {:?}", panel_id);
    println!("  Button: {:?}", button1_id);
    println!("  Label: {:?}", label_id);

    let mut touch_event = Event::new(EventType::TouchBegin, 0)
        .with_target(button1_id)
        .with_data(EventData::Touch(TouchData::new(30.0, 30.0, 0)));

    println!("\nSimulating touch event on button...");
    ui_system
        .get_event_dispatcher()
        .lock()
        .dispatch_event(&mut touch_event);

    let render_data = tree.lock().generate_render_data();
    println!(
        "\nRender data generated: {} draw commands",
        render_data.len()
    );

    for (i, data) in render_data.iter().enumerate() {
        println!(
            "  Widget {}: {} commands at ({}, {})",
            i,
            data.draw_commands.len(),
            data.bounds.x,
            data.bounds.y
        );
    }

    println!("\n=== Demo Complete ===");
}
