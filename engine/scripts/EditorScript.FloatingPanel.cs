using System;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static ulong _floatingPanelId;
        private static bool _floatingDragging;
        private static float _floatingDragStartX;
        private static float _floatingDragStartY;
        private static float _floatingPanelStartX;
        private static float _floatingPanelStartY;

        public static bool IsFloatingPanelOpen { get { return _floatingPanelId != 0; } }

        public static void ToggleFloatingAssetPanel()
        {
            if (_floatingPanelId != 0) { HideFloatingAssetPanel(); return; }
            ShowFloatingAssetPanel();
        }

        public static void ShowFloatingAssetPanel()
        {
            if (_floatingPanelId != 0) return;
            ulong root = UI.GetRootId();
            _floatingPanelId = UI.CreateDialog(root, "浮动资产面板", 560f, 360f);

            ulong content = UI.CreateVStack(_floatingPanelId, 8f);
            UI.CreateLabel(content, 500f, 24f, "资产（浮动窗口）:");
            ulong grid = UI.CreateGridView(content, 0f, 0f, 520f, 220f, 64);
            string[] names = new string[] { "Cube", "Sphere", "Plane", "Cylinder", "CornellBox" };
            ulong[] values = new ulong[] { 1, 2, 3, 4, 8 };
            for (int i = 0; i < names.Length; i++)
            {
                int index = (int)UI.GridViewAddItem(grid, names[i], values[i]);
                UI.GridViewSetItemThumbnail(grid, index, "icon:" + (int)(i == 0 ? IconId.Cube : i == 1 ? IconId.Sun : i == 2 ? IconId.Plane : i == 3 ? IconId.Cylinder : IconId.Grid));
            }

            UI.DialogSetContent(_floatingPanelId, content);
            UI.DialogAddButton(_floatingPanelId, "关闭", 0);
            UI.DialogShow(_floatingPanelId);
        }

        public static void HideFloatingAssetPanel()
        {
            if (_floatingPanelId == 0) return;
            UI.RemoveWidget(_floatingPanelId);
            _floatingPanelId = 0;
        }

        private static bool PointInFloatingPanel(float x, float y)
        {
            if (_floatingPanelId == 0) return false;
            float[] layout = UI.WidgetGetAbsoluteLayout(_floatingPanelId);
            if (layout == null || layout.Length < 4) return false;
            return x >= layout[0] && x <= layout[0] + layout[2]
                && y >= layout[1] && y <= layout[1] + layout[3];
        }

        private static void BeginFloatingDrag(float x, float y)
        {
            float[] layout = UI.WidgetGetAbsoluteLayout(_floatingPanelId);
            if (layout == null || layout.Length < 4) return;
            _floatingDragging = true;
            _floatingDragStartX = x;
            _floatingDragStartY = y;
            _floatingPanelStartX = layout[0];
            _floatingPanelStartY = layout[1];
        }

        private static void UpdateFloatingDrag(float x, float y)
        {
            if (!_floatingDragging) return;
            UI.SetWidgetLayout(_floatingPanelId,
                _floatingPanelStartX + x - _floatingDragStartX,
                _floatingPanelStartY + y - _floatingDragStartY, 0f, 0f);
        }

        private static void EndFloatingDrag() { _floatingDragging = false; }

        public static void TestMoveFloatingPanel(float dx, float dy)
        {
            if (_floatingPanelId == 0) return;
            float[] layout = UI.WidgetGetAbsoluteLayout(_floatingPanelId);
            if (layout == null || layout.Length < 4) return;
            UI.SetWidgetLayout(_floatingPanelId, layout[0] + dx, layout[1] + dy, 0f, 0f);
        }

        public static void TestShowFloatingPanel() { ShowFloatingAssetPanel(); }
        public static void TestHideFloatingPanel() { HideFloatingAssetPanel(); }
    }
}
