// PlayerGame.cs — Hezhou Player 的 C# 游戏入口（0.2 task-30 / task-32）
//
// 用途：Player（Rust 侧 Mono host）在加载 C# 程序集后，通过反射/静态调用：
//   Hezhou.PlayerGame.Initialize(IntPtr context)  // PlayerCSharpContext（Scene 操作 API）
//   Hezhou.PlayerGame.Update(float dt)            // 每帧
//   Hezhou.PlayerGame.Shutdown()                  // 退出
//
// PlayerContext ABI（Rust `hezhou-player::csharp_context::PlayerCSharpContext`）：
//   version(u32) + reserved(u32) + scene(IntPtr) + 6 个函数指针
//   scene_entity_count / scene_create_cube / scene_create_entity /
//   scene_set_entity_position / scene_get_entity_position / log
// context == 0 时保持安全空转（旧 Player/无 Scene 环境）。
//
// 约定：
// - 不依赖 UI/FfiContext/EditorScript，确保 Player 程序集可独立加载；
// - 所有输出走 Console.Error（Player 控制台/日志），任何日志失败都不能抛出异常；
// - Initialize/Shutdown 幂等；Update 在未初始化时安全跳过；
// - C# 不得在 Shutdown 后继续持有/调用 context.scene 指针（Rust 只在帧循环内保活）；
// - 后续 gameplay/animation/audio API 按同一 ABI 追加函数指针（版本号递增）。

using System;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;

namespace Hezhou
{
    /// <summary>Player C# 游戏入口（Rust Player host 静态调用）。</summary>
    public static class PlayerGame
    {
        private const uint PlayerContextVersion = 1;
        private const float CubeX = 2.5f;
        private const float CubeY = 0.5f;
        private const float CubeZ = -1.5f;

        private static bool _initialized;
        private static ContextApi _sceneContext;
        private static ulong _createdEntityId;
        private static uint _lastEntityCount;
        private static float _lastX;
        private static float _lastY;
        private static float _lastZ;
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

        /// <summary>最近一次通过 context 读取的 Scene 实体数（无 context 时为 0）。</summary>
        public static uint LastEntityCount
        {
            get { return _lastEntityCount; }
        }

        /// <summary>
        /// 初始化 Player 游戏入口；context 非 0 时解析 PlayerCSharpContext 并通过它操作 Scene。
        /// </summary>
        public static void Initialize(IntPtr context)
        {
            if (_initialized)
            {
                SafeLog("PlayerGame: 已初始化，忽略重复 Initialize");
                return;
            }

            _initialized = true;
            _frameCount = 0;
            _elapsedSeconds = 0.0;
            _lastLogFrame = -1;
            _sceneContext = null;
            _createdEntityId = 0;
            _lastEntityCount = 0;
            _lastX = 0.0f;
            _lastY = 0.0f;
            _lastZ = 0.0f;

            if (context == IntPtr.Zero)
            {
                SafeLog("PlayerGame: Initialize 完成（context=0x0，无 Scene Context，空转）");
                return;
            }

            SafeLog("PlayerGame: Initialize 完成（context=0x"
                + context.ToInt64().ToString("X") + "）");

            try
            {
                string error;
                ContextApi api = ContextApi.TryParse(context, out error);
                if (api == null)
                {
                    SafeLog("SceneContext: 解析失败：" + error + "（回退纯 Rust/空转）");
                    return;
                }
                _sceneContext = api;

                uint before = api.EntityCount();
                _createdEntityId = api.CreateCube();
                int setResult = api.SetPosition(_createdEntityId, CubeX, CubeY, CubeZ);
                uint after = api.EntityCount();
                float x, y, z;
                int getResult = api.GetPosition(_createdEntityId, out x, out y, out z);
                _lastEntityCount = after;
                _lastX = x;
                _lastY = y;
                _lastZ = z;

                SafeLog("SceneContext: Initialize 创建 cube="
                    + _createdEntityId.ToString(CultureInfo.InvariantCulture)
                    + " setResult=" + setResult.ToString(CultureInfo.InvariantCulture)
                    + " getResult=" + getResult.ToString(CultureInfo.InvariantCulture)
                    + " entities " + before.ToString(CultureInfo.InvariantCulture)
                    + "->" + after.ToString(CultureInfo.InvariantCulture)
                    + " position=(" + Fmt(x) + "," + Fmt(y) + "," + Fmt(z) + ")");
            }
            catch (Exception exception)
            {
                _sceneContext = null;
                SafeLog("SceneContext: Initialize 异常：" + exception.Message
                    + "（回退纯 Rust/空转）");
            }
        }

