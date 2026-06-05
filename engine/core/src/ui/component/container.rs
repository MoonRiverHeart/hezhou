use super::*;
use crate::ui::layout::geometry::{Size, Alignment};
use crate::ui::layout::style::MainAlignment;

pub struct VStack {
    children: Vec<Box<dyn Component>>,
    spacing: f32,
    alignment: Alignment,
    style: Option<Style>,
}

impl VStack {
    pub fn new() -> Self {
        VStack {
            children: Vec::new(),
            spacing: 0.0,
            alignment: Alignment::Stretch,
            style: None,
        }
    }
    
    pub fn child(mut self, component: impl Component + 'static) -> Self {
        self.children.push(Box::new(component));
        self
    }
    
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
    
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl Component for VStack {
    fn build(&self, ctx: &mut BuildContext) -> WidgetId {
        let style = self.style.clone().unwrap_or(
            Style::new().cross_alignment(Alignment::Center)
        );
        let column_id = ctx.create_node(WidgetType::Column, style);
        ctx.tree.get_mut(column_id).style.main_alignment = MainAlignment::Center;
        
        for (i, child) in self.children.iter().enumerate() {
            if i > 0 && self.spacing > 0.0 {
                let spacer_id = ctx.create_node(
                    WidgetType::Spacer(Size::new(0.0, self.spacing)),
                    Style::default()
                );
                ctx.add_child(column_id, spacer_id);
            }
            
            let child_id = child.build(ctx);
            ctx.add_child(column_id, child_id);
        }
        
        column_id
    }
}

pub struct HStack {
    children: Vec<Box<dyn Component>>,
    spacing: f32,
    alignment: Alignment,
    style: Option<Style>,
}

impl HStack {
    pub fn new() -> Self {
        HStack {
            children: Vec::new(),
            spacing: 0.0,
            alignment: Alignment::Center,
            style: None,
        }
    }
    
    pub fn child(mut self, component: impl Component + 'static) -> Self {
        self.children.push(Box::new(component));
        self
    }
    
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl Component for HStack {
    fn build(&self, ctx: &mut BuildContext) -> WidgetId {
        let style = self.style.clone().unwrap_or_default();
        let row_id = ctx.create_node(WidgetType::Row, style);
        
        for (i, child) in self.children.iter().enumerate() {
            if i > 0 && self.spacing > 0.0 {
                let spacer_id = ctx.create_node(
                    WidgetType::Spacer(Size::new(self.spacing, 0.0)),
                    Style::default()
                );
                ctx.add_child(row_id, spacer_id);
            }
            
            let child_id = child.build(ctx);
            ctx.add_child(row_id, child_id);
        }
        
        row_id
    }
}

pub struct ZStack {
    children: Vec<Box<dyn Component>>,
    style: Option<Style>,
}

impl ZStack {
    pub fn new() -> Self {
        ZStack {
            children: Vec::new(),
            style: None,
        }
    }
    
    pub fn child(mut self, component: impl Component + 'static) -> Self {
        self.children.push(Box::new(component));
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl Component for ZStack {
    fn build(&self, ctx: &mut BuildContext) -> WidgetId {
        let style = self.style.clone().unwrap_or_default();
        let container_id = ctx.create_node(WidgetType::Container, style);
        
        for child in &self.children {
            let child_id = child.build(ctx);
            ctx.add_child(container_id, child_id);
        }
        
        container_id
    }
}