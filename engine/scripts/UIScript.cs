using System;
using System.Runtime.InteropServices;

namespace HezhouUI
{
    public static class NativeUI
    {
        private const string LibraryName = "hezhou_ui";
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr ui_system_create();
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_system_destroy(IntPtr system);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_system_update(IntPtr system, float delta_time);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr ui_system_get_widget_tree(IntPtr system);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr ui_system_get_event_dispatcher(IntPtr system);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_widget_tree_handle_destroy(IntPtr handle);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_event_dispatcher_handle_destroy(IntPtr handle);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong ui_widget_tree_create_root_panel(
            IntPtr handle, float x, float y, float width, float height);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong ui_widget_tree_add_button(
            IntPtr handle, ulong parent_id, float x, float y, float width, float height,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string text);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong ui_widget_tree_add_label(
            IntPtr handle, ulong parent_id, float x, float y, float width, float height,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string text);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong ui_widget_tree_add_panel(
            IntPtr handle, ulong parent_id, float x, float y, float width, float height);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_widget_set_layout(
            IntPtr handle, ulong widget_id, float x, float y, float width, float height);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_widget_set_background_color(
            IntPtr handle, ulong widget_id, float r, float g, float b, float a);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_event_dispatcher_dispatch_touch_begin(
            IntPtr handle, float x, float y, uint pointer_id, ulong timestamp);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_event_dispatcher_dispatch_touch_end(
            IntPtr handle, float x, float y, uint pointer_id, ulong timestamp);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_event_dispatcher_dispatch_key_down(
            IntPtr handle, uint keycode, uint modifiers, ulong timestamp);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_event_dispatcher_dispatch_key_up(
            IntPtr handle, uint keycode, uint modifiers, ulong timestamp);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_widget_set_text(
            IntPtr handle, ulong widget_id, [MarshalAs(UnmanagedType.LPUTF8Str)] string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ClickCallbackDelegate(ulong widgetId);
        
        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void ui_button_set_on_click(
            IntPtr handle, ulong widget_id, ClickCallbackDelegate callback);
    }
    
    public class UISystem : IDisposable
    {
        private IntPtr _handle;
        private IntPtr _widgetTreeHandle;
        private IntPtr _eventDispatcherHandle;
        private bool _disposed;
        
        public UISystem()
        {
            _handle = NativeUI.ui_system_create();
            if (_handle == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to create UI system");
            }
            
            _widgetTreeHandle = NativeUI.ui_system_get_widget_tree(_handle);
            _eventDispatcherHandle = NativeUI.ui_system_get_event_dispatcher(_handle);
        }
        
        public void Update(float deltaTime)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_system_update(_handle, deltaTime);
        }
        
        public ulong CreateRootPanel(float x, float y, float width, float height)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            return NativeUI.ui_widget_tree_create_root_panel(_widgetTreeHandle, x, y, width, height);
        }
        
