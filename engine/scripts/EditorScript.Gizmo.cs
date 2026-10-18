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
        private static float _gizmoDragStartX;
        private static float _gizmoDragStartY;
        private static float _gizmoStartPosX;
        private static float _gizmoStartPosY;
        private static float _gizmoStartPosZ;

        private static void CreateGizmoOverlay()
        {
            ulong root = UI.GetRootId();
            _gizmoXBarId = UI.CreatePanel(root, 0f, 0f, 70f, 6f, 0.95f, 0.30f, 0.30f, 1f);
            _gizmoYBarId = UI.CreatePanel(root, 0f, 0f, 6f, 70f, 0.35f, 0.90f, 0.45f, 1f);
            _gizmoZBarId = UI.CreatePanel(root, 0f, 0f, 60f, 6f, 0.35f, 0.60f, 1.0f, 1f);
            _gizmoPanelId = _gizmoXBarId; // marker: overlay exists
            UI.SetWidgetLayer(_gizmoXBarId, 3);
            UI.SetWidgetLayer(_gizmoYBarId, 3);
            UI.SetWidgetLayer(_gizmoZBarId, 3);
            UpdateGizmoLayout();
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
            float cx = preview[0] + preview[2] * 0.5f;
            float cy = preview[1] + preview[3] * 0.5f;
            UI.SetWidgetLayout(_gizmoXBarId, cx, cy - 3f, 70f, 6f);
            UI.SetWidgetLayout(_gizmoYBarId, cx - 3f, cy - 70f, 6f, 70f);
            UI.SetWidgetLayout(_gizmoZBarId, cx + 8f, cy + 8f, 60f, 6f);
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
            UI.SceneSetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, px, py, _gizmoStartPosZ);
            _propertiesDirty = true;
            if (_statusItem != null) _statusItem.Text = "Gizmo: X=" + px.ToString("F2") + " Y=" + py.ToString("F2");
        }

        private static void EndGizmoDrag()
        {
            _gizmoAxis = 0;
        }

        public static void TestGizmoNudge(float dx, float dy)
        {
            if (_gameScene == null || _selectedEntityId == 0) return;
            UI.SceneGetEntityPosition(_gameScene.ScenePtr, _selectedEntityId,
                out float x, out float y, out float z);
            UI.SceneSetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, x + dx, y + dy, z);
            _propertiesDirty = true;
        }

        public static bool TestGizmoVisible()
        {
            if (_gizmoPanelId == 0) return false;
            return UI.WidgetGetAbsoluteLayout(_gizmoXBarId) != null;
        }
    }
}
