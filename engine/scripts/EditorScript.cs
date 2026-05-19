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
        private static Panel _statusBar;
        private static HStack _statusItems;
        private static Label _fpsLabel;
        private static ulong _statusLabelId;
        private static ulong _projectLabelId;
        
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

        public static void Initialize(IntPtr contextPtr)
        {
            UI.InitFromContext(contextPtr);
            Log.Info("Editor", "编辑器初始化开始");
            
            UI.GetScreenSize(out _screenWidth, out _screenHeight);
            _contentScale = UI.GetContentScale();
            
            Log.Info("Editor", $"屏幕尺寸: {_screenWidth}x{_screenHeight}, DPI缩放: {_contentScale}");
            
            _updateCallback = Update;
            UI.RegisterUpdateCallback(_updateCallback);
            
            CreateEditorLayout();
            
            UI.RegisterResizeCallback(OnResize);
            UI.RegisterGlobalClickCallback(OnGlobalClick);
            UI.RegisterKeyCallback(OnKey);
            UI.RegisterMouseMoveCallback(OnMouseMove);
            
            Log.Info("Editor", "编辑器初始化完成");
        }

        private static void OnMouseMove(float x, float y, bool dragging)
        {
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
            
            Log.Info("Editor", $"鼠标拖动: yaw={_cameraYaw}, pitch={_cameraPitch}");
        }

        private static void OnKey(uint keycode, bool pressed, uint modifiers)
        {
            const uint KEY_ESC = 39;
            const uint KEY_LEFT = 45;
            const uint KEY_RIGHT = 46;
            const uint KEY_UP = 47;
            const uint KEY_DOWN = 48;
            
            bool selected = UI.IsPreviewWindowSelected(_previewWindowId);
            
            if (keycode == KEY_ESC && pressed && selected)
            {
                UI.SetPreviewWindowSelected(_previewWindowId, false);
                _cameraX = _savedCameraX;
                _cameraY = _savedCameraY;
                _cameraZ = _savedCameraZ;
                _cameraYaw = _savedCameraYaw;
                _cameraPitch = _savedCameraPitch;
                _keyLeftPressed = false;
                _keyRightPressed = false;
                _keyUpPressed = false;
                _keyDownPressed = false;
                Log.Info("Editor", "ESC: 退出预览窗，恢复摄像机");
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
            newBtn.SetOnClick(OnNewClick);
            
            var openBtn = _toolbarButtons.AddButton(100f, 30f, "打开");
            openBtn.SetOnClick(OnOpenClick);
            
            var saveBtn = _toolbarButtons.AddButton(100f, 30f, "保存");
            saveBtn.SetOnClick(OnSaveClick);
            
            var runBtn = _toolbarButtons.AddButton(100f, 30f, "运行");
            runBtn.SetOnClick(OnRunClick);
            
            _toggleEditorBtn = new Button(_toolbar.Id, 100f, 30f, "编辑器");
            UI.SetWidgetLayout(_toggleEditorBtn.Id, _screenWidth - 120f, 5f, 100f, 30f);
            _toggleEditorBtn.SetOnClick(OnToggleEditorClick);
            
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
            Log.Info("Editor", $"游戏预览面板创建完成: {previewWindowWidth}x{previewWindowHeight}");

            _propertiesPanel = new Panel(rootId, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_propertiesPanel.Id, 10f, 10f, RIGHT_PANEL_WIDTH - 20f, 25f, "属性编辑");
            _propsList = new VStack(_propertiesPanel.Id, 5f);
            _propsList.SetPosition(10f, 40f);
            _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "选中: 无");
            _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "位置: (0, 0)");
            _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "大小: (0, 0)");
            Log.Info("Editor", "属性面板创建完成");

            _statusBar = new Panel(rootId, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT, 0.12f, 0.12f, 0.12f, 1.0f);
            _statusItems = new HStack(_statusBar.Id, 20f);
            _statusItems.SetPosition(10f, 5f);
            _fpsLabel = new Label(_statusItems.Id, 120f, 25f, "FPS: 0");
            _statusLabelId = _statusItems.AddLabel(150f, 20f, "状态: 就绪");
            _projectLabelId = _statusItems.AddLabel(150f, 20f, "项目: 未命名");
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
            if (deltaTime > 0 && _fpsLabel != null)
            {
                try
                {
                    _fpsLabel.Text = $"FPS: {((int)(1000f / deltaTime))}";
                    
                    bool selected = UI.IsPreviewWindowSelected(_previewWindowId);
                    
                    if (selected != _previewSelected)
                    {
                        _previewSelected = selected;
                        if (selected)
                        {
                            _savedCameraX = _cameraX;
                            _savedCameraY = _cameraY;
                            _savedCameraZ = _cameraZ;
                            _savedCameraYaw = _cameraYaw;
                            _savedCameraPitch = _cameraPitch;
                        }
                        UpdateStatusBar();
                    }
                    
                    // Pass camera params to shader when preview is selected
                    if (_previewSelected)
                    {
                        float speed = 2f * (deltaTime / 1000f);  // 2 units/sec
                        if (_keyLeftPressed) _cameraX -= speed;
                        if (_keyRightPressed) _cameraX += speed;
                        if (_keyUpPressed) _cameraZ = Math.Max(0.5f, _cameraZ - speed);
                        if (_keyDownPressed) _cameraZ = Math.Min(10f, _cameraZ + speed);
                        
                        UI.SetCameraParams(_cameraYaw, _cameraPitch, _cameraX, _cameraY, _cameraZ);
                    }
                    else
                    {
                        // Reset to default view when not selected
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
            if (_statusLabelId == 0) return;
            
            if (_previewSelected)
            {
                UI.SetLabelText(_statusLabelId, "按ESC退出Game模式");
            }
            else
            {
                UI.SetLabelText(_statusLabelId, "状态: 就绪");
            }
        }
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate();
        
        private static void OnNewClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"新建\"按钮, id={widgetId}");
            ShowDropdownMenu(10, 45, 
                new string[] { "新建场景", "新建脚本", "新建材质", "新建文件夹" },
                new UI.WidgetCallbackDelegate[] { null, OnNewScriptClick, null, null });
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
            hotReloadBtn.SetOnClick(OnHotReloadClick);
            
            _scriptEditorLabel = new Label(_scriptEditorPanel.Id, 200f, 25f, "Script Editor - NewScript.cs");
            UI.SetWidgetLayout(_scriptEditorLabel.Id, 120f, 10f, 300f, 25f);
            
            _scriptTextEditId = UI.CreateTextEdit(_scriptEditorPanel.Id, editorWidth - 20f, editorHeight - 50f);
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
            Log.Info("Editor", "开始运行游戏...");
        }
        
        private static void OnGlobalClick(float x, float y)
        {
            Log.Info("Editor", $"GlobalClick at ({x}, {y})");
            HideDropdownMenu();
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
            UI.SetOnClick(openBtn, OpenInExplorer);
            
            try
            {
                if (Directory.Exists(_currentDirectory))
                {
                    // 添加返回上一级按钮
                    if (_currentDirectory != "scripts" && Directory.GetParent(_currentDirectory) != null)
                    {
                        ulong backBtnId = _projectTree.AddButton(LEFT_PANEL_WIDTH - 40f, 20f, "⬆ 返回上级");
                        UI.SetOnClick(backBtnId, OnBackClick);
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
                    UI.SetOnClick(btnId, OnDirectoryClick);
                }
                
                string[] files = Directory.GetFiles(path);
                foreach (string file in files)
                {
                    if (file.EndsWith(".cs") || file.EndsWith(".txt") || file.EndsWith(".json"))
                    {
                        string name = Path.GetFileName(file);
                        ulong btnId = stack.AddButton(LEFT_PANEL_WIDTH - 40f, 20f, $"{prefix}📄 {name}");
                        _fileItemPaths[btnId] = file;
                        UI.SetOnClick(btnId, OnFileClick);
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
                    hotReloadBtn.SetOnClick(OnHotReloadClick);
                    
                    _scriptEditorLabel = new Label(_scriptEditorPanel.Id, 200f, 25f, fileName);
                    UI.SetWidgetLayout(_scriptEditorLabel.Id, 120f, 10f, 300f, 25f);
                    
                    _scriptTextEditId = UI.CreateTextEdit(_scriptEditorPanel.Id, editorWidth - 20f, editorHeight - 50f);
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