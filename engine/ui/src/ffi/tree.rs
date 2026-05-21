use crate::*;
use hezhou_dfx::*;
use parking_lot::Mutex;
use std::ffi::CStr;
use std::sync::Arc;

use super::WidgetTreeHandle;

#[unsafe(no_mangle)]
pub extern "C" fn ui_create_tree_view(
    handle: WidgetTreeHandle,
    parent_id: u64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let mut tree_view = crate::widgets::TreeView::new()
            .with_layout(x, y, width, height)
            .with_content_scale(content_scale);
        
        let id = tree_view.id();
        
        let parent = if parent_id == 0 {
            tree.root.unwrap_or(WidgetId::invalid())
        } else {
            WidgetId::from_raw(parent_id)
        };
        
        tree.add_widget(Box::new(tree_view), parent);
        dfx_info!("FFI", "CreateTreeView: id={}, parent={}, scale={}", id.id, parent_id, content_scale);
        id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_add_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    parent_node_id: u64,
    text: *const std::ffi::c_char,
    user_data: u64,
    has_children: bool,
) -> u64 {
    if handle.is_null() || text.is_null() {
        return 0;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        
        let content_scale = crate::thunk::ui_get_content_scale();
        let mut node = crate::widgets::TreeNode::new(&text_str)
            .with_content_scale(content_scale)
            .with_user_data(user_data)
            .with_has_children(has_children);
        
        let node_id = node.id();
        
        if parent_node_id == 0 {
            node.set_depth(0);
            if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
                if widget.widget_type() == "TreeView" {
                    use crate::widgets::TreeView;
                    if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                        tree_view.add_root_node(node_id);
                    }
                }
            }
        } else {
            let parent_id = WidgetId::from_raw(parent_node_id);
            if let Some(widget) = tree.get_widget(parent_id) {
                if widget.widget_type() == "TreeNode" {
                    use crate::widgets::TreeNode;
                    if let Some(parent_node) = widget.as_any().downcast_ref::<TreeNode>() {
                        node.set_depth(parent_node.depth() + 1);
                        node.set_parent(parent_id);
                    }
                }
            }
            if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
                if widget.widget_type() == "TreeView" {
                    use crate::widgets::TreeView;
                    if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                        tree_view.add_child_node(parent_id, node_id);
                    }
                }
            }
        }
        
        tree.add_widget(Box::new(node), tree_view_widget_id);
        dfx_info!("FFI", "TreeViewAddNode: node_id={}, parent={}, text={}, user_data={}, has_children={}", 
            node_id.id, parent_node_id, text_str, user_data, has_children);
        node_id.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_remove_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                    tree_view.remove_node(node_widget_id);
                }
            }
        }
        
        tree.remove_widget(node_widget_id);
        dfx_info!("FFI", "TreeViewRemoveNode: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_set_selected(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                    tree_view.set_selected(node_widget_id);
                }
            }
        }
        
        if let Some(widget) = tree.get_widget_mut(node_widget_id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_selected(true);
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewSetSelected: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_get_selected(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        
        if let Some(widget) = tree.get_widget(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any().downcast_ref::<TreeView>() {
                    return tree_view.selected_node().map(|id| id.id).unwrap_or(0);
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_expand_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(node_widget_id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_expanded(true);
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewExpandNode: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_collapse_node(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    node_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let node_widget_id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(node_widget_id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_expanded(false);
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewCollapseNode: tree_view_id={}, node_id={}", tree_view_id, node_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_set_on_select_thunk_ptr(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
    callback_ptr: *const std::ffi::c_void,
) {
    if callback_ptr.is_null() {
        return;
    }
    let callback: crate::thunk::TreeNodeSelectCallback = unsafe { std::mem::transmute(callback_ptr) };
    crate::thunk::ui_register_tree_node_select_callback(tree_view_id, callback);
    dfx_info!("FFI", "TreeViewSetOnSelectThunkPtr: tree_view_id={}", tree_view_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_node_set_text(
    handle: WidgetTreeHandle,
    node_id: u64,
    text: *const std::ffi::c_char,
) {
    if handle.is_null() || text.is_null() {
        return;
    }
    unsafe {
        let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        let id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget_mut(id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any_mut().downcast_mut::<TreeNode>() {
                    node.set_text(&text_str);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_node_get_user_data(
    handle: WidgetTreeHandle,
    node_id: u64,
) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let tree = arc.lock();
        let id = WidgetId::from_raw(node_id);
        
        if let Some(widget) = tree.get_widget(id) {
            if widget.widget_type() == "TreeNode" {
                use crate::widgets::TreeNode;
                if let Some(node) = widget.as_any().downcast_ref::<TreeNode>() {
                    return node.user_data();
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_tree_view_clear_selection(
    handle: WidgetTreeHandle,
    tree_view_id: u64,
) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let arc = &*(handle as *const Arc<Mutex<WidgetTree>>);
        let mut tree = arc.lock();
        
        let tree_view_widget_id = WidgetId::from_raw(tree_view_id);
        
        if let Some(widget) = tree.get_widget_mut(tree_view_widget_id) {
            if widget.widget_type() == "TreeView" {
                use crate::widgets::TreeView;
                if let Some(tree_view) = widget.as_any_mut().downcast_mut::<TreeView>() {
                    let selected = tree_view.selected_node();
                    tree_view.clear_selection();
                    
                    if let Some(selected_id) = selected {
                        if let Some(selected_widget) = tree.get_widget_mut(selected_id) {
                            if selected_widget.widget_type() == "TreeNode" {
                                use crate::widgets::TreeNode;
                                if let Some(node) = selected_widget.as_any_mut().downcast_mut::<TreeNode>() {
                                    node.set_selected(false);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        dfx_info!("FFI", "TreeViewClearSelection: tree_view_id={}", tree_view_id);
    }
}