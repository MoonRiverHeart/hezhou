using System;
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
}