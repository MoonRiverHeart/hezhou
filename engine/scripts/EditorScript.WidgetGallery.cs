using System;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static ulong _galleryPanelId;
        private static UI.WidgetCallbackDelegate _galleryCloseCallback;

        private static void OnGalleryClose(ulong widgetId)
        {
            HideWidgetGallery();
        }

        private static void GallerySection(ulong parent, string text)
        {
            ulong label = UI.CreateLabel(parent, 860f, 24f, text);
            UI.ThemeSetText(label, ThemeRole.TextSecondary);
        }

        private static void GalleryButton(ulong parent, string text, ThemeRole bg, ThemeRole fg)
        {
            ulong button = UI.CreateButton(parent, 104f, 30f, text);
            UI.ThemeSetBackground(button, bg);
            UI.ThemeSetText(button, fg);
        }

        private static void GallerySwatch(ulong parent, ThemeRole role, string name)
        {
            ulong item = UI.CreateHStack(parent, 6f);
            UI.CreatePanel(item, 0f, 0f, 24f, 24f, role);
            UI.CreateLabel(item, 110f, 22f, name);
        }

        /// <summary>P0 控件画廊：整页展示，用于双主题视觉检查。</summary>
        public static void ShowWidgetGallery()
        {
            if (_galleryPanelId != 0) return;
            if (_galleryCloseCallback == null) _galleryCloseCallback = OnGalleryClose;

            if (_outerSplitViewId != 0) UI.SetWidgetVisible(_outerSplitViewId, false);
            if (_statusBar != null) UI.SetWidgetVisible(_statusBar.Id, false);

            ulong root = UI.GetRootId();
            _galleryPanelId = UI.CreatePanel(root, 0f, TOOLBAR_HEIGHT, _screenWidth,
                _screenHeight - TOOLBAR_HEIGHT, ThemeRole.Canvas);

            ulong close = UI.CreateButton(_galleryPanelId, 88f, 30f, "关闭");
            UI.SetWidgetLayout(close, _screenWidth - 120f, 18f, 88f, 30f);
            UI.SetOnClick(close, _galleryCloseCallback);
            UI.ThemeSetBackground(close, ThemeRole.ButtonSecondaryBg);
            UI.ThemeSetText(close, ThemeRole.TextPrimary);

            ulong content = UI.CreateVStack(_galleryPanelId, 10f);
            UI.SetWidgetLayout(content, 28f, 18f, 900f, _screenHeight - TOOLBAR_HEIGHT - 36f);

            UI.CreateLabel(content, 860f, 30f,
                "Widget Gallery  ·  " + (Theme.IsDark ? "Dark" : "Light"));
            GallerySection(content, "Buttons");

            ulong buttons = UI.CreateHStack(content, 10f);
            GalleryButton(buttons, "Primary", ThemeRole.ButtonPrimaryBg, ThemeRole.TextOnAccent);
            GalleryButton(buttons, "Accent", ThemeRole.Accent, ThemeRole.TextOnAccent);
            GalleryButton(buttons, "Danger", ThemeRole.Danger, ThemeRole.TextOnAccent);
            GalleryButton(buttons, "Success", ThemeRole.Success, ThemeRole.TextOnAccent);
            GalleryButton(buttons, "Warning", ThemeRole.Warning, ThemeRole.TextOnAccent);

            GallerySection(content, "Inputs");
            ulong inputs = UI.CreateHStack(content, 10f);
            ulong input = UI.CreateInputField(inputs, 220f, 28f);
            UI.InputFieldSetPlaceholder(input, "搜索资产...");
            UI.InputFieldSetText(input, "RotationScript");
            ulong dropdown = UI.CreateDropdown(inputs, 200f, 28f);
            UI.DropdownSetOptions(dropdown, new string[] { "Geometry", "Transform", "Render", "Motion", "Physics" });
            UI.DropdownSetSelected(dropdown, 3);
            UI.CreateSlider(inputs, 0f, 0f, 180f, 26f, 0f, 100f, 45f);
            UI.CreateCheckbox(inputs, 0f, 0f, 200f, 26f, "启用脚本实例");

            GallerySection(content, "Icons");
            ulong icons = UI.CreateHStack(content, 8f);
            for (uint i = 0; i < UI.IconCount(); i++)
            {
                ulong icon = UI.CreateIcon(icons, 0f, 0f, 18f, 18f, (IconId)i);
                UI.ThemeSetIconColor(icon, ThemeRole.TextSecondary);
            }

            GallerySection(content, "Tabs");
            ulong tabs = UI.CreateTabWidget(content, 0f, 0f, 820f, 170f);
            ulong tab1 = UI.CreateVStack(tabs, 6f);
            UI.CreateLabel(tab1, 760f, 22f, "几何属性示例");
            UI.CreateLabel(tab1, 760f, 22f, "Position / Rotation / Scale");
            ulong tab2 = UI.CreateVStack(tabs, 6f);
            UI.CreateLabel(tab2, 760f, 22f, "运动属性示例");
            UI.CreateLabel(tab2, 760f, 22f, "Rotation Speed / Initial Angle");
            UI.CreateLabel(tab2, 760f, 22f, "Script Binding");
            UI.TabWidgetAddTab(tabs, "几何", tab1, false);
            UI.TabWidgetAddTab(tabs, "运动", tab2, false);
            UI.TabWidgetSetActive(tabs, 1);

            GallerySection(content, "Theme Colors");
            ulong colors = UI.CreateHStack(content, 16f);
            GallerySwatch(colors, ThemeRole.Border, "Border");
            GallerySwatch(colors, ThemeRole.SelectionBg, "Selection");
            GallerySwatch(colors, ThemeRole.Accent, "Accent");
            GallerySwatch(colors, ThemeRole.Success, "Success");
        }

        public static void HideWidgetGallery()
        {
            if (_galleryPanelId != 0)
            {
                UI.RemoveWidget(_galleryPanelId);
                _galleryPanelId = 0;
            }
            if (_outerSplitViewId != 0) UI.SetWidgetVisible(_outerSplitViewId, true);
            if (_statusBar != null) UI.SetWidgetVisible(_statusBar.Id, true);
        }

        public static void TestShowWidgetGallery()
        {
            ShowWidgetGallery();
        }

        public static void TestHideWidgetGallery()
        {
            HideWidgetGallery();
        }
    }
}
