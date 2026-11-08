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
            LoadSampleTextureOnce();
            
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

            // 主题必须在 CreateEditorLayout 之前恢复，控件构造时按当前主题取色。
            UI.ThemeSetMode(LoadThemePreference());
            
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
            _consoleClickCallback = OnConsoleClick;
            _themeClickCallback = OnThemeClick;
            _qualityClickCallback = OnQualityClick;
            _audioTestCallback = OnAudioTestClick;
            _assetScanCallback = OnAssetScanClick;
            _assetImportCallback = OnAssetImportClick;
            _assetImportDialogResultCallback = OnAssetImportDialogResult;
            _sceneSaveCallback = OnSceneSaveClick;
            _sceneSwitchCallback = OnSceneSwitchClick;
            _sceneUnloadCallback = OnSceneUnloadClick;
            _sceneLoadCallback = OnSceneLoadClick;
            _prefabCaptureCallback = OnPrefabCaptureClick;
            _prefabInstantiateCallback = OnPrefabInstantiateClick;
            _playerExportCallback = OnPlayerExportClick;
            _playerBuildCallback = OnPlayerBuildClick;
            _pbrFieldChangeCallback = OnPbrFieldChanged;
            _pbrClearCallback = OnPbrClearClick;
            _profilerPauseCallback = OnProfilerPauseClick;
            _profilerResetCallback = OnProfilerResetClick;
            _buildSettingsLoadCallback = OnBuildSettingsLoadClick;
            _buildSettingsSaveCallback = OnBuildSettingsSaveClick;
            _buildSettingsValidateCallback = OnBuildSettingsValidateClick;
            _animLoadCallback = OnAnimLoadClick;
            _animBrowseCallback = OnAnimBrowseClick;
            _animPlayCallback = OnAnimPlayClick;
            _animPauseCallback = OnAnimPauseClick;
            _animResetCallback = OnAnimResetClick;
            _animClipSelectCallback = OnAnimClipSelected;
            _animSliderChangeCallback = OnAnimSliderChanged;
            _animFileDialogResultCallback = OnAnimationFileDialogResult;
            _sceneFileDialogResultCallback = OnSceneFileDialogResult;
            _rightMaxClickCallback = OnRightMaxClick;
            _consoleDialogResultCallback = OnConsoleDialogResult;
            _consoleRefreshCallback = OnConsoleRefresh;
            _consoleJumpCallback = OnConsoleJumpToSource;
            _consoleLevelSelectCallback = OnConsoleLevelSelect;
            _consoleSearchChangeCallback = OnConsoleSearchChange;
            _undoClickCallback = OnUndoClick;
            _redoClickCallback = OnRedoClick;
            _projectSearchChangeCallback = OnProjectSearchChange;
            _assetSearchChangeCallback = OnAssetSearchChange;
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
            _materialClickCallback = OnMaterialButtonClick;
            _tabDragEndCallback = OnTabDragEnd;
            _textureGridClickCallback = OnTextureGridClick;
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

            // 自动化测试模式：跳过工作目录对话框，直接使用默认 scripts 目录。
            string autoTestEnv = Environment.GetEnvironmentVariable("HEZHOU_AUTOTEST");
            bool autoTestMode = autoTestEnv == "1"
                || string.Equals(autoTestEnv, "true", StringComparison.OrdinalIgnoreCase);
            if (autoTestMode)
            {
                _currentDirectory = "scripts";
                _workingDirectorySet = true;
            }
            
            LoadDockLayout();

            if (_workingDirectorySet)
            {
                ScanScripts();
                CreateEditorLayout();
                ApplyDockLayout();
            }
            else
            {
                ShowWorkingDirectoryDialog();
            }

            if (autoTestMode)
            {
                UITestRunner.RequestAutoRunFromEnvironment();
            }
            
            UI.RegisterResizeCallback(_resizeCallback);
            UI.RegisterGlobalClickCallback(_globalClickCallback);
            UI.RegisterTabDragEndCallback(_tabDragEndCallback);
            UI.RegisterKeyCallback(_keyCallback);
            UI.RegisterMouseMoveCallback(_mouseMoveCallback);
            UI.RegisterMouseWheelCallback(_mouseWheelCallback);
            UI.RegisterTreeNodeRightClickCallback(_treeNodeRightClickCallback);
            
            // === 脚本类型注册：className → Type ===
            // 每个脚本实体类需在此注册，以便编辑器UI通过反射发现属性并创建实例
            // ★ 关键：使用 Assembly.GetExecutingAssembly().GetType() 而非 typeof()
            // 原因：Mono热重载时 typeof() 可能解析到旧assembly的类型（旧[Expose]属性值=初始值）
            //       GetExecutingAssembly() 确保获取当前新assembly的类型，descriptor值与.cs一致
            System.Reflection.Assembly currentAssembly = System.Reflection.Assembly.GetExecutingAssembly();
            Type rotType = currentAssembly.GetType("HezhouScripts.RotatingEntity");
            if (rotType != null)
            {
                HezhouScripts.ScriptEntityHelper.ReflectProperties(rotType);
            }
            _scriptTypeRegistry["RotatingEntity"] = rotType;

            Type rotationScriptType = currentAssembly.GetType("HezhouScripts.RotationScript");
            if (rotationScriptType != null)
            {
                HezhouScripts.ScriptEntityHelper.ReflectProperties(rotationScriptType);
                _scriptTypeRegistry["RotationScript"] = rotationScriptType;
            }
            
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

            RefreshConsoleIfOpen();

            // 0.3 Profiler：每 30 帧刷新面板（暂停时跳过）。
            _profilerTickCounter++;
            if (_profilerTickCounter >= 30)
            {
                _profilerTickCounter = 0;
                if (!_profilerPaused) RefreshProfilerPanel();
            }

            // 0.3 动画时间轴：播放时间由编辑器每帧驱动（暂停时保持当前时间）。
            if (_animLoaded)
            {
                float animTime = (float)UI.AnimationPreviewTick(deltaTime, _animPlaying ? 1 : 0);
                if (_animPlaying)
                {
                    if (_animSliderId != 0 && _animDuration > 0f)
                    {
                        UI.SliderSetValue(_animSliderId, animTime);
                    }
                    UpdateAnimTimeLabel(animTime);
                }
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
                    
                    UpdateGizmoLayout();

                    // === 状态栏实时信息 ===
                    if (_selectionItem != null) _selectionItem.Text = "选择: " + _selectedEntityIds.Count;
                    if (_themeItem != null) _themeItem.Text = Theme.IsDark ? "主题: 深色" : "主题: 浅色";
                    if (_errorItem != null) _errorItem.Text = "错误: " + TestTelemetry.ErrorCount;
                    // 0.2 渲染质量/PBR：状态栏显示实际 draw path 摘要（Legacy/PBR/pending）
                    if (_renderItem != null)
                    {
                        _renderItem.Text = "渲染: " + ShortenRenderSummary(UI.RendererGetPbrSummary());
                    }
                    if (_qualityButtonId != 0)
                    {
                        UI.SetText(_qualityButtonId, "画质: " + UI.RendererQualityName(_qualityPreset));
                    }
                    // 0.2 资产数据库 / 音频（task-16）：状态栏摘要 + 监听者跟随相机
                    UI.AudioSetListener(_cameraX, _cameraY, _cameraZ);
                    if (_assetItem != null)
                    {
                        _assetItem.Text = "资产: " + ShortenFfiSummary(UI.AssetGetSummary(), 18);
                    }
                    if (_audioItem != null)
                    {
                        _audioItem.Text = "音频: " + AudioShortLabel(UI.AudioGetSummary());
                    }
                    if (_assetDbSummaryLabelId != 0)
                    {
                        UI.SetText(_assetDbSummaryLabelId,
                            "资产: " + ShortenFfiSummary(UI.AssetGetSummary(), 90));
                    }
                    // 0.2 场景槽 / Player 导出状态（task-21）
                    if (_sceneInfoLabelId != 0)
                    {
                        string slotName = UI.SceneManagerCurrentName();
                        if (string.IsNullOrEmpty(slotName)) slotName = "(未保存)";
                        UI.SetText(_sceneInfoLabelId,
                            "场景: " + slotName + "  槽=" + UI.SceneManagerCount());
                    }
                    if (_exportInfoLabelId != 0 && _lastPlayerExportSummary.Length > 0)
                    {
                        UI.SetText(_exportInfoLabelId,
                            "Player 导出: " + ShortenFfiSummary(
                                _lastPlayerExportSummary.Replace('\n', ' '), 90));
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

        // === Theme ===

        private static ThemeMode LoadThemePreference()
        {
            try
            {
                if (System.IO.File.Exists(".hezhou_theme"))
                {
                    string value = System.IO.File.ReadAllText(".hezhou_theme").Trim();
                    if (string.Equals(value, "light", StringComparison.OrdinalIgnoreCase))
                    {
                        return ThemeMode.Light;
                    }
                }
            }
            catch (Exception) { }
            return ThemeMode.Dark;
        }

        private static void SaveThemePreference()
        {
            try
            {
                System.IO.File.WriteAllText(".hezhou_theme", Theme.IsDark ? "dark" : "light");
            }
            catch (Exception) { }
        }

        private static void OnThemeClick(ulong widgetId)
        {
            ApplyTheme(Theme.IsDark ? ThemeMode.Light : ThemeMode.Dark, true);
        }

        // === 0.2 渲染质量 / PBR（FFI: UI.RendererSetQualityPreset / RendererGetPbrSummary） ===

        /// <summary>工具栏画质按钮：Legacy(3) → Low(0) → Medium(1) → High(2) → Legacy 循环。</summary>
        private static void OnQualityClick(ulong widgetId)
        {
            int next = (_qualityPreset + 1) % 4;
            ApplyQualityPreset(next);
        }

        /// <summary>
        /// 应用画质档：0=Low 1=Medium 2=High 3=Legacy。
        /// 失败只记录日志/状态栏并保持当前档位，不抛异常、不崩溃。
        /// </summary>
        public static void ApplyQualityPreset(int preset)
        {
            int rc = UI.RendererSetQualityPreset(preset);
            if (rc == 0)
            {
                _qualityPreset = preset;
                if (_qualityButtonId != 0)
                {
                    UI.SetText(_qualityButtonId, "画质: " + UI.RendererQualityName(preset));
                }
                string summary = UI.RendererGetPbrSummary();
                if (_statusItem != null)
                {
                    _statusItem.Text = "画质: " + UI.RendererQualityName(preset) + " | " + ShortenRenderSummary(summary);
                }
                Log.Info("Editor", "画质切换: " + UI.RendererQualityName(preset) + " | " + summary);
            }
            else
            {
                Log.Warn("Editor", "画质切换失败: preset=" + preset + ", rc=" + rc
                    + "（0=成功,-1=renderer未初始化,-2=非法档位）");
                if (_statusItem != null)
                {
                    _statusItem.Text = "画质切换失败: rc=" + rc;
                }
            }
        }

        /// <summary>状态栏摘要截断，避免超出工具栏/状态栏宽度。</summary>
        private static string ShortenRenderSummary(string summary)
        {
            if (string.IsNullOrEmpty(summary)) return "Legacy";
            const int max = 44;
            return summary.Length <= max ? summary : summary.Substring(0, max - 3) + "...";
        }

        // === 0.2 音频 / 资产数据库（FFI: UI.Audio* / UI.Asset*） ===

        /// <summary>音频按钮：未播放时播放工程内第一个音频资产，播放中则停止。</summary>
        private static void OnAudioTestClick(ulong widgetId)
        {
            string before = UI.AudioGetSummary();
            bool playing = before.IndexOf("audio=on") >= 0 && before.IndexOf("voices=0") < 0;
            int rc = playing ? UI.AudioStop() : UI.AudioPlayFirstAsset(false);
            string after = UI.AudioGetSummary();
            if (rc == 0)
            {
                Log.Info("Audio", (playing ? "停止成功" : "播放成功") + " | " + after);
                if (_statusItem != null) _statusItem.Text = playing ? "音频: 已停止" : "音频: 正在播放测试音";
            }
            else
            {
                Log.Warn("Audio", "音频操作失败 rc=" + rc + "（无声卡时仍应可播放） | " + after);
                if (_statusItem != null) _statusItem.Text = "音频操作失败: rc=" + rc;
            }
            RefreshAssetAudioStatus();
        }

        /// <summary>资产扫描按钮。</summary>
        private static void OnAssetScanClick(ulong widgetId)
        {
            int rc = UI.AssetScan();
            string summary = UI.AssetGetSummary();
            if (rc == 0)
            {
                Log.Info("Asset", "资产扫描完成: " + summary);
                if (_statusItem != null) _statusItem.Text = "资产扫描完成";
            }
            else
            {
                Log.Warn("Asset", "资产扫描失败 rc=" + rc + "（先打开工程）: " + summary);
                if (_statusItem != null) _statusItem.Text = "资产扫描失败: rc=" + rc;
            }
            RefreshAssetAudioStatus();
        }

        /// <summary>导入 glTF/GLB 按钮：弹出文件浏览器。</summary>
        private static void OnAssetImportClick(ulong widgetId)
        {
            ShowAssetImportDialog();
        }

        private static void ShowAssetImportDialog()
        {
            ulong rootId = UI.GetRootId();
            float dialogWidth = 560f;
            float dialogHeight = 420f;
            _assetImportDialogId = UI.CreateDialog(rootId, "导入 glTF/GLB 资产", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_assetImportDialogId, _assetImportDialogResultCallback);
            ulong contentId = UI.CreateVStack(_assetImportDialogId, 10f);
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, "选择 .gltf / .glb 文件（自动注册子资产并写 .meta）:");
            _assetImportFileBrowserId = UI.CreateFileBrowser(
                contentId, 10f, 10f, dialogWidth - 60f, dialogHeight - 120f, ".");
            UI.FileBrowserSetFilter(_assetImportFileBrowserId, "*.*");
            UI.DialogSetContent(_assetImportDialogId, contentId);
            UI.DialogAddButton(_assetImportDialogId, "导入", 1);
            UI.DialogAddButton(_assetImportDialogId, "取消", 0);
            UI.DialogShow(_assetImportDialogId);
        }

        private static void OnAssetImportDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_assetImportDialogId);
            if (result != 1)
            {
                return;
            }
            string selectedPath = GetBrowserSelectedOrCurrent(_assetImportFileBrowserId);
            if (string.IsNullOrEmpty(selectedPath))
            {
                if (_statusItem != null) _statusItem.Text = "请选择 .gltf/.glb 文件";
                return;
            }
            string lower = selectedPath.ToLowerInvariant();
            if (!lower.EndsWith(".gltf") && !lower.EndsWith(".glb"))
            {
                Log.Warn("Asset", "导入失败: 不是 .gltf/.glb 文件: " + selectedPath);
                if (_statusItem != null) _statusItem.Text = "请选择 .gltf 或 .glb 文件";
                return;
            }

            int rc = UI.AssetImportGltf(selectedPath);
            if (rc == 0)
            {
                Log.Info("Asset", "glTF 导入成功: " + selectedPath + " | " + UI.AssetGetSummary());
                if (_statusItem != null) _statusItem.Text = "glTF 导入成功";
            }
            else
            {
                Log.Warn("Asset", "glTF 导入失败 rc=" + rc + ": " + selectedPath);
                if (_statusItem != null) _statusItem.Text = "glTF 导入失败: rc=" + rc;
            }
            RefreshAssetAudioStatus();
        }

        /// <summary>刷新状态栏资产/音频摘要（失败不影响编辑器）。</summary>
        private static void RefreshAssetAudioStatus()
        {
            if (_assetItem != null) _assetItem.Text = "资产: " + ShortenFfiSummary(UI.AssetGetSummary(), 18);
            if (_audioItem != null) _audioItem.Text = "音频: " + AudioShortLabel(UI.AudioGetSummary());
            if (_assetDbSummaryLabelId != 0)
            {
                UI.SetText(_assetDbSummaryLabelId, "资产: " + ShortenFfiSummary(UI.AssetGetSummary(), 90));
            }
        }

        /// <summary>FFI 摘要截断。</summary>
        private static string ShortenFfiSummary(string summary, int max)
        {
            if (string.IsNullOrEmpty(summary)) return "-";
            return summary.Length <= max ? summary : summary.Substring(0, max - 3) + "...";
        }

        /// <summary>音频摘要 → 状态栏短标签（ON/backend 或 OFF）。</summary>
        private static string AudioShortLabel(string summary)
        {
            if (string.IsNullOrEmpty(summary) || summary.IndexOf("audio=on") < 0)
            {
                return "OFF";
            }
            int backend = summary.IndexOf("backend=");
            if (backend < 0)
            {
                return "ON";
            }
            string rest = summary.Substring(backend + 8);
            int space = rest.IndexOf(' ');
            string name = space > 0 ? rest.Substring(0, space) : rest;
            return "ON/" + name;
        }

        // === 0.2 Prefab / SceneManager / Player 导出（task-21） ===

        /// <summary>保存当前编辑场景为命名场景槽。</summary>
        private static void OnSceneSaveClick(ulong widgetId)
        {
            string name = UI.SceneManagerCurrentName();
            if (string.IsNullOrEmpty(name))
            {
                name = UI.SceneManagerCount() == 0 ? "main" : "scene" + (UI.SceneManagerCount() + 1);
            }
            int rc = UI.SceneManagerSaveCurrent(name);
            if (rc == 0)
            {
                Log.Info("SceneManager", "保存场景槽: " + name + " | " + UI.SceneManagerCount() + " 个槽");
                if (_statusItem != null) _statusItem.Text = "场景已保存: " + name;
            }
            else
            {
                Log.Warn("SceneManager", "保存场景失败 rc=" + rc);
                if (_statusItem != null) _statusItem.Text = "场景保存失败: rc=" + rc;
            }
        }

        /// <summary>循环切换已保存的场景槽。</summary>
        private static void OnSceneSwitchClick(ulong widgetId)
        {
            string[] names = UI.SceneManagerNames();
            if (names.Length == 0)
            {
                if (_statusItem != null) _statusItem.Text = "没有场景槽，请先保存当前场景";
                return;
            }
            string current = UI.SceneManagerCurrentName();
            int nextIndex = 0;
            for (int i = 0; i < names.Length; i++)
            {
                if (names[i] == current)
                {
                    nextIndex = (i + 1) % names.Length;
                    break;
                }
            }
            string next = names[nextIndex];
            int rc = UI.SceneManagerSwitch(next);
            if (rc == 0)
            {
                RefreshSceneAfterSwitch();
                Log.Info("SceneManager", "切换场景: " + current + " -> " + next);
                if (_statusItem != null) _statusItem.Text = "切换场景: " + next;
            }
            else
            {
                Log.Warn("SceneManager", "切换场景失败 rc=" + rc + "（-2=当前场景未保存为槽）");
                if (_statusItem != null) _statusItem.Text = "切换场景失败: rc=" + rc;
            }
        }

        /// <summary>卸载当前场景槽。</summary>
        private static void OnSceneUnloadClick(ulong widgetId)
        {
            string current = UI.SceneManagerCurrentName();
            if (string.IsNullOrEmpty(current))
            {
                if (_statusItem != null) _statusItem.Text = "当前没有场景槽可卸载";
                return;
            }
            int rc = UI.SceneManagerUnload(current);
            if (rc == 0)
            {
                Log.Info("SceneManager", "卸载场景槽: " + current + " | 剩余 " + UI.SceneManagerCount());
                if (_statusItem != null) _statusItem.Text = "已卸载场景槽: " + current;
            }
            else
            {
                Log.Warn("SceneManager", "卸载场景失败 rc=" + rc);
                if (_statusItem != null) _statusItem.Text = "卸载场景失败: rc=" + rc;
            }
        }

        private static void OnSceneLoadClick(ulong widgetId) { ShowSceneFileDialog(1); }

        private static void OnPrefabInstantiateClick(ulong widgetId) { ShowSceneFileDialog(2); }

        /// <summary>把当前选中实体子树捕获为 Prefab JSON（assets/prefabs/）。</summary>
        private static void OnPrefabCaptureClick(ulong widgetId)
        {
            if (_gameScene == null || _selectedEntityId == 0)
            {
                if (_statusItem != null) _statusItem.Text = "请先在层级中选择实体再捕获 Prefab";
                return;
            }
            try
            {
                System.IO.Directory.CreateDirectory("assets/prefabs");
            }
            catch (Exception ex)
            {
                Log.Warn("Prefab", "创建 assets/prefabs 失败: " + ex.Message);
            }
            string name = "prefab_" + _selectedEntityId + "_" + System.DateTime.Now.ToString("HHmmss");
            string path = "assets/prefabs/" + name + ".prefab.json";
            int rc = UI.PrefabCaptureFromScene(_gameScene.ScenePtr, _selectedEntityId, name, path);
            if (rc == 0)
            {
                Log.Info("Prefab", "捕获 Prefab: " + name + " -> " + path);
                if (_statusItem != null) _statusItem.Text = "Prefab 已保存: " + path;
            }
            else
            {
                Log.Warn("Prefab", "捕获 Prefab 失败 rc=" + rc);
                if (_statusItem != null) _statusItem.Text = "Prefab 捕获失败: rc=" + rc;
            }
        }

        private static void ShowSceneFileDialog(int mode)
        {
            _sceneFileDialogMode = mode;
            ulong rootId = UI.GetRootId();
            float dialogWidth = 560f;
            float dialogHeight = 420f;
            _sceneFileDialogId = UI.CreateDialog(
                rootId, mode == 1 ? "加载场景文件" : "实例化 Prefab", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_sceneFileDialogId, _sceneFileDialogResultCallback);
            ulong contentId = UI.CreateVStack(_sceneFileDialogId, 10f);
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f,
                mode == 1 ? "选择 Project JSON 场景文件:" : "选择 .prefab.json 文件:");
            _sceneFileBrowserId = UI.CreateFileBrowser(
                contentId, 10f, 10f, dialogWidth - 60f, dialogHeight - 120f, ".");
            UI.FileBrowserSetFilter(_sceneFileBrowserId, "*.*");
            UI.DialogSetContent(_sceneFileDialogId, contentId);
            UI.DialogAddButton(_sceneFileDialogId, mode == 1 ? "加载" : "实例化", 1);
            UI.DialogAddButton(_sceneFileDialogId, "取消", 0);
            UI.DialogShow(_sceneFileDialogId);
        }

        private static void OnSceneFileDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_sceneFileDialogId);
            if (result != 1) return;
            string selectedPath = GetBrowserSelectedOrCurrent(_sceneFileBrowserId);
            if (string.IsNullOrEmpty(selectedPath))
            {
                if (_statusItem != null) _statusItem.Text = "未选择文件";
                return;
            }

            if (_sceneFileDialogMode == 1)
            {
                string name = System.IO.Path.GetFileNameWithoutExtension(selectedPath);
                if (string.IsNullOrEmpty(name)) name = "scene";
                int loadRc = UI.SceneManagerLoadFile(name, selectedPath);
                if (loadRc != 0)
                {
                    Log.Warn("SceneManager", "加载场景文件失败 rc=" + loadRc + ": " + selectedPath);
                    if (_statusItem != null) _statusItem.Text = "加载场景文件失败: rc=" + loadRc;
                    return;
                }
                // 切换需要“当前场景槽”；首次加载时自动把当前场景保存为 main。
                if (string.IsNullOrEmpty(UI.SceneManagerCurrentName()))
                {
                    int saveRc = UI.SceneManagerSaveCurrent("main");
                    if (saveRc != 0)
                    {
                        if (_statusItem != null) _statusItem.Text = "场景已加载，但保存当前场景失败: rc=" + saveRc;
                        return;
                    }
                }
                int switchRc = UI.SceneManagerSwitch(name);
                if (switchRc == 0)
                {
                    RefreshSceneAfterSwitch();
                    Log.Info("SceneManager", "加载并切换场景: " + name + " <- " + selectedPath);
                    if (_statusItem != null) _statusItem.Text = "加载并切换场景: " + name;
                }
                else
                {
                    if (_statusItem != null) _statusItem.Text = "场景已加载，切换失败: rc=" + switchRc;
                }
            }
            else
            {
                if (_gameScene == null)
                {
                    if (_statusItem != null) _statusItem.Text = "场景未初始化";
                    return;
                }
                ulong rootId = UI.PrefabInstantiate(_gameScene.ScenePtr, selectedPath, 0, 0f, 0f, 0f);
                if (rootId != 0)
                {
                    try
                    {
                        RefreshSceneTreeAfterLoad();
                        _gameScene.SelectEntity(rootId);
                        _selectedEntityId = rootId;
                        _selectedEntityIds.Clear();
                        _selectedEntityIds.Add(rootId);
                        RefreshEntityTree();
                        UpdatePropertiesPanel(rootId);
                        _propertiesDirty = true;
                    }
                    catch (Exception ex)
                    {
                        Log.Warn("Prefab", "实例化后刷新 UI 失败: " + ex.Message);
                    }
                    Log.Info("Prefab", "实例化 Prefab: " + selectedPath + " -> root=" + rootId);
                    if (_statusItem != null) _statusItem.Text = "Prefab 实例化: root=" + rootId;
                }
                else
                {
                    Log.Warn("Prefab", "实例化 Prefab 失败: " + selectedPath);
                    if (_statusItem != null) _statusItem.Text = "Prefab 实例化失败";
                }
            }
        }

        private static void RefreshSceneAfterSwitch()
        {
            try
            {
                RefreshSceneTreeAfterLoad();
                ResetUndoHistory();
            }
            catch (Exception ex)
            {
                Log.Warn("SceneManager", "刷新场景 UI 失败: " + ex.Message);
            }
        }

        /// <summary>导出 Player dry-run（双平台计划 + manifest，不复制二进制）。</summary>
        private static void OnPlayerExportClick(ulong widgetId)
        {
            RunPlayerExport(false);
        }

        /// <summary>真实构建当前平台 Player 包（Cooker + PlayerBuilder::build + validate）。</summary>
        private static void OnPlayerBuildClick(ulong widgetId)
        {
            RunPlayerExport(true);
        }

        private static void RunPlayerExport(bool realBuild)
        {
            // 输出目录留空：Rust 侧优先使用 Build Settings，默认 exports/<product>/<platform>。
            string json = BuildPlayerExportRequestJson();
            int rc = realBuild ? UI.PlayerExportBuild(json) : UI.PlayerExportDryRun(json);
            _lastPlayerExportSummary = UI.PlayerExportSummary();
            string action = realBuild ? "Player 真实构建" : "Player dry-run";
            if (rc == 0)
            {
                Log.Info("Player", action + " 完成:\n" + _lastPlayerExportSummary);
                if (_statusItem != null) _statusItem.Text = action + " 完成";
            }
            else
            {
                Log.Warn("Player", action + " 失败: " + _lastPlayerExportSummary);
                if (_statusItem != null) _statusItem.Text = action + " 失败（详见 Console）";
            }
        }

        /// <summary>构造 Player 导出请求 JSON（dry-run 与真实构建共用）。</summary>
        private static string BuildPlayerExportRequestJson(string outputDir)
        {
            string root = System.IO.Directory.GetCurrentDirectory();
            string product = Project.IsLoaded() ? Project.GetName() : "HezhouGame";
            return "{"
                + "\"project_root\":\"" + EscapeJsonPath(root) + "\","
                + "\"output_dir\":\"" + EscapeJsonPath(outputDir) + "\","
                + "\"product_name\":\"" + EscapeJsonPath(product) + "\","
                + "\"version\":\"0.1.0\","
                + "\"start_scene\":\"\","
                + "\"scripts_dir\":\"" + EscapeJsonPath(System.IO.Path.Combine(root, "scripts", "bin", "Mono")) + "\","
                + "\"runtime_dir\":\"" + EscapeJsonPath(System.IO.Path.Combine(root, "runtime", "mono")) + "\","
                + "\"required_font\":\"fonts/HarmonyOS_Sans_SC.ttf\","
                + "\"executable_path\":\"\""
                + "}";
        }

        private static string EscapeJsonPath(string value)
        {
            if (string.IsNullOrEmpty(value)) return "";
            return value.Replace("\\", "\\\\").Replace("\"", "\\\"");
        }

        // === 0.2 per-entity PBR 材质面板（task-26） ===

        /// <summary>在「渲染」属性 Tab 构建 PBR 材质编辑行（13 个输入 + 清除按钮 + 状态）。</summary>
        private static void BuildPbrMaterialPanel(ulong parent)
        {
            // 主题切换会重建控件树：重置控件 ID 映射后重新创建。
            _pbrFieldMap.Clear();
            for (int i = 0; i < _pbrFieldIds.Length; i++)
            {
                _pbrFieldIds[i] = 0;
            }
            _pbrPanelBuilt = true;

            UI.CreateLabel(parent, RIGHT_PANEL_WIDTH - 40f, 20f, "PBR 材质（当前实体）:");
            BuildPbrRow(parent, "Albedo", new int[] { 0, 1, 2, 3 }, 70f);
            BuildPbrRow(parent, "M / R", new int[] { 4, 5 }, 100f);
            BuildPbrRow(parent, "N / AO", new int[] { 6, 7 }, 100f);
            BuildPbrRow(parent, "Emissive", new int[] { 8, 9, 10 }, 60f);
            BuildPbrRow(parent, "强度 / 纹理", new int[] { 11 }, 70f);

            ulong textureRow = UI.CreateHStack(parent, 4f);
            UI.CreateLabel(textureRow, 60f, 22f, "Tex");
            _pbrFieldIds[12] = UI.CreateInputField(textureRow, 300f, 22f);
            UI.InputFieldSetPlaceholder(_pbrFieldIds[12], "assets/tex/albedo.png（可空）");
            UI.InputFieldSetOnChange(_pbrFieldIds[12], _pbrFieldChangeCallback);
            _pbrFieldMap[_pbrFieldIds[12]] = 12;

            ulong statusRow = UI.CreateHStack(parent, 6f);
            _pbrStatusLabelId = UI.CreateLabel(statusRow, 300f, 22f, "PBR: 选择实体后编辑");
            _pbrClearButtonId = UI.CreateButton(statusRow, 100f, 24f, "清除材质");
            UI.SetOnClick(_pbrClearButtonId, _pbrClearCallback);
            UI.ThemeSetBackground(_pbrClearButtonId, ThemeRole.ButtonSecondaryBg);
            UI.ThemeSetText(_pbrClearButtonId, ThemeRole.TextPrimary);

            ResetPbrFields();
        }

        private static void BuildPbrRow(ulong parent, string label, int[] fields, float fieldWidth)
        {
            ulong row = UI.CreateHStack(parent, 4f);
            UI.CreateLabel(row, 100f, 22f, label);
            for (int i = 0; i < fields.Length; i++)
            {
                ulong input = UI.CreateInputField(row, fieldWidth, 22f);
                UI.InputFieldSetOnChange(input, _pbrFieldChangeCallback);
                _pbrFieldIds[fields[i]] = input;
                _pbrFieldMap[input] = fields[i];
            }
        }

        private static void ResetPbrFields()
        {
            float[] defaults = new float[] {
                1f, 1f, 1f, 1f,      // albedo
                0f, 0.5f,            // metallic / roughness
                1f, 1f,              // normal_scale / ao
                0f, 0f, 0f, 1f,      // emissive / strength
            };
            for (int i = 0; i < 12; i++)
            {
                if (_pbrFieldIds[i] != 0)
                {
                    UI.InputFieldSetText(_pbrFieldIds[i], FormatPbrFloat(defaults[i]));
                }
            }
            if (_pbrFieldIds[12] != 0) UI.InputFieldSetText(_pbrFieldIds[12], "");
        }

        private static void SetPbrField(int index, float value)
        {
            if (index >= 0 && index < _pbrFieldIds.Length && _pbrFieldIds[index] != 0)
            {
                UI.InputFieldSetText(_pbrFieldIds[index], FormatPbrFloat(value));
            }
        }

        private static string FormatPbrFloat(float value)
        {
            return value.ToString("0.####", System.Globalization.CultureInfo.InvariantCulture);
        }

        private static float ParsePbrFloat(string text, float fallback)
        {
            float value;
            if (float.TryParse(text, System.Globalization.NumberStyles.Float,
                System.Globalization.CultureInfo.InvariantCulture, out value))
            {
                return value;
            }
            return fallback;
        }

        private static float GetPbrField(int index, float fallback)
        {
            if (index < 0 || index >= _pbrFieldIds.Length || _pbrFieldIds[index] == 0)
            {
                return fallback;
            }
            return ParsePbrFloat(UI.InputFieldGetText(_pbrFieldIds[index]), fallback);
        }

        private static string GetPbrTextureField()
        {
            if (_pbrFieldIds[12] == 0) return "";
            return UI.InputFieldGetText(_pbrFieldIds[12]) ?? "";
        }

        /// <summary>当前选中实体的 PBR 面板状态。</summary>
        private static ulong PbrTargetEntity()
        {
            return (_gameScene == null) ? 0UL : _selectedEntityId;
        }

        /// <summary>选择变化时把实体材质加载到 PBR 面板。</summary>
        private static void RefreshPbrMaterialPanel(ulong entityId)
        {
            if (!_pbrPanelBuilt) return;
            if (_gameScene == null || entityId == 0)
            {
                ResetPbrFields();
                if (_pbrStatusLabelId != 0)
                {
                    UI.SetText(_pbrStatusLabelId, "PBR: 选择实体后编辑");
                }
                return;
            }

            string json = UI.SceneGetEntityPbrMaterial(_gameScene.ScenePtr, entityId);
            if (string.IsNullOrEmpty(json))
            {
                ResetPbrFields();
                if (_pbrStatusLabelId != 0)
                {
                    UI.SetText(_pbrStatusLabelId, "PBR: 未设置（编辑任意字段即创建）");
                }
                return;
            }

            float[] albedo = ParsePbrArray(json, "albedo", 4, new float[] { 1f, 1f, 1f, 1f });
            float[] emissive = ParsePbrArray(json, "emissive", 3, new float[] { 0f, 0f, 0f });
            SetPbrField(0, albedo[0]);
            SetPbrField(1, albedo[1]);
            SetPbrField(2, albedo[2]);
            SetPbrField(3, albedo[3]);
            SetPbrField(4, PbrJsonFloat(json, "metallic", 0f));
            SetPbrField(5, PbrJsonFloat(json, "roughness", 0.5f));
            SetPbrField(6, PbrJsonFloat(json, "normal_scale", 1f));
            SetPbrField(7, PbrJsonFloat(json, "ao", 1f));
            SetPbrField(8, emissive[0]);
            SetPbrField(9, emissive[1]);
            SetPbrField(10, emissive[2]);
            SetPbrField(11, PbrJsonFloat(json, "emissive_strength", 1f));
            if (_pbrFieldIds[12] != 0)
            {
                UI.InputFieldSetText(_pbrFieldIds[12], ParsePbrString(json, "albedo_texture_path"));
            }
            if (_pbrStatusLabelId != 0)
            {
                UI.SetText(_pbrStatusLabelId, "PBR: 已设置（实体 " + entityId + "）");
            }
        }

        /// <summary>由面板输入构造 PBR 材质 JSON。</summary>
        private static string BuildPbrJson()
        {
            string texture = GetPbrTextureField().Trim();
            string textureJson = texture.Length == 0 ? "null" : "\"" + EscapeJsonPath(texture) + "\"";
            return "{"
                + "\"albedo\":[" + FormatPbrFloat(GetPbrField(0, 1f)) + ","
                + FormatPbrFloat(GetPbrField(1, 1f)) + ","
                + FormatPbrFloat(GetPbrField(2, 1f)) + ","
                + FormatPbrFloat(GetPbrField(3, 1f)) + "],"
                + "\"metallic\":" + FormatPbrFloat(GetPbrField(4, 0f)) + ","
                + "\"roughness\":" + FormatPbrFloat(GetPbrField(5, 0.5f)) + ","
                + "\"normal_scale\":" + FormatPbrFloat(GetPbrField(6, 1f)) + ","
                + "\"ao\":" + FormatPbrFloat(GetPbrField(7, 1f)) + ","
                + "\"emissive\":[" + FormatPbrFloat(GetPbrField(8, 0f)) + ","
                + FormatPbrFloat(GetPbrField(9, 0f)) + ","
                + FormatPbrFloat(GetPbrField(10, 0f)) + "],"
                + "\"emissive_strength\":" + FormatPbrFloat(GetPbrField(11, 1f)) + ","
                + "\"albedo_texture_path\":" + textureJson
                + "}";
        }

        private static void OnPbrFieldChanged(ulong widgetId, string text)
        {
            ulong entityId = PbrTargetEntity();
            if (_gameScene == null || entityId == 0)
            {
                if (_pbrStatusLabelId != 0) UI.SetText(_pbrStatusLabelId, "PBR: 请先选择实体");
                return;
            }
            int rc = UI.SceneSetEntityPbrMaterial(_gameScene.ScenePtr, entityId, BuildPbrJson());
            if (rc == 0)
            {
                int synced = UI.EditorSyncPbrMaterialsNow();
                if (_pbrStatusLabelId != 0)
                {
                    UI.SetText(_pbrStatusLabelId, "PBR: 已更新（renderer 覆盖 " + synced + "）");
                }
                _propertiesDirty = true;
            }
            else
            {
                Log.Warn("PBR", "更新实体 PBR 材质失败: entity=" + entityId + ", rc=" + rc);
                if (_pbrStatusLabelId != 0) UI.SetText(_pbrStatusLabelId, "PBR: 更新失败 rc=" + rc);
            }
        }

        private static void OnPbrClearClick(ulong widgetId)
        {
            ulong entityId = PbrTargetEntity();
            if (_gameScene == null || entityId == 0)
            {
                if (_pbrStatusLabelId != 0) UI.SetText(_pbrStatusLabelId, "PBR: 请先选择实体");
                return;
            }
            int rc = UI.SceneSetEntityPbrMaterial(_gameScene.ScenePtr, entityId, "null");
            if (rc == 0)
            {
                int synced = UI.EditorSyncPbrMaterialsNow();
                ResetPbrFields();
                Log.Info("PBR", "已清除实体 PBR 材质: entity=" + entityId + "（renderer 覆盖 " + synced + "）");
                if (_pbrStatusLabelId != 0)
                {
                    UI.SetText(_pbrStatusLabelId, "PBR: 已清除（renderer 覆盖 " + synced + "）");
                }
                _propertiesDirty = true;
            }
            else
            {
                Log.Warn("PBR", "清除实体 PBR 材质失败: entity=" + entityId + ", rc=" + rc);
            }
        }

        /// <summary>从 JSON 数组字段解析前 count 个 float。</summary>
        private static float[] ParsePbrArray(string json, string key, int count, float[] fallback)
        {
            float[] result = new float[count];
            System.Array.Copy(fallback, result, count);
            if (string.IsNullOrEmpty(json)) return result;
            int keyIndex = json.IndexOf("\"" + key + "\":");
            if (keyIndex < 0) return result;
            int start = json.IndexOf('[', keyIndex);
            int end = json.IndexOf(']', start + 1);
            if (start < 0 || end < 0) return result;
            string body = json.Substring(start + 1, end - start - 1);
            string[] parts = body.Split(',');
            for (int i = 0; i < count && i < parts.Length; i++)
            {
                result[i] = ParsePbrFloat(parts[i].Trim(), fallback[i]);
            }
            return result;
        }

        private static float PbrJsonFloat(string json, string key, float fallback)
        {
            if (string.IsNullOrEmpty(json)) return fallback;
            int keyIndex = json.IndexOf("\"" + key + "\":");
            if (keyIndex < 0) return fallback;
            int start = keyIndex + key.Length + 3;
            int end = start;
            while (end < json.Length && json[end] != ',' && json[end] != '}') end++;
            return ParsePbrFloat(json.Substring(start, end - start).Trim(), fallback);
        }

        private static string ParsePbrString(string json, string key)
        {
            if (string.IsNullOrEmpty(json)) return "";
            int keyIndex = json.IndexOf("\"" + key + "\":");
            if (keyIndex < 0) return "";
            int start = keyIndex + key.Length + 3;
            while (start < json.Length && char.IsWhiteSpace(json[start])) start++;
            if (start >= json.Length || json[start] != '"') return "";
            int end = json.IndexOf('"', start + 1);
            if (end < 0) return "";
            return json.Substring(start + 1, end - start - 1);
        }

        // === 0.3 Profiler 面板（task-39） ===

        /// <summary>刷新 Profiler 面板（每 30 帧调用；暂停时跳过）。</summary>
        private static void RefreshProfilerPanel()
        {
            if (_profilerListId == 0 || _profilerItemIds[0] == 0) return;
            string json = UI.ProfilerSnapshotJson();
            if (string.IsNullOrEmpty(json))
            {
                SetProfilerItem(0, "Profiler: 暂无数据（snapshot 为空）");
                return;
            }

            float fps = PbrJsonFloat(json, "fps", 0f);
            float dfxFps = PbrJsonFloat(json, "dfx_fps", 0f);
            float avg = PbrJsonFloat(json, "frame_ms_avg", 0f);
            float p95 = PbrJsonFloat(json, "frame_ms_p95", 0f);
            float max = PbrJsonFloat(json, "frame_ms_max", 0f);
            float startup = PbrJsonFloat(json, "startup_ms", 0f);
            long frames = (long)PbrJsonFloat(json, "frame_count", 0f);
            long samples = (long)PbrJsonFloat(json, "sample_count", 0f);
            long capacity = (long)PbrJsonFloat(json, "ring_capacity", 120f);
            long entities = (long)PbrJsonFloat(json, "entity_count", 0f);
            long pbr = (long)PbrJsonFloat(json, "pbr_entity_count", 0f);
            long skinned = (long)PbrJsonFloat(json, "skinned_mesh_count", 0f);
            long assetCount = (long)PbrJsonFloat(json, "asset_count", 0f);
            long assetTotal = (long)PbrJsonFloat(json, "asset_total_count", 0f);
            float memUsed = PbrJsonFloat(json, "memory_used_mb", 0f);
            float memAvailable = PbrJsonFloat(json, "memory_available_mb", 0f);
            float cpu = PbrJsonFloat(json, "cpu_percent", 0f);
            string audioBackend = ParsePbrString(json, "audio_backend");
            if (string.IsNullOrEmpty(audioBackend)) audioBackend = "off";

            SetProfilerItem(0, "FPS: " + FormatPbrFloat(fps) + "（DFX " + FormatPbrFloat(dfxFps) + "）"
                + "  帧 ms: 平均 " + FormatPbrFloat(avg) + " / p95 " + FormatPbrFloat(p95)
                + " / max " + FormatPbrFloat(max));
            SetProfilerItem(1, "启动耗时: " + FormatPbrFloat(startup) + " ms"
                + "  采样: " + samples + "/" + capacity + "  累计帧: " + frames);
            SetProfilerItem(2, "实体: " + entities + "  PBR 材质: " + pbr
                + "  蒙皮网格: " + skinned);
            SetProfilerItem(3, "资产: " + assetCount + "/" + assetTotal
                + "  音频: " + audioBackend);
            SetProfilerItem(4, "内存: " + FormatPbrFloat(memUsed) + " MB（可用 "
                + FormatPbrFloat(memAvailable) + " MB）  CPU: " + FormatPbrFloat(cpu) + "%");

            if (_profilerStatusLabelId != 0)
            {
                UI.SetText(_profilerStatusLabelId,
                    _profilerPaused ? "Profiler: 已暂停（数据保留）" : "Profiler: 刷新中（每 30 帧）");
            }
            if (!_profilerLoggedOnce && frames > 0)
            {
                _profilerLoggedOnce = true;
                Log.Info("Profiler", "snapshot: frames=" + frames + " samples=" + samples
                    + " fps=" + FormatPbrFloat(fps) + " avg_ms=" + FormatPbrFloat(avg)
                    + " startup_ms=" + FormatPbrFloat(startup)
                    + " entities=" + entities + " pbr=" + pbr + " skinned=" + skinned
                    + " assets=" + assetCount + "/" + assetTotal
                    + " audio=" + audioBackend);
            }
        }

        private static void SetProfilerItem(int index, string text)
        {
            if (index >= 0 && index < _profilerItemIds.Length && _profilerItemIds[index] != 0)
            {
                UI.SetListItemText(_profilerItemIds[index], text);
            }
        }

        private static void OnProfilerPauseClick(ulong widgetId)
        {
            _profilerPaused = !_profilerPaused;
            if (_profilerPauseButtonId != 0)
            {
                UI.SetText(_profilerPauseButtonId, _profilerPaused ? "继续刷新" : "暂停刷新");
            }
            if (_profilerStatusLabelId != 0)
            {
                UI.SetText(_profilerStatusLabelId,
                    _profilerPaused ? "Profiler: 已暂停（数据保留）" : "Profiler: 刷新中（每 30 帧）");
            }
            Log.Info("Profiler", _profilerPaused ? "面板已暂停" : "面板已恢复");
        }

        private static void OnProfilerResetClick(ulong widgetId)
        {
            int rc = UI.ProfilerReset();
            _profilerLoggedOnce = false;
            if (rc == 0)
            {
                RefreshProfilerPanel();
                Log.Info("Profiler", "帧统计已重置（启动耗时保留）");
                if (_profilerStatusLabelId != 0)
                {
                    UI.SetText(_profilerStatusLabelId, "Profiler: 统计已重置（每 30 帧刷新）");
                }
            }
            else
            {
                Log.Warn("Profiler", "重置失败 rc=" + rc);
            }
        }

        /// <summary>构造 Player 导出请求 JSON（dry-run/真实构建共用；未填字段由 Build Settings 补齐）。</summary>
        private static string BuildPlayerExportRequestJson()
        {
            EnsureBuildSettingsLoaded();
            string root = System.IO.Directory.GetCurrentDirectory();
            string product = GetBuildSettingField(0).Trim();
            string version = GetBuildSettingField(2).Trim();
            string startScene = GetBuildSettingField(3).Trim();
            string font = GetBuildSettingField(10).Trim();
            string fontJson = font.Length == 0 ? "\"\"" : "\"" + EscapeJsonPath(font) + "\"";
            return "{"
                + "\"project_root\":\"" + EscapeJsonPath(root) + "\","
                + "\"output_dir\":\"\","
                + "\"product_name\":\"" + EscapeJsonPath(product) + "\","
                + "\"version\":\"" + EscapeJsonPath(version) + "\","
                + "\"start_scene\":\"" + EscapeJsonPath(startScene) + "\","
                + "\"scripts_dir\":\"" + EscapeJsonPath(System.IO.Path.Combine(root, "scripts", "bin", "Mono")) + "\","
                + "\"runtime_dir\":\"" + EscapeJsonPath(System.IO.Path.Combine(root, "runtime", "mono")) + "\","
                + "\"required_font\":" + fontJson + ","
                + "\"executable_path\":\"\""
                + "}";
        }

        // === 0.3 Build Settings（task-41） ===

        /// <summary>读取工程 build.settings.json 并回填面板字段。</summary>
        private static void LoadBuildSettingsIntoFields()
        {
            if (_buildSettingFieldIds == null || _buildSettingFieldIds[0] == 0)
            {
                return;
            }
            string json = UI.BuildSettingsSnapshotJson();
            if (string.IsNullOrEmpty(json))
            {
                if (_buildSettingsStatusLabelId != 0)
                {
                    UI.SetText(_buildSettingsStatusLabelId, "Build Settings: 读取失败（snapshot 为空）");
                }
                return;
            }
            SetBuildSettingField(0, ParsePbrString(json, "product_name"));
            SetBuildSettingField(1, ParsePbrString(json, "executable_name"));
            SetBuildSettingField(2, ParsePbrString(json, "version"));
            SetBuildSettingField(3, ParsePbrString(json, "start_scene"));
            SetBuildSettingField(4, ParsePbrString(json, "quality"));
            SetBuildSettingField(5, ((long)PbrJsonFloat(json, "width", 1280f)).ToString());
            SetBuildSettingField(6, ((long)PbrJsonFloat(json, "height", 720f)).ToString());
            SetBuildSettingField(7, ParsePbrString(json, "mode"));
            SetBuildSettingField(8, ParsePbrStringArray(json, "targets"));
            SetBuildSettingField(9, ParsePbrString(json, "log_dir"));
            SetBuildSettingField(10, ParsePbrString(json, "required_font"));
            _buildSettingsLoaded = true;
            if (_buildSettingsStatusLabelId != 0)
            {
                UI.SetText(_buildSettingsStatusLabelId,
                    "Build Settings: 已读取（修改后点“保存设置”；导出优先使用该配置）");
            }
        }

        private static void SetBuildSettingField(int index, string text)
        {
            if (index >= 0 && index < _buildSettingFieldIds.Length && _buildSettingFieldIds[index] != 0)
            {
                UI.InputFieldSetText(_buildSettingFieldIds[index], text ?? "");
            }
        }

        private static string GetBuildSettingField(int index)
        {
            if (index < 0 || index >= _buildSettingFieldIds.Length || _buildSettingFieldIds[index] == 0)
            {
                return "";
            }
            return UI.InputFieldGetText(_buildSettingFieldIds[index]) ?? "";
        }

        private static void EnsureBuildSettingsLoaded()
        {
            if (!_buildSettingsLoaded && _buildSettingFieldIds[0] != 0)
            {
                LoadBuildSettingsIntoFields();
            }
        }

        /// <summary>由面板字段构造 build.settings.json 内容。</summary>
        private static string BuildBuildSettingsJson()
        {
            string product = GetBuildSettingField(0).Trim();
            string executable = GetBuildSettingField(1).Trim();
            string version = GetBuildSettingField(2).Trim();
            string startScene = GetBuildSettingField(3).Trim();
            string quality = GetBuildSettingField(4).Trim();
            long width = (long)ParsePbrFloat(GetBuildSettingField(5), 1280f);
            long height = (long)ParsePbrFloat(GetBuildSettingField(6), 720f);
            string mode = GetBuildSettingField(7).Trim();
            string targets = GetBuildSettingField(8).Trim();
            string logDir = GetBuildSettingField(9).Trim();
            string font = GetBuildSettingField(10).Trim();

            string[] targetParts = targets.Split(new char[] { ',', ';', ' ' },
                System.StringSplitOptions.RemoveEmptyEntries);
            string targetsJson = "";
            for (int i = 0; i < targetParts.Length; i++)
            {
                if (targetsJson.Length > 0) targetsJson += ",";
                targetsJson += "\"" + EscapeJsonPath(targetParts[i].Trim().ToLower()) + "\"";
            }
            string fontJson = font.Length == 0 ? "null" : "\"" + EscapeJsonPath(font) + "\"";

            return "{"
                + "\"product_name\":\"" + EscapeJsonPath(product) + "\","
                + "\"executable_name\":\"" + EscapeJsonPath(executable) + "\","
                + "\"start_scene\":\"" + EscapeJsonPath(startScene) + "\","
                + "\"quality\":\"" + EscapeJsonPath(quality) + "\","
                + "\"window\":{\"width\":" + width + ",\"height\":" + height
                + ",\"mode\":\"" + EscapeJsonPath(mode) + "\"},"
                + "\"targets\":[" + targetsJson + "],"
                + "\"version\":\"" + EscapeJsonPath(version) + "\","
                + "\"log_dir\":\"" + EscapeJsonPath(logDir) + "\","
                + "\"required_font\":" + fontJson
                + "}";
        }

        private static void OnBuildSettingsLoadClick(ulong widgetId)
        {
            LoadBuildSettingsIntoFields();
            Log.Info("BuildSettings", "已重新读取 build.settings.json");
            if (_buildSettingsStatusLabelId != 0)
            {
                UI.SetText(_buildSettingsStatusLabelId, "Build Settings: 已重新读取");
            }
        }

        private static void OnBuildSettingsValidateClick(ulong widgetId)
        {
            string json = BuildBuildSettingsJson();
            string errors;
            int rc = UI.BuildSettingsValidateJson(json, out errors);
            if (rc == 0)
            {
                Log.Info("BuildSettings", "校验通过");
                if (_buildSettingsStatusLabelId != 0)
                {
                    UI.SetText(_buildSettingsStatusLabelId, "Build Settings: 校验通过");
                }
            }
            else
            {
                Log.Warn("BuildSettings", "校验失败: " + errors);
                if (_buildSettingsStatusLabelId != 0)
                {
                    UI.SetText(_buildSettingsStatusLabelId,
                        "Build Settings: 校验失败（详见 Console）");
                }
            }
        }

        private static void OnBuildSettingsSaveClick(ulong widgetId)
        {
            string json = BuildBuildSettingsJson();
            string errors;
            int validateRc = UI.BuildSettingsValidateJson(json, out errors);
            if (validateRc != 0)
            {
                Log.Warn("BuildSettings", "保存前校验失败: " + errors);
                if (_buildSettingsStatusLabelId != 0)
                {
                    UI.SetText(_buildSettingsStatusLabelId,
                        "Build Settings: 校验失败，未保存（详见 Console）");
                }
                return;
            }
            int saveRc = UI.BuildSettingsSaveJson(json);
            if (saveRc == 0)
            {
                _buildSettingsLoaded = true;
                Log.Info("BuildSettings", "已保存 build.settings.json");
                if (_buildSettingsStatusLabelId != 0)
                {
                    UI.SetText(_buildSettingsStatusLabelId,
                        "Build Settings: 已保存（导出将使用该配置）");
                }
            }
            else
            {
                Log.Warn("BuildSettings", "保存失败 rc=" + saveRc);
                if (_buildSettingsStatusLabelId != 0)
                {
                    UI.SetText(_buildSettingsStatusLabelId,
                        "Build Settings: 保存失败 rc=" + saveRc);
                }
            }
        }

        /// <summary>解析 JSON 字符串数组字段（如 targets）。</summary>
        private static string ParsePbrStringArray(string json, string key)
        {
            if (string.IsNullOrEmpty(json)) return "";
            int keyIndex = json.IndexOf("\"" + key + "\":");
            if (keyIndex < 0) return "";
            int start = json.IndexOf('[', keyIndex);
            int end = json.IndexOf(']', start + 1);
            if (start < 0 || end < 0) return "";
            string body = json.Substring(start + 1, end - start - 1);
            string[] parts = body.Split(',');
            string result = "";
            for (int i = 0; i < parts.Length; i++)
            {
                string part = parts[i].Trim();
                if (part.StartsWith("\"") && part.EndsWith("\"") && part.Length >= 2)
                {
                    if (result.Length > 0) result += ",";
                    result += part.Substring(1, part.Length - 2);
                }
            }
            return result;
        }

        // === 0.3 动画时间轴（task-43） ===

        private static void OnAnimLoadClick(ulong widgetId)
        {
            LoadAnimationAssetFromField();
        }

        private static void OnAnimBrowseClick(ulong widgetId)
        {
            ShowAnimationFileDialog();
        }

        private static void ShowAnimationFileDialog()
        {
            ulong rootId = UI.GetRootId();
            _animFileDialogId = UI.CreateDialog(rootId, "选择 glTF/GLB 资产", 560f, 420f);
            UI.DialogSetOnResult(_animFileDialogId, _animFileDialogResultCallback);
            ulong contentId = UI.CreateVStack(_animFileDialogId, 10f);
            UI.CreateLabel(contentId, 520f, 30f, "选择 glTF/GLB 资产文件:");
            _animFileBrowserId = UI.CreateFileBrowser(contentId, 10f, 10f, 500f, 300f, ".");
            UI.FileBrowserSetFilter(_animFileBrowserId, "*.gltf");
            UI.DialogSetContent(_animFileDialogId, contentId);
            UI.DialogAddButton(_animFileDialogId, "加载", 1);
            UI.DialogAddButton(_animFileDialogId, "取消", 0);
            UI.DialogShow(_animFileDialogId);
        }

        private static void OnAnimationFileDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_animFileDialogId);
            if (result != 1) return;
            string selected = GetBrowserSelectedOrCurrent(_animFileBrowserId);
            if (string.IsNullOrEmpty(selected))
            {
                SetAnimStatus("动画: 未选择文件");
                return;
            }
            UI.InputFieldSetText(_animPathFieldId, selected);
            LoadAnimationAssetFromField();
        }

        /// <summary>读取路径字段 → animation_asset_summary → 填 clip 下拉/时间轴。</summary>
        private static void LoadAnimationAssetFromField()
        {
            if (_animPathFieldId == 0 || _animStatusLabelId == 0) return;
            string path = UI.InputFieldGetText(_animPathFieldId);
            if (string.IsNullOrEmpty(path) || path.Trim().Length == 0)
            {
                SetAnimStatus("动画: 请输入 glTF/GLB 资产路径");
                return;
            }
            path = path.Trim();
            string json = UI.AnimationAssetSummary(path);
            string error = ParsePbrString(json, "error");
            if (error.Length > 0)
            {
                _animLoaded = false;
                _animPlaying = false;
                _animClipNames = new string[0];
                _animClipDurations = new float[0];
                _animDuration = 0f;
                SetAnimStatus("动画: 加载失败（" + error + "）");
                Log.Warn("Animation", "加载失败: " + error);
                return;
            }

            string[] names;
            float[] durations;
            ParseAnimationClips(json, out names, out durations);
            _animClipNames = names;
            _animClipDurations = durations;
            _animDuration = durations.Length > 0 ? durations[0] : 0f;
            _animLoaded = true;
            _animPlaying = false;
            int jointCount = (int)PbrJsonFloat(json, "joint_count", 0f);
            string[] options = names.Length > 0 ? names : new string[] { "(无 clip)" };
            UI.DropdownSetOptions(_animClipDropdownId, options);
            UI.DropdownSetSelected(_animClipDropdownId, 0);
            if (names.Length > 0) UI.AnimationPreviewSetClip(0);
            UI.AnimationPreviewSetTime(0f);
            UI.SliderSetRange(_animSliderId, 0f, _animDuration > 0f ? _animDuration : 1f);
            UI.SliderSetStep(_animSliderId, _animDuration > 0f ? _animDuration / 200f : 0.01f);
            UI.SliderSetValue(_animSliderId, 0f);
            UpdateAnimTimeLabel(0f);
            SetAnimStatus("动画: " + names.Length + " clip / skeleton joints=" + jointCount
                + " / 时长 " + FormatPbrFloat(_animDuration) + "s");
            Log.Info("Animation", "加载完成: " + path + " clips=" + names.Length + " joints=" + jointCount);
        }

        /// <summary>解析 summary JSON 中 clips[] 的 name/duration。</summary>
        private static void ParseAnimationClips(string json, out string[] names, out float[] durations)
        {
            System.Collections.Generic.List<string> nameList =
                new System.Collections.Generic.List<string>();
            System.Collections.Generic.List<float> durationList =
                new System.Collections.Generic.List<float>();
            int clipsIndex = json.IndexOf("\"clips\":[");
            if (clipsIndex >= 0)
            {
                int pos = clipsIndex + "\"clips\":[".Length;
                while (pos < json.Length)
                {
                    int open = json.IndexOf('{', pos);
                    if (open < 0) break;
                    int close = json.IndexOf('}', open + 1);
                    if (close < 0) break;
                    string segment = json.Substring(open, close - open + 1);
                    nameList.Add(ParsePbrString(segment, "name"));
                    durationList.Add(PbrJsonFloat(segment, "duration", 0f));
                    int nextOpen = json.IndexOf('{', close + 1);
                    if (nextOpen < 0) break;
                    pos = nextOpen;
                }
            }
            names = nameList.ToArray();
            durations = durationList.ToArray();
        }

        private static void OnAnimClipSelected(ulong widgetId, ulong index)
        {
            if (!_animLoaded) return;
            if (UI.AnimationPreviewSetClip((int)index) != 0) return;
            _animPlaying = false;
            UI.AnimationPreviewPause();
            _animDuration = (index < (ulong)_animClipDurations.Length)
                ? _animClipDurations[index] : 0f;
            UI.SliderSetRange(_animSliderId, 0f, _animDuration > 0f ? _animDuration : 1f);
            UI.SliderSetValue(_animSliderId, 0f);
            UpdateAnimTimeLabel(0f);
            SetAnimStatus("动画: clip=" + index + " 时长 " + FormatPbrFloat(_animDuration) + "s");
        }

        private static void OnAnimPlayClick(ulong widgetId)
        {
            if (!_animLoaded)
            {
                SetAnimStatus("动画: 请先加载 glTF 资产");
                return;
            }
            if (_animDuration <= 0f)
            {
                SetAnimStatus("动画: 当前 clip 无时长，无法播放");
                return;
            }
            UI.AnimationPreviewPlay();
            _animPlaying = true;
            SetAnimStatus("动画: 播放中（" + FormatPbrFloat(_animDuration) + "s）");
        }

        private static void OnAnimPauseClick(ulong widgetId)
        {
            UI.AnimationPreviewPause();
            _animPlaying = false;
            SetAnimStatus("动画: 已暂停");
        }

        private static void OnAnimResetClick(ulong widgetId)
        {
            UI.AnimationPreviewPause();
            _animPlaying = false;
            UI.AnimationPreviewSetTime(0f);
            if (_animSliderId != 0) UI.SliderSetValue(_animSliderId, 0f);
            UpdateAnimTimeLabel(0f);
            SetAnimStatus("动画: 回到起点");
        }

        private static void OnAnimSliderChanged(ulong widgetId, float value)
        {
            if (!_animLoaded) return;
            _animPlaying = false;
            UI.AnimationPreviewPause();
            float actual = (float)UI.AnimationPreviewSetTime(value);
            UpdateAnimTimeLabel(actual);
        }

        private static void UpdateAnimTimeLabel(float time)
        {
            if (_animTimeLabelId != 0)
            {
                UI.SetText(_animTimeLabelId, "时间: " + FormatPbrFloat(time)
                    + " / " + FormatPbrFloat(_animDuration) + " s");
            }
        }

        private static void SetAnimStatus(string text)
        {
            if (_animStatusLabelId != 0)
            {
                UI.SetText(_animStatusLabelId, text);
            }
        }

        // === 自检用只读辅助（TestRunner 调用） ===

        /// <summary>当前编辑场景指针（0=未初始化）。</summary>
        public static IntPtr TestGetScenePtr()
        {
            return _gameScene != null ? _gameScene.ScenePtr : IntPtr.Zero;
        }

        /// <summary>当前场景第一个根实体 ID（0=无）。</summary>
        public static ulong TestGetFirstRootEntityId()
        {
            IntPtr scenePtr = TestGetScenePtr();
            if (scenePtr == IntPtr.Zero) return 0;
            if (UI.SceneRootEntityCount(scenePtr) == 0) return 0;
            return UI.SceneGetRootEntityIdAt(scenePtr, 0);
        }

        /// <summary>当前场景实体数量（自检用）。</summary>
        public static int TestGetSceneEntityCount()
        {
            IntPtr scenePtr = TestGetScenePtr();
            if (scenePtr == IntPtr.Zero) return 0;
            return (int)UI.SceneGetEntityCount(scenePtr);
        }

        /// <summary>自检用：创建 Cube 实体并返回 ID。</summary>
        public static ulong TestCreateCubeEntity()
        {
            return _gameScene != null ? _gameScene.CreateCube() : 0;
        }

        /// <summary>自检用：删除实体。</summary>
        public static void TestRemoveEntity(ulong entityId)
        {
            if (_gameScene != null && entityId != 0)
            {
                _gameScene.RemoveEntity(entityId);
            }
        }

        public static void ApplyTheme(ThemeMode mode, bool rebuild)
        {
            UI.ThemeSetMode(mode);
            SaveThemePreference();
            if (rebuild)
            {
                RebuildUIForTheme();
            }
        }

        /// <summary>
        /// 主题切换必须重建控件树：当前控件颜色在构造时取自主题。
        /// Scene、选中实体、相机状态保留。
        /// </summary>
        private static void RebuildUIForTheme()
        {
            if (_gameScene == null) return;
            _galleryPanelId = 0;

            ulong selected = _selectedEntityId;
            ulong savedPropertyTab = _propsTabWidget != null ? _propsTabWidget.GetActive() : 0;
            try { ClearPropertiesPanel(); }
            catch (Exception ex) { Log.Error("Editor", "主题切换清理属性面板失败: " + ex.Message); }

            UI.ClearWidgetTreeKeepCallbacks();

            _selectedEntityId = selected;
            _selectedEntityIds.Clear();
            _propertyDescriptors.Clear();
            _scriptRowIds.Clear();
            _scriptPropertyWidgetIds.Clear();
            _scriptPropertyInfoMap.Clear();
            _scriptPropertySliderCallbacks.Clear();
            _scriptPropertyInputCallbacks.Clear();
            _scriptMinMaxInitialToMainWidgetMap.Clear();
            _lastPropertiesEntityId = 0;
            _lastScriptBindingCount = -1;

            CreateEditorLayout();
            ApplyDockLayout();
            OnResize(_screenWidth, _screenHeight);
            if (_propsTabWidget != null) _propsTabWidget.SetActive(savedPropertyTab);

            if (selected != 0)
            {
                _gameScene.SelectEntity(selected);
                _selectedEntityId = selected;
                _selectedEntityIds.Add(selected);
                RefreshEntityTree();
                _propertiesDirty = true;
                _lastPropertiesEntityId = 0;
                _lastScriptBindingCount = -1;
                UpdatePropertiesPanel(selected);
            }

            if (_statusItem != null)
            {
                _statusItem.Text = Theme.IsDark ? "主题: 深色" : "主题: 浅色";
            }
            if (_themeItem != null)
            {
                _themeItem.Text = Theme.IsDark ? "主题: 深色" : "主题: 浅色";
            }
            UpdateSelectionStatus();
        }

        public static void TestSetTheme(ThemeMode mode)
        {
            ApplyTheme(mode, true);
        }

        public static void TestClickThemeButton()
        {
            OnThemeClick(0);
        }

        public static string TestGetThemeName()
        {
            return Theme.IsDark ? "dark" : "light";
        }
    }
}