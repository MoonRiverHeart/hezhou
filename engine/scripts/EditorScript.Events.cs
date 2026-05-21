using System;

namespace Hezhou
{
    public static partial class EditorScript
    {
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
    }
}