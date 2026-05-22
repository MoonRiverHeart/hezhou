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
            if (result == 1)
            {
                string selectedPath = UI.FileBrowserGetSelectedPath(_workingDirectoryFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath) && Directory.Exists(selectedPath))
                {
                    _currentDirectory = selectedPath;
                    _workingDirectorySet = true;
                }
                else
                {
                    string currentPath = UI.FileBrowserGetCurrentPath(_workingDirectoryFileBrowserId);
                    if (!string.IsNullOrEmpty(currentPath) && Directory.Exists(currentPath))
                    {
                        _currentDirectory = currentPath;
                        _workingDirectorySet = true;
                    }
                    else
                    {
                        _currentDirectory = "scripts";
                        _workingDirectorySet = true;
                    }
                }
            }
            else if (result == 2)
            {
                _currentDirectory = "scripts";
                _workingDirectorySet = true;
            }
            
            UI.DialogHide(_workingDirectoryDialogId);
            
            if (_workingDirectorySet)
            {
                CreateEditorLayout();
                ScanScripts();
            }
        }
        
        private static void OnFileBrowserSelect(ulong browserId, string path)
        {
            // File browser selection — UI feedback only, no log needed
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
            if (_previewWindowId != 0 && UI.IsPreviewWindowSelected(_previewWindowId))
            {
                GameState currentState = _gameScene != null ? _gameScene.GetGameState() : GameState.Editing;
                
                if (currentState == GameState.Editing && _gameScene != null)
                {
                    float previewX = LEFT_PANEL_WIDTH + 10f * _contentScale;
                    float previewY = TOOLBAR_HEIGHT + 40f * _contentScale;
                    float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH - 20f * _contentScale;
                    float previewHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT - 50f * _contentScale;
                    
                    float relX = (x - previewX) / previewWidth;
                    float relY = (y - previewY) / previewHeight;
                    
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
                    
                    ulong hitEntity = _gameScene.PickEntity(originX, originY, originZ, dirX, dirY, dirZ);
                    
                    if (hitEntity != 0)
                    {
                        _gameScene.SelectEntity(hitEntity);
                        _statusItem.Text = $"选中Entity: {hitEntity}";
                        UpdatePropertiesPanel(hitEntity);
                    }
                    else
                    {
                        _gameScene.ClearSelection();
                        _statusItem.Text = "状态: 就绪";
                        ClearPropertiesPanel();
                    }
                }
            }
        }

        // === Property Panel Business Logic ===

        private static void UpdatePropertiesPanel(ulong entityId)
        {
            if (_propsTabWidget == null || _gameScene == null) return;
            
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
                else if (desc.Type == 0) // Float
                {
                    float value;
                    if (UI.EntityGetPropertyValueFloat(_gameScene.ScenePtr, entityId, desc.Name, out value))
                    {
                        if (desc.ReadOnly)
                        {
                            UI.SetText(desc.WidgetIds[0], value.ToString("F2"));
                        }
                        else
                        {
                            UI.InputFieldSetText(desc.WidgetIds[0], value.ToString("F2"));
                        }
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
                else if (desc.Type == 3) // Bool
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
        }
        
        private static void ClearPropertiesPanel()
        {
            if (_propsTabWidget == null) return;
            
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
                else if (desc.Type == 0) // Float
                {
                    if (desc.ReadOnly)
                    {
                        UI.SetText(desc.WidgetIds[0], "0.00");
                    }
                    else
                    {
                        UI.InputFieldSetText(desc.WidgetIds[0], "0.00");
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
                else if (desc.Type == 3) // Bool
                {
                    if (desc.ReadOnly)
                    {
                        UI.SetText(desc.WidgetIds[0], "false");
                    }
                    else
                    {
                        UI.InputFieldSetText(desc.WidgetIds[0], "false");
                    }
                }
            }
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
        }
        
        private static void HandleIntPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (int.TryParse(text, out int value))
            {
                UI.EntitySetPropertyValueString(_gameScene.ScenePtr, _selectedEntityId, propertyName, value.ToString());
                _propertiesDirty = true;
            }
        }

        private static void HandleFloatPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float value))
            {
                UI.EntitySetPropertyValueFloat(_gameScene.ScenePtr, _selectedEntityId, propertyName, value);
                _propertiesDirty = true;
            }
        }

        private static void HandleBoolPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            string lowerText = text.ToLower();
            if (lowerText == "true" || lowerText == "1")
            {
                UI.EntitySetPropertyValueString(_gameScene.ScenePtr, _selectedEntityId, propertyName, "true");
                _propertiesDirty = true;
            }
            else if (lowerText == "false" || lowerText == "0")
            {
                UI.EntitySetPropertyValueString(_gameScene.ScenePtr, _selectedEntityId, propertyName, "false");
                _propertiesDirty = true;
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
            
            // Clear existing script rows before rebuilding
            for (int r = 0; r < _scriptRowIds.Count; r++)
            {
                UI.RemoveWidget(_scriptRowIds[r]);
            }
            _scriptRowIds.Clear();
            
            if (scriptCount == 0)
            {
                return;
            }
            
            for (int i = 0; i < scriptCount; i++)
            {
                var info = _gameScene.GetScriptBindingInfo(entityId, i);
                
                string scriptName = Path.GetFileName(info.ScriptPath);
                string labelText = scriptName + " (" + info.ClassName + ") [" + (info.Enabled ? "ON" : "OFF") + "]";
                
                ulong scriptRow = UI.CreateHStack(_scriptsListContainerId, 5f);
                UI.CreateLabel(scriptRow, RIGHT_PANEL_WIDTH - 90f, 20f, labelText);
                
                ulong removeBtnId = UI.CreateButton(scriptRow, 40f, 20f, "X");
                UI.SetOnClick(removeBtnId, _removeScriptClickCallback);
                _removeScriptBtnIndices[removeBtnId] = i;
                _scriptRowIds.Add(scriptRow);
            }
        }

        // === Entity Event Handlers ===

        private static void SaveTreeExpandState()
        {
            _expandedNodeNames.Clear();
            if (_projectTreeViewId != 0)
            {
                // Query actual expand state from Rust side for the 4 category nodes
                if (_assetsNodeId != 0 && UI.TreeViewIsNodeExpanded(_projectTreeViewId, _assetsNodeId))
                    _expandedNodeNames.Add("Assets");
                if (_scenesNodeId != 0 && UI.TreeViewIsNodeExpanded(_projectTreeViewId, _scenesNodeId))
                    _expandedNodeNames.Add("Scenes");
                if (_scriptsNodeId != 0 && UI.TreeViewIsNodeExpanded(_projectTreeViewId, _scriptsNodeId))
                    _expandedNodeNames.Add("Scripts");
                if (_entitiesNodeId != 0 && UI.TreeViewIsNodeExpanded(_projectTreeViewId, _entitiesNodeId))
                    _expandedNodeNames.Add("Entities");
            }
        }

        private static void OnTreeNodeToggle(ulong nodeId)
        {
            if (_nodeIdToName.TryGetValue(nodeId, out string name))
            {
                if (_expandedNodeNames.Contains(name))
                {
                    _expandedNodeNames.Remove(name);
                }
                else
                {
                    _expandedNodeNames.Add(name);
                }
            }
        }

        private static void OnCreateEntityClick(ulong widgetId)
        {
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
            }
        }
        
        private static void UpdateEntityNameInTree(ulong entityId, string newName)
        {
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeNodeSetText(nodeId, newName);
            }
        }
        
        private static void RemoveEntityFromTree(ulong entityId)
        {
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeViewRemoveNode(_projectTreeViewId, nodeId);
                _entityNodeMap.Remove(entityId);
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
                }
                return;
            }
            
            if (_fileItemPaths.TryGetValue(widgetId, out string filePath))
            {
                LoadFileToEditor(filePath);
                return;
            }
            
            // Project tree entity selection
            if (userData != 0)
            {
                SelectEntity(userData);
            }
        }

        private static void RefreshAssetGridView()
        {
            UI.GridViewClear(_assetGridViewId);
            
            UI.GridViewAddItem(_assetGridViewId, "Cube", 1);
            UI.GridViewAddItem(_assetGridViewId, "Sphere", 2);
            UI.GridViewAddItem(_assetGridViewId, "Plane", 3);
            UI.GridViewAddItem(_assetGridViewId, "Cylinder", 4);
        }
        
