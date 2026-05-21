using System;
using System.IO;
using System.Diagnostics;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static void OnNewClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"新建\"按钮, id={widgetId}");
        }
        
        private static void OnOpenClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"打开\"按钮, id={widgetId}");
        }
        
        private static void OnSaveClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"保存\"按钮, id={widgetId}");
        }
        
        private static void OnRunClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"运行\"按钮, id={widgetId}");
            
            if (_gameScene == null)
            {
                Log.Error("Editor", "Scene未创建!");
                return;
            }
            
            var currentState = _gameScene.GetGameState();
            if (currentState == GameState.Editing)
            {
                _gameScene.SetGameState(GameState.Running);
                UI.SetRendererGameState(1);
                UI.SetPreviewWindowEditMode(_previewWindowId, false);
                UI.SetText(_runButtonId, "编辑");
                Log.Info("Editor", "Scene和Renderer切换到Running状态");
                _statusItem.Text = "状态: 运行中";
            }
            else
            {
                _gameScene.SetGameState(GameState.Editing);
                UI.SetRendererGameState(0);
                UI.SetPreviewWindowEditMode(_previewWindowId, true);
                UI.SetText(_runButtonId, "运行");
                Log.Info("Editor", "Scene和Renderer切换到Editing状态");
                _statusItem.Text = "状态: 就绪";
            }
        }
        
        private static void OnToggleEditorClick(ulong widgetId)
        {
            if (_isTransitioning) return;
            
            Log.Info("Editor", $"点击\"编辑器\"切换按钮, id={widgetId}");
            _isTransitioning = true;
            
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
                
                if (!_scriptEditorVisible)
                {
                    Log.Info("Editor", "编辑器未显示，先显示编辑器...");
                    
                    ulong rootId = UI.GetRootId();
                    float editorX = LEFT_PANEL_WIDTH;
                    float editorY = TOOLBAR_HEIGHT;
                    float editorWidth = _screenWidth - LEFT_PANEL_WIDTH;
                    float editorHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
                    
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
                    
                    RefreshDirectoryTree();
                }
                
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

        private static void CreateToolbarMenus()
        {
            _fileMenuId = UI.CreatePopupMenu(0);
            UI.PopupMenuAddItem(_fileMenuId, "新建场景", "Ctrl+N", 1);
            UI.PopupMenuAddItem(_fileMenuId, "新建脚本", "", 2);
            UI.PopupMenuAddSeparator(_fileMenuId);
            UI.PopupMenuAddItem(_fileMenuId, "退出", "", 3);
            UI.PopupMenuSetOnClick(_fileMenuId, _fileMenuClickCallback);
            UI.SetWidgetLayer(_fileMenuId, 2);
            
            _openMenuId = UI.CreatePopupMenu(0);
            UI.PopupMenuAddItem(_openMenuId, "打开场景", "", 1);
            UI.PopupMenuAddItem(_openMenuId, "打开项目", "", 2);
            UI.PopupMenuAddItem(_openMenuId, "打开资源", "", 3);
            UI.PopupMenuSetOnClick(_openMenuId, _openMenuClickCallback);
            UI.SetWidgetLayer(_openMenuId, 2);
            
            _saveMenuId = UI.CreatePopupMenu(0);
            UI.PopupMenuAddItem(_saveMenuId, "保存场景", "Ctrl+S", 1);
            UI.PopupMenuAddItem(_saveMenuId, "保存全部", "", 2);
            UI.PopupMenuAddItem(_saveMenuId, "另存为...", "", 3);
            UI.PopupMenuSetOnClick(_saveMenuId, _saveMenuClickCallback);
            UI.SetWidgetLayer(_saveMenuId, 2);
            
            Log.Info("Editor", "工具栏菜单创建完成 (PopupMenu)");
        }
        
        private static void OnFileMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_fileMenuId);
            if (actionId == 1) OnNewClick(0);
            else if (actionId == 2) OnNewScriptClick(0);
            else if (actionId == 3) { }
            Log.Info("Editor", $"文件菜单点击: actionId={actionId}");
        }
        
        private static void OnOpenMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_openMenuId);
            if (actionId == 1) { }
            else if (actionId == 2) OnOpenClick(0);
            else if (actionId == 3) { }
            Log.Info("Editor", $"打开菜单点击: actionId={actionId}");
        }
        
        private static void OnSaveMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_saveMenuId);
            if (actionId == 1) OnSaveClick(0);
            else if (actionId == 2) { }
            else if (actionId == 3) { }
            Log.Info("Editor", $"保存菜单点击: actionId={actionId}");
        }
    }
}