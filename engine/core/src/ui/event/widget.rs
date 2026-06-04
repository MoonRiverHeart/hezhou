use std::time::SystemTime;
use super::mouse::Point;

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetEvent {
    pub timestamp: SystemTime,
    pub widget_id: String,
    pub event_type: WidgetEventType,
    pub handled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetEventType {
    /// 按钮点击
    ButtonClick,
    /// 复选框状态改变
    CheckboxToggle { checked: bool },
    /// 单选按钮选择
    RadioSelect { value: String },
    /// 滑块值改变
    SliderChanged { value: f64 },
    /// 文本输入改变
    TextChanged { text: String },
    /// 下拉菜单选择
    DropdownSelected { index: usize, value: String },
    /// 列表选择
    ListSelected { indices: Vec<usize> },
    /// 标签页切换
    TabChanged { tab_index: usize },
    /// 树节点展开/折叠
    TreeNodeToggled { node_id: String, expanded: bool },
    /// 上下文菜单请求（右键菜单）
    ContextMenuRequested { position: Point },
    /// 获得焦点
    FocusGained,
    /// 失去焦点
    FocusLost,
    /// 悬停进入
    HoverEntered,
    /// 悬停离开
    HoverExited,
}