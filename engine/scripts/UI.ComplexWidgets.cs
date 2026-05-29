using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static partial class UI
    {
        public static ulong CreateList(ulong parentId, float spacing = 0f, uint orientation = 0)
        {
            if (_ffi.ui_create_list_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateListInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateListInParentDelegate>(_ffi.ui_create_list_in_parent);
            return func(_widgetTree, parentId, spacing, orientation);
        }
        
        public static ulong CreateListItem(ulong parentId, string text, bool showBorder = false)
        {
            if (_ffi.ui_create_list_item_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateListItemInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateListItemInParentDelegate>(_ffi.ui_create_list_item_in_parent);
            return func(_widgetTree, parentId, text, showBorder ? 1u : 0u);
        }
        
        public static void SetListItemText(ulong widgetId, string text)
        {
            if (_ffi.ui_list_item_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "ListItemSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ListItemSetTextDelegate>(_ffi.ui_list_item_set_text);
            func(_widgetTree, widgetId, text);
        }
        
        public static void SetListItemFontSize(ulong widgetId, float fontSize)
        {
            if (_ffi.ui_list_item_set_font_size == IntPtr.Zero)
            {
                Log.Error("C#", "ListItemSetFontSize函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ListItemSetFontSizeDelegate>(_ffi.ui_list_item_set_font_size);
            func(_widgetTree, widgetId, fontSize);
        }

        public static ulong CreateDropdown(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_dropdown == IntPtr.Zero)
            {
                Log.Error("C#", "CreateDropdown函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateDropdownDelegate>(_ffi.ui_create_dropdown);
            return func(_widgetTree, parentId, width, height);
        }
        
        public static void DropdownSetOptions(ulong widgetId, string[] options)
        {
            if (_ffi.ui_dropdown_set_options == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownSetOptions函数指针为空");
                return;
            }
            // 用\0分隔拼接，手动分配内存传递IntPtr（Mono marshalling会截断嵌入\0的string）
            string joined = string.Join("\0", options) + "\0";
            byte[] bytes = System.Text.Encoding.ASCII.GetBytes(joined);
            IntPtr buffer = Marshal.AllocHGlobal(bytes.Length);
            Marshal.Copy(bytes, 0, buffer, bytes.Length);
            var func = Marshal.GetDelegateForFunctionPointer<DropdownSetOptionsDelegate>(_ffi.ui_dropdown_set_options);
            func(_widgetTree, widgetId, buffer, (ulong)options.Length);
            Marshal.FreeHGlobal(buffer);
        }
        
        public static void DropdownSetSelected(ulong widgetId, ulong index)
        {
            if (_ffi.ui_dropdown_set_selected == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownSetSelected函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DropdownSetSelectedDelegate>(_ffi.ui_dropdown_set_selected);
            func(_widgetTree, widgetId, index);
        }
        
        public static ulong DropdownGetSelected(ulong widgetId)
        {
            if (_ffi.ui_dropdown_get_selected == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownGetSelected函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DropdownGetSelectedDelegate>(_ffi.ui_dropdown_get_selected);
            return func(_widgetTree, widgetId);
        }
        
        public static void DropdownSetOnSelect(ulong widgetId, DropdownSelectCallbackDelegate callback)
        {
            if (_ffi.ui_dropdown_set_on_select_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "DropdownSetOnSelectThunkPtr函数指针为空");
                return;
            }
            _dropdownCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<DropdownSetOnSelectThunkPtrDelegate>(_ffi.ui_dropdown_set_on_select_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }

        public static ulong CreateInputField(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_input_field == IntPtr.Zero)
            {
                Log.Error("C#", "CreateInputField函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateInputFieldDelegate>(_ffi.ui_create_input_field);
            return func(_widgetTree, parentId, width, height);
        }
        
        public static void InputFieldSetText(ulong widgetId, string text)
        {
            if (_ffi.ui_input_field_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldSetTextDelegate>(_ffi.ui_input_field_set_text);
            func(_widgetTree, widgetId, text);
        }
        
        public static string InputFieldGetText(ulong widgetId)
        {
            if (_ffi.ui_input_field_get_text == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldGetText函数指针为空");
                return "";
            }
            
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldGetTextDelegate>(_ffi.ui_input_field_get_text);
            int len = func(_widgetTree, widgetId, buffer, 256);
            
            string result = len > 0 ? Marshal.PtrToStringAnsi(buffer, len) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            
            return result;
        }
        
        public static void InputFieldSetOnChange(ulong widgetId, InputFieldChangeCallbackDelegate callback)
        {
            if (_ffi.ui_input_field_set_on_change_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldSetOnChangeThunkPtr函数指针为空");
                return;
            }
            _inputFieldCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldSetOnChangeThunkPtrDelegate>(_ffi.ui_input_field_set_on_change_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }
        
        public static void InputFieldSetPlaceholder(ulong widgetId, string placeholder)
        {
            if (_ffi.ui_input_field_set_placeholder == IntPtr.Zero)
            {
                Log.Error("C#", "InputFieldSetPlaceholder函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<InputFieldSetPlaceholderDelegate>(_ffi.ui_input_field_set_placeholder);
            func(_widgetTree, widgetId, placeholder);
        }
    }

    public class List
    {
        public ulong Id { get; private set; }
        
        public List(ulong parentId, float spacing = 0f, bool horizontal = true)
        {
            Id = UI.CreateList(parentId, spacing, horizontal ? 0u : 1u);
        }
        
        public ListItem AddItem(string text, bool showBorder = false)
        {
            return new ListItem(Id, text, showBorder);
        }
        
        public void SetPosition(float x, float y)
        {
            UI.SetWidgetLayout(Id, x, y, 0, 0);
        }
    }
    
    public class ListItem
    {
        public ulong Id { get; private set; }
        private string _text;
        
        public ListItem(ulong parentId, string text, bool showBorder = false)
        {
            _text = text;
            Id = UI.CreateListItem(parentId, text, showBorder);
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.SetListItemText(Id, _text); }
        }
    }
    
    public class Dropdown
    {
        public ulong Id { get; private set; }
        private string[] _options;
        private UI.DropdownSelectCallbackDelegate _callback;
        
        public Dropdown(ulong parentId, float width, float height, string[] options = null)
        {
            Id = UI.CreateDropdown(parentId, width, height);
            _options = options ?? new string[0];
            if (_options.Length > 0)
            {
                UI.DropdownSetOptions(Id, _options);
            }
        }
        
        public string[] Options
        {
            get => _options;
            set
            {
                _options = value;
                UI.DropdownSetOptions(Id, _options);
            }
        }
        
        public ulong SelectedIndex
        {
            get => UI.DropdownGetSelected(Id);
            set => UI.DropdownSetSelected(Id, value);
        }
        
        public string SelectedValue => _options.Length > 0 && SelectedIndex < (ulong)_options.Length 
            ? _options[SelectedIndex] : null;
        
        public void SetOnSelect(UI.DropdownSelectCallbackDelegate callback)
        {
            _callback = callback;
            UI.DropdownSetOnSelect(Id, callback);
        }
    }
    
    public class InputField
    {
        public ulong Id { get; private set; }
        private string _text;
        private string _placeholder;
        private UI.InputFieldChangeCallbackDelegate _callback;
        
        public InputField(ulong parentId, float width, float height, string placeholder = "")
        {
            Id = UI.CreateInputField(parentId, width, height);
            _text = "";
            _placeholder = placeholder;
            if (!string.IsNullOrEmpty(placeholder))
            {
                UI.InputFieldSetPlaceholder(Id, placeholder);
            }
        }
        
        public string Text
        {
            get => UI.InputFieldGetText(Id);
            set
            {
                _text = value;
                UI.InputFieldSetText(Id, value);
            }
        }
        
        public string Placeholder
        {
            get => _placeholder;
            set
            {
                _placeholder = value;
                UI.InputFieldSetPlaceholder(Id, value);
            }
        }
        
        public void SetOnChange(UI.InputFieldChangeCallbackDelegate callback)
        {
            _callback = callback;
            UI.InputFieldSetOnChange(Id, callback);
        }
    }
}