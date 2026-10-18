using System;
using System.Globalization;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private const string DockLayoutPath = ".hezhou_layout";
        private static bool _projectCollapsed;
        private static bool _assetCollapsed;
        private static float _dockInnerRestore = 0.38f;
        private static float _dockLeftCenterRestore = 0.70f;

        public static void LoadDockLayout()
        {
            try
            {
                if (!System.IO.File.Exists(DockLayoutPath)) return;
                foreach (string raw in System.IO.File.ReadAllLines(DockLayoutPath))
                {
                    string line = raw.Trim();
                    int eq = line.IndexOf('=');
                    if (eq <= 0) continue;
                    string key = line.Substring(0, eq);
                    string value = line.Substring(eq + 1);
                    float f;
                    ulong u;
                    switch (key)
                    {
                        case "outer":
                            if (float.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out f)) _dockOuterRatio = f;
                            break;
                        case "leftCenter":
                            if (float.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out f)) _dockLeftCenterRatio = f;
                            break;
                        case "inner":
                            if (float.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out f)) _dockInnerRatio = f;
                            break;
                        case "propTab":
                            if (ulong.TryParse(value, out u)) _dockPropTab = u;
                            break;
                        case "assetTab":
                            if (ulong.TryParse(value, out u)) _dockAssetTab = u;
                            break;
                        case "rightMax":
                            _rightPanelMaximized = value == "1";
                            break;
                        case "projectCollapsed":
                            _projectCollapsed = value == "1";
                            break;
                        case "assetCollapsed":
                            _assetCollapsed = value == "1";
                            break;
                        case "textureDocked":
                            _textureDockedRight = value == "1";
                            break;
                    }
                }
            }
            catch (Exception) { }
        }

        public static void ApplyDockLayout()
        {
            if (_outerSplitViewId != 0)
            {
                if (_rightPanelMaximized)
                {
                    UI.SplitViewSetMinRatio(_outerSplitViewId, 0.0f);
                    UI.SplitViewSetMaxRatio(_outerSplitViewId, 1.0f);
                    UI.SplitViewSetSplitRatio(_outerSplitViewId, 0.01f);
                }
                else
                {
                    UI.SplitViewSetMinRatio(_outerSplitViewId, 0.4f);
                    UI.SplitViewSetMaxRatio(_outerSplitViewId, 0.9f);
                    UI.SplitViewSetSplitRatio(_outerSplitViewId, _dockOuterRatio);
                }
            }
            if (_leftCenterSplitViewId != 0)
            {
                UI.SplitViewSetSplitRatio(_leftCenterSplitViewId, _assetCollapsed ? 1.0f : _dockLeftCenterRatio);
            }
            if (_innerHorizontalSplitViewId != 0)
            {
                UI.SplitViewSetSplitRatio(_innerHorizontalSplitViewId, _projectCollapsed ? 1.0f : _dockInnerRatio);
            }
            if (_propsTabWidget != null && _dockPropTab < _propsTabWidget.GetTabCount())
            {
                _propsTabWidget.SetActive(_dockPropTab);
            }
            if (_assetTabWidget != null && _dockAssetTab < _assetTabWidget.GetTabCount())
            {
                _assetTabWidget.SetActive(_dockAssetTab);
            }
            if (_rightMaxButtonId != 0)
            {
                UI.SetText(_rightMaxButtonId, _rightPanelMaximized ? "◱" : "□");
            }
            UpdatePreviewExtent();
        }

        public static void SaveDockLayout()
        {
            try
            {
                if (_outerSplitViewId != 0 && !_rightPanelMaximized)
                {
                    _dockOuterRatio = UI.SplitViewGetSplitRatio(_outerSplitViewId);
                }
                if (_leftCenterSplitViewId != 0)
                {
                    _dockLeftCenterRatio = UI.SplitViewGetSplitRatio(_leftCenterSplitViewId);
                }
                if (_innerHorizontalSplitViewId != 0)
                {
                    _dockInnerRatio = UI.SplitViewGetSplitRatio(_innerHorizontalSplitViewId);
                }
                if (_propsTabWidget != null) _dockPropTab = _propsTabWidget.GetActive();
                if (_assetTabWidget != null) _dockAssetTab = _assetTabWidget.GetActive();

                System.IO.File.WriteAllText(DockLayoutPath,
                    "outer=" + _dockOuterRatio.ToString("R", CultureInfo.InvariantCulture) + "\n" +
                    "leftCenter=" + _dockLeftCenterRatio.ToString("R", CultureInfo.InvariantCulture) + "\n" +
                    "inner=" + _dockInnerRatio.ToString("R", CultureInfo.InvariantCulture) + "\n" +
                    "propTab=" + _dockPropTab + "\n" +
                    "assetTab=" + _dockAssetTab + "\n" +
                    "rightMax=" + (_rightPanelMaximized ? "1" : "0") + "\n" +
                    "projectCollapsed=" + (_projectCollapsed ? "1" : "0") + "\n" +
                    "assetCollapsed=" + (_assetCollapsed ? "1" : "0") + "\n" +
                    "textureDocked=" + (_textureDockedRight ? "1" : "0") + "\n");
            }
            catch (Exception) { }
        }

        private static void OnRightMaxClick(ulong widgetId)
        {
            if (!_rightPanelMaximized)
            {
                if (_outerSplitViewId != 0)
                {
                    _rightPanelRestoreRatio = UI.SplitViewGetSplitRatio(_outerSplitViewId);
                }
                _rightPanelMaximized = true;
                UI.SplitViewSetMinRatio(_outerSplitViewId, 0.0f);
                UI.SplitViewSetMaxRatio(_outerSplitViewId, 1.0f);
                UI.SplitViewSetSplitRatio(_outerSplitViewId, 0.01f);
            }
            else
            {
                _rightPanelMaximized = false;
                UI.SplitViewSetMinRatio(_outerSplitViewId, 0.4f);
                UI.SplitViewSetMaxRatio(_outerSplitViewId, 0.9f);
                UI.SplitViewSetSplitRatio(_outerSplitViewId, _rightPanelRestoreRatio);
            }
            if (_rightMaxButtonId != 0)
            {
                UI.SetText(_rightMaxButtonId, _rightPanelMaximized ? "◱" : "□");
            }
            UpdateRightMaxButtonPosition();
            UpdatePreviewExtent();
            SaveDockLayout();
        }

        public static void ToggleProjectPanel()
        {
            if (_projectCollapsed)
            {
                _projectCollapsed = false;
                _dockInnerRatio = Math.Max(0.15f, Math.Min(0.85f, _dockInnerRestore));
                UI.SplitViewSetSplitRatio(_innerHorizontalSplitViewId, _dockInnerRatio);
            }
            else
            {
                _dockInnerRestore = UI.SplitViewGetSplitRatio(_innerHorizontalSplitViewId);
                _projectCollapsed = true;
                UI.SplitViewSetSplitRatio(_innerHorizontalSplitViewId, 1.0f);
            }
            if (_statusItem != null) _statusItem.Text = _projectCollapsed ? "项目面板已收起" : "项目面板已展开";
            SaveDockLayout();
        }

        public static void ToggleAssetPanel()
        {
            if (_assetCollapsed)
            {
                _assetCollapsed = false;
                _dockLeftCenterRatio = Math.Max(0.15f, Math.Min(0.85f, _dockLeftCenterRestore));
                UI.SplitViewSetSplitRatio(_leftCenterSplitViewId, _dockLeftCenterRatio);
            }
            else
            {
                _dockLeftCenterRestore = UI.SplitViewGetSplitRatio(_leftCenterSplitViewId);
                _assetCollapsed = true;
                UI.SplitViewSetSplitRatio(_leftCenterSplitViewId, 1.0f);
            }
            if (_statusItem != null) _statusItem.Text = _assetCollapsed ? "资产面板已收起" : "资产面板已展开";
            SaveDockLayout();
        }

        private static void OnTabDragEnd(ulong widgetId, int index, float localX, float localY, string title)
        {
            if (title != "纹理") return;
            float[] layout = UI.WidgetGetAbsoluteLayout(widgetId);
            float screenX = (layout != null && layout.Length >= 4) ? layout[0] + localX : localX;
            float screenY = (layout != null && layout.Length >= 4) ? layout[1] + localY : localY;

            if (!_textureDockedRight && widgetId == _assetTabWidget.Id
                && screenX > _screenWidth * 0.70f)
            {
                ToggleTextureDock();
            }
            else if (_textureDockedRight && widgetId == _propsTabWidget.Id
                && screenX < _screenWidth * 0.70f)
            {
                ToggleTextureDock();
            }
            else if (_statusItem != null)
            {
                _statusItem.Text = "拖拽 Tab: 未命中 Dock 区域";
            }
        }

        public static void TestDragTextureTabToRight()
        {
            float[] layout = UI.WidgetGetAbsoluteLayout(_assetTabWidget.Id);
            float x = _screenWidth * 0.85f - (layout != null && layout.Length >= 4 ? layout[0] : 0f);
            OnTabDragEnd(_assetTabWidget.Id, 2, x, 0f, "纹理");
        }

        public static void TestDragTextureTabBack()
        {
            float[] layout = UI.WidgetGetAbsoluteLayout(_propsTabWidget.Id);
            float x = _screenWidth * 0.2f - (layout != null && layout.Length >= 4 ? layout[0] : 0f);
            OnTabDragEnd(_propsTabWidget.Id, 5, x, 0f, "纹理");
        }

        public static void ToggleTextureDock()
        {
            // 纹理停靠不应继承右侧最大化状态，否则重建后右栏会占满全屏。
            _rightPanelMaximized = false;
            if (_dockOuterRatio < 0.55f) _dockOuterRatio = 0.72f;
            _textureDockedRight = !_textureDockedRight;
            SaveDockLayout();
            RebuildUIForTheme();
            if (_textureDockedRight && _propsTabWidget != null && _propsTabWidget.GetTabCount() > 5)
            {
                _propsTabWidget.SetActive(5);
            }
            if (_outerSplitViewId != 0)
            {
                _dockOuterRatio = 0.72f;
                UI.SplitViewSetMinRatio(_outerSplitViewId, 0.4f);
                UI.SplitViewSetMaxRatio(_outerSplitViewId, 0.9f);
                UI.SplitViewSetSplitRatio(_outerSplitViewId, _dockOuterRatio);
            }
            SaveDockLayout();
            if (_statusItem != null) _statusItem.Text = _textureDockedRight ? "纹理页已停靠右侧" : "纹理页已回到底部";
        }

        public static void TestToggleTextureDock() { ToggleTextureDock(); }
        public static bool TestIsTextureDockedRight() { return _textureDockedRight; }

        public static void TestToggleAssetPanel() { ToggleAssetPanel(); }
        public static void TestToggleProjectPanel() { ToggleProjectPanel(); }

        public static bool TestIsProjectCollapsed() { return _projectCollapsed; }
        public static bool TestIsAssetCollapsed() { return _assetCollapsed; }

        public static void ResetDockLayout()
        {
            _rightPanelMaximized = false;
            _projectCollapsed = false;
            _assetCollapsed = false;
            _textureDockedRight = false;
            _dockOuterRatio = 0.72f;
            _dockLeftCenterRatio = 0.70f;
            _dockInnerRatio = 0.38f;
            _dockPropTab = 0;
            _dockAssetTab = 0;
            ApplyDockLayout();
            SaveDockLayout();
        }

        private static void UpdateRightMaxButtonPosition()
        {
            if (_rightMaxButtonId == 0 || _outerSplitViewId == 0) return;
            float ratio = UI.SplitViewGetSplitRatio(_outerSplitViewId);
            float rightWidth = _screenWidth * (1.0f - ratio);
            if (rightWidth < 40f) rightWidth = _screenWidth;
            UI.SetWidgetLayout(_rightMaxButtonId, rightWidth - 34f, 8f, 26f, 22f);
        }

        public static void TestSaveDockLayout() { SaveDockLayout(); }
        public static void TestLoadDockLayout() { LoadDockLayout(); ApplyDockLayout(); }
        public static void TestResetDockLayout() { ResetDockLayout(); }
        public static void TestToggleRightPanelMax() { OnRightMaxClick(0); }
        public static bool TestIsRightPanelMaximized() { return _rightPanelMaximized; }
    }
}
