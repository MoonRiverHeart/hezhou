using System;
using System.Runtime.InteropServices;
using System.IO;
using System.Diagnostics;
using System.Collections.Generic;

namespace Hezhou
{
    public static class EditorScript
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
        private static ulong _propsEntityLabelId;    // Entity ID label
        private static ulong _propsPositionLabelId;   // Position label
        private static ulong _propsRotationLabelId;   // Rotation label
        private static ulong _propsScaleLabelId;      // Scale label
        private static ulong _selectedEntityId = 0;   // 当前选中的Entity ID
        private static Panel _statusBar;
        private static List _statusItems;
        private static ListItem _fpsItem;
        private static ListItem _statusItem;
        private static ListItem _projectItem;
        
        private static ulong _runButtonId;  // 运行/编辑按钮ID
        
        private static Panel _dropdownMenu;
        private static VStack _menuItems;
        
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
            
            UI.RegisterUpdateCallback(_updateCallback);
            
            CreateEditorLayout();
            
            UI.RegisterResizeCallback(_resizeCallback);
            UI.RegisterGlobalClickCallback(_globalClickCallback);
            UI.RegisterKeyCallback(_keyCallback);
            UI.RegisterMouseMoveCallback(_mouseMoveCallback);
            
            Log.Info("Editor", "编辑器初始化完成");
        }

        private static void OnMouseMove(float x, float y, bool dragging)
        {
            // Running模式下PreviewWindow控制摄像机
            if (_gameScene == null || _gameScene.GetGameState() != GameState.Running)
            {
                _mouseDragging = false;
                return;
            }
            
            if (!dragging || !_previewSelected)
            {
                _mouseDragging = false;
                return;
            }
            
            if (!_mouseDragging)
            {
                _mouseDragging = true;
                _lastMouseX = x;
                _lastMouseY = y;
                return;
            }
            
            float dx = x - _lastMouseX;
            float dy = y - _lastMouseY;
            _lastMouseX = x;
            _lastMouseY = y;
            
            _cameraYaw += dx * 0.01f;
            _cameraPitch += dy * 0.01f;
            
            if (_cameraPitch > 1.5f) _cameraPitch = 1.5f;
            if (_cameraPitch < -1.5f) _cameraPitch = -1.5f;
        }

        private static void OnKey(uint keycode, bool pressed, uint modifiers)
        {
            const uint KEY_ESC = 39;
            const uint KEY_LEFT = 45;
            const uint KEY_RIGHT = 46;
            const uint KEY_UP = 47;
            const uint KEY_DOWN = 48;
            
            bool selected = UI.IsPreviewWindowSelected(_previewWindowId);
            GameState currentState = _gameScene != null ? _gameScene.GetGameState() : GameState.Editing;
            
            // ESC处理 - Running模式下切换到Editing
            if (keycode == KEY_ESC && pressed)
            {
                if (currentState == GameState.Running)
                {
                    // Running模式：切换到Editing
                    _gameScene.SetGameState(GameState.Editing);
                    UI.SetRendererGameState(0);  // Editing
                    UI.SetPreviewWindowEditMode(_previewWindowId, true);  // 橙色边框
                    UI.SetPreviewWindowSelected(_previewWindowId, false);
                    UI.SetText(_runButtonId, "运行");
                    _cameraX = _savedCameraX;
                    _cameraY = _savedCameraY;
                    _cameraZ = _savedCameraZ;
                    _cameraYaw = _savedCameraYaw;
                    _cameraPitch = _savedCameraPitch;
                    _keyLeftPressed = false;
                    _keyRightPressed = false;
                    _keyUpPressed = false;
                    _keyDownPressed = false;
                    _statusItem.Text = "状态: 就绪";
                    Log.Info("Editor", "ESC: Running → Editing");
                }
                else if (selected)
                {
                    // Editing模式 + PreviewWindow选中：取消选中Entity
                    if (_gameScene != null)
                    {
                        _gameScene.ClearSelection();
                        _statusItem.Text = "状态: 就绪";
                        ClearPropertiesPanel();
                        Log.Info("Editor", "ESC: 取消选中Entity（Editing模式）");
                    }
                }
                UpdateStatusBar();
                return;
            }
            
            if (!selected) return;
            
            if (keycode == KEY_LEFT) _keyLeftPressed = pressed;
            if (keycode == KEY_RIGHT) _keyRightPressed = pressed;
            if (keycode == KEY_UP) _keyUpPressed = pressed;
            if (keycode == KEY_DOWN) _keyDownPressed = pressed;
        }

        private static void CreateEditorLayout()
        {
            // Create game scene
            _gameScene = new Scene();
            Log.Info("Editor", $"Scene created: ptr={_gameScene.ScenePtr}");
            
            // Create test cube
            _testCubeId = _gameScene.CreateCube();
            Log.Info("Editor", $"Test cube created: entityId={_testCubeId}");
            
            // Set to editing mode
            _gameScene.SetGameState(GameState.Editing);
            Log.Info("Editor", $"Scene state: {_gameScene.GetGameState()}");
            
            float toolbarY = 0f;
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float bottomY = _screenHeight - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float statusY = _screenHeight - STATUS_BAR_HEIGHT;
            
            float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH;
            float previewX = LEFT_PANEL_WIDTH;

            ulong rootId = UI.GetRootId();
            Log.Info("Editor", $"RootId={rootId}");

            _toolbar = new Panel(rootId, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT, 0.15f, 0.15f, 0.15f, 1.0f);
            _toolbarButtons = new HStack(_toolbar.Id, 10f);
            _toolbarButtons.SetPosition(10f, 5f);
            
            var newBtn = _toolbarButtons.AddButton(100f, 30f, "新建");
            newBtn.SetOnClick(_newClickCallback);
            
            var openBtn = _toolbarButtons.AddButton(100f, 30f, "打开");
            openBtn.SetOnClick(_openClickCallback);
            
            var saveBtn = _toolbarButtons.AddButton(100f, 30f, "保存");
            saveBtn.SetOnClick(_saveClickCallback);
            
            var runBtn = _toolbarButtons.AddButton(100f, 30f, "运行");
            _runButtonId = runBtn.Id;  // 保存按钮ID
            runBtn.SetOnClick(_runClickCallback);
            
            _toggleEditorBtn = new Button(_toolbar.Id, 100f, 30f, "编辑器");
            UI.SetWidgetLayout(_toggleEditorBtn.Id, _screenWidth - 120f, 5f, 100f, 30f);
            _toggleEditorBtn.SetOnClick(_toggleEditorClickCallback);
            
            Log.Info("Editor", "工具栏创建完成");

            _projectPanel = new Panel(rootId, 0, mainY, LEFT_PANEL_WIDTH, mainHeight, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_projectPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "项目结构");
            _projectTree = new VStack(_projectPanel.Id, 5f);
            _projectTree.SetPosition(10f, 40f);
            _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Assets");
            _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Scenes");
            _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "└─ Scripts");
            Log.Info("Editor", "项目结构面板创建完成");

            _assetPanel = new Panel(rootId, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_assetPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "资产管理");
            _assetList = new VStack(_assetPanel.Id, 5f);
            _assetList.SetPosition(10f, 40f);
            _assetList.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "Textures: 0");
            _assetList.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "Models: 0");
            _assetList.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "Scripts: 1");
            Log.Info("Editor", "资产管理面板创建完成");

            _previewPanel = new Panel(rootId, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT, 0.08f, 0.08f, 0.08f, 0.3f);
            UI.CreateLabel(_previewPanel.Id, 10f, 10f, previewWidth - 20f, 25f, "游戏预览");
            
            // 预览窗组件（显示游戏渲染纹理）
            float previewWindowWidth = previewWidth - 20f;
            float previewWindowHeight = mainHeight - 20f;
            _previewWindowId = UI.CreatePreviewWindow(_previewPanel.Id, 10f, 40f, previewWindowWidth, previewWindowHeight, 1);
            
            // 设置Game Pass渲染尺寸匹配PreviewWindow（避免拉伸变形）
            UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
            Log.Info("Editor", $"游戏预览面板创建完成: PreviewWindow={previewWindowWidth}x{previewWindowHeight}, aspect={previewWindowWidth/previewWindowHeight:F2}");

            _propertiesPanel = new Panel(rootId, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_propertiesPanel.Id, 10f, 10f, RIGHT_PANEL_WIDTH - 20f, 25f, "属性编辑");
            _propsList = new VStack(_propertiesPanel.Id, 5f);
            _propsList.SetPosition(10f, 40f);
            
            // 保存Label ID以便后续更新
            _propsEntityLabelId = _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "Entity: 无");
            _propsPositionLabelId = _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "位置: (0, 0, 0)");
            _propsRotationLabelId = _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "旋转: (0°, 0°, 0°)");
            _propsScaleLabelId = _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "缩放: (1, 1, 1)");
            Log.Info("Editor", "属性面板创建完成");

            _statusBar = new Panel(rootId, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT, 0.12f, 0.12f, 0.12f, 1.0f);
            _statusItems = new List(_statusBar.Id, 0f, true);  // horizontal list
            UI.SetWidgetLayout(_statusItems.Id, 10f * _contentScale, 0f, _screenWidth - 20f * _contentScale, STATUS_BAR_HEIGHT);
            
            float fontSize = 14f * _contentScale;
            float itemHeight = STATUS_BAR_HEIGHT - 4f * _contentScale;  // 减去padding
            
            _fpsItem = _statusItems.AddItem("FPS: 0", false);
            UI.SetWidgetLayout(_fpsItem.Id, 0f, 2f * _contentScale, 120f * _contentScale, itemHeight);
            UI.SetListItemFontSize(_fpsItem.Id, fontSize);
            
            _statusItem = _statusItems.AddItem("状态: 就绪", true);
            UI.SetWidgetLayout(_statusItem.Id, 130f * _contentScale, 2f * _contentScale, 150f * _contentScale, itemHeight);
            UI.SetListItemFontSize(_statusItem.Id, fontSize);
            
            _projectItem = _statusItems.AddItem("项目: 未命名", true);
            UI.SetWidgetLayout(_projectItem.Id, 290f * _contentScale, 2f * _contentScale, 150f * _contentScale, itemHeight);
            UI.SetListItemFontSize(_projectItem.Id, fontSize);
            
            Log.Info("Editor", "状态栏创建完成");
        }

        private static void OnResize(float width, float height)
        {
            _screenWidth = width;
            _screenHeight = height;
            Log.Info("Editor", $"窗口resize: {width}x{height}");
            
            UpdateLayout();
        }

        private static void UpdateLayout()
        {
            float toolbarY = 0f;
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float bottomY = _screenHeight - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float statusY = _screenHeight - STATUS_BAR_HEIGHT;
            
            float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH;
            float previewX = LEFT_PANEL_WIDTH;

            if (_toolbar != null)
                UI.SetWidgetLayout(_toolbar.Id, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT);
            if (_toggleEditorBtn != null)
                UI.SetWidgetLayout(_toggleEditorBtn.Id, _screenWidth - 100f, 5f, 80f, 30f);
            if (_projectPanel != null)
                UI.SetWidgetLayout(_projectPanel.Id, 0, mainY, LEFT_PANEL_WIDTH, mainHeight);
            if (_assetPanel != null)
                UI.SetWidgetLayout(_assetPanel.Id, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT);
            if (_previewPanel != null)
                UI.SetWidgetLayout(_previewPanel.Id, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT);
            
            // Update PreviewWindow size and Game Pass extent
            if (_previewWindowId != 0 && _previewPanel != null && !_scriptEditorVisible)
            {
                float previewWindowWidth = previewWidth - 20f;
                float previewWindowHeight = mainHeight - 20f;
                UI.SetWidgetLayout(_previewWindowId, 10f, 40f, previewWindowWidth, previewWindowHeight);
                UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
                Log.Info("Editor", $"PreviewWindow resize: {previewWindowWidth}x{previewWindowHeight}");
            }
            
            if (_propertiesPanel != null)
                UI.SetWidgetLayout(_propertiesPanel.Id, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT);
            if (_statusBar != null)
                UI.SetWidgetLayout(_statusBar.Id, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT);
            
            if (_scriptEditorPanel != null && _scriptEditorVisible)
            {
                float editorWidth = _screenWidth - LEFT_PANEL_WIDTH;
                float editorHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
                UI.SetWidgetLayout(_scriptEditorPanel.Id, LEFT_PANEL_WIDTH, TOOLBAR_HEIGHT, editorWidth, editorHeight);
                if (_scriptTextEditId != 0)
                    UI.SetWidgetLayout(_scriptTextEditId, 10f, 50f, editorWidth - 20f, editorHeight - 50f);
            }
            
            Log.Info("Editor", "布局更新完成");
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
                        
                        // Running模式下保存摄像机状态
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
                    
                    // Pass camera params to shader when preview is selected AND running
                    if (_previewSelected && _gameScene != null && _gameScene.GetGameState() == GameState.Running)
                    {
                        float speed = 2f * (deltaTime / 1000f);  // 2 units/sec
                        
                        // Calculate forward and right vectors based on camera yaw
                        // forward: direction camera is looking at (sin(yaw), 0, -cos(yaw))
                        // right: camera's right direction (cos(yaw), 0, sin(yaw))
                        float sinYaw = (float)Math.Sin(_cameraYaw);
                        float cosYaw = (float)Math.Cos(_cameraYaw);
                        float forwardX = sinYaw;
                        float forwardZ = -cosYaw;
                        float rightX = cosYaw;
                        float rightZ = sinYaw;
                        
                        // Move in camera local space
                        if (_keyUpPressed)    // Forward
                        {
                            _cameraX += speed * forwardX;
                            _cameraZ += speed * forwardZ;
                        }
                        if (_keyDownPressed)  // Backward
                        {
                            _cameraX -= speed * forwardX;
                            _cameraZ -= speed * forwardZ;
                        }
                        if (_keyLeftPressed)  // Left
                        {
                            _cameraX -= speed * rightX;
                            _cameraZ -= speed * rightZ;
                        }
                        if (_keyRightPressed) // Right
                        {
                            _cameraX += speed * rightX;
                            _cameraZ += speed * rightZ;
                        }
                        
                        UI.SetCameraParams(_cameraYaw, _cameraPitch, _cameraX, _cameraY, _cameraZ);
                        
                        // Update scene when running
                        _gameScene.Update(deltaTime / 1000f);
                    }
                    else
                    {
                        // Editing mode or not selected: default camera
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
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate();
        
        private static void OnNewClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"新建\"按钮, id={widgetId}");
            ShowDropdownMenu(10, 45, 
                new string[] { "新建场景", "新建脚本", "新建材质", "新建文件夹" },
                new UI.WidgetCallbackDelegate[] { null, _newScriptClickCallback, null, null });
        }
        
        private static void OnNewScriptClick(ulong widgetId)
        {
            Log.Info("Editor", "创建新脚本...");
            HideDropdownMenu();
            ShowScriptEditor();
        }
        
        private static void ShowScriptEditor()
        {
            if (_scriptEditorVisible) return;
            
            Log.Info("Editor", "ShowScriptEditor开始...");
            
            // 移除preview相关panel，保留左侧目录树
            if (_previewPanel != null)
            {
                Log.Info("Editor", "移除previewPanel...");
                UI.RemoveWidget(_previewPanel.Id);
                _previewPanel = null;
            }
            if (_assetPanel != null)
            {
                Log.Info("Editor", "移除assetPanel...");
                UI.RemoveWidget(_assetPanel.Id);
                _assetPanel = null;
            }
            if (_propertiesPanel != null)
            {
                Log.Info("Editor", "移除propertiesPanel...");
                UI.RemoveWidget(_propertiesPanel.Id);
                _propertiesPanel = null;
            }
            
            ulong rootId = UI.GetRootId();
            
            // 左侧脚本目录树 (保持projectPanel位置)
            float leftWidth = LEFT_PANEL_WIDTH;
            float editorX = LEFT_PANEL_WIDTH;
            float editorY = TOOLBAR_HEIGHT;
            float editorWidth = _screenWidth - LEFT_PANEL_WIDTH;
            float editorHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
            
            // 刷新目录树显示实际文件系统
            RefreshDirectoryTree();
            Log.Info("Editor", "左侧目录树已刷新");
            
            // 右侧编辑区
            _scriptEditorPanel = new Panel(rootId, editorX, editorY, editorWidth, editorHeight, 0.12f, 0.12f, 0.14f, 1.0f);
            
            var hotReloadBtn = new Button(_scriptEditorPanel.Id, 100f, 30f, "Hot Reload");
            UI.SetWidgetLayout(hotReloadBtn.Id, 10f, 10f, 100f, 30f);
            hotReloadBtn.SetOnClick(_hotReloadClickCallback);
            
            _scriptEditorLabel = new Label(_scriptEditorPanel.Id, 200f, 25f, "Script Editor - NewScript.cs");
            UI.SetWidgetLayout(_scriptEditorLabel.Id, 120f, 10f, 300f, 25f);
            
            _scriptTextEditId = UI.CreateTextEdit(_scriptEditorPanel.Id, editorWidth - 20f, editorHeight - 50f);
            UI.SetTextEditShowLineNumbers(_scriptTextEditId, true);
            UI.SetWidgetLayout(_scriptTextEditId, 10f, 50f, editorWidth - 20f, editorHeight - 50f);
            UI.TextEditSetText(_scriptTextEditId, "// NewScript.cs\nusing System;\nusing Hezhou;\n\npublic class NewScript\n{\n    public void Start()\n    {\n        Console.WriteLine(\"NewScript started!\");\n    }\n    \n    public void Update(float deltaTime)\n    {\n        // Update logic here\n    }\n}");
            
            _scriptEditorVisible = true;
            Log.Info("Editor", "Script Editor显示成功");
        }
        
        private static void OnHotReloadClick(ulong widgetId)
        {
            Log.Info("Editor", "Hot Reload triggered!");
            
            if (_scriptTextEditId == 0)
            {
                Log.Error("Editor", "TextEdit not created");
                return;
            }
            
            // 获取脚本内容
            string scriptContent = UI.TextEditGetText(_scriptTextEditId);
            Log.Info("Editor", $"Script content length: {scriptContent.Length}");
            
            // 保存到临时文件
            try
            {
                string tempPath = "scripts/bin/Mono/NewScript.cs";
                System.IO.Directory.CreateDirectory("scripts/bin/Mono");
                System.IO.File.WriteAllText(tempPath, scriptContent);
                Log.Info("Editor", $"Script saved to {tempPath}");
                
                // 编译（覆盖EditorScript.dll）
                var compileProcess = new System.Diagnostics.Process();
                compileProcess.StartInfo.FileName = "C:\\Program Files\\Mono\\bin\\mcs.bat";
                compileProcess.StartInfo.Arguments = $"-target:library -out:scripts/bin/Mono/EditorScript.dll {tempPath} scripts/UI.cs scripts/DFX.cs";
                compileProcess.StartInfo.UseShellExecute = false;
                compileProcess.StartInfo.RedirectStandardOutput = true;
                compileProcess.StartInfo.RedirectStandardError = true;
                compileProcess.StartInfo.CreateNoWindow = true;
                
                compileProcess.Start();
                string output = compileProcess.StandardOutput.ReadToEnd();
                string error = compileProcess.StandardError.ReadToEnd();
                compileProcess.WaitForExit();
                
                if (compileProcess.ExitCode == 0)
                {
                    Log.Info("Editor", "✓ Compilation successful!");
                    Log.Info("Editor", "Output DLL: scripts/tmp/NewScript.dll");
                    if (!string.IsNullOrEmpty(output))
                        Log.Info("Editor", $"Compiler output:\n{output}");
                    
                    // 触发Rust端hot reload
                    Log.Info("Editor", "Triggering hot reload...");
                    UI.TriggerHotReload();
                }
                else
                {
                    Log.Error("Editor", "✗ Compilation failed!");
                    Log.Error("Editor", $"Error:\n{error}");
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", ex.Message);
            }
        }
        
        private static void OnOpenClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"打开\"按钮, id={widgetId}");
            ShowDropdownMenu(100, 45, new string[] { "打开场景", "打开项目", "打开资源" });
        }
        
        private static void OnSaveClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"保存\"按钮, id={widgetId}");
            ShowDropdownMenu(190, 45, new string[] { "保存场景", "保存全部", "另存为..." });
        }
        
        private static void OnRunClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"运行\"按钮, id={widgetId}");
            HideDropdownMenu();
            
            if (_gameScene == null)
            {
                Log.Error("Editor", "Scene未创建!");
                return;
            }
            
            var currentState = _gameScene.GetGameState();
            if (currentState == GameState.Editing)
            {
                _gameScene.SetGameState(GameState.Running);
                UI.SetRendererGameState(1);  // Running
                UI.SetPreviewWindowEditMode(_previewWindowId, false);  // 蓝色边框
                UI.SetText(_runButtonId, "编辑");  // 更新按钮文字
                Log.Info("Editor", "Scene和Renderer切换到Running状态");
                _statusItem.Text = "状态: 运行中";
            }
            else
            {
                _gameScene.SetGameState(GameState.Editing);
                UI.SetRendererGameState(0);  // Editing
                UI.SetPreviewWindowEditMode(_previewWindowId, true);  // 橙色边框
                UI.SetText(_runButtonId, "运行");  // 更新按钮文字
                Log.Info("Editor", "Scene和Renderer切换到Editing状态");
                _statusItem.Text = "状态: 就绪";
            }
        }
        
        private static void OnGlobalClick(float x, float y)
        {
            Log.Info("Editor", $"GlobalClick at ({x}, {y})");
            HideDropdownMenu();
            
            // Editing模式下PreviewWindow点击拾取Entity
            if (_previewWindowId != 0 && UI.IsPreviewWindowSelected(_previewWindowId))
            {
                GameState currentState = _gameScene != null ? _gameScene.GetGameState() : GameState.Editing;
                
                if (currentState == GameState.Editing && _gameScene != null)
                {
                    Log.Info("Editor", "Editing mode - attempting entity pick");
                    
                    // 将屏幕坐标转换为世界空间射线
                    float previewX = LEFT_PANEL_WIDTH + 10f * _contentScale;
                    float previewY = TOOLBAR_HEIGHT + 40f * _contentScale;
                    float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH - 20f * _contentScale;
                    float previewHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT - 50f * _contentScale;
                    
                    float relX = (x - previewX) / previewWidth;
                    float relY = (y - previewY) / previewHeight;
                    
                    Log.Info("Editor", $"PreviewWindow relative click: ({relX}, {relY})");
                    
                    float ndcX = (relX - 0.5f) * 2.0f;
                    float ndcY = (0.5f - relY) * 2.0f;
                    
                    float aspect = previewWidth / previewHeight;
                    float fov = 60.0f;
                    float tanFov = (float)Math.Tan(fov * 0.5f * Math.PI / 180.0f);
                    
                    float dirX = ndcX * tanFov * aspect;
                    float dirY = ndcY * tanFov;
                    float dirZ = -1.0f;
                    
                    float originX = _cameraX;
                    float originY = _cameraY;
                    float originZ = _cameraZ;
                    
                    Log.Info("Editor", $"Ray: origin=({originX}, {originY}, {originZ}), dir=({dirX}, {dirY}, {dirZ})");
                    
                    ulong hitEntity = _gameScene.PickEntity(originX, originY, originZ, dirX, dirY, dirZ);
                    
                    if (hitEntity != 0)
                    {
                        Log.Info("Editor", $"Entity picked: id={hitEntity}");
                        _gameScene.SelectEntity(hitEntity);
                        _statusItem.Text = $"选中Entity: {hitEntity}";
                        UpdatePropertiesPanel(hitEntity);
                    }
                    else
                    {
                        Log.Info("Editor", "No entity hit");
                        _gameScene.ClearSelection();
                        _statusItem.Text = "状态: 就绪";
                        ClearPropertiesPanel();  // 清空属性面板
                    }
                }
                else
                {
                    Log.Info("Editor", "Running mode - camera control");
                }
            }
        }
        
        private static void UpdatePropertiesPanel(ulong entityId)
        {
            if (_propsList == null || _gameScene == null) return;
            
            _selectedEntityId = entityId;
            
            // 更新Entity ID显示
            UI.SetText(_propsEntityLabelId, $"Entity: {entityId}");
            
            // 获取Entity transform
            float px, py, pz;
            UI.SceneGetEntityPosition(_gameScene.ScenePtr, entityId, out px, out py, out pz);
            UI.SetText(_propsPositionLabelId, $"位置: ({px:F2}, {py:F2}, {pz:F2})");
            
            float rx, ry, rz, rw;
            UI.SceneGetEntityRotation(_gameScene.ScenePtr, entityId, out rx, out ry, out rz, out rw);
            // 将四元数转换为欧拉角（简化版本）
            float eulerX = (float)Math.Atan2(2.0 * (rw * rx + ry * rz), 1.0 - 2.0 * (rx * rx + ry * ry)) * 180.0f / (float)Math.PI;
            float eulerY = (float)Math.Asin(2.0 * (rw * ry - rz * rx)) * 180.0f / (float)Math.PI;
            float eulerZ = (float)Math.Atan2(2.0 * (rw * rz + rx * ry), 1.0 - 2.0 * (ry * ry + rz * rz)) * 180.0f / (float)Math.PI;
            UI.SetText(_propsRotationLabelId, $"旋转: ({eulerX:F0}°, {eulerY:F0}°, {eulerZ:F0}°)");
            
            float sx, sy, sz;
            UI.SceneGetEntityScale(_gameScene.ScenePtr, entityId, out sx, out sy, out sz);
            UI.SetText(_propsScaleLabelId, $"缩放: ({sx:F2}, {sy:F2}, {sz:F2})");
            
            Log.Info("Editor", $"Properties panel updated for entity {entityId}: pos=({px}, {py}, {pz})");
        }
        
        private static void ClearPropertiesPanel()
        {
            if (_propsList == null) return;
            
            _selectedEntityId = 0;
            UI.SetText(_propsEntityLabelId, "Entity: 无");
            UI.SetText(_propsPositionLabelId, "位置: (0, 0, 0)");
            UI.SetText(_propsRotationLabelId, "旋转: (0°, 0°, 0°)");
            UI.SetText(_propsScaleLabelId, "缩放: (1, 1, 1)");
            
            Log.Info("Editor", "Properties panel cleared");
        }
        