        /// <summary>每帧更新（dt 秒；非法值按 0 处理）；有 context 时读取 Scene 状态。</summary>
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

            if (_sceneContext != null)
            {
                try
                {
                    _lastEntityCount = _sceneContext.EntityCount();
                    int getResult = _sceneContext.GetPosition(
                        _createdEntityId, out _lastX, out _lastY, out _lastZ);
                    // 首帧 + 每 60 帧输出一次，避免刷屏；Player 5 帧冒烟也能看到一次。
                    if (_frameCount == 1 || _frameCount % 60 == 0)
                    {
                        SafeLog("SceneContext: frame="
                            + _frameCount.ToString(CultureInfo.InvariantCulture)
                            + " entities=" + _lastEntityCount.ToString(CultureInfo.InvariantCulture)
                            + " cube=" + _createdEntityId.ToString(CultureInfo.InvariantCulture)
                            + " getResult=" + getResult.ToString(CultureInfo.InvariantCulture)
                            + " position=(" + Fmt(_lastX) + "," + Fmt(_lastY) + "," + Fmt(_lastZ) + ")");
                    }
                }
                catch (Exception exception)
                {
                    _sceneContext = null;
                    SafeLog("SceneContext: Update 读取异常：" + exception.Message
                        + "（停止 context 调用）");
                }
                return;
            }

            // 无 context：保留 task-30 的帧统计（每 60 帧一次）。
            if (_frameCount % 60 == 0 && _frameCount != _lastLogFrame)
            {
                _lastLogFrame = _frameCount;
                double averageDt = _frameCount > 0 ? _elapsedSeconds / _frameCount : 0.0;
                SafeLog("PlayerGame: frame=" + _frameCount
                    + " elapsed=" + _elapsedSeconds.ToString("F3") + "s"
                    + " avgDt=" + averageDt.ToString("F6") + "s");
            }
        }

        /// <summary>退出：有 context 时读取最终 Scene 状态，输出统计并重置（幂等）。</summary>
        public static void Shutdown()
        {
            if (!_initialized)
            {
                SafeLog("PlayerGame: 未初始化，忽略 Shutdown");
                return;
            }

            if (_sceneContext != null)
            {
                try
                {
                    uint count = _sceneContext.EntityCount();
                    float x, y, z;
                    int getResult = _sceneContext.GetPosition(_createdEntityId, out x, out y, out z);
                    _lastEntityCount = count;
                    _lastX = x;
                    _lastY = y;
                    _lastZ = z;
                    SafeLog("SceneContext: Shutdown entities="
                        + count.ToString(CultureInfo.InvariantCulture)
                        + " cube=" + _createdEntityId.ToString(CultureInfo.InvariantCulture)
                        + " getResult=" + getResult.ToString(CultureInfo.InvariantCulture)
                        + " position=(" + Fmt(x) + "," + Fmt(y) + "," + Fmt(z) + ")");
                }
                catch (Exception exception)
                {
                    SafeLog("SceneContext: Shutdown 读取异常：" + exception.Message);
                }
            }

            SafeLog("PlayerGame: Shutdown（frames=" + _frameCount
                + " elapsed=" + _elapsedSeconds.ToString("F3") + "s）");
            _initialized = false;
            _sceneContext = null;
            _createdEntityId = 0;
            _frameCount = 0;
            _elapsedSeconds = 0.0;
            _lastLogFrame = -1;
        }

        private static string Fmt(float value)
        {
            return value.ToString("F3", CultureInfo.InvariantCulture);
        }

        /// <summary>日志安全包装：任何输出异常都不允许中断 Player。</summary>
        private static void SafeLog(string message)
        {
            try
            {
                Console.Error.WriteLine("[PlayerGame] " + message);
                // 同步追加到 HEZHOU_PLAYER_MONO_LOG（如果 Rust host 提供），便于自动化断言。
                string logPath = Environment.GetEnvironmentVariable("HEZHOU_PLAYER_MONO_LOG");
                if (!string.IsNullOrEmpty(logPath))
                {
                    File.AppendAllText(logPath, message + Environment.NewLine);
                }
            }
            catch (Exception)
            {
                // 无控制台/流已关闭时静默忽略（Player 不可因日志崩溃）。
            }
        }

