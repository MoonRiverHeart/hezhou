use super::*;
use crate::ui::layout::geometry::{Size, EdgeInsets};

pub struct GridColumn {
    pub flex: f32,
    pub fixed_width: Option<f32>,
}

impl GridColumn {
    pub fn flex(flex: f32) -> Self {
        GridColumn { flex, fixed_width: None }
    }
    
    pub fn fixed(width: f32) -> Self {
        GridColumn { flex: 0.0, fixed_width: Some(width) }
    }
}

pub struct GridCell {
    content: Box<dyn Component>,
    column_span: u32,
}

impl GridCell {
    pub fn new(content: impl Component + 'static) -> Self {
        GridCell { content: Box::new(content), column_span: 1 }
    }
    
    pub fn column_span(mut self, span: u32) -> Self {
        self.column_span = span;
        self
    }
}

pub struct Grid {
    columns: Vec<GridColumn>,
    cells: Vec<GridCell>,
    row_gap: f32,
    column_gap: f32,
    style: Option<Style>,
}

impl Grid {
    pub fn new(columns: Vec<GridColumn>) -> Self {
        Grid {
            columns,
            cells: Vec::new(),
            row_gap: 8.0,
            column_gap: 8.0,
            style: None,
        }
    }
    
    pub fn cell(mut self, cell: GridCell) -> Self {
        self.cells.push(cell);
        self
    }
    
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = gap;
        self
    }
    
    pub fn column_gap(mut self, gap: f32) -> Self {
        self.column_gap = gap;
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl Component for Grid {
    fn build(&self, ctx: &mut BuildContext) -> WidgetId {
        let style = self.style.clone().unwrap_or_default();
        let col_count = self.columns.len();
        let grid_id = ctx.create_node(WidgetType::Column, style);
        
        let mut cell_index = 0;
        while cell_index < self.cells.len() {
            let row_id = ctx.create_node(WidgetType::Row, Style::new().flex_grow(1.0));
            
            // 行间距
            if cell_index >= col_count {
                let spacer_id = ctx.create_node(
                    WidgetType::Spacer(Size::new(0.0, self.row_gap)),
                    Style::default()
                );
                ctx.add_child(grid_id, spacer_id);
            }
            
            for col in 0..col_count {
                if cell_index >= self.cells.len() {
                    break;
                }
                
                let cell = &self.cells[cell_index];
                let col_def = &self.columns[col];
                
                let mut cell_style = Style::new().flex_grow(col_def.flex);
                if let Some(w) = col_def.fixed_width {
                    cell_style = cell_style.width(w);
                }
                if col > 0 {
                    cell_style = cell_style.margin(EdgeInsets {
                        left: self.column_gap / 2.0,
                        right: self.column_gap / 2.0,
                        top: 0.0,
                        bottom: 0.0,
                    });
                }
                
                let cell_container = ctx.create_node(WidgetType::Container, cell_style);
                let content_id = cell.content.build(ctx);
                ctx.add_child(cell_container, content_id);
                ctx.add_child(row_id, cell_container);
                
                cell_index += 1;
            }
            
            ctx.add_child(grid_id, row_id);
        }
        
        grid_id
    }
}