using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public static partial class UI
    {
        // === Pipeline FFI Bridge Methods ===

        // 获取可用管线名列表 — 返回string[]
        public static string[] GetPipelineNames()
        {
            if (_ffi.get_pipeline_names == IntPtr.Zero) return new string[0];
            IntPtr buffer = Marshal.AllocHGlobal(1024);
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<GetPipelineNamesDelegate>(_ffi.get_pipeline_names);
                ulong count = func(_widgetTree, buffer, 1024);
                if (count == 0) return new string[0];
                // PtrToStringAnsi只读到第一个\0就停止，但buffer用\0分隔多个字符串
                // 需要手动读取count个字节再Split('\0')
                byte[] bytes = new byte[count];
                Marshal.Copy(buffer, bytes, 0, (int)count);
                string joined = System.Text.Encoding.ASCII.GetString(bytes);
                Log.Info("Pipeline", "GetPipelineNames: count=" + count + " joined_len=" + joined.Length + " joined=[" + joined.Replace('\0', '|') + "]");
                if (joined == null || joined.Length == 0) return new string[0];
                string[] result = joined.Split('\0');
                Log.Info("Pipeline", "GetPipelineNames: split_count=" + result.Length);
                return result;
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        // 切换管线 — 返回true=成功
        public static bool SwitchPipeline(string pipelineName)
        {
            if (_ffi.switch_pipeline == IntPtr.Zero) return false;
            var func = Marshal.GetDelegateForFunctionPointer<SwitchPipelineDelegate>(_ffi.switch_pipeline);
            int result = func(_widgetTree, pipelineName);
            return result == 0;
        }

        // 获取当前活跃管线名
        public static string GetActivePipelineName()
        {
            if (_ffi.get_active_pipeline_name == IntPtr.Zero) return "raster";
            IntPtr buffer = Marshal.AllocHGlobal(256);
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<GetActivePipelineNameDelegate>(_ffi.get_active_pipeline_name);
                ulong len = func(_widgetTree, buffer, 256);
                if (len == 0) return "raster";
                string name = Marshal.PtrToStringAnsi(buffer);
                return name ?? "raster";
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }
    }
}