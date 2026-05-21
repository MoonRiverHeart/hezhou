using System;
using System.IO;
using System.Diagnostics;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        // =====================================================
        // PRESENTER: Event Handlers and Business Logic
        // =====================================================

        // === Working Directory Dialog Handlers ===

        private static void OnWorkingDirectoryDialogResult(ulong dialogId, int result)
        {
            Log.Info("Editor", $"工作目录对话框结果: result={result}");
            
            if (result == 1)
            {
                string selectedPath = UI.FileBrowserGetSelectedPath(_workingDirectoryFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath) && Directory.Exists(selectedPath))
                {
                    _currentDirectory = selectedPath;
                    _workingDirectorySet = true;
                    Log.Info("Editor", $"工作目录已设置: {_currentDirectory}");
                }
                else
                {
                    string currentPath = UI.FileBrowserGetCurrentPath(_workingDirectoryFileBrowserId);
                    if (!string.IsNullOrEmpty(currentPath) && Directory.Exists(currentPath))
                    {
                        _currentDirectory = currentPath;
                        _workingDirectorySet = true;
                        Log.Info("Editor", $"工作目录已设置(使用当前路径): {_currentDirectory}");
                    }
                    else
                    {
                        _currentDirectory = "scripts";
                        _workingDirectorySet = true;
                        Log.Info("Editor", "使用默认工作目录: scripts");
                    }
                }
            }
            else if (result == 2)
            {
                _currentDirectory = "scripts";
                _workingDirectorySet = true;
                Log.Info("Editor", "使用默认工作目录: scripts");
            }
            
            UI.DialogHide(_workingDirectoryDialogId);
            
            if (_workingDirectorySet)
            {
                CreateEditorLayout();
                ScanScripts();
                Log.Info("Editor", "编辑器初始化完成");
            }
        }
        
        private static void OnFileBrowserSelect(ulong browserId, string path)
        {
            Log.Info("Editor", $"文件浏览器选择: {path}");
        }

        // === Input Event Handlers ===

        private static void OnMouseMove(float x, float y, bool dragging)
        {
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
        }

        private static void OnKey(uint keycode, bool pressed, uint modifiers)
        {
            const uint KEY_ESC = 39;
            const uint KEY_LEFT = 45;
            const uint KEY_RIGHT = 46;
            const uint KEY_UP = 47;
            const uint KEY_DOWN = 48;
            const uint KEY_D = 4;

            bool ctrl = (modifiers & 2) != 0;
            bool shift = (modifiers & 1) != 0;

            if (keycode == KEY_D && pressed && ctrl && shift)
            {
                UI.DebugPrintUITree();
                Log.Info("Editor", "Ctrl+Shift+D: UI tree printed");
                return;
            }
            
            bool selected = UI.IsPreviewWindowSelected(_previewWindowId);
            GameState currentState = _gameScene != null ? _gameScene.GetGameState() : GameState.Editing;
            
            if (keycode == KEY_ESC && pressed)
            {
                if (currentState == GameState.Running)
                {
                    _gameScene.SetGameState(GameState.Editing);
                    UI.SetRendererGameState(0);
                    UI.SetPreviewWindowEditMode(_previewWindowId, true);
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
        
        private static void OnGlobalClick(float x, float y)
        {
            Log.Info("Editor", $"GlobalClick at ({x}, {y})");
            
            if (_previewWindowId != 0 && UI.IsPreviewWindowSelected(_previewWindowId))
            {
                GameState currentState = _gameScene != null ? _gameScene.GetGameState() : GameState.Editing;
                
                if (currentState == GameState.Editing && _gameScene != null)
                {
                    Log.Info("Editor", "Editing mode - attempting entity pick");
                    
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
                        ClearPropertiesPanel();
                    }
                }
                else
                {
                    Log.Info("Editor", "Running mode - camera control");
                }
            }
        }

        // === Property Panel Business Logic ===

        private static void UpdatePropertiesPanel(ulong entityId)
        {
            if (_propsList == null || _gameScene == null) return;
            
            if (!_propertiesDirty && _lastPropertiesEntityId == entityId)
            {
                return;
            }
            
            _propertiesDirty = false;
            _lastPropertiesEntityId = entityId;
            _selectedEntityId = entityId;
            
            for (int i = 0; i < _propertyDescriptors.Count; i++)
            {
                PropertyDescriptor desc = _propertyDescriptors[i];
                
                if (desc.Type == 1) // Float3
                {
                    float x, y, z;
                    if (UI.EntityGetPropertyValueFloat3(_gameScene.ScenePtr, entityId, desc.Name, out x, out y, out z))
                    {
                        // Rotation values are euler degrees (Rust converts quaternion→euler)
                        string format = (desc.Name == "rotation") ? "F0" : "F2";
                        UI.InputFieldSetText(desc.WidgetIds[0], x.ToString(format));
                        UI.InputFieldSetText(desc.WidgetIds[1], y.ToString(format));
                        UI.InputFieldSetText(desc.WidgetIds[2], z.ToString(format));
                    }
                }
                else if (desc.Type == 2) // String
                {
                    string value = UI.EntityGetPropertyValueString(_gameScene.ScenePtr, entityId, desc.Name);
                    if (desc.ReadOnly)
                    {
                        UI.SetText(desc.WidgetIds[0], value);
                    }
                    else
                    {
                        UI.InputFieldSetText(desc.WidgetIds[0], value);
                    }
                }
                else if (desc.Type == 4) // Int
                {
                    string value = UI.EntityGetPropertyValueString(_gameScene.ScenePtr, entityId, desc.Name);
                    if (desc.ReadOnly)
                    {
                        UI.SetText(desc.WidgetIds[0], value);
                    }
                    else
                    {
                        UI.InputFieldSetText(desc.WidgetIds[0], value);
                    }
                }
            }
            
            UpdateScriptBindingsList(entityId);
            
            Log.Info("Editor", "Properties panel updated for entity " + entityId + " (dynamic)");
        }
        
        private static void ClearPropertiesPanel()
        {
            if (_propsList == null) return;
            
            _selectedEntityId = 0;
            
            for (int i = 0; i < _propertyDescriptors.Count; i++)
            {
                PropertyDescriptor desc = _propertyDescriptors[i];
                
                if (desc.Type == 1) // Float3
                {
                    // Default: position/rotation = 0, scale = 1
                    string defaultVal = (desc.Name == "scale") ? "1" : "0";
                    for (int j = 0; j < desc.WidgetIds.Length; j++)
                    {
                        UI.InputFieldSetText(desc.WidgetIds[j], defaultVal);
                    }
                }
                else if (desc.Type == 2) // String
                {
                    if (desc.ReadOnly)
                    {
                        UI.SetText(desc.WidgetIds[0], "");
                    }
                    else
                    {
                        UI.InputFieldSetText(desc.WidgetIds[0], "");
                    }
                }
                else if (desc.Type == 4) // Int
                {
                    if (desc.ReadOnly)
                    {
                        UI.SetText(desc.WidgetIds[0], "");
                    }
                    else
                    {
                        UI.InputFieldSetText(desc.WidgetIds[0], "");
                    }
                }
            }
            
            Log.Info("Editor", "Properties panel cleared (dynamic)");
        }
        
        private static void HandleFloat3ComponentChange(string propertyName, int componentIndex, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float value))
            {
                float x, y, z;
                UI.EntityGetPropertyValueFloat3(_gameScene.ScenePtr, _selectedEntityId, propertyName, out x, out y, out z);
                switch (componentIndex)
                {
                    case 0: x = value; break;
                    case 1: y = value; break;
                    case 2: z = value; break;
                }
                UI.EntitySetPropertyValueFloat3(_gameScene.ScenePtr, _selectedEntityId, propertyName, x, y, z);
                _propertiesDirty = true;
                Log.Info("Editor", "Property " + propertyName + " component " + componentIndex + " changed to: " + value);
            }
        }
        
        private static void HandleStringPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            UI.EntitySetPropertyValueString(_gameScene.ScenePtr, _selectedEntityId, propertyName, text);
            
            if (propertyName == "name")
            {
                UpdateEntityNameInTree(_selectedEntityId, text);
            }
            
            _propertiesDirty = true;
            Log.Info("Editor", "Property " + propertyName + " changed to: " + text);
        }
        
        private static void HandleIntPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (int.TryParse(text, out int value))
            {
                UI.EntitySetPropertyValueString(_gameScene.ScenePtr, _selectedEntityId, propertyName, value.ToString());
                _propertiesDirty = true;
                Log.Info("Editor", "Property " + propertyName + " changed to: " + value);
            }
        }
        
        private static void UpdateScriptBindingsList(ulong entityId)
        {
            if (_gameScene == null || _scriptsListContainerId == 0) return;
            
            int scriptCount = _gameScene.GetScriptBindingCount(entityId);
            
            if (scriptCount == _lastScriptBindingCount && _lastPropertiesEntityId == entityId)
            {
                return;
            }
            
            _lastScriptBindingCount = scriptCount;
            _removeScriptBtnIndices.Clear();
            
            if (scriptCount == 0)
            {
                Log.Info("Editor", "UpdateScriptBindingsList: no scripts attached");
                return;
            }
            
            Log.Info("Editor", "UpdateScriptBindingsList: entityId=" + entityId + ", count=" + scriptCount);
            
            for (int i = 0; i < scriptCount; i++)
            {
                var info = _gameScene.GetScriptBindingInfo(entityId, i);
                Log.Info("Editor", "  Script[" + i + "]: path=" + info.ScriptPath + ", class=" + info.ClassName + ", enabled=" + info.Enabled);
                
                string scriptName = Path.GetFileName(info.ScriptPath);
                string labelText = scriptName + " (" + info.ClassName + ") [" + (info.Enabled ? "ON" : "OFF") + "]";
                
                var scriptRow = UI.CreateHStack(_scriptsListContainerId, 5f);
                UI.CreateLabel(scriptRow, RIGHT_PANEL_WIDTH - 90f, 20f, labelText);
                
                ulong removeBtnId = UI.CreateButton(scriptRow, 40f, 20f, "X");
                UI.SetOnClick(removeBtnId, _removeScriptClickCallback);
                _removeScriptBtnIndices[removeBtnId] = i;
            }
        }

        // === Entity Event Handlers ===

        private static void OnCreateEntityClick(ulong widgetId)
        {
            Log.Info("Editor", "点击\"创建Entity\"按钮");
            
            if (_gameScene == null)
            {
                Log.Error("Editor", "Scene未创建!");
                return;
            }
            
            ulong entityId = _gameScene.CreateEntity();
            if (entityId != 0)
            {
                Log.Info("Editor", $"创建新Entity: id={entityId}");
                _statusItem.Text = $"创建Entity: {entityId}";
                
                string name = UI.SceneGetEntityName(_gameScene.ScenePtr, entityId);
                AddEntityToTree(entityId, name);
                SelectEntity(entityId);
            }
            else
            {
                Log.Error("Editor", "创建Entity失败!");
            }
        }
        
        private static void SelectEntity(ulong entityId)
        {
            if (_gameScene == null) return;
            _gameScene.SelectEntity(entityId);
            _selectedEntityId = entityId;
            UpdatePropertiesPanel(entityId);
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeViewSetSelected(_projectTreeViewId, nodeId);
            }
        }

        private static void AddEntityToTree(ulong entityId, string name)
        {
            if (_projectTreeViewId != 0 && _entitiesNodeId != 0)
            {
                ulong nodeId = UI.TreeViewAddNode(_projectTreeViewId, _entitiesNodeId, name, entityId, false);
                _entityNodeMap[entityId] = nodeId;
                Log.Info("Editor", $"添加Entity到树: entityId={entityId}, name={name}");
            }
        }
        
        private static void UpdateEntityNameInTree(ulong entityId, string newName)
        {
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeNodeSetText(nodeId, newName);
                Log.Info("Editor", $"更新Entity名称: entityId={entityId}, name={newName}");
            }
        }
        
        private static void RemoveEntityFromTree(ulong entityId)
        {
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeViewRemoveNode(_projectTreeViewId, nodeId);
                _entityNodeMap.Remove(entityId);
                Log.Info("Editor", $"从树移除Entity: entityId={entityId}");
            }
        }

        private static void OnTreeNodeSelect(ulong widgetId, ulong userData)
        {
            // Check if this is a directory tree node selection
            if (_dirItemPaths.TryGetValue(widgetId, out string dirPath))
            {
                if (dirPath != null && Directory.Exists(dirPath))
                {
                    _currentDirectory = dirPath;
                    RefreshDirectoryTree();
                    Log.Info("Editor", $"进入目录: {dirPath}");
                }
                return;
            }
            
            if (_fileItemPaths.TryGetValue(widgetId, out string filePath))
            {
                Log.Info("Editor", $"点击文件: {filePath}");
                LoadFileToEditor(filePath);
                return;
            }
            
            // Project tree entity selection
            if (userData != 0)
            {
                SelectEntity(userData);
                Log.Info("Editor", $"TreeView选中Entity: entityId={userData}");
            }
        }

        private static void RefreshAssetGridView()
        {
            UI.GridViewClear(_assetGridViewId);
            
            UI.GridViewAddItem(_assetGridViewId, "Cube", 1);
            UI.GridViewAddItem(_assetGridViewId, "Sphere", 2);
            UI.GridViewAddItem(_assetGridViewId, "Plane", 3);
            UI.GridViewAddItem(_assetGridViewId, "Cylinder", 4);
            
            Log.Info("Editor", $"资产GridView刷新完成: {UI.GridViewItemCount(_assetGridViewId)}项");
        }
        
        private static void OnGridViewClick(ulong widgetId, int index, ulong userData)
        {
            if (_gameScene != null && userData != 0)
            {
                ulong entityId = _gameScene.CreateEntity();
                if (entityId != 0)
                {
                    string name = $"Asset_{userData}";
                    UI.SceneSetEntityName(_gameScene.ScenePtr, entityId, name);
                    AddEntityToTree(entityId, name);
                    SelectEntity(entityId);
                    _statusItem.Text = $"从资产库创建Entity: {entityId}";
                    Log.Info("Editor", $"从资产GridView创建Entity: index={index}, userData={userData}, entityId={entityId}");
                }
            }
        }

        // === Menu Event Handlers ===

        private static void OnNewClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"文件\"菜单项, id={widgetId}");
            UI.PopupMenuShow(_fileMenuId, 10f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
        }
        
        private static void OnOpenClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"打开\"菜单项, id={widgetId}");
            UI.PopupMenuShow(_openMenuId, 90f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
        }
        
        private static void OnSaveClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"保存\"菜单项, id={widgetId}");
            UI.PopupMenuShow(_saveMenuId, 170f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
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

        // === Script Event Handlers ===

        private static void ScanScripts()
        {
            _availableScripts.Clear();
            
            try
            {
                if (Directory.Exists("scripts"))
                {
                    string[] files = Directory.GetFiles("scripts", "*.cs");
                    foreach (string file in files)
                    {
                        string fileName = Path.GetFileName(file);
                        if (fileName != "UI.cs" && fileName != "DFX.cs" && fileName != "EditorScript.cs")
                        {
                            _availableScripts.Add(fileName);
                        }
                    }
                    
                    Log.Info("Editor", $"扫描scripts目录: 找到 {_availableScripts.Count} 个可用脚本");
                    foreach (var script in _availableScripts)
                    {
                        Log.Info("Editor", $"  - {script}");
                    }
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", $"扫描scripts目录失败: {ex.Message}");
            }
        }

        private static void OnScriptDropdownSelect(ulong widgetId, ulong index)
        {
            _selectedScriptIndex = (int)index;
            int idx = (int)index;
            Log.Info("Editor", $"选择脚本: index={index}, script={(idx < _availableScripts.Count ? _availableScripts[idx] : "none")}");
        }
        
        private static void OnAddScriptClick(ulong widgetId)
        {
            Log.Info("Editor", "点击\"Add Script\"按钮");
            
            if (_selectedEntityId == 0 || _gameScene == null)
            {
                Log.Error("Editor", "未选中Entity!");
                return;
            }
            
            if (_availableScripts.Count == 0 || _selectedScriptIndex >= _availableScripts.Count)
            {
                Log.Error("Editor", "无可用脚本!");
                return;
            }
            
            string scriptName = _availableScripts[_selectedScriptIndex];
            string scriptPath = $"scripts/{scriptName}";
            string className = Path.GetFileNameWithoutExtension(scriptName);
            
            _gameScene.AttachScriptBinding(_selectedEntityId, scriptPath, className);
            Log.Info("Editor", $"添加脚本绑定: entityId={_selectedEntityId}, script={scriptPath}, class={className}");
            
            UpdatePropertiesPanel(_selectedEntityId);
        }
        
        private static void OnRemoveScriptClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"Remove Script\"按钮: widgetId={widgetId}");
            
            if (_selectedEntityId == 0 || _gameScene == null)
            {
                Log.Error("Editor", "未选中Entity!");
                return;
            }
            
            if (_removeScriptBtnIndices.TryGetValue(widgetId, out int index))
            {
                _gameScene.RemoveScriptBinding(_selectedEntityId, index);
                Log.Info("Editor", $"移除脚本绑定: entityId={_selectedEntityId}, index={index}");
                
                UpdatePropertiesPanel(_selectedEntityId);
            }
        }
        
        private static void OnHotReloadComplete()
        {
            Log.Info("Editor", "=== OnHotReloadComplete回调 ===");
            
            ScanScripts();
            Log.Info("Editor", $"脚本扫描完成: {_availableScripts.Count} 个脚本");
            
            if (_scriptDropdownId != 0)
            {
                string[] scriptOptions = _availableScripts.Count > 0 ? _availableScripts.ToArray() : new string[] { "无可用脚本" };
                UI.DropdownSetOptions(_scriptDropdownId, scriptOptions);
                Log.Info("Editor", "脚本下拉菜单已刷新");
            }
            
            if (_statusItem != null)
            {
                _statusItem.Text = "状态: 就绪";
            }
            
            Log.Info("Editor", "OnHotReloadComplete完成");
        }
        
        private static void OnTabSelect(ulong widgetId, ulong index)
        {
            Log.Info("Editor", $"TabWidget选择: widgetId={widgetId}, index={index}");
            string tabName = index == 0 ? "Transform" : "Scripts";
            _statusItem.Text = $"属性页: {tabName}";
        }
        
        private static void OnNewScriptClick(ulong widgetId)
        {
            Log.Info("Editor", "创建新脚本...");
            ShowScriptEditor();
        }

        private static void OnHotReloadClick(ulong widgetId)
        {
            Log.Info("Editor", "=== Hot Reload按钮点击 ===");
            
            if (_statusItem != null)
            {
                _statusItem.Text = "正在保存脚本...";
            }
            
            if (_scriptTextEditId != 0)
            {
                try
                {
                    string scriptContent = UI.TextEditGetText(_scriptTextEditId);
                    string scriptPath = "scripts/bin/Mono/EditorScript.cs";
                    
                    System.IO.Directory.CreateDirectory("scripts/bin/Mono");
                    System.IO.File.WriteAllText(scriptPath, scriptContent);
                    Log.Info("Editor", $"✓ 脚本已保存: {scriptPath} ({scriptContent.Length} chars)");
                    
                    UI.SetStatusText("正在热更新脚本...");
                    UI.TriggerHotReload();
                    
                    Log.Info("Editor", "已触发Rust端HotReload流程");
                }
                catch (Exception ex)
                {
                    Log.Error("Editor", $"保存脚本失败: {ex.Message}");
                    if (_statusItem != null)
                    {
                        _statusItem.Text = $"保存失败: {ex.Message}";
                    }
                }
            }
            else
            {
                Log.Error("Editor", "TextEdit未创建");
                if (_statusItem != null)
                {
                    _statusItem.Text = "错误: 编辑器未初始化";
                }
            }
        }
    }
}