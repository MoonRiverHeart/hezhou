using System;
using System.Runtime.InteropServices;

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
            
            ShowWorkingDirectoryDialog();
            
            UI.RegisterResizeCallback(_resizeCallback);
            UI.RegisterGlobalClickCallback(_globalClickCallback);
            UI.RegisterKeyCallback(_keyCallback);
            UI.RegisterMouseMoveCallback(_mouseMoveCallback);
            UI.RegisterMouseWheelCallback(_mouseWheelCallback);
            UI.RegisterTreeNodeRightClickCallback(_treeNodeRightClickCallback);
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
                    _fpsItem.Text = $"FPS: {((int)(1000f / deltaTime))}";
                    
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
                    
                    if (_previewSelected && _gameScene != null && _gameScene.GetGameState() == GameState.Running)
                    {
                        float speed = 2f * (deltaTime / 1000f);
                        
                        float sinYaw = (float)Math.Sin(_cameraYaw);
                        float cosYaw = (float)Math.Cos(_cameraYaw);
                        float sinPitch = (float)Math.Sin(_cameraPitch);
                        float cosPitch = (float)Math.Cos(_cameraPitch);
                        // forward包含pitch分量: 朝相机真正指向的方向前进
                        float forwardX = sinYaw * cosPitch;
                        float forwardY = sinPitch;
                        float forwardZ = -cosYaw * cosPitch;
                        // right保持水平(不含pitch): 左右平移
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
                        
                        UI.SetCameraParams(_cameraYaw, _cameraPitch, _cameraX, _cameraY, _cameraZ);
                        
                        _gameScene.Update(deltaTime / 1000f);
                    }
                    else if (_gameScene != null && _gameScene.GetGameState() == GameState.Paused)
                    {
                        // Paused: keep camera params but don't update scene
                        UI.SetCameraParams(_cameraYaw, _cameraPitch, _cameraX, _cameraY, _cameraZ);
                    }
                    else
                    {
                        // Editing mode: orbit camera
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