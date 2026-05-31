using System;
using System.Reflection;
using System.Runtime.InteropServices;
using HezhouScripts;

namespace Hezhou
{
    public static partial class EditorScript
    {
        // =====================================================
        // ENTRY POINT: Initialize, Update, UpdateStatusBar
        // =====================================================

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate();
        
        public static void Initialize(IntPtr contextPtr)
        {
            UI.InitFromContext(contextPtr);
            
            // 热重载时先清空旧的widget树和回调(在所有回调注册之前)
            // 首次初始化时SceneGetExistingPtr返回0，不需要清空
            IntPtr existingScenePtr = UI.SceneGetExistingPtr();
            if (existingScenePtr != IntPtr.Zero)
            {
                UI.ClearWidgetTree();
                Log.Info("Editor", "热重载: 已清空旧widget树和回调");
            }
            
            UI.GetScreenSize(out _screenWidth, out _screenHeight);
            _contentScale = UI.GetContentScale();
            
            Log.Info("Editor", $"编辑器初始化: {_screenWidth}x{_screenHeight}, DPI={_contentScale}");
            
            _updateCallback = Update;
            _resizeCallback = OnResize;
            _globalClickCallback = OnGlobalClick;
            _keyCallback = OnKey;
            _mouseMoveCallback = OnMouseMove;
            _mouseWheelCallback = OnMouseWheel;
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
             
_bindScriptClickCallback = OnBindScriptClick;
            _removeScriptClickCallback = OnRemoveScriptClick;
            _pauseClickCallback = OnPauseClick;
            _scriptToggleClickCallback = OnScriptToggleClick;
            _scriptDropdownSelectCallback = OnScriptDropdownSelect;
            _hotReloadCompleteCallback = OnHotReloadComplete;
            UI.RegisterHotReloadCompleteCallback(_hotReloadCompleteCallback);
            _tabSelectCallback = OnTabSelect;
            _treeNodeToggleCallback = OnTreeNodeToggle;
            
            // SplitView ratio change callbacks (static fields to prevent GC)
            _outerSplitViewRatioCallback = OnOuterSplitViewRatioChange;
            _leftCenterSplitViewRatioCallback = OnLeftCenterSplitViewRatioChange;
            _innerHorizontalSplitViewRatioCallback = OnInnerHorizontalSplitViewRatioChange;
            
            _fileMenuClickCallback = OnFileMenuClick;
            _openMenuClickCallback = OnOpenMenuClick;
            _saveMenuClickCallback = OnSaveMenuClick;
            _treeNodeSelectCallback = OnTreeNodeSelect;
            _treeNodeRightClickCallback = OnTreeNodeRightClick;
            _gridViewClickCallback = OnGridViewClick;
            _fileBrowserSelectCallback = OnFileBrowserSelect;
            _workingDirectoryDialogResultCallback = OnWorkingDirectoryDialogResult;
            _deleteConfirmDialogResultCallback = OnDeleteConfirmDialogResult;
            _openSceneDialogResultCallback = OnOpenSceneDialogResult;
            _openProjectDialogResultCallback = OnOpenProjectDialogResult;
            _saveAsDialogResultCallback = OnSaveAsDialogResult;
            
            UI.RegisterUpdateCallback(_updateCallback);
            
            // 读取配置文件：如果之前已设置工作目录（包括热重载后的恢复），跳过dialog
            try
            {
                if (System.IO.File.Exists(".hezhou_config"))
                {
                    string configContent = System.IO.File.ReadAllText(".hezhou_config");
                    string[] configLines = configContent.Split('\n');
                    
                    foreach (string line in configLines)
                    {
                        string trimmed = line.Trim();
                        if (trimmed.StartsWith("workingDir="))
                        {
                            _currentDirectory = trimmed.Substring("workingDir=".Length);
                            _workingDirectorySet = true;
                            Log.Info("Editor", "从配置文件恢复工作目录: " + _currentDirectory);
                        }
                        else if (!trimmed.StartsWith("scenePtr=") && !trimmed.StartsWith("pid=") && trimmed.Length > 0 && !trimmed.StartsWith("#"))
                        {
                            // 旧格式兼容: 只有一行workingDir字符串（无前缀）
                            _currentDirectory = trimmed;
                            _workingDirectorySet = true;
                            Log.Info("Editor", "从配置文件(旧格式)恢复工作目录: " + _currentDirectory);
                            break;
                        }
                    }
                }
            }
            catch (Exception) { }
            
            if (_workingDirectorySet)
            {
                ScanScripts();
                CreateEditorLayout();
            }
            else
            {
                ShowWorkingDirectoryDialog();
            }
            
            UI.RegisterResizeCallback(_resizeCallback);
            UI.RegisterGlobalClickCallback(_globalClickCallback);
            UI.RegisterKeyCallback(_keyCallback);
            UI.RegisterMouseMoveCallback(_mouseMoveCallback);
            UI.RegisterMouseWheelCallback(_mouseWheelCallback);
            UI.RegisterTreeNodeRightClickCallback(_treeNodeRightClickCallback);
            
            // === 脚本类型注册：className → Type ===
            // 每个脚本实体类需在此注册，以便编辑器UI通过反射发现属性并创建实例
            _scriptTypeRegistry["RotatingEntity"] = typeof(RotatingEntity);
            
            // 保存配置文件（只保存workingDir，Scene指针通过Rust FFI获取）
            if (_workingDirectorySet)
            {
                try
                {
                    System.IO.File.WriteAllText(".hezhou_config", "workingDir=" + _currentDirectory);
                }
                catch (Exception) { }
            }

            // 运行反射管线测试（TDD RED阶段验证）
            TestRunner.RunAllTests();
        }
        
