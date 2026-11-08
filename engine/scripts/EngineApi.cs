// EngineApi.cs — 稳定 C# 引擎 API 外观（0.3 task-48）
//
// 目的：让游戏脚本通过 `Hezhou.Engine` 命名空间操作场景/实体/时间/音频/动画预览，
// 不直接依赖 UI 内部实现（UI.cs/UI.Scene.cs 的静态 FFI 包装）。
//
// 边界：
// - 本文件只做「易用外观 + 参数校验 + 状态缓存」，所有实际调用转发到现有 UI.* 静态包装；
// - 不复制 FFI 逻辑，不新增 FFI；
// - 与 EditorScript.dll 同一程序集（非独立 DLL）；Player 独立 context 后续统一。
//
// 已知限制（0.3）：
// - Entity.Visible 尚无对应 C# FFI 包装（仅本地字段，未同步渲染层）；
// - Entity.Rotation 目前只读（Euler 角度；缺少 C# setter 包装）；
// - Audio.PlayFirst 播放 AssetDatabase 中第一个音频资产（无参数选择）。

using System;
using Hezhou;

namespace Hezhou.Engine
{
    /// <summary>引擎级入口：版本/初始化状态/当前场景。</summary>
    public static class Engine
    {
        /// <summary>API 版本（与 engine 0.3 能力对齐）。</summary>
        public const string Version = "0.3.0";

        private static bool _initialized;
        private static IntPtr _sceneOverride = IntPtr.Zero;

        /// <summary>引擎 API 是否可用（显式 Initialize 或已有场景）。</summary>
        public static bool IsInitialized
        {
            get { return _initialized || CurrentScenePtr != IntPtr.Zero; }
        }

        /// <summary>当前场景指针（优先显式绑定，否则取编辑器现有场景）。</summary>
        public static IntPtr CurrentScenePtr
        {
            get
            {
                if (_sceneOverride != IntPtr.Zero) return _sceneOverride;
                IntPtr existing = UI.SceneGetExistingPtr();
                return existing;
            }
        }

        /// <summary>初始化 API（幂等）。</summary>
        public static void Initialize()
        {
            _initialized = true;
            Debug.Log("Engine.Initialize（Version=" + Version + "）");
        }

        /// <summary>关闭 API（幂等）。</summary>
        public static void Shutdown()
        {
            _initialized = false;
            _sceneOverride = IntPtr.Zero;
            Debug.Log("Engine.Shutdown");
        }

        /// <summary>显式绑定场景（测试/Player 宿主用；传 IntPtr.Zero 恢复自动绑定）。</summary>
        public static void BindScene(IntPtr scene)
        {
            _sceneOverride = scene;
        }

        /// <summary>输出信息日志（向后兼容简写）。</summary>
        public static void Log(string message)
        {
            Debug.Log(message);
        }
    }

    /// <summary>场景操作（实体 CRUD/查询/父子）。</summary>
    public static class Scene
    {
        private static IntPtr Ptr { get { return Engine.CurrentScenePtr; } }

        /// <summary>场景实体数量。</summary>
        public static int EntityCount
        {
            get { return Ptr == IntPtr.Zero ? 0 : (int)UI.SceneGetEntityCount(Ptr); }
        }

        /// <summary>创建空实体。</summary>
        public static Entity CreateEntity()
        {
            if (Ptr == IntPtr.Zero) return Entity.Invalid;
            return new Entity(UI.SceneCreateEntity(Ptr));
        }

        /// <summary>创建内置 Cube 实体。</summary>
        public static Entity CreateCube()
        {
            if (Ptr == IntPtr.Zero) return Entity.Invalid;
            return new Entity(UI.SceneCreateCube(Ptr));
        }

        /// <summary>创建内置 Plane 实体。</summary>
        public static Entity CreatePlane()
        {
            if (Ptr == IntPtr.Zero) return Entity.Invalid;
            return new Entity(UI.SceneCreatePlane(Ptr));
        }

        /// <summary>创建方向光实体。</summary>
        public static Entity CreateDirectionalLight()
        {
            if (Ptr == IntPtr.Zero) return Entity.Invalid;
            return new Entity(UI.SceneCreateDirectionalLight(Ptr));
        }

