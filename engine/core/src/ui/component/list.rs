use super::*;
use crate::ui::event::types::UIEvent;
use crate::ui::event::mouse::MouseEventType;

pub struct ListItem {
    content: Box<dyn Component>,
    on_select: Option<Box<dyn Fn()>>,
}

impl ListItem {
    pub fn new(content: impl Component + 'static) -> Self {
        ListItem {
            content: Box::new(content),
            on_select: None,
        }
    }
    
    pub fn on_select(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }
}

pub struct List {
    items: Vec<ListItem>,
    style: Option<Style>,
}

impl List {
    pub fn new() -> Self {
        List {
            items: Vec::new(),
            style: None,
        }
    }
    
    pub fn item(mut self, item: ListItem) -> Self {
        self.items.push(item);
        self
    }
    
    pub fn items(mut self, items: Vec<ListItem>) -> Self {
        self.items = items;
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl Component for List {
    fn build(&self, ctx: &mut BuildContext) -> WidgetId {
        let style = self.style.clone().unwrap_or_else(|| Style::new().width(300.0));
        let list_id = ctx.create_node(WidgetType::Column, style);
        
        for (_index, item) in self.items.iter().enumerate() {
            let item_style = ctx.theme.list_item_style();
            let item_container = ctx.create_node(WidgetType::Container, item_style);
            
            let content_id = item.content.build(ctx);
            ctx.add_child(item_container, content_id);
            ctx.add_child(list_id, item_container);
            
            // 注册选择事件
            if item.on_select.is_some() {
                ctx.on_event(item_container, move |event| {
                    if let UIEvent::Mouse(mouse_event) = event {
                        if mouse_event.event_type == MouseEventType::Clicked {
                            // 运行时处理
                        }
                    }
                });
            }
        }
        
        list_id
    }
}