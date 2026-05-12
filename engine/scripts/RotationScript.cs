using System;
using System.Runtime.InteropServices;

namespace HezhouScripts
{
    public static class RotationController
    {
        private static float _currentAngle = 0.0f;
        private static float _rotationSpeed = 90.0f;
        
        [UnmanagedCallersOnly(EntryPoint = "calculate_rotation")]
        public static float CalculateRotation(float deltaTime)
        {
            _currentAngle += _rotationSpeed * deltaTime;
            
            if (_currentAngle >= 360.0f)
            {
                _currentAngle -= 360.0f;
            }
            
            return _currentAngle;
        }
        
        [UnmanagedCallersOnly(EntryPoint = "set_rotation_speed")]
        public static void SetRotationSpeed(float speed)
        {
            _rotationSpeed = speed;
        }
        
        [UnmanagedCallersOnly(EntryPoint = "get_rotation_speed")]
        public static float GetRotationSpeed()
        {
            return _rotationSpeed;
        }
        
        [UnmanagedCallersOnly(EntryPoint = "reset_rotation")]
        public static void ResetRotation()
        {
            _currentAngle = 0.0f;
        }
    }
}