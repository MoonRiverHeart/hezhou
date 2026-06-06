use slotmap::{SlotMap, new_key_type};
use super::geometry::{Size, Rect, Axis};
use super::style::Style;
use super::text::GlyphInfo;

new_key_type! {
    /// Widget节点的安全ID
    pub struct WidgetId;
}

/// Widget节点类型
pub enum WidgetType {
    /// 普通容器（单个子节点）
    Container,
    /// 水平排列
    Row,
    /// 垂直排列
    Column,
    /// 叶子节点：文本
    Text(TextData),
    /// 叶子节点：空白占位
    Spacer(Size),
}

/// 文本数据
pub struct TextData {
    pub content: String,
    pub font_size: f32,
    pub glyphs: Option<Vec<GlyphInfo>>,  // 新增：字形信息
}

impl TextData {
    pub fn new(content: impl Into<String>, font_size: f32) -> Self {
        TextData {
            content: content.into(),
            font_size,
            glyphs: None,
        }
    }
    
    pub fn with_glyphs(content: impl Into<String>, font_size: f32, glyphs: Vec<GlyphInfo>) -> Self {
        TextData {
            content: content.into(),
            font_size,
            glyphs: Some(glyphs),
        }
    }
}

/// Widget节点
pub struct WidgetNode {
    /// 节点ID
    pub id: WidgetId,
    /// 节点类型
    pub widget_type: WidgetType,
    /// 父节点ID
    pub parent: Option<WidgetId>,
    /// 子节点ID列表
    pub children: Vec<WidgetId>,
    /// 样式
    pub style: Style,
    /// 计算结果：布局位置和尺寸
    pub layout: Option<Rect>,
    /// 脏标记：需要重新计算
    pub dirty: bool,
}

impl WidgetNode {
    fn new(id: WidgetId, widget_type: WidgetType, style: Style) -> Self {
        WidgetNode {
            id,
            widget_type,
            parent: None,
            children: Vec::new(),
            style,
            layout: None,
            dirty: true,
        }
    }
    
    /// 是否为弹性容器（Row或Column）
    pub fn is_flex_container(&self) -> bool {
        matches!(self.widget_type, WidgetType::Row | WidgetType::Column)
    }
    
    /// 获取主轴方向
    pub fn main_axis(&self) -> Option<Axis> {
        match self.widget_type {
            WidgetType::Row => Some(Axis::Horizontal),
            WidgetType::Column => Some(Axis::Vertical),
            _ => None,
        }
    }
}

/// Widget树管理器
pub struct WidgetTree {
    nodes: SlotMap<WidgetId, WidgetNode>,
    root: Option<WidgetId>,
}

impl WidgetTree {
    pub fn new() -> Self {
        WidgetTree {
            nodes: SlotMap::with_key(),
            root: None,
        }
    }
    
    /// 创建节点
    pub fn create_node(&mut self, widget_type: WidgetType, style: Style) -> WidgetId {
        let id = self.nodes.insert_with_key(|id| WidgetNode::new(id, widget_type, style));
        id
    }
    
    /// 添加子节点
    pub fn add_child(&mut self, parent_id: WidgetId, child_id: WidgetId) {
        self.nodes[child_id].parent = Some(parent_id);
        self.nodes[parent_id].children.push(child_id);
        self.mark_dirty(parent_id);
    }
    
    /// 设置根节点
    pub fn set_root(&mut self, node_id: WidgetId) {
        self.root = Some(node_id);
        self.mark_dirty(node_id);
    }
    
    /// 获取节点引用
    pub fn get(&self, id: WidgetId) -> &WidgetNode {
        &self.nodes[id]
    }
    
    /// 获取节点可变引用
    pub fn get_mut(&mut self, id: WidgetId) -> &mut WidgetNode {
        &mut self.nodes[id]
    }
    
    /// 标记节点及其祖先为脏
    pub fn mark_dirty(&mut self, id: WidgetId) {
        let mut current = Some(id);
        while let Some(node_id) = current {
            let node = &mut self.nodes[node_id];
            node.dirty = true;
            current = node.parent;
        }
    }
    
    /// 获取根节点
    pub fn root(&self) -> Option<WidgetId> {
        self.root
    }
    
    /// 获取所有节点
    pub fn nodes(&self) -> &SlotMap<WidgetId, WidgetNode> {
        &self.nodes
    }
    
    /// 获取子节点列表的副本（用于遍历）
    pub fn get_children(&self, id: WidgetId) -> Vec<WidgetId> {
        self.nodes[id].children.clone()
    }
}