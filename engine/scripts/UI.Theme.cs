using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public enum ThemeMode : uint
    {
        Dark = 0,
        Light = 1
    }

    /// <summary>语义颜色角色。数值必须与 Rust ThemeRole 一致。</summary>
    public enum ThemeRole : uint
    {
        Canvas = 0,
        Panel = 1,
        Elevated = 2,
        Hover = 3,
        Active = 4,
        Border = 5,
        BorderStrong = 6,
        TextPrimary = 7,
        TextSecondary = 8,
        TextMuted = 9,
        TextDisabled = 10,
        Accent = 11,
        AccentHover = 12,
        AccentMuted = 13,
        Success = 14,
        SuccessHover = 15,
        Warning = 16,
        Danger = 17,
        DangerHover = 18,
        SelectionBg = 19,
        SelectionFg = 20,
        ButtonPrimaryBg = 21,
        ButtonPrimaryHover = 22,
        ButtonPrimaryPressed = 23,
        ButtonSecondaryBg = 24,
        ButtonSecondaryHover = 25,
        ButtonSecondaryPressed = 26,
        InputBg = 27,
        InputBorder = 28,
        InputFocus = 29,
        TabBg = 30,
        TabActiveBg = 31,
        TabActiveFg = 32,
        ScrollbarTrack = 33,
        ScrollbarThumb = 34,
        ScrollbarThumbHover = 35,
        ToolbarBg = 36,
        MenuBg = 37,
        MenuHover = 38,
        TooltipBg = 39,
        TooltipFg = 40,
        Overlay = 41,
        Shadow = 42,
        TextOnAccent = 43
    }

    public struct ThemeColor
    {
        public float R;
        public float G;
        public float B;
        public float A;

        public ThemeColor(float r, float g, float b, float a)
        {
            R = r;
            G = g;
            B = b;
            A = a;
        }
    }

    public static partial class UI
    {
        private static ThemeColor UnpackThemeColor(uint packed)
        {
            float a = (packed & 0xFF) / 255f;
            float b = ((packed >> 8) & 0xFF) / 255f;
            float g = ((packed >> 16) & 0xFF) / 255f;
            float r = ((packed >> 24) & 0xFF) / 255f;
            return new ThemeColor(r, g, b, a);
        }

        public static ThemeColor ThemeColorOf(ThemeRole role)
        {
            if (_ffi.ui_theme_color == IntPtr.Zero)
            {
                return new ThemeColor(0.5f, 0.5f, 0.5f, 1f);
            }
            var func = Marshal.GetDelegateForFunctionPointer<ThemeColorDelegate>(_ffi.ui_theme_color);
            return UnpackThemeColor(func((uint)role));
        }

        public static ThemeMode ThemeGetMode()
        {
            if (_ffi.ui_theme_get_mode == IntPtr.Zero) return ThemeMode.Dark;
            var func = Marshal.GetDelegateForFunctionPointer<ThemeGetModeDelegate>(_ffi.ui_theme_get_mode);
            return func() == 1 ? ThemeMode.Light : ThemeMode.Dark;
        }

        public static ThemeMode ThemeSetMode(ThemeMode mode)
        {
            if (_ffi.ui_theme_set_mode == IntPtr.Zero) return ThemeMode.Dark;
            var func = Marshal.GetDelegateForFunctionPointer<ThemeSetModeDelegate>(_ffi.ui_theme_set_mode);
            return func((uint)mode) == 1 ? ThemeMode.Light : ThemeMode.Dark;
        }

        public static void ThemeSetBackground(ulong widgetId, ThemeRole role)
        {
            ThemeColor color = ThemeColorOf(role);
            SetWidgetBackgroundColor(widgetId, color.R, color.G, color.B, color.A);
        }

        public static void ThemeSetText(ulong widgetId, ThemeRole role)
        {
            if (_ffi.ui_widget_set_text_color == IntPtr.Zero) return;
            ThemeColor color = ThemeColorOf(role);
            var func = Marshal.GetDelegateForFunctionPointer<SetWidgetTextColorDelegate>(_ffi.ui_widget_set_text_color);
            func(_widgetTree, widgetId, color.R, color.G, color.B, color.A);
        }

        public static uint ThemeRoleCount()
        {
            if (_ffi.ui_theme_role_count == IntPtr.Zero) return 44;
            var func = Marshal.GetDelegateForFunctionPointer<ThemeRoleCountDelegate>(_ffi.ui_theme_role_count);
            return func();
        }
    }

    /// <summary>C# 侧只写语义角色，不直接写 RGBA。</summary>
    public static class Theme
    {
        public static ThemeColor Color(ThemeRole role)
        {
            return UI.ThemeColorOf(role);
        }

        public static ThemeMode Mode
        {
            get { return UI.ThemeGetMode(); }
        }

        public static bool IsDark
        {
            get { return UI.ThemeGetMode() == ThemeMode.Dark; }
        }
    }
}
