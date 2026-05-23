using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static partial class UI
    {
        public static ulong CreateTabWidget(ulong parentId, float x, float y, float width, float height)
        {
            if (_ffi.ui_create_tab_widget == IntPtr.Zero)
            {
                Log.Error("C#", "CreateTabWidget函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateTabWidgetDelegate>(_ffi.ui_create_tab_widget);
            return func(_widgetTree, parentId, x, y, width, height);
        }
        
        public static uint TabWidgetAddTab(ulong tabWidgetId, string title, ulong contentId, bool closable = false)
        {
            if (_ffi.ui_tab_widget_add_tab == IntPtr.Zero)
            {
                Log.Error("C#", "TabWidgetAddTab函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TabWidgetAddTabDelegate>(_ffi.ui_tab_widget_add_tab);
            return func(_widgetTree, tabWidgetId, title, contentId, closable);
        }
        
        public static void TabWidgetSetActive(ulong tabWidgetId, ulong index)
        {
            if (_ffi.ui_tab_widget_set_active == IntPtr.Zero)
            {
                Log.Error("C#", "TabWidgetSetActive函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TabWidgetSetActiveDelegate>(_ffi.ui_tab_widget_set_active);
            func(_widgetTree, tabWidgetId, index);
        }
        
        public static ulong TabWidgetGetActive(ulong tabWidgetId)
        {
            if (_ffi.ui_tab_widget_get_active == IntPtr.Zero)
            {
                Log.Error("C#", "TabWidgetGetActive函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TabWidgetGetActiveDelegate>(_ffi.ui_tab_widget_get_active);
            return func(_widgetTree, tabWidgetId);
        }
        
        public static void TabWidgetRemoveTab(ulong tabWidgetId, ulong index)
        {
            if (_ffi.ui_tab_widget_remove_tab == IntPtr.Zero)
            {
                Log.Error("C#", "TabWidgetRemoveTab函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TabWidgetRemoveTabDelegate>(_ffi.ui_tab_widget_remove_tab);
            func(_widgetTree, tabWidgetId, index);
        }
        
        public static void TabWidgetSetOnSelect(ulong tabWidgetId, TabSelectCallbackDelegate callback)
        {
            if (_ffi.ui_tab_widget_set_on_select_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "TabWidgetSetOnSelectThunkPtr函数指针为空");
                return;
            }
            _tabSelectCallbacks[tabWidgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<TabWidgetSetOnSelectThunkPtrDelegate>(_ffi.ui_tab_widget_set_on_select_thunk_ptr);
            func(_widgetTree, tabWidgetId, callbackPtr);
        }
        
        public static void TabWidgetSetOnClose(ulong tabWidgetId, TabCloseCallbackDelegate callback)
        {
            if (_ffi.ui_tab_widget_set_on_close_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "TabWidgetSetOnCloseThunkPtr函数指针为空");
                return;
            }
            _tabCloseCallbacks[tabWidgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<TabWidgetSetOnCloseThunkPtrDelegate>(_ffi.ui_tab_widget_set_on_close_thunk_ptr);
            func(_widgetTree, tabWidgetId, callbackPtr);
        }
        
        public static ulong TabWidgetGetTabCount(ulong tabWidgetId)
        {
            if (_ffi.ui_tab_widget_get_tab_count == IntPtr.Zero)
            {
                Log.Error("C#", "TabWidgetGetTabCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TabWidgetGetTabCountDelegate>(_ffi.ui_tab_widget_get_tab_count);
            return func(_widgetTree, tabWidgetId);
        }

        public static ulong CreateTreeView(ulong parentId, float x, float y, float width, float height)
        {
            if (_ffi.ui_create_tree_view == IntPtr.Zero)
            {
                Log.Error("C#", "CreateTreeView函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateTreeViewDelegate>(_ffi.ui_create_tree_view);
            return func(_widgetTree, parentId, x, y, width, height);
        }
        
        public static ulong TreeViewAddNode(ulong treeViewId, ulong parentNodeId, string text, ulong userData, bool hasChildren)
        {
            if (_ffi.ui_tree_view_add_node == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewAddNode函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewAddNodeDelegate>(_ffi.ui_tree_view_add_node);
            return func(_widgetTree, treeViewId, parentNodeId, text, userData, hasChildren);
        }
        
        public static void TreeViewRemoveNode(ulong treeViewId, ulong nodeId)
        {
            if (_ffi.ui_tree_view_remove_node == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewRemoveNode函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewRemoveNodeDelegate>(_ffi.ui_tree_view_remove_node);
            func(_widgetTree, treeViewId, nodeId);
        }
        
        public static void TreeViewSetSelected(ulong treeViewId, ulong nodeId)
        {
            if (_ffi.ui_tree_view_set_selected == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewSetSelected函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewSetSelectedDelegate>(_ffi.ui_tree_view_set_selected);
            func(_widgetTree, treeViewId, nodeId);
        }
        
        public static ulong TreeViewGetSelected(ulong treeViewId)
        {
            if (_ffi.ui_tree_view_get_selected == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewGetSelected函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewGetSelectedDelegate>(_ffi.ui_tree_view_get_selected);
            return func(_widgetTree, treeViewId);
        }
        
        public static void TreeViewExpandNode(ulong treeViewId, ulong nodeId)
        {
            if (_ffi.ui_tree_view_expand_node == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewExpandNode函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewExpandNodeDelegate>(_ffi.ui_tree_view_expand_node);
            func(_widgetTree, treeViewId, nodeId);
        }
        
        public static void TreeViewCollapseNode(ulong treeViewId, ulong nodeId)
        {
            if (_ffi.ui_tree_view_collapse_node == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewCollapseNode函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewCollapseNodeDelegate>(_ffi.ui_tree_view_collapse_node);
            func(_widgetTree, treeViewId, nodeId);
        }
        
        public static void TreeViewSetOnSelect(ulong treeViewId, TreeNodeSelectCallbackDelegate callback)
        {
            if (_ffi.ui_tree_view_set_on_select_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewSetOnSelectThunkPtr函数指针为空");
                return;
            }
            _treeNodeSelectCallbacks[treeViewId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewSetOnSelectThunkPtrDelegate>(_ffi.ui_tree_view_set_on_select_thunk_ptr);
            func(_widgetTree, treeViewId, callbackPtr);
        }
        
        public static void TreeViewSetOnToggle(ulong treeViewId, TreeNodeToggleCallbackDelegate callback)
        {
            if (_ffi.ui_tree_view_set_on_toggle_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewSetOnToggleThunkPtr函数指针为空");
                return;
            }
            _treeNodeToggleCallbacks[treeViewId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewSetOnToggleThunkPtrDelegate>(_ffi.ui_tree_view_set_on_toggle_thunk_ptr);
            func(_widgetTree, treeViewId, callbackPtr);
        }
        
        public static bool TreeViewIsNodeExpanded(ulong treeViewId, ulong nodeId)
        {
            if (_ffi.ui_tree_view_is_node_expanded == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewIsNodeExpanded函数指针为空");
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewIsNodeExpandedDelegate>(_ffi.ui_tree_view_is_node_expanded);
            return func(_widgetTree, treeViewId, nodeId);
        }
        
        public static void TreeNodeSetText(ulong nodeId, string text)
        {
            if (_ffi.ui_tree_node_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "TreeNodeSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeNodeSetTextDelegate>(_ffi.ui_tree_node_set_text);
            func(_widgetTree, nodeId, text);
        }
        
        public static ulong TreeNodeGetUserData(ulong nodeId)
        {
            if (_ffi.ui_tree_node_get_user_data == IntPtr.Zero)
            {
                Log.Error("C#", "TreeNodeGetUserData函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeNodeGetUserDataDelegate>(_ffi.ui_tree_node_get_user_data);
            return func(_widgetTree, nodeId);
        }
        
        public static void TreeViewClearSelection(ulong treeViewId)
        {
            if (_ffi.ui_tree_view_clear_selection == IntPtr.Zero)
            {
                Log.Error("C#", "TreeViewClearSelection函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TreeViewClearSelectionDelegate>(_ffi.ui_tree_view_clear_selection);
            func(_widgetTree, treeViewId);
        }

        public static ulong CreatePopupMenu(ulong parentId)
        {
            if (_ffi.ui_create_popup_menu == IntPtr.Zero)
            {
                Log.Error("C#", "CreatePopupMenu函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreatePopupMenuDelegate>(_ffi.ui_create_popup_menu);
            return func(_widgetTree, parentId);
        }
        
        public static void PopupMenuAddItem(ulong menuId, string text, string shortcut, int actionId)
        {
            if (_ffi.ui_popup_menu_add_item == IntPtr.Zero)
            {
                Log.Error("C#", "PopupMenuAddItem函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<PopupMenuAddItemDelegate>(_ffi.ui_popup_menu_add_item);
            func(_widgetTree, menuId, text, shortcut ?? "", actionId);
        }
        
        public static void PopupMenuAddSeparator(ulong menuId)
        {
            if (_ffi.ui_popup_menu_add_separator == IntPtr.Zero)
            {
                Log.Error("C#", "PopupMenuAddSeparator函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<PopupMenuAddSeparatorDelegate>(_ffi.ui_popup_menu_add_separator);
            func(_widgetTree, menuId);
        }
        
        public static void PopupMenuShow(ulong menuId, float x, float y)
        {
            if (_ffi.ui_popup_menu_show == IntPtr.Zero)
            {
                Log.Error("C#", "PopupMenuShow函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<PopupMenuShowDelegate>(_ffi.ui_popup_menu_show);
            func(_widgetTree, menuId, x, y);
        }
        
        public static void PopupMenuHide(ulong menuId)
        {
            if (_ffi.ui_popup_menu_hide == IntPtr.Zero)
            {
                Log.Error("C#", "PopupMenuHide函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<PopupMenuHideDelegate>(_ffi.ui_popup_menu_hide);
            func(_widgetTree, menuId);
        }
        
        public static bool PopupMenuIsVisible(ulong menuId)
        {
            if (_ffi.ui_popup_menu_is_visible == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<PopupMenuIsVisibleDelegate>(_ffi.ui_popup_menu_is_visible);
            return func(_widgetTree, menuId);
        }
        
        public static void PopupMenuSetOnClick(ulong menuId, PopupMenuClickCallbackDelegate callback)
        {
            if (_ffi.ui_popup_menu_set_on_click_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "PopupMenuSetOnClickThunkPtr函数指针为空");
                return;
            }
            _popupMenuCallbacks[menuId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<PopupMenuSetOnClickThunkPtrDelegate>(_ffi.ui_popup_menu_set_on_click_thunk_ptr);
            func(_widgetTree, menuId, callbackPtr);
        }

        public static ulong CreateGridView(ulong parentId, float x, float y, float width, float height, float cellSize = 64)
        {
            if (_ffi.ui_create_grid_view == IntPtr.Zero)
            {
                Log.Error("C#", "CreateGridView函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateGridViewDelegate>(_ffi.ui_create_grid_view);
            return func(_widgetTree, parentId, x, y, width, height, cellSize);
        }
        
        public static uint GridViewAddItem(ulong gridId, string label, ulong userData)
        {
            if (_ffi.ui_grid_view_add_item == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewAddItem函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GridViewAddItemDelegate>(_ffi.ui_grid_view_add_item);
            return func(_widgetTree, gridId, label, userData);
        }
        
        public static void GridViewRemoveItem(ulong gridId, int index)
        {
            if (_ffi.ui_grid_view_remove_item == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewRemoveItem函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GridViewRemoveItemDelegate>(_ffi.ui_grid_view_remove_item);
            func(_widgetTree, gridId, index);
        }
        
        public static void GridViewSetSelected(ulong gridId, int index)
        {
            if (_ffi.ui_grid_view_set_selected == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewSetSelected函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GridViewSetSelectedDelegate>(_ffi.ui_grid_view_set_selected);
            func(_widgetTree, gridId, index);
        }
        
        public static int GridViewGetSelected(ulong gridId)
        {
            if (_ffi.ui_grid_view_get_selected == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewGetSelected函数指针为空");
                return -1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GridViewGetSelectedDelegate>(_ffi.ui_grid_view_get_selected);
            int index = func(_widgetTree, gridId);
            return index == int.MaxValue ? -1 : index;
        }
        
        public static ulong GridViewGetSelectedUserData(ulong gridId)
        {
            if (_ffi.ui_grid_view_get_selected_user_data == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewGetSelectedUserData函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GridViewGetSelectedUserDataDelegate>(_ffi.ui_grid_view_get_selected_user_data);
            return func(_widgetTree, gridId);
        }
        
        public static void GridViewClear(ulong gridId)
        {
            if (_ffi.ui_grid_view_clear == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewClear函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GridViewClearDelegate>(_ffi.ui_grid_view_clear);
            func(_widgetTree, gridId);
        }
        
        public static int GridViewItemCount(ulong gridId)
        {
            if (_ffi.ui_grid_view_item_count == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewItemCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GridViewItemCountDelegate>(_ffi.ui_grid_view_item_count);
            return func(_widgetTree, gridId);
        }
        
        public static void GridViewSetOnClick(ulong gridId, GridViewClickCallbackDelegate callback)
        {
            if (_ffi.ui_grid_view_set_on_click_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "GridViewSetOnClickThunkPtr函数指针为空");
                return;
            }
            _gridViewCallbacks[gridId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<GridViewSetOnClickThunkPtrDelegate>(_ffi.ui_grid_view_set_on_click_thunk_ptr);
            func(_widgetTree, gridId, callbackPtr);
        }

        public static ulong CreateDialog(ulong parentId, string title, float width, float height)
        {
            if (_ffi.ui_create_dialog == IntPtr.Zero)
            {
                Log.Error("C#", "CreateDialog函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateDialogDelegate>(_ffi.ui_create_dialog);
            return func(_widgetTree, parentId, title, width, height);
        }
        
        public static void DialogSetContent(ulong dialogId, ulong contentId)
        {
            if (_ffi.ui_dialog_set_content == IntPtr.Zero)
            {
                Log.Error("C#", "DialogSetContent函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DialogSetContentDelegate>(_ffi.ui_dialog_set_content);
            func(_widgetTree, dialogId, contentId);
        }
        
        public static void DialogAddButton(ulong dialogId, string text, int action)
        {
            if (_ffi.ui_dialog_add_button == IntPtr.Zero)
            {
                Log.Error("C#", "DialogAddButton函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DialogAddButtonDelegate>(_ffi.ui_dialog_add_button);
            func(_widgetTree, dialogId, text, action);
        }
        
        public static void DialogShow(ulong dialogId)
        {
            if (_ffi.ui_dialog_show == IntPtr.Zero)
            {
                Log.Error("C#", "DialogShow函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DialogShowDelegate>(_ffi.ui_dialog_show);
            func(_widgetTree, dialogId);
        }
        
        public static void DialogHide(ulong dialogId)
        {
            if (_ffi.ui_dialog_hide == IntPtr.Zero)
            {
                Log.Error("C#", "DialogHide函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DialogHideDelegate>(_ffi.ui_dialog_hide);
            func(_widgetTree, dialogId);
        }
        
        public static bool DialogIsVisible(ulong dialogId)
        {
            if (_ffi.ui_dialog_is_visible == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DialogIsVisibleDelegate>(_ffi.ui_dialog_is_visible);
            return func(_widgetTree, dialogId);
        }
        
        public static int DialogGetResult(ulong dialogId)
        {
            if (_ffi.ui_dialog_get_result == IntPtr.Zero)
            {
                return -1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DialogGetResultDelegate>(_ffi.ui_dialog_get_result);
            return func(_widgetTree, dialogId);
        }
        
        public static void DialogSetOnResult(ulong dialogId, DialogResultCallbackDelegate callback)
        {
            if (_ffi.ui_dialog_set_on_result_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "DialogSetOnResultThunkPtr函数指针为空");
                return;
            }
            _dialogCallbacks[dialogId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<DialogSetOnResultThunkPtrDelegate>(_ffi.ui_dialog_set_on_result_thunk_ptr);
            func(_widgetTree, dialogId, callbackPtr);
        }
        
        public static ulong CreateFileBrowser(ulong parentId, float x, float y, float width, float height, string initialPath = null)
        {
            if (_ffi.ui_create_file_browser == IntPtr.Zero)
            {
                Log.Error("C#", "CreateFileBrowser函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateFileBrowserDelegate>(_ffi.ui_create_file_browser);
            return func(_widgetTree, parentId, x, y, width, height, initialPath ?? ".");
        }
        
        public static void FileBrowserSetPath(ulong browserId, string path)
        {
            if (_ffi.ui_file_browser_set_path == IntPtr.Zero)
            {
                Log.Error("C#", "FileBrowserSetPath函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserSetPathDelegate>(_ffi.ui_file_browser_set_path);
            func(_widgetTree, browserId, path);
        }
        
        public static void FileBrowserSetFilter(ulong browserId, string filter)
        {
            if (_ffi.ui_file_browser_set_filter == IntPtr.Zero)
            {
                Log.Error("C#", "FileBrowserSetFilter函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserSetFilterDelegate>(_ffi.ui_file_browser_set_filter);
            func(_widgetTree, browserId, filter);
        }
        
        public static void FileBrowserNavigateUp(ulong browserId)
        {
            if (_ffi.ui_file_browser_navigate_up == IntPtr.Zero)
            {
                Log.Error("C#", "FileBrowserNavigateUp函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserNavigateUpDelegate>(_ffi.ui_file_browser_navigate_up);
            func(_widgetTree, browserId);
        }
        
        public static void FileBrowserRefresh(ulong browserId)
        {
            if (_ffi.ui_file_browser_refresh == IntPtr.Zero)
            {
                Log.Error("C#", "FileBrowserRefresh函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserRefreshDelegate>(_ffi.ui_file_browser_refresh);
            func(_widgetTree, browserId);
        }
        
        public static string FileBrowserGetSelectedPath(ulong browserId)
        {
            if (_ffi.ui_file_browser_get_selected_path == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(512);
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserGetSelectedPathDelegate>(_ffi.ui_file_browser_get_selected_path);
            bool success = func(_widgetTree, browserId, buffer, 512);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static string FileBrowserGetCurrentPath(ulong browserId)
        {
            if (_ffi.ui_file_browser_get_current_path == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(512);
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserGetCurrentPathDelegate>(_ffi.ui_file_browser_get_current_path);
            bool success = func(_widgetTree, browserId, buffer, 512);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static void FileBrowserSetOnSelect(ulong browserId, FileBrowserSelectCallbackDelegate callback)
        {
            if (_ffi.ui_file_browser_set_on_select_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "FileBrowserSetOnSelectThunkPtr函数指针为空");
                return;
            }
            _fileBrowserSelectCallbacks[browserId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserSetOnSelectThunkPtrDelegate>(_ffi.ui_file_browser_set_on_select_thunk_ptr);
            func(_widgetTree, browserId, callbackPtr);
        }
        
        public static void FileBrowserSetOnDoubleClick(ulong browserId, FileBrowserDoubleClickCallbackDelegate callback)
        {
            if (_ffi.ui_file_browser_set_on_double_click_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "FileBrowserSetOnDoubleClickThunkPtr函数指针为空");
                return;
            }
            _fileBrowserDoubleClickCallbacks[browserId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<FileBrowserSetOnDoubleClickThunkPtrDelegate>(_ffi.ui_file_browser_set_on_double_click_thunk_ptr);
            func(_widgetTree, browserId, callbackPtr);
        }

        // === Checkbox ===

        public static ulong CreateCheckbox(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_checkbox == IntPtr.Zero)
            {
                Log.Error("C#", "CreateCheckbox函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateCheckboxDelegate>(_ffi.ui_create_checkbox);
            return func(_widgetTree, parentId, width, height);
        }

        public static ulong CreateCheckbox(ulong parentId, float x, float y, float width, float height, string text)
        {
            if (_ffi.ui_create_checkbox_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateCheckboxInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateCheckboxInParentDelegate>(_ffi.ui_create_checkbox_in_parent);
            return func(_widgetTree, parentId, x, y, width, height, text);
        }

        public static void CheckboxSetChecked(ulong widgetId, bool isChecked)
        {
            if (_ffi.ui_checkbox_set_checked == IntPtr.Zero)
            {
                Log.Error("C#", "CheckboxSetChecked函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CheckboxSetCheckedDelegate>(_ffi.ui_checkbox_set_checked);
            func(_widgetTree, widgetId, isChecked ? 1u : 0u);
        }

        public static bool CheckboxGetChecked(ulong widgetId)
        {
            if (_ffi.ui_checkbox_get_checked == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CheckboxGetCheckedDelegate>(_ffi.ui_checkbox_get_checked);
            return func(_widgetTree, widgetId) != 0;
        }

        public static void CheckboxSetOnChange(ulong widgetId, CheckboxChangeCallbackDelegate callback)
        {
            if (_ffi.ui_checkbox_set_on_change_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "CheckboxSetOnChangeThunkPtr函数指针为空");
                return;
            }
            _checkboxCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<CheckboxSetOnChangeThunkPtrDelegate>(_ffi.ui_checkbox_set_on_change_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }

        public static void CheckboxSetText(ulong widgetId, string text)
        {
            if (_ffi.ui_checkbox_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "CheckboxSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CheckboxSetTextDelegate>(_ffi.ui_checkbox_set_text);
            func(_widgetTree, widgetId, text);
        }

        public static void RegisterCheckboxOnChangeThunkPtr(ulong widgetId, IntPtr callbackPtr)
        {
            if (_ffi.ui_checkbox_set_on_change_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "CheckboxSetOnChangeThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CheckboxSetOnChangeThunkPtrDelegate>(_ffi.ui_checkbox_set_on_change_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }

        // === Slider ===

        public static ulong CreateSlider(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_slider == IntPtr.Zero)
            {
                Log.Error("C#", "CreateSlider函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateSliderDelegate>(_ffi.ui_create_slider);
            return func(_widgetTree, parentId, width, height);
        }

        public static ulong CreateSlider(ulong parentId, float x, float y, float width, float height, float min, float max, float value)
        {
            if (_ffi.ui_create_slider_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateSliderInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateSliderInParentDelegate>(_ffi.ui_create_slider_in_parent);
            return func(_widgetTree, parentId, x, y, width, height, min, max, value);
        }

        public static void SliderSetValue(ulong widgetId, float value)
        {
            if (_ffi.ui_slider_set_value == IntPtr.Zero)
            {
                Log.Error("C#", "SliderSetValue函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SliderSetValueDelegate>(_ffi.ui_slider_set_value);
            func(_widgetTree, widgetId, value);
        }

        public static float SliderGetValue(ulong widgetId)
        {
            if (_ffi.ui_slider_get_value == IntPtr.Zero)
            {
                return 0.0f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SliderGetValueDelegate>(_ffi.ui_slider_get_value);
            return func(_widgetTree, widgetId);
        }

        public static void SliderSetRange(ulong widgetId, float min, float max)
        {
            if (_ffi.ui_slider_set_range == IntPtr.Zero)
            {
                Log.Error("C#", "SliderSetRange函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SliderSetRangeDelegate>(_ffi.ui_slider_set_range);
            func(_widgetTree, widgetId, min, max);
        }

        public static void SliderSetStep(ulong widgetId, float step)
        {
            if (_ffi.ui_slider_set_step == IntPtr.Zero)
            {
                Log.Error("C#", "SliderSetStep函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SliderSetStepDelegate>(_ffi.ui_slider_set_step);
            func(_widgetTree, widgetId, step);
        }

        public static void SliderSetOnChange(ulong widgetId, SliderChangeCallbackDelegate callback)
        {
            if (_ffi.ui_slider_set_on_change_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "SliderSetOnChangeThunkPtr函数指针为空");
                return;
            }
            _sliderCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<SliderSetOnChangeThunkPtrDelegate>(_ffi.ui_slider_set_on_change_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }

        // === ScrollView ===

        public static ulong CreateScrollView(ulong parentId, float x, float y, float width, float height)
        {
            if (_ffi.ui_create_scroll_view == IntPtr.Zero)
            {
                Log.Error("C#", "CreateScrollView函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateScrollViewDelegate>(_ffi.ui_create_scroll_view);
            return func(_widgetTree, parentId, x, y, width, height);
        }

        public static void ScrollViewSetScrollOffset(ulong widgetId, float offsetY)
        {
            if (_ffi.ui_scroll_view_set_scroll_offset == IntPtr.Zero)
            {
                Log.Error("C#", "ScrollViewSetScrollOffset函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ScrollViewSetScrollOffsetDelegate>(_ffi.ui_scroll_view_set_scroll_offset);
            func(_widgetTree, widgetId, offsetY);
        }

        public static float ScrollViewGetScrollOffset(ulong widgetId)
        {
            if (_ffi.ui_scroll_view_get_scroll_offset == IntPtr.Zero)
            {
                return 0.0f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ScrollViewGetScrollOffsetDelegate>(_ffi.ui_scroll_view_get_scroll_offset);
            return func(_widgetTree, widgetId);
        }

        public static void ScrollViewSetShowScrollbars(ulong widgetId, bool showV, bool showH)
        {
            if (_ffi.ui_scroll_view_set_show_scrollbars == IntPtr.Zero)
            {
                Log.Error("C#", "ScrollViewSetShowScrollbars函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ScrollViewSetShowScrollbarsDelegate>(_ffi.ui_scroll_view_set_show_scrollbars);
            func(_widgetTree, widgetId, showV ? 1u : 0u, showH ? 1u : 0u);
        }

        public static void ScrollViewSetOnScroll(ulong widgetId, ScrollViewScrollCallbackDelegate callback)
        {
            if (_ffi.ui_scroll_view_set_on_scroll_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "ScrollViewSetOnScrollThunkPtr函数指针为空");
                return;
            }
            _scrollViewCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<ScrollViewSetOnScrollThunkPtrDelegate>(_ffi.ui_scroll_view_set_on_scroll_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }

        // === SplitView ===

        public static ulong CreateSplitView(ulong parentId, float x, float y, float width, float height, uint orientation)
        {
            if (_ffi.ui_create_split_view == IntPtr.Zero)
            {
                Log.Error("C#", "CreateSplitView函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateSplitViewDelegate>(_ffi.ui_create_split_view);
            return func(_widgetTree, parentId, x, y, width, height, orientation);
        }

        public static void SplitViewSetSplitRatio(ulong widgetId, float ratio)
        {
            if (_ffi.ui_split_view_set_split_ratio == IntPtr.Zero)
            {
                Log.Error("C#", "SplitViewSetSplitRatio函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SplitViewSetSplitRatioDelegate>(_ffi.ui_split_view_set_split_ratio);
            func(_widgetTree, widgetId, ratio);
        }

        public static float SplitViewGetSplitRatio(ulong widgetId)
        {
            if (_ffi.ui_split_view_get_split_ratio == IntPtr.Zero)
            {
                return 0.5f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SplitViewGetSplitRatioDelegate>(_ffi.ui_split_view_get_split_ratio);
            return func(_widgetTree, widgetId);
        }

        public static void SplitViewSetMinRatio(ulong widgetId, float min)
        {
            if (_ffi.ui_split_view_set_min_ratio == IntPtr.Zero)
            {
                Log.Error("C#", "SplitViewSetMinRatio函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SplitViewSetMinRatioDelegate>(_ffi.ui_split_view_set_min_ratio);
            func(_widgetTree, widgetId, min);
        }

        public static void SplitViewSetMaxRatio(ulong widgetId, float max)
        {
            if (_ffi.ui_split_view_set_max_ratio == IntPtr.Zero)
            {
                Log.Error("C#", "SplitViewSetMaxRatio函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SplitViewSetMaxRatioDelegate>(_ffi.ui_split_view_set_max_ratio);
            func(_widgetTree, widgetId, max);
        }

        public static void SplitViewSetOnRatioChange(ulong widgetId, SplitViewRatioChangeCallbackDelegate callback)
        {
            if (_ffi.ui_split_view_set_on_ratio_change_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "SplitViewSetOnRatioChangeThunkPtr函数指针为空");
                return;
            }
            _splitViewCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<SplitViewSetOnRatioChangeThunkPtrDelegate>(_ffi.ui_split_view_set_on_ratio_change_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }
    }

    public class TabWidget
    {
        public ulong Id { get; private set; }
        private UI.TabSelectCallbackDelegate _selectCallback;
        private UI.TabCloseCallbackDelegate _closeCallback;
        
        public TabWidget(ulong parentId, float x, float y, float width, float height)
        {
            Id = UI.CreateTabWidget(parentId, x, y, width, height);
        }
        
        public uint AddTab(string title, ulong contentId, bool closable = false)
        {
            return UI.TabWidgetAddTab(Id, title, contentId, closable);
        }
        
        public void SetActive(ulong index)
        {
            UI.TabWidgetSetActive(Id, index);
        }
        
        public ulong GetActive()
        {
            return UI.TabWidgetGetActive(Id);
        }
        
        public void RemoveTab(ulong index)
        {
            UI.TabWidgetRemoveTab(Id, index);
        }
        
        public ulong GetTabCount()
        {
            return UI.TabWidgetGetTabCount(Id);
        }
        
        public void SetOnSelect(UI.TabSelectCallbackDelegate callback)
        {
            _selectCallback = callback;
            UI.TabWidgetSetOnSelect(Id, callback);
        }
        
        public void SetOnClose(UI.TabCloseCallbackDelegate callback)
        {
            _closeCallback = callback;
            UI.TabWidgetSetOnClose(Id, callback);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }

    public class TreeView
    {
        public ulong Id { get; private set; }
        private UI.TreeNodeSelectCallbackDelegate _selectCallback;
        
        public TreeView(ulong parentId, float x, float y, float width, float height)
        {
            Id = UI.CreateTreeView(parentId, x, y, width, height);
        }
        
        public TreeNode AddRootNode(string text, ulong userData = 0, bool hasChildren = false)
        {
            ulong nodeId = UI.TreeViewAddNode(Id, 0, text, userData, hasChildren);
            return new TreeNode(nodeId, text, userData);
        }
        
        public TreeNode AddChildNode(ulong parentNodeId, string text, ulong userData = 0, bool hasChildren = false)
        {
            ulong nodeId = UI.TreeViewAddNode(Id, parentNodeId, text, userData, hasChildren);
            return new TreeNode(nodeId, text, userData);
        }
        
        public void RemoveNode(ulong nodeId)
        {
            UI.TreeViewRemoveNode(Id, nodeId);
        }
        
        public void SetSelected(ulong nodeId)
        {
            UI.TreeViewSetSelected(Id, nodeId);
        }
        
        public ulong GetSelected()
        {
            return UI.TreeViewGetSelected(Id);
        }
        
        public void ExpandNode(ulong nodeId)
        {
            UI.TreeViewExpandNode(Id, nodeId);
        }
        
        public void CollapseNode(ulong nodeId)
        {
            UI.TreeViewCollapseNode(Id, nodeId);
        }
        
        public void ClearSelection()
        {
            UI.TreeViewClearSelection(Id);
        }
        
        public void SetOnSelect(UI.TreeNodeSelectCallbackDelegate callback)
        {
            _selectCallback = callback;
            UI.TreeViewSetOnSelect(Id, callback);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }
    
    public class TreeNode
    {
        public ulong Id { get; private set; }
        private string _text;
        private ulong _userData;
        
        public TreeNode(ulong id, string text, ulong userData = 0)
        {
            Id = id;
            _text = text;
            _userData = userData;
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.TreeNodeSetText(Id, _text); }
        }
        
        public ulong UserData
        {
            get => _userData;
        }
        
        public ulong GetUserDataFromWidget()
        {
            return UI.TreeNodeGetUserData(Id);
        }
    }
    
    public class PopupMenu
    {
        public ulong Id { get; private set; }
        private UI.PopupMenuClickCallbackDelegate _callback;
        
        public PopupMenu(ulong parentId)
        {
            Id = UI.CreatePopupMenu(parentId);
        }
        
        public void AddItem(string text, string shortcut, int actionId)
        {
            UI.PopupMenuAddItem(Id, text, shortcut, actionId);
        }
        
        public void AddItem(string text, int actionId)
        {
            UI.PopupMenuAddItem(Id, text, null, actionId);
        }
        
        public void AddSeparator()
        {
            UI.PopupMenuAddSeparator(Id);
        }
        
        public void Show(float x, float y)
        {
            UI.PopupMenuShow(Id, x, y);
        }
        
        public void Hide()
        {
            UI.PopupMenuHide(Id);
        }
        
        public bool IsVisible => UI.PopupMenuIsVisible(Id);
        
        public void SetOnClick(UI.PopupMenuClickCallbackDelegate callback)
        {
            _callback = callback;
            UI.PopupMenuSetOnClick(Id, callback);
        }
    }
    
    public class GridView
    {
        public ulong Id { get; private set; }
        private UI.GridViewClickCallbackDelegate _callback;
        private float _cellSize;
        
        public GridView(ulong parentId, float x, float y, float width, float height, float cellSize = 64)
        {
            _cellSize = cellSize;
            Id = UI.CreateGridView(parentId, x, y, width, height, cellSize);
        }
        
        public uint AddItem(string label, ulong userData)
        {
            return UI.GridViewAddItem(Id, label, userData);
        }
        
        public void RemoveItem(int index)
        {
            UI.GridViewRemoveItem(Id, index);
        }
        
        public void SetSelected(int index)
        {
            UI.GridViewSetSelected(Id, index);
        }
        
        public int SelectedIndex => UI.GridViewGetSelected(Id);
        
        public ulong SelectedUserData => UI.GridViewGetSelectedUserData(Id);
        
        public void Clear()
        {
            UI.GridViewClear(Id);
        }
        
        public int ItemCount => UI.GridViewItemCount(Id);
        
        public void SetOnClick(UI.GridViewClickCallbackDelegate callback)
        {
            _callback = callback;
            UI.GridViewSetOnClick(Id, callback);
        }
        
        public float CellSize => _cellSize;
    }

    public class Checkbox
    {
        public ulong Id;
        public string Text;
        public bool Checked;

        private UI.CheckboxChangeCallbackDelegate _onChangeCallback;

        public Checkbox(ulong parentId, float x, float y, float w, float h, string text)
        {
            Text = text;
            Checked = false;
            Id = UI.CreateCheckbox(parentId, x, y, w, h, text);
        }

        public static ulong Create(ulong parentId, float x, float y, float w, float h, string text)
        {
            return UI.CreateCheckbox(parentId, x, y, w, h, text);
        }

        public static void SetChecked(ulong id, bool isChecked)
        {
            UI.CheckboxSetChecked(id, isChecked);
        }

        public static bool GetChecked(ulong id)
        {
            return UI.CheckboxGetChecked(id);
        }

        public static void SetOnChangeThunkPtr(ulong id, UI.CheckboxChangeCallbackDelegate callback)
        {
            UI.CheckboxSetOnChange(id, callback);
        }

        public static void SetText(ulong id, string text)
        {
            UI.CheckboxSetText(id, text);
        }

        public void SetChecked(bool isChecked)
        {
            Checked = isChecked;
            UI.CheckboxSetChecked(Id, isChecked);
        }

        public bool IsChecked()
        {
            Checked = UI.CheckboxGetChecked(Id);
            return Checked;
        }

        public void SetOnChange(UI.CheckboxChangeCallbackDelegate callback)
        {
            _onChangeCallback = callback;
            UI.CheckboxSetOnChange(Id, callback);
        }

        public void SetText(string text)
        {
            Text = text;
            UI.CheckboxSetText(Id, text);
        }
    }

    public class Slider
    {
        public ulong Id;
        public float Value;
        public float Min;
        public float Max;

        private UI.SliderChangeCallbackDelegate _onChangeCallback;

        public Slider(ulong parentId, float x, float y, float w, float h, float min, float max, float value)
        {
            Min = min;
            Max = max;
            Value = value;
            Id = UI.CreateSlider(parentId, x, y, w, h, min, max, value);
        }

        public static ulong Create(ulong parentId, float x, float y, float w, float h, float min, float max, float value)
        {
            return UI.CreateSlider(parentId, x, y, w, h, min, max, value);
        }

        public static void SetValue(ulong id, float value)
        {
            UI.SliderSetValue(id, value);
        }

        public static float GetValue(ulong id)
        {
            return UI.SliderGetValue(id);
        }

        public static void SetRange(ulong id, float min, float max)
        {
            UI.SliderSetRange(id, min, max);
        }

        public static void SetStep(ulong id, float step)
        {
            UI.SliderSetStep(id, step);
        }

        public static void SetOnChangeThunkPtr(ulong id, UI.SliderChangeCallbackDelegate callback)
        {
            UI.SliderSetOnChange(id, callback);
        }

        public void SetValue(float value)
        {
            Value = value;
            UI.SliderSetValue(Id, value);
        }

        public float GetValue()
        {
            Value = UI.SliderGetValue(Id);
            return Value;
        }

        public void SetOnChange(UI.SliderChangeCallbackDelegate callback)
        {
            _onChangeCallback = callback;
            UI.SliderSetOnChange(Id, callback);
        }
    }

    public class ScrollView
    {
        public ulong Id;
        private UI.ScrollViewScrollCallbackDelegate _onScrollCallback;

        public ScrollView(ulong parentId, float x, float y, float w, float h)
        {
            Id = UI.CreateScrollView(parentId, x, y, w, h);
        }

        public static ulong Create(ulong parentId, float x, float y, float w, float h)
        {
            return UI.CreateScrollView(parentId, x, y, w, h);
        }

        public static void SetScrollOffset(ulong id, float offsetY)
        {
            UI.ScrollViewSetScrollOffset(id, offsetY);
        }

        public static float GetScrollOffset(ulong id)
        {
            return UI.ScrollViewGetScrollOffset(id);
        }

        public static void SetShowScrollbars(ulong id, bool showV, bool showH)
        {
            UI.ScrollViewSetShowScrollbars(id, showV, showH);
        }

        public static void SetOnScrollThunkPtr(ulong id, UI.ScrollViewScrollCallbackDelegate callback)
        {
            UI.ScrollViewSetOnScroll(id, callback);
        }

        public void SetScrollOffset(float offsetY)
        {
            UI.ScrollViewSetScrollOffset(Id, offsetY);
        }

        public float GetScrollOffset()
        {
            return UI.ScrollViewGetScrollOffset(Id);
        }

        public void SetOnScroll(UI.ScrollViewScrollCallbackDelegate callback)
        {
            _onScrollCallback = callback;
            UI.ScrollViewSetOnScroll(Id, callback);
        }
    }

    public class SplitView
    {
        public ulong Id;
        public float Ratio;
        private UI.SplitViewRatioChangeCallbackDelegate _onRatioChangeCallback;

        public SplitView(ulong parentId, float x, float y, float w, float h, uint orientation)
        {
            Id = UI.CreateSplitView(parentId, x, y, w, h, orientation);
            Ratio = 0.5f;
        }

        public static ulong Create(ulong parentId, float x, float y, float w, float h, uint orientation)
        {
            return UI.CreateSplitView(parentId, x, y, w, h, orientation);
        }

        public static void SetSplitRatio(ulong id, float ratio)
        {
            UI.SplitViewSetSplitRatio(id, ratio);
        }

        public static float GetSplitRatio(ulong id)
        {
            return UI.SplitViewGetSplitRatio(id);
        }

        public static void SetMinRatio(ulong id, float min)
        {
            UI.SplitViewSetMinRatio(id, min);
        }

        public static void SetMaxRatio(ulong id, float max)
        {
            UI.SplitViewSetMaxRatio(id, max);
        }

        public static void SetOnRatioChangeThunkPtr(ulong id, UI.SplitViewRatioChangeCallbackDelegate callback)
        {
            UI.SplitViewSetOnRatioChange(id, callback);
        }

        public void SetSplitRatio(float ratio)
        {
            Ratio = ratio;
            UI.SplitViewSetSplitRatio(Id, ratio);
        }

        public float GetSplitRatio()
        {
            Ratio = UI.SplitViewGetSplitRatio(Id);
            return Ratio;
        }

        public void SetOnRatioChange(UI.SplitViewRatioChangeCallbackDelegate callback)
        {
            _onRatioChangeCallback = callback;
            UI.SplitViewSetOnRatioChange(Id, callback);
        }
    }
}