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
        public delegate void KeyCallbackDelegate(uint keycode, bool pressed, uint modifiers);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void MouseMoveCallbackDelegate(float x, float y, bool dragging);

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
        public delegate ulong CreatePreviewWindowDelegate(IntPtr handle, ulong parentId, float x, float y, float width, float height, ulong textureId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPreviewTextureDelegate(IntPtr handle, ulong widgetId, ulong textureId);

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

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetContentScaleDelegate(float scale);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate float GetContentScaleDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void TriggerHotReloadDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetGamePreviewExtentDelegate(uint width, uint height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetCameraParamsDelegate(float yaw, float pitch, float x, float y, float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool IsPreviewWindowSelectedDelegate(IntPtr handle, ulong widgetId);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPreviewWindowSelectedDelegate(IntPtr handle, ulong widgetId, bool selected);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetPreviewWindowEditModeDelegate(IntPtr handle, ulong widgetId, bool editMode);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListDelegate(IntPtr handle, float spacing, uint orientation);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListInParentDelegate(IntPtr handle, ulong parentId, float spacing, uint orientation);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListItemDelegate(IntPtr handle, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong CreateListItemInParentDelegate(IntPtr handle, ulong parentId, string text, uint showBorder);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ListItemSetTextDelegate(IntPtr handle, ulong widgetId, string text);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ListItemSetFontSizeDelegate(IntPtr handle, ulong widgetId, float fontSize);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate IntPtr SceneCreateDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneDestroyDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong SceneCreateCubeDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneAttachScriptDelegate(IntPtr scene, ulong entityId, string scriptPath, string className);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSetGameStateDelegate(IntPtr scene, int state);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int SceneGetGameStateDelegate(IntPtr scene);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate ulong ScenePickEntityDelegate(IntPtr scene, float ox, float oy, float oz, float dx, float dy, float dz);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneSelectEntityDelegate(IntPtr scene, ulong entityId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneUpdateDelegate(IntPtr scene, float deltaTime);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetRendererGameStateDelegate(int state);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int GetRendererGameStateDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetEntityTransformDelegate(float px, float py, float pz,
                                                         float rx, float ry, float rz, float rw,
                                                         float sx, float sy, float sz);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetEntityAngleDelegate(float angle);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate float GetEntityAngleDelegate();
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneGetEntityPositionDelegate(IntPtr scene, ulong entityId, out float x, out float y, out float z);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneGetEntityRotationDelegate(IntPtr scene, ulong entityId, out float x, out float y, out float z, out float w);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneGetEntityScaleDelegate(IntPtr scene, ulong entityId, out float x, out float y, out float z);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SceneRotateEntityDelegate(IntPtr scene, ulong entityId, float angleDegrees);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SetSelectedEntityDelegate(ulong entityId, bool selected);

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
            public IntPtr ui_register_key_thunk_ptr;
            public IntPtr ui_register_mouse_move_thunk_ptr;
            public IntPtr ui_trigger_resize;
            public IntPtr ui_get_screen_size;
            public IntPtr ui_set_content_scale;
            public IntPtr ui_get_content_scale;
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
            public IntPtr ui_create_preview_window;
            public IntPtr ui_set_preview_texture;
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
            public IntPtr ui_trigger_hot_reload;
            public IntPtr ui_set_game_preview_extent;
            public IntPtr ui_set_camera_params;
            public IntPtr ui_is_preview_window_selected;
            public IntPtr ui_set_preview_window_selected;
            public IntPtr ui_set_preview_window_edit_mode;
            public IntPtr ui_create_list;
            public IntPtr ui_create_list_in_parent;
            public IntPtr ui_create_list_item;
            public IntPtr ui_create_list_item_in_parent;
            public IntPtr ui_list_item_set_text;
            public IntPtr ui_list_item_set_font_size;
            public IntPtr scene_create;
            public IntPtr scene_destroy;
            public IntPtr scene_create_cube;
            public IntPtr scene_attach_script;
            public IntPtr scene_set_game_state;
            public IntPtr scene_get_game_state;
            public IntPtr scene_pick_entity;
            public IntPtr scene_select_entity;
            public IntPtr scene_update;
            public IntPtr set_renderer_game_state;
            public IntPtr get_renderer_game_state;
            public IntPtr set_entity_transform;
            public IntPtr set_entity_angle;
            public IntPtr get_entity_angle;
            public IntPtr scene_get_entity_position;
            public IntPtr scene_get_entity_rotation;
            public IntPtr scene_get_entity_scale;
            public IntPtr scene_rotate_entity;
            public IntPtr set_selected_entity;
            public IntPtr widget_tree_ptr;
            public IntPtr dfx_handle;
        }

        public static void InitFromContext(IntPtr contextPtr)
        {
            _ffi = Marshal.PtrToStructure<FfiContext>(contextPtr);
            _widgetTree = _ffi.widget_tree_ptr;
            
            if (_ffi.dfx_handle != IntPtr.Zero)
            {
                Log.Init(_ffi.dfx_handle);
            }
            
            Log.Info("C#", "FfiContext初始化成功");
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

        public static float GetContentScale()
        {
            if (_ffi.ui_get_content_scale == IntPtr.Zero)
            {
                return 1.0f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetContentScaleDelegate>(_ffi.ui_get_content_scale);
            return func();
        }

        public static void SetContentScale(float scale)
        {
            if (_ffi.ui_set_content_scale == IntPtr.Zero)
            {
                Log.Error("C#", "SetContentScale函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetContentScaleDelegate>(_ffi.ui_set_content_scale);
            func(scale);
        }

public static void RegisterResizeCallback(ResizeCallbackDelegate callback)
        {
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_resize_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterResizeThunkPtr函数指针为空");
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
                Log.Error("C#", "RegisterGlobalClickThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterGlobalClickDelegate>(_ffi.ui_register_global_click_thunk_ptr);
            func(callbackPtr);
        }

        public static void RegisterKeyCallback(KeyCallbackDelegate callback)
        {
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_key_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterKeyThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterKeyDelegate>(_ffi.ui_register_key_thunk_ptr);
            func(callbackPtr);
        }

        public static void RegisterMouseMoveCallback(MouseMoveCallbackDelegate callback)
        {
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_mouse_move_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterMouseMoveThunkPtr函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<RegisterMouseMoveDelegate>(_ffi.ui_register_mouse_move_thunk_ptr);
            func(callbackPtr);
        }

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
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            var func = Marshal.GetDelegateForFunctionPointer<SetOnClickDelegate>(_ffi.ui_button_set_on_click_thunk_ptr);
            func(_widgetTree, widgetId, callbackPtr);
        }
        
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

        public static IntPtr SceneCreate()
        {
            if (_ffi.scene_create == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreate函数指针为空");
                return IntPtr.Zero;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateDelegate>(_ffi.scene_create);
            return func();
        }

        public static void SceneDestroy(IntPtr scene)
        {
            if (_ffi.scene_destroy == IntPtr.Zero)
            {
                Log.Error("C#", "SceneDestroy函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneDestroyDelegate>(_ffi.scene_destroy);
            func(scene);
        }

        public static ulong SceneCreateCube(IntPtr scene)
        {
            if (_ffi.scene_create_cube == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreateCube函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateCubeDelegate>(_ffi.scene_create_cube);
            return func(scene);
        }

        public static void SceneAttachScript(IntPtr scene, ulong entityId, string scriptPath, string className)
        {
            if (_ffi.scene_attach_script == IntPtr.Zero)
            {
                Log.Error("C#", "SceneAttachScript函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneAttachScriptDelegate>(_ffi.scene_attach_script);
            func(scene, entityId, scriptPath, className);
        }

        public static void SceneSetGameState(IntPtr scene, int state)
        {
            if (_ffi.scene_set_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetGameState函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetGameStateDelegate>(_ffi.scene_set_game_state);
            func(scene, state);
        }

        public static int SceneGetGameState(IntPtr scene)
        {
            if (_ffi.scene_get_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetGameState函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetGameStateDelegate>(_ffi.scene_get_game_state);
            return func(scene);
        }

        public static ulong ScenePickEntity(IntPtr scene, float ox, float oy, float oz, float dx, float dy, float dz)
        {
            if (_ffi.scene_pick_entity == IntPtr.Zero)
            {
                Log.Error("C#", "ScenePickEntity函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ScenePickEntityDelegate>(_ffi.scene_pick_entity);
            return func(scene, ox, oy, oz, dx, dy, dz);
        }

        public static void SceneSelectEntity(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_select_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSelectEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSelectEntityDelegate>(_ffi.scene_select_entity);
            func(scene, entityId);
        }

        public static void SceneUpdate(IntPtr scene, float deltaTime)
        {
            if (_ffi.scene_update == IntPtr.Zero)
            {
                Log.Error("C#", "SceneUpdate函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneUpdateDelegate>(_ffi.scene_update);
            func(scene, deltaTime);
        }

        public static void SetRendererGameState(int state)
        {
            if (_ffi.set_renderer_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SetRendererGameState函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetRendererGameStateDelegate>(_ffi.set_renderer_game_state);
            func(state);
        }

        public static int GetRendererGameState()
        {
            if (_ffi.get_renderer_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "GetRendererGameState函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetRendererGameStateDelegate>(_ffi.get_renderer_game_state);
            return func();
        }

        public static void SetEntityTransform(float px, float py, float pz,
                                               float rx, float ry, float rz, float rw,
                                               float sx, float sy, float sz)
        {
            if (_ffi.set_entity_transform == IntPtr.Zero)
            {
                Log.Error("C#", "SetEntityTransform函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetEntityTransformDelegate>(_ffi.set_entity_transform);
            func(px, py, pz, rx, ry, rz, rw, sx, sy, sz);
        }

        public static void SetEntityAngle(float angle)
        {
            if (_ffi.set_entity_angle == IntPtr.Zero)
            {
                Log.Error("C#", "SetEntityAngle函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetEntityAngleDelegate>(_ffi.set_entity_angle);
            func(angle);
        }

        public static float GetEntityAngle()
        {
            if (_ffi.get_entity_angle == IntPtr.Zero)
            {
                Log.Error("C#", "GetEntityAngle函数指针为空");
                return 0.0f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetEntityAngleDelegate>(_ffi.get_entity_angle);
            return func();
        }

        public static void SceneGetEntityPosition(IntPtr scene, ulong entityId, out float x, out float y, out float z)
        {
            if (_ffi.scene_get_entity_position == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityPosition函数指针为空");
                x = 0; y = 0; z = 0;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityPositionDelegate>(_ffi.scene_get_entity_position);
            func(scene, entityId, out x, out y, out z);
        }

        public static void SceneGetEntityRotation(IntPtr scene, ulong entityId, out float x, out float y, out float z, out float w)
        {
            if (_ffi.scene_get_entity_rotation == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityRotation函数指针为空");
                x = 0; y = 0; z = 0; w = 1;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityRotationDelegate>(_ffi.scene_get_entity_rotation);
            func(scene, entityId, out x, out y, out z, out w);
        }

        public static void SceneGetEntityScale(IntPtr scene, ulong entityId, out float x, out float y, out float z)
        {
            if (_ffi.scene_get_entity_scale == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityScale函数指针为空");
                x = 1; y = 1; z = 1;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityScaleDelegate>(_ffi.scene_get_entity_scale);
            func(scene, entityId, out x, out y, out z);
        }

        public static void SceneRotateEntity(IntPtr scene, ulong entityId, float angleDegrees)
        {
            if (_ffi.scene_rotate_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRotateEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRotateEntityDelegate>(_ffi.scene_rotate_entity);
            func(scene, entityId, angleDegrees);
        }

        public static void SetSelectedEntity(ulong entityId, bool selected)
        {
            if (_ffi.set_selected_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SetSelectedEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetSelectedEntityDelegate>(_ffi.set_selected_entity);
            func(entityId, selected);
        }

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate(ulong widgetId);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ResizeCallbackDelegate(float width, float height);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void GlobalClickCallbackDelegate(float x, float y);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void UpdateCallbackDelegate(float deltaTime);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterGlobalClickDelegate(IntPtr callbackPtr);
        public delegate void RegisterKeyDelegate(IntPtr callbackPtr);
        public delegate void RegisterMouseMoveDelegate(IntPtr callbackPtr);
        
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void RegisterUpdateDelegate(IntPtr callbackPtr);
        
        public static void RegisterUpdateCallback(UpdateCallbackDelegate callback)
        {
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            
            if (_ffi.ui_register_update_thunk_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "RegisterUpdateThunkPtr函数指针为空");
                return;
            }
            
            var func = Marshal.GetDelegateForFunctionPointer<RegisterUpdateDelegate>(_ffi.ui_register_update_thunk_ptr);
            func(callbackPtr);
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

    public enum GameState
    {
        Editing = 0,
        Running = 1,
        Paused = 2
    }

    public class Scene
    {
        private IntPtr _scenePtr;

        public Scene()
        {
            _scenePtr = UI.SceneCreate();
            if (_scenePtr == IntPtr.Zero)
            {
                throw new Exception("Failed to create scene");
            }
        }

        public IntPtr ScenePtr => _scenePtr;

        public void Destroy()
        {
            if (_scenePtr != IntPtr.Zero)
            {
                UI.SceneDestroy(_scenePtr);
                _scenePtr = IntPtr.Zero;
            }
        }

        public ulong CreateCube()
        {
            return UI.SceneCreateCube(_scenePtr);
        }

        public void AttachScript(ulong entityId, string scriptPath, string className)
        {
            UI.SceneAttachScript(_scenePtr, entityId, scriptPath, className);
        }

        public void SetGameState(GameState state)
        {
            UI.SceneSetGameState(_scenePtr, (int)state);
        }

        public GameState GetGameState()
        {
            return (GameState)UI.SceneGetGameState(_scenePtr);
        }

        public ulong PickEntity(float ox, float oy, float oz, float dx, float dy, float dz)
        {
            return UI.ScenePickEntity(_scenePtr, ox, oy, oz, dx, dy, dz);
        }

        public void SelectEntity(ulong entityId)
        {
            UI.SceneSelectEntity(_scenePtr, entityId);
            UI.SetSelectedEntity(entityId, true);  // 高亮显示
            Log.Info("Scene", $"Entity {entityId} selected with highlight");
        }

        public void ClearSelection()
        {
            // 清除所有选中Entity的高亮
            // TODO: 需要FFI函数获取选中列表
            UI.SetSelectedEntity(0, false);  // 清除高亮
            Log.Info("Scene", "Selection cleared, highlight removed");
        }

        public void Update(float deltaTime)
        {
            UI.SceneUpdate(_scenePtr, deltaTime);
        }
    }
}