using System;
using System.Collections.Generic;
using Hezhou;

namespace HezhouScripts
{
    /// <summary>
    /// 每实体旋转脚本 — 每个Entity拥有独立的旋转速度和角度偏移。
    /// 
    /// 与旧的RotationController（静态单例，所有Entity共享同一速度）不同，
    /// RotatingEntity为每个绑定脚本的Entity创建独立实例，
    /// 实例存储在静态字典 _instances 中，Rust侧通过instanceKey（IntPtr）引用。
    /// 
    /// 遵循ScriptEntityHelper定义的脚本实体约定（见IScriptEntity.cs）：
    ///   - 静态字典 _instances 存储所有实例（GC安全）
    ///   - 静态方法 GetPropertyDescriptors/GetPropertyValue/SetPropertyValue/UpdateInstance
    ///   - 静态方法 CreateInstance/DestroyInstance/SetScenePtr/ResetAll 用于生命周期管理
    /// 
    /// 脚本生命周期：
    ///   1. Rust调用 SetScenePtr(scenePtr) — 设置Scene指针
    ///   2. Rust调用 CreateInstance() — 为Entity创建脚本实例，返回instanceKey
    ///   3. Rust将instanceKey存储在ScriptBinding中
    ///   4. Rust每帧调用 UpdateInstance(instanceKey, deltaTime, entityId)
    ///   5. 编辑器UI调用 GetPropertyDescriptors → 渲染属性控件
    ///   6. 编辑器UI修改属性 → Rust调用 SetPropertyValue(instanceKey, name, value)
    ///   7. Entity销毁时 → Rust调用 DestroyInstance(instanceKey)
    /// 
    /// FFI旋转调用：
    ///   当前使用 UI.SceneRotateEntity(scene, entityId, deltaAngle) — 增量旋转
    ///   未来应添加 UI.SceneSetEntityRotation(scene, entityId, absoluteAngle) — 绝对旋转
    ///   绝对旋转可支持动态修改angleOffset（增量旋转无法动态调整偏移）
    /// </summary>
    public class RotatingEntity
    {
        // ===== 每实例属性（[Expose]标记，编辑器可见） =====

        /// <summary>旋转速度（度/秒），每个Entity独立控制</summary>
        [Expose(DisplayName = "旋转速度", Category = "运动", Min = 0, Max = 100, Step = 1, Initial = 10, Widget = "slider")]
        private float rotationSpeed = 10.0f;

        /// <summary>旋转角度偏移（度），用于调整初始旋转方向</summary>
        [Expose(DisplayName = "旋转角度偏移", Category = "运动", Min = -360, Max = 360, Step = 1, Initial = 0.0f, Widget = "input")]
        private float angleOffset = 0.0f;

        // ===== 每实例内部状态（编辑器不可见） =====

        /// <summary>当前累计旋转角度（度）</summary>
        private float _currentAngle = 0.0f;

        // ===== 静态实例管理 =====

        /// <summary>下一个实例ID（从1开始，0表示无效）</summary>
        private static long _nextInstanceId = 1;

        /// <summary>
        /// 实例存储字典 — key是instanceId（long），value是RotatingEntity实例。
        /// 静态字典持有强引用，防止Mono GC回收实例对象。
        /// Rust侧存储instanceId（以IntPtr形式），通过FFI传回C#查找实例。
        /// </summary>
        private static Dictionary<long, RotatingEntity> _instances = new Dictionary<long, RotatingEntity>();

        /// <summary>Scene指针 — Rust侧在初始化时设置，用于FFI旋转调用</summary>
        private static IntPtr _scenePtr = IntPtr.Zero;

        // ===== 生命周期管理方法 =====

        /// <summary>
        /// 设置Scene指针 — Rust侧在初始化脚本前调用此方法。
        /// Scene指针用于调用SceneRotateEntity等FFI方法。
        /// </summary>
        /// <param name="scenePtr">Scene对象的指针（由Rust侧SceneCreate返回）</param>
        public static void SetScenePtr(IntPtr scenePtr)
        {
            _scenePtr = scenePtr;
            Log.Info("C#", $"RotatingEntity.SetScenePtr: scenePtr={scenePtr}");
        }

        /// <summary>
        /// 创建新实例 — Rust侧为Entity绑定脚本时调用。
        /// 创建一个新的RotatingEntity实例，存入_instances字典，
        /// 返回instanceKey（IntPtr），Rust存储在ScriptBinding中。
        /// </summary>
        /// <returns>实例key（IntPtr形式），Rust侧作为instancePtr使用</returns>
        public static IntPtr CreateInstance()
        {
            long id = _nextInstanceId++;
            RotatingEntity instance = new RotatingEntity();
            _instances[id] = instance;
            Log.Info("C#", $"RotatingEntity.CreateInstance: id={id}, rotationSpeed={instance.rotationSpeed}, angleOffset={instance.angleOffset}");
            return new IntPtr(id);
        }

        /// <summary>
        /// 销毁实例 — Rust侧移除脚本绑定时调用。
        /// 从_instances字典中移除实例，释放C#侧引用。
        /// </summary>
        /// <param name="instancePtr">要销毁的实例key</param>
        public static void DestroyInstance(IntPtr instancePtr)
        {
            long key = instancePtr.ToInt64();
            if (_instances.ContainsKey(key))
            {
                _instances.Remove(key);
                Log.Info("C#", $"RotatingEntity.DestroyInstance: key={key}");
            }
        }

        // ===== 脚本实体约定方法（4个核心静态方法） =====

