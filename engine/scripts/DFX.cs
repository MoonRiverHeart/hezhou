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

    internal static class DfxNativeMethods
    {
        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr dfx_create();

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_destroy(IntPtr system);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_enable_all(IntPtr system);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_set_log_level(IntPtr system, byte level);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_log(
            IntPtr system,
            byte level,
            string module,
            string message,
            string file,
            uint line);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_trace_begin(IntPtr system, string name, string category);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_trace_end(IntPtr system, string name, string category);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern float dfx_get_fps(IntPtr system);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern ulong dfx_get_frame_count(IntPtr system);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_clear_log_buffer(IntPtr system);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern int dfx_enable_file_output(IntPtr system, string path);
    }

    public static class Log
    {
        private static IntPtr _dfxHandle = IntPtr.Zero;

        public static void Init(IntPtr dfxHandle)
        {
            _dfxHandle = dfxHandle;
        }

        public static void Trace(string module, string message)
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_log(_dfxHandle, (byte)LogLevel.Trace, module, message, "", 0);
            else
                Console.WriteLine($"[TRACE][{module}] {message}");
        }

        public static void Debug(string module, string message)
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_log(_dfxHandle, (byte)LogLevel.Debug, module, message, "", 0);
            else
                Console.WriteLine($"[DEBUG][{module}] {message}");
        }

        public static void Info(string module, string message)
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_log(_dfxHandle, (byte)LogLevel.Info, module, message, "", 0);
            else
                Console.WriteLine($"[INFO][{module}] {message}");
        }

        public static void Warn(string module, string message)
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_log(_dfxHandle, (byte)LogLevel.Warn, module, message, "", 0);
            else
                Console.WriteLine($"[WARN][{module}] {message}");
        }

        public static void Error(string module, string message)
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_log(_dfxHandle, (byte)LogLevel.Error, module, message, "", 0);
            else
                Console.WriteLine($"[ERROR][{module}] {message}");
        }

        public static void Fatal(string module, string message)
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_log(_dfxHandle, (byte)LogLevel.Fatal, module, message, "", 0);
            else
                Console.WriteLine($"[FATAL][{module}] {message}");
        }

        public static void TraceBegin(string name, string category = "ui")
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_trace_begin(_dfxHandle, name, category);
        }

        public static void TraceEnd(string name, string category = "ui")
        {
            if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_trace_end(_dfxHandle, name, category);
        }
    }

    public class DFX : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        public static DFX Create()
        {
            var handle = DfxNativeMethods.dfx_create();
            return new DFX(handle);
        }

        private DFX(IntPtr handle)
        {
            _handle = handle;
            Log.Init(handle);
        }

        public void EnableAll()
        {
            DfxNativeMethods.dfx_enable_all(_handle);
        }

        public void SetLogLevel(LogLevel level)
        {
            DfxNativeMethods.dfx_set_log_level(_handle, (byte)level);
        }

        public void EnableFileOutput(string path)
        {
            DfxNativeMethods.dfx_enable_file_output(_handle, path);
        }

        public void LogMsg(string message, LogLevel level = LogLevel.Info, string module = "UIScript")
        {
            DfxNativeMethods.dfx_log(_handle, (byte)level, module, message, "DFX.cs", 0);
        }

        public void TraceBegin(string name, string category = "ui")
        {
            DfxNativeMethods.dfx_trace_begin(_handle, name, category);
        }

        public void TraceEnd(string name, string category = "ui")
        {
            DfxNativeMethods.dfx_trace_end(_handle, name, category);
        }

        public float GetFPS()
        {
            return DfxNativeMethods.dfx_get_fps(_handle);
        }

        public ulong GetFrameCount()
        {
            return DfxNativeMethods.dfx_get_frame_count(_handle);
        }

        public void ClearLogBuffer()
        {
            DfxNativeMethods.dfx_clear_log_buffer(_handle);
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != IntPtr.Zero)
                {
                    DfxNativeMethods.dfx_destroy(_handle);
                    _handle = IntPtr.Zero;
                }
                _disposed = true;
            }
        }
    }
}