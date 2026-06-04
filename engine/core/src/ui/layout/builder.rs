use super::geometry::*;
use super::style::Style;
use super::widget::*;

/// Widget树构建器，提供链式API
pub struct WidgetBuilder {
    tree: WidgetTree,
    current_parent: Option<WidgetId>,
}

impl WidgetBuilder {
    pub fn new() -> Self {
        WidgetBuilder {
            tree: WidgetTree::new(),
            current_parent: None,
        }
    }
    
    /// 开始一个Row容器
    pub fn row(&mut self, style: Style) -> WidgetId {
        let id = self.tree.create_node(WidgetType::Row, style);
        self.add_to_parent(id);
        self.current_parent = Some(id);
        id
    }
    
    /// 开始一个Column容器
    pub fn column(&mut self, style: Style) -> WidgetId {
        let id = self.tree.create_node(WidgetType::Column, style);
        self.add_to_parent(id);
        self.current_parent = Some(id);
        id
    }
    
    /// 开始一个Container
    pub fn container(&mut self, style: Style) -> WidgetId {
        let id = self.tree.create_node(WidgetType::Container, style);
        self.add_to_parent(id);
        self.current_parent = Some(id);
        id
    }
    
    /// 添加文本节点
    pub fn text(&mut self, content: &str, font_size: f32, style: Style) -> WidgetId {
        let text_data = TextData::new(content, font_size);
        let id = self.tree.create_node(WidgetType::Text(text_data), style);
        self.add_to_parent(id);
        id
    }
    
    /// 添加空白占位
    pub fn spacer(&mut self, size: Size) -> WidgetId {
        let id = self.tree.create_node(WidgetType::Spacer(size), Style::default());
        self.add_to_parent(id);
        id
    }
    
    /// 结束当前容器，返回父容器
    pub fn end(&mut self) {
        if let Some(current_id) = self.current_parent {
            self.current_parent = self.tree.get(current_id).parent;
        }
    }
    
    /// 构建完成，返回WidgetTree
    pub fn build(self) -> WidgetTree {
        self.tree
    }
    
    /// 添加节点到当前父节点
    fn add_to_parent(&mut self, child_id: WidgetId) {
        if let Some(parent_id) = self.current_parent {
            self.tree.add_child(parent_id, child_id);
        } else {
            // 如果没有父节点，设置为根节点
            self.tree.set_root(child_id);
        }
    }
}