        /// <summary>
        /// 返回属性描述符 — Rust侧调用以发现[Expose]字段。
        /// 编辑器UI根据描述符渲染Input/Slider控件。
        /// </summary>
        /// <returns>ScriptPropertyDescriptor数组，每个[Expose]字段一个描述符</returns>
        public static ScriptPropertyDescriptor[] GetPropertyDescriptors()
        {
            return ScriptEntityHelper.ReflectProperties(typeof(RotatingEntity));
        }

        /// <summary>
        /// 读取实例属性值 — Rust侧调用以获取编辑器UI显示的值。
        /// instancePtr是字典key，ScriptEntityHelper通过反射查找实例并读取字段。
        /// </summary>
        /// <param name="instancePtr">实例字典key</param>
        /// <param name="propertyName">要读取的字段名</param>
        /// <returns>字段值（float）</returns>
        public static float GetPropertyValue(IntPtr instancePtr, string propertyName)
        {
            return ScriptEntityHelper.GetFieldValue(instancePtr, propertyName, typeof(RotatingEntity));
        }

        /// <summary>
        /// 写入实例属性值 — Rust侧调用以设置编辑器UI修改的值。
        /// instancePtr是字典key，ScriptEntityHelper通过反射查找实例并写入字段。
        /// </summary>
        /// <param name="instancePtr">实例字典key</param>
        /// <param name="propertyName">要写入的字段名</param>
        /// <param name="value">要写入的float值</param>
        public static void SetPropertyValue(IntPtr instancePtr, string propertyName, float value)
        {
            ScriptEntityHelper.SetFieldValue(instancePtr, propertyName, value, typeof(RotatingEntity));
            Log.Info("C#", $"RotatingEntity.SetPropertyValue: {propertyName}={value}");
        }

        /// <summary>
        /// 每帧更新 — Rust侧每帧调用，传入instanceKey和entityId。
        /// 
        /// 逻辑：
        ///   1. 从_instances字典查找实例
        ///   2. _currentAngle += rotationSpeed * deltaTime
        ///   3. 角度归一化到0-360范围
        ///   4. 调用UI.SceneRotateEntity应用增量旋转
        /// 
        /// 注意：当前使用SceneRotateEntity（增量旋转，每帧加deltaAngle度）。
        /// angleOffset在CreateInstance时通过SceneRotateEntity一次性应用。
        /// 未来添加SceneSetEntityRotation后，可改为绝对旋转：
        ///   UI.SceneSetEntityRotation(_scenePtr, entityId, _currentAngle + angleOffset)
        /// 这样angleOffset可动态修改（编辑器拖动slider立即生效）。
        /// </summary>
        /// <param name="instancePtr">实例字典key</param>
        /// <param name="deltaTime">帧间隔时间（秒）</param>
        /// <param name="entityId">Entity ID（用于FFI旋转调用）</param>
        public static void UpdateInstance(IntPtr instancePtr, float deltaTime, ulong entityId)
        {
            long key = instancePtr.ToInt64();

            RotatingEntity instance;
            if (!_instances.TryGetValue(key, out instance))
            {
                return;
            }

            // 计算增量角度
            float deltaAngle = instance.rotationSpeed * deltaTime;

            // 更新累计角度
            instance._currentAngle += deltaAngle;

            // 角度归一化到0-360范围
            if (instance._currentAngle >= 360.0f)
            {
                instance._currentAngle -= 360.0f;
            }
            else if (instance._currentAngle < 0.0f)
            {
                instance._currentAngle += 360.0f;
            }

            // 通过FFI应用增量旋转
            // SceneRotateEntity(scene, entityId, angleDegrees) — 每帧旋转deltaAngle度
            if (_scenePtr != IntPtr.Zero)
            {
                UI.SceneRotateEntity(_scenePtr, entityId, deltaAngle);
            }
        }

        // ===== 热重载支持 =====

        /// <summary>
        /// 重置所有实例 — 热重载时由Rust侧调用。
        /// 清除_instances字典，重置计数器，清空scene指针。
        /// </summary>
        public static void ResetAll()
        {
            _instances.Clear();
            _nextInstanceId = 1;
            _scenePtr = IntPtr.Zero;
            Log.Info("C#", "RotatingEntity.ResetAll: 所有实例已清除");
        }

        // ===== 方向7: 碰撞回调约定方法 =====

        /// <summary>
        /// 碰撞开始回调 — 当Entity与其他Entity发生碰撞时由Rust侧EventFlushSystem调用。
        /// entityId是当前实体ID，otherEntityId是碰撞对象ID。
        /// 注意：双向通知，两个碰撞实体都会收到此回调。
        /// </summary>
        /// <param name="entityId">当前实体ID</param>
        /// <param name="otherEntityId">碰撞对象实体ID</param>
        public static void OnCollisionStart(ulong entityId, ulong otherEntityId)
        {
            Log.Info("C#", "RotatingEntity " + entityId + " collided with " + otherEntityId);
        }

        /// <summary>
        /// 碰撞结束回调 — 当Entity与其他Entity停止碰撞时由Rust侧EventFlushSystem调用。
        /// </summary>
        /// <param name="entityId">当前实体ID</param>
        /// <param name="otherEntityId">碰撞对象实体ID</param>
        public static void OnCollisionStop(ulong entityId, ulong otherEntityId)
        {
            Log.Info("C#", "RotatingEntity " + entityId + " stopped colliding with " + otherEntityId);
        }
    }
}