        public ulong AddButton(ulong parentId, float x, float y, float width, float height, string text)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            return NativeUI.ui_widget_tree_add_button(_widgetTreeHandle, parentId, x, y, width, height, text);
        }
        
        public ulong AddLabel(ulong parentId, float x, float y, float width, float height, string text)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            return NativeUI.ui_widget_tree_add_label(_widgetTreeHandle, parentId, x, y, width, height, text);
        }
        
        public ulong AddPanel(ulong parentId, float x, float y, float width, float height)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            return NativeUI.ui_widget_tree_add_panel(_widgetTreeHandle, parentId, x, y, width, height);
        }
        
        public void SetWidgetLayout(ulong widgetId, float x, float y, float width, float height)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_widget_set_layout(_widgetTreeHandle, widgetId, x, y, width, height);
        }
        
        public void SetWidgetBackgroundColor(ulong widgetId, float r, float g, float b, float a = 1.0f)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_widget_set_background_color(_widgetTreeHandle, widgetId, r, g, b, a);
        }
        
        public void DispatchTouchBegin(float x, float y, uint pointerId, ulong timestamp)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_event_dispatcher_dispatch_touch_begin(_eventDispatcherHandle, x, y, pointerId, timestamp);
        }
        
        public void DispatchTouchEnd(float x, float y, uint pointerId, ulong timestamp)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_event_dispatcher_dispatch_touch_end(_eventDispatcherHandle, x, y, pointerId, timestamp);
        }
        
        public void DispatchKeyDown(uint keycode, uint modifiers, ulong timestamp)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_event_dispatcher_dispatch_key_down(_eventDispatcherHandle, keycode, modifiers, timestamp);
        }
        
        public void DispatchKeyUp(uint keycode, uint modifiers, ulong timestamp)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_event_dispatcher_dispatch_key_up(_eventDispatcherHandle, keycode, modifiers, timestamp);
        }
        
        public void SetWidgetText(ulong widgetId, string text)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_widget_set_text(_widgetTreeHandle, widgetId, text);
        }
        
        public void SetButtonOnClick(ulong widgetId, NativeUI.ClickCallbackDelegate callback)
        {
            if (_disposed) throw new ObjectDisposedException(nameof(UISystem));
            NativeUI.ui_button_set_on_click(_widgetTreeHandle, widgetId, callback);
        }
        
        public void Dispose()
        {
            if (!_disposed)
            {
                if (_widgetTreeHandle != IntPtr.Zero)
                {
                    NativeUI.ui_widget_tree_handle_destroy(_widgetTreeHandle);
                }
                
                if (_eventDispatcherHandle != IntPtr.Zero)
                {
                    NativeUI.ui_event_dispatcher_handle_destroy(_eventDispatcherHandle);
                }
                
                if (_handle != IntPtr.Zero)
                {
                    NativeUI.ui_system_destroy(_handle);
                }
                
                _disposed = true;
            }
        }
    }
    
    public struct Color
    {
        public float R, G, B, A;
        
        public Color(float r, float g, float b, float a = 1.0f)
        {
            R = r; G = g; B = b; A = a;
        }
        
        public static Color White => new Color(1, 1, 1, 1);
        public static Color Black => new Color(0, 0, 0, 1);
        public static Color Red => new Color(1, 0, 0, 1);
        public static Color Green => new Color(0, 1, 0, 1);
        public static Color Blue => new Color(0, 0, 1, 1);
        public static Color Transparent => new Color(0, 0, 0, 0);
    }
    
    public struct Layout
    {
        public float X, Y, Width, Height;
        
        public Layout(float x, float y, float width, float height)
        {
            X = x; Y = y; Width = width; Height = height;
        }
    }
    
    public enum KeyCode
    {
        Unknown = 0,
        A = 1, B = 2, C = 3, D = 4, E = 5, F = 6, G = 7, H = 8, I = 9, J = 10,
        K = 11, L = 12, M = 13, N = 14, O = 15, P = 16, Q = 17, R = 18, S = 19,
        T = 20, U = 21, V = 22, W = 23, X = 24, Y = 25, Z = 26,
        Num0 = 27, Num1 = 28, Num2 = 29, Num3 = 30, Num4 = 31,
        Num5 = 32, Num6 = 33, Num7 = 34, Num8 = 35, Num9 = 36,
        Space = 37, Enter = 38, Escape = 39, Backspace = 40, Tab = 41,
        Shift = 42, Ctrl = 43, Alt = 44,
        Left = 45, Right = 46, Up = 47, Down = 48,
    }
    
    public struct KeyModifiers
    {
        public bool Shift;
        public bool Ctrl;
        public bool Alt;
        
        public uint ToFlags()
        {
            uint flags = 0;
            if (Shift) flags |= 1;
            if (Ctrl) flags |= 2;
            if (Alt) flags |= 4;
            return flags;
        }
    }
}