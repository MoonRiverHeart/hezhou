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

            _previewPanel = new Panel(rootId, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT, 0.1f, 0.1f, 0.1f, 1.0f);
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

            UI.SetWidgetLayout(_toolbar.Id, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT);
            UI.SetWidgetLayout(_projectPanel.Id, 0, mainY, LEFT_PANEL_WIDTH, mainHeight);
            UI.SetWidgetLayout(_assetPanel.Id, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT);
            UI.SetWidgetLayout(_previewPanel.Id, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT);
            UI.SetWidgetLayout(_propertiesPanel.Id, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT);
            UI.SetWidgetLayout(_statusBar.Id, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT);
            
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
            ShowDropdownMenu(10, 45, new string[] { "新建场景", "新建脚本", "新建材质", "新建文件夹" });
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
        
        private static void ShowDropdownMenu(float x, float y, string[] items)
        {
            HideDropdownMenu();
            
            ulong rootId = UI.GetRootId();
            _dropdownMenu = new Panel(rootId, x, y, 150, items.Length * 25 + 5, 0.25f, 0.25f, 0.25f, 0.95f);
            _menuItems = new VStack(_dropdownMenu.Id, 2f);
_menuItems.SetPosition(5, 5);
            
            foreach (var item in items)
            {
                _menuItems.AddLabel(140, 20, item);
            }
            
            Console.WriteLine($"[Editor] 显示下拉菜单: {items.Length}项");
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