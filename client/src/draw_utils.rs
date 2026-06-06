use rhi::DrawCommand;
use core::ui::layout::widget::{WidgetTree, WidgetId, WidgetType};

pub fn build_draw_commands(tree: &WidgetTree, node_id: WidgetId, commands: &mut Vec<DrawCommand>) {
    build_draw_commands_impl(tree, node_id, 0.0, 0.0, commands);
}

fn build_draw_commands_impl(tree: &WidgetTree, node_id: WidgetId, parent_x: f32, parent_y: f32, commands: &mut Vec<DrawCommand>) {
    let node = tree.get(node_id);
    let layout = if let Some(l) = node.layout { l } else { return };
    
    let abs_x = parent_x + layout.x;
    let abs_y = parent_y + layout.y;
    
    match &node.widget_type {
        WidgetType::Container | WidgetType::Row | WidgetType::Column => {
            // 背景色
            if let Some(bg) = node.style.background_color {
                commands.push(DrawCommand::rect(
                    abs_x, abs_y, layout.width, layout.height,
                    bg.0 as f32 / 255.0, bg.1 as f32 / 255.0, bg.2 as f32 / 255.0, 1.0,
                ));
            }
            
            // debug版本画红绿框
            #[cfg(debug_assertions)]
            {
                let margin = node.style.margin;
                let outer_x = abs_x - margin.left;
                let outer_y = abs_y - margin.top;
                let outer_w = layout.width + margin.left + margin.right;
                let outer_h = layout.height + margin.top + margin.bottom;
                if outer_w > 0.0 && outer_h > 0.0 {
                    commands.push(DrawCommand::rect_line(outer_x, outer_y, outer_w, outer_h, 0.0, 1.0, 0.0, 1.0));
                }
                
                let padding = node.style.padding;
                let inner_x = abs_x + padding.left;
                let inner_y = abs_y + padding.top;
                let inner_w = layout.width - padding.left - padding.right;
                let inner_h = layout.height - padding.top - padding.bottom;
                if inner_w > 0.0 && inner_h > 0.0 {
                    commands.push(DrawCommand::rect_line(inner_x, inner_y, inner_w, inner_h, 1.0, 0.0, 0.0, 1.0));
                }
            }
        }
        WidgetType::Text(data) => {
            let font_size = data.font_size;
            let char_width = font_size * 1.0;
            for i in 0..data.content.len() {
                let x = abs_x + i as f32 * char_width;
                let y = abs_y;
                commands.push(DrawCommand::rect(
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