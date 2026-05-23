using System;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        // === Layout Constants ===
        private const float TOOLBAR_HEIGHT = 40f;
        private const float STATUS_BAR_HEIGHT = 40f;
        private const float LEFT_PANEL_WIDTH = 250f;
        private const float RIGHT_PANEL_WIDTH = 250f;
        private const float BOTTOM_PANEL_HEIGHT = 200f;

        // === UI Widget References ===
        private static Panel _toolbar;
        private static HStack _toolbarButtons;
        private static Panel _projectPanel;
        private static Panel _assetPanel;
        private static VStack _assetList;
        private static Panel _previewPanel;
        private static ulong _previewWindowId;
        private static Panel _propertiesPanel;
        private static VStack _propsList;
        private static TabWidget _propsTabWidget;
        private static ulong _transformTabContentId;
        private static ulong _scriptsTabContentId;

        // === Entity Selection State ===
        private static ulong _selectedEntityId = 0;
        private static bool _propertiesDirty = false;
        private static ulong _lastPropertiesEntityId = 0;
        private static int _lastScriptBindingCount = -1;

        // === Property Panel State ===
        private static List<PropertyDescriptor> _propertyDescriptors = new List<PropertyDescriptor>();
        private static Dictionary<ulong, PropertyChangeCallback> _propertyCallbacks = new Dictionary<ulong, PropertyChangeCallback>();
        private static Dictionary<ulong, UI.InputFieldChangeCallbackDelegate> _propertyInputFieldCallbacks = new Dictionary<ulong, UI.InputFieldChangeCallbackDelegate>();

        // === Script Management State ===
        private static List<string> _availableScripts = new List<string>();
        private static ulong _scriptDropdownId;
        private static ulong _addScriptBtnId;
        private static ulong _scriptsListContainerId;
        private static List<ulong> _scriptRowIds = new List<ulong>();
        private static ulong _createEntityBtnId;
        private static Dictionary<ulong, int> _removeScriptBtnIndices = new Dictionary<ulong, int>();
        private static Dictionary<ulong, int> _scriptToggleBtnIndices = new Dictionary<ulong, int>();

        // === Status Bar State ===
        private static Panel _statusBar;
        private static List _statusItems;
        private static ListItem _fpsItem;
        private static ListItem _statusItem;
        private static ListItem _projectItem;

        // === Toolbar State ===
        private static ulong _runButtonId;
        private static ulong _pauseButtonId;
        private static ulong _fileMenuId;
        private static ulong _openMenuId;
        private static ulong _saveMenuId;
        private static ulong[] _menuBarLabelIds = new ulong[3];

        // === Project Tree State ===
        private static ulong _projectTreeViewId;
        private static ulong _assetsNodeId;
        private static ulong _scenesNodeId;
        private static ulong _scriptsNodeId;
        private static ulong _entitiesNodeId;
        private static Dictionary<ulong, ulong> _entityNodeMap = new Dictionary<ulong, ulong>();
        private static HashSet<string> _expandedNodeNames = new HashSet<string>();
        private static Dictionary<ulong, string> _nodeIdToName = new Dictionary<ulong, string>();

        // === Asset Grid State ===
        private static ulong _assetGridViewId;

        // === Popup Menu Callbacks ===
        private static UI.PopupMenuClickCallbackDelegate _fileMenuClickCallback;
        private static UI.PopupMenuClickCallbackDelegate _openMenuClickCallback;
        private static UI.PopupMenuClickCallbackDelegate _saveMenuClickCallback;
        private static UI.TreeNodeSelectCallbackDelegate _treeNodeSelectCallback;
        private static UI.TreeNodeRightClickCallbackDelegate _treeNodeRightClickCallback;
        private static UI.GridViewClickCallbackDelegate _gridViewClickCallback;
        private static UI.FileBrowserSelectCallbackDelegate _fileBrowserSelectCallback;
        private static UI.DialogResultCallbackDelegate _workingDirectoryDialogResultCallback;
        
        // === Delete Confirmation Dialog State ===
        private static ulong _deleteConfirmDialogId;
        private static UI.DialogResultCallbackDelegate _deleteConfirmDialogResultCallback;
        
        // === Open/Save Dialog State ===
        private static ulong _openSceneDialogId;
        private static ulong _openSceneFileBrowserId;
        private static UI.DialogResultCallbackDelegate _openSceneDialogResultCallback;
        private static ulong _openProjectDialogId;
        private static ulong _openProjectFileBrowserId;
        private static UI.DialogResultCallbackDelegate _openProjectDialogResultCallback;
        private static ulong _saveAsDialogId;
        private static ulong _saveAsFileBrowserId;
        private static UI.DialogResultCallbackDelegate _saveAsDialogResultCallback;

        // === Script Editor State ===
        private static Panel _scriptEditorPanel;
        private static ulong _scriptTextEditId;
        private static Label _scriptEditorLabel;
        private static bool _scriptEditorVisible = false;
        private static Button _toggleEditorBtn;

        // === Screen State ===
        private static float _screenWidth = 1280f;
        private static float _screenHeight = 720f;
        private static float _contentScale = 1.0f;

        // === File Browser State ===
        private static string _currentDirectory = "scripts";
        private static Dictionary<ulong, string> _fileItemPaths = new Dictionary<ulong, string>();
        private static Dictionary<ulong, string> _dirItemPaths = new Dictionary<ulong, string>();
        private static bool _previewSelected = false;

        // === Directory TreeView State ===
        private static ulong _directoryTreeViewId;
        private static ulong _directoryRootNodeId;
        private static ulong _directoryBackNodeId;

        // === Working Directory Dialog State ===
        private static bool _workingDirectorySet = false;
        private static ulong _workingDirectoryDialogId;
        private static ulong _workingDirectoryFileBrowserId;

        // === Camera State ===
        private static float _savedCameraX = 0f;
        private static float _savedCameraY = 0f;
        private static float _savedCameraZ = 3f;
        private static float _savedCameraYaw = 0f;
        private static float _savedCameraPitch = 0f;

        private static float _cameraX = 0f;
        private static float _cameraY = 0f;
        private static float _cameraZ = 3f;