        /// <summary>销毁实体（0 忽略）。</summary>
        public static void DestroyEntity(ulong entityId)
        {
            if (Ptr == IntPtr.Zero || entityId == 0) return;
            UI.SceneRemoveEntity(Ptr, entityId);
        }

        /// <summary>销毁实体。</summary>
        public static void DestroyEntity(Entity entity)
        {
            if (entity != null) DestroyEntity(entity.Id);
        }

        /// <summary>实体是否存在。</summary>
        public static bool EntityExists(ulong entityId)
        {
            if (Ptr == IntPtr.Zero || entityId == 0) return false;
            int count = (int)UI.SceneGetEntityCount(Ptr);
            for (int i = 0; i < count; i++)
            {
                if (UI.SceneGetEntityId(Ptr, (ulong)i) == entityId) return true;
            }
            return false;
        }

        /// <summary>按名称查找实体（找不到返回 null）。</summary>
        public static Entity FindByName(string name)
        {
            if (Ptr == IntPtr.Zero || string.IsNullOrEmpty(name)) return null;
            int count = (int)UI.SceneGetEntityCount(Ptr);
            for (int i = 0; i < count; i++)
            {
                ulong id = UI.SceneGetEntityId(Ptr, (ulong)i);
                if (id != 0 && UI.SceneGetEntityName(Ptr, id) == name)
                {
                    return new Entity(id);
                }
            }
            return null;
        }

        /// <summary>设置父实体（parentId=0 脱离父级）。</summary>
        public static void SetParent(ulong entityId, ulong parentId)
        {
            if (Ptr == IntPtr.Zero || entityId == 0) return;
            UI.SceneSetParent(Ptr, entityId, parentId);
        }

        /// <summary>获取父实体 ID（0=根）。</summary>
        public static ulong GetParent(ulong entityId)
        {
            if (Ptr == IntPtr.Zero || entityId == 0) return 0;
            return UI.SceneGetParent(Ptr, entityId);
        }
    }

    /// <summary>三维向量（C# 外观层简单值类型）。</summary>
    public struct Vector3
    {
        public float X;
        public float Y;
        public float Z;

        public Vector3(float x, float y, float z)
        {
            X = x;
            Y = y;
            Z = z;
        }

        public override string ToString()
        {
            return "(" + X + ", " + Y + ", " + Z + ")";
        }
    }

    /// <summary>实体句柄（轻量包装；属性即时读写场景）。</summary>
    public sealed class Entity
    {
        /// <summary>无效实体（ID=0）。</summary>
        public static readonly Entity Invalid = new Entity(0);

        /// <summary>实体 ID。</summary>
        public ulong Id { get; private set; }

        /// <summary>本地 Visible 字段（0.3 尚未同步渲染层，见文件头限制）。</summary>
        public bool Visible { get; set; }

        public Entity(ulong id)
        {
            Id = id;
            Visible = true;
        }

        private static IntPtr Ptr { get { return Engine.CurrentScenePtr; } }

        /// <summary>实体是否仍存在。</summary>
        public bool IsAlive
        {
            get { return Scene.EntityExists(Id); }
        }

        /// <summary>名称（读写）。</summary>
        public string Name
        {
            get
            {
                if (Ptr == IntPtr.Zero || Id == 0) return "";
                return UI.SceneGetEntityName(Ptr, Id);
            }
            set
            {
                if (Ptr == IntPtr.Zero || Id == 0) return;
                UI.SceneSetEntityName(Ptr, Id, value ?? "");
            }
        }

        /// <summary>位置（Euler 世界坐标，读写）。</summary>
        public Vector3 Position
        {
            get
            {
                if (Ptr == IntPtr.Zero || Id == 0) return new Vector3(0f, 0f, 0f);
                float x, y, z;
                UI.SceneGetEntityPosition(Ptr, Id, out x, out y, out z);
                return new Vector3(x, y, z);
            }
            set
            {
                if (Ptr == IntPtr.Zero || Id == 0) return;
                UI.SceneSetEntityPosition(Ptr, Id, value.X, value.Y, value.Z);
            }
        }

        /// <summary>旋转（Euler 角度；由四元数换算，0.3 只读）。</summary>
        public Vector3 Rotation
        {
            get
            {
                if (Ptr == IntPtr.Zero || Id == 0) return new Vector3(0f, 0f, 0f);
                float x, y, z, w;
                UI.SceneGetEntityRotation(Ptr, Id, out x, out y, out z, out w);
                return QuaternionToEulerDegrees(x, y, z, w);
            }
        }

