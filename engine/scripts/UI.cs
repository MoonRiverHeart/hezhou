using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class UI
    {
        private static IntPtr _widgetTree;
        private static FfiContext _ffi;
        
        private static ResizeCallbackDelegate _savedResizeCallback;
        private static GlobalClickCallbackDelegate _savedGlobalClickCallback;
        private static KeyCallbackDelegate _savedKeyCallback;
        private static MouseMoveCallbackDelegate _savedMouseMoveCallback;
        private static UpdateCallbackDelegate _savedUpdateCallback;
        private static Dictionary<ulong, WidgetCallbackDelegate> _onclickCallbacks = new Dictionary<ulong, WidgetCallbackDelegate>();
        private static Dictionary<ulong, DropdownSelectCallbackDelegate> _dropdownCallbacks = new Dictionary<ulong, DropdownSelectCallbackDelegate>();
        private static Dictionary<ulong, InputFieldChangeCallbackDelegate> _inputFieldCallbacks = new Dictionary<ulong, InputFieldChangeCallbackDelegate>();
        private static Dictionary<ulong, TabSelectCallbackDelegate> _tabSelectCallbacks = new Dictionary<ulong, TabSelectCallbackDelegate>();
        private static Dictionary<ulong, TabCloseCallbackDelegate> _tabCloseCallbacks = new Dictionary<ulong, TabCloseCallbackDelegate>();
        private static Dictionary<ulong, TreeNodeSelectCallbackDelegate> _treeNodeSelectCallbacks = new Dictionary<ulong, TreeNodeSelectCallbackDelegate>();
        private static Dictionary<ulong, TreeNodeToggleCallbackDelegate> _treeNodeToggleCallbacks = new Dictionary<ulong, TreeNodeToggleCallbackDelegate>();
        private static Dictionary<ulong, PopupMenuClickCallbackDelegate> _popupMenuCallbacks = new Dictionary<ulong, PopupMenuClickCallbackDelegate>();
        private static Dictionary<ulong, GridViewClickCallbackDelegate> _gridViewCallbacks = new Dictionary<ulong, GridViewClickCallbackDelegate>();
        private static Dictionary<ulong, DialogResultCallbackDelegate> _dialogCallbacks = new Dictionary<ulong, DialogResultCallbackDelegate>();
        private static Dictionary<ulong, FileBrowserSelectCallbackDelegate> _fileBrowserSelectCallbacks = new Dictionary<ulong, FileBrowserSelectCallbackDelegate>();
        private static Dictionary<ulong, FileBrowserDoubleClickCallbackDelegate> _fileBrowserDoubleClickCallbacks = new Dictionary<ulong, FileBrowserDoubleClickCallbackDelegate>();
        private static Dictionary<ulong, CheckboxChangeCallbackDelegate> _checkboxCallbacks = new Dictionary<ulong, CheckboxChangeCallbackDelegate>();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong GetButtonIdDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetOnClickDelegate(IntPtr handle, ulong widgetId, IntPtr callback);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void KeyCallbackDelegate(uint keycode, bool pressed, uint modifiers);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void MouseMoveCallbackDelegate(float x, float y, bool dragging);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetTextDelegate(IntPtr handle, ulong widgetId, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateVStackDelegate(IntPtr handle, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateVStackInParentDelegate(IntPtr handle, ulong parentId, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateHStackDelegate(IntPtr handle, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateHStackInParentDelegate(IntPtr handle, ulong parentId, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateButtonInParentDelegate(IntPtr handle, ulong parentId, float width, float height, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateLabelInParentDelegate(IntPtr handle, ulong parentId, float width, float height, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreatePreviewWindowDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height, ulong textureId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPreviewTextureDelegate(IntPtr handle, ulong widgetId, ulong textureId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreatePanelInParentDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height, float r, float g, float b, float a);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong GetRootIdDelegate(IntPtr handle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetWidgetLayoutDelegate(IntPtr handle, ulong widgetId, float x, float y, float width, float height);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPositionDelegate(IntPtr handle, ulong widgetId, float x, float y);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetSizeDelegate(IntPtr handle, ulong widgetId, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RemoveWidgetDelegate(IntPtr handle, ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateTextEditDelegate(IntPtr handle, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateTextEditInParentDelegate(IntPtr handle, ulong parentId, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditSetTextDelegate(IntPtr handle, ulong widgetId, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditInsertCharDelegate(IntPtr handle, ulong widgetId, byte c);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditDeleteCharDelegate(IntPtr handle, ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int TextEditGetTextLenDelegate(IntPtr handle, ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditGetTextDelegate(IntPtr handle, ulong widgetId, IntPtr buffer, int bufferSize);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditShowLineNumbersDelegate(IntPtr handle, ulong widgetId, bool show);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterResizeDelegate(IntPtr callbackPtr);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GetScreenSizeDelegate(out float width, out float height);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetContentScaleDelegate(float scale);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate float GetContentScaleDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TriggerHotReloadDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetStatusTextDelegate([MarshalAs(UnmanagedType.LPStr)] string status);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void OnHotReloadCompleteDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DebugPrintWidgetTreeDelegate(IntPtr handle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetFlexExpandDelegate(IntPtr handle, ulong widgetId, uint expand);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetCrossAxisFillDelegate(IntPtr handle, ulong widgetId, uint fill);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetWidgetBackgroundColorDelegate(IntPtr handle, ulong widgetId, float r, float g, float b, float a);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterHotReloadCompleteCallbackDelegate(IntPtr callbackPtr);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetGamePreviewExtentDelegate(uint width, uint height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetCameraParamsDelegate(float yaw, float pitch, float x, float y, float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool IsPreviewWindowSelectedDelegate(IntPtr handle, ulong widgetId);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPreviewWindowSelectedDelegate(IntPtr handle, ulong widgetId, bool selected);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPreviewWindowEditModeDelegate(IntPtr handle, ulong widgetId, bool editMode);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListDelegate(IntPtr handle, float spacing, uint orientation);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListInParentDelegate(IntPtr handle, ulong parentId, float spacing, uint orientation);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListItemDelegate(IntPtr handle, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListItemInParentDelegate(IntPtr handle, ulong parentId, string text, uint showBorder);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ListItemSetTextDelegate(IntPtr handle, ulong widgetId, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ListItemSetFontSizeDelegate(IntPtr handle, ulong widgetId, float fontSize);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateDropdownDelegate(IntPtr handle, ulong parentId, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DropdownSetOptionsDelegate(IntPtr handle, ulong widgetId, string options, ulong count);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DropdownSetSelectedDelegate(IntPtr handle, ulong widgetId, ulong index);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong DropdownGetSelectedDelegate(IntPtr handle, ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DropdownSetOnSelectThunkPtrDelegate(IntPtr handle, ulong widgetId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DropdownSelectCallbackDelegate(ulong widgetId, ulong index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateInputFieldDelegate(IntPtr handle, ulong parentId, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void InputFieldSetTextDelegate(IntPtr handle, ulong widgetId, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int InputFieldGetTextDelegate(IntPtr handle, ulong widgetId, IntPtr buffer, int bufferSize);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void InputFieldSetOnChangeThunkPtrDelegate(IntPtr handle, ulong widgetId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void InputFieldChangeCallbackDelegate(ulong widgetId, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void InputFieldSetPlaceholderDelegate(IntPtr handle, ulong widgetId, string placeholder);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateTabWidgetDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint TabWidgetAddTabDelegate(IntPtr handle, ulong tabWidgetId, string title, ulong contentWidgetId, bool closable);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TabWidgetSetActiveDelegate(IntPtr handle, ulong tabWidgetId, ulong index);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong TabWidgetGetActiveDelegate(IntPtr handle, ulong tabWidgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TabWidgetRemoveTabDelegate(IntPtr handle, ulong tabWidgetId, ulong index);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TabWidgetSetOnSelectThunkPtrDelegate(IntPtr handle, ulong tabWidgetId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TabWidgetSetOnCloseThunkPtrDelegate(IntPtr handle, ulong tabWidgetId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong TabWidgetGetTabCountDelegate(IntPtr handle, ulong tabWidgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TabSelectCallbackDelegate(ulong widgetId, ulong index);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TabCloseCallbackDelegate(ulong widgetId, ulong index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateTreeViewDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong TreeViewAddNodeDelegate(IntPtr handle, ulong treeViewId, ulong parentNodeId, string text, ulong userData, bool hasChildren);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeViewRemoveNodeDelegate(IntPtr handle, ulong treeViewId, ulong nodeId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeViewSetSelectedDelegate(IntPtr handle, ulong treeViewId, ulong nodeId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong TreeViewGetSelectedDelegate(IntPtr handle, ulong treeViewId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeViewExpandNodeDelegate(IntPtr handle, ulong treeViewId, ulong nodeId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeViewCollapseNodeDelegate(IntPtr handle, ulong treeViewId, ulong nodeId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeViewSetOnSelectThunkPtrDelegate(IntPtr handle, ulong treeViewId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeNodeSelectCallbackDelegate(ulong widgetId, ulong userData);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeViewSetOnToggleThunkPtrDelegate(IntPtr handle, ulong treeViewId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeNodeToggleCallbackDelegate(ulong nodeId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool TreeViewIsNodeExpandedDelegate(IntPtr handle, ulong treeViewId, ulong nodeId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void PopupMenuClickCallbackDelegate(ulong widgetId, int actionId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GridViewClickCallbackDelegate(ulong widgetId, int index, ulong userData);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DialogResultCallbackDelegate(ulong dialogId, int result);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserSelectCallbackDelegate(ulong browserId, string path);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserDoubleClickCallbackDelegate(ulong browserId, string path);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeNodeSetTextDelegate(IntPtr handle, ulong nodeId, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong TreeNodeGetUserDataDelegate(IntPtr handle, ulong nodeId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TreeViewClearSelectionDelegate(IntPtr handle, ulong treeViewId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreatePopupMenuDelegate(IntPtr handle, ulong parentId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void PopupMenuAddItemDelegate(IntPtr handle, ulong menuId, string text, string shortcut, int actionId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void PopupMenuAddSeparatorDelegate(IntPtr handle, ulong menuId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void PopupMenuShowDelegate(IntPtr handle, ulong menuId, float x, float y);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void PopupMenuHideDelegate(IntPtr handle, ulong menuId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool PopupMenuIsVisibleDelegate(IntPtr handle, ulong menuId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void PopupMenuSetOnClickThunkPtrDelegate(IntPtr handle, ulong menuId, IntPtr callbackPtr);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateGridViewDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height, float cellSize);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint GridViewAddItemDelegate(IntPtr handle, ulong gridId, string label, ulong userData);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GridViewRemoveItemDelegate(IntPtr handle, ulong gridId, int index);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GridViewSetSelectedDelegate(IntPtr handle, ulong gridId, int index);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int GridViewGetSelectedDelegate(IntPtr handle, ulong gridId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong GridViewGetSelectedUserDataDelegate(IntPtr handle, ulong gridId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GridViewClearDelegate(IntPtr handle, ulong gridId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int GridViewItemCountDelegate(IntPtr handle, ulong gridId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GridViewSetOnClickThunkPtrDelegate(IntPtr handle, ulong gridId, IntPtr callbackPtr);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateDialogDelegate(IntPtr handle, ulong parentId, string title, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DialogSetContentDelegate(IntPtr handle, ulong dialogId, ulong contentId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DialogAddButtonDelegate(IntPtr handle, ulong dialogId, string text, int action);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DialogShowDelegate(IntPtr handle, ulong dialogId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DialogHideDelegate(IntPtr handle, ulong dialogId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool DialogIsVisibleDelegate(IntPtr handle, ulong dialogId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int DialogGetResultDelegate(IntPtr handle, ulong dialogId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DialogSetOnResultThunkPtrDelegate(IntPtr handle, ulong dialogId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateFileBrowserDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height, string initialPath);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserSetPathDelegate(IntPtr handle, ulong browserId, string path);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserSetFilterDelegate(IntPtr handle, ulong browserId, string filter);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserNavigateUpDelegate(IntPtr handle, ulong browserId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserRefreshDelegate(IntPtr handle, ulong browserId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool FileBrowserGetSelectedPathDelegate(IntPtr handle, ulong browserId, IntPtr buffer, int size);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool FileBrowserGetCurrentPathDelegate(IntPtr handle, ulong browserId, IntPtr buffer, int size);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserSetOnSelectThunkPtrDelegate(IntPtr handle, ulong browserId, IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void FileBrowserSetOnDoubleClickThunkPtrDelegate(IntPtr handle, ulong browserId, IntPtr callbackPtr);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateCheckboxDelegate(IntPtr handle, ulong parentId, float width, float height);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateCheckboxInParentDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void CheckboxSetCheckedDelegate(IntPtr handle, ulong widgetId, uint isChecked);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint CheckboxGetCheckedDelegate(IntPtr handle, ulong widgetId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void CheckboxSetOnChangeThunkPtrDelegate(IntPtr handle, ulong widgetId, IntPtr callbackPtr);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void CheckboxChangeCallbackDelegate(ulong widgetId, bool isChecked);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void CheckboxSetTextDelegate(IntPtr handle, ulong widgetId, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetWidgetLayerDelegate(IntPtr handle, ulong widgetId, uint layer);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint GetWidgetLayerDelegate(IntPtr handle, ulong widgetId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate IntPtr SceneCreateDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneDestroyDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneCreateCubeDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneCreatePlaneDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneCreateDirectionalLightDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneAttachScriptDelegate(IntPtr scene, ulong entityId, string scriptPath, string className);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSetGameStateDelegate(IntPtr scene, int state);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int SceneGetGameStateDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong ScenePickEntityDelegate(IntPtr scene, float ox, float oy, float oz, float dx, float dy, float dz);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSelectEntityDelegate(IntPtr scene, ulong entityId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneUpdateDelegate(IntPtr scene, float deltaTime);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetRendererGameStateDelegate(int state);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int GetRendererGameStateDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetEntityTransformDelegate(float px, float py, float pz,
                                                         float rx, float ry, float rz, float rw,
                                                         float sx, float sy, float sz);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetEntityAngleDelegate(float angle);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate float GetEntityAngleDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneGetEntityPositionDelegate(IntPtr scene, ulong entityId, out float x, out float y, out float z);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneGetEntityRotationDelegate(IntPtr scene, ulong entityId, out float x, out float y, out float z, out float w);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneGetEntityScaleDelegate(IntPtr scene, ulong entityId, out float x, out float y, out float z);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSetEntityPositionDelegate(IntPtr scene, ulong entityId, float x, float y, float z);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSetEntityScaleDelegate(IntPtr scene, ulong entityId, float x, float y, float z);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneRotateEntityDelegate(IntPtr scene, ulong entityId, float angleDegrees);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSetEntityNameDelegate(IntPtr scene, ulong entityId, string name);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int SceneGetEntityNameDelegate(IntPtr scene, ulong entityId, IntPtr buffer, int bufferSize);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetSelectedEntityDelegate(ulong entityId, bool selected);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneAttachScriptBindingDelegate(IntPtr scene, ulong entityId, string scriptPath, string className);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneRemoveScriptBindingDelegate(IntPtr scene, ulong entityId, ulong scriptIndex);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneGetScriptBindingCountDelegate(IntPtr scene, ulong entityId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool SceneGetScriptBindingInfoDelegate(IntPtr scene, ulong entityId, ulong index, IntPtr pathBuffer, int pathBufferSize, IntPtr classBuffer, int classBufferSize, out bool enabled);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSetScriptBindingEnabledDelegate(IntPtr scene, ulong entityId, ulong index, bool enabled);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneCreateEntityDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneGetEntityCountDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneGetEntityIdDelegate(IntPtr scene, ulong index);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneRemoveEntityDelegate(IntPtr scene, ulong entityId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint EntityGetPropertyCountDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate IntPtr EntityGetPropertyNameDelegate(uint index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint EntityGetPropertyTypeDelegate(uint index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate IntPtr EntityGetPropertyCategoryDelegate(uint index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool EntityGetPropertyReadOnlyDelegate(uint index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool EntityGetPropertyValueFloat3Delegate(IntPtr scene, ulong entityId, IntPtr propertyName, out float x, out float y, out float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool EntitySetPropertyValueFloat3Delegate(IntPtr scene, ulong entityId, IntPtr propertyName, float x, float y, float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool EntityGetPropertyValueFloatDelegate(IntPtr scene, ulong entityId, IntPtr propertyName, out float value);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool EntitySetPropertyValueFloatDelegate(IntPtr scene, ulong entityId, IntPtr propertyName, float value);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint EntityGetPropertyValueStringDelegate(IntPtr scene, ulong entityId, IntPtr propertyName, IntPtr outBuf, uint bufLen);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool EntitySetPropertyValueStringDelegate(IntPtr scene, ulong entityId, IntPtr propertyName, IntPtr value);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int AssetLibraryGetCategoryCountDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool AssetLibraryGetCategoryNameDelegate(int index, IntPtr buffer, int bufferSize);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int AssetLibraryGetAssetCountDelegate(int categoryIndex);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool AssetLibraryGetAssetInfoDelegate(int categoryIndex, int assetIndex, 
            out ulong id, IntPtr nameBuffer, int nameSize, out uint type, IntPtr descBuffer, int descSize);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong AssetLibraryCreateEntityFromTemplateDelegate(IntPtr scene, ulong templateId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong AssetLibraryCreateMeshEntityDelegate(IntPtr scene, uint meshType);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectCreateNewDelegate([MarshalAs(UnmanagedType.LPStr)] string name, [MarshalAs(UnmanagedType.LPStr)] string path);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectLoadDelegate([MarshalAs(UnmanagedType.LPStr)] string path);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectSaveDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectGetNameDelegate(IntPtr buffer, int size);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectGetPathDelegate(IntPtr buffer, int size);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int ProjectGetEntityCountDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectIsLoadedDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ProjectSyncToSceneDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ProjectSyncFromSceneDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectGetSettingsDelegate(out uint width, out uint height, out uint fps);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectSetSettingsDelegate(uint width, uint height, uint fps);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectGetEntityInfoDelegate(ulong entityId, IntPtr nameBuffer, int nameSize,
            IntPtr posBuffer, IntPtr rotBuffer, IntPtr scaleBuffer, IntPtr meshTypeBuffer, int meshTypeSize);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectAddEntityDelegate(ulong entityId, [MarshalAs(UnmanagedType.LPStr)] string name,
            float posX, float posY, float posZ, float rotX, float rotY, float rotZ,
            float scaleX, float scaleY, float scaleZ, [MarshalAs(UnmanagedType.LPStr)] string meshType);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool ProjectRemoveEntityDelegate(ulong entityId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate(ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ResizeCallbackDelegate(float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GlobalClickCallbackDelegate(float x, float y);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void UpdateCallbackDelegate(float deltaTime);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterGlobalClickDelegate(IntPtr callbackPtr);
        public delegate void RegisterKeyDelegate(IntPtr callbackPtr);
        public delegate void RegisterMouseMoveDelegate(IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterUpdateDelegate(IntPtr callbackPtr);

[StructLayout(LayoutKind.Sequential)]
        public struct FfiContext
        {
            public IntPtr ui_get_primary_button_id;
            public IntPtr ui_set_primary_button_id;
            public IntPtr ui_widget_set_text;
            public IntPtr ui_button_set_on_click_thunk_ptr;
            public IntPtr ui_register_update_thunk_ptr;
            public IntPtr ui_register_resize_thunk_ptr;
            public IntPtr ui_register_global_click_thunk_ptr;
            public IntPtr ui_register_key_thunk_ptr;
            public IntPtr ui_register_mouse_move_thunk_ptr;
            public IntPtr ui_trigger_resize;
            public IntPtr ui_get_screen_size;
            public IntPtr ui_set_content_scale;
            public IntPtr ui_get_content_scale;
            public IntPtr ui_create_button;
            public IntPtr ui_create_label;
            public IntPtr ui_create_panel;
            public IntPtr ui_create_vstack;
            public IntPtr ui_create_vstack_in_parent;
            public IntPtr ui_create_hstack;
            public IntPtr ui_create_hstack_in_parent;
            public IntPtr ui_create_button_in_parent;
            public IntPtr ui_create_label_in_parent;
            public IntPtr ui_create_panel_in_parent;
            public IntPtr ui_create_preview_window;
            public IntPtr ui_set_preview_texture;
            public IntPtr ui_get_root_id;
            public IntPtr ui_set_widget_layout;
            public IntPtr ui_widget_set_position;
            public IntPtr ui_widget_set_size;
            public IntPtr ui_widget_set_layer;
            public IntPtr ui_widget_get_layer;
            public IntPtr ui_remove_widget;
            public IntPtr ui_create_text_edit;
            public IntPtr ui_create_text_edit_in_parent;
            public IntPtr ui_text_edit_set_text;
            public IntPtr ui_text_edit_insert_char;
            public IntPtr ui_text_edit_delete_char;
            public IntPtr ui_text_edit_get_text_len;
            public IntPtr ui_text_edit_get_text;
            public IntPtr ui_text_edit_show_line_numbers;
            public IntPtr ui_trigger_hot_reload;
            public IntPtr ui_set_game_preview_extent;
            public IntPtr ui_set_camera_params;
            public IntPtr ui_is_preview_window_selected;
            public IntPtr ui_set_preview_window_selected;
            public IntPtr ui_set_preview_window_edit_mode;
            public IntPtr ui_create_list;
            public IntPtr ui_create_list_in_parent;
            public IntPtr ui_create_list_item;
            public IntPtr ui_create_list_item_in_parent;
            public IntPtr ui_list_item_set_text;
            public IntPtr ui_list_item_set_font_size;
            public IntPtr ui_create_dropdown;
            public IntPtr ui_dropdown_set_options;
            public IntPtr ui_dropdown_set_selected;
            public IntPtr ui_dropdown_get_selected;
            public IntPtr ui_dropdown_set_on_select_thunk_ptr;
            public IntPtr ui_create_input_field;
            public IntPtr ui_input_field_set_text;
            public IntPtr ui_input_field_get_text;
            public IntPtr ui_input_field_set_on_change_thunk_ptr;
            public IntPtr ui_input_field_set_placeholder;
            public IntPtr ui_create_checkbox;
            public IntPtr ui_create_checkbox_in_parent;
            public IntPtr ui_checkbox_set_checked;
            public IntPtr ui_checkbox_get_checked;
            public IntPtr ui_checkbox_set_on_change_thunk_ptr;
            public IntPtr ui_checkbox_set_text;
            public IntPtr ui_create_tab_widget;
            public IntPtr ui_tab_widget_add_tab;
            public IntPtr ui_tab_widget_set_active;
            public IntPtr ui_tab_widget_get_active;
            public IntPtr ui_tab_widget_remove_tab;
            public IntPtr ui_tab_widget_set_on_select_thunk_ptr;
            public IntPtr ui_tab_widget_set_on_close_thunk_ptr;
            public IntPtr ui_tab_widget_get_tab_count;
            public IntPtr ui_create_tree_view;
            public IntPtr ui_tree_view_add_node;
            public IntPtr ui_tree_view_remove_node;
            public IntPtr ui_tree_view_set_selected;
            public IntPtr ui_tree_view_get_selected;
            public IntPtr ui_tree_view_expand_node;
            public IntPtr ui_tree_view_collapse_node;
            public IntPtr ui_tree_view_set_on_select_thunk_ptr;
            public IntPtr ui_tree_view_set_on_toggle_thunk_ptr;
            public IntPtr ui_tree_view_is_node_expanded;
            public IntPtr ui_tree_node_set_text;
            public IntPtr ui_tree_node_get_user_data;
            public IntPtr ui_tree_view_clear_selection;
            public IntPtr ui_create_popup_menu;
            public IntPtr ui_popup_menu_add_item;
            public IntPtr ui_popup_menu_add_separator;
            public IntPtr ui_popup_menu_show;
            public IntPtr ui_popup_menu_hide;
            public IntPtr ui_popup_menu_is_visible;
            public IntPtr ui_popup_menu_set_on_click_thunk_ptr;
            public IntPtr ui_create_grid_view;
            public IntPtr ui_grid_view_add_item;
            public IntPtr ui_grid_view_remove_item;
            public IntPtr ui_grid_view_set_selected;
            public IntPtr ui_grid_view_get_selected;
            public IntPtr ui_grid_view_get_selected_user_data;
            public IntPtr ui_grid_view_clear;
            public IntPtr ui_grid_view_item_count;
            public IntPtr ui_grid_view_set_on_click_thunk_ptr;
            public IntPtr ui_create_dialog;
            public IntPtr ui_dialog_set_content;
            public IntPtr ui_dialog_add_button;
            public IntPtr ui_dialog_show;
            public IntPtr ui_dialog_hide;
            public IntPtr ui_dialog_is_visible;
            public IntPtr ui_dialog_get_result;
            public IntPtr ui_dialog_set_on_result_thunk_ptr;
            public IntPtr ui_create_file_browser;
            public IntPtr ui_file_browser_set_path;
            public IntPtr ui_file_browser_set_filter;
            public IntPtr ui_file_browser_navigate_up;
            public IntPtr ui_file_browser_refresh;
            public IntPtr ui_file_browser_get_selected_path;
            public IntPtr ui_file_browser_get_current_path;
            public IntPtr ui_file_browser_set_on_select_thunk_ptr;
            public IntPtr ui_file_browser_set_on_double_click_thunk_ptr;
            public IntPtr scene_create;
            public IntPtr scene_destroy;
            public IntPtr scene_create_cube;
            public IntPtr scene_create_plane;
            public IntPtr scene_create_directional_light;
            public IntPtr scene_attach_script;
            public IntPtr scene_set_game_state;
            public IntPtr scene_get_game_state;
            public IntPtr scene_pick_entity;
            public IntPtr scene_select_entity;
            public IntPtr scene_update;
            public IntPtr set_renderer_game_state;
            public IntPtr get_renderer_game_state;
            public IntPtr set_entity_transform;
            public IntPtr set_entity_angle;
            public IntPtr get_entity_angle;
            public IntPtr scene_get_entity_position;
            public IntPtr scene_get_entity_rotation;
            public IntPtr scene_get_entity_scale;
            public IntPtr scene_set_entity_position;
            public IntPtr scene_set_entity_scale;
            public IntPtr scene_rotate_entity;
            public IntPtr scene_set_entity_name;
            public IntPtr scene_get_entity_name;
            public IntPtr set_selected_entity;
            public IntPtr scene_attach_script_binding;
            public IntPtr scene_remove_script_binding;
            public IntPtr scene_get_script_binding_count;
            public IntPtr scene_get_script_binding_info;
            public IntPtr scene_set_script_binding_enabled;
            public IntPtr scene_create_entity;
            public IntPtr scene_get_entity_count;
            public IntPtr scene_get_entity_id;
            public IntPtr scene_remove_entity;
            public IntPtr ui_entity_get_property_count;
            public IntPtr ui_entity_get_property_name;
            public IntPtr ui_entity_get_property_type;
            public IntPtr ui_entity_get_property_category;
            public IntPtr ui_entity_get_property_read_only;
            public IntPtr ui_entity_get_property_value_float3;
            public IntPtr ui_entity_set_property_value_float3;
            public IntPtr ui_entity_get_property_value_float;
            public IntPtr ui_entity_set_property_value_float;
            public IntPtr ui_entity_get_property_value_string;
            public IntPtr ui_entity_set_property_value_string;
            public IntPtr widget_tree_ptr;
            public IntPtr dfx_handle;
            public IntPtr dfx_log;
            public IntPtr dfx_trace_begin;
            public IntPtr dfx_trace_end;
            public IntPtr dfx_set_counter;
            public IntPtr dfx_perf_begin_frame;
            public IntPtr dfx_perf_end_frame;
            public IntPtr set_status_text;
            public IntPtr on_hot_reload_complete;
            public IntPtr ui_debug_print_widget_tree;
            public IntPtr ui_widget_set_flex_expand;
            public IntPtr ui_widget_set_cross_axis_fill;
            public IntPtr ui_widget_set_background_color;
            public IntPtr asset_library_get_category_count;
            public IntPtr asset_library_get_category_name;
            public IntPtr asset_library_get_asset_count;
            public IntPtr asset_library_get_asset_info;
            public IntPtr asset_library_create_entity_from_template;
            public IntPtr asset_library_create_mesh_entity;
            public IntPtr project_create_new;
            public IntPtr project_load;
            public IntPtr project_save;
            public IntPtr project_get_name;
            public IntPtr project_get_path;
            public IntPtr project_get_entity_count;
            public IntPtr project_is_loaded;
            public IntPtr project_sync_to_scene;
            public IntPtr project_sync_from_scene;
            public IntPtr project_get_settings;
            public IntPtr project_set_settings;
            public IntPtr project_get_entity_info;
            public IntPtr project_add_entity;
            public IntPtr project_remove_entity;
        }

        public static void InitFromContext(IntPtr contextPtr)
        {
            _ffi = Marshal.PtrToStructure<FfiContext>(contextPtr);
            _widgetTree = _ffi.widget_tree_ptr;
            
            if (_ffi.dfx_handle != IntPtr.Zero)
            {
                Log.Init(_ffi.dfx_handle);
                Log.SetFunctionPointers(_ffi.dfx_log, _ffi.dfx_trace_begin, _ffi.dfx_trace_end);
                Log.SetPerfFunctionPointers(_ffi.dfx_set_counter, _ffi.dfx_perf_begin_frame, _ffi.dfx_perf_end_frame);
            }
        }

        public static void GetScreenSize(out float width, out float height)
        {
            if (_ffi.ui_get_screen_size == IntPtr.Zero)
            {
                width = 800f;
                height = 600f;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetScreenSizeDelegate>(_ffi.ui_get_screen_size);
            func(out width, out height);
        }

        public static float GetContentScale()
        {
            if (_ffi.ui_get_content_scale == IntPtr.Zero)
            {
                return 1.0f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetContentScaleDelegate>(_ffi.ui_get_content_scale);
            return func();
        }

        public static void SetContentScale(float scale)
        {
            if (_ffi.ui_set_content_scale == IntPtr.Zero)
            {
                Log.Error("C#", "SetContentScale函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetContentScaleDelegate>(_ffi.ui_set_content_scale);
            func(scale);
        }

public static void RegisterResizeCallback(ResizeCallbackDelegate callback)
        {
            _savedResizeCallback = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_resize_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterResizeThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterResizeDelegate>(_ffi.ui_register_resize_thunk_ptr);
            func(callbackPtr);
        }
        
        public static void RegisterGlobalClickCallback(GlobalClickCallbackDelegate callback)
        {
            _savedGlobalClickCallback = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_global_click_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterGlobalClickThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterGlobalClickDelegate>(_ffi.ui_register_global_click_thunk_ptr);
            func(callbackPtr);
        }

        public static void RegisterKeyCallback(KeyCallbackDelegate callback)
        {
            _savedKeyCallback = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_key_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterKeyThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterKeyDelegate>(_ffi.ui_register_key_thunk_ptr);
            func(callbackPtr);
        }

        public static void RegisterMouseMoveCallback(MouseMoveCallbackDelegate callback)
        {
            _savedMouseMoveCallback = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_mouse_move_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterMouseMoveThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterMouseMoveDelegate>(_ffi.ui_register_mouse_move_thunk_ptr);
            func(callbackPtr);
        }

        private static OnHotReloadCompleteDelegate _savedHotReloadCompleteCallback;

        public static void RegisterHotReloadCompleteCallback(OnHotReloadCompleteDelegate callback)
        {
            _savedHotReloadCompleteCallback = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
        }

        public static void RegisterUpdateCallback(UpdateCallbackDelegate callback)
        {
            _savedUpdateCallback = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_update_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterUpdateThunkPtr函数指针为空");
                return;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<RegisterUpdateDelegate>(_ffi.ui_register_update_thunk_ptr);
            func(callbackPtr);
        }

        // Property reflection API
        public static uint EntityGetPropertyCount()
        {
            if (_ffi.ui_entity_get_property_count == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyCountDelegate>(_ffi.ui_entity_get_property_count);
            return func();
        }

        public static string EntityGetPropertyName(uint index)
        {
            if (_ffi.ui_entity_get_property_name == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyName函数指针为空");
                return "";
            }
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyNameDelegate>(_ffi.ui_entity_get_property_name);
            IntPtr namePtr = func(index);
            return namePtr != IntPtr.Zero ? Marshal.PtrToStringAnsi(namePtr) ?? "" : "";
        }

        public static uint EntityGetPropertyType(uint index)
        {
            if (_ffi.ui_entity_get_property_type == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyType函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyTypeDelegate>(_ffi.ui_entity_get_property_type);
            return func(index);
        }

        public static string EntityGetPropertyCategory(uint index)
        {
            if (_ffi.ui_entity_get_property_category == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyCategory函数指针为空");
                return "";
            }
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyCategoryDelegate>(_ffi.ui_entity_get_property_category);
            IntPtr catPtr = func(index);
            return catPtr != IntPtr.Zero ? Marshal.PtrToStringAnsi(catPtr) ?? "" : "";
        }

        public static bool EntityGetPropertyReadOnly(uint index)
        {
            if (_ffi.ui_entity_get_property_read_only == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyReadOnly函数指针为空");
                return true;
            }
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyReadOnlyDelegate>(_ffi.ui_entity_get_property_read_only);
            return func(index);
        }

        public static bool EntityGetPropertyValueFloat3(IntPtr scene, ulong entityId, string propertyName, out float x, out float y, out float z)
        {
            if (_ffi.ui_entity_get_property_value_float3 == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyValueFloat3函数指针为空");
                x = 0; y = 0; z = 0;
                return false;
            }
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(propertyName);
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyValueFloat3Delegate>(_ffi.ui_entity_get_property_value_float3);
            bool result = func(scene, entityId, namePtr, out x, out y, out z);
            Marshal.FreeHGlobal(namePtr);
            return result;
        }

        public static bool EntitySetPropertyValueFloat3(IntPtr scene, ulong entityId, string propertyName, float x, float y, float z)
        {
            if (_ffi.ui_entity_set_property_value_float3 == IntPtr.Zero)
            {
                Log.Error("C#", "EntitySetPropertyValueFloat3函数指针为空");
                return false;
            }
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(propertyName);
            var func = Marshal.GetDelegateForFunctionPointer<EntitySetPropertyValueFloat3Delegate>(_ffi.ui_entity_set_property_value_float3);
            bool result = func(scene, entityId, namePtr, x, y, z);
            Marshal.FreeHGlobal(namePtr);
            return result;
        }

        public static bool EntityGetPropertyValueFloat(IntPtr scene, ulong entityId, string propertyName, out float value)
        {
            if (_ffi.ui_entity_get_property_value_float == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyValueFloat函数指针为空");
                value = 0;
                return false;
            }
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(propertyName);
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyValueFloatDelegate>(_ffi.ui_entity_get_property_value_float);
            bool result = func(scene, entityId, namePtr, out value);
            Marshal.FreeHGlobal(namePtr);
            return result;
        }

        public static bool EntitySetPropertyValueFloat(IntPtr scene, ulong entityId, string propertyName, float value)
        {
            if (_ffi.ui_entity_set_property_value_float == IntPtr.Zero)
            {
                Log.Error("C#", "EntitySetPropertyValueFloat函数指针为空");
                return false;
            }
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(propertyName);
            var func = Marshal.GetDelegateForFunctionPointer<EntitySetPropertyValueFloatDelegate>(_ffi.ui_entity_set_property_value_float);
            bool result = func(scene, entityId, namePtr, value);
            Marshal.FreeHGlobal(namePtr);
            return result;
        }

        public static string EntityGetPropertyValueString(IntPtr scene, ulong entityId, string propertyName)
        {
            if (_ffi.ui_entity_get_property_value_string == IntPtr.Zero)
            {
                Log.Error("C#", "EntityGetPropertyValueString函数指针为空");
                return "";
            }
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(propertyName);
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<EntityGetPropertyValueStringDelegate>(_ffi.ui_entity_get_property_value_string);
            uint len = func(scene, entityId, namePtr, buffer, 256);
            Marshal.FreeHGlobal(namePtr);
            string result = len > 0 ? Marshal.PtrToStringAnsi(buffer, (int)len) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }

        public static bool EntitySetPropertyValueString(IntPtr scene, ulong entityId, string propertyName, string value)
        {
            if (_ffi.ui_entity_set_property_value_string == IntPtr.Zero)
            {
                Log.Error("C#", "EntitySetPropertyValueString函数指针为空");
                return false;
            }
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(propertyName);
            IntPtr valuePtr = Marshal.StringToHGlobalAnsi(value);
            var func = Marshal.GetDelegateForFunctionPointer<EntitySetPropertyValueStringDelegate>(_ffi.ui_entity_set_property_value_string);
            bool result = func(scene, entityId, namePtr, valuePtr);
            Marshal.FreeHGlobal(namePtr);
            Marshal.FreeHGlobal(valuePtr);
            return result;
        }
    }
}