private static void OnGridViewClick(ulong widgetId, int index, ulong userData)
        {
            if (_gameScene != null && userData != 0)
            {
                // userData: 1=Cube, 2=Sphere, 3=Plane, 4=Cylinder
                // MeshType index: 0=Cube, 1=Sphere, 2=Plane, 3=Cylinder
                int meshType = (int)userData - 1;
                ulong entityId = _gameScene.CreateMeshEntity(meshType);
                if (entityId != 0)
                {
                    string[] meshNames = { "Cube", "Sphere", "Plane", "Cylinder" };
                    string name = meshNames[meshType];
                    UI.SceneSetEntityName(_gameScene.ScenePtr, entityId, name);
                    AddEntityToTree(entityId, name);
SelectEntity(entityId);
                    _statusItem.Text = $"从资产库创建{name}: {entityId}";
                }
            }
        }

        // === Menu Event Handlers ===

        private static void OnNewClick(ulong widgetId)
        {
            UI.PopupMenuShow(_fileMenuId, 10f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
        }
        
        private static void OnOpenClick(ulong widgetId)
        {
            UI.PopupMenuShow(_openMenuId, 90f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
        }
        
        private static void OnSaveClick(ulong widgetId)
        {
            UI.PopupMenuShow(_saveMenuId, 170f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
        }
        
        private static void OnRunClick(ulong widgetId)
        {
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
                _statusItem.Text = "状态: 运行中";
            }
            else
            {
                _gameScene.SetGameState(GameState.Editing);
                UI.SetRendererGameState(0);
                UI.SetPreviewWindowEditMode(_previewWindowId, true);
                UI.SetText(_runButtonId, "运行");
                _statusItem.Text = "状态: 就绪";
            }
        }
        
        private static void OnToggleEditorClick(ulong widgetId)
        {
            if (_isTransitioning) return;
            
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
                RefreshDirectoryTree();
            }
        }
        
        private static void OnDirectoryClick(ulong widgetId)
        {
            if (_dirItemPaths.TryGetValue(widgetId, out string path))
            {
                _currentDirectory = path;
                RefreshDirectoryTree();
            }
        }
        
        private static void OnFileClick(ulong widgetId)
        {
            if (_fileItemPaths.TryGetValue(widgetId, out string path))
            {
                LoadFileToEditor(path);
            }
        }
        
        private static void LoadFileToEditor(string filePath)
        {
            try
            {
                string content = File.ReadAllText(filePath);
                string fileName = Path.GetFileName(filePath);
                
                if (!_scriptEditorVisible)
                {
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
                    
                    RefreshDirectoryTree();
                }
                
                if (_scriptTextEditId != 0)
                {
                    UI.TextEditSetText(_scriptTextEditId, content);
                    
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
        }
        
        private static void OnOpenMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_openMenuId);
            if (actionId == 1) { }
            else if (actionId == 2) OnOpenClick(0);
            else if (actionId == 3) { }
        }
        
        private static void OnSaveMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_saveMenuId);
            if (actionId == 1) OnSaveClick(0);
            else if (actionId == 2) { }
            else if (actionId == 3) { }
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
        }
        
        private static void OnAddScriptClick(ulong widgetId)
        {
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
            
            UpdatePropertiesPanel(_selectedEntityId);
        }
        
        private static void OnRemoveScriptClick(ulong widgetId)
        {
            if (_selectedEntityId == 0 || _gameScene == null)
            {
                Log.Error("Editor", "未选中Entity!");
                return;
            }
            
            if (_removeScriptBtnIndices.TryGetValue(widgetId, out int index))
            {
                _gameScene.RemoveScriptBinding(_selectedEntityId, index);
                
                UpdatePropertiesPanel(_selectedEntityId);
            }
        }
        
        private static void OnHotReloadComplete()
        {
            ScanScripts();
            
            if (_scriptDropdownId != 0)
            {
                string[] scriptOptions = _availableScripts.Count > 0 ? _availableScripts.ToArray() : new string[] { "无可用脚本" };
                UI.DropdownSetOptions(_scriptDropdownId, scriptOptions);
            }
            
            if (_statusItem != null)
            {
                _statusItem.Text = "状态: 就绪";
            }
        }
        
        private static void OnTabSelect(ulong widgetId, ulong index)
        {
            string tabName = index == 0 ? "Transform" : "Scripts";
            _statusItem.Text = $"属性页: {tabName}";
        }
        
        private static void OnNewScriptClick(ulong widgetId)
        {
            ShowScriptEditor();
        }

        private static void OnHotReloadClick(ulong widgetId)
        {
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
                    
                    UI.SetStatusText("正在热更新脚本...");
                    UI.TriggerHotReload();
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