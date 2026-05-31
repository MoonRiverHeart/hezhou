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

        // === Input Event Handlers ===

        private static void OnMouseMove(float x, float y, bool dragging)
        {
            if (_gameScene == null) return;
            
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
            const uint KEY_DELETE = 49;
            const uint KEY_F5 = 57;
            const uint KEY_F6 = 58;
            const uint KEY_F7 = 59;

            bool ctrl = (modifiers & 2) != 0;
            bool shift = (modifiers & 1) != 0;

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
                    UI.SetWidgetBackgroundColor(_previewWindowId, 0.08f, 0.08f, 0.08f, 0.3f);
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
            _scriptToggleBtnIndices.Clear();
            
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
                
                string scriptName = Path.GetFileName(info.ScriptPath);
                string labelText = scriptName + " (" + info.ClassName + ")";
                
                ulong scriptRow = UI.CreateHStack(_scriptsListContainerId, 5f);
                UI.CreateLabel(scriptRow, RIGHT_PANEL_WIDTH - 130f, 20f, labelText);
                
                // Toggle button: [ON] or [OFF] with colored background
                ulong toggleBtnId = UI.CreateButton(scriptRow, 40f, 20f, info.Enabled ? "[ON]" : "[OFF]");
                UI.SetOnClick(toggleBtnId, _scriptToggleClickCallback);
                _scriptToggleBtnIndices[toggleBtnId] = i;
                
                if (info.Enabled)
                {
                    UI.SetWidgetBackgroundColor(toggleBtnId, 0.2f, 0.8f, 0.2f, 1.0f);
                }
                else
                {
                    UI.SetWidgetBackgroundColor(toggleBtnId, 0.8f, 0.2f, 0.2f, 1.0f);
                }
                
                ulong removeBtnId = UI.CreateButton(scriptRow, 40f, 20f, "X");
                UI.SetOnClick(removeBtnId, _removeScriptClickCallback);
                _removeScriptBtnIndices[removeBtnId] = i;
                _scriptRowIds.Add(scriptRow);
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

            if (_gameScene == null || _scriptsListContainerId == 0) return;

            int scriptCount = _gameScene.GetScriptBindingCount(entityId);

            for (int i = 0; i < scriptCount; i++)
            {
                ScriptBindingInfo bindingInfo = _gameScene.GetScriptBindingInfo(entityId, i);

                // 查找脚本类型 — 未注册的类型跳过（无法反射属性）
                Type scriptType = null;
                if (!_scriptTypeRegistry.TryGetValue(bindingInfo.ClassName, out scriptType))
                {
                    continue;
                }

                // 获取instance_id（0表示尚未创建C#实例）
                ulong instanceId = UI.SceneGetScriptBindingInstanceId(_gameScene.ScenePtr, entityId, (ulong)i);

                // 反射[Expose]属性 → ScriptPropertyDescriptor[]
                ScriptPropertyDescriptor[] descriptors = HezhouScripts.ScriptEntityHelper.ReflectProperties(scriptType);

                for (int d = 0; d < descriptors.Length; d++)
                {
                    // 记录创建前的子控件数量，用于追踪新创建的属性控件
                    uint childCountBefore = UI.WidgetGetChildCount(_scriptsListContainerId);

                    // 创建属性控件（Label + Slider/Input + Min/Max/Step/Initial HStack）
                    // BuildSingleScriptProperty内部填充_scriptPropertyInfoMap和_scriptMinMaxInitialToMainWidgetMap
                    ScriptPropertyInfo propInfo = BuildSingleScriptProperty(
                        _scriptsListContainerId, descriptors[d], entityId, i, instanceId, scriptType);

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
                ulong nodeId = UI.TreeViewAddNode(_projectTreeViewId, _entitiesNodeId, "🔷 " + name, entityId, false);
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
            // Clear previous node selection highlight
            if (_lastSelectedNodeId != 0 && _lastSelectedNodeId != widgetId)
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
                SelectEntity(userData);
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
                    // Delete entity from scene
                    if (_gameScene != null)
                    {
                        _gameScene.RemoveEntity(_contextMenuTargetEntityId);
                    }
                    // Remove from project tree
                    UI.TreeViewRemoveNode(_projectTreeViewId, _contextMenuTargetNodeId);
                    _entityNodeMap.Remove(_contextMenuTargetEntityId);
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
            UI.PopupMenuHide(popupMenuId);
        }

        private static void RefreshAssetGridView()
        {
            // Use _assetModelGridViewId (the actual GridView inside the 模型 tab)
            // _assetGridViewId is a legacy alias that is never set
            ulong gridViewId = _assetModelGridViewId != 0 ? _assetModelGridViewId : _assetGridViewId;
            if (gridViewId == 0) return;
            
            UI.GridViewClear(gridViewId);
            
            UI.GridViewAddItem(gridViewId, "Cube", 1);
            UI.GridViewAddItem(gridViewId, "Sphere", 2);
            UI.GridViewAddItem(gridViewId, "Plane", 3);
            UI.GridViewAddItem(gridViewId, "Cylinder", 4);
            UI.GridViewAddItem(gridViewId, "CornellBox", 8);
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
                UI.SetWidgetBackgroundColor(_previewWindowId, 0.2f, 0.4f, 0.8f, 0.3f);
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
                UI.SetWidgetBackgroundColor(_previewWindowId, 0.08f, 0.08f, 0.08f, 0.3f);
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
            
            // Bug4 fix: 如果.cs源文件已被ModifyScriptSourceFile修改过，先触发hot reload再绑定
            if (_sourceFilesModified)
            {
                _pendingBindEntityId = _selectedEntityId;
                _pendingBindScriptIndex = _selectedScriptIndex;
                
                // 保存脚本编辑器内容到.cs文件（如果有内容）
                if (_scriptTextEditId != 0)
                {
                    try
                    {
                        string scriptContent = UI.TextEditGetText(_scriptTextEditId);
                        string saveScriptPath = "scripts/bin/Mono/EditorScript.cs";
                        System.IO.Directory.CreateDirectory("scripts/bin/Mono");
                        System.IO.File.WriteAllText(saveScriptPath, scriptContent);
                    }
                    catch (Exception ex)
                    {
                        Log.Error("Editor", "保存脚本编辑器内容失败: " + ex.Message);
                    }
                }
                
                if (_statusItem != null)
                {
                    _statusItem.Text = "正在热更新脚本，稍后自动绑定...";
                }
                
                SaveAllScriptPropertyValues();
                UI.TriggerHotReload();
                return; // 不立即创建实例 — 等待OnHotReloadComplete回调
            }
            
            string scriptName = _availableScripts[_selectedScriptIndex];
            string scriptPath = "scripts/" + scriptName;
            string className = Path.GetFileNameWithoutExtension(scriptName);
            
            // 1) Rust侧创建ScriptBinding(instance_id=0)
            _gameScene.AttachScriptBinding(_selectedEntityId, scriptPath, className);
            
            // 2) 如果脚本类型在注册表中，创建C#实例并更新instance_id
            if (_scriptTypeRegistry.TryGetValue(className, out Type scriptType))
            {
                // 设置Scene指针 — 脚本实例需要scenePtr来调用FFI旋转等方法
                MethodInfo setScenePtrMethod = scriptType.GetMethod("SetScenePtr",
                    BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                if (setScenePtrMethod != null)
                {
                    setScenePtrMethod.Invoke(null, new object[] { _gameScene.ScenePtr });
                }
                
                // 创建C#实例 — 返回IntPtr(instanceKey)
                MethodInfo createInstanceMethod = scriptType.GetMethod("CreateInstance",
                    BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                if (createInstanceMethod != null)
                {
                    IntPtr instancePtr = (IntPtr)createInstanceMethod.Invoke(null, null);
                    ulong instanceId = (ulong)instancePtr.ToInt64();
                    
                    // 获取刚创建的ScriptBinding的index（最后一个）
                    int bindingIndex = _gameScene.GetScriptBindingCount(_selectedEntityId) - 1;
                    
                    // 更新Rust侧ScriptBinding的instance_id
                    UI.SceneSetScriptBindingInstanceId(_gameScene.ScenePtr, _selectedEntityId, (ulong)bindingIndex, instanceId);
                    
                    Log.Info("Editor", "脚本实例创建: className=" + className + ", instanceId=" + instanceId + ", bindingIndex=" + bindingIndex);
                }
            }
            
            // 强制刷新属性面板 — 必须设_propertiesDirty=true才能绕过UpdatePropertiesPanel自身的缓存检查
            _propertiesDirty = true;
            _lastScriptBindingCount = -1;
            UpdatePropertiesPanel(_selectedEntityId);
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

                        Log.Info("Editor", "热重载后重建实例: className=" + info.ClassName + ", entityId=" + eid + ", bindingIndex=" + b + ", instanceId=" + newInstanceId);
                    }
                }
            }
        }

        private static void RestoreAllScriptPropertyValues()
        {
            if (_gameScene == null) return;

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

                        // clamp越界: runtime值不能超出新descriptor的Min/Max范围
                        float clampedValue = savedValue.RuntimeValue;
                        if (clampedValue > descriptor.Max) clampedValue = descriptor.Max;
                        if (clampedValue < descriptor.Min) clampedValue = descriptor.Min;

                        // 恢复runtime值到新实例
                        HezhouScripts.ScriptEntityHelper.SetFieldValue(instancePtr, savedValue.PropertyName, clampedValue, scriptType);
                        restoredCount++;

                        // 通过MainWidgetId直接查找propInfo
                        if (!_scriptPropertyInfoMap.TryGetValue(savedValue.MainWidgetId, out ScriptPropertyInfo propInfo)) continue;

                        // 更新Slider/Input显示值
                        UI.SliderSetValue(propInfo.MainWidgetId, clampedValue);

                        // 更新Slider范围和步长（从新descriptor读取）
                        UI.SliderSetRange(propInfo.MainWidgetId, descriptor.Min, descriptor.Max);
                        UI.SliderSetStep(propInfo.MainWidgetId, descriptor.Step);

                        // 更新Min/Max/Step/Initial输入框（从新descriptor读取）
                        UI.InputFieldSetText(propInfo.MinInputId, descriptor.Min.ToString());
                        UI.InputFieldSetText(propInfo.MaxInputId, descriptor.Max.ToString());
                        UI.InputFieldSetText(propInfo.StepInputId, descriptor.Step.ToString());
                        UI.InputFieldSetText(propInfo.InitialInputId, descriptor.Initial.ToString());

                        // 更新propInfo的config值和InstanceId
                        propInfo.CurrentMin = descriptor.Min;
                        propInfo.CurrentMax = descriptor.Max;
                        propInfo.CurrentStep = descriptor.Step;
                        propInfo.CurrentInitial = descriptor.Initial;
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
            
            // 非pending场景也要刷新属性面板
            _propertiesDirty = true;
            _lastScriptBindingCount = -1;
            
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
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, "确定要删除选中的Entity吗?");
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
                ulong entityId = _selectedEntityId;
                _gameScene.RemoveEntity(entityId);
                RemoveEntityFromTree(entityId);
                _gameScene.ClearSelection();
                ClearPropertiesPanel();
                _selectedEntityId = 0;
                _statusItem.Text = "已删除 Entity";
                Log.Info("Editor", $"删除Entity: id={entityId}");
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
                UI.SetWidgetBackgroundColor(_previewWindowId, 0.2f, 0.8f, 0.2f, 0.3f);
                _statusItem.Text = "状态: 已暂停";
                Log.Info("Editor", "Pause: Running → Paused");
            }
            else if (currentState == GameState.Paused)
            {
                // Paused → Running
                _gameScene.SetGameState(GameState.Running);
                UI.SetRendererGameState(1);
                UI.SetText(_pauseButtonId, "暂停");
                UI.SetWidgetBackgroundColor(_previewWindowId, 0.2f, 0.4f, 0.8f, 0.3f);
                _statusItem.Text = "状态: 运行中";
                Log.Info("Editor", "Pause: Paused → Running");
            }
        }

        // === Menu Action Handlers ===

        private static void OnFileMenuClick(ulong widgetId, int actionId)
        {
            UI.PopupMenuHide(_fileMenuId);
            if (actionId == 1)
            {
                // 新建场景: clear scene, create default Cube+Plane+Light
                if (_gameScene != null)
                {
                    // Remove all entities from tree
                    foreach (var entityId in _entityNodeMap.Keys)
                    {
                        RemoveEntityFromTree(entityId);
                    }
                    _entityNodeMap.Clear();
                    
                    // Remove all entities from scene
                    int count = _gameScene.GetEntityCount();
                    for (int i = count - 1; i >= 0; i--)
                    {
                        ulong eid = _gameScene.GetEntityId(i);
                        _gameScene.RemoveEntity(eid);
                    }
                    
                    // Create default entities
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
                    _statusItem.Text = "新建场景完成";
                    Log.Info("Editor", "新建场景");
                }
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
                // 保存场景: Project.Save current project
                if (Project.IsLoaded())
                {
                    Project.SyncFromScene(_gameScene.ScenePtr);
                    bool success = Project.Save();
                    _statusItem.Text = success ? "保存成功" : "保存失败";
                    Log.Info("Editor", $"保存场景: {success}");
                }
                else
                {
                    _statusItem.Text = "未加载项目，无法保存";
                    Log.Info("Editor", "保存失败: 未加载项目");
                }
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
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, "选择.project.json文件:");
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
                string selectedPath = UI.FileBrowserGetSelectedPath(_openSceneFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath) && selectedPath.EndsWith(".project.json"))
                {
                    bool success = Project.Load(selectedPath);
                    if (success)
                    {
                        Project.SyncToScene(_gameScene.ScenePtr);
                        _statusItem.Text = "场景加载成功";
                        Log.Info("Editor", $"打开场景: {selectedPath}");
                        
                        // Refresh entity tree
                        foreach (var entityId in _entityNodeMap.Keys)
                        {
                            RemoveEntityFromTree(entityId);
                        }
                        _entityNodeMap.Clear();
                        
                        int entityCount = _gameScene.GetEntityCount();
                        for (int i = 0; i < entityCount; i++)
                        {
                            ulong eid = _gameScene.GetEntityId(i);
                            string name = UI.SceneGetEntityName(_gameScene.ScenePtr, eid);
                            AddEntityToTree(eid, name);
                        }
                    }
                    else
                    {
                        _statusItem.Text = "场景加载失败";
                        Log.Error("Editor", $"打开场景失败: {selectedPath}");
                    }
                }
                else
                {
                    _statusItem.Text = "请选择.project.json文件";
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
                string selectedPath = UI.FileBrowserGetSelectedPath(_openProjectFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath))
                {
                    // Find .project.json in the selected directory
                    string projectFile = System.IO.Path.Combine(selectedPath, "project.project.json");
                    if (!System.IO.File.Exists(projectFile))
                    {
                        // Try finding any .project.json in the directory
                        string[] files = System.IO.Directory.GetFiles(selectedPath, "*.project.json");
                        if (files.Length > 0)
                        {
                            projectFile = files[0];
                        }
                    }
                    
                    bool success = Project.Load(projectFile);
                    if (success)
                    {
                        Project.SyncToScene(_gameScene.ScenePtr);
                        _statusItem.Text = "项目加载成功: " + Project.GetName();
                        Log.Info("Editor", $"打开项目: {projectFile}");
                        
                        // Refresh entity tree
                        foreach (var entityId in _entityNodeMap.Keys)
                        {
                            RemoveEntityFromTree(entityId);
                        }
                        _entityNodeMap.Clear();
                        
                        int entityCount = _gameScene.GetEntityCount();
                        for (int i = 0; i < entityCount; i++)
                        {
                            ulong eid = _gameScene.GetEntityId(i);
                            string name = UI.SceneGetEntityName(_gameScene.ScenePtr, eid);
                            AddEntityToTree(eid, name);
                        }
                        
                        // Update project name in status bar
                        if (_projectItem != null)
                        {
                            _projectItem.Text = "项目: " + Project.GetName();
                        }
                    }
                    else
                    {
                        _statusItem.Text = "项目加载失败";
                        Log.Error("Editor", $"打开项目失败: {projectFile}");
                    }
                }
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
                string selectedPath = UI.FileBrowserGetSelectedPath(_saveAsFileBrowserId);
                if (!string.IsNullOrEmpty(selectedPath))
                {
                    Project.SyncFromScene(_gameScene.ScenePtr);
                    string savePath = System.IO.Path.Combine(selectedPath, "project.project.json");
                    bool success = UI.ProjectCreateNew(Project.GetName(), savePath);
                    if (success)
                    {
                        Project.SyncFromScene(_gameScene.ScenePtr);
                        success = Project.Save();
                    }
                    _statusItem.Text = success ? "另存为成功" : "另存为失败";
                    Log.Info("Editor", $"另存为: {savePath}, success={success}");
                }
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
                
                _gameScene.SetScriptBindingEnabled(_selectedEntityId, scriptIndex, newEnabled);
                
                // Update button text and color
                if (newEnabled)
                {
                    UI.SetText(widgetId, "[ON]");
                    UI.SetWidgetBackgroundColor(widgetId, 0.2f, 0.8f, 0.2f, 1.0f);
                }
                else
                {
                    UI.SetText(widgetId, "[OFF]");
                    UI.SetWidgetBackgroundColor(widgetId, 0.8f, 0.2f, 0.2f, 1.0f);
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
            
            // Initial修改: 不需要额外UI更新（Initial只是.cs文件中的默认值）
            
            // struct值类型 → 修改后必须重新赋值回字典
            _scriptPropertyInfoMap[mainWidgetId] = propInfo;
            
            Log.Info("Editor", "脚本属性配置修改: " + propInfo.ClassName + "." + propInfo.PropertyName + " " + paramName + "=" + value);
        }
        
        // === .cs源文件修改机制 ===
        
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