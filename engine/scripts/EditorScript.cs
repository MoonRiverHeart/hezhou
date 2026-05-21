using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static Panel _toolbar;
        private static HStack _toolbarButtons;
        private static Panel _projectPanel;
        private static VStack _projectTree;
        private static Panel _assetPanel;
        private static VStack _assetList;
        private static Panel _previewPanel;
        private static ulong _previewWindowId;
        private static Panel _propertiesPanel;
        private static VStack _propsList;
        private static TabWidget _propsTabWidget;
        private static ulong _transformTabContentId;
        private static ulong _scriptsTabContentId;
        private static ulong _selectedEntityId = 0;
        
        private static bool _propertiesDirty = false;
        private static ulong _lastPropertiesEntityId = 0;
        private static int _lastScriptBindingCount = -1;
        
        private static ulong _nameInputFieldId;
        private static ulong _posXInputFieldId;
        private static ulong _posYInputFieldId;
        private static ulong _posZInputFieldId;
        private static ulong _rotXInputFieldId;
        private static ulong _rotYInputFieldId;
        private static ulong _rotZInputFieldId;
        private static ulong _scaleXInputFieldId;
        private static ulong _scaleYInputFieldId;
        private static ulong _scaleZInputFieldId;
        
        private static List<string> _availableScripts = new List<string>();
        private static ulong _scriptDropdownId;
        private static ulong _addScriptBtnId;
        private static ulong _scriptsListContainerId;
        private static ulong _createEntityBtnId;
        private static Dictionary<ulong, int> _removeScriptBtnIndices = new Dictionary<ulong, int>();
        
        private static UI.InputFieldChangeCallbackDelegate _nameInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _posXInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _posYInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _posZInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _rotXInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _rotYInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _rotZInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _scaleXInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _scaleYInputCallback;
        private static UI.InputFieldChangeCallbackDelegate _scaleZInputCallback;
        private static Panel _statusBar;
        private static List _statusItems;
        private static ListItem _fpsItem;
        private static ListItem _statusItem;
        private static ListItem _projectItem;
        
        private static ulong _runButtonId;
        
        private static ulong _fileMenuId;
        private static ulong _openMenuId;
        private static ulong _saveMenuId;
        
        private static ulong _projectTreeViewId;
        private static ulong _assetsNodeId;
        private static ulong _scenesNodeId;
        private static ulong _scriptsNodeId;
        private static ulong _entitiesNodeId;
        private static Dictionary<ulong, ulong> _entityNodeMap = new Dictionary<ulong, ulong>();
        
        private static ulong _assetGridViewId;
        
        private static UI.PopupMenuClickCallbackDelegate _fileMenuClickCallback;
        private static UI.PopupMenuClickCallbackDelegate _openMenuClickCallback;
        private static UI.PopupMenuClickCallbackDelegate _saveMenuClickCallback;
        private static UI.TreeNodeSelectCallbackDelegate _treeNodeSelectCallback;
        private static UI.GridViewClickCallbackDelegate _gridViewClickCallback;
        
        private static Panel _scriptEditorPanel;
        private static ulong _scriptTextEditId;
        private static Label _scriptEditorLabel;
        private static bool _scriptEditorVisible = false;
        private static Button _toggleEditorBtn;
        
        private static float _screenWidth = 1280f;
        private static float _screenHeight = 720f;
        private static float _contentScale = 1.0f;
        
        private static string _currentDirectory = "scripts";
        private static Dictionary<ulong, string> _fileItemPaths = new Dictionary<ulong, string>();
        private static Dictionary<ulong, string> _dirItemPaths = new Dictionary<ulong, string>();

        private static bool _previewSelected = false;
        
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
        
        private static bool _mouseDragging = false;
        private static float _lastMouseX = 0f;
        private static float _lastMouseY = 0f;
        
        private static bool _keyLeftPressed = false;
        private static bool _keyRightPressed = false;
        private static bool _keyUpPressed = false;
        private static bool _keyDownPressed = false;

        private const float TOOLBAR_HEIGHT = 40f;
        private const float STATUS_BAR_HEIGHT = 40f;
        private const float LEFT_PANEL_WIDTH = 250f;
        private const float RIGHT_PANEL_WIDTH = 250f;
        private const float BOTTOM_PANEL_HEIGHT = 200f;
        
        private static UI.UpdateCallbackDelegate _updateCallback;
        private static UI.ResizeCallbackDelegate _resizeCallback;
        private static UI.GlobalClickCallbackDelegate _globalClickCallback;
        private static UI.KeyCallbackDelegate _keyCallback;
        private static UI.MouseMoveCallbackDelegate _mouseMoveCallback;
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
        
        private static UI.WidgetCallbackDelegate _addScriptClickCallback;
        private static UI.WidgetCallbackDelegate _removeScriptClickCallback;
        private static UI.WidgetCallbackDelegate _createEntityClickCallback;
        private static UI.DropdownSelectCallbackDelegate _scriptDropdownSelectCallback;
        private static UI.OnHotReloadCompleteDelegate _hotReloadCompleteCallback;
        private static UI.TabSelectCallbackDelegate _tabSelectCallback;

        private static bool _isTransitioning = false;
        private static int _selectedScriptIndex = 0;

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate();
        
        public static void Initialize(IntPtr contextPtr)
        {
            UI.InitFromContext(contextPtr);
            Log.Info("Editor", "编辑器初始化开始");
            
            UI.GetScreenSize(out _screenWidth, out _screenHeight);
            _contentScale = UI.GetContentScale();
            
            Log.Info("Editor", $"屏幕尺寸: {_screenWidth}x{_screenHeight}, DPI缩放: {_contentScale}");
            
            _updateCallback = Update;
            _resizeCallback = OnResize;
            _globalClickCallback = OnGlobalClick;
            _keyCallback = OnKey;
            _mouseMoveCallback = OnMouseMove;
            _newClickCallback = OnNewClick;
            _openClickCallback = OnOpenClick;
            _saveClickCallback = OnSaveClick;
            _runClickCallback = OnRunClick;
            _toggleEditorClickCallback = OnToggleEditorClick;
            _hotReloadClickCallback = OnHotReloadClick;
            _newScriptClickCallback = OnNewScriptClick;
            _openInExplorerCallback = OpenInExplorer;
            _backClickCallback = OnBackClick;
            _directoryClickCallback = OnDirectoryClick;
            _fileClickCallback = OnFileClick;
             
_addScriptClickCallback = OnAddScriptClick;
            _removeScriptClickCallback = OnRemoveScriptClick;
            _createEntityClickCallback = OnCreateEntityClick;
            _scriptDropdownSelectCallback = OnScriptDropdownSelect;
_hotReloadCompleteCallback = OnHotReloadComplete;
            _tabSelectCallback = OnTabSelect;
            
            _fileMenuClickCallback = OnFileMenuClick;
            _openMenuClickCallback = OnOpenMenuClick;
            _saveMenuClickCallback = OnSaveMenuClick;
            _treeNodeSelectCallback = OnTreeNodeSelect;
            _gridViewClickCallback = OnGridViewClick;
            
            UI.RegisterUpdateCallback(_updateCallback);
            
            CreateEditorLayout();
            
            UI.RegisterResizeCallback(_resizeCallback);
            UI.RegisterGlobalClickCallback(_globalClickCallback);
            UI.RegisterKeyCallback(_keyCallback);
            UI.RegisterMouseMoveCallback(_mouseMoveCallback);
            
            ScanScripts();
            
            Log.Info("Editor", "编辑器初始化完成");
        }
        
        public static void Update(float deltaTime)
        {
            if (deltaTime > 0 && _fpsItem != null)
            {
                try
                {
                    _fpsItem.Text = $"FPS: {((int)(1000f / deltaTime))}";
                    
                    bool selected = UI.IsPreviewWindowSelected(_previewWindowId);
                    
                    if (selected != _previewSelected)
                    {
                        _previewSelected = selected;
                        
                        if (selected && _gameScene != null && _gameScene.GetGameState() == GameState.Running)
                        {
                            _savedCameraX = _cameraX;
                            _savedCameraY = _cameraY;
                            _savedCameraZ = _cameraZ;
                            _savedCameraYaw = _cameraYaw;
                            _savedCameraPitch = _cameraPitch;
                        }
                        
                        UpdateStatusBar();
                    }
                    
                    if (_previewSelected && _gameScene != null && _gameScene.GetGameState() == GameState.Running)
                    {
                        float speed = 2f * (deltaTime / 1000f);
                        
                        float sinYaw = (float)Math.Sin(_cameraYaw);
                        float cosYaw = (float)Math.Cos(_cameraYaw);
                        float forwardX = sinYaw;
                        float forwardZ = -cosYaw;
                        float rightX = cosYaw;
                        float rightZ = sinYaw;
                        
                        if (_keyUpPressed)
                        {
                            _cameraX += speed * forwardX;
                            _cameraZ += speed * forwardZ;
                        }
                        if (_keyDownPressed)
                        {
                            _cameraX -= speed * forwardX;
                            _cameraZ -= speed * forwardZ;
                        }
                        if (_keyLeftPressed)
                        {
                            _cameraX -= speed * rightX;
                            _cameraZ -= speed * rightZ;
                        }
                        if (_keyRightPressed)
                        {
                            _cameraX += speed * rightX;
                            _cameraZ += speed * rightZ;
                        }
                        
                        UI.SetCameraParams(_cameraYaw, _cameraPitch, _cameraX, _cameraY, _cameraZ);
                        
                        _gameScene.Update(deltaTime / 1000f);
                    }
                    else
                    {
                        UI.SetCameraParams(0f, 0f, 0f, 0f, 3f);
                    }
                }
                catch (Exception ex)
                {
                    Log.Error("Editor", $"Update error: {ex.Message}");
                }
            }
        }
        
        private static void UpdateStatusBar()
        {
            if (_statusItem == null) return;
            
            if (_previewSelected)
            {
                _statusItem.Text = "按ESC退出Game模式";
            }
            else
            {
                _statusItem.Text = "状态: 就绪";
            }
        }
    }
}