private static void ShowDropdownMenu(float x, float y, string[] items, UI.WidgetCallbackDelegate[] callbacks)
        {
            HideDropdownMenu();
            
            ulong rootId = UI.GetRootId();
            _dropdownMenu = new Panel(rootId, x, y, 160, items.Length * 35 + 10, 0.25f, 0.25f, 0.25f, 0.95f);
            _menuItems = new VStack(_dropdownMenu.Id, 5f);
            _menuItems.SetPosition(10, 10);
            
            for (int i = 0; i < items.Length; i++)
            {
                ulong btnId = _menuItems.AddButton(140, 30f, items[i]);
                Log.Info("Editor", $"菜单项{i}: '{items[i]}', btnId={btnId}");
                if (i < callbacks.Length && callbacks[i] != null)
                {
                    Log.Info("Editor", $"注册回调: btnId={btnId}");
                    UI.SetOnClick(btnId, callbacks[i]);
                }
            }
            
            Log.Info("Editor", $"显示下拉菜单: {items.Length}项");
        }
        
        private static void ShowDropdownMenu(float x, float y, string[] items)
        {
            ShowDropdownMenu(x, y, items, new UI.WidgetCallbackDelegate[items.Length]);
        }
        
        private static void HideDropdownMenu()
        {
            if (_dropdownMenu != null)
            {
                UI.RemoveWidget(_dropdownMenu.Id);
                _dropdownMenu = null;
                _menuItems = null;
                Log.Info("Editor", "隐藏下拉菜单");
            }
        }
        
        private static bool _isTransitioning = false;  // 防止快速点击
        
        private static void OnToggleEditorClick(ulong widgetId)
        {
            if (_isTransitioning) return;  // 正在切换中，忽略点击
            
            Log.Info("Editor", $"点击\"编辑器\"切换按钮, id={widgetId}");
            _isTransitioning = true;
            
            HideDropdownMenu();
            
            try
            {
                if (_scriptEditorVisible)
                {
                    HideScriptEditor();
                    ShowMainLayout();
                }
                else
                {
                    HideMainLayout();
                    ShowScriptEditor();
                }
            }
            finally
            {
                _isTransitioning = false;
            }
        }
        
        private static void HideScriptEditor()
        {
            if (!_scriptEditorVisible) return;
            
            if (_scriptEditorPanel != null)
            {
                UI.RemoveWidget(_scriptEditorPanel.Id);
                _scriptEditorPanel = null;
            }
            _scriptEditorVisible = false;
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "编辑器";
            }
            
            Log.Info("Editor", "脚本编辑器隐藏");
        }
        
        private static void ShowMainLayout()
        {
            Log.Info("Editor", "显示主界面...");
            
            ulong rootId = UI.GetRootId();
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float bottomY = _screenHeight - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH;
            float previewX = LEFT_PANEL_WIDTH;
            
            // 恢复projectTree内容
            if (_projectPanel != null)
            {
                // 移除旧的projectTree（如果有）
                if (_projectTree != null)
                {
                    UI.RemoveWidget(_projectTree.Id);
                }
                
                // 创建新的projectTree
                _projectTree = new VStack(_projectPanel.Id, 5f);
                _projectTree.SetPosition(10f, 40f);
                _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Assets");
                _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Scenes");
                _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "└─ Scripts");
            }
            
            // 重新创建主界面panels
            if (_previewPanel == null)
            {
                _previewPanel = new Panel(rootId, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT, 0.08f, 0.08f, 0.08f, 0.3f);
                UI.CreateLabel(_previewPanel.Id, 10f, 10f, previewWidth - 20f, 25f, "游戏预览");
                
                // 创建预览窗组件
                float previewWindowWidth = previewWidth - 20f;
                float previewWindowHeight = mainHeight - 20f;
                _previewWindowId = UI.CreatePreviewWindow(_previewPanel.Id, 10f, 40f, previewWindowWidth, previewWindowHeight, 1);
                
                // 设置Game Pass渲染尺寸匹配PreviewWindow
                UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
                Log.Info("Editor", $"PreviewWindow创建完成 (ShowMainLayout): {previewWindowWidth}x{previewWindowHeight}");
            }
            else
            {
                // PreviewWindow已存在，更新渲染尺寸
                float previewWindowWidth = previewWidth - 20f;
                float previewWindowHeight = mainHeight - 20f;
                UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
            }
            
            if (_assetPanel == null)
            {
                _assetPanel = new Panel(rootId, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
                UI.CreateLabel(_assetPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "资产管理");
                var assetList = new VStack(_assetPanel.Id, 5f);
                assetList.SetPosition(10f, 40f);
                assetList.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "Textures: 0");
            }
            
            if (_propertiesPanel == null)
            {
                _propertiesPanel = new Panel(rootId, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
                UI.CreateLabel(_propertiesPanel.Id, 10f, 10f, RIGHT_PANEL_WIDTH - 20f, 25f, "属性编辑");
            }
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "编辑器";
            }
            
            Log.Info("Editor", "主界面显示完成");
        }
        
        private static void HideMainLayout()
        {
            Log.Info("Editor", "隐藏主界面...");
            
            if (_previewPanel != null)
            {
                UI.RemoveWidget(_previewPanel.Id);
                _previewPanel = null;
            }
            
            if (_assetPanel != null)
            {
                UI.RemoveWidget(_assetPanel.Id);
                _assetPanel = null;
            }
            
            if (_propertiesPanel != null)
            {
                UI.RemoveWidget(_propertiesPanel.Id);
                _propertiesPanel = null;
            }
            
            // 清空projectPanel中的脚本项，保留基础结构
            if (_projectPanel != null && _projectTree != null)
            {
                // 先移除旧的projectTree
                UI.RemoveWidget(_projectTree.Id);
                
                // 重新创建projectTree以清空内容
                _projectTree = new VStack(_projectPanel.Id, 5f);
                _projectTree.SetPosition(10f, 40f);
                _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Assets");
                _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Scenes");
                _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "└─ Scripts");
            }
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "预览";
            }
            
            Log.Info("Editor", "主界面隐藏完成");
        }
        
        private static void OpenInExplorer(ulong widgetId)
        {
            Log.Info("Editor", "打开文件管理器...");
            try
            {
                Process.Start("explorer.exe", _currentDirectory);
            }
            catch (Exception ex)
            {
                Log.Error("Editor", ex.Message);
            }
        }
        
        private static void RefreshDirectoryTree()
        {
            if (_projectPanel == null) return;
            
            if (_projectTree != null)
            {
                UI.RemoveWidget(_projectTree.Id);
            }
            
            _fileItemPaths.Clear();
            _dirItemPaths.Clear();
            
            _projectTree = new VStack(_projectPanel.Id, 5f);
            _projectTree.SetPosition(10f, 40f);
            
            var openBtn = _projectTree.AddButton(LEFT_PANEL_WIDTH - 40f, 20f, "📂 打开目录");
            UI.SetOnClick(openBtn, _openInExplorerCallback);
            
            try
            {
                if (Directory.Exists(_currentDirectory))
                {
                    // 添加返回上一级按钮
                    if (_currentDirectory != "scripts" && Directory.GetParent(_currentDirectory) != null)
                    {
                        ulong backBtnId = _projectTree.AddButton(LEFT_PANEL_WIDTH - 40f, 20f, "⬆ 返回上级");
                        UI.SetOnClick(backBtnId, _backClickCallback);
                    }
                    
                    AddDirectoryItems(_projectTree, _currentDirectory, 0);
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", $"reading directory: {ex.Message}");
            }
            
            Log.Info("Editor", "目录树刷新完成");
        }
        
        private static void AddDirectoryItems(VStack stack, string path, int depth)
        {
            string prefix = new string(' ', depth * 2);
            
            try
            {
                string[] dirs = Directory.GetDirectories(path);
                foreach (string dir in dirs)
                {
                    string name = Path.GetFileName(dir);
                    ulong btnId = stack.AddButton(LEFT_PANEL_WIDTH - 40f, 20f, $"{prefix}📁 {name}/");
                    _dirItemPaths[btnId] = dir;
                    UI.SetOnClick(btnId, _directoryClickCallback);
                }
                
                string[] files = Directory.GetFiles(path);
                foreach (string file in files)
                {
                    if (file.EndsWith(".cs") || file.EndsWith(".txt") || file.EndsWith(".json"))
                    {
                        string name = Path.GetFileName(file);
                        ulong btnId = stack.AddButton(LEFT_PANEL_WIDTH - 40f, 20f, $"{prefix}📄 {name}");
                    _fileItemPaths[btnId] = file;
                    UI.SetOnClick(btnId, _fileClickCallback);
                    }
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", ex.Message);
            }
        }
        
        private static void OnBackClick(ulong widgetId)
        {
            var parent = Directory.GetParent(_currentDirectory);
            if (parent != null)
            {
                _currentDirectory = parent.FullName;
                Log.Info("Editor", $"返回上级目录: {_currentDirectory}");
                RefreshDirectoryTree();
            }
        }
        
        private static void OnDirectoryClick(ulong widgetId)
        {
            if (_dirItemPaths.TryGetValue(widgetId, out string path))
            {
                _currentDirectory = path;
                RefreshDirectoryTree();
                Log.Info("Editor", $"进入目录: {path}");
            }
        }
        
        private static void OnFileClick(ulong widgetId)
        {
            if (_fileItemPaths.TryGetValue(widgetId, out string path))
            {
                Log.Info("Editor", $"点击文件: {path}");
                LoadFileToEditor(path);
            }
        }
        
        private static void LoadFileToEditor(string filePath)
        {
            try
            {
                string content = File.ReadAllText(filePath);
                string fileName = Path.GetFileName(filePath);
                
                Log.Info("Editor", $"读取文件: {fileName} ({content.Length} chars)");
                
                // 确保编辑器已显示
                if (!_scriptEditorVisible)
                {
                    Log.Info("Editor", "编辑器未显示，先显示编辑器...");
                    
                    // 先创建编辑器（不刷新目录树）
                    ulong rootId = UI.GetRootId();
                    float editorX = LEFT_PANEL_WIDTH;
                    float editorY = TOOLBAR_HEIGHT;
                    float editorWidth = _screenWidth - LEFT_PANEL_WIDTH;
                    float editorHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
                    
                    // 移除主界面panel
                    if (_previewPanel != null)
                    {
                        UI.RemoveWidget(_previewPanel.Id);
                        _previewPanel = null;
                    }
                    if (_assetPanel != null)
                    {
                        UI.RemoveWidget(_assetPanel.Id);
                        _assetPanel = null;
                    }
                    if (_propertiesPanel != null)
                    {
                        UI.RemoveWidget(_propertiesPanel.Id);
                        _propertiesPanel = null;
                    }
                    
                    // 创建编辑器面板
                    _scriptEditorPanel = new Panel(rootId, editorX, editorY, editorWidth, editorHeight, 0.12f, 0.12f, 0.14f, 1.0f);
                    
                    var hotReloadBtn = new Button(_scriptEditorPanel.Id, 100f, 30f, "Hot Reload");
                    UI.SetWidgetLayout(hotReloadBtn.Id, 10f, 10f, 100f, 30f);
                    hotReloadBtn.SetOnClick(_hotReloadClickCallback);
                    
                    _scriptEditorLabel = new Label(_scriptEditorPanel.Id, 200f, 25f, fileName);
                    UI.SetWidgetLayout(_scriptEditorLabel.Id, 120f, 10f, 300f, 25f);
                    
                    _scriptTextEditId = UI.CreateTextEdit(_scriptEditorPanel.Id, editorWidth - 20f, editorHeight - 50f);
            UI.SetTextEditShowLineNumbers(_scriptTextEditId, true);
            UI.SetWidgetLayout(_scriptTextEditId, 10f, 50f, editorWidth - 20f, editorHeight - 50f);
                    
                    _scriptEditorVisible = true;
                    Log.Info("Editor", "编辑器面板创建完成");
                    
                    // 刷新目录树（保持点击回调有效）
                    RefreshDirectoryTree();
                }
                
                // 设置文本内容
                if (_scriptTextEditId != 0)
                {
                    Log.Info("Editor", $"设置TextEdit内容，id={_scriptTextEditId}");
                    UI.TextEditSetText(_scriptTextEditId, content);
                    Log.Info("Editor", $"✓ 文件已加载: {fileName}");
                    
                    if (_scriptEditorLabel != null)
                    {
                        _scriptEditorLabel.Text = $"Script Editor - {fileName}";
                    }
                }
                else
                {
                    Log.Error("Editor", "TextEdit id为0");
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", $"loading file: {ex.Message}\n{ex.StackTrace}");
            }
        }
    }
}