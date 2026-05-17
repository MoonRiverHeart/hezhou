using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static class UI
    {
        private static IntPtr _widgetTree;
        private static FfiContext _ffi;

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong GetButtonIdDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetOnClickDelegate(IntPtr handle, ulong widgetId, IntPtr callback);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetTextDelegate(IntPtr handle, ulong widgetId, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateButtonDelegate(IntPtr handle, float x, float y, float width, float height, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateLabelDelegate(IntPtr handle, float x, float y, float width, float height, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreatePanelDelegate(IntPtr handle, float x, float y, float width, float height);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPositionDelegate(IntPtr handle, ulong widgetId, float x, float y);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetSizeDelegate(IntPtr handle, ulong widgetId, float width, float height);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterResizeDelegate(IntPtr callbackPtr);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GetScreenSizeDelegate(out float width, out float height);

        [StructLayout(LayoutKind.Sequential)]
        public struct FfiContext
        {
            public IntPtr ui_get_primary_button_id;
            public IntPtr ui_set_primary_button_id;
            public IntPtr ui_widget_set_text;
            public IntPtr ui_button_set_on_click_thunk_ptr;
            public IntPtr ui_register_update_thunk_ptr;
            public IntPtr ui_register_resize_thunk_ptr;
            public IntPtr ui_trigger_resize;
            public IntPtr ui_get_screen_size;
            public IntPtr ui_create_button;
            public IntPtr ui_create_label;
            public IntPtr ui_create_panel;
            public IntPtr ui_widget_set_position;
            public IntPtr ui_widget_set_size;
            public IntPtr widget_tree_ptr;
        }

        public static void InitFromContext(IntPtr contextPtr)
        {
            _ffi = Marshal.PtrToStructure<FfiContext>(contextPtr);
            _widgetTree = _ffi.widget_tree_ptr;
            Console.WriteLine("[C#] FfiContext初始化成功");
        }

        public static void GetScreenSize(out float width, out float height)
        {
            if (_ffi.ui_get_screen_size == IntPtr.Zero)
            {
                width = 800f;
                height = 600f;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetScreenSizeDelegate>(_ffi.ui_get_screen_size);
            func(out width, out height);
        }

        public static void RegisterResizeCallback(ResizeCallbackDelegate callback)
        {
            if (_ffi.ui_register_resize_thunk_ptr == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: RegisterResize函数指针为空");
                return;
            }
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<RegisterResizeDelegate>(_ffi.ui_register_resize_thunk_ptr);
            func(callbackPtr);
            Console.WriteLine("[C#] Resize回调已注册");
        }

        public static ulong GetPrimaryButtonId()
        {
            if (_ffi.ui_get_primary_button_id == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: 函数指针为空");
                return 0;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<GetButtonIdDelegate>(_ffi.ui_get_primary_button_id);
            return func();
        }

        public static void SetOnClick(ulong widgetId, WidgetCallbackDelegate callback)
        {
            if (_ffi.ui_button_set_on_click_thunk_ptr == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: SetOnClick函数指针为空");
                return;
            }
            
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<SetOnClickDelegate>(_ffi.ui_button_set_on_click_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }

        public static void SetText(ulong widgetId, string text)
        {
            if (_ffi.ui_widget_set_text == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: SetText函数指针为空");
                return;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<SetTextDelegate>(_ffi.ui_widget_set_text);
            func(_widgetTree, widgetId, text);
        }

        public static ulong CreateButton(float x, float y, float width, float height, string text)
        {
            if (_ffi.ui_create_button == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateButton函数指针为空");
                return 0;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<CreateButtonDelegate>(_ffi.ui_create_button);
            return func(_widgetTree, x, y, width, height, text);
        }

        public static ulong CreateLabel(float x, float y, float width, float height, string text)
        {
            if (_ffi.ui_create_label == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateLabel函数指针为空");
                return 0;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<CreateLabelDelegate>(_ffi.ui_create_label);
            return func(_widgetTree, x, y, width, height, text);
        }

        public static ulong CreatePanel(float x, float y, float width, float height)
        {
            if (_ffi.ui_create_panel == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreatePanel函数指针为空");
                return 0;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<CreatePanelDelegate>(_ffi.ui_create_panel);
            return func(_widgetTree, x, y, width, height);
        }

        public static void SetPosition(ulong widgetId, float x, float y)
        {
            if (_ffi.ui_widget_set_position == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: SetPosition函数指针为空");
                return;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<SetPositionDelegate>(_ffi.ui_widget_set_position);
            func(_widgetTree, widgetId, x, y);
        }

        public static void SetSize(ulong widgetId, float width, float height)
        {
            if (_ffi.ui_widget_set_size == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: SetSize函数指针为空");
                return;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<SetSizeDelegate>(_ffi.ui_widget_set_size);
            func(_widgetTree, widgetId, width, height);
        }
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate(ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ResizeCallbackDelegate(float width, float height);
    }

    public abstract class Widget
    {
        protected ulong _widgetId;
        protected bool _created;
        protected float _x, _y, _width, _height;

        public ulong Id => _widgetId;
        public bool IsCreated => _created;

        public float X
        {
            get => _x;
            set { _x = value; if (_created) UI.SetPosition(_widgetId, _x, _y); }
        }

        public float Y
        {
            get => _y;
            set { _y = value; if (_created) UI.SetPosition(_widgetId, _x, _y); }
        }

        public float Width
        {
            get => _width;
            set { _width = value; if (_created) UI.SetSize(_widgetId, _width, _height); }
        }

        public float Height
        {
            get => _height;
            set { _height = value; if (_created) UI.SetSize(_widgetId, _width, _height); }
        }

        protected abstract void Create();

        public void EnsureCreated()
        {
            if (!_created)
            {
                Create();
                _created = true;
            }
        }
    }

    public class Button : Widget
    {
        private string _text;
        private UI.WidgetCallbackDelegate _clickCallback;

        public string Text
        {
            get => _text;
            set { _text = value; if (_created) UI.SetText(_widgetId, _text); }
        }

        public Button(string text, float x = 0, float y = 0, float width = 200, float height = 50)
        {
            _text = text;
            _x = x;
            _y = y;
            _width = width;
            _height = height;
        }

        protected override void Create()
        {
            _widgetId = UI.CreateButton(_x, _y, _width, _height, _text);
            Console.WriteLine($"[C#] Button创建成功: id={_widgetId}, text=\"{_text}\"");
            
            if (_clickCallback != null)
            {
                UI.SetOnClick(_widgetId, _clickCallback);
            }
        }

        public void SetOnClick(UI.WidgetCallbackDelegate callback)
        {
            _clickCallback = callback;
            if (_created)
            {
                UI.SetOnClick(_widgetId, _clickCallback);
            }
        }

        public void Click()
        {
            if (_clickCallback != null)
            {
                _clickCallback(_widgetId);
            }
        }
    }

    public class Label : Widget
    {
        private string _text;

        public string Text
        {
            get => _text;
            set { _text = value; if (_created) UI.SetText(_widgetId, _text); }
        }

        public Label(string text, float x = 0, float y = 0, float width = 200, float height = 30)
        {
            _text = text;
            _x = x;
            _y = y;
            _width = width;
            _height = height;
        }

        protected override void Create()
        {
            _widgetId = UI.CreateLabel(_x, _y, _width, _height, _text);
            Console.WriteLine($"[C#] Label创建成功: id={_widgetId}, text=\"{_text}\"");
        }
    }

    public class Panel : Widget
    {
        public Panel(float x = 0, float y = 0, float width = 400, float height = 400)
        {
            _x = x;
            _y = y;
            _width = width;
            _height = height;
        }

        protected override void Create()
        {
            _widgetId = UI.CreatePanel(_x, _y, _width, _height);
            Console.WriteLine($"[C#] Panel创建成功: id={_widgetId}");
        }
    }
}