// PlayerGame.cs — Hezhou Player 的 C# 游戏入口（0.2 task-30）
//
// 用途：Player（Rust 侧 Mono host）在加载 C# 程序集后，通过反射/静态调用：
//   Hezhou.PlayerGame.Initialize(IntPtr context)  // Player 上下文（本轮忽略，保留签名）
//   Hezhou.PlayerGame.Update(float dt)            // 每帧
//   Hezhou.PlayerGame.Shutdown()                  // 退出
//
// 约定：
// - 不依赖 UI/FfiContext/EditorScript，确保 Player 程序集可独立加载；
// - 所有输出走 Console.Error（Player 控制台/日志），任何日志失败都不能抛出异常；
// - Initialize/Shutdown 幂等；Update 在未初始化时安全跳过；
// - 后续 gameplay/animation API 通过 context（Player API 指针）或 FFI 暴露，
//   届时替换 Initialize 中保存的 _context 即可，签名保持稳定。

using System;

namespace Hezhou
{
    /// <summary>Player C# 游戏入口（Rust Player host 静态调用）。</summary>
    public static class PlayerGame
    {
        private static bool _initialized;
        private static IntPtr _context = IntPtr.Zero;
        private static long _frameCount;
        private static double _elapsedSeconds;
        private static long _lastLogFrame = -1;

        /// <summary>是否已初始化（调试/自检用）。</summary>
        public static bool IsInitialized
        {
            get { return _initialized; }
        }

        /// <summary>已执行的 Update 帧数（调试/自检用）。</summary>
        public static long FrameCount
        {
            get { return _frameCount; }
        }

        /// <summary>累计游戏时间（秒，调试/自检用）。</summary>
        public static double ElapsedSeconds
        {
            get { return _elapsedSeconds; }
        }

        /// <summary>
        /// 初始化 Player 游戏入口。
        /// </summary>
        /// <param name="context">
        /// Player 上下文指针（本轮忽略；后续用于暴露 gameplay/animation API）。
        /// </param>
        public static void Initialize(IntPtr context)
        {
            if (_initialized)
            {
                SafeLog("PlayerGame: 已初始化，忽略重复 Initialize");
                return;
            }

            _context = context;
            _initialized = true;
            _frameCount = 0;
            _elapsedSeconds = 0.0;
            _lastLogFrame = -1;
            SafeLog("PlayerGame: Initialize 完成（context=0x"
                + context.ToInt64().ToString("X") + "）");
        }

        /// <summary>每帧更新（dt 秒；非法值按 0 处理）。</summary>
        public static void Update(float dt)
        {
            if (!_initialized)
            {
                return;
            }
            if (float.IsNaN(dt) || float.IsInfinity(dt) || dt < 0.0f)
            {
                dt = 0.0f;
            }

            _frameCount++;
            _elapsedSeconds += dt;

            // 每 60 帧输出一次简单状态，避免刷屏；日志失败不影响游戏循环。
            if (_frameCount % 60 == 0 && _frameCount != _lastLogFrame)
            {
                _lastLogFrame = _frameCount;
                double averageDt = _frameCount > 0 ? _elapsedSeconds / _frameCount : 0.0;
                SafeLog("PlayerGame: frame=" + _frameCount
                    + " elapsed=" + _elapsedSeconds.ToString("F3") + "s"
                    + " avgDt=" + averageDt.ToString("F6") + "s");
            }
        }

        /// <summary>退出：输出统计并重置状态（幂等）。</summary>
        public static void Shutdown()
        {
            if (!_initialized)
            {
                SafeLog("PlayerGame: 未初始化，忽略 Shutdown");
                return;
            }

            SafeLog("PlayerGame: Shutdown（frames=" + _frameCount
                + " elapsed=" + _elapsedSeconds.ToString("F3") + "s）");
            _initialized = false;
            _context = IntPtr.Zero;
            _frameCount = 0;
            _elapsedSeconds = 0.0;
            _lastLogFrame = -1;
        }

        /// <summary>日志安全包装：任何输出异常都不允许中断 Player。</summary>
        private static void SafeLog(string message)
        {
            try
            {
                Console.Error.WriteLine("[PlayerGame] " + message);
            }
            catch (Exception)
            {
                // 无控制台/流已关闭时静默忽略（Player 不可因日志崩溃）。
            }
        }
    }
}
