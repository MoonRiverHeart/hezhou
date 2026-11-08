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
        private const uint PlayerContextVersion = 7;
        private const float CubeX = 2.5f;
        private const float CubeY = 0.5f;
        private const float CubeZ = -1.5f;
        private const float CubeRotationY = 90.0f;
        private const float CubeScale = 1.5f;
        private const float InitialAnimationTime = 0.25f;
        private const float InitialAnimationSpeed = 1.5f;
        /// <summary>第三人称相机俯仰（弧度，约 -15°）。</summary>
        private const float DemoCameraPitch = -0.26179939f;

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
        private static bool _characterCreated;
        private static string _lastGraphState = "";
        private static long _frameCount;
        // v7 第三人称 demo 状态（task-59）。
        private static bool _demoMode;
        private static bool _demoCharacterCreated;
        private static ulong _demoController = 0;
        private static ulong _demoGround = 0;
        private static ulong _demoBody = 0;
        private static ulong _demoHead = 0;
        private static ulong _demoLeftArm = 0;
        private static ulong _demoRightArm = 0;
        private static ulong _demoLeftLeg = 0;
        private static ulong _demoRightLeg = 0;
        private static float _demoYaw;
        private static float _demoWalkPhase;
        private static float _demoLastX;
        private static float _demoLastY;
        private static float _demoLastZ;
        private static bool _demoAutopilot;
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
            _characterCreated = false;

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

                // v7 第三人称 demo：仅当 host 提供 input/camera 且设置 HEZHOU_PLAYER_DEMO=third_person。
                string demoEnv = Environment.GetEnvironmentVariable("HEZHOU_PLAYER_DEMO");
                _demoMode = api.HasInput && api.HasCamera
                    && string.Equals(demoEnv, "third_person", StringComparison.OrdinalIgnoreCase);
                if (_demoMode)
                {
                    InitializeThirdPersonDemo(api);
                    return;
                }

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

                // v4：资产枚举 + 音频控制。
                if (api.HasAssets)
                {
                    uint assetCount = api.AssetCount();
                    uint shown = assetCount < 8 ? assetCount : 8;
                    string assetList = "";
                    for (uint index = 0; index < shown; index++)
                    {
                        string id = api.AssetIdString(index);
                        string kind = api.AssetKindString(index);
                        string path = api.AssetPathString(index);
                        if (index > 0)
                        {
                            assetList += ", ";
                        }
                        assetList += index.ToString(CultureInfo.InvariantCulture)
                            + ":" + kind + ":" + path + ":" + id;
                    }
                    SafeLog("Assets: count="
                        + assetCount.ToString(CultureInfo.InvariantCulture)
                        + " [" + assetList + "]");
                }
                else
                {
                    SafeLog("Assets: context 无 v4 资产 API");
                }

                if (api.HasAudio)
                {
                    string backend = api.AudioBackendString();
                    int playResult = api.AudioPlayFirst();
                    int pauseResult = api.AudioPause();
                    int playingAfterPause = api.AudioIsPlaying();
                    int resumeResult = api.AudioResume();
                    int playingAfterResume = api.AudioIsPlaying();
                    int volumeResult = api.AudioSetVolume(0.5f);
                    SafeLog("Audio: backend=" + backend
                        + " playResult=" + playResult.ToString(CultureInfo.InvariantCulture)
                        + " pauseResult=" + pauseResult.ToString(CultureInfo.InvariantCulture)
                        + " isPlayingAfterPause=" + playingAfterPause.ToString(CultureInfo.InvariantCulture)
                        + " resumeResult=" + resumeResult.ToString(CultureInfo.InvariantCulture)
                        + " isPlayingAfterResume=" + playingAfterResume.ToString(CultureInfo.InvariantCulture)
                        + " volumeResult=" + volumeResult.ToString(CultureInfo.InvariantCulture)
                        + " volume=0.500");
                }
                else
                {
                    SafeLog("Audio: context 无 v4 音频 API");
                }

                // v5：为创建的 cube 建立胶囊角色（地面 y=0）。
                if (api.HasGameplay)
                {
                    int createResult = api.CharacterCreate(
                        _createdEntityId, 0.3f, 1.8f);
                    _characterCreated = createResult == 0;
                    float cx, cy, cz;
                    int posResult = api.CharacterPosition(
                        _createdEntityId, out cx, out cy, out cz);
                    int grounded = api.CharacterGrounded(_createdEntityId);
                    SafeLog("Character: create entity="
                        + _createdEntityId.ToString(CultureInfo.InvariantCulture)
                        + " radius=0.300 height=1.800 result="
                        + createResult.ToString(CultureInfo.InvariantCulture)
                        + " posResult=" + posResult.ToString(CultureInfo.InvariantCulture)
                        + " position=(" + Fmt(cx) + "," + Fmt(cy) + "," + Fmt(cz) + ")"
                        + " grounded=" + grounded.ToString(CultureInfo.InvariantCulture));
                }
                else
                {
                    SafeLog("Character: context 无 v5 角色 API");
                }

                // v6：9 状态动画状态机初始状态。
                if (api.HasAnimationGraph)
                {
                    _lastGraphState = api.AnimationCurrentStateString();
                    SafeLog("AnimGraph: initial state=" + _lastGraphState
                        + " transitioning=" + api.AnimationIsTransitioning().ToString(CultureInfo.InvariantCulture)
                        + " alpha=" + Fmt(api.AnimationTransitionAlpha()));
                }
                else
                {
                    SafeLog("AnimGraph: context 无 v6 状态机 API");
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

            if (_demoMode)
            {
                UpdateThirdPersonDemo(dt);
                return;
            }

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
                    if (_sceneContext.HasAudio && (_frameCount == 1 || _frameCount % 60 == 0))
                    {
                        SafeLog("Audio: frame="
                            + _frameCount.ToString(CultureInfo.InvariantCulture)
                            + " isPlaying=" + _sceneContext.AudioIsPlaying().ToString(CultureInfo.InvariantCulture));
                    }
                    if (_sceneContext.HasGameplay && _characterCreated)
                    {
                        // 每帧 step：+X 1m/s，无跳跃；结果位置写回 Scene。
                        int stepResult = _sceneContext.CharacterStep(
                            _createdEntityId, 1.0f, 0.0f, false, dt);
                        float x, y, z;
                        int posResult = _sceneContext.CharacterPosition(
                            _createdEntityId, out x, out y, out z);
                        int grounded = _sceneContext.CharacterGrounded(_createdEntityId);
                        if (_frameCount == 1 || _frameCount % 60 == 0)
                        {
                            SafeLog("Character: frame="
                                + _frameCount.ToString(CultureInfo.InvariantCulture)
                                + " stepResult=" + stepResult.ToString(CultureInfo.InvariantCulture)
                                + " posResult=" + posResult.ToString(CultureInfo.InvariantCulture)
                                + " position=(" + Fmt(x) + "," + Fmt(y) + "," + Fmt(z) + ")"
                                + " grounded=" + grounded.ToString(CultureInfo.InvariantCulture));
                        }
                    }
                    if (_sceneContext.HasAnimationGraph)
                    {
                        // 首帧触发 Run，第 15 帧/每 60 帧触发 Attack（可配置）。
                        if (_frameCount == 1)
                        {
                            int triggerResult = _sceneContext.TriggerAnimationState("Run");
                            SafeLog("AnimGraph: trigger=Run result="
                                + triggerResult.ToString(CultureInfo.InvariantCulture));
                        }
                        else if (_frameCount == 15 || _frameCount % 60 == 0)
                        {
                            int triggerResult = _sceneContext.TriggerAnimationState("Attack");
                            SafeLog("AnimGraph: trigger=Attack result="
                                + triggerResult.ToString(CultureInfo.InvariantCulture));
                        }
                        string current = _sceneContext.AnimationCurrentStateString();
                        int transitioning = _sceneContext.AnimationIsTransitioning();
                        float alpha = _sceneContext.AnimationTransitionAlpha();
                        if (_frameCount == 1 || _frameCount == 15 || _frameCount % 60 == 0
                            || current != _lastGraphState)
                        {
                            SafeLog("AnimGraph: frame="
                                + _frameCount.ToString(CultureInfo.InvariantCulture)
                                + " state=" + current
                                + " transitioning=" + transitioning.ToString(CultureInfo.InvariantCulture)
                                + " alpha=" + Fmt(alpha));
                            _lastGraphState = current;
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

            if (_demoMode)
            {
                ShutdownThirdPersonDemo();
                return;
            }

            if (_sceneContext != null && _sceneContext.HasAnimationGraph)
            {
                try
                {
                    SafeLog("AnimGraph: Shutdown state="
                        + _sceneContext.AnimationCurrentStateString()
                        + " transitioning=" + _sceneContext.AnimationIsTransitioning().ToString(CultureInfo.InvariantCulture)
                        + " alpha=" + Fmt(_sceneContext.AnimationTransitionAlpha()));
                }
                catch (Exception exception)
                {
                    SafeLog("AnimGraph: Shutdown 异常：" + exception.Message);
                }
            }

            if (_sceneContext != null && _sceneContext.HasGameplay && _characterCreated)
            {
                try
                {
                    float cx, cy, cz;
                    int posResult = _sceneContext.CharacterPosition(
                        _createdEntityId, out cx, out cy, out cz);
                    int grounded = _sceneContext.CharacterGrounded(_createdEntityId);
                    int removeResult = _sceneContext.CharacterRemove(_createdEntityId);
                    SafeLog("Character: Shutdown posResult="
                        + posResult.ToString(CultureInfo.InvariantCulture)
                        + " position=(" + Fmt(cx) + "," + Fmt(cy) + "," + Fmt(cz) + ")"
                        + " grounded=" + grounded.ToString(CultureInfo.InvariantCulture)
                        + " removeResult=" + removeResult.ToString(CultureInfo.InvariantCulture));
                }
                catch (Exception exception)
                {
                    SafeLog("Character: Shutdown 异常：" + exception.Message);
                }
                _characterCreated = false;
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
                    if (_sceneContext.HasAudio)
                    {
                        int playing = _sceneContext.AudioIsPlaying();
                        int stopResult = _sceneContext.AudioStop();
                        int playingAfterStop = _sceneContext.AudioIsPlaying();
                        SafeLog("Audio: Shutdown isPlaying="
                            + playing.ToString(CultureInfo.InvariantCulture)
                            + " stopResult=" + stopResult.ToString(CultureInfo.InvariantCulture)
                            + " isPlayingAfterStop=" + playingAfterStop.ToString(CultureInfo.InvariantCulture));
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

        private static void InitializeThirdPersonDemo(ContextApi api)
        {
            _demoCharacterCreated = false;
            _demoYaw = 0.0f;
            _demoWalkPhase = 0.0f;
            _demoLastX = 0.0f;
            _demoLastY = 0.0f;
            _demoLastZ = 0.0f;
            _demoAutopilot = string.Equals(
                Environment.GetEnvironmentVariable("HEZHOU_PLAYER_DEMO_AUTOPILOT"),
                "1",
                StringComparison.OrdinalIgnoreCase);

            // 地面 + 可见方块角色（身体/头/双臂/双腿）；胶囊由 CharacterMotor 驱动。
            _demoGround = api.CreateCube();
            api.SetPosition(_demoGround, 0.0f, -0.1f, 0.0f);
            api.SetScale(_demoGround, 20.0f, 0.2f, 20.0f);
            _demoController = api.CreateCube();
            api.SetPosition(_demoController, 0.0f, 0.0f, 0.0f);
            api.SetScale(_demoController, 0.18f, 0.18f, 0.18f);
            _demoBody = api.CreateCube();
            api.SetScale(_demoBody, 0.6f, 0.9f, 0.4f);
            _demoHead = api.CreateCube();
            api.SetScale(_demoHead, 0.45f, 0.45f, 0.45f);
            _demoLeftArm = api.CreateCube();
            api.SetScale(_demoLeftArm, 0.18f, 0.7f, 0.18f);
            _demoRightArm = api.CreateCube();
            api.SetScale(_demoRightArm, 0.18f, 0.7f, 0.18f);
            _demoLeftLeg = api.CreateCube();
            api.SetScale(_demoLeftLeg, 0.2f, 0.8f, 0.2f);
            _demoRightLeg = api.CreateCube();
            api.SetScale(_demoRightLeg, 0.2f, 0.8f, 0.2f);
            UpdateDemoParts(0.0f, 0.0f, 0);

            if (api.HasGameplay)
            {
                int characterResult = api.CharacterCreate(_demoController, 0.3f, 1.8f);
                _demoCharacterCreated = characterResult == 0;
            }
            int followResult = api.CameraFollow(
                _demoController, 5.0f, 2.5f, _demoYaw, DemoCameraPitch);
            SafeLog("ThirdPerson: ground="
                + _demoGround.ToString(CultureInfo.InvariantCulture)
                + " controller=" + _demoController.ToString(CultureInfo.InvariantCulture)
                + " body=" + _demoBody.ToString(CultureInfo.InvariantCulture)
                + " head=" + _demoHead.ToString(CultureInfo.InvariantCulture)
                + " character=" + BoolText(_demoCharacterCreated)
                + " cameraFollow=" + followResult.ToString(CultureInfo.InvariantCulture)
                + " autopilot=" + BoolText(_demoAutopilot)
                + "（WASD/方向键移动，Space 跳跃）");
        }

        private static void UpdateThirdPersonDemo(float dt)
        {
            ContextApi api = _sceneContext;
            if (api == null)
            {
                return;
            }
            float axisX = api.InputAxisX();
            float axisZ = api.InputAxisZ();
            bool jump = api.InputJumpPressed() != 0;
            if (_demoAutopilot)
            {
                // 自动化 demo：向 +X 移动，第 20 帧跳跃（默认关闭，仅测试/演示用）。
                axisX = 1.0f;
                axisZ = 0.0f;
                jump = _frameCount == 20;
            }
            bool moving = Math.Abs(axisX) + Math.Abs(axisZ) > 0.01f;
            if (moving)
            {
                _demoYaw = (float)Math.Atan2(axisX, axisZ);
            }
            const float moveSpeed = 4.5f;
            int stepResult = -1;
            float x = _demoLastX;
            float y = _demoLastY;
            float z = _demoLastZ;
            int grounded = 0;
            if (_demoCharacterCreated)
            {
                stepResult = api.CharacterStep(
                    _demoController, axisX * moveSpeed, axisZ * moveSpeed, jump, dt);
                int positionResult = api.CharacterPosition(
                    _demoController, out x, out y, out z);
                if (positionResult != 0)
                {
                    x = _demoLastX;
                    y = _demoLastY;
                    z = _demoLastZ;
                }
                grounded = api.CharacterGrounded(_demoController);
            }
            _demoLastX = x;
            _demoLastY = y;
            _demoLastZ = z;

            _demoWalkPhase += moving ? dt * 9.0f : 0.0f;
            UpdateDemoParts(x, y, grounded);

            if (api.HasCamera)
            {
                api.CameraFollow(
                    _demoController, 5.0f, 2.5f, _demoYaw, DemoCameraPitch);
            }
            if (_frameCount == 1 || _frameCount % 30 == 0)
            {
                SafeLog("ThirdPerson: frame="
                    + _frameCount.ToString(CultureInfo.InvariantCulture)
                    + " input=(" + Fmt(axisX) + "," + Fmt(axisZ) + ")"
                    + " jump=" + BoolText(jump)
                    + " stepResult=" + stepResult.ToString(CultureInfo.InvariantCulture)
                    + " position=(" + Fmt(x) + "," + Fmt(y) + "," + Fmt(z) + ")"
                    + " grounded=" + grounded.ToString(CultureInfo.InvariantCulture)
                    + " yaw=" + Fmt(_demoYaw * 57.29578f)
                    + " swing=" + Fmt((float)Math.Sin(_demoWalkPhase) * 28.0f));
            }
            if (jump)
            {
                SafeLog("ThirdPerson: jump frame="
                    + _frameCount.ToString(CultureInfo.InvariantCulture)
                    + " position=(" + Fmt(x) + "," + Fmt(y) + "," + Fmt(z) + ")"
                    + " grounded=" + grounded.ToString(CultureInfo.InvariantCulture));
            }
        }

        private static void UpdateDemoParts(float x, float y, int grounded)
        {
            ContextApi api = _sceneContext;
            if (api == null)
            {
                return;
            }
            float yawDeg = _demoYaw * 57.29578f;
            float swing = (float)Math.Sin(_demoWalkPhase) * 28.0f;
            api.SetPosition(_demoBody, x, y + 0.9f, 0.0f);
            api.SetPosition(_demoHead, x, y + 1.55f, 0.0f);
            api.SetPosition(_demoLeftArm, x - 0.45f, y + 0.95f, 0.0f);
            api.SetPosition(_demoRightArm, x + 0.45f, y + 0.95f, 0.0f);
            api.SetPosition(_demoLeftLeg, x - 0.18f, y + 0.45f, 0.0f);
            api.SetPosition(_demoRightLeg, x + 0.18f, y + 0.45f, 0.0f);
            api.SetRotation(_demoBody, 0.0f, yawDeg, 0.0f);
            api.SetRotation(_demoHead, 0.0f, yawDeg, 0.0f);
            api.SetRotation(_demoLeftArm, swing, yawDeg, 0.0f);
            api.SetRotation(_demoRightArm, -swing, yawDeg, 0.0f);
            api.SetRotation(_demoLeftLeg, -swing, yawDeg, 0.0f);
            api.SetRotation(_demoRightLeg, swing, yawDeg, 0.0f);
            if (!_demoCharacterCreated || grounded == 0)
            {
                // 离地/无角色时保持视觉件随控制器位置（由调用方 y 提供）。
            }
        }

        private static void ShutdownThirdPersonDemo()
        {
            ContextApi api = _sceneContext;
            if (api != null)
            {
                if (_demoCharacterCreated)
                {
                    api.CharacterRemove(_demoController);
                }
                if (api.HasCamera)
                {
                    api.CameraClear();
                }
                SafeLog("ThirdPerson: Shutdown position=("
                    + Fmt(_demoLastX) + "," + Fmt(_demoLastY) + "," + Fmt(_demoLastZ)
                    + ") grounded=" + api.CharacterGrounded(_demoController).ToString(CultureInfo.InvariantCulture)
                    + " frames=" + _frameCount.ToString(CultureInfo.InvariantCulture));
                ulong[] parts = new ulong[] {
                    _demoBody, _demoHead, _demoLeftArm, _demoRightArm,
                    _demoLeftLeg, _demoRightLeg, _demoGround, _demoController
                };
                for (int index = 0; index < parts.Length; index++)
                {
                    if (parts[index] != 0)
                    {
                        api.DestroyEntity(parts[index]);
                    }
                }
            }
            _demoMode = false;
            _demoCharacterCreated = false;
            _sceneContext = null;
            _initialized = false;
            SafeLog("ThirdPerson: demo 结束");
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
            // v4 tail（task-35）：资产目录 + 音频。
            public IntPtr Assets;
            public IntPtr AssetCount;
            public IntPtr AssetId;
            public IntPtr AssetKind;
            public IntPtr AssetPath;
            public IntPtr Audio;
            public IntPtr AudioPlayFirst;
            public IntPtr AudioPause;
            public IntPtr AudioResume;
            public IntPtr AudioStop;
            public IntPtr AudioSetVolume;
            public IntPtr AudioIsPlaying;
            public IntPtr AudioBackend;
            // v5 tail（task-36）：角色控制。
            public IntPtr GameplayWorld;
            public IntPtr CharacterCreate;
            public IntPtr CharacterStep;
            public IntPtr CharacterGrounded;
            public IntPtr CharacterPosition;
            public IntPtr CharacterRemove;
            // v6 tail（task-37）：9 状态动画状态机。
            public IntPtr AnimationStateMachine;
            public IntPtr AnimationSetState;
            public IntPtr AnimationTrigger;
            public IntPtr AnimationCurrentState;
            public IntPtr AnimationIsTransitioning;
            public IntPtr AnimationTransitionAlpha;
            // v7 tail（task-59）：输入快照 + 第三人称相机。
            public IntPtr InputState;
            public IntPtr InputAxisX;
            public IntPtr InputAxisZ;
            public IntPtr InputJumpPressed;
            public IntPtr CameraState;
            public IntPtr CameraFollow;
            public IntPtr CameraClear;
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

        // v4：资产目录 + 音频（task-35）。
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate uint AssetCountDelegate(IntPtr assets);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AssetStringDelegate(
            IntPtr assets, uint index, IntPtr buffer, uint bufferLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AudioPlayFirstDelegate(IntPtr audio);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AudioPauseDelegate(IntPtr audio);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AudioResumeDelegate(IntPtr audio);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AudioStopDelegate(IntPtr audio);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AudioSetVolumeDelegate(IntPtr audio, float volume);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AudioIsPlayingDelegate(IntPtr audio);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AudioBackendDelegate(
            IntPtr audio, uint unused, IntPtr buffer, uint bufferLength);

        // v5：角色控制（task-36）。
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int CharacterCreateDelegate(
            IntPtr world, ulong entityId, float radius, float totalHeight);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int CharacterStepDelegate(
            IntPtr world, ulong entityId, float inputX, float inputZ, int jump, float dt);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int CharacterGroundedDelegate(IntPtr world, ulong entityId);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int CharacterPositionDelegate(
            IntPtr world, ulong entityId, out float x, out float y, out float z);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int CharacterRemoveDelegate(IntPtr world, ulong entityId);

        // v6：9 状态动画状态机（task-37）。
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationSetStateDelegate(IntPtr graph, IntPtr name);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationTriggerDelegate(IntPtr graph, IntPtr name);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationCurrentStateDelegate(
            IntPtr graph, IntPtr buffer, uint bufferLength);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int AnimationIsTransitioningDelegate(IntPtr graph);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate float AnimationTransitionAlphaDelegate(IntPtr graph);

        // v7：输入快照 + 第三人称相机（task-59）。
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate float InputAxisXDelegate(IntPtr input);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate float InputAxisZDelegate(IntPtr input);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int InputJumpPressedDelegate(IntPtr input);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int CameraFollowDelegate(
            IntPtr camera, ulong entityId, float distance, float height, float yaw, float pitch);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int CameraClearDelegate(IntPtr camera);

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
            // v4 资产/音频。
            public bool HasAssets;
            public IntPtr Assets;
            public AssetCountDelegate AssetCountFn;
            public AssetStringDelegate AssetIdFn;
            public AssetStringDelegate AssetKindFn;
            public AssetStringDelegate AssetPathFn;
            public bool HasAudio;
            public IntPtr Audio;
            public AudioPlayFirstDelegate AudioPlayFirstFn;
            public AudioPauseDelegate AudioPauseFn;
            public AudioResumeDelegate AudioResumeFn;
            public AudioStopDelegate AudioStopFn;
            public AudioSetVolumeDelegate AudioSetVolumeFn;
            public AudioIsPlayingDelegate AudioIsPlayingFn;
            public AudioBackendDelegate AudioBackendFn;
            // v5 角色控制。
            public bool HasGameplay;
            public IntPtr GameplayWorld;
            public CharacterCreateDelegate CharacterCreateFn;
            public CharacterStepDelegate CharacterStepFn;
            public CharacterGroundedDelegate CharacterGroundedFn;
            public CharacterPositionDelegate CharacterPositionFn;
            public CharacterRemoveDelegate CharacterRemoveFn;
            // v6 动画状态机。
            public bool HasAnimationGraph;
            public IntPtr AnimationStateMachine;
            public AnimationSetStateDelegate AnimationSetStateFn;
            public AnimationTriggerDelegate AnimationTriggerFn;
            public AnimationCurrentStateDelegate AnimationCurrentStateFn;
            public AnimationIsTransitioningDelegate AnimationIsTransitioningFn;
            public AnimationTransitionAlphaDelegate AnimationTransitionAlphaFn;
            // v7 输入/相机。
            public bool HasInput;
            public bool HasCamera;
            public IntPtr InputState;
            public IntPtr CameraState;
            public InputAxisXDelegate InputAxisXFn;
            public InputAxisZDelegate InputAxisZFn;
            public InputJumpPressedDelegate InputJumpPressedFn;
            public CameraFollowDelegate CameraFollowFn;
            public CameraClearDelegate CameraClearFn;

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

            public uint AssetCount()
            {
                return AssetCountFn(Assets);
            }

            public string AssetIdString(uint index)
            {
                return ReadUtf8(AssetIdFn, Assets, index);
            }

            public string AssetKindString(uint index)
            {
                return ReadUtf8(AssetKindFn, Assets, index);
            }

            public string AssetPathString(uint index)
            {
                return ReadUtf8(AssetPathFn, Assets, index);
            }

            public int AudioPlayFirst()
            {
                return AudioPlayFirstFn(Audio);
            }

            public int AudioPause()
            {
                return AudioPauseFn(Audio);
            }

            public int AudioResume()
            {
                return AudioResumeFn(Audio);
            }

            public int AudioStop()
            {
                return AudioStopFn(Audio);
            }

            public int AudioSetVolume(float volume)
            {
                return AudioSetVolumeFn(Audio, volume);
            }

            public int AudioIsPlaying()
            {
                return AudioIsPlayingFn(Audio);
            }

            public int CharacterCreate(ulong entityId, float radius, float totalHeight)
            {
                return CharacterCreateFn(GameplayWorld, entityId, radius, totalHeight);
            }

            public int CharacterStep(
                ulong entityId, float inputX, float inputZ, bool jump, float dt)
            {
                return CharacterStepFn(
                    GameplayWorld, entityId, inputX, inputZ, jump ? 1 : 0, dt);
            }

            public int CharacterGrounded(ulong entityId)
            {
                return CharacterGroundedFn(GameplayWorld, entityId);
            }

            public int CharacterPosition(
                ulong entityId, out float x, out float y, out float z)
            {
                return CharacterPositionFn(GameplayWorld, entityId, out x, out y, out z);
            }

            public int CharacterRemove(ulong entityId)
            {
                return CharacterRemoveFn(GameplayWorld, entityId);
            }

            public float InputAxisX()
            {
                return InputAxisXFn(InputState);
            }

            public float InputAxisZ()
            {
                return InputAxisZFn(InputState);
            }

            public int InputJumpPressed()
            {
                return InputJumpPressedFn(InputState);
            }

            public int CameraFollow(
                ulong entityId, float distance, float height, float yaw, float pitch)
            {
                return CameraFollowFn(CameraState, entityId, distance, height, yaw, pitch);
            }

            public int CameraClear()
            {
                return CameraClearFn(CameraState);
            }

            public int SetAnimationState(string stateName)
            {
                return CallWithAnsiString(AnimationSetStateFn, stateName);
            }

            public int TriggerAnimationState(string stateName)
            {
                return CallWithAnsiString(AnimationTriggerFn, stateName);
            }

            public string AnimationCurrentStateString()
            {
                int needed = AnimationCurrentStateFn(
                    AnimationStateMachine, IntPtr.Zero, 0);
                if (needed <= 0)
                {
                    return "";
                }
                IntPtr buffer = Marshal.AllocHGlobal(needed);
                try
                {
                    int written = AnimationCurrentStateFn(
                        AnimationStateMachine, buffer, (uint)needed);
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

            public int AnimationIsTransitioning()
            {
                return AnimationIsTransitioningFn(AnimationStateMachine);
            }

            public float AnimationTransitionAlpha()
            {
                return AnimationTransitionAlphaFn(AnimationStateMachine);
            }

            private int CallWithAnsiString(
                AnimationSetStateDelegate function, string stateName)
            {
                return CallWithAnsiStringCore(
                    (graph, name) => function(graph, name), stateName);
            }

            private int CallWithAnsiString(
                AnimationTriggerDelegate function, string stateName)
            {
                return CallWithAnsiStringCore(
                    (graph, name) => function(graph, name), stateName);
            }

            private int CallWithAnsiStringCore(
                Func<IntPtr, IntPtr, int> function, string stateName)
            {
                IntPtr name = Marshal.StringToHGlobalAnsi(stateName);
                try
                {
                    return function(AnimationStateMachine, name);
                }
                finally
                {
                    Marshal.FreeHGlobal(name);
                }
            }

            public string AudioBackendString()
            {
                int needed = AudioBackendFn(Audio, 0, IntPtr.Zero, 0);
                if (needed <= 0)
                {
                    return "";
                }
                IntPtr buffer = Marshal.AllocHGlobal(needed);
                try
                {
                    int written = AudioBackendFn(Audio, 0, buffer, (uint)needed);
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

            private static string ReadUtf8(AssetStringDelegate reader, IntPtr owner, uint index)
            {
                int needed = reader(owner, index, IntPtr.Zero, 0);
                if (needed <= 0)
                {
                    return "";
                }
                IntPtr buffer = Marshal.AllocHGlobal(needed);
                try
                {
                    int written = reader(owner, index, buffer, (uint)needed);
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

                // v6 兼容策略：接受 v1（position）/v2（Transform）/v3（动画）/
                // v4（资产/音频）/v5（角色）/v6（9 状态机）。
                if (native.Version < 1 || native.Version > PlayerContextVersion)
                {
                    error = "ABI 版本不支持：native=" + native.Version
                        + " supported=1.." + PlayerContextVersion;
                    return null;
                }
                bool hasTransform = native.Version >= 2;
                bool hasAnimation = native.Version >= 3 && native.AnimationState != IntPtr.Zero;
                bool hasAssets = native.Version >= 4 && native.Assets != IntPtr.Zero;
                bool hasAudio = native.Version >= 4 && native.Audio != IntPtr.Zero;
                bool hasGameplay = native.Version >= 5 && native.GameplayWorld != IntPtr.Zero;
                bool hasAnimationGraph =
                    native.Version >= 6 && native.AnimationStateMachine != IntPtr.Zero;
                bool hasInput = native.Version >= 7 && native.InputState != IntPtr.Zero;
                bool hasCamera = native.Version >= 7 && native.CameraState != IntPtr.Zero;
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
                if (hasAssets
                    && (native.AssetCount == IntPtr.Zero
                        || native.AssetId == IntPtr.Zero
                        || native.AssetKind == IntPtr.Zero
                        || native.AssetPath == IntPtr.Zero))
                {
                    error = "v4 资产函数指针缺失";
                    return null;
                }
                if (hasAudio
                    && (native.AudioPlayFirst == IntPtr.Zero
                        || native.AudioPause == IntPtr.Zero
                        || native.AudioResume == IntPtr.Zero
                        || native.AudioStop == IntPtr.Zero
                        || native.AudioSetVolume == IntPtr.Zero
                        || native.AudioIsPlaying == IntPtr.Zero
                        || native.AudioBackend == IntPtr.Zero))
                {
                    error = "v4 音频函数指针缺失";
                    return null;
                }
                if (hasGameplay
                    && (native.CharacterCreate == IntPtr.Zero
                        || native.CharacterStep == IntPtr.Zero
                        || native.CharacterGrounded == IntPtr.Zero
                        || native.CharacterPosition == IntPtr.Zero
                        || native.CharacterRemove == IntPtr.Zero))
                {
                    error = "v5 角色函数指针缺失";
                    return null;
                }
                if (hasAnimationGraph
                    && (native.AnimationSetState == IntPtr.Zero
                        || native.AnimationTrigger == IntPtr.Zero
                        || native.AnimationCurrentState == IntPtr.Zero
                        || native.AnimationIsTransitioning == IntPtr.Zero
                        || native.AnimationTransitionAlpha == IntPtr.Zero))
                {
                    error = "v6 动画状态机函数指针缺失";
                    return null;
                }
                if (hasInput
                    && (native.InputAxisX == IntPtr.Zero
                        || native.InputAxisZ == IntPtr.Zero
                        || native.InputJumpPressed == IntPtr.Zero))
                {
                    error = "v7 输入函数指针缺失";
                    return null;
                }
                if (hasCamera
                    && (native.CameraFollow == IntPtr.Zero
                        || native.CameraClear == IntPtr.Zero))
                {
                    error = "v7 相机函数指针缺失";
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
                    if (hasAssets)
                    {
                        api.HasAssets = true;
                        api.Assets = native.Assets;
                        api.AssetCountFn = Marshal.GetDelegateForFunctionPointer<AssetCountDelegate>(
                            native.AssetCount);
                        api.AssetIdFn = Marshal.GetDelegateForFunctionPointer<AssetStringDelegate>(
                            native.AssetId);
                        api.AssetKindFn = Marshal.GetDelegateForFunctionPointer<AssetStringDelegate>(
                            native.AssetKind);
                        api.AssetPathFn = Marshal.GetDelegateForFunctionPointer<AssetStringDelegate>(
                            native.AssetPath);
                    }
                    if (hasAudio)
                    {
                        api.HasAudio = true;
                        api.Audio = native.Audio;
                        api.AudioPlayFirstFn = Marshal.GetDelegateForFunctionPointer<AudioPlayFirstDelegate>(
                            native.AudioPlayFirst);
                        api.AudioPauseFn = Marshal.GetDelegateForFunctionPointer<AudioPauseDelegate>(
                            native.AudioPause);
                        api.AudioResumeFn = Marshal.GetDelegateForFunctionPointer<AudioResumeDelegate>(
                            native.AudioResume);
                        api.AudioStopFn = Marshal.GetDelegateForFunctionPointer<AudioStopDelegate>(
                            native.AudioStop);
                        api.AudioSetVolumeFn = Marshal.GetDelegateForFunctionPointer<AudioSetVolumeDelegate>(
                            native.AudioSetVolume);
                        api.AudioIsPlayingFn = Marshal.GetDelegateForFunctionPointer<AudioIsPlayingDelegate>(
                            native.AudioIsPlaying);
                        api.AudioBackendFn = Marshal.GetDelegateForFunctionPointer<AudioBackendDelegate>(
                            native.AudioBackend);
                    }
                    if (hasGameplay)
                    {
                        api.HasGameplay = true;
                        api.GameplayWorld = native.GameplayWorld;
                        api.CharacterCreateFn = Marshal.GetDelegateForFunctionPointer<CharacterCreateDelegate>(
                            native.CharacterCreate);
                        api.CharacterStepFn = Marshal.GetDelegateForFunctionPointer<CharacterStepDelegate>(
                            native.CharacterStep);
                        api.CharacterGroundedFn = Marshal.GetDelegateForFunctionPointer<CharacterGroundedDelegate>(
                            native.CharacterGrounded);
                        api.CharacterPositionFn = Marshal.GetDelegateForFunctionPointer<CharacterPositionDelegate>(
                            native.CharacterPosition);
                        api.CharacterRemoveFn = Marshal.GetDelegateForFunctionPointer<CharacterRemoveDelegate>(
                            native.CharacterRemove);
                    }
                    if (hasAnimationGraph)
                    {
                        api.HasAnimationGraph = true;
                        api.AnimationStateMachine = native.AnimationStateMachine;
                        api.AnimationSetStateFn = Marshal.GetDelegateForFunctionPointer<AnimationSetStateDelegate>(
                            native.AnimationSetState);
                        api.AnimationTriggerFn = Marshal.GetDelegateForFunctionPointer<AnimationTriggerDelegate>(
                            native.AnimationTrigger);
                        api.AnimationCurrentStateFn = Marshal.GetDelegateForFunctionPointer<AnimationCurrentStateDelegate>(
                            native.AnimationCurrentState);
                        api.AnimationIsTransitioningFn = Marshal.GetDelegateForFunctionPointer<AnimationIsTransitioningDelegate>(
                            native.AnimationIsTransitioning);
                        api.AnimationTransitionAlphaFn = Marshal.GetDelegateForFunctionPointer<AnimationTransitionAlphaDelegate>(
                            native.AnimationTransitionAlpha);
                    }
                    if (hasInput)
                    {
                        api.HasInput = true;
                        api.InputState = native.InputState;
                        api.InputAxisXFn = Marshal.GetDelegateForFunctionPointer<InputAxisXDelegate>(
                            native.InputAxisX);
                        api.InputAxisZFn = Marshal.GetDelegateForFunctionPointer<InputAxisZDelegate>(
                            native.InputAxisZ);
                        api.InputJumpPressedFn = Marshal.GetDelegateForFunctionPointer<InputJumpPressedDelegate>(
                            native.InputJumpPressed);
                    }
                    if (hasCamera)
                    {
                        api.HasCamera = true;
                        api.CameraState = native.CameraState;
                        api.CameraFollowFn = Marshal.GetDelegateForFunctionPointer<CameraFollowDelegate>(
                            native.CameraFollow);
                        api.CameraClearFn = Marshal.GetDelegateForFunctionPointer<CameraClearDelegate>(
                            native.CameraClear);
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