        public static void Update(float deltaTime)
        {
            // If UITestRunner is active, delegate to it and skip normal editor update
            if (UITestRunner.IsRunning)
            {
                UITestRunner.Update(deltaTime);
                return;
            }

            if (deltaTime > 0 && _fpsItem != null)
            {
                try
                {
                    _fpsItem.Text = "FPS: " + ((int)(1000f / deltaTime));
                    
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
                    
                    // === 场景更新: 基于GameState，不依赖_previewSelected ===
                    if (_gameScene != null && _gameScene.GetGameState() == GameState.Running)
                    {
                        // 相机控制: 仅在previewSelected时响应方向键
                        if (_previewSelected)
                        {
                            float speed = 2f * (deltaTime / 1000f);
                            
                            float sinYaw = (float)Math.Sin(_cameraYaw);
                            float cosYaw = (float)Math.Cos(_cameraYaw);
                            float sinPitch = (float)Math.Sin(_cameraPitch);
                            float cosPitch = (float)Math.Cos(_cameraPitch);
                            float forwardX = sinYaw * cosPitch;
                            float forwardY = sinPitch;
                            float forwardZ = -cosYaw * cosPitch;
                            float rightX = cosYaw;
                            float rightZ = sinYaw;
                            
                            if (_keyUpPressed)
                            {
                                _cameraX += speed * forwardX;
                                _cameraY += speed * forwardY;
                                _cameraZ += speed * forwardZ;
                            }
                            if (_keyDownPressed)
                            {
                                _cameraX -= speed * forwardX;
                                _cameraY -= speed * forwardY;
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
                        }
                        
                        UI.SetCameraParams(_cameraYaw, _cameraPitch, _cameraX, _cameraY, _cameraZ);
                        
                        // 场景更新和脚本更新: 无论previewSelected，只要GameState=Running就执行
                        _gameScene.Update(deltaTime / 1000f);
                        
                        // === 脚本实例每帧更新: 遍历所有Entity的ScriptBinding，反射调用UpdateInstance ===
                        int entityCount = _gameScene.GetEntityCount();
                        for (int e = 0; e < entityCount; e++)
                        {
                            ulong eid = _gameScene.GetEntityId(e);
                            int bindingCount = _gameScene.GetScriptBindingCount(eid);
                            for (int b = 0; b < bindingCount; b++)
                            {
                                var bindingInfo = _gameScene.GetScriptBindingInfo(eid, b);
                                if (!bindingInfo.Enabled) continue;
                                
                                ulong instId = UI.SceneGetScriptBindingInstanceId(_gameScene.ScenePtr, eid, (ulong)b);
                                if (instId == 0) continue;
                                
                                if (_scriptTypeRegistry.TryGetValue(bindingInfo.ClassName, out Type scriptType))
                                {
                                    try
                                    {
                                        IntPtr instancePtr = new IntPtr((long)instId);
                                        MethodInfo updateMethod = scriptType.GetMethod("UpdateInstance",
                                            BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
                                        if (updateMethod != null)
                                        {
                                            updateMethod.Invoke(null, new object[] { instancePtr, deltaTime / 1000f, eid });
                                        }
                                    }
                                    catch (Exception ex)
                                    {
                                        Log.Error("Editor", "脚本UpdateInstance失败: " + bindingInfo.ClassName + ", entityId=" + eid + ", 错误=" + ex.Message);
                                    }
                                }
                            }
                        }
                    }
                    else if (_gameScene != null && _gameScene.GetGameState() == GameState.Paused)
                    {
                        // Paused: keep camera params but don't update scene
                        UI.SetCameraParams(_cameraYaw, _cameraPitch, _cameraX, _cameraY, _cameraZ);
                    }
                    else
                    {
                        // Editing mode: orbit camera
                        // 方向键移动orbit target
                        float orbitSpeed = 2f * (deltaTime / 1000f);
                        bool anyKey = _keyLeftPressed || _keyRightPressed || _keyUpPressed || _keyDownPressed;
                        if (anyKey)
                        {
                            if (_altPressed)
                            {
                                // Alt+方向键: 朝相机方向移动orbit target
                                float sinYaw = (float)Math.Sin(_orbitYaw);
                                float cosYaw = (float)Math.Cos(_orbitYaw);
                                float forwardX = sinYaw;
                                float forwardZ = -cosYaw;
                                float rightX = cosYaw;
                                float rightZ = sinYaw;
                                if (_keyUpPressed)
                                {
                                    _orbitTargetX += orbitSpeed * forwardX;
                                    _orbitTargetZ += orbitSpeed * forwardZ;
                                }
                                if (_keyDownPressed)
                                {
                                    _orbitTargetX -= orbitSpeed * forwardX;
                                    _orbitTargetZ -= orbitSpeed * forwardZ;
                                }
                                if (_keyLeftPressed)
                                {
                                    _orbitTargetX -= orbitSpeed * rightX;
                                    _orbitTargetZ -= orbitSpeed * rightZ;
                                }
                                if (_keyRightPressed)
                                {
                                    _orbitTargetX += orbitSpeed * rightX;
                                    _orbitTargetZ += orbitSpeed * rightZ;
                                }
                            }
                            else
                            {
                                // 无Alt: 朝世界坐标移动orbit target
                                if (_keyUpPressed)    _orbitTargetZ -= orbitSpeed;
                                if (_keyDownPressed)  _orbitTargetZ += orbitSpeed;
                                if (_keyLeftPressed)  _orbitTargetX -= orbitSpeed;
                                if (_keyRightPressed) _orbitTargetX += orbitSpeed;
                            }
                        }

                        // 球坐标计算camera位置（围绕target旋转）
                        // 球坐标计算camera位置（围绕target旋转）
                        float camX = _orbitTargetX + _orbitDistance * (float)Math.Sin(_orbitYaw) * (float)Math.Cos(_orbitPitch);
                        float camY = _orbitTargetY + _orbitDistance * (float)Math.Sin(_orbitPitch);
                        float camZ = _orbitTargetZ + _orbitDistance * (float)Math.Cos(_orbitYaw) * (float)Math.Cos(_orbitPitch);
                        // 从camera位置看向target点，计算FPS相机yaw/pitch
                        // shader的viewMatrix使用FPS风格：yaw=atan2(dx,dz), pitch=atan2(dy,sqrt(dx²+dz²))
                        float dx = _orbitTargetX - camX;
                        float dy = _orbitTargetY - camY;
                        float dz = _orbitTargetZ - camZ;
                        float fpsYaw = (float)Math.Atan2(dx, -dz);  // shader convention: forward = (sin(yaw)*cos(pitch), -sin(pitch), -cos(yaw)*cos(pitch))
                        float fpsPitch = (float)Math.Atan2(-dy, (float)Math.Sqrt(dx * dx + dz * dz));  // shader: pitch正=向上看
                        UI.SetCameraParams(fpsYaw, fpsPitch, camX, camY, camZ);
                    }
                    
                    // === 脚本属性runtime值→UI同步 (每5帧) ===
                    SyncScriptPropertyValuesToUI();
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