use super::geometry::*;
use super::widget::*;
use super::text::TextMeasurer;
use super::style::MainAlignment;

/// 布局引擎
pub struct LayoutEngine<T: TextMeasurer> {
    text_measurer: T,
}

struct ChildMeasure {
    id: WidgetId,
    size: Size,
    flex_grow: f32,
    flex_shrink: f32,
    margin: EdgeInsets,
    cross_alignment: Alignment,
}

impl<T: TextMeasurer> LayoutEngine<T> {
    pub fn new(text_measurer: T) -> Self {
        LayoutEngine { text_measurer }
    }
    
    /// 计算整个树的布局
    pub fn calculate_layout(&mut self, tree: &mut WidgetTree, available_size: Size) {
        if let Some(root_id) = tree.root() {
            // 根节点使用紧约束，填满窗口
            let constraints = Constraints::tight(available_size);
            self.layout_node(tree, root_id, constraints);
        }
    }
    
    /// 递归布局单个节点
    fn layout_node(
        &mut self,
        tree: &mut WidgetTree,
        node_id: WidgetId,
        constraints: Constraints,
    ) -> Size {
        // 如果节点不脏且有缓存，跳过计算
        if !tree.get(node_id).dirty {
            if let Some(layout) = tree.get(node_id).layout {
                return layout.size();
            }
        }
        
        // 获取节点样式的副本
        let style = tree.get(node_id).style.clone();
        
        // 1. 确定该节点的实际约束
        // 如果节点有固定尺寸，收紧约束
        let mut node_constraints = constraints;
        if let Some(w) = style.width {
            node_constraints.min_width = w;
            node_constraints.max_width = w;
        }
        if let Some(h) = style.height {
            node_constraints.min_height = h;
            node_constraints.max_height = h;
        }
        
        // 2. 减去内边距，得到内容区域约束
        let inner_constraints = node_constraints.deflate(&style.padding);
        
        // 3. 根据节点类型计算内容尺寸
        let content_size = match &tree.get(node_id).widget_type {
            WidgetType::Container => {
                self.layout_container(tree, node_id, inner_constraints)
            }
            WidgetType::Row => {
                self.layout_flex(tree, node_id, Axis::Horizontal, inner_constraints)
            }
            WidgetType::Column => {
                self.layout_flex(tree, node_id, Axis::Vertical, inner_constraints)
            }
            WidgetType::Text(text_data) => {
                self.text_measurer.measure_text(
                    &text_data.content,
                    text_data.font_size,
                    inner_constraints.max_width,
                )
            }
            WidgetType::Spacer(size) => *size,
        };
        
        // 4. 加上内边距
        let padded_size = style.padding.inflate(&content_size);
        
        // 5. 约束最终尺寸
        let mut final_size = padded_size;
        final_size.width = final_size.width.clamp(node_constraints.min_width, node_constraints.max_width);
        final_size.height = final_size.height.clamp(node_constraints.min_height, node_constraints.max_height);
        
        // 6. 存储布局结果
        tree.get_mut(node_id).layout = Some(Rect::new(0.0, 0.0, final_size.width, final_size.height));
        tree.get_mut(node_id).dirty = false;
        
        final_size
    }
    
    fn layout_container(
        &mut self,
        tree: &mut WidgetTree,
        node_id: WidgetId,
        constraints: Constraints,
    ) -> Size {
        let children = tree.get_children(node_id);
        if let Some(&child_id) = children.first() {
            // 给子节点松约束，让它自由决定尺寸
            let loose_constraints = Constraints {
                min_width: 0.0,
                max_width: constraints.max_width,
                min_height: 0.0,
                max_height: constraints.max_height,
            };
            let child_size = self.layout_node(tree, child_id, loose_constraints);
            
            let padding = tree.get(node_id).style.padding;
            let style = tree.get(node_id).style.clone();
            
            let min_w = (child_size.width + padding.left + padding.right)
                .max(style.width.unwrap_or(0.0));
            let min_h = (child_size.height + padding.top + padding.bottom)
                .max(style.height.unwrap_or(0.0));
            
            let container_w = min_w.max(constraints.min_width);
            let container_h = min_h.max(constraints.min_height);
            
            let inner_w = container_w - padding.left - padding.right;
            let inner_h = container_h - padding.top - padding.bottom;
            
            let x = padding.left + (inner_w - child_size.width).max(0.0) / 2.0;
            let y = padding.top + (inner_h - child_size.height).max(0.0) / 2.0;
            
            println!("DEBUG container inner: inner_w={:.1}, child_w={:.1}, x_offset={:.1}, container_w={:.1}", 
                inner_w, child_size.width, (inner_w - child_size.width).max(0.0) / 2.0, container_w);
            
            if let Some(child_layout) = &mut tree.get_mut(child_id).layout {
                child_layout.x = x;
                child_layout.y = y;
            }
            
            Size::new(container_w, container_h)
        } else {
            Size::default()
        }
    }
    
