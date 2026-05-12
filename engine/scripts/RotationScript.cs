using System;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

namespace HezhouScripts
{
    public static class RotationController
    {
        private static float _rotationSpeed = 90.0f;
        private static float _currentAngle = 0.0f;
        
        public static unsafe delegate* unmanaged[Cdecl]<float, float> CalculateRotationPtr;
        
        public static unsafe void Initialize()
        {
            CalculateRotationPtr = &CalculateRotation;
            
            NativeMethods.register_rotation_callback(CalculateRotationPtr);
            
            Console.WriteLine($"[C#] Initialized, rotation_speed = {_rotationSpeed}°/s");
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_initialize")]
        public static void ExportInitialize()
        {
            Initialize();
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static float CalculateRotation(float deltaTime)
        {
            float angleIncrement = _rotationSpeed * deltaTime;
            _currentAngle += angleIncrement;
            
            if (_currentAngle >= 360.0f)
            {
                _currentAngle -= 360.0f;
            }
            
            Console.WriteLine($"[C#] CalculateRotation: dt={deltaTime:F4}s, increment={angleIncrement:F2}°, angle={_currentAngle:F1}°");
            
            return angleIncrement;
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void SetRotationSpeed(float speed)
        {
            _rotationSpeed = speed;
            Console.WriteLine($"[C#] SetRotationSpeed: {speed}°/s");
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static float GetRotationSpeed()
        {
            return _rotationSpeed;
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void ResetRotation()
        {
            _currentAngle = 0.0f;
            Console.WriteLine($"[C#] ResetRotation: angle=0°");
        }
    }
    
    internal static class NativeMethods
    {
        [DllImport("hezhou_scripting", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe void register_rotation_callback(delegate* unmanaged[Cdecl]<float, float> callback);
        
        [DllImport("hezhou_scripting", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe float trigger_rotation_callback(float deltaTime);
    }
}