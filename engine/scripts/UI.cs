using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Hezhou
{
    public static class UI
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
        private static Dictionary<ulong, PopupMenuClickCallbackDelegate> _popupMenuCallbacks = new Dictionary<ulong, PopupMenuClickCallbackDelegate>();
        private static Dictionary<ulong, GridViewClickCallbackDelegate> _gridViewCallbacks = new Dictionary<ulong, GridViewClickCallbackDelegate>();
        private static Dictionary<ulong, DialogResultCallbackDelegate> _dialogCallbacks = new Dictionary<ulong, DialogResultCallbackDelegate>();
        private static Dictionary<ulong, FileBrowserSelectCallbackDelegate> _fileBrowserSelectCallbacks = new Dictionary<ulong, FileBrowserSelectCallbackDelegate>();
        private static Dictionary<ulong, FileBrowserDoubleClickCallbackDelegate> _fileBrowserDoubleClickCallbacks = new Dictionary<ulong, FileBrowserDoubleClickCallbackDelegate>();

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
            public IntPtr widget_tree_ptr;
            public IntPtr dfx_handle;
            public IntPtr dfx_log;
            public IntPtr dfx_trace_begin;
            public IntPtr dfx_trace_end;
            public IntPtr set_status_text;
            public IntPtr on_hot_reload_complete;
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
            public IntPtr ui_debug_print_widget_tree;
        }

        public static void InitFromContext(IntPtr contextPtr)
        {
            _ffi = Marshal.PtrToStructure<FfiContext>(contextPtr);
            _widgetTree = _ffi.widget_tree_ptr;
            
            if (_ffi.dfx_handle != IntPtr.Zero)
            {
                Log.Init(_ffi.dfx_handle);
                Log.SetFunctionPointers(_ffi.dfx_log, _ffi.dfx_trace_begin, _ffi.dfx_trace_end);
            }
            
            Log.Info("C#", "FfiContext初始化成功");
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

        public static ulong CreateVStack(float spacing = 8f)
        {
            if (_ffi.ui_create_vstack == IntPtr.Zero)
            {
                Log.Error("C#", "CreateVStack函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateVStackDelegate>(_ffi.ui_create_vstack);
            return func(_widgetTree, spacing);
        }

        public static ulong CreateVStack(ulong parentId, float spacing = 8f)
        {
            if (_ffi.ui_create_vstack_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateVStackInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateVStackInParentDelegate>(_ffi.ui_create_vstack_in_parent);
            return func(_widgetTree, parentId, spacing);
        }

        public static ulong CreateHStack(float spacing = 8f)
        {
            if (_ffi.ui_create_hstack == IntPtr.Zero)
            {
                Log.Error("C#", "CreateHStack函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateHStackDelegate>(_ffi.ui_create_hstack);
            return func(_widgetTree, spacing);
        }

        public static ulong CreateHStack(ulong parentId, float spacing = 8f)
        {
            if (_ffi.ui_create_hstack_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateHStackInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateHStackInParentDelegate>(_ffi.ui_create_hstack_in_parent);
            return func(_widgetTree, parentId, spacing);
        }

        public static ulong CreateButton(ulong parentId, float width, float height, string text)
        {
            if (_ffi.ui_create_button_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateButtonInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateButtonInParentDelegate>(_ffi.ui_create_button_in_parent);
            return func(_widgetTree, parentId, width, height, text);
        }

        public static ulong CreateLabel(ulong parentId, float width, float height, string text)
        {
            if (_ffi.ui_create_label_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateLabelInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateLabelInParentDelegate>(_ffi.ui_create_label_in_parent);
            return func(_widgetTree, parentId, width, height, text);
        }

        public static ulong CreateLabel(ulong parentId, float x, float y, float width, float height, string text)
        {
            ulong id = CreateLabel(parentId, width, height, text);
            SetWidgetLayout(id, x, y, width, height);
            return id;
        }

        public static ulong CreatePreviewWindow(ulong parentId, float x, float y, float width, float height, ulong textureId = 1)
        {
            if (_ffi.ui_create_preview_window == IntPtr.Zero)
            {
                Log.Error("C#", "CreatePreviewWindow函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreatePreviewWindowDelegate>(_ffi.ui_create_preview_window);
            return func(_widgetTree, parentId, x, y, width, height, textureId);
        }

        public static void SetPreviewTexture(ulong widgetId, ulong textureId)
        {
            if (_ffi.ui_set_preview_texture == IntPtr.Zero)
            {
                Log.Error("C#", "SetPreviewTexture函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetPreviewTextureDelegate>(_ffi.ui_set_preview_texture);
            func(_widgetTree, widgetId, textureId);
        }

        public static ulong CreatePanel(ulong parentId, float x, float y, float width, float height, float r = 0.2f, float g = 0.2f, float b = 0.2f, float a = 1.0f)
        {
            if (_ffi.ui_create_panel_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreatePanelInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreatePanelInParentDelegate>(_ffi.ui_create_panel_in_parent);
            return func(_widgetTree, parentId, x, y, width, height, r, g, b, a);
        }

        public static ulong CreateTextEdit(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_text_edit_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateTextEditInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateTextEditInParentDelegate>(_ffi.ui_create_text_edit_in_parent);
            return func(_widgetTree, parentId, width, height);
        }

        public static void TextEditSetText(ulong widgetId, string text)
        {
            if (_ffi.ui_text_edit_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "TextEditSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TextEditSetTextDelegate>(_ffi.ui_text_edit_set_text);
            func(_widgetTree, widgetId, text);
        }
        
        public static string TextEditGetText(ulong widgetId)
        {
            if (_ffi.ui_text_edit_get_text_len == IntPtr.Zero || _ffi.ui_text_edit_get_text == IntPtr.Zero)
            {
                Log.Error("C#", "TextEditGetText函数指针为空");
                return "";
            }
            
            var getLenFunc = Marshal.GetDelegateForFunctionPointer<TextEditGetTextLenDelegate>(_ffi.ui_text_edit_get_text_len);
            int len = getLenFunc(_widgetTree, widgetId);
            
            if (len == 0) return "";
            
            IntPtr buffer = Marshal.AllocHGlobal(len + 1);
            var getTextFunc = Marshal.GetDelegateForFunctionPointer<TextEditGetTextDelegate>(_ffi.ui_text_edit_get_text);
            getTextFunc(_widgetTree, widgetId, buffer, len + 1);
            
            string result = Marshal.PtrToStringAnsi(buffer);
            Marshal.FreeHGlobal(buffer);
            
            return result ?? "";
        }
        
        public static void TriggerHotReload()
        {
            if (_ffi.ui_trigger_hot_reload == IntPtr.Zero)
            {
                Log.Error("C#", "TriggerHotReload函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TriggerHotReloadDelegate>(_ffi.ui_trigger_hot_reload);
            func();
        }

        public static void SetStatusText(string status)
        {
            if (_ffi.set_status_text == IntPtr.Zero)
            {
                Log.Error("C#", "SetStatusText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetStatusTextDelegate>(_ffi.set_status_text);
            func(status);
        }

        private static OnHotReloadCompleteDelegate _savedHotReloadCompleteCallback;

        public static void RegisterHotReloadCompleteCallback(OnHotReloadCompleteDelegate callback)
        {
            _savedHotReloadCompleteCallback = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            // Call Rust to register the callback
            try
            {
                // Use internal call through FFI context if available
                Log.Info("C#", "Registering hot reload complete callback");
            }
            catch (Exception ex)
            {
                Log.Error("C#", $"RegisterHotReloadCompleteCallback error: {ex.Message}");
            }
        }

        public static void SetGamePreviewExtent(uint width, uint height)
        {
            if (_ffi.ui_set_game_preview_extent == IntPtr.Zero)
            {
                Log.Error("C#", "SetGamePreviewExtent函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetGamePreviewExtentDelegate>(_ffi.ui_set_game_preview_extent);
            func(width, height);
            Log.Info("C#", $"Game preview extent set to {width}x{height}");
        }

        public static void SetCameraParams(float yaw, float pitch, float x, float y, float z)
        {
            if (_ffi.ui_set_camera_params == IntPtr.Zero)
            {
                Log.Error("C#", "SetCameraParams函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetCameraParamsDelegate>(_ffi.ui_set_camera_params);
            func(yaw, pitch, x, y, z);
            Log.Info("C#", $"Camera params set: yaw={yaw}, pitch={pitch}, pos=({x}, {y}, {z})");
        }

        public static bool IsPreviewWindowSelected(ulong widgetId)
        {
            if (_ffi.ui_is_preview_window_selected == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<IsPreviewWindowSelectedDelegate>(_ffi.ui_is_preview_window_selected);
            return func(_widgetTree, widgetId);
        }

public static void SetPreviewWindowSelected(ulong widgetId, bool selected)
        {
            if (_ffi.ui_set_preview_window_selected == IntPtr.Zero)
            {
                Log.Error("C#", "SetPreviewWindowSelected函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetPreviewWindowSelectedDelegate>(_ffi.ui_set_preview_window_selected);
            func(_widgetTree, widgetId, selected);
        }
        
        public static void SetPreviewWindowEditMode(ulong widgetId, bool editMode)
        {
            if (_ffi.ui_set_preview_window_edit_mode == IntPtr.Zero)
            {
                Log.Error("C#", "SetPreviewWindowEditMode函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetPreviewWindowEditModeDelegate>(_ffi.ui_set_preview_window_edit_mode);
            func(_widgetTree, widgetId, editMode);
            Log.Info("C#", $"PreviewWindow editMode set to: {editMode}");
        }
        
public static ulong GetRootId()
        {
            if (_ffi.ui_get_root_id == IntPtr.Zero)
            {
                Log.Error("C#", "GetRootId函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetRootIdDelegate>(_ffi.ui_get_root_id);
            return func(_widgetTree);
        }
        
        public static void SetTextEditShowLineNumbers(ulong widgetId, bool show)
        {
            if (_ffi.ui_text_edit_show_line_numbers == IntPtr.Zero)
            {
                Log.Error("C#", "SetTextEditShowLineNumbers函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TextEditShowLineNumbersDelegate>(_ffi.ui_text_edit_show_line_numbers);
            func(_widgetTree, widgetId, show);
        }

        public static void SetWidgetLayout(ulong widgetId, float x, float y, float width, float height)
        {
            if (_ffi.ui_set_widget_layout == IntPtr.Zero)
            {
                Log.Error("C#", "SetWidgetLayout函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetWidgetLayoutDelegate>(_ffi.ui_set_widget_layout);
            func(_widgetTree, widgetId, x, y, width, height);
        }
        
        public static void SetWidgetLayer(ulong widgetId, uint layer)
        {
            if (_ffi.ui_widget_set_layer == IntPtr.Zero)
            {
                Log.Error("C#", "SetWidgetLayer函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetWidgetLayerDelegate>(_ffi.ui_widget_set_layer);
            func(_widgetTree, widgetId, layer);
        }

        public static void DebugPrintUITree()
        {
            if (_ffi.ui_debug_print_widget_tree == IntPtr.Zero)
            {
                Log.Error("C#", "DebugPrintUITree函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DebugPrintWidgetTreeDelegate>(_ffi.ui_debug_print_widget_tree);
            func(_widgetTree);
        }
        
        public static uint GetWidgetLayer(ulong widgetId)
        {
            if (_ffi.ui_widget_get_layer == IntPtr.Zero)
            {
                Log.Error("C#", "GetWidgetLayer函数指针为空");
                return 1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetWidgetLayerDelegate>(_ffi.ui_widget_get_layer);
            return func(_widgetTree, widgetId);
        }

        public static void RemoveWidget(ulong widgetId)
        {
            if (_ffi.ui_remove_widget == IntPtr.Zero)
            {
                Log.Error("C#", "RemoveWidget函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RemoveWidgetDelegate>(_ffi.ui_remove_widget);
            func(_widgetTree, widgetId);
        }

        public static void SetText(ulong widgetId, string text)
        {
            if (_ffi.ui_widget_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "SetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetTextDelegate>(_ffi.ui_widget_set_text);
            func(_widgetTree, widgetId, text);
        }

        public static void SetOnClick(ulong widgetId, WidgetCallbackDelegate callback)
        {
            if (_ffi.ui_button_set_on_click_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "SetOnClick函数指针为空");
                return;
            }
            _onclickCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<SetOnClickDelegate>(_ffi.ui_button_set_on_click_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }
        
        public static ulong CreateList(ulong parentId, float spacing = 0f, uint orientation = 0)
        {
            if (_ffi.ui_create_list_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateListInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateListInParentDelegate>(_ffi.ui_create_list_in_parent);
            return func(_widgetTree, parentId, spacing, orientation);
        }
        
        public static ulong CreateListItem(ulong parentId, string text, bool showBorder = false)
        {
            if (_ffi.ui_create_list_item_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateListItemInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateListItemInParentDelegate>(_ffi.ui_create_list_item_in_parent);
            return func(_widgetTree, parentId, text, showBorder ? 1u : 0u);
        }
        
        public static void SetListItemText(ulong widgetId, string text)
        {
            if (_ffi.ui_list_item_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "ListItemSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ListItemSetTextDelegate>(_ffi.ui_list_item_set_text);
            func(_widgetTree, widgetId, text);
        }
        
        public static void SetListItemFontSize(ulong widgetId, float fontSize)
        {
            if (_ffi.ui_list_item_set_font_size == IntPtr.Zero)
            {
                Log.Error("C#", "ListItemSetFontSize函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ListItemSetFontSizeDelegate>(_ffi.ui_list_item_set_font_size);
            func(_widgetTree, widgetId, fontSize);
        }

        public static ulong CreateDropdown(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_dropdown == IntPtr.Zero)
            {
                Log.Error("C#", "CreateDropdown函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateDropdownDelegate>(_ffi.ui_create_dropdown);
            return func(_widgetTree, parentId, width, height);
        }
        
        public static void DropdownSetOptions(ulong widgetId, string[] options)
        {
            if (_ffi.ui_dropdown_set_options == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownSetOptions函数指针为空");
                return;
            }
            string joined = string.Join("\0", options) + "\0";
            var func = Marshal.GetDelegateForFunctionPointer<DropdownSetOptionsDelegate>(_ffi.ui_dropdown_set_options);
            func(_widgetTree, widgetId, joined, (ulong)options.Length);
        }
        
        public static void DropdownSetSelected(ulong widgetId, ulong index)
        {
            if (_ffi.ui_dropdown_set_selected == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownSetSelected函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DropdownSetSelectedDelegate>(_ffi.ui_dropdown_set_selected);
            func(_widgetTree, widgetId, index);
        }
        
        public static ulong DropdownGetSelected(ulong widgetId)
        {
            if (_ffi.ui_dropdown_get_selected == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownGetSelected函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DropdownGetSelectedDelegate>(_ffi.ui_dropdown_get_selected);
            return func(_widgetTree, widgetId);
        }
        
        public static void DropdownSetOnSelect(ulong widgetId, DropdownSelectCallbackDelegate callback)
        {
            if (_ffi.ui_dropdown_set_on_select_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownSetOnSelectThunkPtr函数指针为空");
                return;
            }
            _dropdownCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<DropdownSetOnSelectThunkPtrDelegate>(_ffi.ui_dropdown_set_on_select_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }

        public static ulong CreateInputField(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_input_field == IntPtr.Zero)
            {
                Log.Error("C#", "CreateInputField函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateInputFieldDelegate>(_ffi.ui_create_input_field);
            return func(_widgetTree, parentId, width, height);
        }
        
        public static void InputFieldSetText(ulong widgetId, string text)
        {
            if (_ffi.ui_input_field_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldSetTextDelegate>(_ffi.ui_input_field_set_text);
            func(_widgetTree, widgetId, text);
        }
        
        public static string InputFieldGetText(ulong widgetId)
        {
            if (_ffi.ui_input_field_get_text == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldGetText函数指针为空");
                return "";
            }
            
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldGetTextDelegate>(_ffi.ui_input_field_get_text);
            int len = func(_widgetTree, widgetId, buffer, 256);
            
            string result = len > 0 ? Marshal.PtrToStringAnsi(buffer, len) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            
            return result;
        }
        
        public static void InputFieldSetOnChange(ulong widgetId, InputFieldChangeCallbackDelegate callback)
        {
            if (_ffi.ui_input_field_set_on_change_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldSetOnChangeThunkPtr函数指针为空");
                return;
            }
            _inputFieldCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldSetOnChangeThunkPtrDelegate>(_ffi.ui_input_field_set_on_change_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }
        
        public static void InputFieldSetPlaceholder(ulong widgetId, string placeholder)
        {
            if (_ffi.ui_input_field_set_placeholder == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldSetPlaceholder函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldSetPlaceholderDelegate>(_ffi.ui_input_field_set_placeholder);
            func(_widgetTree, widgetId, placeholder);
        }

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

        public static IntPtr SceneCreate()
        {
            if (_ffi.scene_create == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreate函数指针为空");
                return IntPtr.Zero;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateDelegate>(_ffi.scene_create);
            return func();
        }

        public static void SceneDestroy(IntPtr scene)
        {
            if (_ffi.scene_destroy == IntPtr.Zero)
            {
                Log.Error("C#", "SceneDestroy函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneDestroyDelegate>(_ffi.scene_destroy);
            func(scene);
        }

        public static ulong SceneCreateCube(IntPtr scene)
        {
            if (_ffi.scene_create_cube == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreateCube函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateCubeDelegate>(_ffi.scene_create_cube);
            return func(scene);
        }

        public static void SceneAttachScript(IntPtr scene, ulong entityId, string scriptPath, string className)
        {
            if (_ffi.scene_attach_script == IntPtr.Zero)
            {
                Log.Error("C#", "SceneAttachScript函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneAttachScriptDelegate>(_ffi.scene_attach_script);
            func(scene, entityId, scriptPath, className);
        }

        public static void SceneSetGameState(IntPtr scene, int state)
        {
            if (_ffi.scene_set_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetGameState函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetGameStateDelegate>(_ffi.scene_set_game_state);
            func(scene, state);
        }

        public static int SceneGetGameState(IntPtr scene)
        {
            if (_ffi.scene_get_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetGameState函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetGameStateDelegate>(_ffi.scene_get_game_state);
            return func(scene);
        }

        public static ulong ScenePickEntity(IntPtr scene, float ox, float oy, float oz, float dx, float dy, float dz)
        {
            if (_ffi.scene_pick_entity == IntPtr.Zero)
            {
                Log.Error("C#", "ScenePickEntity函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ScenePickEntityDelegate>(_ffi.scene_pick_entity);
            return func(scene, ox, oy, oz, dx, dy, dz);
        }

        public static void SceneSelectEntity(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_select_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSelectEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSelectEntityDelegate>(_ffi.scene_select_entity);
            func(scene, entityId);
        }

        public static void SceneUpdate(IntPtr scene, float deltaTime)
        {
            if (_ffi.scene_update == IntPtr.Zero)
            {
                Log.Error("C#", "SceneUpdate函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneUpdateDelegate>(_ffi.scene_update);
            func(scene, deltaTime);
        }

        public static void SetRendererGameState(int state)
        {
            if (_ffi.set_renderer_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SetRendererGameState函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetRendererGameStateDelegate>(_ffi.set_renderer_game_state);
            func(state);
        }

        public static int GetRendererGameState()
        {
            if (_ffi.get_renderer_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "GetRendererGameState函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetRendererGameStateDelegate>(_ffi.get_renderer_game_state);
            return func();
        }

        public static void SetEntityTransform(float px, float py, float pz,
                                               float rx, float ry, float rz, float rw,
                                               float sx, float sy, float sz)
        {
            if (_ffi.set_entity_transform == IntPtr.Zero)
            {
                Log.Error("C#", "SetEntityTransform函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetEntityTransformDelegate>(_ffi.set_entity_transform);
            func(px, py, pz, rx, ry, rz, rw, sx, sy, sz);
        }

        public static void SetEntityAngle(float angle)
        {
            if (_ffi.set_entity_angle == IntPtr.Zero)
            {
                Log.Error("C#", "SetEntityAngle函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetEntityAngleDelegate>(_ffi.set_entity_angle);
            func(angle);
        }

        public static float GetEntityAngle()
        {
            if (_ffi.get_entity_angle == IntPtr.Zero)
            {
                Log.Error("C#", "GetEntityAngle函数指针为空");
                return 0.0f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetEntityAngleDelegate>(_ffi.get_entity_angle);
            return func();
        }

        public static void SceneGetEntityPosition(IntPtr scene, ulong entityId, out float x, out float y, out float z)
        {
            if (_ffi.scene_get_entity_position == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityPosition函数指针为空");
                x = 0; y = 0; z = 0;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityPositionDelegate>(_ffi.scene_get_entity_position);
            func(scene, entityId, out x, out y, out z);
        }

        public static void SceneGetEntityRotation(IntPtr scene, ulong entityId, out float x, out float y, out float z, out float w)
        {
            if (_ffi.scene_get_entity_rotation == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityRotation函数指针为空");
                x = 0; y = 0; z = 0; w = 1;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityRotationDelegate>(_ffi.scene_get_entity_rotation);
            func(scene, entityId, out x, out y, out z, out w);
        }

        public static void SceneGetEntityScale(IntPtr scene, ulong entityId, out float x, out float y, out float z)
        {
            if (_ffi.scene_get_entity_scale == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityScale函数指针为空");
                x = 1; y = 1; z = 1;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityScaleDelegate>(_ffi.scene_get_entity_scale);
            func(scene, entityId, out x, out y, out z);
        }

        public static void SceneSetEntityPosition(IntPtr scene, ulong entityId, float x, float y, float z)
        {
            if (_ffi.scene_set_entity_position == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetEntityPosition函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityPositionDelegate>(_ffi.scene_set_entity_position);
            func(scene, entityId, x, y, z);
        }

        public static void SceneSetEntityScale(IntPtr scene, ulong entityId, float x, float y, float z)
        {
            if (_ffi.scene_set_entity_scale == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetEntityScale函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityScaleDelegate>(_ffi.scene_set_entity_scale);
            func(scene, entityId, x, y, z);
        }

        public static void SceneRotateEntity(IntPtr scene, ulong entityId, float angleDegrees)
        {
            if (_ffi.scene_rotate_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRotateEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRotateEntityDelegate>(_ffi.scene_rotate_entity);
            func(scene, entityId, angleDegrees);
        }

        public static void SceneSetEntityName(IntPtr scene, ulong entityId, string name)
        {
            if (_ffi.scene_set_entity_name == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetEntityName函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityNameDelegate>(_ffi.scene_set_entity_name);
            func(scene, entityId, name);
        }

        public static string SceneGetEntityName(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_get_entity_name == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityName函数指针为空");
                return "";
            }
            
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityNameDelegate>(_ffi.scene_get_entity_name);
            int len = func(scene, entityId, buffer, 256);
            
            string result = len > 0 ? Marshal.PtrToStringAnsi(buffer, len) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            
            return result;
        }

        public static void SetSelectedEntity(ulong entityId, bool selected)
        {
            if (_ffi.set_selected_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SetSelectedEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetSelectedEntityDelegate>(_ffi.set_selected_entity);
            func(entityId, selected);
        }
        
        public static void SceneAttachScriptBinding(IntPtr scene, ulong entityId, string scriptPath, string className)
        {
            if (_ffi.scene_attach_script_binding == IntPtr.Zero)
            {
                Log.Error("C#", "SceneAttachScriptBinding函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneAttachScriptBindingDelegate>(_ffi.scene_attach_script_binding);
            func(scene, entityId, scriptPath, className);
        }
        
        public static void SceneRemoveScriptBinding(IntPtr scene, ulong entityId, ulong scriptIndex)
        {
            if (_ffi.scene_remove_script_binding == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRemoveScriptBinding函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRemoveScriptBindingDelegate>(_ffi.scene_remove_script_binding);
            func(scene, entityId, scriptIndex);
        }
        
        public static ulong SceneGetScriptBindingCount(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_get_script_binding_count == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetScriptBindingCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetScriptBindingCountDelegate>(_ffi.scene_get_script_binding_count);
            return func(scene, entityId);
        }
        
        public static ScriptBindingInfo SceneGetScriptBindingInfo(IntPtr scene, ulong entityId, ulong index)
        {
            var info = new ScriptBindingInfo();
            if (_ffi.scene_get_script_binding_info == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetScriptBindingInfo函数指针为空");
                return info;
            }
            
            IntPtr pathBuffer = Marshal.AllocHGlobal(256);
            IntPtr classBuffer = Marshal.AllocHGlobal(256);
            
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetScriptBindingInfoDelegate>(_ffi.scene_get_script_binding_info);
            bool success = func(scene, entityId, index, pathBuffer, 256, classBuffer, 256, out info.Enabled);
            
            if (success)
            {
                info.ScriptPath = Marshal.PtrToStringAnsi(pathBuffer) ?? "";
                info.ClassName = Marshal.PtrToStringAnsi(classBuffer) ?? "";
            }
            
            Marshal.FreeHGlobal(pathBuffer);
            Marshal.FreeHGlobal(classBuffer);
            
            return info;
        }
        
        public static void SceneSetScriptBindingEnabled(IntPtr scene, ulong entityId, ulong index, bool enabled)
        {
            if (_ffi.scene_set_script_binding_enabled == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetScriptBindingEnabled函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetScriptBindingEnabledDelegate>(_ffi.scene_set_script_binding_enabled);
            func(scene, entityId, index, enabled);
        }
        
        public static ulong SceneCreateEntity(IntPtr scene)
        {
            if (_ffi.scene_create_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreateEntity函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateEntityDelegate>(_ffi.scene_create_entity);
            return func(scene);
        }
        
        public static ulong SceneGetEntityCount(IntPtr scene)
        {
            if (_ffi.scene_get_entity_count == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityCountDelegate>(_ffi.scene_get_entity_count);
            return func(scene);
        }
        
        public static ulong SceneGetEntityId(IntPtr scene, ulong index)
        {
            if (_ffi.scene_get_entity_id == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityId函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityIdDelegate>(_ffi.scene_get_entity_id);
            return func(scene, index);
        }
        
        public static void SceneRemoveEntity(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_remove_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRemoveEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRemoveEntityDelegate>(_ffi.scene_remove_entity);
            func(scene, entityId);
        }

        public static int AssetLibraryGetCategoryCount()
        {
            if (_ffi.asset_library_get_category_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetCategoryCountDelegate>(_ffi.asset_library_get_category_count);
            return func();
        }
        
        public static string AssetLibraryGetCategoryName(int index)
        {
            if (_ffi.asset_library_get_category_name == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetCategoryNameDelegate>(_ffi.asset_library_get_category_name);
            bool success = func(index, buffer, 256);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static int AssetLibraryGetAssetCount(int categoryIndex)
        {
            if (_ffi.asset_library_get_asset_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetAssetCountDelegate>(_ffi.asset_library_get_asset_count);
            return func(categoryIndex);
        }
        
        public static AssetInfo AssetLibraryGetAssetInfo(int categoryIndex, int assetIndex)
        {
            var info = new AssetInfo();
            if (_ffi.asset_library_get_asset_info == IntPtr.Zero)
            {
                return info;
            }
            
            IntPtr nameBuffer = Marshal.AllocHGlobal(256);
            IntPtr descBuffer = Marshal.AllocHGlobal(256);
            
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetAssetInfoDelegate>(_ffi.asset_library_get_asset_info);
            bool success = func(categoryIndex, assetIndex, out info.Id, nameBuffer, 256, out uint typeValue, descBuffer, 256);
            
            if (success)
            {
                info.Name = Marshal.PtrToStringAnsi(nameBuffer) ?? "";
                info.Type = (AssetType)typeValue;
                info.Description = Marshal.PtrToStringAnsi(descBuffer) ?? "";
            }
            
            Marshal.FreeHGlobal(nameBuffer);
            Marshal.FreeHGlobal(descBuffer);
            
            return info;
        }
        
        public static ulong AssetLibraryCreateEntityFromTemplate(IntPtr scene, ulong templateId)
        {
            if (_ffi.asset_library_create_entity_from_template == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryCreateEntityFromTemplateDelegate>(_ffi.asset_library_create_entity_from_template);
            return func(scene, templateId);
        }
        
        public static ulong AssetLibraryCreateMeshEntity(IntPtr scene, int meshType)
        {
            if (_ffi.asset_library_create_mesh_entity == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryCreateMeshEntityDelegate>(_ffi.asset_library_create_mesh_entity);
            return func(scene, (uint)meshType);
        }

        public static bool ProjectCreateNew(string name, string path)
        {
            if (_ffi.project_create_new == IntPtr.Zero)
            {
                Log.Error("C#", "ProjectCreateNew函数指针为空");
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectCreateNewDelegate>(_ffi.project_create_new);
            return func(name, path);
        }
        
        public static bool ProjectLoad(string path)
        {
            if (_ffi.project_load == IntPtr.Zero)
            {
                Log.Error("C#", "ProjectLoad函数指针为空");
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectLoadDelegate>(_ffi.project_load);
            return func(path);
        }
        
        public static bool ProjectSave()
        {
            if (_ffi.project_save == IntPtr.Zero)
            {
                Log.Error("C#", "ProjectSave函数指针为空");
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSaveDelegate>(_ffi.project_save);
            return func();
        }
        
        public static string ProjectGetName()
        {
            if (_ffi.project_get_name == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetNameDelegate>(_ffi.project_get_name);
            bool success = func(buffer, 256);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static string ProjectGetPath()
        {
            if (_ffi.project_get_path == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(512);
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetPathDelegate>(_ffi.project_get_path);
            bool success = func(buffer, 512);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static int ProjectGetEntityCount()
        {
            if (_ffi.project_get_entity_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetEntityCountDelegate>(_ffi.project_get_entity_count);
            return func();
        }
        
        public static bool ProjectIsLoaded()
        {
            if (_ffi.project_is_loaded == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectIsLoadedDelegate>(_ffi.project_is_loaded);
            return func();
        }
        
        public static void ProjectSyncToScene(IntPtr scene)
        {
            if (_ffi.project_sync_to_scene == IntPtr.Zero)
            {
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSyncToSceneDelegate>(_ffi.project_sync_to_scene);
            func(scene);
        }
        
        public static void ProjectSyncFromScene(IntPtr scene)
        {
            if (_ffi.project_sync_from_scene == IntPtr.Zero)
            {
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSyncFromSceneDelegate>(_ffi.project_sync_from_scene);
            func(scene);
        }
        
        public static ProjectSettings ProjectGetSettings()
        {
            var settings = new ProjectSettings();
            if (_ffi.project_get_settings == IntPtr.Zero)
            {
                return settings;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetSettingsDelegate>(_ffi.project_get_settings);
            func(out settings.GameWidth, out settings.GameHeight, out settings.Fps);
            return settings;
        }
        
        public static bool ProjectSetSettings(uint width, uint height, uint fps)
        {
            if (_ffi.project_set_settings == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSetSettingsDelegate>(_ffi.project_set_settings);
            return func(width, height, fps);
        }
        
        public static ProjectEntityInfo ProjectGetEntityInfo(ulong entityId)
        {
            var info = new ProjectEntityInfo();
            if (_ffi.project_get_entity_info == IntPtr.Zero)
            {
                return info;
            }
            
            IntPtr nameBuffer = Marshal.AllocHGlobal(256);
            IntPtr posBuffer = Marshal.AllocHGlobal(12);
            IntPtr rotBuffer = Marshal.AllocHGlobal(12);
            IntPtr scaleBuffer = Marshal.AllocHGlobal(12);
            IntPtr meshTypeBuffer = Marshal.AllocHGlobal(64);
            
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetEntityInfoDelegate>(_ffi.project_get_entity_info);
            bool success = func(entityId, nameBuffer, 256, posBuffer, rotBuffer, scaleBuffer, meshTypeBuffer, 64);
            
            if (success)
            {
                info.Id = entityId;
                info.Name = Marshal.PtrToStringAnsi(nameBuffer) ?? "";
                info.MeshType = Marshal.PtrToStringAnsi(meshTypeBuffer) ?? "";
                
                info.Position = new float[3];
                info.Rotation = new float[3];
                info.Scale = new float[3];
                
                Marshal.Copy(posBuffer, info.Position, 0, 3);
                Marshal.Copy(rotBuffer, info.Rotation, 0, 3);
                Marshal.Copy(scaleBuffer, info.Scale, 0, 3);
            }
            
            Marshal.FreeHGlobal(nameBuffer);
            Marshal.FreeHGlobal(posBuffer);
            Marshal.FreeHGlobal(rotBuffer);
            Marshal.FreeHGlobal(scaleBuffer);
            Marshal.FreeHGlobal(meshTypeBuffer);
            
            return info;
        }
        
        public static bool ProjectAddEntity(ulong entityId, string name,
            float posX, float posY, float posZ,
            float rotX, float rotY, float rotZ,
            float scaleX, float scaleY, float scaleZ,
            string meshType)
        {
            if (_ffi.project_add_entity == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectAddEntityDelegate>(_ffi.project_add_entity);
            return func(entityId, name, posX, posY, posZ, rotX, rotY, rotZ, scaleX, scaleY, scaleZ, meshType);
        }
        
        public static bool ProjectRemoveEntity(ulong entityId)
        {
            if (_ffi.project_remove_entity == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectRemoveEntityDelegate>(_ffi.project_remove_entity);
            return func(entityId);
        }

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
    }

    public class VStack
    {
        public ulong Id { get; private set; }
        
        public VStack(ulong parentId, float spacing = 8f)
        {
            Id = UI.CreateVStack(parentId, spacing);
            Log.Info("C#", $"VStack创建成功: id={Id}, parent={parentId}");
        }
        
        public ulong AddButton(float width, float height, string text)
        {
            return UI.CreateButton(Id, width, height, text);
        }
        
        public ulong AddLabel(float width, float height, string text)
        {
            return UI.CreateLabel(Id, width, height, text);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }

    public class HStack
    {
        public ulong Id { get; private set; }
        
        public HStack(ulong parentId, float spacing = 8f)
        {
            Id = UI.CreateHStack(parentId, spacing);
            Log.Info("C#", $"HStack created: id={Id}, parent={parentId}");
        }
        
        public Button AddButton(float width, float height, string text)
        {
            return new Button(Id, width, height, text);
        }
        
        public ulong AddLabel(float width, float height, string text)
        {
            return UI.CreateLabel(Id, width, height, text);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }

    public class Button
    {
        public ulong Id { get; private set; }
        private string _text;
        private UI.WidgetCallbackDelegate _callback;
        
        public Button(ulong parentId, float width, float height, string text)
        {
            _text = text;
            Id = UI.CreateButton(parentId, width, height, text);
            Log.Info("C#", $"Button created: id={Id}, text=\"{text}\"");
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.SetText(Id, _text); }
        }
        
        public void SetOnClick(UI.WidgetCallbackDelegate callback)
        {
            _callback = callback;
            UI.SetOnClick(Id, callback);
        }
    }

    public class Label
    {
        public ulong Id { get; private set; }
        private string _text;
        
        public Label(ulong parentId, float width, float height, string text)
        {
            _text = text;
            Id = UI.CreateLabel(parentId, width, height, text);
            Log.Info("C#", $"Label创建成功: id={Id}, text=\"{text}\"");
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.SetText(Id, _text); }
        }
    }

    public class Panel
    {
        public ulong Id { get; private set; }
        
        public Panel(ulong parentId, float x, float y, float width, float height, float r = 0.2f, float g = 0.2f, float b = 0.2f, float a = 1.0f)
        {
            Id = UI.CreatePanel(parentId, x, y, width, height, r, g, b, a);
            Log.Info("Editor", $"Panel创建成功: id={Id}");
        }
        
        public ulong AddButton(float width, float height, string text)
        {
            return UI.CreateButton(Id, width, height, text);
        }
        
        public ulong AddLabel(float width, float height, string text)
        {
            return UI.CreateLabel(Id, width, height, text);
        }
        
        public ulong AddPanel(float x, float y, float width, float height, float r = 0.2f, float g = 0.2f, float b = 0.2f, float a = 1.0f)
        {
            return UI.CreatePanel(Id, x, y, width, height, r, g, b, a);
        }
        
public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
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
            Log.Info("Editor", $"TabWidget创建成功: id={Id}");
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
    
    public class List
    {
        public ulong Id { get; private set; }
        
        public List(ulong parentId, float spacing = 0f, bool horizontal = true)
        {
            Id = UI.CreateList(parentId, spacing, horizontal ? 0u : 1u);
        }
        
        public ListItem AddItem(string text, bool showBorder = false)
        {
            return new ListItem(Id, text, showBorder);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }
    
    public class ListItem
    {
        public ulong Id { get; private set; }
        private string _text;
        
        public ListItem(ulong parentId, string text, bool showBorder = false)
        {
            _text = text;
            Id = UI.CreateListItem(parentId, text, showBorder);
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.SetListItemText(Id, _text); }
        }
    }
    
    public class Dropdown
    {
        public ulong Id { get; private set; }
        private string[] _options;
        private UI.DropdownSelectCallbackDelegate _callback;
        
        public Dropdown(ulong parentId, float width, float height, string[] options = null)
        {
            Id = UI.CreateDropdown(parentId, width, height);
            _options = options ?? new string[0];
            if (_options.Length > 0)
            {
                UI.DropdownSetOptions(Id, _options);
            }
            Log.Info("C#", $"Dropdown created: id={Id}, options={_options.Length}");
        }
        
        public string[] Options
        {
            get => _options;
            set
            {
                _options = value;
                UI.DropdownSetOptions(Id, _options);
            }
        }
        
        public ulong SelectedIndex
        {
            get => UI.DropdownGetSelected(Id);
            set => UI.DropdownSetSelected(Id, value);
        }
        
        public string SelectedValue => _options.Length > 0 && SelectedIndex < (ulong)_options.Length 
            ? _options[SelectedIndex] : null;
        
        public void SetOnSelect(UI.DropdownSelectCallbackDelegate callback)
        {
            _callback = callback;
            UI.DropdownSetOnSelect(Id, callback);
        }
    }
    
    public class InputField
    {
        public ulong Id { get; private set; }
        private string _text;
        private string _placeholder;
        private UI.InputFieldChangeCallbackDelegate _callback;
        
        public InputField(ulong parentId, float width, float height, string placeholder = "")
        {
            Id = UI.CreateInputField(parentId, width, height);
            _text = "";
            _placeholder = placeholder;
            if (!string.IsNullOrEmpty(placeholder))
            {
                UI.InputFieldSetPlaceholder(Id, placeholder);
            }
            Log.Info("C#", $"InputField created: id={Id}, placeholder=\"{placeholder}\"");
        }
        
        public string Text
        {
            get => UI.InputFieldGetText(Id);
            set
            {
                _text = value;
                UI.InputFieldSetText(Id, value);
            }
        }
        
        public string Placeholder
        {
            get => _placeholder;
            set
            {
                _placeholder = value;
                UI.InputFieldSetPlaceholder(Id, value);
            }
        }
        
        public void SetOnChange(UI.InputFieldChangeCallbackDelegate callback)
        {
            _callback = callback;
            UI.InputFieldSetOnChange(Id, callback);
        }
    }
    
    public class TreeView
    {
        public ulong Id { get; private set; }
        private UI.TreeNodeSelectCallbackDelegate _selectCallback;
        
        public TreeView(ulong parentId, float x, float y, float width, float height)
        {
            Id = UI.CreateTreeView(parentId, x, y, width, height);
            Log.Info("Editor", $"TreeView创建成功: id={Id}");
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
            Log.Info("Editor", $"PopupMenu创建成功: id={Id}");
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
            Log.Info("Editor", $"GridView创建成功: id={Id}, cellSize={cellSize}");
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

    public enum AssetType
    {
        EntityTemplate = 0,
        Texture = 1,
        Material = 2,
        Script = 3
    }
    
    public struct AssetInfo
    {
        public ulong Id;
        public string Name;
        public AssetType Type;
        public string Description;
    }
    
    public struct ProjectSettings
    {
        public uint GameWidth;
        public uint GameHeight;
        public uint Fps;
    }
    
    public struct ProjectEntityInfo
    {
        public ulong Id;
        public string Name;
        public float[] Position;
        public float[] Rotation;
        public float[] Scale;
        public string MeshType;
    }
    
    public static class AssetLibrary
    {
        public static int GetCategoryCount()
        {
            return UI.AssetLibraryGetCategoryCount();
        }
        
        public static string GetCategoryName(int index)
        {
            return UI.AssetLibraryGetCategoryName(index);
        }
        
        public static int GetAssetCount(int categoryIndex)
        {
            return UI.AssetLibraryGetAssetCount(categoryIndex);
        }
        
        public static AssetInfo GetAssetInfo(int categoryIndex, int assetIndex)
        {
            return UI.AssetLibraryGetAssetInfo(categoryIndex, assetIndex);
        }
        
        public static ulong CreateEntityFromTemplate(IntPtr scene, ulong templateId)
        {
            return UI.AssetLibraryCreateEntityFromTemplate(scene, templateId);
        }
        
        public static ulong CreateMeshEntity(IntPtr scene, int meshType)
        {
            return UI.AssetLibraryCreateMeshEntity(scene, meshType);
        }
    }
    
    public static class Project
    {
        public static bool CreateNew(string name, string path)
        {
            return UI.ProjectCreateNew(name, path);
        }
        
        public static bool Load(string path)
        {
            return UI.ProjectLoad(path);
        }
        
        public static bool Save()
        {
            return UI.ProjectSave();
        }
        
        public static string GetName()
        {
            return UI.ProjectGetName();
        }
        
        public static string GetPath()
        {
            return UI.ProjectGetPath();
        }
        
        public static int GetEntityCount()
        {
            return UI.ProjectGetEntityCount();
        }
        
        public static bool IsLoaded()
        {
            return UI.ProjectIsLoaded();
        }
        
        public static void SyncToScene(IntPtr scene)
        {
            UI.ProjectSyncToScene(scene);
        }
        
        public static void SyncFromScene(IntPtr scene)
        {
            UI.ProjectSyncFromScene(scene);
        }
        
        public static ProjectSettings GetSettings()
        {
            return UI.ProjectGetSettings();
        }
        
        public static bool SetSettings(uint width, uint height, uint fps)
        {
            return UI.ProjectSetSettings(width, height, fps);
        }
        
        public static ProjectEntityInfo GetEntityInfo(ulong entityId)
        {
            return UI.ProjectGetEntityInfo(entityId);
        }
        
        public static bool AddEntity(ulong entityId, string name, 
            float posX, float posY, float posZ,
            float rotX, float rotY, float rotZ,
            float scaleX, float scaleY, float scaleZ,
            string meshType)
        {
            return UI.ProjectAddEntity(entityId, name, 
                posX, posY, posZ, 
                rotX, rotY, rotZ,
                scaleX, scaleY, scaleZ, 
                meshType);
        }
        
        public static bool RemoveEntity(ulong entityId)
        {
            return UI.ProjectRemoveEntity(entityId);
        }
    }

    public enum GameState
    {
        Editing = 0,
        Running = 1,
        Paused = 2
    }
    
    public struct ScriptBindingInfo
    {
        public string ScriptPath;
        public string ClassName;
        public bool Enabled;
    }
    
    public class ScriptBinding
    {
        public string ScriptPath;
        public string ClassName;
        public bool Enabled;
    }
    
    public class Entity
    {
        public ulong Id;
        public string Name;
        public List<ScriptBinding> Scripts = new List<ScriptBinding>();
    }

    public class Scene
    {
        private IntPtr _scenePtr;
        private Dictionary<ulong, Entity> _entities = new Dictionary<ulong, Entity>();

        public Scene()
        {
            _scenePtr = UI.SceneCreate();
            if (_scenePtr == IntPtr.Zero)
            {
                throw new Exception("Failed to create scene");
            }
        }

        public IntPtr ScenePtr => _scenePtr;

        public void Destroy()
        {
            if (_scenePtr != IntPtr.Zero)
            {
                UI.SceneDestroy(_scenePtr);
                _scenePtr = IntPtr.Zero;
            }
        }

        public ulong CreateCube()
        {
            return UI.SceneCreateCube(_scenePtr);
        }
        
        public ulong CreateEntity()
        {
            ulong entityId = UI.SceneCreateEntity(_scenePtr);
            if (entityId != 0)
            {
                var entity = new Entity { Id = entityId, Name = $"Entity_{entityId}" };
                _entities[entityId] = entity;
            }
            return entityId;
        }
        
        public Entity GetEntity(ulong entityId)
        {
            if (_entities.TryGetValue(entityId, out var entity))
            {
                return entity;
            }
            return null;
        }
        
        public Dictionary<ulong, Entity>.ValueCollection GetAllEntities()
        {
            return _entities.Values;
        }
        
        public int GetEntityCount()
        {
            return _entities.Count;
        }
        
        public ulong GetEntityId(int index)
        {
            if (index < 0 || index >= _entities.Count)
                return 0;
            
            int i = 0;
            foreach (var key in _entities.Keys)
            {
                if (i == index)
                    return key;
                i++;
            }
            return 0;
        }
        
        public void RemoveEntity(ulong entityId)
        {
            if (_entities.ContainsKey(entityId))
            {
                UI.SceneRemoveEntity(_scenePtr, entityId);
                _entities.Remove(entityId);
            }
        }

        public void AttachScript(ulong entityId, string scriptPath, string className)
        {
            UI.SceneAttachScript(_scenePtr, entityId, scriptPath, className);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                var binding = new ScriptBinding { ScriptPath = scriptPath, ClassName = className, Enabled = true };
                entity.Scripts.Add(binding);
            }
        }
        
        public void AttachScriptBinding(ulong entityId, string scriptPath, string className)
        {
            UI.SceneAttachScriptBinding(_scenePtr, entityId, scriptPath, className);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                var binding = new ScriptBinding { ScriptPath = scriptPath, ClassName = className, Enabled = true };
                entity.Scripts.Add(binding);
            }
        }
        
        public void RemoveScriptBinding(ulong entityId, int index)
        {
            UI.SceneRemoveScriptBinding(_scenePtr, entityId, (ulong)index);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                if (index >= 0 && index < entity.Scripts.Count)
                {
                    entity.Scripts.RemoveAt(index);
                }
            }
        }
        
        public int GetScriptBindingCount(ulong entityId)
        {
            return (int)UI.SceneGetScriptBindingCount(_scenePtr, entityId);
        }
        
        public ScriptBindingInfo GetScriptBindingInfo(ulong entityId, int index)
        {
            return UI.SceneGetScriptBindingInfo(_scenePtr, entityId, (ulong)index);
        }
        
        public void SetScriptBindingEnabled(ulong entityId, int index, bool enabled)
        {
            UI.SceneSetScriptBindingEnabled(_scenePtr, entityId, (ulong)index, enabled);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                if (index >= 0 && index < entity.Scripts.Count)
                {
                    entity.Scripts[index].Enabled = enabled;
                }
            }
        }

        public void SetGameState(GameState state)
        {
            UI.SceneSetGameState(_scenePtr, (int)state);
        }

        public GameState GetGameState()
        {
            return (GameState)UI.SceneGetGameState(_scenePtr);
        }

        public ulong PickEntity(float ox, float oy, float oz, float dx, float dy, float dz)
        {
            return UI.ScenePickEntity(_scenePtr, ox, oy, oz, dx, dy, dz);
        }

        public void SelectEntity(ulong entityId)
        {
            UI.SceneSelectEntity(_scenePtr, entityId);
            UI.SetSelectedEntity(entityId, true);
            Log.Info("Scene", $"Entity {entityId} selected with highlight");
        }

        public void ClearSelection()
        {
            UI.SetSelectedEntity(0, false);
            Log.Info("Scene", "Selection cleared, highlight removed");
        }

        public void Update(float deltaTime)
        {
            UI.SceneUpdate(_scenePtr, deltaTime);
        }
    }
}