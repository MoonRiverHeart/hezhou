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
        private const uint PlayerContextVersion = 3;
        private const float CubeX = 2.5f;
        private const float CubeY = 0.5f;
        private const float CubeZ = -1.5f;
        private const float CubeRotationY = 90.0f;
        private const float CubeScale = 1.5f;
        private const float InitialAnimationTime = 0.25f;
        private const float InitialAnimationSpeed = 1.5f;

        private static bool _initialized;
        private static ContextApi _sceneContext;
        private static ulong _createdEntityId;
        private static uint _lastEntityCount;
        private static float _lastX;
        private static float _lastY;
        private static float _lastZ;
        private static float _lastRotationX;
        private static float _lastRotationY;
        private static float _lastRotationZ;
        private static float _lastScaleX = 1.0f;
        private static float _lastScaleY = 1.0f;
        private static float _lastScaleZ = 1.0f;
        private static int _lastAnimationClip = -1;
        private static float _lastAnimationTime;
        private static float _lastAnimationSpeed = 1.0f;
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
            _lastRotationX = 0.0f;
            _lastRotationY = 0.0f;
            _lastRotationZ = 0.0f;
            _lastScaleX = 1.0f;
            _lastScaleY = 1.0f;
            _lastScaleZ = 1.0f;
            _lastAnimationClip = -1;
            _lastAnimationTime = 0.0f;
            _lastAnimationSpeed = 1.0f;

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

                // v2：Transform rotation（欧拉角，度）/scale；v1 兼容时跳过。
                int rotationSet = -1;
                int rotationGet = -1;
                int scaleSet = -1;
                int scaleGet = -1;
                float rx = 0.0f, ry = 0.0f, rz = 0.0f;
                float sx = 1.0f, sy = 1.0f, sz = 1.0f;
                if (api.HasTransform)
                {
                    rotationSet = api.SetRotation(
                        _createdEntityId, 0.0f, CubeRotationY, 0.0f);
                    scaleSet = api.SetScale(
                        _createdEntityId, CubeScale, CubeScale, CubeScale);
                    rotationGet = api.GetRotation(
                        _createdEntityId, out rx, out ry, out rz);
                    scaleGet = api.GetScale(
                        _createdEntityId, out sx, out sy, out sz);
                    _lastRotationX = rx;
                    _lastRotationY = ry;
                    _lastRotationZ = rz;
                    _lastScaleX = sx;
                    _lastScaleY = sy;
                    _lastScaleZ = sz;
                }

                SafeLog("SceneContext: Initialize 创建 cube="
                    + _createdEntityId.ToString(CultureInfo.InvariantCulture)
                    + " setResult=" + setResult.ToString(CultureInfo.InvariantCulture)
                    + " getResult=" + getResult.ToString(CultureInfo.InvariantCulture)
                    + " entities " + before.ToString(CultureInfo.InvariantCulture)
                    + "->" + after.ToString(CultureInfo.InvariantCulture)
                    + " position=(" + Fmt(x) + "," + Fmt(y) + "," + Fmt(z) + ")"
                    + " rotationSet=" + rotationSet.ToString(CultureInfo.InvariantCulture)
                    + " rotationGet=" + rotationGet.ToString(CultureInfo.InvariantCulture)
                    + " rotation=(" + Fmt(rx) + "," + Fmt(ry) + "," + Fmt(rz) + ")"
                    + " scaleSet=" + scaleSet.ToString(CultureInfo.InvariantCulture)
                    + " scaleGet=" + scaleGet.ToString(CultureInfo.InvariantCulture)
                    + " scale=(" + Fmt(sx) + "," + Fmt(sy) + "," + Fmt(sz) + ")");

                // v3：枚举 clip、播放 clip 0、设置 time/speed（v1/v2 context 跳过）。
                if (api.HasAnimation)
                {
                    uint clipCount = api.AnimationClipCount();
                    string clipList = "";
                    for (uint index = 0; index < clipCount; index++)
                    {
                        float duration = api.AnimationClipDuration(index);
                        string name = ReadClipName(api, index);
                        if (index > 0)
                        {
                            clipList += ", ";
                        }
                        clipList += index.ToString(CultureInfo.InvariantCulture)
                            + ":" + name + "(" + Fmt(duration) + "s)";
                    }
                    SafeLog("Animation: clips="
                        + clipCount.ToString(CultureInfo.InvariantCulture)
                        + " [" + clipList + "]");
                    if (clipCount > 0)
                    {
                        int playResult = api.AnimationPlay(0);
                        int timeResult = api.AnimationSetTime(InitialAnimationTime);
                        int speedResult = api.AnimationSetSpeed(InitialAnimationSpeed);
                        _lastAnimationClip = api.AnimationCurrent();
                        _lastAnimationTime = api.AnimationGetTime();
                        _lastAnimationSpeed = api.AnimationGetSpeed();
                        SafeLog("Animation: play clip="
                            + _lastAnimationClip.ToString(CultureInfo.InvariantCulture)
                            + " playResult=" + playResult.ToString(CultureInfo.InvariantCulture)
                            + " setTimeResult=" + timeResult.ToString(CultureInfo.InvariantCulture)
                            + " time=" + Fmt(_lastAnimationTime)
                            + " setSpeedResult=" + speedResult.ToString(CultureInfo.InvariantCulture)
                            + " speed=" + Fmt(_lastAnimationSpeed));
                    }
                }
                else
                {
                    SafeLog("Animation: context 无 v3 动画 API（v1/v2 DLL 或 Player 未提供动画状态）");
                }
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
                    int rotationResult = -1;
                    int scaleResult = -1;
                    if (_sceneContext.HasTransform)
                    {
                        rotationResult = _sceneContext.GetRotation(
                            _createdEntityId,
                            out _lastRotationX,
                            out _lastRotationY,
                            out _lastRotationZ);
                        scaleResult = _sceneContext.GetScale(
                            _createdEntityId,
                            out _lastScaleX,
                            out _lastScaleY,
                            out _lastScaleZ);
                    }
                    // 首帧 + 每 60 帧输出一次，避免刷屏；Player 5 帧冒烟也能看到一次。
                    if (_frameCount == 1 || _frameCount % 60 == 0)
                    {
                        SafeLog("SceneContext: frame="
                            + _frameCount.ToString(CultureInfo.InvariantCulture)
                            + " entities=" + _lastEntityCount.ToString(CultureInfo.InvariantCulture)
                            + " cube=" + _createdEntityId.ToString(CultureInfo.InvariantCulture)
                            + " exists=" + (_sceneContext.HasTransform
                                ? ExistsText(_sceneContext, _createdEntityId)
                                : "n/a")
                            + " getResult=" + getResult.ToString(CultureInfo.InvariantCulture)
                            + " position=(" + Fmt(_lastX) + "," + Fmt(_lastY) + "," + Fmt(_lastZ) + ")"
                            + " rotationResult=" + rotationResult.ToString(CultureInfo.InvariantCulture)
                            + " rotation=(" + Fmt(_lastRotationX) + "," + Fmt(_lastRotationY)
                            + "," + Fmt(_lastRotationZ) + ")"
                            + " scaleResult=" + scaleResult.ToString(CultureInfo.InvariantCulture)
                            + " scale=(" + Fmt(_lastScaleX) + "," + Fmt(_lastScaleY)
                            + "," + Fmt(_lastScaleZ) + ")");
                    }
                    if (_sceneContext.HasAnimation)
                    {
                        _lastAnimationClip = _sceneContext.AnimationCurrent();
                        _lastAnimationTime = _sceneContext.AnimationGetTime();
                        _lastAnimationSpeed = _sceneContext.AnimationGetSpeed();
                        if (_frameCount == 1 || _frameCount % 60 == 0)
                        {
                            SafeLog("Animation: frame="
                                + _frameCount.ToString(CultureInfo.InvariantCulture)
                                + " current=" + _lastAnimationClip.ToString(CultureInfo.InvariantCulture)
                                + " time=" + Fmt(_lastAnimationTime)
                                + " speed=" + Fmt(_lastAnimationSpeed));
                        }
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

                    bool existsBefore = false;
                    bool existsAfter = false;
                    int destroyResult = -1;
                    if (_sceneContext.HasTransform)
                    {
                        existsBefore = _sceneContext.EntityExists(_createdEntityId) != 0;
                        if (existsBefore)
                        {
                            destroyResult = _sceneContext.DestroyEntity(_createdEntityId);
                        }
                        existsAfter = _sceneContext.EntityExists(_createdEntityId) != 0;
                        count = _sceneContext.EntityCount();
                        _lastEntityCount = count;
                    }

                    SafeLog("SceneContext: Shutdown entities="
                        + count.ToString(CultureInfo.InvariantCulture)
                        + " cube=" + _createdEntityId.ToString(CultureInfo.InvariantCulture)
                        + " existsBefore=" + BoolText(existsBefore)
                        + " destroyResult=" + destroyResult.ToString(CultureInfo.InvariantCulture)
                        + " existsAfter=" + BoolText(existsAfter)
                        + " getResult=" + getResult.ToString(CultureInfo.InvariantCulture)
                        + " position=(" + Fmt(x) + "," + Fmt(y) + "," + Fmt(z) + ")");
                    if (_sceneContext.HasAnimation)
                    {
                        SafeLog("Animation: Shutdown current="
                            + _sceneContext.AnimationCurrent().ToString(CultureInfo.InvariantCulture)
                            + " time=" + Fmt(_sceneContext.AnimationGetTime())
                            + " speed=" + Fmt(_sceneContext.AnimationGetSpeed()));
                    }
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

        private static string BoolText(bool value)
        {
            return value ? "1" : "0";
        }

        private static string ExistsText(ContextApi api, ulong entityId)
        {
            return BoolText(api.EntityExists(entityId) != 0);
        }

        private static string ReadClipName(ContextApi api, uint index)
        {
            int needed = api.AnimationClipName(index, IntPtr.Zero, 0);
            if (needed <= 0)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(needed);
            try
            {
                int written = api.AnimationClipName(index, buffer, (uint)needed);
                if (written <= 0)
                {
                    return "";
                }
                byte[] bytes = new byte[written];
                Marshal.Copy(buffer, bytes, 0, written);
                return System.Text.Encoding.UTF8.GetString(bytes);
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
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

        /// <summary>Rust `PlayerCSharpContext` 的 C# 镜像（v3：24 个 8 字节槽）。</summary>
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
            // v2 tail（task-33）。
            public IntPtr SceneSetEntityRotation;
            public IntPtr SceneGetEntityRotation;
            public IntPtr SceneSetEntityScale;
            public IntPtr SceneGetEntityScale;
            public IntPtr SceneEntityExists;
            public IntPtr SceneDestroyEntity;
            // v3 tail（task-34）：动画运行时。
            public IntPtr AnimationState;
            public IntPtr AnimationClipCount;
            public IntPtr AnimationClipName;
            public IntPtr AnimationClipDuration;
            public IntPtr AnimationCurrent;
            public IntPtr AnimationPlay;
            public IntPtr AnimationSetTime;
            public IntPtr AnimationGetTime;
            public IntPtr AnimationSetSpeed;
            public IntPtr AnimationGetSpeed;
            public IntPtr AnimationSetPlaying;
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

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneSetEntityRotationDelegate(
            IntPtr scene, ulong entityId, float x, float y, float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneGetEntityRotationDelegate(
            IntPtr scene, ulong entityId, out float x, out float y, out float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneSetEntityScaleDelegate(
            IntPtr scene, ulong entityId, float x, float y, float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneGetEntityScaleDelegate(
            IntPtr scene, ulong entityId, out float x, out float y, out float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneEntityExistsDelegate(IntPtr scene, ulong entityId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int SceneDestroyEntityDelegate(IntPtr scene, ulong entityId);

        // v3：动画运行时（task-34）。
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate uint AnimationClipCountDelegate(IntPtr state);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationClipNameDelegate(
            IntPtr state, uint index, IntPtr buffer, uint bufferLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate float AnimationClipDurationDelegate(IntPtr state, uint index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationCurrentDelegate(IntPtr state);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationPlayDelegate(IntPtr state, uint index);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationSetTimeDelegate(IntPtr state, float seconds);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate float AnimationGetTimeDelegate(IntPtr state);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationSetSpeedDelegate(IntPtr state, float speed);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate float AnimationGetSpeedDelegate(IntPtr state);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationSetPlayingDelegate(IntPtr state, int playing);

        /// <summary>已解析的 PlayerContext（函数指针 + scene）；GC 期间保持委托存活。</summary>
        private sealed class ContextApi
        {
            public IntPtr Scene;
            public bool HasTransform;
            public SceneEntityCountDelegate EntityCountFn;
            public SceneCreateCubeDelegate CreateCubeFn;
            public SceneCreateEntityDelegate CreateEntityFn;
            public SceneSetEntityPositionDelegate SetPositionFn;
            public SceneGetEntityPositionDelegate GetPositionFn;
            public SceneSetEntityRotationDelegate SetRotationFn;
            public SceneGetEntityRotationDelegate GetRotationFn;
            public SceneSetEntityScaleDelegate SetScaleFn;
            public SceneGetEntityScaleDelegate GetScaleFn;
            public SceneEntityExistsDelegate EntityExistsFn;
            public SceneDestroyEntityDelegate DestroyEntityFn;
            // v3 动画。
            public bool HasAnimation;
            public IntPtr AnimationState;
            public AnimationClipCountDelegate AnimationClipCountFn;
            public AnimationClipNameDelegate AnimationClipNameFn;
            public AnimationClipDurationDelegate AnimationClipDurationFn;
            public AnimationCurrentDelegate AnimationCurrentFn;
            public AnimationPlayDelegate AnimationPlayFn;
            public AnimationSetTimeDelegate AnimationSetTimeFn;
            public AnimationGetTimeDelegate AnimationGetTimeFn;
            public AnimationSetSpeedDelegate AnimationSetSpeedFn;
            public AnimationGetSpeedDelegate AnimationGetSpeedFn;
            public AnimationSetPlayingDelegate AnimationSetPlayingFn;

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

            public int SetRotation(ulong entityId, float x, float y, float z)
            {
                return SetRotationFn(Scene, entityId, x, y, z);
            }

            public int GetRotation(ulong entityId, out float x, out float y, out float z)
            {
                return GetRotationFn(Scene, entityId, out x, out y, out z);
            }

            public int SetScale(ulong entityId, float x, float y, float z)
            {
                return SetScaleFn(Scene, entityId, x, y, z);
            }

            public int GetScale(ulong entityId, out float x, out float y, out float z)
            {
                return GetScaleFn(Scene, entityId, out x, out y, out z);
            }

            public int EntityExists(ulong entityId)
            {
                return EntityExistsFn(Scene, entityId);
            }

            public int DestroyEntity(ulong entityId)
            {
                return DestroyEntityFn(Scene, entityId);
            }

            public uint AnimationClipCount()
            {
                return AnimationClipCountFn(AnimationState);
            }

            public int AnimationClipName(uint index, IntPtr buffer, uint bufferLength)
            {
                return AnimationClipNameFn(AnimationState, index, buffer, bufferLength);
            }

            public float AnimationClipDuration(uint index)
            {
                return AnimationClipDurationFn(AnimationState, index);
            }

            public int AnimationCurrent()
            {
                return AnimationCurrentFn(AnimationState);
            }

            public int AnimationPlay(uint index)
            {
                return AnimationPlayFn(AnimationState, index);
            }

            public int AnimationSetTime(float seconds)
            {
                return AnimationSetTimeFn(AnimationState, seconds);
            }

            public float AnimationGetTime()
            {
                return AnimationGetTimeFn(AnimationState);
            }

            public int AnimationSetSpeed(float speed)
            {
                return AnimationSetSpeedFn(AnimationState, speed);
            }

            public float AnimationGetSpeed()
            {
                return AnimationGetSpeedFn(AnimationState);
            }

            public int AnimationSetPlaying(bool playing)
            {
                return AnimationSetPlayingFn(AnimationState, playing ? 1 : 0);
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

                // v3 兼容策略：接受 v1（position）/v2（Transform/生命周期）/
                // v3（+动画）；旧 Player 的 context 版本低时跳过对应 API。
                if (native.Version < 1 || native.Version > PlayerContextVersion)
                {
                    error = "ABI 版本不支持：native=" + native.Version
                        + " supported=1.." + PlayerContextVersion;
                    return null;
                }
                bool hasTransform = native.Version >= 2;
                bool hasAnimation = native.Version >= 3 && native.AnimationState != IntPtr.Zero;
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
                    error = "基础函数指针缺失";
                    return null;
                }
                if (hasTransform
                    && (native.SceneSetEntityRotation == IntPtr.Zero
                        || native.SceneGetEntityRotation == IntPtr.Zero
                        || native.SceneSetEntityScale == IntPtr.Zero
                        || native.SceneGetEntityScale == IntPtr.Zero
                        || native.SceneEntityExists == IntPtr.Zero
                        || native.SceneDestroyEntity == IntPtr.Zero))
                {
                    error = "v2 函数指针缺失";
                    return null;
                }
                if (hasAnimation
                    && (native.AnimationClipCount == IntPtr.Zero
                        || native.AnimationClipName == IntPtr.Zero
                        || native.AnimationClipDuration == IntPtr.Zero
                        || native.AnimationCurrent == IntPtr.Zero
                        || native.AnimationPlay == IntPtr.Zero
                        || native.AnimationSetTime == IntPtr.Zero
                        || native.AnimationGetTime == IntPtr.Zero
                        || native.AnimationSetSpeed == IntPtr.Zero
                        || native.AnimationGetSpeed == IntPtr.Zero
                        || native.AnimationSetPlaying == IntPtr.Zero))
                {
                    error = "v3 动画函数指针缺失";
                    return null;
                }

                try
                {
                    ContextApi api = new ContextApi
                    {
                        Scene = native.Scene,
                        HasTransform = hasTransform,
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
                    if (hasTransform)
                    {
                        api.SetRotationFn = Marshal.GetDelegateForFunctionPointer<SceneSetEntityRotationDelegate>(
                            native.SceneSetEntityRotation);
                        api.GetRotationFn = Marshal.GetDelegateForFunctionPointer<SceneGetEntityRotationDelegate>(
                            native.SceneGetEntityRotation);
                        api.SetScaleFn = Marshal.GetDelegateForFunctionPointer<SceneSetEntityScaleDelegate>(
                            native.SceneSetEntityScale);
                        api.GetScaleFn = Marshal.GetDelegateForFunctionPointer<SceneGetEntityScaleDelegate>(
                            native.SceneGetEntityScale);
                        api.EntityExistsFn = Marshal.GetDelegateForFunctionPointer<SceneEntityExistsDelegate>(
                            native.SceneEntityExists);
                        api.DestroyEntityFn = Marshal.GetDelegateForFunctionPointer<SceneDestroyEntityDelegate>(
                            native.SceneDestroyEntity);
                    }
                    if (hasAnimation)
                    {
                        api.HasAnimation = true;
                        api.AnimationState = native.AnimationState;
                        api.AnimationClipCountFn = Marshal.GetDelegateForFunctionPointer<AnimationClipCountDelegate>(
                            native.AnimationClipCount);
                        api.AnimationClipNameFn = Marshal.GetDelegateForFunctionPointer<AnimationClipNameDelegate>(
                            native.AnimationClipName);
                        api.AnimationClipDurationFn = Marshal.GetDelegateForFunctionPointer<AnimationClipDurationDelegate>(
                            native.AnimationClipDuration);
                        api.AnimationCurrentFn = Marshal.GetDelegateForFunctionPointer<AnimationCurrentDelegate>(
                            native.AnimationCurrent);
                        api.AnimationPlayFn = Marshal.GetDelegateForFunctionPointer<AnimationPlayDelegate>(
                            native.AnimationPlay);
                        api.AnimationSetTimeFn = Marshal.GetDelegateForFunctionPointer<AnimationSetTimeDelegate>(
                            native.AnimationSetTime);
                        api.AnimationGetTimeFn = Marshal.GetDelegateForFunctionPointer<AnimationGetTimeDelegate>(
                            native.AnimationGetTime);
                        api.AnimationSetSpeedFn = Marshal.GetDelegateForFunctionPointer<AnimationSetSpeedDelegate>(
                            native.AnimationSetSpeed);
                        api.AnimationGetSpeedFn = Marshal.GetDelegateForFunctionPointer<AnimationGetSpeedDelegate>(
                            native.AnimationGetSpeed);
                        api.AnimationSetPlayingFn = Marshal.GetDelegateForFunctionPointer<AnimationSetPlayingDelegate>(
                            native.AnimationSetPlaying);
                    }
                    return api;
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
