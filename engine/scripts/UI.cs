using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static class UI
    {
        private static IntPtr _widgetTree;
        private static IntPtr _eventDispatcher;

        public static void SetHandles(IntPtr widgetTree, IntPtr eventDispatcher)
        {
            _widgetTree = widgetTree;
            _eventDispatcher = eventDispatcher;
        }

        public static ulong CreateRootPanel(float width, float height)
        {
            return NativeMethods.ui_widget_tree_create_root_panel(
                _widgetTree, 0, 0, width, height);
        }

        public static ulong AddButton(ulong parentId, string text,
            float x = 0, float y = 0, float width = 200, float height = 40)
        {
            return NativeMethods.ui_widget_tree_add_button(
                _widgetTree, parentId, x, y, width, height, text);
        }

        public static ulong AddLabel(ulong parentId, string text,
            float x = 0, float y = 0, float width = 300, float height = 30)
        {
            return NativeMethods.ui_widget_tree_add_label(
                _widgetTree, parentId, x, y, width, height, text);
        }

        public static ulong AddPanel(ulong parentId,
            float x, float y, float width, float height)
        {
            return NativeMethods.ui_widget_tree_add_panel(
                _widgetTree, parentId, x, y, width, height);
        }

        public static void SetText(ulong widgetId, string text)
        {
            NativeMethods.ui_widget_set_text(_widgetTree, widgetId, text);
        }

        public static void SetBackgroundColor(ulong widgetId,
            float r, float g, float b, float a = 1.0f)
        {
            NativeMethods.ui_widget_set_background_color(
                _widgetTree, widgetId, r, g, b, a);
        }

        public static void SetLayout(ulong widgetId,
            float x, float y, float width, float height)
        {
            NativeMethods.ui_widget_set_layout(
                _widgetTree, widgetId, x, y, width, height);
        }

        public static unsafe void SetOnClick(ulong widgetId,
            delegate* unmanaged[Cdecl]<ulong, void> callback)
        {
            NativeMethods.ui_button_set_on_click_thunk(
                _widgetTree, widgetId, callback);
        }

        public static unsafe void RegisterUpdateCallback(
            delegate* unmanaged[Cdecl]<float, void> callback)
        {
            NativeMethods.ui_register_update_thunk(callback);
        }

        public static unsafe void RegisterInitCallback(
            delegate* unmanaged[Cdecl]<void> callback)
        {
            NativeMethods.ui_register_init_thunk(callback);
        }

        public static void DispatchTouchBegin(float x, float y,
            uint pointerId = 0, ulong timestamp = 0)
        {
            NativeMethods.ui_event_dispatcher_dispatch_touch_begin(
                _eventDispatcher, x, y, pointerId, timestamp);
        }

        public static void DispatchTouchEnd(float x, float y,
            uint pointerId = 0, ulong timestamp = 0)
        {
            NativeMethods.ui_event_dispatcher_dispatch_touch_end(
                _eventDispatcher, x, y, pointerId, timestamp);
        }

        public static void DispatchKeyDown(uint keycode,
            uint modifiers = 0, ulong timestamp = 0)
        {
            NativeMethods.ui_event_dispatcher_dispatch_key_down(
                _eventDispatcher, keycode, modifiers, timestamp);
        }

        public static void TriggerUpdate(float deltaTime)
        {
            NativeMethods.ui_trigger_update(deltaTime);
        }

        public static void TriggerOnClick(ulong widgetId)
        {
            NativeMethods.ui_trigger_onclick(widgetId);
        }

        private static class NativeMethods
        {
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong ui_widget_tree_create_root_panel(
                IntPtr handle, float x, float y, float w, float h);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong ui_widget_tree_add_button(
                IntPtr handle, ulong parent, float x, float y,
                float w, float h, string text);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong ui_widget_tree_add_label(
                IntPtr handle, ulong parent, float x, float y,
                float w, float h, string text);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong ui_widget_tree_add_panel(
                IntPtr handle, ulong parent, float x, float y,
                float w, float h);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_widget_set_text(
                IntPtr handle, ulong widgetId, string text);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_widget_set_background_color(
                IntPtr handle, ulong widgetId, float r, float g, float b, float a);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_widget_set_layout(
                IntPtr handle, ulong widgetId, float x, float y, float w, float h);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern unsafe void ui_button_set_on_click_thunk(
                IntPtr handle, ulong widgetId,
                delegate* unmanaged[Cdecl]<ulong, void> callback);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern unsafe void ui_register_update_thunk(
                delegate* unmanaged[Cdecl]<float, void> callback);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern unsafe void ui_register_init_thunk(
                delegate* unmanaged[Cdecl]<void> callback);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_event_dispatcher_dispatch_touch_begin(
                IntPtr handle, float x, float y, uint pointerId, ulong timestamp);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_event_dispatcher_dispatch_touch_end(
                IntPtr handle, float x, float y, uint pointerId, ulong timestamp);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_event_dispatcher_dispatch_key_down(
                IntPtr handle, uint keycode, uint modifiers, ulong timestamp);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_trigger_update(float deltaTime);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_trigger_onclick(ulong widgetId);
        }
    }
}