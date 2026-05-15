using System;

#if NATIVEAOT
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
#endif

namespace HezhouScripts
{
    public class RotationController
    {
        private static float _rotationSpeed = 90.0f;
        private static float _currentAngle = 0.0f;
        
        public static float UpdateRotation(float deltaTime)
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
            return _rotationSpeed;
        }
        
        public static void SetRotationSpeed(float speed)
        {
            _rotationSpeed = speed;
            Console.WriteLine($"[C#] SetRotationSpeed: {speed}°/s");
        }
        
        public static void ResetRotation()
        {
            _currentAngle = 0.0f;
            Console.WriteLine($"[C#] ResetRotation: angle=0°");
        }
        
        public static float GetCurrentAngle()
        {
            return _currentAngle;
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
            Console.WriteLine($"[C# NativeAOT] Initialized, rotation_speed = {_rotationSpeed}°/s");
        }
#endif
    }
}