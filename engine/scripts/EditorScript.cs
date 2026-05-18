using System;
using System.Runtime.InteropServices;

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
        private static Panel _propertiesPanel;
        private static VStack _propsList;
        private static Panel _statusBar;
        private static HStack _statusItems;
        private static Label _fpsLabel;
        
        private static Panel _dropdownMenu;
        private static VStack _menuItems;
        
        private static Panel _scriptEditorPanel;
        private static ulong _scriptTextEditId;
        private static Label _scriptEditorLabel;
        private static bool _scriptEditorVisible = false;
        
        private static float _screenWidth = 1280f;
        private static float _screenHeight = 720f;

        private const float TOOLBAR_HEIGHT = 40f;
        private const float STATUS_BAR_HEIGHT = 30f;
        private const float LEFT_PANEL_WIDTH = 250f;
        private const float RIGHT_PANEL_WIDTH = 250f;
        private const float BOTTOM_PANEL_HEIGHT = 200f;

        public static void Initialize(IntPtr contextPtr)
        {
            Console.WriteLine("[Editor] 编辑器初始化开始");
            
            UI.InitFromContext(contextPtr);
            UI.GetScreenSize(out _screenWidth, out _screenHeight);
            
            Console.WriteLine($"[Editor] 屏幕尺寸: {_screenWidth}x{_screenHeight}");
            
            CreateEditorLayout();
            
            UI.RegisterResizeCallback(OnResize);
            UI.RegisterGlobalClickCallback(OnGlobalClick);
            
            Console.WriteLine("[Editor] 编辑器初始化完成");
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
            Console.WriteLine($"[Editor] RootId={rootId}");

            _toolbar = new Panel(rootId, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT, 0.15f, 0.15f, 0.15f, 1.0f);
            _toolbarButtons = new HStack(_toolbar.Id, 10f);
            _toolbarButtons.SetPosition(10f, 5f);
            
            var newBtn = _toolbarButtons.AddButton(80f, 30f, "新建");
            newBtn.SetOnClick(OnNewClick);
            
            var openBtn = _toolbarButtons.AddButton(80f, 30f, "打开");
            openBtn.SetOnClick(OnOpenClick);
            
            var saveBtn = _toolbarButtons.AddButton(80f, 30f, "保存");
            saveBtn.SetOnClick(OnSaveClick);
            
            var runBtn = _toolbarButtons.AddButton(80f, 30f, "运行");
            runBtn.SetOnClick(OnRunClick);
            
            Console.WriteLine("[Editor] 工具栏创建完成");

            _projectPanel = new Panel(rootId, 0, mainY, LEFT_PANEL_WIDTH, mainHeight, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_projectPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "项目结构");
            _projectTree = new VStack(_projectPanel.Id, 5f);
            _projectTree.SetPosition(10f, 40f);
            _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Assets");
            _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "├─ Scenes");
            _projectTree.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "└─ Scripts");
            Console.WriteLine("[Editor] 项目结构面板创建完成");

            _assetPanel = new Panel(rootId, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_assetPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "资产管理");
            _assetList = new VStack(_assetPanel.Id, 5f);
            _assetList.SetPosition(10f, 40f);
            _assetList.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "Textures: 0");
            _assetList.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "Models: 0");
            _assetList.AddLabel(LEFT_PANEL_WIDTH - 40f, 20f, "Scripts: 1");
            Console.WriteLine("[Editor] 资产管理面板创建完成");

            _previewPanel = new Panel(rootId, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT, 0.08f, 0.08f, 0.08f, 0.3f);
            UI.CreateLabel(_previewPanel.Id, 10f, 10f, previewWidth - 20f, 25f, "游戏预览");
            Console.WriteLine("[Editor] 游戏预览面板创建完成");

            _propertiesPanel = new Panel(rootId, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_propertiesPanel.Id, 10f, 10f, RIGHT_PANEL_WIDTH - 20f, 25f, "属性编辑");
            _propsList = new VStack(_propertiesPanel.Id, 5f);
            _propsList.SetPosition(10f, 40f);
            _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "选中: 无");
            _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "位置: (0, 0)");
            _propsList.AddLabel(RIGHT_PANEL_WIDTH - 40f, 20f, "大小: (0, 0)");
            Console.WriteLine("[Editor] 属性面板创建完成");

            _statusBar = new Panel(rootId, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT, 0.12f, 0.12f, 0.12f, 1.0f);
            _statusItems = new HStack(_statusBar.Id, 20f);
            _statusItems.SetPosition(10f, 5f);
            _fpsLabel = new Label(_statusItems.Id, 100f, 20f, "FPS: 0");
            _statusItems.AddLabel(150f, 20f, "状态: 就绪");
            _statusItems.AddLabel(150f, 20f, "项目: 未命名");
            Console.WriteLine("[Editor] 状态栏创建完成");
        }

        private static void OnResize(float width, float height)
        {
            _screenWidth = width;
            _screenHeight = height;
            Console.WriteLine($"[Editor] 窗口resize: {width}x{height}");
            
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
            if (_projectPanel != null)
                UI.SetWidgetLayout(_projectPanel.Id, 0, mainY, LEFT_PANEL_WIDTH, mainHeight);
            if (_assetPanel != null)
                UI.SetWidgetLayout(_assetPanel.Id, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT);
            if (_previewPanel != null)
                UI.SetWidgetLayout(_previewPanel.Id, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT);
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
            
            Console.WriteLine("[Editor] 布局更新完成");
        }

        public static void Update(float deltaTime)
        {
            if (deltaTime > 0)
            {
                _fpsLabel.Text = $"FPS: {((int)(1000f / deltaTime))}";
            }
        }
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate();
        
        private static void OnNewClick(ulong widgetId)
        {
            Console.WriteLine($"[Editor] 点击\"新建\"按钮, id={widgetId}");
            ShowDropdownMenu(10, 45, 
                new string[] { "新建场景", "新建脚本", "新建材质", "新建文件夹" },
                new UI.WidgetCallbackDelegate[] { null, OnNewScriptClick, null, null });
        }
        
        private static void OnNewScriptClick(ulong widgetId)
        {
            Console.WriteLine("[Editor] 创建新脚本...");
            HideDropdownMenu();
            ShowScriptEditor();
        }
        
        private static void ShowScriptEditor()
        {
            if (_scriptEditorVisible) return;
            
            Console.WriteLine("[Editor] ShowScriptEditor开始...");
            
            // 移除preview相关panel，保留左侧目录树
            if (_previewPanel != null)
            {
                Console.WriteLine("[Editor] 移除previewPanel...");
                UI.RemoveWidget(_previewPanel.Id);
                _previewPanel = null;
            }
            if (_assetPanel != null)
            {
                Console.WriteLine("[Editor] 移除assetPanel...");
                UI.RemoveWidget(_assetPanel.Id);
                _assetPanel = null;
            }
            if (_propertiesPanel != null)
            {
                Console.WriteLine("[Editor] 移除propertiesPanel...");
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
            
            // 在projectPanel添加脚本项
            if (_projectPanel != null)
            {
                var scriptsLabel = new Label(_projectPanel.Id, 150f, 25f, "Scripts/");
                UI.SetWidgetLayout(scriptsLabel.Id, 10f, 40f, 150f, 25f);
                
                var newScriptLabel = new Label(_projectPanel.Id, 150f, 25f, "  NewScript.cs");
                UI.SetWidgetLayout(newScriptLabel.Id, 10f, 70f, 150f, 25f);
                
                Console.WriteLine("[Editor] 左侧目录树添加脚本项");
            }
            
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
            Console.WriteLine("[Editor] Script Editor显示成功");
        }
        
        private static void OnHotReloadClick(ulong widgetId)
        {
            Console.WriteLine("[Editor] Hot Reload triggered!");
        }
        
        private static void OnOpenClick(ulong widgetId)
        {
            Console.WriteLine($"[Editor] 点击\"打开\"按钮, id={widgetId}");
            ShowDropdownMenu(100, 45, new string[] { "打开场景", "打开项目", "打开资源" });
        }
        
        private static void OnSaveClick(ulong widgetId)
        {
            Console.WriteLine($"[Editor] 点击\"保存\"按钮, id={widgetId}");
            ShowDropdownMenu(190, 45, new string[] { "保存场景", "保存全部", "另存为..." });
        }
        
        private static void OnRunClick(ulong widgetId)
        {
            Console.WriteLine($"[Editor] 点击\"运行\"按钮, id={widgetId}");
            HideDropdownMenu();
            Console.WriteLine("[Editor] 开始运行游戏...");
        }
        
        private static void OnGlobalClick(float x, float y)
        {
            Console.WriteLine($"[Editor] GlobalClick at ({x}, {y})");
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
                Console.WriteLine($"[Editor] 菜单项{i}: '{items[i]}', btnId={btnId}");
                if (i < callbacks.Length && callbacks[i] != null)
                {
                    Console.WriteLine($"[Editor] 注册回调: btnId={btnId}");
                    UI.SetOnClick(btnId, callbacks[i]);
                }
            }
            
            Console.WriteLine($"[Editor] 显示下拉菜单: {items.Length}项");
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
                Console.WriteLine("[Editor] 隐藏下拉菜单");
            }
        }
    }
}