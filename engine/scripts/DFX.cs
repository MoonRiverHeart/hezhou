using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public enum LogLevel : byte
    {
        Trace = 0,
        Debug = 1,
        Info = 2,
        Warn = 3,
        Error = 4,
        Fatal = 5
    }

    public class DFX : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        public static DFX Create()
        {
            var handle = NativeMethods.dfx_create();
            return new DFX(handle);
        }

        private DFX(IntPtr handle)
        {
            _handle = handle;
        }

        public void EnableAll()
        {
            NativeMethods.dfx_enable_all(_handle);
        }

        public void SetLogLevel(LogLevel level)
        {
            NativeMethods.dfx_set_log_level(_handle, (byte)level);
        }

        public void Log(string message, LogLevel level = LogLevel.Info, string module = "UIScript")
        {
            NativeMethods.dfx_log(_handle, (byte)level, module, message, "DFX.cs", 0);
        }

        public void TraceBegin(string name, string category = "ui")
        {
            NativeMethods.dfx_trace_begin(_handle, name, category);
        }

        public void TraceEnd(string name, string category = "ui")
        {
            NativeMethods.dfx_trace_end(_handle, name, category);
        }

        public float GetFPS()
        {
            return NativeMethods.dfx_get_fps(_handle);
        }

        public ulong GetFrameCount()
        {
            return NativeMethods.dfx_get_frame_count(_handle);
        }

        public void ClearLogBuffer()
        {
            NativeMethods.dfx_clear_log_buffer(_handle);
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != IntPtr.Zero)
                {
                    NativeMethods.dfx_destroy(_handle);
                    _handle = IntPtr.Zero;
                }
                _disposed = true;
            }
        }

        private static class NativeMethods
        {
            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern IntPtr dfx_create();

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_destroy(IntPtr system);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_enable_all(IntPtr system);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_set_log_level(IntPtr system, byte level);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_log(
                IntPtr system,
                byte level,
                string module,
                string message,
                string file,
                uint line);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_trace_begin(IntPtr system, string name, string category);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_trace_end(IntPtr system, string name, string category);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern float dfx_get_fps(IntPtr system);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern ulong dfx_get_frame_count(IntPtr system);

            [DllImport("hezhou_engine", CallingConvention = CallingConvention.Cdecl)]
            public static extern void dfx_clear_log_buffer(IntPtr system);
        }
    }
}