        // ==== PlayerCSharpContext ABI（必须与 Rust repr(C) 布局一致）====

        /// <summary>Rust `PlayerCSharpContext` 的 C# 镜像（9 个 8 字节槽）。</summary>
        [StructLayout(LayoutKind.Sequential)]
        private struct PlayerCSharpContextNative
        {
            public uint Version;
            public uint Reserved;
            public IntPtr Scene;
            public IntPtr SceneEntityCount;
            public IntPtr SceneCreateCube;
            public IntPtr SceneCreateEntity;
            public IntPtr SceneSetEntityPosition;
            public IntPtr SceneGetEntityPosition;
            public IntPtr Log;
        }

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate uint SceneEntityCountDelegate(IntPtr scene);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate ulong SceneCreateCubeDelegate(IntPtr scene);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate ulong SceneCreateEntityDelegate(IntPtr scene);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneSetEntityPositionDelegate(
            IntPtr scene, ulong entityId, float x, float y, float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneGetEntityPositionDelegate(
            IntPtr scene, ulong entityId, out float x, out float y, out float z);

        /// <summary>已解析的 PlayerContext（函数指针 + scene）；GC 期间保持委托存活。</summary>
        private sealed class ContextApi
        {
            public IntPtr Scene;
            public SceneEntityCountDelegate EntityCountFn;
            public SceneCreateCubeDelegate CreateCubeFn;
            public SceneCreateEntityDelegate CreateEntityFn;
            public SceneSetEntityPositionDelegate SetPositionFn;
            public SceneGetEntityPositionDelegate GetPositionFn;

            public uint EntityCount()
            {
                return EntityCountFn(Scene);
            }

            public ulong CreateCube()
            {
                return CreateCubeFn(Scene);
            }

            public ulong CreateEntity()
            {
                return CreateEntityFn(Scene);
            }

            public int SetPosition(ulong entityId, float x, float y, float z)
            {
                return SetPositionFn(Scene, entityId, x, y, z);
            }

            public int GetPosition(ulong entityId, out float x, out float y, out float z)
            {
                return GetPositionFn(Scene, entityId, out x, out y, out z);
            }

            public static ContextApi TryParse(IntPtr context, out string error)
            {
                error = null;
                PlayerCSharpContextNative native;
                try
                {
                    native = (PlayerCSharpContextNative)Marshal.PtrToStructure(
                        context, typeof(PlayerCSharpContextNative));
                }
                catch (Exception exception)
                {
                    error = "PtrToStructure 失败：" + exception.Message;
                    return null;
                }

                if (native.Version != PlayerContextVersion)
                {
                    error = "ABI 版本不匹配：native=" + native.Version
                        + " expected=" + PlayerContextVersion;
                    return null;
                }
                if (native.Scene == IntPtr.Zero)
                {
                    error = "scene 指针为 0";
                    return null;
                }
                if (native.SceneEntityCount == IntPtr.Zero
                    || native.SceneCreateCube == IntPtr.Zero
                    || native.SceneCreateEntity == IntPtr.Zero
                    || native.SceneSetEntityPosition == IntPtr.Zero
                    || native.SceneGetEntityPosition == IntPtr.Zero)
                {
                    error = "函数指针缺失";
                    return null;
                }

                try
                {
                    return new ContextApi
                    {
                        Scene = native.Scene,
                        EntityCountFn = Marshal.GetDelegateForFunctionPointer<SceneEntityCountDelegate>(
                            native.SceneEntityCount),
                        CreateCubeFn = Marshal.GetDelegateForFunctionPointer<SceneCreateCubeDelegate>(
                            native.SceneCreateCube),
                        CreateEntityFn = Marshal.GetDelegateForFunctionPointer<SceneCreateEntityDelegate>(
                            native.SceneCreateEntity),
                        SetPositionFn = Marshal.GetDelegateForFunctionPointer<SceneSetEntityPositionDelegate>(
                            native.SceneSetEntityPosition),
                        GetPositionFn = Marshal.GetDelegateForFunctionPointer<SceneGetEntityPositionDelegate>(
                            native.SceneGetEntityPosition),
                    };
                }
                catch (Exception exception)
                {
                    error = "GetDelegateForFunctionPointer 失败：" + exception.Message;
                    return null;
                }
            }
        }
    }
}
