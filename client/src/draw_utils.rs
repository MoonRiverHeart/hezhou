use rhi::DrawCommand;
use rhi::Vertex;
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
            if let Some(bg) = node.style.background_color {
                let radius = layout.height / 2.0;
                commands.push(DrawCommand::rect(
                    abs_x, abs_y, layout.width, layout.height,
                    bg.0 as f32 / 255.0, bg.1 as f32 / 255.0, bg.2 as f32 / 255.0, 1.0,
                    radius,
                ));
            }
            
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
            if let Some(ref glyphs) = data.glyphs {
                let mut cursor_x = abs_x;
                let max_height = glyphs.iter()
                    .map(|g| g.size.height)
                    .fold(0.0f32, f32::max);
                
                // 用第一个字形的比例作为统一缩放
                let first_scale = if let Some(first) = glyphs.first() {
                    (first.advance_x / first.size.width.max(1.0)).clamp(0.7, 1.0)
                } else {
                    1.0
                };

                // 用第一个字形的竖直中点作为基线
                let first_mid_y = if let Some(first) = glyphs.first() {
                    let scale = (first.advance_x / first.size.width.max(1.0)).clamp(0.5, 1.0);
                    let gh = first.size.height * scale;
                    abs_y + max_height - first.size.height + (first.size.height - gh) / 2.0 + gh / 2.0
                } else {
                    abs_y + max_height / 2.0
                };
                
                for glyph in glyphs {
                    let scale = glyph.advance_x / glyph.size.width.max(1.0);
                    let scale = scale.clamp(0.5, 1.0);
                    let gw = glyph.size.width * scale;
                    let gh = glyph.size.height * scale;
                    let offset_y = (glyph.size.height - gh) / 2.0;
                    
                    let gx = cursor_x + glyph.bearing_x;
                    // let gy = abs_y + max_height - glyph.size.height + offset_y;
                    let gy = first_mid_y - gh / 2.0;
                    // let gy = abs_y + max_height - gh;
                    let uv = glyph.uv;
                    
                    commands.push(DrawCommand {
                        vertices: vec![
                            Vertex { position: [gx, gy], color: [0.0, 0.0, 0.0, 1.0], uv: [uv[0], uv[3]], border_radius: [0.0; 4] },
                            Vertex { position: [gx + gw, gy], color: [0.0, 0.0, 0.0, 1.0], uv: [uv[2], uv[3]], border_radius: [0.0; 4] },
                            Vertex { position: [gx + gw, gy + gh], color: [0.0, 0.0, 0.0, 1.0], uv: [uv[2], uv[1]], border_radius: [0.0; 4] },
                            Vertex { position: [gx, gy + gh], color: [0.0, 0.0, 0.0, 1.0], uv: [uv[0], uv[1]], border_radius: [0.0; 4] },
                        ],
                        indices: Some(vec![0, 1, 2, 2, 3, 0]),
                        clip_rect: None,
                        texture_id: 1,
                    });
                    
                    cursor_x += glyph.advance_x;
                }
            }
        }
        _ => {}
    }
    
    for &child_id in &node.children {
        build_draw_commands_impl(tree, child_id, abs_x, abs_y, commands);
    }
}

pub fn hit_test(tree: &WidgetTree, node_id: WidgetId, px: f32, py: f32, parent_x: f32, parent_y: f32) -> Option<WidgetId> {
    let node = tree.get(node_id);
    let layout = node.layout?;
    
    let abs_x = parent_x + layout.x;
    let abs_y = parent_y + layout.y;
    
    let hit = px >= abs_x && px <= abs_x + layout.width 
           && py >= abs_y && py <= abs_y + layout.height;
    
    if !hit {
        return None;
    }
    
    for &child_id in node.children.iter().rev() {
        if let Some(hit_id) = hit_test(tree, child_id, px, py, abs_x, abs_y) {
            return Some(hit_id);
        }
    }
    
    Some(node_id)
}