private static float _cameraYaw = 0f;
        private static float _cameraPitch = 0f;

        // === Orbit Camera State (Editing mode) ===
        private static float _orbitYaw = 0.3f;     // initial slight angle to show cube
        private static float _orbitPitch = -0.3f;   // slight downward look
        private static float _orbitDistance = 5f;    // distance from target
        private static float _orbitTargetX = 0f;
        private static float _orbitTargetY = 0f;
        private static float _orbitTargetZ = 0f;

        private static bool _mouseDragging = false;
        private static float _lastMouseX = 0f;
        private static float _lastMouseY = 0f;

        // === Key Press State ===
        private static bool _keyLeftPressed = false;
        private static bool _keyRightPressed = false;
        private static bool _keyUpPressed = false;
        private static bool _keyDownPressed = false;

        // === Core Callback Delegates ===
        private static UI.UpdateCallbackDelegate _updateCallback;
        private static UI.ResizeCallbackDelegate _resizeCallback;
        private static UI.GlobalClickCallbackDelegate _globalClickCallback;
        private static UI.KeyCallbackDelegate _keyCallback;
        private static UI.MouseMoveCallbackDelegate _mouseMoveCallback;
        private static UI.MouseWheelCallbackDelegate _mouseWheelCallback;
        private static UI.WidgetCallbackDelegate _newClickCallback;
        private static UI.WidgetCallbackDelegate _openClickCallback;
        private static UI.WidgetCallbackDelegate _saveClickCallback;
        private static UI.WidgetCallbackDelegate _runClickCallback;
        private static UI.WidgetCallbackDelegate _toggleEditorClickCallback;
        private static UI.WidgetCallbackDelegate _hotReloadClickCallback;
        private static UI.WidgetCallbackDelegate _newScriptClickCallback;
        private static UI.WidgetCallbackDelegate _openInExplorerCallback;
        private static UI.WidgetCallbackDelegate _backClickCallback;
        private static UI.WidgetCallbackDelegate _directoryClickCallback;
        private static UI.WidgetCallbackDelegate _fileClickCallback;
        private static Scene _gameScene;
        private static ulong _testCubeId;

        // === Script Callback Delegates ===
        private static UI.WidgetCallbackDelegate _addScriptClickCallback;
        private static UI.WidgetCallbackDelegate _removeScriptClickCallback;
        private static UI.WidgetCallbackDelegate _createEntityClickCallback;
        private static UI.WidgetCallbackDelegate _pauseClickCallback;
        private static UI.WidgetCallbackDelegate _scriptToggleClickCallback;
        private static UI.DropdownSelectCallbackDelegate _scriptDropdownSelectCallback;
        private static UI.OnHotReloadCompleteDelegate _hotReloadCompleteCallback;
        private static UI.TabSelectCallbackDelegate _tabSelectCallback;

        // === Tree Toggle Callback ===
        private static UI.TreeNodeToggleCallbackDelegate _treeNodeToggleCallback;

        // === Transition State ===
        private static bool _isTransitioning = false;
        private static int _selectedScriptIndex = 0;

        // === Property Descriptor (Model Type) ===
        // PropertyDescriptor caches reflection metadata and widget IDs for each property
        private struct PropertyDescriptor
        {
            public string Name;
            public uint Type;       // 0=Float, 1=Float3, 2=String, 3=Bool, 4=Int, 5=Enum
            public string Category;
            public bool ReadOnly;
            public ulong LabelId;   // Label showing property display name
            public ulong[] WidgetIds; // Float3: [xId, yId, zId]; String/Int: [inputId]; ReadOnly: [valueLabelId]
        }

        // === Property Change Callback Delegate (Model Type) ===
        // New delegate type for property change callbacks (prevents GC when stored in dictionary)
        public delegate void PropertyChangeCallback(ulong widgetId, string text);
    }
}