    /// 布局Flex容器（Row或Column）
    fn layout_flex(
        &mut self,
        tree: &mut WidgetTree,
        node_id: WidgetId,
        axis: Axis,
        constraints: Constraints,
    ) -> Size {
        let children = tree.get_children(node_id);
        
        if children.is_empty() {
            return Size::default();
        }
        
        // === 第一阶段：测量所有子节点 ===
        
        let mut measures = Vec::new();
        let mut total_main_size: f32 = 0.0;
        let mut total_flex_grow: f32 = 0.0;
        let mut total_flex_shrink: f32 = 0.0;
        let mut max_cross_size: f32 = 0.0;
        
        for &child_id in &children {
            let mut child_style = tree.get(child_id).style.clone();
            // 如果子节点是默认的Stretch，继承父节点的cross_alignment
            if child_style.cross_alignment == Alignment::Stretch {
                child_style.cross_alignment = tree.get(node_id).style.cross_alignment;
            }
            
            // 创建子节点约束
            let child_constraints = match axis {
                Axis::Horizontal => {
                    let cross_min = if child_style.cross_alignment != Alignment::Stretch {
                        0.0
                    } else {
                        constraints.min_height
                    };
                    Constraints {
                        min_width: 0.0,
                        max_width: f32::INFINITY,
                        min_height: cross_min,
                        max_height: constraints.max_height,
                    }
                },
                Axis::Vertical => {
                    let cross_min = if child_style.cross_alignment != Alignment::Stretch {
                        0.0
                    } else {
                        constraints.min_width
                    };
                    Constraints {
                        min_width: cross_min,
                        max_width: constraints.max_width,
                        min_height: 0.0,
                        max_height: f32::INFINITY,
                    }
                },
            };
            
            let child_size = self.layout_node(tree, child_id, child_constraints);
            
            let main_size = match axis {
                Axis::Horizontal => child_size.width + child_style.margin.left + child_style.margin.right,
                Axis::Vertical => child_size.height + child_style.margin.top + child_style.margin.bottom,
            };
            
            let cross_size = match axis {
                Axis::Horizontal => child_size.height + child_style.margin.top + child_style.margin.bottom,
                Axis::Vertical => child_size.width + child_style.margin.left + child_style.margin.right,
            };
            
            // 固定尺寸的节点不参与弹性收缩（但可以参与增长）
            let has_fixed_main_size = match axis {
                Axis::Horizontal => child_style.width.is_some(),
                Axis::Vertical => child_style.height.is_some(),
            };
            
            let effective_flex_shrink = if has_fixed_main_size {
                0.0
            } else {
                child_style.flex_shrink
            };
            
            total_main_size += main_size;
            total_flex_grow += child_style.flex_grow;
            total_flex_shrink += effective_flex_shrink * main_size;
            max_cross_size = f32::max(max_cross_size, cross_size);
            
            measures.push(ChildMeasure {
                id: child_id,
                size: child_size,
                flex_grow: child_style.flex_grow,
                flex_shrink: effective_flex_shrink,
                margin: child_style.margin,
                cross_alignment: child_style.cross_alignment,
            });
        }
        
        // === 第二阶段：分配空间 ===
        let available_main = match axis {
            Axis::Horizontal => constraints.max_width,
            Axis::Vertical => constraints.max_height,
        };
        
        let free_space = available_main - total_main_size;
        
        // 分配弹性空间（扩展）
        if free_space > 0.0 && total_flex_grow > 0.0 {
            for measure in &mut measures {
                if measure.flex_grow > 0.0 {
                    let extra = free_space * (measure.flex_grow / total_flex_grow);
                    match axis {
                        Axis::Horizontal => measure.size.width += extra,
                        Axis::Vertical => measure.size.height += extra,
                    }
                }
            }
        } 
        // 分配弹性空间（收缩）
        else if free_space < 0.0 && total_flex_shrink > 0.0 {
            let deficit = -free_space;
            for measure in &mut measures {
                if measure.flex_shrink > 0.0 {
                    let shrink = deficit * (measure.flex_shrink * match axis {
                        Axis::Horizontal => measure.size.width,
                        Axis::Vertical => measure.size.height,
                    } / total_flex_shrink);
                    match axis {
                        Axis::Horizontal => measure.size.width = (measure.size.width - shrink).max(0.0),
                        Axis::Vertical => measure.size.height = (measure.size.height - shrink).max(0.0),
                    }
                }
            }
        }

        // 确保交叉轴尺寸至少是容器的最小高度/宽度
        let final_cross_size = match axis {
            Axis::Horizontal => max_cross_size.max(constraints.min_height),
            Axis::Vertical => max_cross_size.max(constraints.min_width),
        };
        
        // === 第三阶段：定位子节点 ===
        let main_alignment = tree.get(node_id).style.main_alignment;
        let parent_padding = tree.get(node_id).style.padding;
        
        // 计算实际的交叉轴可用空间
        let actual_cross_size = match axis {
            Axis::Horizontal => final_cross_size.max(constraints.min_height),
            Axis::Vertical => final_cross_size.max(constraints.min_width),
        };

        // 计算起始偏移
        let mut main_offset = self.calculate_main_start_offset(
            free_space,
            total_flex_grow,
            main_alignment,
            &measures,
            axis,
        );
        
        // 加上padding偏移
        main_offset += match axis {
            Axis::Horizontal => parent_padding.left,
            Axis::Vertical => parent_padding.top,
        };

        // 计算 Space 对齐的间隙
        let space_gap = if free_space > 0.0 && total_flex_grow == 0.0 {
            match main_alignment {
                MainAlignment::SpaceBetween if measures.len() > 1 => {
                    Some(free_space / (measures.len() - 1) as f32)
                }
                MainAlignment::SpaceAround if !measures.is_empty() => {
                    let gap = free_space / (measures.len() * 2) as f32;
                    main_offset += gap;
                    Some(gap * 2.0)
                }
                MainAlignment::SpaceEvenly if !measures.is_empty() => {
                    let gap = free_space / (measures.len() + 1) as f32;
                    main_offset += gap;
                    Some(gap)
                }
                _ => None,
            }
        } else {
            None
        };
        
        for measure in &measures {
            let child_id = measure.id;
            let mut child_rect = Rect::new(0.0, 0.0, measure.size.width, measure.size.height);
            
            match axis {
                Axis::Horizontal => {
                    child_rect.x = main_offset + measure.margin.left;
                    main_offset += measure.size.width + measure.margin.left + measure.margin.right;
                    
                    let available_cross = actual_cross_size - parent_padding.top - parent_padding.bottom;
                    child_rect.y = parent_padding.top + self.align_cross(
                        measure.cross_alignment,
                        available_cross,
                        measure.size.height,
                        measure.margin,
                    );
                }
                Axis::Vertical => {
                    child_rect.y = main_offset + measure.margin.top;
                    main_offset += measure.size.height + measure.margin.top + measure.margin.bottom;
                    
                    let available_cross = actual_cross_size - parent_padding.left - parent_padding.right;
                    child_rect.x = parent_padding.left + self.align_cross(
                        measure.cross_alignment,
                        available_cross,
                        measure.size.width,
                        measure.margin,
                    );
                }
            }
            
            tree.get_mut(child_id).layout = Some(child_rect);

            // 添加 Space 对齐间隙
            if let Some(gap) = space_gap {
                match axis {
                    Axis::Horizontal => main_offset += gap,
                    Axis::Vertical => main_offset += gap,
                }
            }
        }
        
        // 重新计算分配弹性空间后的实际主轴总尺寸
        let actual_total_main: f32 = measures.iter().map(|m| {
            match axis {
                Axis::Horizontal => m.size.width + m.margin.left + m.margin.right,
                Axis::Vertical => m.size.height + m.margin.top + m.margin.bottom,
            }
        }).sum();
        
        // 返回考虑了固定尺寸和约束的最终尺寸
        match axis {
            Axis::Horizontal => Size::new(
                actual_total_main.max(constraints.min_width),
                actual_cross_size,
            ),
            Axis::Vertical => Size::new(
                actual_cross_size,
                actual_total_main.max(constraints.min_height),
            ),
        }
    }
    
    /// 计算主轴起始偏移
    fn calculate_main_start_offset(
        &self,
        free_space: f32,
        total_flex_grow: f32,
        alignment: MainAlignment,
        _measures: &[ChildMeasure],
        _axis: Axis,
    ) -> f32 {
        if total_flex_grow > 0.0 && free_space > 0.0 {
            return 0.0;
        }
        
        match alignment {
            MainAlignment::Start => 0.0,
            MainAlignment::Center => free_space.max(0.0) / 2.0,
            MainAlignment::End => free_space.max(0.0),
            MainAlignment::SpaceBetween => 0.0,
            MainAlignment::SpaceAround => 0.0,
            MainAlignment::SpaceEvenly => 0.0,
        }
    }
    
    /// 交叉轴对齐
    fn align_cross(
        &self,
        alignment: Alignment,
        available_space: f32,
        item_size: f32,
        margin: EdgeInsets,
    ) -> f32 {
        let total_item_size = item_size + margin.top + margin.bottom;
        
        match alignment {
            Alignment::Start => margin.top,
            Alignment::Center => margin.top + (available_space - total_item_size) / 2.0,
            Alignment::End => margin.top + available_space - total_item_size,
            Alignment::Stretch => margin.top,
        }
    }
}