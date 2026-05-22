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

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern PerformanceSnapshot dfx_get_perf_snapshot(IntPtr system);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_set_counter(IntPtr system, string name, string category, long value);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_set_draw_calls(IntPtr system, uint count);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_set_triangle_count(IntPtr system, uint count);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_perf_begin_frame(IntPtr system);

        [DllImport("hezhou_dfx", CallingConvention = CallingConvention.Cdecl)]
        public static extern void dfx_perf_end_frame(IntPtr system);
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct PerformanceSnapshot
    {
        public ulong timestamp;
        public float fps;
        public float frame_time_ms;
        public float cpu_usage_percent;
        public float memory_used_mb;
        public float memory_available_mb;
        public uint draw_calls;
        public uint triangle_count;
    }

    public static class Log
    {
        private static IntPtr _dfxHandle = IntPtr.Zero;
        private static DfxLogDelegate _dfxLog = null;
        private static DfxTraceBeginDelegate _dfxTraceBegin = null;
        private static DfxTraceEndDelegate _dfxTraceEnd = null;
        private static DfxSetCounterDelegate _dfxSetCounter = null;
        private static DfxPerfBeginFrameDelegate _dfxPerfBeginFrame = null;
        private static DfxPerfEndFrameDelegate _dfxPerfEndFrame = null;

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void DfxLogDelegate(IntPtr system, byte level, string module, string message, string file, uint line);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void DfxTraceBeginDelegate(IntPtr system, string name, string category);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void DfxTraceEndDelegate(IntPtr system, string name, string category);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void DfxSetCounterDelegate(IntPtr system, string name, string category, long value);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void DfxPerfBeginFrameDelegate(IntPtr system);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate void DfxPerfEndFrameDelegate(IntPtr system);

        public static void Init(IntPtr dfxHandle)
        {
            _dfxHandle = dfxHandle;
        }

        public static void SetFunctionPointers(IntPtr logPtr, IntPtr traceBeginPtr, IntPtr traceEndPtr)
        {
            if (logPtr != IntPtr.Zero)
                _dfxLog = Marshal.GetDelegateForFunctionPointer<DfxLogDelegate>(logPtr);
            if (traceBeginPtr != IntPtr.Zero)
                _dfxTraceBegin = Marshal.GetDelegateForFunctionPointer<DfxTraceBeginDelegate>(traceBeginPtr);
            if (traceEndPtr != IntPtr.Zero)
                _dfxTraceEnd = Marshal.GetDelegateForFunctionPointer<DfxTraceEndDelegate>(traceEndPtr);
        }

        public static void SetPerfFunctionPointers(IntPtr setCounterPtr, IntPtr perfBeginFramePtr, IntPtr perfEndFramePtr)
        {
            if (setCounterPtr != IntPtr.Zero)
                _dfxSetCounter = Marshal.GetDelegateForFunctionPointer<DfxSetCounterDelegate>(setCounterPtr);
            if (perfBeginFramePtr != IntPtr.Zero)
                _dfxPerfBeginFrame = Marshal.GetDelegateForFunctionPointer<DfxPerfBeginFrameDelegate>(perfBeginFramePtr);
            if (perfEndFramePtr != IntPtr.Zero)
                _dfxPerfEndFrame = Marshal.GetDelegateForFunctionPointer<DfxPerfEndFrameDelegate>(perfEndFramePtr);
        }

        private static void CallDfxLog(byte level, string module, string message)
        {
            if (_dfxLog != null && _dfxHandle != IntPtr.Zero)
                _dfxLog(_dfxHandle, level, module, message, "", 0);
            else if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_log(_dfxHandle, level, module, message, "", 0);
            else
                Console.WriteLine($"[{level}][{module}] {message}");
        }

        public static void Trace(string module, string message)
        {
            CallDfxLog((byte)LogLevel.Trace, module, message);
        }

        public static void Debug(string module, string message)
        {
            CallDfxLog((byte)LogLevel.Debug, module, message);
        }

        public static void Info(string module, string message)
        {
            CallDfxLog((byte)LogLevel.Info, module, message);
        }

        public static void Warn(string module, string message)
        {
            CallDfxLog((byte)LogLevel.Warn, module, message);
        }

        public static void Error(string module, string message)
        {
            CallDfxLog((byte)LogLevel.Error, module, message);
        }

        public static void Fatal(string module, string message)
        {
            CallDfxLog((byte)LogLevel.Fatal, module, message);
        }

        public static void TraceBegin(string name, string category = "ui")
        {
            if (_dfxTraceBegin != null && _dfxHandle != IntPtr.Zero)
                _dfxTraceBegin(_dfxHandle, name, category);
            else if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_trace_begin(_dfxHandle, name, category);
        }

        public static void TraceEnd(string name, string category = "ui")
        {
            if (_dfxTraceEnd != null && _dfxHandle != IntPtr.Zero)
                _dfxTraceEnd(_dfxHandle, name, category);
            else if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_trace_end(_dfxHandle, name, category);
        }

        public static void SetCounter(string name, string category, long value)
        {
            if (_dfxSetCounter != null && _dfxHandle != IntPtr.Zero)
                _dfxSetCounter(_dfxHandle, name, category, value);
            else if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_set_counter(_dfxHandle, name, category, value);
        }

        public static void PerfBeginFrame()
        {
            if (_dfxPerfBeginFrame != null && _dfxHandle != IntPtr.Zero)
                _dfxPerfBeginFrame(_dfxHandle);
            else if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_perf_begin_frame(_dfxHandle);
        }

        public static void PerfEndFrame()
        {
            if (_dfxPerfEndFrame != null && _dfxHandle != IntPtr.Zero)
                _dfxPerfEndFrame(_dfxHandle);
            else if (_dfxHandle != IntPtr.Zero)
                DfxNativeMethods.dfx_perf_end_frame(_dfxHandle);
        }

        public static PerformanceSnapshot GetPerfSnapshot()
        {
            if (_dfxHandle != IntPtr.Zero)
                return DfxNativeMethods.dfx_get_perf_snapshot(_dfxHandle);
            return new PerformanceSnapshot();
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

        public PerformanceSnapshot GetPerfSnapshot()
        {
            return DfxNativeMethods.dfx_get_perf_snapshot(_handle);
        }

        public void SetCounter(string name, string category, long value)
        {
            DfxNativeMethods.dfx_set_counter(_handle, name, category, value);
        }

        public void PerfBeginFrame()
        {
            DfxNativeMethods.dfx_perf_begin_frame(_handle);
        }

        public void PerfEndFrame()
        {
            DfxNativeMethods.dfx_perf_end_frame(_handle);
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