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
        public delegate ulong CreateVStackDelegate(IntPtr handle, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateVStackInParentDelegate(IntPtr handle, ulong parentId, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateHStackDelegate(IntPtr handle, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateHStackInParentDelegate(IntPtr handle, ulong parentId, float spacing);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateButtonInParentDelegate(IntPtr handle, ulong parentId, float width, float height, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateLabelInParentDelegate(IntPtr handle, ulong parentId, float width, float height, [MarshalAs(UnmanagedType.LPStr)] string text);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreatePanelInParentDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height, float r, float g, float b, float a);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong GetRootIdDelegate(IntPtr handle);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetWidgetLayoutDelegate(IntPtr handle, ulong widgetId, float x, float y, float width, float height);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPositionDelegate(IntPtr handle, ulong widgetId, float x, float y);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetSizeDelegate(IntPtr handle, ulong widgetId, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RemoveWidgetDelegate(IntPtr handle, ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateTextEditDelegate(IntPtr handle, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateTextEditInParentDelegate(IntPtr handle, ulong parentId, float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditSetTextDelegate(IntPtr handle, ulong widgetId, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditInsertCharDelegate(IntPtr handle, ulong widgetId, byte c);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditDeleteCharDelegate(IntPtr handle, ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int TextEditGetTextLenDelegate(IntPtr handle, ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TextEditGetTextDelegate(IntPtr handle, ulong widgetId, IntPtr buffer, int bufferSize);
        
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
            public IntPtr ui_register_global_click_thunk_ptr;
            public IntPtr ui_trigger_resize;
            public IntPtr ui_get_screen_size;
            public IntPtr ui_create_button;
            public IntPtr ui_create_label;
            public IntPtr ui_create_panel;
            public IntPtr ui_create_vstack;
            public IntPtr ui_create_vstack_in_parent;
            public IntPtr ui_create_hstack;
            public IntPtr ui_create_hstack_in_parent;
            public IntPtr ui_create_button_in_parent;
            public IntPtr ui_create_label_in_parent;
            public IntPtr ui_create_panel_in_parent;
            public IntPtr ui_get_root_id;
            public IntPtr ui_set_widget_layout;
            public IntPtr ui_widget_set_position;
            public IntPtr ui_widget_set_size;
            public IntPtr ui_remove_widget;
            public IntPtr ui_create_text_edit;
            public IntPtr ui_create_text_edit_in_parent;
            public IntPtr ui_text_edit_set_text;
            public IntPtr ui_text_edit_insert_char;
            public IntPtr ui_text_edit_delete_char;
            public IntPtr ui_text_edit_get_text_len;
            public IntPtr ui_text_edit_get_text;
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
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_resize_thunk_ptr == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: RegisterResizeThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterResizeDelegate>(_ffi.ui_register_resize_thunk_ptr);
            func(callbackPtr);
        }
        
        public static void RegisterGlobalClickCallback(GlobalClickCallbackDelegate callback)
        {
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_global_click_thunk_ptr == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: RegisterGlobalClickThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterGlobalClickDelegate>(_ffi.ui_register_global_click_thunk_ptr);
            func(callbackPtr);
        }

        public static ulong CreateVStack(float spacing = 8f)
        {
            if (_ffi.ui_create_vstack == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateVStack函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateVStackDelegate>(_ffi.ui_create_vstack);
            return func(_widgetTree, spacing);
        }

        public static ulong CreateVStack(ulong parentId, float spacing = 8f)
        {
            if (_ffi.ui_create_vstack_in_parent == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateVStackInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateVStackInParentDelegate>(_ffi.ui_create_vstack_in_parent);
            return func(_widgetTree, parentId, spacing);
        }

        public static ulong CreateHStack(float spacing = 8f)
        {
            if (_ffi.ui_create_hstack == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateHStack函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateHStackDelegate>(_ffi.ui_create_hstack);
            return func(_widgetTree, spacing);
        }

        public static ulong CreateHStack(ulong parentId, float spacing = 8f)
        {
            if (_ffi.ui_create_hstack_in_parent == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateHStackInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateHStackInParentDelegate>(_ffi.ui_create_hstack_in_parent);
            return func(_widgetTree, parentId, spacing);
        }

        public static ulong CreateButton(ulong parentId, float width, float height, string text)
        {
            if (_ffi.ui_create_button_in_parent == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateButtonInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateButtonInParentDelegate>(_ffi.ui_create_button_in_parent);
            return func(_widgetTree, parentId, width, height, text);
        }

        public static ulong CreateLabel(ulong parentId, float width, float height, string text)
        {
            if (_ffi.ui_create_label_in_parent == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateLabelInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateLabelInParentDelegate>(_ffi.ui_create_label_in_parent);
            return func(_widgetTree, parentId, width, height, text);
        }

        public static ulong CreateLabel(ulong parentId, float x, float y, float width, float height, string text)
        {
            ulong id = CreateLabel(parentId, width, height, text);
            SetWidgetLayout(id, x, y, width, height);
            return id;
        }

        public static ulong CreatePanel(ulong parentId, float x, float y, float width, float height, float r = 0.2f, float g = 0.2f, float b = 0.2f, float a = 1.0f)
        {
            if (_ffi.ui_create_panel_in_parent == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreatePanelInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreatePanelInParentDelegate>(_ffi.ui_create_panel_in_parent);
            return func(_widgetTree, parentId, x, y, width, height, r, g, b, a);
        }

        public static ulong CreateTextEdit(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_text_edit_in_parent == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: CreateTextEditInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateTextEditInParentDelegate>(_ffi.ui_create_text_edit_in_parent);
            return func(_widgetTree, parentId, width, height);
        }

        public static void TextEditSetText(ulong widgetId, string text)
        {
            if (_ffi.ui_text_edit_set_text == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: TextEditSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TextEditSetTextDelegate>(_ffi.ui_text_edit_set_text);
            func(_widgetTree, widgetId, text);
        }
        
        public static string TextEditGetText(ulong widgetId)
        {
            if (_ffi.ui_text_edit_get_text_len == IntPtr.Zero || _ffi.ui_text_edit_get_text == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: TextEditGetText函数指针为空");
                return "";
            }
            
            var getLenFunc = Marshal.GetDelegateForFunctionPointer<TextEditGetTextLenDelegate>(_ffi.ui_text_edit_get_text_len);
            int len = getLenFunc(_widgetTree, widgetId);
            
            if (len == 0) return "";
            
            IntPtr buffer = Marshal.AllocHGlobal(len + 1);
            var getTextFunc = Marshal.GetDelegateForFunctionPointer<TextEditGetTextDelegate>(_ffi.ui_text_edit_get_text);
            getTextFunc(_widgetTree, widgetId, buffer, len + 1);
            
            string result = Marshal.PtrToStringAnsi(buffer);
            Marshal.FreeHGlobal(buffer);
            
            return result ?? "";
        }
        
        [DllImport("__Internal", CallingConvention = CallingConvention.Cdecl)]
        private static extern void trigger_hot_reload();
        
        public static void TriggerHotReload()
        {
            trigger_hot_reload();
        }

        public static ulong GetRootId()
        {
            if (_ffi.ui_get_root_id == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: GetRootId函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetRootIdDelegate>(_ffi.ui_get_root_id);
            return func(_widgetTree);
        }

        public static void SetWidgetLayout(ulong widgetId, float x, float y, float width, float height)
        {
            if (_ffi.ui_set_widget_layout == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: SetWidgetLayout函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetWidgetLayoutDelegate>(_ffi.ui_set_widget_layout);
            func(_widgetTree, widgetId, x, y, width, height);
        }

        public static void RemoveWidget(ulong widgetId)
        {
            if (_ffi.ui_remove_widget == IntPtr.Zero)
            {
                Console.WriteLine("[C#] ERROR: RemoveWidget函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RemoveWidgetDelegate>(_ffi.ui_remove_widget);
            func(_widgetTree, widgetId);
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
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate(ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ResizeCallbackDelegate(float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GlobalClickCallbackDelegate(float x, float y);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterGlobalClickDelegate(IntPtr callbackPtr);
    }

    public class VStack
    {
        public ulong Id { get; private set; }
        
        public VStack(ulong parentId, float spacing = 8f)
        {
            Id = UI.CreateVStack(parentId, spacing);
            Console.WriteLine($"[C#] VStack创建成功: id={Id}, parent={parentId}");
        }
        
        public ulong AddButton(float width, float height, string text)
        {
            return UI.CreateButton(Id, width, height, text);
        }
        
        public ulong AddLabel(float width, float height, string text)
        {
            return UI.CreateLabel(Id, width, height, text);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }

    public class HStack
    {
        public ulong Id { get; private set; }
        
        public HStack(ulong parentId, float spacing = 8f)
        {
            Id = UI.CreateHStack(parentId, spacing);
            Console.WriteLine($"[C#] HStack创建成功: id={Id}, parent={parentId}");
        }
        
        public Button AddButton(float width, float height, string text)
        {
            return new Button(Id, width, height, text);
        }
        
        public ulong AddLabel(float width, float height, string text)
        {
            return UI.CreateLabel(Id, width, height, text);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }

    public class Button
    {
        public ulong Id { get; private set; }
        private string _text;
        
        public Button(ulong parentId, float width, float height, string text)
        {
            _text = text;
            Id = UI.CreateButton(parentId, width, height, text);
            Console.WriteLine($"[C#] Button创建成功: id={Id}, text=\"{text}\"");
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.SetText(Id, _text); }
        }
        
        public void SetOnClick(UI.WidgetCallbackDelegate callback)
        {
            UI.SetOnClick(Id, callback);
        }
    }

    public class Label
    {
        public ulong Id { get; private set; }
        private string _text;
        
        public Label(ulong parentId, float width, float height, string text)
        {
            _text = text;
            Id = UI.CreateLabel(parentId, width, height, text);
            Console.WriteLine($"[C#] Label创建成功: id={Id}, text=\"{text}\"");
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.SetText(Id, _text); }
        }
    }

    public class Panel
    {
        public ulong Id { get; private set; }
        
        public Panel(ulong parentId, float x, float y, float width, float height, float r = 0.2f, float g = 0.2f, float b = 0.2f, float a = 1.0f)
        {
            Id = UI.CreatePanel(parentId, x, y, width, height, r, g, b, a);
            Console.WriteLine($"[Editor] Panel创建成功: id={Id}");
        }
        
        public ulong AddButton(float width, float height, string text)
        {
            return UI.CreateButton(Id, width, height, text);
        }
        
        public ulong AddLabel(float width, float height, string text)
        {
            return UI.CreateLabel(Id, width, height, text);
        }
        
        public ulong AddPanel(float x, float y, float width, float height, float r = 0.2f, float g = 0.2f, float b = 0.2f, float a = 1.0f)
        {
            return UI.CreatePanel(Id, x, y, width, height, r, g, b, a);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }
}