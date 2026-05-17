using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static class UI
    {
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void UpdateCallbackDelegate(float deltaTime);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void WidgetCallbackDelegate(ulong widgetId);

        public static void RegisterUpdateCallback(UpdateCallbackDelegate callback)
        {
            IntPtr callbackPtr = Marshal.GetFunctionPointerForDelegate(callback);
            NativeMethods.ui_register_update_thunk_ptr(callbackPtr);
        }

        private static class NativeMethods
        {
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void ui_register_update_thunk_ptr(IntPtr callback);
        }
    }
}