        /// <summary>四元数 (x,y,z,w) → Euler 角度（度，XYZ 顺序）。</summary>
        private static Vector3 QuaternionToEulerDegrees(float x, float y, float z, float w)
        {
            const double radToDeg = 180.0 / System.Math.PI;
            double sinrCosp = 2.0 * (w * x + y * z);
            double cosrCosp = 1.0 - 2.0 * (x * x + y * y);
            double roll = System.Math.Atan2(sinrCosp, cosrCosp);
            double sinPitch = 2.0 * (w * y - z * x);
            if (sinPitch > 1.0) sinPitch = 1.0;
            if (sinPitch < -1.0) sinPitch = -1.0;
            double pitch = System.Math.Asin(sinPitch);
            double sinyCosp = 2.0 * (w * z + x * y);
            double cosyCosp = 1.0 - 2.0 * (y * y + z * z);
            double yaw = System.Math.Atan2(sinyCosp, cosyCosp);
            return new Vector3((float)(roll * radToDeg), (float)(pitch * radToDeg), (float)(yaw * radToDeg));
        }

        /// <summary>缩放（读写）。</summary>
        public Vector3 Scale
        {
            get
            {
                if (Ptr == IntPtr.Zero || Id == 0) return new Vector3(1f, 1f, 1f);
                float x, y, z;
                UI.SceneGetEntityScale(Ptr, Id, out x, out y, out z);
                return new Vector3(x, y, z);
            }
            set
            {
                if (Ptr == IntPtr.Zero || Id == 0) return;
                UI.SceneSetEntityScale(Ptr, Id, value.X, value.Y, value.Z);
            }
        }

        /// <summary>父实体 ID（0=根；读写）。</summary>
        public ulong ParentId
        {
            get { return Scene.GetParent(Id); }
            set { Scene.SetParent(Id, value); }
        }

        /// <summary>设置父实体。</summary>
        public void SetParent(Entity parent)
        {
            Scene.SetParent(Id, parent == null ? 0UL : parent.Id);
        }

        public override string ToString()
        {
            return "Entity(" + Id + ", " + Name + ")";
        }
    }

    /// <summary>时间/帧计数（由编辑器 Update 每帧驱动）。</summary>
    public static class Time
    {
        private static float _deltaTime;
        private static double _time;
        private static long _frameCount = -1;

        /// <summary>上一帧耗时（秒）。</summary>
        public static float DeltaTime { get { return _deltaTime; } }

        /// <summary>累计运行时间（秒）。</summary>
        public static double ElapsedTime { get { return _time; } }

        /// <summary>已运行帧数。</summary>
        public static long FrameCount { get { return _frameCount < 0 ? 0 : _frameCount; } }

        /// <summary>编辑器/Player 每帧调用（非法 dt 按 0 处理）。</summary>
        public static void Tick(float deltaTime)
        {
            if (float.IsNaN(deltaTime) || float.IsInfinity(deltaTime) || deltaTime < 0f)
            {
                deltaTime = 0f;
            }
            _deltaTime = deltaTime;
            _time += deltaTime;
            _frameCount = FrameCount + 1;
        }

        /// <summary>重置时间统计（测试/场景重载用）。</summary>
        public static void Reset()
        {
            _deltaTime = 0f;
            _time = 0.0;
            _frameCount = -1;
        }
    }

    /// <summary>音频外观（封装现有 9 个 audio FFI 包装）。</summary>
    public static class Audio
    {
        /// <summary>播放 AssetDatabase 中第一个音频资产；返回是否成功。</summary>
        public static bool PlayFirst(bool looping)
        {
            return UI.AudioPlayFirstAsset(looping) == 0;
        }

        /// <summary>暂停当前声部。</summary>
        public static bool Pause()
        {
            return UI.AudioPause() == 0;
        }

        /// <summary>恢复当前声部。</summary>
        public static bool Resume()
        {
            return UI.AudioResume() == 0;
        }

        /// <summary>停止当前声部。</summary>
        public static bool Stop()
        {
            return UI.AudioStop() == 0;
        }

