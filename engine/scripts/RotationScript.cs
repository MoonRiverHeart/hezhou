using System;
using System.Collections.Generic;
using Hezhou;

#if NATIVEAOT
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
#endif

namespace HezhouScripts
{
    public class RotationController
    {
        private const float DefaultRotationSpeed = 9.0f;
        
        private static RotationController _instance;
        private float _rotationSpeed;
        private float _currentAngle;
        
        private RotationController()
        {
            _rotationSpeed = DefaultRotationSpeed;
            _currentAngle = 0.0f;
        }
        
        private static RotationController GetInstance()
        {
            if (_instance == null)
            {
                _instance = new RotationController();
            }
            return _instance;
        }
        
        public static float UpdateRotation(float deltaTime)
        {
            return GetInstance()._UpdateRotation(deltaTime);
        }
        
        private float _UpdateRotation(float deltaTime)
        {
            float angleIncrement = _rotationSpeed * deltaTime;
            _currentAngle += angleIncrement;
            
            if (_currentAngle >= 360.0f)
            {
                _currentAngle -= 360.0f;
            }
            
            return angleIncrement;
        }
        
        public static float GetRotationSpeed()
        {
            return GetInstance()._rotationSpeed;
        }
        
        public static void SetRotationSpeed(float speed)
        {
            GetInstance()._rotationSpeed = speed;
            Log.Info("C#", $"SetRotationSpeed: {speed}°/s");
        }
        
        public static void ResetRotation()
        {
            GetInstance()._currentAngle = 0.0f;
            Log.Info("C#", "ResetRotation: angle=0°");
        }
        
        public static void ResetAll()
        {
            _instance = null;
            GetInstance();
            Log.Info("C#", $"ResetAll complete, speed={GetInstance()._rotationSpeed}°/s");
        }
        
        public static float GetCurrentAngle()
        {
            return GetInstance()._currentAngle;
        }
        
#if NATIVEAOT
        [UnmanagedCallersOnly(EntryPoint = "csharp_update_rotation", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static float ExportUpdateRotation(float deltaTime)
        {
            return UpdateRotation(deltaTime);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_get_rotation_speed", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static float ExportGetRotationSpeed()
        {
            return GetRotationSpeed();
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_set_rotation_speed", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void ExportSetRotationSpeed(float speed)
        {
            SetRotationSpeed(speed);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_reset_rotation", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void ExportResetRotation()
        {
            ResetRotation();
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_initialize", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void ExportInitialize()
        {
            Log.Info("C# NativeAOT", $"Initialized, rotation_speed = {GetInstance()._rotationSpeed}°/s");
        }
#endif
    }

    /// <summary>
    /// 编辑器脚本绑定用的逐实体旋转脚本（与文件名 RotationScript.cs 对应）。
    /// RotationController 是旧版静态单例，不能按 Entity 创建实例；本类实现
    /// ScriptEntityHelper 约定，供“运动”标签页绑定/属性面板使用。
    /// </summary>
    public class RotationScript
    {
        [Expose(DisplayName = "旋转速度", Category = "运动", Min = 0, Max = 180, Step = 1, Initial = 45, Widget = "slider")]
        private float rotationSpeed = 45.0f;

        [Expose(DisplayName = "初始角度", Category = "运动", Min = -360, Max = 360, Step = 1, Initial = 0, Widget = "input")]
        private float initialAngle = 0.0f;

        private float _currentAngle = 0.0f;
        private bool _initialized = false;
        private static long _nextInstanceId = 1;
        private static Dictionary<long, RotationScript> _instances = new Dictionary<long, RotationScript>();
        private static IntPtr _scenePtr = IntPtr.Zero;

        public static void SetScenePtr(IntPtr scenePtr)
        {
            _scenePtr = scenePtr;
            Log.Info("C#", "RotationScript.SetScenePtr: " + scenePtr);
        }

        public static IntPtr CreateInstance()
        {
            long id = _nextInstanceId++;
            RotationScript instance = new RotationScript();
            _instances[id] = instance;
            Log.Info("C#", "RotationScript.CreateInstance: id=" + id + ", speed=" + instance.rotationSpeed);
            return new IntPtr(id);
        }

        public static void DestroyInstance(IntPtr instancePtr)
        {
            long key = instancePtr.ToInt64();
            if (_instances.Remove(key))
            {
                Log.Info("C#", "RotationScript.DestroyInstance: key=" + key);
            }
        }

        public static ScriptPropertyDescriptor[] GetPropertyDescriptors()
        {
            return ScriptEntityHelper.ReflectProperties(typeof(RotationScript));
        }

        public static float GetPropertyValue(IntPtr instancePtr, string propertyName)
        {
            return ScriptEntityHelper.GetFieldValue(instancePtr, propertyName, typeof(RotationScript));
        }

        public static void SetPropertyValue(IntPtr instancePtr, string propertyName, float value)
        {
            ScriptEntityHelper.SetFieldValue(instancePtr, propertyName, value, typeof(RotationScript));
            Log.Info("C#", "RotationScript.SetPropertyValue: " + propertyName + "=" + value);
        }

        public static void UpdateInstance(IntPtr instancePtr, float deltaTime, ulong entityId)
        {
            RotationScript instance;
            if (!_instances.TryGetValue(instancePtr.ToInt64(), out instance))
            {
                return;
            }

            if (!instance._initialized)
            {
                instance._currentAngle = instance.initialAngle;
                instance._initialized = true;
                if (_scenePtr != IntPtr.Zero)
                {
                    UI.SceneRotateEntity(_scenePtr, entityId, instance.initialAngle);
                }
            }

            float deltaAngle = instance.rotationSpeed * deltaTime;
            instance._currentAngle += deltaAngle;
            if (instance._currentAngle >= 360.0f) instance._currentAngle -= 360.0f;
            if (instance._currentAngle < 0.0f) instance._currentAngle += 360.0f;

            if (_scenePtr != IntPtr.Zero)
            {
                UI.SceneRotateEntity(_scenePtr, entityId, deltaAngle);
            }
        }

        public static void ResetAll()
        {
            _instances.Clear();
            _nextInstanceId = 1;
            _scenePtr = IntPtr.Zero;
            Log.Info("C#", "RotationScript.ResetAll");
        }
    }
}