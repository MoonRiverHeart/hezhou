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
             
_addScriptClickCallback = OnAddScriptClick;
            _removeScriptClickCallback = OnRemoveScriptClick;
            _createEntityClickCallback = OnCreateEntityClick;
            _pauseClickCallback = OnPauseClick;
            _scriptToggleClickCallback = OnScriptToggleClick;
            _scriptDropdownSelectCallback = OnScriptDropdownSelect;
_hotReloadCompleteCallback = OnHotReloadComplete;
            _tabSelectCallback = OnTabSelect;
            _treeNodeToggleCallback = OnTreeNodeToggle;
            
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
                        float forwardX = sinYaw;
                        float forwardZ = -cosYaw;
                        float rightX = cosYaw;
                        float rightZ = sinYaw;
                        
                        if (_keyUpPressed)
                        {
                            _cameraX += speed * forwardX;
                            _cameraZ += speed * forwardZ;
                        }
                        if (_keyDownPressed)
                        {
                            _cameraX -= speed * forwardX;
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
                        float camX = _orbitTargetX + _orbitDistance * (float)Math.Sin(_orbitYaw) * (float)Math.Cos(_orbitPitch);
                        float camY = _orbitTargetY + _orbitDistance * (float)Math.Sin(_orbitPitch);
                        float camZ = _orbitTargetZ + _orbitDistance * (float)Math.Cos(_orbitYaw) * (float)Math.Cos(_orbitPitch);
                        UI.SetCameraParams(_orbitYaw, -_orbitPitch, camX, camY, camZ);
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