        /// <summary>设置主音量（0..1，越界 clamp）。</summary>
        public static float SetVolume(float volume)
        {
            float clamped = float.IsNaN(volume) ? 0f : Math.Max(0f, Math.Min(1f, volume));
            UI.AudioSetMasterVolume(clamped);
            return clamped;
        }

        /// <summary>是否有活动声部（解析 audio 摘要 voices 字段）。</summary>
        public static bool IsPlaying
        {
            get
            {
                string summary = UI.AudioGetSummary();
                return ParseIntField(summary, "voices=") > 0;
            }
        }

        /// <summary>音频后端名（如 cpal / null / off）。</summary>
        public static string Backend
        {
            get
            {
                string summary = UI.AudioGetSummary();
                string backend = ParseStringField(summary, "backend=");
                if (string.IsNullOrEmpty(backend)) backend = "off";
                return backend;
            }
        }

        private static int ParseIntField(string text, string key)
        {
            if (string.IsNullOrEmpty(text)) return 0;
            int index = text.IndexOf(key);
            if (index < 0) return 0;
            int start = index + key.Length;
            int end = start;
            while (end < text.Length && char.IsDigit(text[end])) end++;
            int value;
            if (end > start && int.TryParse(text.Substring(start, end - start), out value))
            {
                return value;
            }
            return 0;
        }

        private static string ParseStringField(string text, string key)
        {
            if (string.IsNullOrEmpty(text)) return "";
            int index = text.IndexOf(key);
            if (index < 0) return "";
            int start = index + key.Length;
            int end = start;
            while (end < text.Length && !char.IsWhiteSpace(text[end])) end++;
            return text.Substring(start, end - start);
        }
    }

    /// <summary>动画预览外观（复用编辑器动画状态机/时间轴 FFI 包装）。</summary>
    public static class Animation
    {
        /// <summary>请求切换到指定状态（需先初始化状态图）。</summary>
        public static bool PlayClip(string stateName)
        {
            if (string.IsNullOrEmpty(stateName)) return false;
            return UI.AnimationPreviewSetState(stateName) == 0;
        }

        /// <summary>设置预览时间（秒；返回实际 clamp 后时间）。</summary>
        public static double SetClipTime(float time)
        {
            return UI.AnimationPreviewSetTime(time);
        }

        /// <summary>当前状态名（未初始化返回空串）。</summary>
        public static string State
        {
            get { return ParseJsonString(UI.AnimationPreviewStateSnapshot(), "current_state"); }
        }

        /// <summary>过渡权重（0..1；未初始化返回 0）。</summary>
        public static float TransitionAlpha
        {
            get { return ParseJsonFloat(UI.AnimationPreviewStateSnapshot(), "transition_alpha", 0f); }
        }

        private static string ParseJsonString(string json, string key)
        {
            if (string.IsNullOrEmpty(json)) return "";
            int index = json.IndexOf("\"" + key + "\":");
            if (index < 0) return "";
            int start = index + key.Length + 3;
            while (start < json.Length && char.IsWhiteSpace(json[start])) start++;
            if (start >= json.Length || json[start] != '"') return "";
            int end = json.IndexOf('"', start + 1);
            if (end < 0) return "";
            return json.Substring(start + 1, end - start - 1);
        }

        private static float ParseJsonFloat(string json, string key, float fallback)
        {
            if (string.IsNullOrEmpty(json)) return fallback;
            int index = json.IndexOf("\"" + key + "\":");
            if (index < 0) return fallback;
            int start = index + key.Length + 3;
            int end = start;
            while (end < json.Length && json[end] != ',' && json[end] != '}') end++;
            float value;
            if (float.TryParse(json.Substring(start, end - start).Trim(),
                System.Globalization.NumberStyles.Float,
                System.Globalization.CultureInfo.InvariantCulture, out value))
            {
                return value;
            }
            return fallback;
        }
    }

    /// <summary>日志外观（转发 DFX Log）。</summary>
    public static class Debug
    {
        private const string Module = "Game";

        public static void Log(string message)
        {
            Hezhou.Log.Info(Module, message ?? "");
        }

        public static void LogWarning(string message)
        {
            Hezhou.Log.Warn(Module, message ?? "");
        }

        public static void LogError(string message)
        {
            Hezhou.Log.Error(Module, message ?? "");
        }
    }
}
