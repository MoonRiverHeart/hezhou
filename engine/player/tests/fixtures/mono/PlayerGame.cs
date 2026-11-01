using System;
using System.Globalization;
using System.IO;

namespace Hezhou
{
    /// <summary>
    /// Player C# 入口测试 fixture：与 task-30 的 Hezhou.PlayerGame 签名一致。
    /// 通过环境变量 HEZHOU_PLAYER_MONO_LOG 追加调用记录，供 Rust 测试断言。
    /// </summary>
    public static class PlayerGame
    {
        private static int _updateCount;

        private static string LogPath()
        {
            return Environment.GetEnvironmentVariable("HEZHOU_PLAYER_MONO_LOG");
        }

        private static void Log(string message)
        {
            string path = LogPath();
            if (!string.IsNullOrEmpty(path))
            {
                File.AppendAllText(path, message + Environment.NewLine);
            }
        }

        public static void Initialize(IntPtr context)
        {
            Log("Initialize context=" + context.ToInt64().ToString(CultureInfo.InvariantCulture));
        }

        public static void Update(float dt)
        {
            _updateCount++;
            Log("Update #" + _updateCount.ToString(CultureInfo.InvariantCulture)
                + " dt=" + dt.ToString("R", CultureInfo.InvariantCulture));
        }

        public static void Shutdown()
        {
            Log("Shutdown updates=" + _updateCount.ToString(CultureInfo.InvariantCulture));
        }
    }
}
