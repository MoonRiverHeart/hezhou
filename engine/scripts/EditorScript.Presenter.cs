using System;
using System.IO;
using System.Diagnostics;
using System.Collections.Generic;
using System.Reflection;
using System.Text.RegularExpressions;
using HezhouScripts;

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
            Log.Info("Editor", "OnWorkingDirectoryDialogResult: dialogId=" + dialogId + " result=" + result);
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
                ScanScripts();            // Populate scripts BEFORE building UI
                CreateEditorLayout();     // Now dropdown gets populated scripts
            }
        }
        
        private static void OnFileBrowserSelect(ulong browserId, string path)
        {
            // File browser selection — UI feedback only, no log needed
        }

        /// <summary>
        /// 文件浏览器中可能只进入了目录而没有选中文件。
        /// 优先使用选中项，否则回退到当前目录。
        /// </summary>
        private static string GetBrowserSelectedOrCurrent(ulong browserId)
        {
            string selected = UI.FileBrowserGetSelectedPath(browserId);
            if (!string.IsNullOrEmpty(selected)) return selected;
            return UI.FileBrowserGetCurrentPath(browserId);
        }

        // === Input Event Handlers ===

        private static void OnMouseMove(float x, float y, bool dragging)
        {
            if (_gameScene == null) return;

            // === 浮动面板拖拽 ===
            if (_floatingDragging)
            {
                if (dragging) { UpdateFloatingDrag(x, y); return; }
                EndFloatingDrag();
            }
            else if (dragging && _floatingPanelId != 0 && PointInFloatingPanel(x, y) && y < 120f)
            {
                BeginFloatingDrag(x, y);
                return;
            }

            // === 材质拖拽 ===
            if (dragging && _materialDragName == null)
            {
                string hoverMaterial = FindMaterialAtPoint(x, y);
                if (hoverMaterial != null) _materialDragName = hoverMaterial;
            }
            if (_materialDragName != null)
            {
                if (dragging)
                {
                    if (_statusItem != null) _statusItem.Text = "拖拽材质: " + _materialDragName;
                    return;
                }
                ulong dropTarget = FindEntityAtPoint(x, y);
                if (dropTarget == 0) dropTarget = _selectedEntityId;
                AssignMaterial(dropTarget, _materialDragName);
                _materialDragName = null;
                return;
            }

            // === Gizmo 拖拽 ===
            if (_gizmoAxis != 0)
            {
                if (dragging) { UpdateGizmoDrag(x, y); return; }
                EndGizmoDrag();
            }
            else if (dragging && _selectedEntityId != 0)
            {
                int axis = HitTestGizmo(x, y);
                if (axis != 0) { BeginGizmoDrag(axis, x, y); return; }
            }

            // === Hierarchy 拖拽 ===
            if (dragging && _hierarchyDragSource == 0 && _selectedEntityId != 0
                && IsPointInsideEntityNode(_selectedEntityId, x, y))
            {
                _hierarchyDragSource = _selectedEntityId;
                _hierarchyDragActive = true;
            }
            if (_hierarchyDragActive)
            {
                if (dragging)
                {
                    ulong hover = FindEntityAtPoint(x, y);
                    if (hover != 0 && hover != _hierarchyDragSource)
                    {
                        _hierarchyDragTarget = hover;
                        if (_statusItem != null) _statusItem.Text = "拖拽到: " + UI.SceneGetEntityName(_gameScene.ScenePtr, hover);
                    }
                    return;
                }
                else
                {
                    PerformHierarchyDrop(_hierarchyDragSource, _hierarchyDragTarget);
                    ResetHierarchyDrag();
                    return;
                }
            }
            
            GameState state = _gameScene.GetGameState();
            
            // Running mode: yaw/pitch camera rotation
            if (state == GameState.Running)
            {
                if (!dragging || !_previewSelected)
                {
                    _mouseDragging = false;
                    return;
                }
                
                // 检查鼠标是否在预览窗范围内 — 防止拖出预览窗后仍能旋转视角
                float[] previewBounds = UI.WidgetGetAbsoluteLayout(_previewWindowId);
                if (previewBounds != null && previewBounds.Length >= 4)
                {
                    float px = previewBounds[0];
                    float py = previewBounds[1];
                    float pw = previewBounds[2];
                    float ph = previewBounds[3];
                    if (x < px || x > px + pw || y < py || y > py + ph)
                    {
                        _mouseDragging = false;
                        return;
                    }
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
                return;
            }
            
            // Editing mode: orbit camera rotation
            if (state == GameState.Editing)
            {
                if (!dragging || !_previewSelected)
                {
                    _mouseDragging = false;
                    return;
                }
                
                // 检查鼠标是否在预览窗范围内 — 防止拖出预览窗后仍能orbit(用绝对坐标匹配鼠标坐标)
                float[] previewBounds = UI.WidgetGetAbsoluteLayout(_previewWindowId);
                if (previewBounds != null && previewBounds.Length >= 4)
                {
                    float px = previewBounds[0];
                    float py = previewBounds[1];
                    float pw = previewBounds[2];
                    float ph = previewBounds[3];
                    if (x < px || x > px + pw || y < py || y > py + ph)
                    {
                        _mouseDragging = false;
                        return;
                    }
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
                
                _orbitYaw += dx * 0.005f;
                _orbitPitch -= dy * 0.005f;
                // Clamp pitch to avoid flipping
                if (_orbitPitch > 1.4f) _orbitPitch = 1.4f;
                if (_orbitPitch < -1.4f) _orbitPitch = -1.4f;
                return;
            }
            
            _mouseDragging = false;
        }

        private static void OnMouseWheel(float deltaX, float deltaY)
        {
            if (_gameScene == null) return;
            GameState state = _gameScene.GetGameState();
            
            // Editing mode: orbit camera zoom
            if (state == GameState.Editing && _previewSelected)
            {
                float zoomSpeed = 0.5f;
                _orbitDistance -= deltaY * zoomSpeed;
                if (_orbitDistance < 0.5f) _orbitDistance = 0.5f;
                if (_orbitDistance > 50f) _orbitDistance = 50f;
            }
        }

        private static void OnKey(uint keycode, bool pressed, uint modifiers)
        {
            const uint KEY_ESC = 39;
            const uint KEY_LEFT = 45;
            const uint KEY_RIGHT = 46;
            const uint KEY_UP = 47;
            const uint KEY_DOWN = 48;
            const uint KEY_D = 4;
            const uint KEY_N = 14;
            const uint KEY_O = 15;
            const uint KEY_P = 16;
            const uint KEY_G = 6;
            const uint KEY_S = 19;
            const uint KEY_Y = 25;
            const uint KEY_Z = 26;
            const uint KEY_DELETE = 49;
            const uint KEY_F5 = 57;
            const uint KEY_F6 = 58;
            const uint KEY_F7 = 59;

            bool ctrl = (modifiers & 2) != 0;
            bool shift = (modifiers & 1) != 0;
            _ctrlPressed = ctrl;
            _shiftPressed = shift;

            if (pressed && ctrl && !shift && keycode == KEY_G)
            {
                TestGizmoCycleMode();
                return;
            }
            if (pressed && ctrl && shift && keycode == KEY_P)
            {
                ToggleCommandPalette();
                return;
            }
            if (pressed && keycode == KEY_ESC && IsCommandPaletteOpen)
            {
                HideCommandPalette();
                return;
            }

            // 文件快捷键：Ctrl+S 保存、Ctrl+Shift+S 另存为、Ctrl+O 打开项目、Ctrl+N 新建场景
            if (pressed && ctrl)
            {
                if (keycode == KEY_Z)
                {
                    Undo();
                    return;
                }
                if (keycode == KEY_Y)
                {
                    Redo();
                    return;
                }
                if (keycode == KEY_S)
                {
                    if (shift) ShowSaveAsDialog();
                    else SaveCurrentProject();
                    return;
                }
                if (keycode == KEY_O)
                {
                    ShowOpenProjectDialog();
                    return;
                }
                if (keycode == KEY_N)
                {
                    NewScene();
                    return;
                }
            }

            if (keycode == KEY_D && pressed && ctrl && shift)
            {
                UI.DebugPrintUITree();
                return;
            }

            // UI Test shortcuts: Ctrl+Shift+F5/F6/F7 → show config dialog
            if (pressed && ctrl && shift)
            {
                if (keycode == KEY_F5)
                {
                    UITestRunner.ShowTestConfigDialog(TestMode.SequentialTraversal);
                    return;
                }
                if (keycode == KEY_F6)
                {
                    UITestRunner.ShowTestConfigDialog(TestMode.RandomTraversal);
                    return;
                }
                if (keycode == KEY_F7)
                {
                    UITestRunner.ShowTestConfigDialog(TestMode.StressTest);
                    return;
                }
            }
            
            bool selected = UI.IsPreviewWindowSelected(_previewWindowId);
            GameState currentState = _gameScene != null ? _gameScene.GetGameState() : GameState.Editing;
            
            // Delete key: delete selected entity (with confirmation dialog)
            if (keycode == KEY_DELETE && pressed)
            {
                if (_selectedEntityId != 0 && currentState == GameState.Editing)
                {
                    ShowDeleteConfirmDialog();
                    return;
                }
            }
            
            if (keycode == KEY_ESC && pressed)
            {
                if (currentState == GameState.Running || currentState == GameState.Paused)
                {
                    _gameScene.SetGameState(GameState.Editing);
                    UI.SetRendererGameState(0);
                    UI.SetPreviewWindowEditMode(_previewWindowId, true);
                    UI.SetPreviewWindowSelected(_previewWindowId, false);
                    UI.SetText(_runButtonId, "运行");
                    // Hide pause button — move off-screen since it's a direct toolbar child
                    UI.SetWidgetLayout(_pauseButtonId, -100f, -100f, 80f, 30f);
                    // Reset preview border
                    UI.ThemeSetBackground(_previewWindowId, ThemeRole.Canvas);
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
                    Log.Info("Editor", "ESC: Running/Paused → Editing");
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
            
            if (!selected && currentState != GameState.Running) return;
            
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
                    // Use absolute preview window layout (screen coordinates match mouse coordinates)
                    float[] previewLayout = UI.WidgetGetAbsoluteLayout(_previewWindowId);
                    if (previewLayout == null || previewLayout.Length < 4) return;
                    
                    float previewX = previewLayout[0];
                    float previewY = previewLayout[1];
                    float previewWidth = previewLayout[2];
                    float previewHeight = previewLayout[3];
                    
                    if (previewWidth <= 0 || previewHeight <= 0) return;
                    
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
                        _statusItem.Text = "选中Entity: " + hitEntity;
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
            
            // Bug1 fix: 保存当前active tab index，防止UI重建时跳转到其他标签页
            ulong savedActiveTab = _propsTabWidget.GetActive();
            Log.Info("Editor", "[UpdatePropertiesPanel] ENTRY: savedActiveTab=" + savedActiveTab + ", _propertiesDirty=" + _propertiesDirty + ", _lastScriptBindingCount=" + _lastScriptBindingCount);
            
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
            
            // Bug1 fix: 如果刚添加了新脚本绑定（scriptCount > 0），强制切换到运动标签页(index=3)
            // 不能仅恢复savedActiveTab，因为首次启动时savedActiveTab=0(几何)
            int currentScriptCount = _gameScene.GetScriptBindingCount(entityId);
            if (currentScriptCount > 0)
            {
                _propsTabWidget.SetActive(3); // 运动 = index 3
                Log.Info("Editor", "[UpdatePropertiesPanel] forced SetActive(3) because scriptCount=" + currentScriptCount);
            }
            else
            {
                Log.Info("Editor", "[UpdatePropertiesPanel] EXIT: restoring savedActiveTab=" + savedActiveTab + " (no scripts)");
                _propsTabWidget.SetActive(savedActiveTab);
            }
        }
        
        // === 脚本属性runtime值→UI同步 (每5帧执行一次) ===
        // 防止循环更新: 仅值改变时更新UI, Slider/Input回调中同步更新_prevScriptPropertyValues
        private static void SyncScriptPropertyValuesToUI()
        {
            _updateFrameCount++;
            
            // 每5帧执行一次同步, 减少FFI开销
            if (_updateFrameCount % 5 != 0) return;
            
            // 仅在Editing状态 + 有选中Entity + 有脚本属性时执行
            if (_selectedEntityId == 0 || _scriptPropertyInfoMap.Count == 0) return;
            if (_gameScene == null || _gameScene.GetGameState() != GameState.Editing) return;
            
            foreach (KeyValuePair<ulong, ScriptPropertyInfo> entry in _scriptPropertyInfoMap)
            {
                ScriptPropertyInfo propInfo = entry.Value;
                ulong mainWidgetId = propInfo.MainWidgetId;
                
                // 跳过未创建C#实例的属性(instanceId==0)
                if (propInfo.InstanceId == 0) continue;
                
                // 跳过未注册类型
                Type scriptType;
                if (!_scriptTypeRegistry.TryGetValue(propInfo.ClassName, out scriptType)) continue;
                
                IntPtr instancePtr = new IntPtr(propInfo.InstanceId);
                if (instancePtr == IntPtr.Zero) continue;
                
                // 读取当前runtime值
                float runtimeValue = HezhouScripts.ScriptEntityHelper.GetFieldValue(instancePtr, propInfo.PropertyName, scriptType);
                
                // 与_prevScriptPropertyValues对比 — 值未变化时不更新(减少FFI开销)
                float prevValue;
                if (_prevScriptPropertyValues.TryGetValue(mainWidgetId, out prevValue))
                {
                    // 值相同则跳过(防止循环更新)
                    if (runtimeValue == prevValue) continue;
                }
                
                // 值变化 → 更新UI
                if (propInfo.WidgetType == "slider")
                {
                    UI.SliderSetValue(mainWidgetId, runtimeValue);
                }
                else
                {
                    // "input" 或其他 → InputFieldSetText
                    UI.InputFieldSetText(mainWidgetId, runtimeValue.ToString());
                }
                
                // 更新_prevScriptPropertyValues
                _prevScriptPropertyValues[mainWidgetId] = runtimeValue;
            }
        }
        
        /// <summary>
        /// 清空运动/脚本标签页中属于旧场景的绑定行与属性控件。
        /// 仅清场景不清这些控件会留下“幽灵绑定”（新场景仍显示上一个实体的脚本）。
        /// </summary>
        private static void ClearScriptBindingUI()
        {
            // 先按容器实际子节点清理，避免只移除父 HStack 后子 Label/Button 变成孤儿。
            if (_scriptsListContainerId != 0)
            {
                while (UI.WidgetGetChildCount(_scriptsListContainerId) > 0)
                {
                    ulong childId = UI.WidgetGetChildId(_scriptsListContainerId, 0);
                    if (childId == 0) break;
                    UI.RemoveWidget(childId);
                }
            }

            for (int i = 0; i < _scriptRowIds.Count; i++)
            {
                UI.RemoveWidget(_scriptRowIds[i]);
            }
            _scriptRowIds.Clear();

            for (int i = 0; i < _scriptPropertyWidgetIds.Count; i++)
            {
                UI.RemoveWidget(_scriptPropertyWidgetIds[i]);
            }
            _scriptPropertyWidgetIds.Clear();

            _scriptPropertyInfoMap.Clear();
            _scriptPropertySliderCallbacks.Clear();
            _scriptPropertyInputCallbacks.Clear();
            _scriptMinMaxInitialToMainWidgetMap.Clear();
            _removeScriptBtnIndices.Clear();
            _scriptToggleBtnIndices.Clear();
            _lastScriptBindingCount = -1;
        }

        private static void ClearPropertiesPanel()
        {
            if (_propsTabWidget == null) return;
            
            _selectedEntityId = 0;
            _selectedEntityIds.Clear();
            UpdateSelectionStatus();
            ClearScriptBindingUI();
            ClearEntityNodeSelectionHighlights();
            
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
                CaptureUndoSnapshot();
                List<ulong> targets = GetSelectedEntityIdsForEdit();
                for (int i = 0; i < targets.Count; i++)
                {
                    ulong entityId = targets[i];
                    float x, y, z;
                    if (!UI.EntityGetPropertyValueFloat3(_gameScene.ScenePtr, entityId, propertyName, out x, out y, out z))
                    {
                        continue;
                    }
                    switch (componentIndex)
                    {
                        case 0: x = value; break;
                        case 1: y = value; break;
                        case 2: z = value; break;
                    }
                    UI.EntitySetPropertyValueFloat3(_gameScene.ScenePtr, entityId, propertyName, x, y, z);
                }
                _propertiesDirty = true;
            }
        }

        /// <summary>
        /// 批量编辑目标：多选时对全部选中实体生效；否则回退到主选中实体。
        /// </summary>
        private static List<ulong> GetSelectedEntityIdsForEdit()
        {
            if (_selectedEntityIds.Count > 0)
            {
                return new List<ulong>(_selectedEntityIds);
            }
            List<ulong> single = new List<ulong>();
            if (_selectedEntityId != 0) single.Add(_selectedEntityId);
            return single;
        }
        
        private static void HandleStringPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            CaptureUndoSnapshot();
            List<ulong> targets = GetSelectedEntityIdsForEdit();
            for (int i = 0; i < targets.Count; i++)
            {
                ulong entityId = targets[i];
                UI.EntitySetPropertyValueString(_gameScene.ScenePtr, entityId, propertyName, text);
                if (propertyName == "name")
                {
                    UpdateEntityNameInTree(entityId, text);
                }
            }
            _propertiesDirty = true;
        }
        
        private static void HandleIntPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (int.TryParse(text, out int value))
            {
                CaptureUndoSnapshot();
                List<ulong> targets = GetSelectedEntityIdsForEdit();
                for (int i = 0; i < targets.Count; i++)
                {
                    UI.EntitySetPropertyValueString(_gameScene.ScenePtr, targets[i], propertyName, value.ToString());
                }
                _propertiesDirty = true;
            }
        }

        private static void HandleFloatPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float value))
            {
                CaptureUndoSnapshot();
                List<ulong> targets = GetSelectedEntityIdsForEdit();
                for (int i = 0; i < targets.Count; i++)
                {
                    UI.EntitySetPropertyValueFloat(_gameScene.ScenePtr, targets[i], propertyName, value);
                }
                _propertiesDirty = true;
            }
        }

        private static void HandleBoolPropertyChange(string propertyName, ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            string lowerText = text.ToLower();
            string boolValue = null;
            if (lowerText == "true" || lowerText == "1") boolValue = "true";
            else if (lowerText == "false" || lowerText == "0") boolValue = "false";
            if (boolValue == null) return;

            CaptureUndoSnapshot();
            List<ulong> targets = GetSelectedEntityIdsForEdit();
            for (int i = 0; i < targets.Count; i++)
            {
                UI.EntitySetPropertyValueString(_gameScene.ScenePtr, targets[i], propertyName, boolValue);
            }
            _propertiesDirty = true;
        }
        
        private static void UpdateScriptBindingsList(ulong entityId)
        {
            if (_gameScene == null || _scriptsListContainerId == 0) return;
            
            int scriptCount = _gameScene.GetScriptBindingCount(entityId);
            
            if (scriptCount == _lastScriptBindingCount && _lastPropertiesEntityId == entityId)
            {
                return;
            }
            
            // 增量更新: 如果只是新增了一个脚本绑定，不重建全部控件（避免UI跳转）
            bool incrementalAdd = (scriptCount == _lastScriptBindingCount + 1) 
                && _lastPropertiesEntityId == entityId 
                && _scriptRowIds.Count == _lastScriptBindingCount;
            
            _lastScriptBindingCount = scriptCount;
            _removeScriptBtnIndices.Clear();
            _scriptToggleBtnIndices.Clear();
            
            if (incrementalAdd)
            {
                // 只添加新的绑定行和属性控件，不清除现有控件
                int newIndex = scriptCount - 1;
                var info = _gameScene.GetScriptBindingInfo(entityId, newIndex);
                
                // 添加新绑定行
                CreateScriptBindingRow(entityId, newIndex, info);
                
                // 添加新脚本的属性控件
                AddScriptPropertyWidgets(entityId, newIndex, info);
            }
            else
            {
                // 完全重建: 实体切换或绑定数量变化>1时
                // Clear existing script rows before rebuilding
                for (int r = 0; r < _scriptRowIds.Count; r++)
                {
                    UI.RemoveWidget(_scriptRowIds[r]);
                }
                _scriptRowIds.Clear();
                
                // 重建脚本属性控件（清除旧的+创建新的Slider/Input）
                BuildScriptPropertyWidgets(entityId);
                
                if (scriptCount == 0)
                {
                    return;
                }
                
                for (int i = 0; i < scriptCount; i++)
                {
                    var info = _gameScene.GetScriptBindingInfo(entityId, i);
                    CreateScriptBindingRow(entityId, i, info);
                }
            }
        }
        
        // 创建单个脚本绑定行 (HStack + Label + Toggle + Remove)
        private static void CreateScriptBindingRow(ulong entityId, int bindingIndex, ScriptBindingInfo info)
        {
            string labelText = info.ClassName;
            
            ulong scriptRow = UI.CreateHStack(_scriptsListContainerId, 5f);
            UI.CreateLabel(scriptRow, 160f, 20f, labelText);
            
            // Toggle button: [ON] or [OFF] with colored background
            ulong toggleBtnId = UI.CreateButton(scriptRow, 40f, 20f, info.Enabled ? "[ON]" : "[OFF]");
            UI.SetOnClick(toggleBtnId, _scriptToggleClickCallback);
            _scriptToggleBtnIndices[toggleBtnId] = bindingIndex;
            
            if (info.Enabled)
            {
                UI.ThemeSetBackground(toggleBtnId, ThemeRole.Success);
            }
            else
            {
                UI.ThemeSetBackground(toggleBtnId, ThemeRole.Danger);
            }
            
            ulong removeBtnId = UI.CreateButton(scriptRow, 40f, 20f, "X");
            UI.SetOnClick(removeBtnId, _removeScriptClickCallback);
            _removeScriptBtnIndices[removeBtnId] = bindingIndex;
            _scriptRowIds.Add(scriptRow);
        }
        
        // 增量添加单个脚本的属性控件 (不清除现有属性控件)
        private static void AddScriptPropertyWidgets(ulong entityId, int bindingIndex, ScriptBindingInfo info)
        {
            Type scriptType = null;
            if (!_scriptTypeRegistry.TryGetValue(info.ClassName, out scriptType))
            {
                return;
            }
            
            ulong instanceId = UI.SceneGetScriptBindingInstanceId(_gameScene.ScenePtr, entityId, (ulong)bindingIndex);
            ScriptPropertyDescriptor[] descriptors = HezhouScripts.ScriptEntityHelper.ReflectProperties(scriptType);
            
            for (int d = 0; d < descriptors.Length; d++)
            {
                uint childCountBefore = UI.WidgetGetChildCount(_scriptsListContainerId);
                
                ScriptPropertyInfo propInfo = BuildSingleScriptProperty(
                    _scriptsListContainerId, descriptors[d], entityId, bindingIndex, instanceId, scriptType);
                
                uint childCountAfter = UI.WidgetGetChildCount(_scriptsListContainerId);
                for (uint c = childCountBefore; c < childCountAfter; c++)
                {
                    ulong newChildId = UI.WidgetGetChildId(_scriptsListContainerId, c);
                    _scriptPropertyWidgetIds.Add(newChildId);
                }
                
                // === 注册主控件回调 ===
                if (descriptors[d].Widget == "slider")
                {
                    UI.SliderChangeCallbackDelegate sliderCallback =
                        new UI.SliderChangeCallbackDelegate(OnScriptPropertySliderChange);
                    _scriptPropertySliderCallbacks[propInfo.MainWidgetId] = sliderCallback;
                    UI.SliderSetOnChange(propInfo.MainWidgetId, sliderCallback);
                }
                else
                {
                    UI.InputFieldChangeCallbackDelegate inputCallback =
                        new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyInputChange);
                    _scriptPropertyInputCallbacks[propInfo.MainWidgetId] = inputCallback;
                    UI.InputFieldSetOnChange(propInfo.MainWidgetId, inputCallback);
                }
                
                // === 注册 Min/Max/Step/Initial InputField回调 ===
                UI.InputFieldChangeCallbackDelegate minCallback =
                    new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                _scriptPropertyInputCallbacks[propInfo.MinInputId] = minCallback;
                UI.InputFieldSetOnChange(propInfo.MinInputId, minCallback);
                
                UI.InputFieldChangeCallbackDelegate maxCallback =
                    new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                _scriptPropertyInputCallbacks[propInfo.MaxInputId] = maxCallback;
                UI.InputFieldSetOnChange(propInfo.MaxInputId, maxCallback);
                
                UI.InputFieldChangeCallbackDelegate stepCallback =
                    new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                _scriptPropertyInputCallbacks[propInfo.StepInputId] = stepCallback;
                UI.InputFieldSetOnChange(propInfo.StepInputId, stepCallback);
                
                UI.InputFieldChangeCallbackDelegate initialCallback =
                    new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                _scriptPropertyInputCallbacks[propInfo.InitialInputId] = initialCallback;
                UI.InputFieldSetOnChange(propInfo.InitialInputId, initialCallback);
            }
        }

        // === Script Property Widget State (for removal tracking) ===
        private static List<ulong> _scriptPropertyWidgetIds = new List<ulong>();

        // === Build Script Property Widgets: 遍历Entity脚本绑定→反射[Expose]属性→创建Slider/Input控件 ===

        private static void BuildScriptPropertyWidgets(ulong entityId)
        {
            // 移除旧的属性控件（Label, Slider/Input, ConfigHStack及其子控件）
            for (int i = 0; i < _scriptPropertyWidgetIds.Count; i++)
            {
                UI.RemoveWidget(_scriptPropertyWidgetIds[i]);
            }
            _scriptPropertyWidgetIds.Clear();

            // 清空属性数据字典（重建前必须清除旧数据）
            _scriptPropertyInfoMap.Clear();
            _scriptPropertySliderCallbacks.Clear();
            _scriptPropertyInputCallbacks.Clear();
            _scriptMinMaxInitialToMainWidgetMap.Clear();

            // ★ 诊断日志: 函数入口
            Log.Info("Editor", "[BuildScriptPropertyWidgets] entityId=" + entityId
                + ", _gameScene=" + (_gameScene != null ? "ptr=" + _gameScene.ScenePtr.ToInt64() : "null")
                + ", _scriptsListContainerId=" + _scriptsListContainerId);

            if (_gameScene == null || _scriptsListContainerId == 0) return;

            int scriptCount = _gameScene.GetScriptBindingCount(entityId);
            Log.Info("Editor", "[BuildScriptPropertyWidgets] scriptCount=" + scriptCount);

            for (int i = 0; i < scriptCount; i++)
            {
                ScriptBindingInfo bindingInfo = _gameScene.GetScriptBindingInfo(entityId, i);

                // ★ 诊断日志: binding信息
                Log.Info("Editor", "[BuildScriptPropertyWidgets] binding[" + i + "]: ClassName='" + bindingInfo.ClassName + "', ScriptPath='" + bindingInfo.ScriptPath + "', Enabled=" + bindingInfo.Enabled);

                // 查找脚本类型 — 未注册的类型跳过（无法反射属性）
                Type scriptType = null;
                if (!_scriptTypeRegistry.TryGetValue(bindingInfo.ClassName, out scriptType))
                {
                    // ★ 诊断日志: 类型查找失败
                    Log.Error("Editor", "[BuildScriptPropertyWidgets] 类型查找失败: ClassName='" + bindingInfo.ClassName + "' 不在_registry中! registryKeys=" + _scriptTypeRegistry.Keys.Count);
                    continue;
                }

                // 获取instance_id（0表示尚未创建C#实例）
                ulong instanceId = UI.SceneGetScriptBindingInstanceId(_gameScene.ScenePtr, entityId, (ulong)i);
                Log.Info("Editor", "[BuildScriptPropertyWidgets] binding[" + i + "]: scriptType=" + scriptType.Name + ", instanceId=" + instanceId);

                // 反射[Expose]属性 → ScriptPropertyDescriptor[]
                ScriptPropertyDescriptor[] descriptors = HezhouScripts.ScriptEntityHelper.ReflectProperties(scriptType);
                Log.Info("Editor", "[BuildScriptPropertyWidgets] ReflectProperties(" + scriptType.Name + ") → " + descriptors.Length + " descriptors");

                for (int d = 0; d < descriptors.Length; d++)
                {
                    Log.Info("Editor", "[BuildScriptPropertyWidgets] descriptor[" + d + "]: Name='" + descriptors[d].Name + "', Widget='" + descriptors[d].Widget + "', Min=" + descriptors[d].Min + ", Max=" + descriptors[d].Max);

                    // 记录创建前的子控件数量，用于追踪新创建的属性控件
                    uint childCountBefore = UI.WidgetGetChildCount(_scriptsListContainerId);

                    // 创建属性控件（Label + Slider/Input + Min/Max/Step/Initial HStack）
                    // BuildSingleScriptProperty内部填充_scriptPropertyInfoMap和_scriptMinMaxInitialToMainWidgetMap
                    ScriptPropertyInfo propInfo = BuildSingleScriptProperty(
                        _scriptsListContainerId, descriptors[d], entityId, i, instanceId, scriptType);

                    Log.Info("Editor", "[BuildScriptPropertyWidgets] BuildSingleScriptProperty返回: MainWidgetId=" + propInfo.MainWidgetId + ", PropertyName='" + propInfo.PropertyName + "'");

                    // 追踪新创建的直接子控件（Label, MainWidget, ConfigHStack）
                    // 移除这些直接子控件时，其嵌套子控件（HStack内的Label/Input）也会被移除
                    uint childCountAfter = UI.WidgetGetChildCount(_scriptsListContainerId);
                    for (uint c = childCountBefore; c < childCountAfter; c++)
                    {
                        ulong newChildId = UI.WidgetGetChildId(_scriptsListContainerId, c);
                        _scriptPropertyWidgetIds.Add(newChildId);
                    }

                    // === 注册主控件回调 ===
                    if (descriptors[d].Widget == "slider")
                    {
                        // Slider回调: void(ulong widgetId, float value)
                        UI.SliderChangeCallbackDelegate sliderCallback =
                            new UI.SliderChangeCallbackDelegate(OnScriptPropertySliderChange);
                        _scriptPropertySliderCallbacks[propInfo.MainWidgetId] = sliderCallback;
                        UI.SliderSetOnChange(propInfo.MainWidgetId, sliderCallback);
                    }
                    else
                    {
                        // InputField回调: void(ulong widgetId, string text)
                        UI.InputFieldChangeCallbackDelegate inputCallback =
                            new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyInputChange);
                        _scriptPropertyInputCallbacks[propInfo.MainWidgetId] = inputCallback;
                        UI.InputFieldSetOnChange(propInfo.MainWidgetId, inputCallback);
                    }

                    // === 注册 Min/Max/Step/Initial InputField回调 ===
                    // 配置参数修改回调 — Task 4将增强: ModifyScriptSourceFile + Slider范围更新
                    UI.InputFieldChangeCallbackDelegate minCallback =
                        new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                    _scriptPropertyInputCallbacks[propInfo.MinInputId] = minCallback;
                    UI.InputFieldSetOnChange(propInfo.MinInputId, minCallback);

                    UI.InputFieldChangeCallbackDelegate maxCallback =
                        new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                    _scriptPropertyInputCallbacks[propInfo.MaxInputId] = maxCallback;
                    UI.InputFieldSetOnChange(propInfo.MaxInputId, maxCallback);

                    UI.InputFieldChangeCallbackDelegate stepCallback =
                        new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                    _scriptPropertyInputCallbacks[propInfo.StepInputId] = stepCallback;
                    UI.InputFieldSetOnChange(propInfo.StepInputId, stepCallback);

                    UI.InputFieldChangeCallbackDelegate initialCallback =
                        new UI.InputFieldChangeCallbackDelegate(OnScriptPropertyConfigInputChange);
                    _scriptPropertyInputCallbacks[propInfo.InitialInputId] = initialCallback;
                    UI.InputFieldSetOnChange(propInfo.InitialInputId, initialCallback);
                }
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
            
            CaptureUndoSnapshot();
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
        
        private static bool IsPointInsideEntityNode(ulong entityId, float x, float y)
        {
            if (_entityNodeMap.TryGetValue(entityId, out ulong nodeId))
            {
                float[] layout = UI.WidgetGetAbsoluteLayout(nodeId);
                if (layout != null && layout.Length >= 4)
                {
                    return x >= layout[0] && x <= layout[0] + layout[2]
                        && y >= layout[1] && y <= layout[1] + layout[3];
                }
            }
            return false;
        }

        private static ulong FindEntityAtPoint(float x, float y)
        {
            foreach (var entry in _entityNodeMap)
            {
                float[] layout = UI.WidgetGetAbsoluteLayout(entry.Value);
                if (layout == null || layout.Length < 4) continue;
                if (x >= layout[0] && x <= layout[0] + layout[2]
                    && y >= layout[1] && y <= layout[1] + layout[3])
                {
                    return entry.Key;
                }
            }
            return 0;
        }

        private static void ResetHierarchyDrag()
        {
            _hierarchyDragSource = 0;
            _hierarchyDragTarget = 0;
            _hierarchyDragActive = false;
        }

        private static void PerformHierarchyDrop(ulong source, ulong target)
        {
            if (_gameScene == null || source == 0 || target == 0 || source == target) return;
            _gameScene.SetParent(source, target);
            bool ok = _gameScene.GetParent(source) == target;
            RefreshEntityTree();
            SelectEntity(source);
            if (_statusItem != null)
            {
                _statusItem.Text = ok ? "已拖拽设置父子层级" : "拖拽失败：不能形成循环层级";
            }
        }

        private static void UpdateSelectionStatus()
        {
            if (_selectionItem != null) _selectionItem.Text = "选择: " + _selectedEntityIds.Count;
        }

        private static void SelectEntity(ulong entityId)
        {
            if (_gameScene == null) return;
            _gameScene.SelectEntity(entityId);
            _selectedEntityId = entityId;
            _selectedEntityIds.Clear();
            _selectedEntityIds.Add(entityId);
            UpdateSelectionStatus();
            ClearEntityNodeSelectionHighlights();
            UpdatePropertiesPanel(entityId);
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeViewSetSelected(_projectTreeViewId, nodeId);
                UI.TreeNodeSetSelected(nodeId, true);
            }
        }

        /// <summary>
        /// Ctrl+Click 多选：保持 Rust 侧主选中项为最后点击的实体，
        /// C# 侧维护完整 id 列表用于批量属性编辑。
        /// </summary>
        private static void ToggleEntitySelection(ulong entityId)
        {
            if (_gameScene == null || entityId == 0) return;

            if (_selectedEntityIds.Contains(entityId))
            {
                _selectedEntityIds.Remove(entityId);
                _gameScene.RemoveFromSelection(entityId);
                if (_selectedEntityIds.Count == 0)
                {
                    UI.SetSelectedEntity(0, false);
                    _selectedEntityId = 0;
                    ClearPropertiesPanel();
                    ClearEntityNodeSelectionHighlights();
                    return;
                }
                _selectedEntityId = _selectedEntityIds[_selectedEntityIds.Count - 1];
            }
            else
            {
                if (_selectedEntityIds.Count == 0)
                {
                    _gameScene.SelectEntity(entityId);
                }
                else
                {
                    _gameScene.AddToSelection(entityId);
                }
                _selectedEntityIds.Add(entityId);
                _selectedEntityId = entityId;
            }

            UpdateSelectionStatus();
            for (int i = 0; i < _selectedEntityIds.Count; i++)
            {
                if (_entityNodeMap.TryGetValue(_selectedEntityIds[i], out ulong nodeId))
                {
                    UI.TreeNodeSetSelected(nodeId, true);
                }
            }
            UpdatePropertiesPanel(_selectedEntityId);
            _statusItem.Text = "已选中 " + _selectedEntityIds.Count + " 个实体";
        }

        /// <summary>Shift 范围选择：按当前树可见顺序选中 anchor..target。</summary>
        private static void SelectEntityRange(ulong anchor, ulong target)
        {
            int a = _entityDisplayOrder.IndexOf(anchor);
            int b = _entityDisplayOrder.IndexOf(target);
            if (a < 0 || b < 0)
            {
                SelectEntity(target);
                _selectionAnchorEntityId = target;
                return;
            }
            int start = Math.Min(a, b);
            int end = Math.Max(a, b);
            ulong primary = target;
            _selectedEntityIds.Clear();
            _gameScene.SelectEntity(_entityDisplayOrder[start]);
            _selectedEntityIds.Add(_entityDisplayOrder[start]);
            for (int i = start + 1; i <= end; i++)
            {
                ulong id = _entityDisplayOrder[i];
                _gameScene.AddToSelection(id);
                _selectedEntityIds.Add(id);
            }
            _selectedEntityId = primary;
            UpdateSelectionStatus();
            ClearEntityNodeSelectionHighlights();
            UpdateSelectionStatus();
            for (int i = 0; i < _selectedEntityIds.Count; i++)
            {
                if (_entityNodeMap.TryGetValue(_selectedEntityIds[i], out ulong nodeId))
                {
                    UI.TreeNodeSetSelected(nodeId, true);
                }
            }
            _propertiesDirty = true;
            _lastPropertiesEntityId = 0;
            _lastScriptBindingCount = -1;
            UpdatePropertiesPanel(primary);
            if (_statusItem != null) _statusItem.Text = "范围选择: " + (end - start + 1) + " 个实体";
        }

        private static void ClearEntityNodeSelectionHighlights()
        {
            foreach (var entry in _entityNodeMap)
            {
                UI.TreeNodeSetSelected(entry.Value, false);
            }
        }

        private static void AddEntityToTree(ulong entityId, string name)
        {
            if (!_entityDisplayOrder.Contains(entityId)) _entityDisplayOrder.Add(entityId);
            if (_projectTreeViewId == 0 || _entitiesNodeId == 0) return;

            ulong parentNodeId = _entitiesNodeId;
            ulong parentEntityId = _gameScene != null ? _gameScene.GetParent(entityId) : 0;
            if (parentEntityId != 0 && _entityNodeMap.TryGetValue(parentEntityId, out ulong parentNode))
            {
                parentNodeId = parentNode;
            }

            ulong nodeId = UI.TreeViewAddNode(_projectTreeViewId, parentNodeId, "🔷 " + name, entityId, false);
            _entityNodeMap[entityId] = nodeId;
            _nodeIdToName[nodeId] = name;
            _expandedNodeNames.Add(name);
        }

        /// <summary>
        /// 根据 Rust Scene 的 parent/children 关系重建实体层级树。
        /// 用于父子关系变更、删除父实体、工程加载后的 UI 同步。
        /// </summary>
        private static void RefreshEntityTree()
        {
            if (_projectTreeViewId == 0 || _entitiesNodeId == 0 || _gameScene == null) return;

            List<ulong> oldEntityIds = new List<ulong>(_entityNodeMap.Keys);
            for (int i = 0; i < oldEntityIds.Count; i++)
            {
                RemoveEntityFromTree(oldEntityIds[i]);
            }
            _entityNodeMap.Clear();
            _nodeIdToName.Clear();
            _entityDisplayOrder.Clear();

            if (!string.IsNullOrEmpty(_projectSearchQuery))
            {
                // 搜索模式：扁平显示匹配实体，避免父节点被过滤后子节点无处挂载。
                int entityCount = _gameScene.GetEntityCount();
                for (int i = 0; i < entityCount; i++)
                {
                    ulong eid = _gameScene.GetEntityId(i);
                    string name = UI.SceneGetEntityName(_gameScene.ScenePtr, eid);
                    if (name != null && name.IndexOf(_projectSearchQuery, StringComparison.OrdinalIgnoreCase) >= 0)
                    {
                        AddEntityToTree(eid, name);
                    }
                }
                return;
            }

            uint rootCount = UI.SceneRootEntityCount(_gameScene.ScenePtr);
            for (uint i = 0; i < rootCount; i++)
            {
                ulong eid = UI.SceneGetRootEntityIdAt(_gameScene.ScenePtr, i);
                if (eid != 0)
                {
                    AddEntityNodeRecursive(eid);
                }
            }
        }

        private static void OnProjectSearchChange(ulong widgetId, string text)
        {
            _projectSearchQuery = text ?? "";
            RefreshEntityTree();
        }

        private static void AddEntityNodeRecursive(ulong entityId)
        {
            string name = UI.SceneGetEntityName(_gameScene.ScenePtr, entityId);
            AddEntityToTree(entityId, name);

            ulong childCount = UI.SceneGetChildCount(_gameScene.ScenePtr, entityId);
            for (ulong i = 0; i < childCount; i++)
            {
                ulong childId = UI.SceneGetChildId(_gameScene.ScenePtr, entityId, i);
                if (childId != 0)
                {
                    AddEntityNodeRecursive(childId);
                }
            }
        }
        
        private static void UpdateEntityNameInTree(ulong entityId, string newName)
        {
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeNodeSetText(nodeId, "🔷 " + newName);
                _nodeIdToName[nodeId] = newName;
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
            // Ctrl+Click 多选时保留已有节点高亮；普通点击清除上一个高亮。
            if (!_ctrlPressed && _lastSelectedNodeId != 0 && _lastSelectedNodeId != widgetId)
            {
                UI.TreeNodeSetSelected(_lastSelectedNodeId, false);
            }
            _lastSelectedNodeId = widgetId;
            
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
                if (_shiftPressed && _selectionAnchorEntityId != 0 && _selectionAnchorEntityId != userData)
                {
                    SelectEntityRange(_selectionAnchorEntityId, userData);
                }
                else if (_ctrlPressed)
                {
                    ToggleEntitySelection(userData);
                    if (!_shiftPressed) _selectionAnchorEntityId = userData;
                }
                else
                {
                    SelectEntity(userData);
                    _selectionAnchorEntityId = userData;
                }
            }
        }

        private static ulong _contextMenuId;
        private static ulong _contextMenuTargetNodeId;
        private static ulong _contextMenuTargetEntityId;
        private static UI.PopupMenuClickCallbackDelegate _contextMenuClickCallback;

        private static void OnTreeNodeRightClick(ulong widgetId, float x, float y)
        {
            // Only show context menu for project tree (not script editor tree)
            // Check if the node corresponds to an entity in the project tree
            ulong entityId = 0;
            foreach (var entry in _entityNodeMap)
            {
                if (entry.Value == widgetId)
                {
                    entityId = entry.Key;
                    break;
                }
            }

            // Also check _fileItemPaths for script tree nodes
            if (_fileItemPaths.ContainsKey(widgetId))
            {
                _contextMenuId = UI.CreatePopupMenu(0);
                UI.PopupMenuAddItem(_contextMenuId, "Delete", "", 0);
                _contextMenuTargetNodeId = widgetId;
                _contextMenuTargetEntityId = 0;
                
                if (_contextMenuClickCallback == null)
                {
                    _contextMenuClickCallback = new UI.PopupMenuClickCallbackDelegate(OnContextMenuClick);
                }
                UI.PopupMenuSetOnClick(_contextMenuId, _contextMenuClickCallback);
                UI.PopupMenuShow(_contextMenuId, x, y);
                return;
            }

            // Show entity context menu if this is an entity node
            if (entityId != 0)
            {
                _contextMenuId = UI.CreatePopupMenu(0);
                UI.PopupMenuAddItem(_contextMenuId, "Delete Entity", "", 0);
                UI.PopupMenuAddSeparator(_contextMenuId);
                UI.PopupMenuAddItem(_contextMenuId, "设为选中实体的子级", "", 1);
                UI.PopupMenuAddItem(_contextMenuId, "脱离父级", "", 2);
                _contextMenuTargetNodeId = widgetId;
                _contextMenuTargetEntityId = entityId;
                
                if (_contextMenuClickCallback == null)
                {
                    _contextMenuClickCallback = new UI.PopupMenuClickCallbackDelegate(OnContextMenuClick);
                }
                UI.PopupMenuSetOnClick(_contextMenuId, _contextMenuClickCallback);
                UI.PopupMenuShow(_contextMenuId, x, y);
            }
        }

        private static void OnContextMenuClick(ulong popupMenuId, int actionId)
        {
            if (actionId == 0) // Delete
            {
                if (_contextMenuTargetEntityId != 0)
                {
                    // 与 Delete 键保持一致：先选中目标实体，再弹出确认对话框。
                    if (_gameScene != null)
                    {
                        _selectedEntityId = _contextMenuTargetEntityId;
                        _gameScene.SelectEntity(_contextMenuTargetEntityId);
                        UpdatePropertiesPanel(_contextMenuTargetEntityId);
                        ShowDeleteConfirmDialog();
                    }
                }
                else if (_fileItemPaths.ContainsKey(_contextMenuTargetNodeId))
                {
                    // Delete file node from tree
                    string filePath;
                    if (_fileItemPaths.TryGetValue(_contextMenuTargetNodeId, out filePath))
                    {
                        _fileItemPaths.Remove(_contextMenuTargetNodeId);
                        UI.TreeViewRemoveNode(_projectTreeViewId, _contextMenuTargetNodeId);
                    }
                }
            }
            else if (actionId == 1)
            {
                // 将右键目标设为当前选中实体的子级。
                if (_gameScene != null && _contextMenuTargetEntityId != 0 && _selectedEntityId != 0
                    && _contextMenuTargetEntityId != _selectedEntityId)
                {
                    CaptureUndoSnapshot();
                    _gameScene.SetParent(_contextMenuTargetEntityId, _selectedEntityId);
                    RefreshEntityTree();
                    if (_gameScene.GetParent(_contextMenuTargetEntityId) == _selectedEntityId)
                    {
                        _statusItem.Text = "已设置父子层级";
                    }
                    else
                    {
                        _statusItem.Text = "层级设置失败（不能形成循环）";
                    }
                }
            }
            else if (actionId == 2)
            {
                // 从父级脱离，回到根层级。
                if (_gameScene != null && _contextMenuTargetEntityId != 0)
                {
                    CaptureUndoSnapshot();
                    _gameScene.SetParent(_contextMenuTargetEntityId, 0);
                    RefreshEntityTree();
                    _statusItem.Text = "已脱离父级";
                }
            }
            UI.PopupMenuHide(popupMenuId);
        }

        private static void RefreshAssetGridView()
        {
            // Use _assetModelGridViewId (the actual GridView inside the 模型 tab)
            // _assetGridViewId is a legacy alias that is never set
            ulong gridViewId = _assetModelGridViewId != 0 ? _assetModelGridViewId : _assetGridViewId;
            if (gridViewId == 0) return;
            
            UI.GridViewClear(gridViewId);

            string[] names = new string[] { "Cube", "Sphere", "Plane", "Cylinder", "CornellBox" };
            ulong[] values = new ulong[] { 1, 2, 3, 4, 8 };
            for (int i = 0; i < names.Length; i++)
            {
                if (!string.IsNullOrEmpty(_assetSearchQuery)
                    && names[i].IndexOf(_assetSearchQuery, StringComparison.OrdinalIgnoreCase) < 0)
                {
                    continue;
                }
                int itemIndex = (int)UI.GridViewAddItem(gridViewId, names[i], values[i]);
                IconId thumb = IconId.Cube;
                if (names[i] == "Sphere") thumb = IconId.Sun;
                else if (names[i] == "Plane") thumb = IconId.Plane;
                else if (names[i] == "Cylinder") thumb = IconId.Cylinder;
                else if (names[i] == "CornellBox") thumb = IconId.Grid;
                UI.GridViewSetItemThumbnail(gridViewId, itemIndex, "icon:" + (int)thumb);
            }
        }

        private static void OnAssetSearchChange(ulong widgetId, string text)
        {
            _assetSearchQuery = text ?? "";
            RefreshAssetGridView();
        }
        
        private static string FindMaterialAtPoint(float x, float y)
        {
            foreach (var entry in _materialButtons)
            {
                float[] layout = UI.WidgetGetAbsoluteLayout(entry.Key);
                if (layout == null || layout.Length < 4) continue;
                if (x >= layout[0] && x <= layout[0] + layout[2]
                    && y >= layout[1] && y <= layout[1] + layout[3])
                    return entry.Value;
            }
            return null;
        }

        private static void OnMaterialButtonClick(ulong widgetId)
        {
            string name;
            if (!_materialButtons.TryGetValue(widgetId, out name)) return;
            AssignMaterial(_selectedEntityId, name);
        }

        private static void AssignMaterial(ulong entityId, string materialName)
        {
            if (_gameScene == null || entityId == 0 || string.IsNullOrEmpty(materialName))
            {
                if (_statusItem != null) _statusItem.Text = "请先选择实体再赋材质";
                return;
            }
            CaptureUndoSnapshot();
            UI.SceneSetEntityMaterial(_gameScene.ScenePtr, entityId, materialName);
            string entityName = UI.SceneGetEntityName(_gameScene.ScenePtr, entityId);
            if (_statusItem != null) _statusItem.Text = "材质 " + materialName + " → " + entityName;
        }

        public static void TestAssignMaterial(string materialName)
        {
            AssignMaterial(_selectedEntityId, materialName);
        }

        public static string TestGetEntityMaterial(ulong entityId)
        {
            if (_gameScene == null) return "";
            return UI.SceneGetEntityMaterial(_gameScene.ScenePtr, entityId);
        }

        private static void RefreshTextureGridView()
        {
            if (_assetTextureGridViewId == 0) return;
            UI.GridViewClear(_assetTextureGridViewId);
            int categoryCount = AssetLibrary.GetCategoryCount();
            int total = 0;
            for (int c = 0; c < categoryCount; c++)
            {
                string category = AssetLibrary.GetCategoryName(c);
                if (category == null) continue;
                if (category.IndexOf("Texture", StringComparison.OrdinalIgnoreCase) < 0
                    && category.IndexOf("纹理", StringComparison.Ordinal) < 0) continue;
                int count = AssetLibrary.GetAssetCount(c);
                for (int i = 0; i < count; i++)
                {
                    AssetInfo info = AssetLibrary.GetAssetInfo(c, i);
                    int index = (int)UI.GridViewAddItem(_assetTextureGridViewId, info.Name, info.Id);
                    string thumb = "tex:solid";
                    if (info.Name != null && info.Name.IndexOf("Checker", StringComparison.OrdinalIgnoreCase) >= 0) thumb = "tex:checker";
                    else if (info.Name != null && info.Name.IndexOf("UV", StringComparison.OrdinalIgnoreCase) >= 0) thumb = "tex:uv";
                    UI.GridViewSetItemThumbnail(_assetTextureGridViewId, index, thumb);
                    total++;
                }
            }
            _textureTabItemCount = total;
        }

        private static void OnTextureGridClick(ulong widgetId, int index, ulong userData)
        {
            int categoryCount = AssetLibrary.GetCategoryCount();
            for (int c = 0; c < categoryCount; c++)
            {
                int count = AssetLibrary.GetAssetCount(c);
                for (int i = 0; i < count; i++)
                {
                    AssetInfo info = AssetLibrary.GetAssetInfo(c, i);
                    if (info.Id == userData)
                    {
                        if (_statusItem != null) _statusItem.Text = "纹理: " + info.Name;
                        return;
                    }
                }
            }
        }

        public static int TestTextureCount() { return _textureTabItemCount; }

        private static void RefreshMaterialGridView()
        {
            if (_assetMaterialGridViewId == 0) return;
            UI.GridViewClear(_assetMaterialGridViewId);
            int categoryCount = AssetLibrary.GetCategoryCount();
            for (int c = 0; c < categoryCount; c++)
            {
                string category = AssetLibrary.GetCategoryName(c);
                if (category == null) continue;
                if (category.IndexOf("Material", StringComparison.OrdinalIgnoreCase) < 0
                    && category.IndexOf("材质", StringComparison.Ordinal) < 0) continue;
                int count = AssetLibrary.GetAssetCount(c);
                for (int i = 0; i < count; i++)
                {
                    AssetInfo info = AssetLibrary.GetAssetInfo(c, i);
                    int itemIndex = (int)UI.GridViewAddItem(_assetMaterialGridViewId, info.Name, info.Id);
                    UI.GridViewSetItemThumbnail(_assetMaterialGridViewId, itemIndex, "icon:9");
                }
            }
        }

        private static void OnMaterialGridClick(ulong widgetId, int index, ulong userData)
        {
            _selectedMaterialId = userData;
            int categoryCount = AssetLibrary.GetCategoryCount();
            for (int c = 0; c < categoryCount; c++)
            {
                int count = AssetLibrary.GetAssetCount(c);
                for (int i = 0; i < count; i++)
                {
                    AssetInfo info = AssetLibrary.GetAssetInfo(c, i);
                    if (info.Id == userData)
                    {
                        if (_statusItem != null) _statusItem.Text = "材质: " + info.Name;
                        return;
                    }
                }
            }
        }

        public static int TestMaterialCount()
        {
            int total = 0;
            int categoryCount = AssetLibrary.GetCategoryCount();
            for (int c = 0; c < categoryCount; c++)
            {
                string category = AssetLibrary.GetCategoryName(c);
                if (category == null) continue;
                if (category.IndexOf("Material", StringComparison.OrdinalIgnoreCase) >= 0
                    || category.IndexOf("材质", StringComparison.Ordinal) >= 0)
                    total += AssetLibrary.GetAssetCount(c);
            }
            return total;
        }

        public static ulong TestGetAssetTab()
        {
            return _assetTabWidget != null ? _assetTabWidget.GetActive() : 99;
        }

        public static void TestMoveAssetTab(int from, int to)
        {
            if (_assetTabWidget != null) _assetTabWidget.MoveTab(from, to);
        }

        public static void TestSelectAssetTab(ulong index)
        {
            if (_assetTabWidget != null) _assetTabWidget.SetActive(index);
        }

        private static void OnGridViewClick(ulong widgetId, int index, ulong userData)
        {
            if (_gameScene != null && userData != 0)
            {
                // userData: 1=Cube, 2=Sphere, 3=Plane, 4=Cylinder, 8=CornellBox
                // MeshType index: 0=Cube, 1=Sphere, 2=Plane, 3=Cylinder, 7=CornellBox
                int meshType = (int)userData - 1;
                // Special case: CornellBox userData=8 → meshType=7
                if (userData == 8) meshType = 7;
                CaptureUndoSnapshot();
                ulong entityId = _gameScene.CreateMeshEntity(meshType);
                if (entityId != 0)
                {
                    string[] meshNames = { "Cube", "Sphere", "Plane", "Cylinder", "Cone", "Custom", "Bunny", "CornellBox" };
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
            UI.PopupMenuShow(_fileMenuId, 10f, TOOLBAR_HEIGHT);
        }
        
        private static void OnOpenClick(ulong widgetId)
        {
            UI.PopupMenuShow(_openMenuId, 90f, TOOLBAR_HEIGHT);
        }
        
        private static void OnSaveClick(ulong widgetId)
        {
            UI.PopupMenuShow(_saveMenuId, 170f, TOOLBAR_HEIGHT);
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
                // Show pause button — position right after run button in toolbar
                UI.SetWidgetLayout(_pauseButtonId, 375f, 5f, 80f, 30f);
                UI.SetText(_pauseButtonId, "暂停");
                // Set preview border blue for Running
                UI.ThemeSetBackground(_previewWindowId, ThemeRole.SelectionBg);
                _statusItem.Text = "状态: 运行中";
            }
            else if (currentState == GameState.Running || currentState == GameState.Paused)
            {
                _gameScene.SetGameState(GameState.Editing);
                UI.SetRendererGameState(0);
                UI.SetPreviewWindowEditMode(_previewWindowId, true);
UI.SetText(_runButtonId, "运行");
                // Hide pause button — move off-screen
                UI.SetWidgetLayout(_pauseButtonId, -100f, -100f, 80f, 30f);
                // Reset preview border to default
                UI.ThemeSetBackground(_previewWindowId, ThemeRole.Canvas);
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
                    HideScriptEditor();  // 隐藏脚本编辑器，内部已调用ShowMainLayout
                }
                else
                {
                    ShowScriptEditor();  // 显示脚本编辑器，内部已调用HideMainLayout
                }
            }
            finally
            {
                _isTransitioning = false;
            }
        }

        // === Console Panel ===

        private static void OnConsoleClick(ulong widgetId)
        {
            if (_consoleDialogId != 0)
            {
                UI.DialogShow(_consoleDialogId);
                return;
            }

            ulong rootId = UI.GetRootId();
            // 逻辑坐标下窗口约 853x480（1280x720 / DPI 1.5），对话框不能超过该范围。
            float dialogWidth = 680f;
            float dialogHeight = 360f;

            _consoleDialogId = UI.CreateDialog(rootId, "Console", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_consoleDialogId, _consoleDialogResultCallback);

            ulong contentId = UI.CreateVStack(_consoleDialogId, 8f);
            UI.CreateLabel(contentId, dialogWidth - 50f, 24f,
                "日志（打开期间每 30 帧自动刷新；Warn/Error 同时进入测试报告）:");

            ulong filterRow = UI.CreateHStack(contentId, 8f);
            _consoleLevelDropdownId = UI.CreateDropdown(filterRow, 120f, 26f);
            UI.DropdownSetOptions(_consoleLevelDropdownId, new string[] { "All", "Info", "Warn+", "Error+" });
            UI.DropdownSetSelected(_consoleLevelDropdownId, 2);
            UI.DropdownSetOnSelect(_consoleLevelDropdownId, _consoleLevelSelectCallback);

            _consoleSearchInputId = UI.CreateInputField(filterRow, 220f, 26f);
            UI.InputFieldSetPlaceholder(_consoleSearchInputId, "搜索模块/消息...");
            UI.InputFieldSetOnChange(_consoleSearchInputId, _consoleSearchChangeCallback);

            _consoleRefreshBtnId = UI.CreateButton(filterRow, 80f, 26f, "刷新");
            UI.SetOnClick(_consoleRefreshBtnId, _consoleRefreshCallback);

            _consoleJumpBtnId = UI.CreateButton(filterRow, 90f, 26f, "跳转源码");
            UI.SetOnClick(_consoleJumpBtnId, _consoleJumpCallback);

            _consoleTextEditId = UI.CreateTextEdit(contentId, dialogWidth - 50f, 250f);
            RefreshConsoleText();

            UI.DialogSetContent(_consoleDialogId, contentId);
            UI.DialogAddButton(_consoleDialogId, "Close", 0);
            UI.DialogShow(_consoleDialogId);
        }

        private static void RefreshConsoleText()
        {
            if (_consoleTextEditId == 0) return;
            string logs = TestTelemetry.GetRecentLogs(300, _consoleLevelFilter, _consoleSearchQuery);
            UI.TextEditSetText(_consoleTextEditId, logs);
        }

        private static void OnConsoleRefresh(ulong widgetId)
        {
            RefreshConsoleText();
        }

        private static void OnConsoleLevelSelect(ulong widgetId, ulong index)
        {
            string[] levels = new string[] { "All", "Info", "Warn+", "Error+" };
            _consoleLevelFilter = index < (ulong)levels.Length ? levels[index] : "All";
            RefreshConsoleText();
        }

        private static void OnConsoleJumpToSource(ulong widgetId)
        {
            string sourceFile;
            int sourceLine;
            if (!TestTelemetry.GetLastErrorLocation(out sourceFile, out sourceLine))
            {
                if (_statusItem != null) _statusItem.Text = "没有可跳转的错误日志";
                return;
            }
            if (!File.Exists(sourceFile))
            {
                if (_statusItem != null) _statusItem.Text = "错误源文件不存在: " + sourceFile;
                return;
            }
            LoadFileToEditor(sourceFile);
            if (_scriptTextEditId != 0 && sourceLine > 0)
            {
                UI.TextEditSetCursorLine(_scriptTextEditId, (uint)sourceLine);
            }
            if (_statusItem != null)
            {
                _statusItem.Text = "已跳转: " + Path.GetFileName(sourceFile) + ":" + sourceLine;
            }
            UI.DialogHide(_consoleDialogId);
        }

        private static void OnConsoleSearchChange(ulong widgetId, string text)
        {
            _consoleSearchQuery = text ?? "";
            RefreshConsoleText();
        }

        /// <summary>Console 打开时每 30 帧刷新一次，避免每帧重建 TextEdit 文本。</summary>
        private static void RefreshConsoleIfOpen()
        {
            if (_consoleDialogId == 0 || _consoleTextEditId == 0) return;
            _consoleRefreshFrame++;
            if ((_consoleRefreshFrame % 30) == 0)
            {
                RefreshConsoleText();
            }
        }

        private static void OnConsoleDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_consoleDialogId);
            _consoleDialogId = 0;
            _consoleTextEditId = 0;
            _consoleLevelDropdownId = 0;
            _consoleSearchInputId = 0;
            _consoleRefreshBtnId = 0;
            _consoleRefreshFrame = 0;
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
                    // Use ShowScriptEditor() to toggle visibility (no destroy/recreate)
                    ShowScriptEditor();
                }
                
                if (_scriptTextEditId != 0)
                {
                    UI.TextEditSetText(_scriptTextEditId, content);
                    
                    if (_scriptEditorLabel != null)
                    {
                        _scriptEditorLabel.Text = "Script Editor - " + fileName;
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
            // Legacy alias - now delegates to OnBindScriptClick
            OnBindScriptClick(widgetId);
        }
        
        private static void OnBindScriptClick(ulong widgetId)
        {
            Log.Info("Editor", "[OnBindScriptClick] _selectedEntityId=" + _selectedEntityId + ", _gameScene=" + (_gameScene != null ? "ptr=" + _gameScene.ScenePtr.ToInt64() : "null") + ", _sourceFilesModified=" + _sourceFilesModified);
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
            string scriptPath = "scripts/" + scriptName;
            string className = Path.GetFileNameWithoutExtension(scriptName);

            // 重复绑定检查
            int existingCount = _gameScene.GetScriptBindingCount(_selectedEntityId);
            for (int i = 0; i < existingCount; i++)
            {
                if (_gameScene.GetScriptBindingInfo(_selectedEntityId, i).ClassName == className)
                {
                    _statusItem.Text = className + " 已绑定到此实体";
                    return;
                }
            }

            CaptureUndoSnapshot("绑定脚本 " + className);

            // 关键：binding 必须在 hot reload 之前写入 Rust Scene。
            // 旧实现只保存 pending 字段再 reload，而 reload 会重建 C# assembly，
            // pending 静态字段被重置为 0，导致“点击绑定没反应”。
            _gameScene.AttachScriptBinding(_selectedEntityId, scriptPath, className);

            // 立即创建 C# 实例并刷新 UI，不再依赖 hot reload 的 pending 状态。
            // pending 字段会在 Initialize 重建 assembly 时丢失，导致“点击没反应”。
            int bindingIndex = _gameScene.GetScriptBindingCount(_selectedEntityId) - 1;
            Type scriptType;
            if (_scriptTypeRegistry.TryGetValue(className, out scriptType) && scriptType != null)
            {
                MethodInfo setScenePtr = scriptType.GetMethod("SetScenePtr",
                    BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                if (setScenePtr != null) setScenePtr.Invoke(null, new object[] { _gameScene.ScenePtr });

                MethodInfo createInstance = scriptType.GetMethod("CreateInstance",
                    BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                if (createInstance != null)
                {
                    IntPtr instancePtr = (IntPtr)createInstance.Invoke(null, null);
                    UI.SceneSetScriptBindingInstanceId(
                        _gameScene.ScenePtr,
                        _selectedEntityId,
                        (ulong)bindingIndex,
                        (ulong)instancePtr.ToInt64());
                }
            }

            _propertiesDirty = true;
            _lastScriptBindingCount = -1;
            UpdatePropertiesPanel(_selectedEntityId);

            if (_statusItem != null)
            {
                _statusItem.Text = "已绑定 " + className;
            }
            Log.Info("Editor", "脚本绑定成功: entity=" + _selectedEntityId + ", className=" + className);

            // 只有 .cs 被修改过才需要重新编译；普通绑定即时生效。
            if (_sourceFilesModified)
            {
                SaveAllScriptPropertyValues();
                UI.TriggerHotReload();
            }
            return;
        }
        
        private static void OnRemoveScriptClick(ulong widgetId)
        {
            // Bug4 fix: 重置pending bind防止冲突
            _pendingBindEntityId = 0;
            
            if (_selectedEntityId == 0 || _gameScene == null)
            {
                Log.Error("Editor", "未选中Entity!");
                return;
            }
            
            if (_removeScriptBtnIndices.TryGetValue(widgetId, out int index))
            {
                // 销毁C#脚本实例（如果已注册且有有效instance_id）
                var info = _gameScene.GetScriptBindingInfo(_selectedEntityId, index);
                if (_scriptTypeRegistry.TryGetValue(info.ClassName, out Type scriptType))
                {
                    ulong instanceId = UI.SceneGetScriptBindingInstanceId(_gameScene.ScenePtr, _selectedEntityId, (ulong)index);
                    if (instanceId != 0)
                    {
                        try
                        {
                            IntPtr instancePtr = new IntPtr((long)instanceId);
                            
                            MethodInfo destroyMethod = scriptType.GetMethod("DestroyInstance",
                                BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                            if (destroyMethod != null)
                            {
                                destroyMethod.Invoke(null, new object[] { instancePtr });
                                Log.Info("Editor", "脚本实例销毁: className=" + info.ClassName + ", instanceId=" + instanceId);
                            }
                        }
                        catch (Exception ex)
                        {
                            Log.Error("Editor", "脚本实例销毁失败: className=" + info.ClassName + ", instanceId=" + instanceId + ", 错误=" + ex.Message);
                        }
                    }
                }
                
                CaptureUndoSnapshot();
                _gameScene.RemoveScriptBinding(_selectedEntityId, index);
                
                // 强制刷新属性面板 — 必须设_propertiesDirty=true才能绕过UpdatePropertiesPanel自身的缓存检查
                _propertiesDirty = true;
                _lastScriptBindingCount = -1;
                UpdatePropertiesPanel(_selectedEntityId);
            }
        }
        
        // === 热重载属性值保存/恢复机制 ===

        private static void SaveAllScriptPropertyValues()
        {
            if (_gameScene == null) return;
            if (_scriptPropertyInfoMap.Count == 0) return;

            try
            {
                // 清空内存存储
                _savedScriptPropertyValues.Clear();

                // 遍历_scriptPropertyInfoMap → 读取runtime值 → 保存到_savedScriptPropertyValues
                // mcs兼容: 用foreach遍历Dictionary(避免Dictionary<K,V>.Enumerator)
                foreach (KeyValuePair<ulong, ScriptPropertyInfo> entry in _scriptPropertyInfoMap)
                {
                    ulong mainWidgetId = entry.Key;
                    ScriptPropertyInfo propInfo = entry.Value;

                    // 如果instanceId==0(尚未创建C#实例)，跳过(无法读取runtime值)
                    if (propInfo.InstanceId == 0) continue;

                    // 获取scriptType — 如果未注册类型，跳过
                    if (!_scriptTypeRegistry.TryGetValue(propInfo.ClassName, out Type scriptType)) continue;
                    if (scriptType == null) continue;

                    IntPtr instancePtr = new IntPtr(propInfo.InstanceId);
                    if (instancePtr == IntPtr.Zero) continue;

                    // 用ScriptEntityHelper.GetFieldValue读取runtime值
                    float runtimeValue = HezhouScripts.ScriptEntityHelper.GetFieldValue(instancePtr, propInfo.PropertyName, scriptType);

                    SavedScriptPropertyValue saved = new SavedScriptPropertyValue();
                    saved.EntityId = propInfo.EntityId;
                    saved.BindingIndex = propInfo.BindingIndex;
                    saved.ClassName = propInfo.ClassName;
                    saved.PropertyName = propInfo.PropertyName;
                    saved.RuntimeValue = runtimeValue;
                    saved.MainWidgetId = mainWidgetId;
                    _savedScriptPropertyValues.Add(saved);
                }

                // 写入临时文件 — 格式: entityId|bindingIndex|className|propertyName|runtimeValue|mainWidgetId
                // mcs兼容: 使用StringWriter逐行写入(不使用LINQ)
                string tempFile = ".hezhou_hotreload_property";
                StringWriter writer = new StringWriter();
                for (int i = 0; i < _savedScriptPropertyValues.Count; i++)
                {
                    SavedScriptPropertyValue sv = _savedScriptPropertyValues[i];
                    writer.WriteLine(sv.EntityId + "|" + sv.BindingIndex + "|" + sv.ClassName + "|" + sv.PropertyName + "|" + sv.RuntimeValue + "|" + sv.MainWidgetId);
                }
                File.WriteAllText(tempFile, writer.ToString());
                writer.Close();

                Log.Info("Editor", "保存脚本属性值: " + _savedScriptPropertyValues.Count + " 条记录 → " + tempFile);
            }
            catch (Exception ex)
            {
                Log.Error("Editor", "SaveAllScriptPropertyValues失败: " + ex.Message);
                // 保存失败时删除临时文件+清空内存
                try { File.Delete(".hezhou_hotreload_property"); } catch {}
                _savedScriptPropertyValues.Clear();
            }
        }

        private static void RecreateAllScriptInstances()
        {
            if (_gameScene == null) return;

            int entityCount = _gameScene.GetEntityCount();

            for (int e = 0; e < entityCount; e++)
            {
                ulong eid = _gameScene.GetEntityId(e);
                int bindingCount = _gameScene.GetScriptBindingCount(eid);

                for (int b = 0; b < bindingCount; b++)
                {
                    ulong instId = UI.SceneGetScriptBindingInstanceId(_gameScene.ScenePtr, eid, (ulong)b);
                    if (instId != 0) continue; // 已有实例，跳过

                    var info = _gameScene.GetScriptBindingInfo(eid, b);
                    if (!_scriptTypeRegistry.TryGetValue(info.ClassName, out Type scriptType)) continue;

                    // 设置Scene指针
                    MethodInfo setScenePtrMethod = scriptType.GetMethod("SetScenePtr",
                        BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                    if (setScenePtrMethod != null)
                    {
                        setScenePtrMethod.Invoke(null, new object[] { _gameScene.ScenePtr });
                    }

                    // 创建实例
                    MethodInfo createInstanceMethod = scriptType.GetMethod("CreateInstance",
                        BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                    if (createInstanceMethod != null)
                    {
                        IntPtr instancePtr = (IntPtr)createInstanceMethod.Invoke(null, null);
                        ulong newInstanceId = (ulong)instancePtr.ToInt64();

                        UI.SceneSetScriptBindingInstanceId(_gameScene.ScenePtr, eid, (ulong)b, newInstanceId);

                        // 用.cs文件Initial值初始化实例字段（绕过Mono缓存导致的旧DLL默认值）
                        ScriptPropertyDescriptor[] initDescs = HezhouScripts.ScriptEntityHelper.ReflectProperties(scriptType);
                        for (int id = 0; id < initDescs.Length; id++)
                        {
                            ExposeConfigValues csCfg = ReadExposeConfigFromCsFile(info.ClassName, initDescs[id].Name);
                            float csInitial = csCfg.HasInitial ? csCfg.Initial : initDescs[id].Initial;
                            HezhouScripts.ScriptEntityHelper.SetFieldValue(instancePtr, initDescs[id].Name, csInitial, scriptType);
                        }

                        Log.Info("Editor", "热重载后重建实例: className=" + info.ClassName + ", entityId=" + eid + ", bindingIndex=" + b + ", instanceId=" + newInstanceId);
                    }
                }
            }
        }

        private static void RestoreAllScriptPropertyValues()
        {
            if (_gameScene == null) return;

            // Bug2 fix: 设置flag阻止InputFieldSetText触发的onChange回调写回.cs文件
            _isRestoringPropertyValues = true;

            string tempFile = ".hezhou_hotreload_property";

            try
            {
                // 从临时文件读取_savedScriptPropertyValues
                // 格式: entityId|bindingIndex|className|propertyName|runtimeValue|mainWidgetId
                _savedScriptPropertyValues.Clear();

                if (File.Exists(tempFile))
                {
                    string[] lines = File.ReadAllLines(tempFile);
                    for (int i = 0; i < lines.Length; i++)
                    {
                        string line = lines[i];
                        if (string.IsNullOrEmpty(line)) continue;

                        string[] parts = line.Split('|');
                        if (parts.Length < 6) continue;

                        SavedScriptPropertyValue sv = new SavedScriptPropertyValue();
                        sv.EntityId = ulong.Parse(parts[0]);
                        sv.BindingIndex = int.Parse(parts[1]);
                        sv.ClassName = parts[2];
                        sv.PropertyName = parts[3];
                        sv.RuntimeValue = float.Parse(parts[4]);
                        sv.MainWidgetId = ulong.Parse(parts[5]);
                        _savedScriptPropertyValues.Add(sv);
                    }
                }

                if (_savedScriptPropertyValues.Count == 0)
                {
                    Log.Info("Editor", "无保存的属性值，跳过恢复");
                    // 删除临时文件 — 确保无残留
                    try { File.Delete(tempFile); } catch {}
                    // Bug2 fix: 必须在return前清除flag，否则_isRestoringPropertyValues卡在true
                    _isRestoringPropertyValues = false;
                    return;
                }

                int restoredCount = 0;

                for (int i = 0; i < _savedScriptPropertyValues.Count; i++)
                {
                    try
                    {
                        SavedScriptPropertyValue savedValue = _savedScriptPropertyValues[i];

                        // 获取当前instanceId（已被RecreateAllScriptInstances重建）
                        ulong instId = UI.SceneGetScriptBindingInstanceId(_gameScene.ScenePtr, savedValue.EntityId, (ulong)savedValue.BindingIndex);
                        if (instId == 0) continue; // 尚未创建C#实例 → 跳过

                        if (!_scriptTypeRegistry.TryGetValue(savedValue.ClassName, out Type scriptType)) continue;
                        if (scriptType == null) continue; // scriptType为null → 跳过

                        IntPtr instancePtr = new IntPtr((long)instId);
                        if (instancePtr == IntPtr.Zero) continue; // instancePtr为Zero → 跳过

                        // 从新assembly的descriptor读取config值(Min/Max/Step/Initial)
                        ScriptPropertyDescriptor[] descriptors = HezhouScripts.ScriptEntityHelper.ReflectProperties(scriptType);
                        ScriptPropertyDescriptor descriptor = new ScriptPropertyDescriptor();
                        bool foundDescriptor = false;
                        for (int d = 0; d < descriptors.Length; d++)
                        {
                            if (descriptors[d].Name == savedValue.PropertyName)
                            {
                                descriptor = descriptors[d];
                                foundDescriptor = true;
                                break;
                            }
                        }
                        if (!foundDescriptor) continue; // 找不到descriptor → 跳过

                        // 从.cs文件读取config值（覆盖stale descriptor，因为Mono缓存导致descriptor可能是旧值）
                        ExposeConfigValues csConfig = ReadExposeConfigFromCsFile(savedValue.ClassName, savedValue.PropertyName);
                        float effectiveMin = csConfig.HasMin ? csConfig.Min : descriptor.Min;
                        float effectiveMax = csConfig.HasMax ? csConfig.Max : descriptor.Max;
                        float effectiveStep = csConfig.HasStep ? csConfig.Step : descriptor.Step;
                        float effectiveInitial = csConfig.HasInitial ? csConfig.Initial : descriptor.Initial;

                        // clamp越界: runtime值不能超出新.cs的Min/Max范围
                        float clampedValue = savedValue.RuntimeValue;
                        if (clampedValue > effectiveMax) clampedValue = effectiveMax;
                        if (clampedValue < effectiveMin) clampedValue = effectiveMin;

                        // 恢复runtime值到新实例
                        HezhouScripts.ScriptEntityHelper.SetFieldValue(instancePtr, savedValue.PropertyName, clampedValue, scriptType);
                        restoredCount++;

                        // 通过MainWidgetId直接查找propInfo
                        if (!_scriptPropertyInfoMap.TryGetValue(savedValue.MainWidgetId, out ScriptPropertyInfo propInfo)) continue;

                        // 更新Slider/Input显示值
                        UI.SliderSetValue(propInfo.MainWidgetId, clampedValue);

                        // 更新Slider范围和步长（从.cs文件读取）
                        UI.SliderSetRange(propInfo.MainWidgetId, effectiveMin, effectiveMax);
                        UI.SliderSetStep(propInfo.MainWidgetId, effectiveStep);

                        // 更新Min/Max/Step/Initial输入框（从.cs文件读取）
                        UI.InputFieldSetText(propInfo.MinInputId, effectiveMin.ToString());
                        UI.InputFieldSetText(propInfo.MaxInputId, effectiveMax.ToString());
                        UI.InputFieldSetText(propInfo.StepInputId, effectiveStep.ToString());
                        UI.InputFieldSetText(propInfo.InitialInputId, effectiveInitial.ToString());

                        // 更新propInfo的config值和InstanceId
                        propInfo.CurrentMin = effectiveMin;
                        propInfo.CurrentMax = effectiveMax;
                        propInfo.CurrentStep = effectiveStep;
                        propInfo.CurrentInitial = effectiveInitial;
                        propInfo.InstanceId = (long)instId;

                        // struct值类型 → 重新赋值回字典
                        _scriptPropertyInfoMap[savedValue.MainWidgetId] = propInfo;
                    }
                    catch (Exception ex)
                    {
                        Log.Error("Editor", "恢复单个属性值失败: " + ex.Message);
                    }
                }

                // 清空内存存储
                _savedScriptPropertyValues.Clear();

                // 删除临时文件 — 确保无残留
                try { File.Delete(tempFile); } catch {}

                // 重建属性面板（新descriptor值需要更新UI）
                _lastScriptBindingCount = -1;
                _propertiesDirty = true;

                Log.Info("Editor", "恢复脚本属性值: " + restoredCount + " 条记录");
            }
            catch (Exception ex)
            {
                Log.Error("Editor", "RestoreAllScriptPropertyValues失败: " + ex.Message);
                // 恢复失败时删除临时文件+清空内存
                try { File.Delete(tempFile); } catch {}
                _savedScriptPropertyValues.Clear();
            }
            
            // Bug2 fix: 清除flag，后续onChange回调正常执行
            _isRestoringPropertyValues = false;
        }

        private static void OnHotReloadComplete()
        {
            Log.Info("Editor", "[OnHotReloadComplete] _gameScene=" + (_gameScene != null ? "ptr=" + _gameScene.ScenePtr.ToInt64() + ",entityCount=" + _gameScene.GetEntityCount() : "null") + ", _pendingBindEntityId=" + _pendingBindEntityId + ", _selectedEntityId=" + _selectedEntityId);
            ScanScripts();
            
            if (_scriptDropdownId != 0)
            {
                string[] scriptOptions = _availableScripts.Count > 0 ? _availableScripts.ToArray() : new string[] { "无可用脚本" };
                UI.DropdownSetOptions(_scriptDropdownId, scriptOptions);
            }
            
            // ★ 无论是否pending bind，都要重建所有现有binding的C#实例并恢复属性值
            RecreateAllScriptInstances();
            RestoreAllScriptPropertyValues();
            
            // 重建属性面板（新descriptor值需要更新UI）
            _lastScriptBindingCount = -1;
            _propertiesDirty = true;
            
            // Bug4 fix: 如果有pending bind，在hot reload完成后执行绑定
            if (_pendingBindEntityId != 0)
            {
                _selectedEntityId = _pendingBindEntityId;
                _selectedScriptIndex = _pendingBindScriptIndex;
                
                if (_availableScripts.Count == 0 || _selectedScriptIndex >= _availableScripts.Count)
                {
                    Log.Error("Editor", "Hot reload后绑定失败: 无可用脚本!");
                    _pendingBindEntityId = 0;
                    _pendingBindScriptIndex = -1;
                    _sourceFilesModified = false;
                    if (_statusItem != null)
                    {
                        _statusItem.Text = "状态: 热更新后绑定失败";
                    }
                    return;
                }
                
                string scriptName = _availableScripts[_selectedScriptIndex];
                string scriptPath = "scripts/" + scriptName;
                string className = Path.GetFileNameWithoutExtension(scriptName);
                
                // Bug3 fix: 重复绑定检查 — 与正常流程(line 1381-1393)逻辑一致
                int existingCount = _gameScene.GetScriptBindingCount(_selectedEntityId);
                for (int j = 0; j < existingCount; j++)
                {
                    var existingInfo = _gameScene.GetScriptBindingInfo(_selectedEntityId, j);
                    if (existingInfo.ClassName == className)
                    {
                        Log.Info("Editor", "重复绑定(hot reload): entity=" + _selectedEntityId + " 已绑定 " + className + "，跳过");
                        if (_statusItem != null)
                        {
                            _statusItem.Text = className + " 已绑定到此实体";
                        }
                        _pendingBindEntityId = 0;
                        _pendingBindScriptIndex = -1;
                        _sourceFilesModified = false;
                        return;
                    }
                }
                
                // 1) Rust侧创建ScriptBinding(instance_id=0)
                _gameScene.AttachScriptBinding(_selectedEntityId, scriptPath, className);
                
                // 2) 如果脚本类型在注册表中，创建C#实例并更新instance_id
                if (_scriptTypeRegistry.TryGetValue(className, out Type scriptType))
                {
                    MethodInfo setScenePtrMethod = scriptType.GetMethod("SetScenePtr",
                        BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                    if (setScenePtrMethod != null)
                    {
                        setScenePtrMethod.Invoke(null, new object[] { _gameScene.ScenePtr });
                    }
                    
                    MethodInfo createInstanceMethod = scriptType.GetMethod("CreateInstance",
                        BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                    if (createInstanceMethod != null)
                    {
                        IntPtr instancePtr = (IntPtr)createInstanceMethod.Invoke(null, null);
                        ulong instanceId = (ulong)instancePtr.ToInt64();
                        
                        int bindingIndex = _gameScene.GetScriptBindingCount(_selectedEntityId) - 1;
                        UI.SceneSetScriptBindingInstanceId(_gameScene.ScenePtr, _selectedEntityId, (ulong)bindingIndex, instanceId);
                        
                        Log.Info("Editor", "Hot reload后脚本实例创建: className=" + className + ", instanceId=" + instanceId + ", bindingIndex=" + bindingIndex);
                    }
                }
                
                // 重置pending状态
                _pendingBindEntityId = 0;
                _pendingBindScriptIndex = -1;
                _sourceFilesModified = false;
                
                // 强制刷新属性面板
                _propertiesDirty = true;
                _lastScriptBindingCount = -1;
                UpdatePropertiesPanel(_selectedEntityId);
                
                if (_statusItem != null)
                {
                    _statusItem.Text = "状态: 就绪 (脚本已热更新并绑定)";
                }
                return;
            }
            
            // 热重载会重建 C# assembly，_selectedEntityId 静态字段被清零；
            // 从 Rust Scene 恢复当前选中项，否则新绑定行不会重建。
            _propertiesDirty = true;
            _lastScriptBindingCount = -1;

            ulong selectedAfterReload = UI.SceneGetFirstSelectedEntityId(_gameScene.ScenePtr);
            if (selectedAfterReload != 0)
            {
                _selectedEntityId = selectedAfterReload;
                _selectedEntityIds.Clear();
                _selectedEntityIds.Add(selectedAfterReload);
                UpdatePropertiesPanel(selectedAfterReload);
            }
            
            if (_statusItem != null)
            {
                _statusItem.Text = "状态: 就绪";
            }
        }
        
        private static void OnTabSelect(ulong widgetId, ulong index)
        {
            string[] tabNames = new string[] { "几何", "位置", "渲染", "运动", "物理" };
            string tabName = ((int)index < tabNames.Length) ? tabNames[(int)index] : "未知";
            _statusItem.Text = "属性页: " + tabName;
            SaveDockLayout();
        }
        
        // === Pipeline Selection Handler ===
        
        private static void OnPipelineSelected(ulong widgetId, ulong index)
        {
            if ((int)index < 0 || (int)index >= _pipelineNames.Length) return;
            string selectedPipeline = _pipelineNames[(int)index];
            
            // 切换管线
            bool success = UI.SwitchPipeline(selectedPipeline);
            if (success)
            {
                _currentPipeline = selectedPipeline;
                Log.Info("Editor", "管线切换成功: " + selectedPipeline);
                // 更新管线信息Label
                if (_pipelineInfoLabelId != 0)
                {
                    UI.SetText(_pipelineInfoLabelId, "当前管线: " + _currentPipeline);
                }
            }
            else
            {
                Log.Warn("Editor", "管线切换失败: " + selectedPipeline);
            }
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
                    SaveAllScriptPropertyValues();
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

        // === Delete Entity Confirmation Dialog ===

        private static void ShowDeleteConfirmDialog()
        {
            ulong rootId = UI.GetRootId();
            float dialogWidth = 300f;
            float dialogHeight = 150f;
            
            _deleteConfirmDialogId = UI.CreateDialog(rootId, "确认删除?", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_deleteConfirmDialogId, _deleteConfirmDialogResultCallback);
            
            ulong contentId = UI.CreateVStack(_deleteConfirmDialogId, 10f);
            int deleteCount = GetSelectedEntityIdsForEdit().Count;
            string prompt = deleteCount > 1
                ? "确定要删除选中的 " + deleteCount + " 个Entity吗?"
                : "确定要删除选中的Entity吗?";
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, prompt);
            UI.DialogSetContent(_deleteConfirmDialogId, contentId);
            UI.DialogAddButton(_deleteConfirmDialogId, "确认", 1);
            UI.DialogAddButton(_deleteConfirmDialogId, "取消", 0);
            UI.DialogShow(_deleteConfirmDialogId);
        }
        
        private static void OnDeleteConfirmDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_deleteConfirmDialogId);
            
            if (result == 1 && _selectedEntityId != 0 && _gameScene != null)
            {
                List<ulong> targets = GetSelectedEntityIdsForEdit();
                CaptureUndoSnapshot();
                for (int i = 0; i < targets.Count; i++)
                {
                    _gameScene.RemoveEntity(targets[i]);
                }
                RefreshEntityTree();
                _gameScene.ClearSelection();
                ClearPropertiesPanel();
                _selectedEntityId = 0;
                _propertiesDirty = true;
                _lastScriptBindingCount = -1;
                _statusItem.Text = "已删除 " + targets.Count + " 个 Entity";
                Log.Info("Editor", "删除Entity数量: " + targets.Count);
            }
        }

        // === Pause/Resume Handler ===

        private static void OnPauseClick(ulong widgetId)
        {
            if (_gameScene == null) return;
            
            var currentState = _gameScene.GetGameState();
            
            if (currentState == GameState.Running)
            {
                // Running → Paused
                _gameScene.SetGameState(GameState.Paused);
                UI.SetRendererGameState(2);
                UI.SetText(_pauseButtonId, "继续");
                UI.ThemeSetBackground(_previewWindowId, ThemeRole.Success);
                _statusItem.Text = "状态: 已暂停";
                Log.Info("Editor", "Pause: Running → Paused");
            }
            else if (currentState == GameState.Paused)
            {
                // Paused → Running
                _gameScene.SetGameState(GameState.Running);
                UI.SetRendererGameState(1);
                UI.SetText(_pauseButtonId, "暂停");
                UI.ThemeSetBackground(_previewWindowId, ThemeRole.SelectionBg);
                _statusItem.Text = "状态: 运行中";
                Log.Info("Editor", "Pause: Paused → Running");
            }
        }

        // === Menu Action Handlers ===

        /// <summary>
        /// 清空当前场景并创建默认的 Cube + Plane + DirectionalLight。
        /// 菜单“新建场景”和 Ctrl+N 共用该逻辑。
        /// </summary>
        private static void NewScene()
        {
            if (_gameScene == null) return;

            // 新建场景也可撤销。
            CaptureUndoSnapshot();

            // 场景即将整体替换，先销毁旧 C# 脚本实例，避免 Mono 侧残留。
            CleanupScriptInstancesBeforeSceneReplace();

            // 先复制 key 列表，RemoveEntityFromTree 会修改 _entityNodeMap，
            // 直接遍历 Keys 会抛 InvalidOperationException。
            List<ulong> oldEntityIds = new List<ulong>(_entityNodeMap.Keys);
            for (int i = 0; i < oldEntityIds.Count; i++)
            {
                RemoveEntityFromTree(oldEntityIds[i]);
            }
            _entityNodeMap.Clear();

            int count = _gameScene.GetEntityCount();
            for (int i = count - 1; i >= 0; i--)
            {
                ulong eid = _gameScene.GetEntityId(i);
                _gameScene.RemoveEntity(eid);
            }

            ulong cubeId = _gameScene.CreateCube();
            UI.SceneSetEntityName(_gameScene.ScenePtr, cubeId, "Cube");
            AddEntityToTree(cubeId, "Cube");

            ulong planeId = _gameScene.CreatePlane();
            UI.SceneSetEntityName(_gameScene.ScenePtr, planeId, "Plane");
            AddEntityToTree(planeId, "Plane");

            ulong lightId = _gameScene.CreateDirectionalLight();
            UI.SceneSetEntityName(_gameScene.ScenePtr, lightId, "DirectionalLight");
            AddEntityToTree(lightId, "DirectionalLight");

            _gameScene.ClearSelection();
            ClearPropertiesPanel();
            _selectedEntityId = 0;
            _propertiesDirty = true;
            _lastScriptBindingCount = -1;
            _statusItem.Text = "新建场景完成";
            Log.Info("Editor", "新建场景");
        }

        /// <summary>
        /// 保存当前工程；未加载工程时转为“另存为”。
        /// 菜单“保存场景”和 Ctrl+S 共用该逻辑。
        /// </summary>
        private static bool SaveCurrentProject()
        {
            if (_gameScene == null) return false;

            if (!Project.IsLoaded())
            {
                _statusItem.Text = "未加载项目，请选择另存为";
                Log.Info("Editor", "保存失败: 未加载项目");
                ShowSaveAsDialog();
                return false;
            }

            Project.SyncFromScene(_gameScene.ScenePtr);
            bool success = Project.Save();
            _statusItem.Text = success ? "项目已保存" : "保存失败";
            Log.Info("Editor", "保存场景: " + success);
            return success;
        }

        private static void OnFileMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_fileMenuId);
            if (actionId == 1)
            {
                NewScene();
            }
            else if (actionId == 2) OnNewScriptClick(0);
            else if (actionId == 3)
            {
                // 退出: no window close API available, show message
                _statusItem.Text = "请使用窗口关闭按钮退出";
                Log.Info("Editor", "退出请求（请关闭窗口）");
            }
        }
        
        private static void OnOpenMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_openMenuId);
            if (actionId == 1)
            {
                // 打开场景: show FileBrowser dialog to select .project.json
                ShowOpenSceneDialog();
            }
            else if (actionId == 2)
            {
                // 打开项目: show FileBrowser dialog, then Project.Load
                ShowOpenProjectDialog();
            }
            else if (actionId == 3) { }
        }
        
        private static void OnSaveMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_saveMenuId);
            if (actionId == 1)
            {
                SaveCurrentProject();
            }
            else if (actionId == 2) { }
            else if (actionId == 3)
            {
                // 另存为: show FileBrowser to select save path, then Project.Save
                ShowSaveAsDialog();
            }
        }

        // === Open/Save Dialog Methods ===

        private static void ShowOpenSceneDialog()
        {
            ulong rootId = UI.GetRootId();
            float dialogWidth = 500f;
            float dialogHeight = 400f;
            
            _openSceneDialogId = UI.CreateDialog(rootId, "打开场景", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_openSceneDialogId, _openSceneDialogResultCallback);
            
            ulong contentId = UI.CreateVStack(_openSceneDialogId, 10f);
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, "选择 project.json 或 *.project.json:");
            _openSceneFileBrowserId = UI.CreateFileBrowser(contentId, 10f, 10f, dialogWidth - 60f, dialogHeight - 120f, ".");
            UI.FileBrowserSetFilter(_openSceneFileBrowserId, "*.json");
            UI.DialogSetContent(_openSceneDialogId, contentId);
            UI.DialogAddButton(_openSceneDialogId, "打开", 1);
            UI.DialogAddButton(_openSceneDialogId, "取消", 0);
            UI.DialogShow(_openSceneDialogId);
        }
        
        private static void OnOpenSceneDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_openSceneDialogId);
            
            if (result == 1)
            {
                string selectedPath = GetBrowserSelectedOrCurrent(_openSceneFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath))
                {
                    // Project.Load 同时支持工程目录和直接传入 project.json / *.project.json
                    bool success = Project.Load(selectedPath);
                    if (success)
                    {
                        CleanupScriptInstancesBeforeSceneReplace();
                        Project.SyncToScene(_gameScene.ScenePtr);
                        RefreshSceneTreeAfterLoad();
                        ResetUndoHistory();
                        _statusItem.Text = "场景加载成功";
                        Log.Info("Editor", "打开场景: " + selectedPath);
                    }
                    else
                    {
                        _statusItem.Text = "场景加载失败";
                        Log.Error("Editor", "打开场景失败: " + selectedPath);
                    }
                }
                else
                {
                    _statusItem.Text = "请选择 project.json 文件或工程目录";
                }
            }
        }
        
        private static void ShowOpenProjectDialog()
        {
            ulong rootId = UI.GetRootId();
            float dialogWidth = 500f;
            float dialogHeight = 400f;
            
            _openProjectDialogId = UI.CreateDialog(rootId, "打开项目", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_openProjectDialogId, _openProjectDialogResultCallback);
            
            ulong contentId = UI.CreateVStack(_openProjectDialogId, 10f);
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, "选择项目目录:");
            _openProjectFileBrowserId = UI.CreateFileBrowser(contentId, 10f, 10f, dialogWidth - 60f, dialogHeight - 120f, ".");
            UI.FileBrowserSetFilter(_openProjectFileBrowserId, "*.json");
            UI.DialogSetContent(_openProjectDialogId, contentId);
            UI.DialogAddButton(_openProjectDialogId, "打开", 1);
            UI.DialogAddButton(_openProjectDialogId, "取消", 0);
            UI.DialogShow(_openProjectDialogId);
        }
        
        private static void OnOpenProjectDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_openProjectDialogId);
            
            if (result == 1)
            {
                string selectedPath = GetBrowserSelectedOrCurrent(_openProjectFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath))
                {
                    // Project.Load 支持工程目录和直接传入 json 文件。
                    bool success = Project.Load(selectedPath);
                    if (success)
                    {
                        CleanupScriptInstancesBeforeSceneReplace();
                        Project.SyncToScene(_gameScene.ScenePtr);
                        RefreshSceneTreeAfterLoad();
                        ResetUndoHistory();
                        _statusItem.Text = "项目加载成功: " + Project.GetName();
                        Log.Info("Editor", "打开项目: " + selectedPath);
                    }
                    else
                    {
                        _statusItem.Text = "项目加载失败";
                        Log.Error("Editor", "打开项目失败: " + selectedPath);
                    }
                }
            }
        }
        
        /// <summary>
        /// 工程/场景整体替换前，销毁所有已绑定的 C# 脚本实例。
        /// 否则旧 Entity 被 Rust Scene 清掉后，Mono 的 _instances 字典仍持有对象，
        /// 热重载/重新加载后会残留并继续被引用。
        /// </summary>
        private static void CleanupScriptInstancesBeforeSceneReplace()
        {
            if (_gameScene == null) return;

            int destroyed = 0;
            int failed = 0;
            int entityCount = _gameScene.GetEntityCount();

            for (int e = 0; e < entityCount; e++)
            {
                ulong eid = _gameScene.GetEntityId(e);
                int bindingCount = _gameScene.GetScriptBindingCount(eid);

                for (int b = 0; b < bindingCount; b++)
                {
                    ulong instanceId = UI.SceneGetScriptBindingInstanceId(
                        _gameScene.ScenePtr, eid, (ulong)b);
                    if (instanceId == 0) continue;

                    var info = _gameScene.GetScriptBindingInfo(eid, b);
                    Type scriptType;
                    if (!_scriptTypeRegistry.TryGetValue(info.ClassName, out scriptType) || scriptType == null)
                    {
                        failed++;
                        Log.Warn("Editor", "无法清理未注册脚本实例: " + info.ClassName + ", entityId=" + eid);
                        continue;
                    }

                    try
                    {
                        MethodInfo destroyMethod = scriptType.GetMethod("DestroyInstance",
                            BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                        if (destroyMethod != null)
                        {
                            destroyMethod.Invoke(null, new object[] { new IntPtr((long)instanceId) });
                            destroyed++;
                        }
                        else
                        {
                            failed++;
                        }

                        // 让旧 Scene 中的绑定不再指向已销毁实例。
                        UI.SceneSetScriptBindingInstanceId(_gameScene.ScenePtr, eid, (ulong)b, 0);
                    }
                    catch (Exception ex)
                    {
                        failed++;
                        Log.Error("Editor", "清理脚本实例失败: " + info.ClassName + ", id=" + instanceId + ", " + ex.Message);
                    }
                }
            }

            // 属性面板中的脚本控件与映射也属于旧场景，下一轮选中时重建。
            for (int i = 0; i < _scriptPropertyWidgetIds.Count; i++)
            {
                UI.RemoveWidget(_scriptPropertyWidgetIds[i]);
            }
            _scriptPropertyWidgetIds.Clear();
            _scriptPropertyInfoMap.Clear();
            _scriptPropertySliderCallbacks.Clear();
            _scriptPropertyInputCallbacks.Clear();
            _scriptMinMaxInitialToMainWidgetMap.Clear();
            _prevScriptPropertyValues.Clear();
            _savedScriptPropertyValues.Clear();
            _scriptRowIds.Clear();
            _removeScriptBtnIndices.Clear();
            _scriptToggleBtnIndices.Clear();
            _lastScriptBindingCount = -1;
            _propertiesDirty = true;

            Log.Info("Editor", "场景替换前清理脚本实例: destroyed=" + destroyed + ", failed=" + failed);
        }

        /// <summary>
        /// 工程/场景加载后重建实体树，并清理旧的选中态与属性面板。
        /// 加载会替换整个场景，旧的 entity id 已失效，不能继续保留在 UI 状态中。
        /// </summary>
        private static void RefreshSceneTreeAfterLoad()
        {
            if (_gameScene == null) return;

            _gameScene.ClearSelection();
            _selectedEntityId = 0;
            _lastPropertiesEntityId = 0;
            _propertiesDirty = true;
            _lastScriptBindingCount = -1;
            ClearPropertiesPanel();

            // 按 parent/children 递归重建，保留父子层级。
            RefreshEntityTree();

            if (_projectItem != null && Project.IsLoaded())
            {
                _projectItem.Text = "项目: " + Project.GetName();
            }
        }

        private static void ShowSaveAsDialog()
        {
            ulong rootId = UI.GetRootId();
            float dialogWidth = 500f;
            float dialogHeight = 400f;
            
            _saveAsDialogId = UI.CreateDialog(rootId, "另存为", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_saveAsDialogId, _saveAsDialogResultCallback);
            
            ulong contentId = UI.CreateVStack(_saveAsDialogId, 10f);
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, "选择保存路径:");
            _saveAsFileBrowserId = UI.CreateFileBrowser(contentId, 10f, 10f, dialogWidth - 60f, dialogHeight - 120f, ".");
            UI.FileBrowserSetFilter(_saveAsFileBrowserId, "*.json");
            UI.DialogSetContent(_saveAsDialogId, contentId);
            UI.DialogAddButton(_saveAsDialogId, "保存", 1);
            UI.DialogAddButton(_saveAsDialogId, "取消", 0);
            UI.DialogShow(_saveAsDialogId);
        }
        
        private static void OnSaveAsDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_saveAsDialogId);
            
            if (result == 1)
            {
                string selectedPath = GetBrowserSelectedOrCurrent(_saveAsFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath))
                {
                    // FileBrowser 选中的是目标父目录；工程会创建为 <父目录>/<工程名>/。
                    if (!System.IO.Directory.Exists(selectedPath))
                    {
                        selectedPath = System.IO.Path.GetDirectoryName(selectedPath);
                    }
                    if (string.IsNullOrEmpty(selectedPath) || !System.IO.Directory.Exists(selectedPath))
                    {
                        _statusItem.Text = "另存为失败: 请选择目录";
                        return;
                    }

                    string projectName = Project.IsLoaded() ? Project.GetName() : "";
                    if (string.IsNullOrEmpty(projectName))
                    {
                        projectName = "UntitledProject";
                    }

                    bool success = UI.ProjectCreateNew(projectName, selectedPath);
                    if (success)
                    {
                        Project.SyncFromScene(_gameScene.ScenePtr);
                        success = Project.Save();
                    }
                    _statusItem.Text = success ? "另存为成功" : "另存为失败";
                    Log.Info("Editor", "另存为: " + selectedPath + ", success=" + success);

                    if (success && _projectItem != null)
                    {
                        _projectItem.Text = "项目: " + Project.GetName();
                    }
                }
            }
        }

        // === Undo / Redo (场景 JSON 快照) ===

        private static void ResetUndoHistory()
        {
            _undoSnapshots.Clear();
            _redoSnapshots.Clear();
            _undoLabels.Clear();
            _redoLabels.Clear();
        }

        /// <summary>
        /// 在修改场景之前调用。保存完整场景快照，支持层级/组件/脚本绑定回滚。
        /// label 用于状态栏/按钮显示“撤销 创建实体”等可读描述。
        /// </summary>
        private static void CaptureUndoSnapshot(string label = "编辑")
        {
            if (_isRestoringSnapshot || _gameScene == null) return;

            string snapshot = UI.SceneSnapshotJson(_gameScene.ScenePtr);
            if (string.IsNullOrEmpty(snapshot)) return;

            // 连续输入时避免压入完全相同快照。
            if (_undoSnapshots.Count > 0
                && _undoSnapshots[_undoSnapshots.Count - 1] == snapshot)
            {
                return;
            }

            _undoSnapshots.Add(snapshot);
            _undoLabels.Add(label ?? "编辑");
            if (_undoSnapshots.Count > MAX_UNDO_SNAPSHOTS)
            {
                _undoSnapshots.RemoveAt(0);
                _undoLabels.RemoveAt(0);
            }
            _redoSnapshots.Clear();
            _redoLabels.Clear();
            UpdateUndoRedoButtons();
        }

        private static void Undo()
        {
            if (_undoSnapshots.Count == 0)
            {
                if (_statusItem != null) _statusItem.Text = "没有可撤销的操作";
                return;
            }

            string current = UI.SceneSnapshotJson(_gameScene.ScenePtr);
            string target = _undoSnapshots[_undoSnapshots.Count - 1];
            string label = _undoLabels.Count > 0 ? _undoLabels[_undoLabels.Count - 1] : "编辑";
            _undoSnapshots.RemoveAt(_undoSnapshots.Count - 1);
            if (_undoLabels.Count > 0) _undoLabels.RemoveAt(_undoLabels.Count - 1);
            if (!string.IsNullOrEmpty(current))
            {
                _redoSnapshots.Add(current);
                _redoLabels.Add(label);
            }

            RestoreSceneSnapshot(target);
            if (_statusItem != null) _statusItem.Text = "已撤销: " + label;
            UpdateUndoRedoButtons();
        }

        private static void Redo()
        {
            if (_redoSnapshots.Count == 0)
            {
                if (_statusItem != null) _statusItem.Text = "没有可重做的操作";
                return;
            }

            string current = UI.SceneSnapshotJson(_gameScene.ScenePtr);
            string target = _redoSnapshots[_redoSnapshots.Count - 1];
            string label = _redoLabels.Count > 0 ? _redoLabels[_redoLabels.Count - 1] : "编辑";
            _redoSnapshots.RemoveAt(_redoSnapshots.Count - 1);
            if (_redoLabels.Count > 0) _redoLabels.RemoveAt(_redoLabels.Count - 1);
            if (!string.IsNullOrEmpty(current))
            {
                _undoSnapshots.Add(current);
                _undoLabels.Add(label);
            }

            RestoreSceneSnapshot(target);
            if (_statusItem != null) _statusItem.Text = "已重做: " + label;
            UpdateUndoRedoButtons();
        }

        private static void OnUndoClick(ulong widgetId)
        {
            Undo();
        }

        private static void OnRedoClick(ulong widgetId)
        {
            Redo();
        }

        private static void UpdateUndoRedoButtons()
        {
            if (_undoButtonId != 0)
            {
                string text = _undoSnapshots.Count > 0
                    ? "撤销 " + _undoSnapshots.Count
                    : "撤销";
                UI.SetText(_undoButtonId, text);
            }
            if (_redoButtonId != 0)
            {
                string text = _redoSnapshots.Count > 0
                    ? "重做 " + _redoSnapshots.Count
                    : "重做";
                UI.SetText(_redoButtonId, text);
            }
        }

        private static void RestoreSceneSnapshot(string json)
        {
            if (_gameScene == null || string.IsNullOrEmpty(json)) return;

            _isRestoringSnapshot = true;
            try
            {
                CleanupScriptInstancesBeforeSceneReplace();
                if (!UI.SceneRestoreJson(_gameScene.ScenePtr, json))
                {
                    Log.Error("Editor", "撤销/重做快照恢复失败");
                    return;
                }

                RefreshSceneTreeAfterLoad();
                RecreateAllScriptInstances();
            }
            finally
            {
                _isRestoringSnapshot = false;
            }
        }

        // === Test Hooks (供 IntentTestRunner / 外部自动化调用，不属于运行时业务逻辑) ===

        public static int TestGetEntityCount()
        {
            return _gameScene != null ? _gameScene.GetEntityCount() : -1;
        }

        public static int TestGetGameState()
        {
            return _gameScene != null ? (int)_gameScene.GetGameState() : -1;
        }

        public static ulong TestGetSelectedEntityId()
        {
            return _selectedEntityId;
        }

        public static int TestGetSelectedCount()
        {
            return _selectedEntityIds.Count;
        }

        public static void TestShiftRangeSelect(ulong anchor, ulong target)
        {
            SelectEntityRange(anchor, target);
        }

        public static ulong TestCreateMeshEntity(int meshType)
        {
            if (_gameScene == null) return 0;

            CaptureUndoSnapshot();
            ulong entityId = _gameScene.CreateMeshEntity(meshType);
            if (entityId != 0)
            {
                string name = "IntentEntity_" + entityId;
                UI.SceneSetEntityName(_gameScene.ScenePtr, entityId, name);
                AddEntityToTree(entityId, name);
                SelectEntity(entityId);
            }
            return entityId;
        }

        public static bool TestSelectEntity(ulong entityId)
        {
            if (_gameScene == null || entityId == 0) return false;
            SelectEntity(entityId);
            return _selectedEntityId == entityId;
        }

        public static bool TestDeleteSelectedEntity()
        {
            if (_gameScene == null || _selectedEntityId == 0) return false;

            List<ulong> targets = GetSelectedEntityIdsForEdit();
            for (int i = 0; i < targets.Count; i++)
            {
                _gameScene.RemoveEntity(targets[i]);
            }
            RefreshEntityTree();
            _gameScene.ClearSelection();
            ClearPropertiesPanel();
            _selectedEntityId = 0;
            _propertiesDirty = true;
            _lastScriptBindingCount = -1;
            return _gameScene.GetEntityCount() >= 0;
        }

        public static bool TestSetGameState(int state)
        {
            if (_gameScene == null) return false;
            if (state < 0 || state > 2) return false;
            _gameScene.SetGameState((GameState)state);
            UI.SetRendererGameState(state);
            return (int)_gameScene.GetGameState() == state;
        }

        public static void TestExpandHierarchy()
        {
            if (_projectTreeViewId == 0) return;
            if (_scriptsNodeId != 0) UI.TreeViewCollapseNode(_projectTreeViewId, _scriptsNodeId);
            if (_scenesNodeId != 0) UI.TreeViewExpandNode(_projectTreeViewId, _scenesNodeId);
            if (_entitiesNodeId != 0) UI.TreeViewExpandNode(_projectTreeViewId, _entitiesNodeId);
            foreach (var entry in _entityNodeMap)
            {
                UI.TreeViewExpandNode(_projectTreeViewId, entry.Value);
            }
        }

        public static void TestHierarchyDrop(ulong source, ulong target)
        {
            PerformHierarchyDrop(source, target);
        }

        public static bool TestSetParent(ulong entityId, ulong parentId)
        {
            if (_gameScene == null || entityId == 0) return false;
            _gameScene.SetParent(entityId, parentId);
            RefreshEntityTree();
            return _gameScene.GetParent(entityId) == parentId;
        }

        public static ulong TestGetParent(ulong entityId)
        {
            return _gameScene != null ? _gameScene.GetParent(entityId) : 0;
        }

        public static int TestGetChildCount(ulong parentId)
        {
            return _gameScene != null ? (int)_gameScene.GetChildCount(parentId) : -1;
        }

        public static void TestNewScene()
        {
            NewScene();
        }

        /// <summary>测试用：按脚本文件名绑定到当前选中实体，覆盖 UI 脚本列表路径。</summary>
        public static bool TestBindScript(string fileName)
        {
            if (_gameScene == null || _selectedEntityId == 0 || string.IsNullOrEmpty(fileName))
            {
                return false;
            }

            string className = Path.GetFileNameWithoutExtension(fileName);
            string scriptPath = "scripts/" + fileName;

            for (int i = 0; i < _availableScripts.Count; i++)
            {
                if (_availableScripts[i] == fileName)
                {
                    _selectedScriptIndex = i;
                    if (_scriptDropdownId != 0)
                    {
                        UI.DropdownSetSelected(_scriptDropdownId, (ulong)i);
                    }
                    break;
                }
            }

            int existing = _gameScene.GetScriptBindingCount(_selectedEntityId);
            for (int i = 0; i < existing; i++)
            {
                if (_gameScene.GetScriptBindingInfo(_selectedEntityId, i).ClassName == className)
                {
                    return true;
                }
            }

            CaptureUndoSnapshot("绑定脚本 " + className);
            _gameScene.AttachScriptBinding(_selectedEntityId, scriptPath, className);
            int bindingIndex = _gameScene.GetScriptBindingCount(_selectedEntityId) - 1;

            Type scriptType;
            if (_scriptTypeRegistry.TryGetValue(className, out scriptType) && scriptType != null)
            {
                MethodInfo setScenePtr = scriptType.GetMethod("SetScenePtr",
                    BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                if (setScenePtr != null) setScenePtr.Invoke(null, new object[] { _gameScene.ScenePtr });

                MethodInfo createInstance = scriptType.GetMethod("CreateInstance",
                    BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                if (createInstance != null)
                {
                    IntPtr instancePtr = (IntPtr)createInstance.Invoke(null, null);
                    UI.SceneSetScriptBindingInstanceId(
                        _gameScene.ScenePtr,
                        _selectedEntityId,
                        (ulong)bindingIndex,
                        (ulong)instancePtr.ToInt64());
                }
            }

            _propertiesDirty = true;
            _lastScriptBindingCount = -1;
            UpdatePropertiesPanel(_selectedEntityId);
            return _gameScene.GetScriptBindingCount(_selectedEntityId) > 0;
        }

        public static string TestGetLastBindingClass()
        {
            if (_gameScene == null || _selectedEntityId == 0) return "";
            int count = _gameScene.GetScriptBindingCount(_selectedEntityId);
            if (count == 0) return "";
            return _gameScene.GetScriptBindingInfo(_selectedEntityId, count - 1).ClassName;
        }

        public static void TestCaptureUndo()
        {
            CaptureUndoSnapshot();
        }

        public static void TestUndo()
        {
            Undo();
        }

        public static void TestRedo()
        {
            Redo();
        }

        public static bool TestSetPosition(ulong entityId, float x, float y, float z)
        {
            if (_gameScene == null || entityId == 0) return false;
            return UI.EntitySetPropertyValueFloat3(_gameScene.ScenePtr, entityId, "position", x, y, z);
        }

        public static float[] TestGetPosition(ulong entityId)
        {
            float x = 0, y = 0, z = 0;
            if (_gameScene == null || entityId == 0) return new float[] { 0, 0, 0 };
            UI.EntityGetPropertyValueFloat3(_gameScene.ScenePtr, entityId, "position", out x, out y, out z);
            return new float[] { x, y, z };
        }

        public static void TestToggleEntitySelection(ulong entityId)
        {
            ToggleEntitySelection(entityId);
        }

        public static int TestGetSelectionCount()
        {
            return _selectedEntityIds.Count;
        }

        /// <summary>通过真实属性编辑路径批量设置选中实体的 position.x。</summary>
        public static void TestBatchSetPositionX(float x)
        {
            HandleFloat3ComponentChange("position", 0, 0, x.ToString());
        }

        public static void TestSetAssetSearch(string text)
        {
            _assetSearchQuery = text ?? "";
            if (_assetSearchInputId != 0) UI.InputFieldSetText(_assetSearchInputId, _assetSearchQuery);
            RefreshAssetGridView();
        }

        public static void TestSetProjectSearch(string text)
        {
            _projectSearchQuery = text ?? "";
            if (_projectSearchInputId != 0) UI.InputFieldSetText(_projectSearchInputId, _projectSearchQuery);
            RefreshEntityTree();
        }

        public static void TestClearSourceModifiedFlag()
        {
            _sourceFilesModified = false;
        }

        public static void TestSelectScript(string fileName)
        {
            for (int i = 0; i < _availableScripts.Count; i++)
            {
                if (_availableScripts[i] == fileName)
                {
                    _selectedScriptIndex = i;
                    if (_scriptDropdownId != 0)
                    {
                        UI.DropdownSetSelected(_scriptDropdownId, (ulong)i);
                    }
                    return;
                }
            }
        }

        public static void TestClickBindButton()
        {
            OnBindScriptClick(0);
        }

        public static void TestSetPropertyTab(ulong index)
        {
            if (_propsTabWidget != null)
            {
                _propsTabWidget.SetActive(index);
            }
        }

        public static bool TestOpenConsole()
        {
            OnConsoleClick(0);
            return _consoleDialogId != 0;
        }

        public static void TestCloseConsole()
        {
            if (_consoleDialogId != 0)
            {
                OnConsoleDialogResult(_consoleDialogId, 0);
            }
        }

        // === Script Toggle Handler ===

        private static void OnScriptToggleClick(ulong widgetId)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            
            if (_scriptToggleBtnIndices.TryGetValue(widgetId, out int scriptIndex))
            {
                var info = _gameScene.GetScriptBindingInfo(_selectedEntityId, scriptIndex);
                bool newEnabled = !info.Enabled;
                
                CaptureUndoSnapshot();
                _gameScene.SetScriptBindingEnabled(_selectedEntityId, scriptIndex, newEnabled);
                
                // Update button text and color
                if (newEnabled)
                {
                    UI.SetText(widgetId, "[ON]");
                    UI.ThemeSetBackground(widgetId, ThemeRole.Success);
                }
                else
                {
                    UI.SetText(widgetId, "[OFF]");
                    UI.ThemeSetBackground(widgetId, ThemeRole.Danger);
                }
                
                _statusItem.Text = newEnabled ? "脚本已启用: " + info.ClassName : "脚本已禁用: " + info.ClassName;
            }
        }
        
        // === Script Property Change Handlers ===
        
        private static void OnScriptPropertyInputChange(ulong widgetId, string text)
        {
            if (!_scriptPropertyInfoMap.TryGetValue(widgetId, out ScriptPropertyInfo propInfo))
            {
                return;
            }
            
            Type scriptType;
            if (!_scriptTypeRegistry.TryGetValue(propInfo.ClassName, out scriptType))
            {
                return;
            }
            
            IntPtr instancePtr = new IntPtr(propInfo.InstanceId);
            
            if (!float.TryParse(text, out float value))
            {
                // TryParse失败时恢复原值
                float currentValue = HezhouScripts.ScriptEntityHelper.GetFieldValue(instancePtr, propInfo.PropertyName, scriptType);
                UI.InputFieldSetText(widgetId, currentValue.ToString());
                return;
            }
            
            // InputField修改只改runtime值，不触发.cs写回
            HezhouScripts.ScriptEntityHelper.SetFieldValue(instancePtr, propInfo.PropertyName, value, scriptType);
            // 更新_prevScriptPropertyValues — 防止下一帧SyncScriptPropertyValuesToUI重复设置InputField值
            _prevScriptPropertyValues[widgetId] = value;
            Log.Info("Editor", "脚本属性Input修改(runtime): " + propInfo.ClassName + "." + propInfo.PropertyName + "=" + value);
        }
        
        private static void OnScriptPropertySliderChange(ulong widgetId, float value)
        {
            if (!_scriptPropertyInfoMap.TryGetValue(widgetId, out ScriptPropertyInfo propInfo))
            {
                return;
            }
            
            // Slider拖动只修改runtime值，不触发.cs写回
            if (_scriptTypeRegistry.TryGetValue(propInfo.ClassName, out Type scriptType))
            {
                IntPtr instancePtr = new IntPtr(propInfo.InstanceId);
                HezhouScripts.ScriptEntityHelper.SetFieldValue(instancePtr, propInfo.PropertyName, value, scriptType);
                // 更新_prevScriptPropertyValues — 防止下一帧SyncScriptPropertyValuesToUI重复设置Slider值
                _prevScriptPropertyValues[widgetId] = value;
                Log.Info("Editor", "脚本属性Slider修改(runtime): " + propInfo.ClassName + "." + propInfo.PropertyName + "=" + value);
            }
        }
        
        // === 脚本属性配置参数(Min/Max/Step/Initial)修改回调 ===
        // 路由: widgetId → _scriptMinMaxInitialToMainWidgetMap → mainWidgetId → _scriptPropertyInfoMap
        // Task 4将增强: ModifyScriptSourceFile写回.cs + Slider范围/步长更新
        
        private static void OnScriptPropertyConfigInputChange(ulong widgetId, string text)
        {
            // Bug2 fix: Restore期间InputFieldSetText可能触发onChange，阻止写回.cs
            if (_isRestoringPropertyValues)
            {
                Log.Info("Editor", "[OnScriptPropertyConfigInputChange] BLOCKED by _isRestoringPropertyValues=true, widgetId=" + widgetId);
                return;
            }
            Log.Info("Editor", "[OnScriptPropertyConfigInputChange] widgetId=" + widgetId + ", text=" + text);

            // 查找对应的mainWidgetId（Min/Max/Step/Initial input → main slider/input映射）
            ulong mainWidgetId;
            if (!_scriptMinMaxInitialToMainWidgetMap.TryGetValue(widgetId, out mainWidgetId))
            {
                return;
            }
            
            // 查找属性信息
            ScriptPropertyInfo propInfo;
            if (!_scriptPropertyInfoMap.TryGetValue(mainWidgetId, out propInfo))
            {
                return;
            }
            
            // 确定是哪个配置参数
            string paramName = "";
            if (widgetId == propInfo.MinInputId) paramName = "Min";
            else if (widgetId == propInfo.MaxInputId) paramName = "Max";
            else if (widgetId == propInfo.StepInputId) paramName = "Step";
            else if (widgetId == propInfo.InitialInputId) paramName = "Initial";
            
            if (paramName == "")
            {
                return;
            }
            
            // TryParse失败时恢复原值
            if (!float.TryParse(text, out float value))
            {
                // 恢复当前config值到InputField
                float currentConfigValue = 0.0f;
                if (paramName == "Min") currentConfigValue = propInfo.CurrentMin;
                else if (paramName == "Max") currentConfigValue = propInfo.CurrentMax;
                else if (paramName == "Step") currentConfigValue = propInfo.CurrentStep;
                else if (paramName == "Initial") currentConfigValue = propInfo.CurrentInitial;
                UI.InputFieldSetText(widgetId, currentConfigValue.ToString());
                return;
            }
            
            // 写回.cs源文件
            ModifyScriptSourceFile(propInfo.ClassName, propInfo.PropertyName, paramName, value);
            
            // 更新propInfo中的config值（struct是值类型，修改后必须重新赋值回字典）
            if (paramName == "Min") propInfo.CurrentMin = value;
            else if (paramName == "Max") propInfo.CurrentMax = value;
            else if (paramName == "Step") propInfo.CurrentStep = value;
            else if (paramName == "Initial") propInfo.CurrentInitial = value;
            
            // Min/Max修改: 更新Slider范围
            if (paramName == "Min" || paramName == "Max")
            {
                UI.SliderSetRange(propInfo.MainWidgetId, propInfo.CurrentMin, propInfo.CurrentMax);
            }
            
            // Step修改: 更新Slider步长
            if (paramName == "Step")
            {
                UI.SliderSetStep(propInfo.MainWidgetId, propInfo.CurrentStep);
            }
            
            // Initial修改: 更新Slider/InputField显示值和实例运行值
            if (paramName == "Initial")
            {
                if (propInfo.WidgetType == "slider")
                {
                    UI.SliderSetValue(propInfo.MainWidgetId, value);
                }
                else
                {
                    UI.InputFieldSetText(propInfo.MainWidgetId, value.ToString());
                }
                
                if (propInfo.InstanceId != 0)
                {
                    IntPtr instancePtr = new IntPtr(propInfo.InstanceId);
                    Type scriptType = null;
                    if (_scriptTypeRegistry.TryGetValue(propInfo.ClassName, out scriptType) && scriptType != null)
                    {
                        HezhouScripts.ScriptEntityHelper.SetFieldValue(instancePtr, propInfo.PropertyName, value, scriptType);
                    }
                }
            }
            
            // struct值类型 → 修改后必须重新赋值回字典
            _scriptPropertyInfoMap[mainWidgetId] = propInfo;
            
            Log.Info("Editor", "脚本属性配置修改: " + propInfo.ClassName + "." + propInfo.PropertyName + " " + paramName + "=" + value);
        }
        
        // === .cs源文件修改机制 ===

        // 从.cs文件读取[Expose]属性参数值（绕过Mono assembly缓存导致的descriptor旧值问题）
        private struct ExposeConfigValues
        {
            public float Min;
            public float Max;
            public float Step;
            public float Initial;
            public bool HasMin;
            public bool HasMax;
            public bool HasStep;
            public bool HasInitial;
        }

        private static bool TryParseExposeParam(string exposeBlock, string paramName, out float value)
        {
            value = 0f;
            Regex paramRegex = new Regex(paramName + @"\s*=\s*(-?\d+\.?\d*f?)");
            Match match = paramRegex.Match(exposeBlock);
            if (!match.Success)
            {
                return false;
            }
            string valueStr = match.Groups[1].Value;
            if (valueStr.EndsWith("f"))
            {
                valueStr = valueStr.Substring(0, valueStr.Length - 1);
            }
            try
            {
                value = float.Parse(valueStr);
                return true;
            }
            catch
            {
                return false;
            }
        }

        private static ExposeConfigValues ReadExposeConfigFromCsFile(string className, string fieldName)
        {
            ExposeConfigValues result = new ExposeConfigValues();

            string filePath = FindScriptSourceFile(className);
            if (filePath == null)
            {
                return result;
            }

            string content = File.ReadAllText(filePath);
            string[] lines = content.Split('\n');

            int exposeLineIndex = -1;
            int fieldLineIndex = -1;

            // 查找[Expose]属性行和字段声明行（与ModifyScriptSourceFile相同逻辑）
            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i].Trim();
                if (line.Contains(fieldName) &&
                    (line.Contains("private") || line.Contains("public") || line.Contains("protected")) &&
                    line.Contains("float"))
                {
                    fieldLineIndex = i;
                    if (line.Contains("[Expose"))
                    {
                        exposeLineIndex = i;
                    }
                    else if (i > 0 && lines[i - 1].Trim().Contains("[Expose"))
                    {
                        exposeLineIndex = i - 1;
                    }
                    else
                    {
                        for (int j = i - 1; j >= 0; j--)
                        {
                            string prevLine = lines[j].Trim();
                            if (prevLine.Contains("[Expose"))
                            {
                                exposeLineIndex = j;
                                break;
                            }
                            if (!prevLine.StartsWith("[") && !prevLine.StartsWith(",") && prevLine.Length > 0)
                            {
                                break;
                            }
                        }
                    }
                    break;
                }
            }

            if (exposeLineIndex == -1)
            {
                return result;
            }

            // 构建expose block
            int exposeEnd = exposeLineIndex;
            for (int i = exposeLineIndex; i < lines.Length; i++)
            {
                if (lines[i].Contains(")]"))
                {
                    exposeEnd = i;
                    break;
                }
            }

            string exposeBlock = "";
            for (int i = exposeLineIndex; i <= exposeEnd; i++)
            {
                exposeBlock += lines[i];
            }

            // 解析[Expose]参数值
            result.HasMin = TryParseExposeParam(exposeBlock, "Min", out result.Min);
            result.HasMax = TryParseExposeParam(exposeBlock, "Max", out result.Max);
            result.HasStep = TryParseExposeParam(exposeBlock, "Step", out result.Step);
            result.HasInitial = TryParseExposeParam(exposeBlock, "Initial", out result.Initial);

            // 如果[Expose]没有Initial参数，尝试从字段默认值读取
            if (!result.HasInitial && fieldLineIndex >= 0 && fieldLineIndex < lines.Length)
            {
                string fieldLine = lines[fieldLineIndex].Trim();
                Regex fieldDefaultRegex = new Regex(@"=\s*(-?\d+\.?\d*f?)\s*;");
                Match fieldMatch = fieldDefaultRegex.Match(fieldLine);
                if (fieldMatch.Success)
                {
                    string valueStr = fieldMatch.Groups[1].Value;
                    if (valueStr.EndsWith("f"))
                    {
                        valueStr = valueStr.Substring(0, valueStr.Length - 1);
                    }
                    try
                    {
                        result.Initial = float.Parse(valueStr);
                        result.HasInitial = true;
                    }
                    catch
                    {
                    }
                }
            }

            return result;
        }

        private static string FindScriptSourceFile(string className)
        {
            // 优先: scripts/className.cs 直接匹配
            string directPath = Path.Combine("scripts", className + ".cs");
            if (File.Exists(directPath))
            {
                return directPath;
            }
            
            // 回退: 扫描scripts/目录下所有.cs文件，查找class声明
            if (!Directory.Exists("scripts"))
            {
                return null;
            }
            
            string[] csFiles = Directory.GetFiles("scripts", "*.cs");
            string classPattern = "class " + className;
            
            for (int i = 0; i < csFiles.Length; i++)
            {
                try
                {
                    string content = File.ReadAllText(csFiles[i]);
                    if (content.Contains(classPattern))
                    {
                        return csFiles[i];
                    }
                }
                catch
                {
                    // 读取失败则跳过该文件
                }
            }
            
            return null;
        }
        
        private static string FormatFloatForAttribute(float value)
        {
            if (Math.Abs(value - (float)Math.Round(value)) < 0.001f)
            {
                return ((int)Math.Round(value)).ToString();
            }
            return value.ToString("F1") + "f";
        }
        
        private static string FormatFloatForField(float value)
        {
            return value.ToString("F1") + "f";
        }
        
        private static void ModifyScriptSourceFile(string className, string fieldName, string paramName, float newValue)
        {
            string filePath = FindScriptSourceFile(className);
            if (filePath == null)
            {
                Log.Warn("Editor", "ModifyScriptSourceFile: 未找到脚本源文件 class=" + className);
                return;
            }
            
            string content = File.ReadAllText(filePath);
            string[] lines = content.Split('\n');
            
            int exposeLineIndex = -1;
            int fieldLineIndex = -1;
            
            // 查找[Expose]属性行和字段声明行
            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i].Trim();
                
                if (line.Contains(fieldName) && 
                    (line.Contains("private") || line.Contains("public") || line.Contains("protected")) &&
                    line.Contains("float"))
                {
                    fieldLineIndex = i;
                    
                    if (line.Contains("[Expose"))
                    {
                        exposeLineIndex = i;
                    }
                    else if (i > 0 && lines[i - 1].Trim().Contains("[Expose"))
                    {
                        exposeLineIndex = i - 1;
                    }
                    else
                    {
                        for (int j = i - 1; j >= 0; j--)
                        {
                            string prevLine = lines[j].Trim();
                            if (prevLine.Contains("[Expose"))
                            {
                                exposeLineIndex = j;
                                break;
                            }
                            if (!prevLine.StartsWith("[") && !prevLine.StartsWith(",") && prevLine.Length > 0)
                            {
                                break;
                            }
                        }
                    }
                    
                    break;
                }
            }
            
            if (exposeLineIndex == -1 || fieldLineIndex == -1)
            {
                Log.Warn("Editor", "ModifyScriptSourceFile: 未找到[Expose]属性或字段声明 " + className + "." + fieldName);
                return;
            }
            
            // 修改[Expose]属性中的参数
            string formattedAttrValue = FormatFloatForAttribute(newValue);
            string paramPattern = paramName + @"\s*=\s*-?\d+\.?\d*f?";
            Regex paramRegex = new Regex(paramPattern);
            
            int exposeStart = exposeLineIndex;
            int exposeEnd = exposeLineIndex;
            
            for (int i = exposeStart; i < lines.Length; i++)
            {
                if (lines[i].Contains(")]"))
                {
                    exposeEnd = i;
                    break;
                }
            }
            
            string exposeBlock = "";
            for (int i = exposeStart; i <= exposeEnd; i++)
            {
                exposeBlock += lines[i];
            }
            
            if (paramRegex.IsMatch(exposeBlock))
            {
                exposeBlock = paramRegex.Replace(exposeBlock, paramName + " = " + formattedAttrValue);
            }
            else
            {
                exposeBlock = exposeBlock.Replace(")]", ", " + paramName + " = " + formattedAttrValue + ")]");
            }
            
            string[] newExposeLines = exposeBlock.Split('\n');
            
            string[] newLines = new string[lines.Length - (exposeEnd - exposeStart + 1) + newExposeLines.Length];
            int destIdx = 0;
            
            for (int i = 0; i < exposeStart; i++)
            {
                newLines[destIdx++] = lines[i];
            }
            
            for (int i = 0; i < newExposeLines.Length; i++)
            {
                newLines[destIdx++] = newExposeLines[i];
            }
            
            for (int i = exposeEnd + 1; i < lines.Length; i++)
            {
                newLines[destIdx++] = lines[i];
            }
            
            lines = newLines;
            
            // 重新查找fieldLineIndex
            fieldLineIndex = -1;
            for (int i = 0; i < lines.Length; i++)
            {
                string line = lines[i].Trim();
                if (line.Contains(fieldName) &&
                    (line.Contains("private") || line.Contains("public") || line.Contains("protected")) &&
                    line.Contains("float"))
                {
                    fieldLineIndex = i;
                    break;
                }
            }
            
            // 对于Initial参数，还需要修改字段默认值
            if (paramName == "Initial" && fieldLineIndex != -1)
            {
                string formattedFieldValue = FormatFloatForField(newValue);
                string fieldLine = lines[fieldLineIndex];
                
                Regex fieldDefaultRegex = new Regex(@"=\s*-?\d+\.?\d*f?\s*;");
                
                if (fieldDefaultRegex.IsMatch(fieldLine))
                {
                    fieldLine = fieldDefaultRegex.Replace(fieldLine, "= " + formattedFieldValue + ";");
                }
                else
                {
                    fieldLine = fieldLine.Replace(";", "= " + formattedFieldValue + ";");
                }
                
                lines[fieldLineIndex] = fieldLine;
            }
            
            // 写回文件
            string modifiedContent = string.Join("\n", lines);
            File.WriteAllText(filePath, modifiedContent);
            
            Log.Info("Editor", "ModifyScriptSourceFile: 已修改 " + filePath + " " + className + "." + fieldName + " " + paramName + "=" + formattedAttrValue);
            _sourceFilesModified = true;
        }
    }
}