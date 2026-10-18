using System;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static ulong _floatingPanelId;

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

        public static void TestShowFloatingPanel() { ShowFloatingAssetPanel(); }
        public static void TestHideFloatingPanel() { HideFloatingAssetPanel(); }
    }
}
