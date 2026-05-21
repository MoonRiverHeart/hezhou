using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static partial class UI
    {
        public static ulong CreateVStack(float spacing = 8f)
        {
            if (_ffi.ui_create_vstack == IntPtr.Zero)
            {
                Log.Error("C#", "CreateVStack函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateVStackDelegate>(_ffi.ui_create_vstack);
            return func(_widgetTree, spacing);
        }

        public static ulong CreateVStack(ulong parentId, float spacing = 8f)
        {
            if (_ffi.ui_create_vstack_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateVStackInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateVStackInParentDelegate>(_ffi.ui_create_vstack_in_parent);
            return func(_widgetTree, parentId, spacing);
        }

        public static ulong CreateHStack(float spacing = 8f)
        {
            if (_ffi.ui_create_hstack == IntPtr.Zero)
            {
                Log.Error("C#", "CreateHStack函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateHStackDelegate>(_ffi.ui_create_hstack);
            return func(_widgetTree, spacing);
        }

        public static ulong CreateHStack(ulong parentId, float spacing = 8f)
        {
            if (_ffi.ui_create_hstack_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateHStackInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateHStackInParentDelegate>(_ffi.ui_create_hstack_in_parent);
            return func(_widgetTree, parentId, spacing);
        }

        public static ulong CreateButton(ulong parentId, float width, float height, string text)
        {
            if (_ffi.ui_create_button_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateButtonInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateButtonInParentDelegate>(_ffi.ui_create_button_in_parent);
            return func(_widgetTree, parentId, width, height, text);
        }

        public static ulong CreateLabel(ulong parentId, float width, float height, string text)
        {
            if (_ffi.ui_create_label_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateLabelInParent函数指针为空");
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

        public static ulong CreatePreviewWindow(ulong parentId, float x, float y, float width, float height, ulong textureId = 1)
        {
            if (_ffi.ui_create_preview_window == IntPtr.Zero)
            {
                Log.Error("C#", "CreatePreviewWindow函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreatePreviewWindowDelegate>(_ffi.ui_create_preview_window);
            return func(_widgetTree, parentId, x, y, width, height, textureId);
        }

        public static void SetPreviewTexture(ulong widgetId, ulong textureId)
        {
            if (_ffi.ui_set_preview_texture == IntPtr.Zero)
            {
                Log.Error("C#", "SetPreviewTexture函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetPreviewTextureDelegate>(_ffi.ui_set_preview_texture);
            func(_widgetTree, widgetId, textureId);
        }

        public static ulong CreatePanel(ulong parentId, float x, float y, float width, float height, float r = 0.2f, float g = 0.2f, float b = 0.2f, float a = 1.0f)
        {
            if (_ffi.ui_create_panel_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreatePanelInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreatePanelInParentDelegate>(_ffi.ui_create_panel_in_parent);
            return func(_widgetTree, parentId, x, y, width, height, r, g, b, a);
        }

        public static ulong CreateTextEdit(ulong parentId, float width, float height)
        {
            if (_ffi.ui_create_text_edit_in_parent == IntPtr.Zero)
            {
                Log.Error("C#", "CreateTextEditInParent函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<CreateTextEditInParentDelegate>(_ffi.ui_create_text_edit_in_parent);
            return func(_widgetTree, parentId, width, height);
        }

        public static void TextEditSetText(ulong widgetId, string text)
        {
            if (_ffi.ui_text_edit_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "TextEditSetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TextEditSetTextDelegate>(_ffi.ui_text_edit_set_text);
            func(_widgetTree, widgetId, text);
        }
        
        public static string TextEditGetText(ulong widgetId)
        {
            if (_ffi.ui_text_edit_get_text_len == IntPtr.Zero || _ffi.ui_text_edit_get_text == IntPtr.Zero)
            {
                Log.Error("C#", "TextEditGetText函数指针为空");
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

        public static void TriggerHotReload()
        {
            if (_ffi.ui_trigger_hot_reload == IntPtr.Zero)
            {
                Log.Error("C#", "TriggerHotReload函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TriggerHotReloadDelegate>(_ffi.ui_trigger_hot_reload);
            func();
        }

        public static void SetStatusText(string status)
        {
            if (_ffi.set_status_text == IntPtr.Zero)
            {
                Log.Error("C#", "SetStatusText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetStatusTextDelegate>(_ffi.set_status_text);
            func(status);
        }

        public static void SetGamePreviewExtent(uint width, uint height)
        {
            if (_ffi.ui_set_game_preview_extent == IntPtr.Zero)
            {
                Log.Error("C#", "SetGamePreviewExtent函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetGamePreviewExtentDelegate>(_ffi.ui_set_game_preview_extent);
            func(width, height);
            Log.Info("C#", $"Game preview extent set to {width}x{height}");
        }

        public static void SetCameraParams(float yaw, float pitch, float x, float y, float z)
        {
            if (_ffi.ui_set_camera_params == IntPtr.Zero)
            {
                Log.Error("C#", "SetCameraParams函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetCameraParamsDelegate>(_ffi.ui_set_camera_params);
            func(yaw, pitch, x, y, z);
            Log.Info("C#", $"Camera params set: yaw={yaw}, pitch={pitch}, pos=({x}, {y}, {z})");
        }

        public static bool IsPreviewWindowSelected(ulong widgetId)
        {
            if (_ffi.ui_is_preview_window_selected == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<IsPreviewWindowSelectedDelegate>(_ffi.ui_is_preview_window_selected);
            return func(_widgetTree, widgetId);
        }

public static void SetPreviewWindowSelected(ulong widgetId, bool selected)
        {
            if (_ffi.ui_set_preview_window_selected == IntPtr.Zero)
            {
                Log.Error("C#", "SetPreviewWindowSelected函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetPreviewWindowSelectedDelegate>(_ffi.ui_set_preview_window_selected);
            func(_widgetTree, widgetId, selected);
        }
        
        public static void SetPreviewWindowEditMode(ulong widgetId, bool editMode)
        {
            if (_ffi.ui_set_preview_window_edit_mode == IntPtr.Zero)
            {
                Log.Error("C#", "SetPreviewWindowEditMode函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetPreviewWindowEditModeDelegate>(_ffi.ui_set_preview_window_edit_mode);
            func(_widgetTree, widgetId, editMode);
            Log.Info("C#", $"PreviewWindow editMode set to: {editMode}");
        }
        
public static ulong GetRootId()
        {
            if (_ffi.ui_get_root_id == IntPtr.Zero)
            {
                Log.Error("C#", "GetRootId函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetRootIdDelegate>(_ffi.ui_get_root_id);
            return func(_widgetTree);
        }
        
        public static void SetTextEditShowLineNumbers(ulong widgetId, bool show)
        {
            if (_ffi.ui_text_edit_show_line_numbers == IntPtr.Zero)
            {
                Log.Error("C#", "SetTextEditShowLineNumbers函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<TextEditShowLineNumbersDelegate>(_ffi.ui_text_edit_show_line_numbers);
            func(_widgetTree, widgetId, show);
        }

        public static void SetWidgetLayout(ulong widgetId, float x, float y, float width, float height)
        {
            if (_ffi.ui_set_widget_layout == IntPtr.Zero)
            {
                Log.Error("C#", "SetWidgetLayout函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetWidgetLayoutDelegate>(_ffi.ui_set_widget_layout);
            func(_widgetTree, widgetId, x, y, width, height);
        }
        
        public static void SetWidgetLayer(ulong widgetId, uint layer)
        {
            if (_ffi.ui_widget_set_layer == IntPtr.Zero)
            {
                Log.Error("C#", "SetWidgetLayer函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetWidgetLayerDelegate>(_ffi.ui_widget_set_layer);
            func(_widgetTree, widgetId, layer);
        }

        public static void SetFlexExpand(ulong widgetId, bool expand)
        {
            if (_ffi.ui_widget_set_flex_expand == IntPtr.Zero)
            {
                Log.Error("C#", "SetFlexExpand函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetFlexExpandDelegate>(_ffi.ui_widget_set_flex_expand);
            func(_widgetTree, widgetId, expand ? 1u : 0u);
        }

        public static void SetCrossAxisFill(ulong widgetId, bool fill)
        {
            if (_ffi.ui_widget_set_cross_axis_fill == IntPtr.Zero)
            {
                Log.Error("C#", "SetCrossAxisFill函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetCrossAxisFillDelegate>(_ffi.ui_widget_set_cross_axis_fill);
            func(_widgetTree, widgetId, fill ? 1u : 0u);
        }

        public static void DebugPrintUITree()
        {
            if (_ffi.ui_debug_print_widget_tree == IntPtr.Zero)
            {
                Log.Error("C#", "DebugPrintUITree函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<DebugPrintWidgetTreeDelegate>(_ffi.ui_debug_print_widget_tree);
            func(_widgetTree);
        }
        
        public static uint GetWidgetLayer(ulong widgetId)
        {
            if (_ffi.ui_widget_get_layer == IntPtr.Zero)
            {
                Log.Error("C#", "GetWidgetLayer函数指针为空");
                return 1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetWidgetLayerDelegate>(_ffi.ui_widget_get_layer);
            return func(_widgetTree, widgetId);
        }

        public static void RemoveWidget(ulong widgetId)
        {
            if (_ffi.ui_remove_widget == IntPtr.Zero)
            {
                Log.Error("C#", "RemoveWidget函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RemoveWidgetDelegate>(_ffi.ui_remove_widget);
            func(_widgetTree, widgetId);
        }

        public static void SetText(ulong widgetId, string text)
        {
            if (_ffi.ui_widget_set_text == IntPtr.Zero)
            {
                Log.Error("C#", "SetText函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetTextDelegate>(_ffi.ui_widget_set_text);
            func(_widgetTree, widgetId, text);
        }

        public static void SetOnClick(ulong widgetId, WidgetCallbackDelegate callback)
        {
            if (_ffi.ui_button_set_on_click_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "SetOnClick函数指针为空");
                return;
            }
            _onclickCallbacks[widgetId] = callback;
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<SetOnClickDelegate>(_ffi.ui_button_set_on_click_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }
    }

    public class VStack
    {
        public ulong Id { get; private set; }
        
        public VStack(ulong parentId, float spacing = 8f)
        {
            Id = UI.CreateVStack(parentId, spacing);
            Log.Info("C#", $"VStack创建成功: id={Id}, parent={parentId}");
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
            Log.Info("C#", $"HStack created: id={Id}, parent={parentId}");
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
        private UI.WidgetCallbackDelegate _callback;
        
        public Button(ulong parentId, float width, float height, string text)
        {
            _text = text;
            Id = UI.CreateButton(parentId, width, height, text);
            Log.Info("C#", $"Button created: id={Id}, text=\"{text}\"");
        }
        
        public string Text
        {
            get => _text;
            set { _text = value; UI.SetText(Id, _text); }
        }
        
        public void SetOnClick(UI.WidgetCallbackDelegate callback)
        {
            _callback = callback;
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
            Log.Info("C#", $"Label创建成功: id={Id}, text=\"{text}\"");
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
            Log.Info("Editor", $"Panel创建成功: id={Id}");
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