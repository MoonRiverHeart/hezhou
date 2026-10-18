using System;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static ulong _gizmoPanelId;
        private static ulong _gizmoXBarId;
        private static ulong _gizmoYBarId;
        private static ulong _gizmoZBarId;
        private static int _gizmoAxis; // 0=none, 1=X, 2=Y, 3=Z
        private static int _gizmoMode; // 0=translate, 1=scale, 2=rotate
        private static float _gizmoStartScaleX, _gizmoStartScaleY, _gizmoStartScaleZ;
        private static float _gizmoLastX;
        private static float _gizmoDragStartX;
        private static float _gizmoDragStartY;
        private static float _gizmoStartPosX;
        private static float _gizmoStartPosY;
        private static float _gizmoStartPosZ;

        private static void CreateGizmoOverlay()
        {
            ulong root = UI.GetRootId();
            _gizmoXBarId = UI.CreateButton(root, 70f, 24f, "X");
            _gizmoYBarId = UI.CreateButton(root, 24f, 70f, "Y");
            _gizmoZBarId = UI.CreateButton(root, 60f, 24f, "Z");
            UI.SetWidgetBackgroundColor(_gizmoXBarId, 0.95f, 0.30f, 0.30f, 1f);
            UI.SetWidgetBackgroundColor(_gizmoYBarId, 0.35f, 0.90f, 0.45f, 1f);
            UI.SetWidgetBackgroundColor(_gizmoZBarId, 0.35f, 0.60f, 1.0f, 1f);
            _gizmoPanelId = _gizmoXBarId; // marker: overlay exists
            UI.SetWidgetLayer(_gizmoXBarId, 3);
            UI.SetWidgetLayer(_gizmoYBarId, 3);
            UI.SetWidgetLayer(_gizmoZBarId, 3);
            UpdateGizmoLayout();
        }

        private static bool ProjectSelectedEntityToPreview(out float sx, out float sy)
        {
            sx = 0f;
            sy = 0f;
            if (_gameScene == null || _selectedEntityId == 0) return false;
            UI.SceneGetEntityPosition(_gameScene.ScenePtr, _selectedEntityId,
                out float ex, out float ey, out float ez);

            float leftWidth = _screenWidth * _dockOuterRatio;
            float previewW = leftWidth * (1f - _dockInnerRatio) - 8f;
            float previewH = _screenHeight * _dockLeftCenterRatio - 15f;
            float previewX = leftWidth * _dockInnerRatio + 4f;
            float previewY = 80f;
            if (previewW <= 10f || previewH <= 10f) return false;

            float cyaw = (float)Math.Cos(_cameraYaw);
            float syaw = (float)Math.Sin(_cameraYaw);
            float cp = (float)Math.Cos(_cameraPitch);
            float sp = (float)Math.Sin(_cameraPitch);
            float fx = syaw * cp, fy = -sp, fz = -cyaw * cp;
            float rx = cyaw, ry = 0f, rz = syaw;
            float ux = syaw * sp, uy = cp, uz = -cyaw * sp;

            float dx = ex - _cameraX;
            float dy = ey - _cameraY;
            float dz = ez - _cameraZ;
            float viewX = rx * dx + ry * dy + rz * dz;
            float viewY = ux * dx + uy * dy + uz * dz;
            float viewZ = -(fx * dx + fy * dy + fz * dz);
            if (viewZ <= 0.1f) return false;

            float f = 1f / (float)Math.Tan(60.0 * Math.PI / 360.0);
            float aspect = previewW / previewH;
            float ndcX = (f / aspect) * viewX / viewZ;
            float ndcY = (-f) * viewY / viewZ;
            sx = previewX + (ndcX * 0.5f + 0.5f) * previewW;
            sy = previewY + (ndcY * 0.5f + 0.5f) * previewH;
            return true;
        }

        private static void UpdateGizmoLayout()
        {
            if (_gizmoPanelId == 0) return;
            bool visible = _selectedEntityId != 0 && _previewWindowId != 0;
            UI.SetWidgetVisible(_gizmoXBarId, visible);
            UI.SetWidgetVisible(_gizmoYBarId, visible);
            UI.SetWidgetVisible(_gizmoZBarId, visible);
            if (!visible) return;

            float[] preview = UI.WidgetGetAbsoluteLayout(_previewWindowId);
            if (preview == null || preview.Length < 4 || preview[2] <= 0f) return;
            float cx = _screenWidth * 0.5f;
            float cy = _screenHeight * 0.42f;
            UI.SetWidgetLayout(_gizmoXBarId, cx - 35f, cy - 12f, 70f, 24f);
            UI.SetWidgetLayout(_gizmoYBarId, cx - 12f, cy - 70f, 24f, 70f);
            UI.SetWidgetLayout(_gizmoZBarId, cx + 8f, cy + 8f, 60f, 24f);
        }

        private static int HitTestGizmo(float x, float y)
        {
            if (_selectedEntityId == 0) return 0;
            if (PointInWidget(_gizmoXBarId, x, y)) return 1;
            if (PointInWidget(_gizmoYBarId, x, y)) return 2;
            if (PointInWidget(_gizmoZBarId, x, y)) return 3;
            return 0;
        }

        private static bool PointInWidget(ulong widgetId, float x, float y)
        {
            float[] layout = UI.WidgetGetAbsoluteLayout(widgetId);
            if (layout == null || layout.Length < 4) return false;
            return x >= layout[0] && x <= layout[0] + layout[2]
                && y >= layout[1] && y <= layout[1] + layout[3];
        }

        private static void BeginGizmoDrag(int axis, float x, float y)
        {
            if (_gameScene == null || _selectedEntityId == 0) return;
            _gizmoAxis = axis;
            _gizmoDragStartX = x;
            _gizmoDragStartY = y;
            UI.SceneGetEntityPosition(_gameScene.ScenePtr, _selectedEntityId,
                out _gizmoStartPosX, out _gizmoStartPosY, out _gizmoStartPosZ);
            UI.SceneGetEntityScale(_gameScene.ScenePtr, _selectedEntityId,
                out _gizmoStartScaleX, out _gizmoStartScaleY, out _gizmoStartScaleZ);
            _gizmoLastX = x;
            CaptureUndoSnapshot();
        }

        private static void UpdateGizmoDrag(float x, float y)
        {
            if (_gizmoAxis == 0 || _gameScene == null || _selectedEntityId == 0) return;
            float dx = (x - _gizmoDragStartX) * 0.02f;
            float dy = (y - _gizmoDragStartY) * 0.02f;
            float px = _gizmoStartPosX;
            float py = _gizmoStartPosY;
            if (_gizmoAxis == 1) px += dx;
            else if (_gizmoAxis == 2) py -= dy;
            // Z uses horizontal delta as a simple depth proxy.
            else if (_gizmoAxis == 3) px += dx;
            if (_gizmoMode == 1)
            {
                float factor = Math.Max(0.05f, 1f + dx);
                float sx = _gizmoAxis == 1 ? _gizmoStartScaleX * factor : _gizmoStartScaleX;
                float sy = _gizmoAxis == 2 ? _gizmoStartScaleY * factor : _gizmoStartScaleY;
                float sz = _gizmoAxis == 3 ? _gizmoStartScaleZ * factor : _gizmoStartScaleZ;
                UI.SceneSetEntityScale(_gameScene.ScenePtr, _selectedEntityId, sx, sy, sz);
                if (_statusItem != null) _statusItem.Text = "Gizmo 缩放: " + factor.ToString("F2");
            }
            else if (_gizmoMode == 2)
            {
                float deltaX = x - _gizmoLastX;
                _gizmoLastX = x;
                UI.SceneRotateEntity(_gameScene.ScenePtr, _selectedEntityId, deltaX * 0.5f);
                if (_statusItem != null) _statusItem.Text = "Gizmo 旋转";
            }
            else
            {
                UI.SceneSetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, px, py, _gizmoStartPosZ);
                if (_statusItem != null) _statusItem.Text = "Gizmo: X=" + px.ToString("F2") + " Y=" + py.ToString("F2");
            }
            _propertiesDirty = true;
        }

        private static void EndGizmoDrag()
        {
            _gizmoAxis = 0;
        }

        public static void TestGizmoCycleMode()
        {
            _gizmoMode = (_gizmoMode + 1) % 3;
            if (_statusItem != null)
            {
                string[] names = new string[] { "平移", "缩放", "旋转" };
                _statusItem.Text = "Gizmo 模式: " + names[_gizmoMode];
            }
        }

        public static int TestGetGizmoMode() { return _gizmoMode; }

        public static void TestGizmoNudge(float dx, float dy)
        {
            if (_gameScene == null || _selectedEntityId == 0) return;
            UI.SceneGetEntityPosition(_gameScene.ScenePtr, _selectedEntityId,
                out float x, out float y, out float z);
            UI.SceneSetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, x + dx, y + dy, z);
            _propertiesDirty = true;
        }

        public static float[] TestGetScale(ulong entityId)
        {
            float x = 1, y = 1, z = 1;
            if (_gameScene != null && entityId != 0)
                UI.SceneGetEntityScale(_gameScene.ScenePtr, entityId, out x, out y, out z);
            return new float[] { x, y, z };
        }

        public static void TestGizmoScale(float factor)
        {
            if (_gameScene == null || _selectedEntityId == 0) return;
            UI.SceneGetEntityScale(_gameScene.ScenePtr, _selectedEntityId,
                out float sx, out float sy, out float sz);
            UI.SceneSetEntityScale(_gameScene.ScenePtr, _selectedEntityId, sx * factor, sy * factor, sz * factor);
            _propertiesDirty = true;
        }

        public static bool TestGizmoVisible()
        {
            if (_gizmoPanelId == 0) return false;
            return UI.WidgetGetAbsoluteLayout(_gizmoXBarId) != null;
        }
    }
}
