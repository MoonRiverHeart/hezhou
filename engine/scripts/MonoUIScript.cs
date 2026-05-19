using System;
using HezhouUI;
using Hezhou;

#if MONO
namespace HezhouScripts
{
    public class MonoUIScript
    {
        private static UISystem _uiSystem;
        private static ulong _rootId;
        private static ulong _buttonId;
        private static ulong _labelId;
        private static ulong _frameCount;
        
        public static void Initialize()
        {
            Log.Info("C# Mono", "Initialize UI System");
            
            _uiSystem = new UISystem();
            _frameCount = 0;
            
            _rootId = _uiSystem.CreateRootPanel(0, 0, 800, 600);
            Log.Info("C# Mono", $"Root panel created: id={_rootId}");
            
            _buttonId = _uiSystem.AddButton(_rootId, 50, 50, 200, 40, "Click Me");
            Log.Info("C# Mono", $"Button created: id={_buttonId}");
            
            _labelId = _uiSystem.AddLabel(_rootId, 50, 100, 200, 30, "Hello Mono UI!");
            Log.Info("C# Mono", $"Label created: id={_labelId}");
            
            _uiSystem.SetWidgetBackgroundColor(_buttonId, 0.2f, 0.6f, 1.0f, 1.0f);
            _uiSystem.SetWidgetBackgroundColor(_labelId, 0.0f, 0.0f, 0.0f, 0.0f);
        }
        
        public static void Update(float deltaTime)
        {
            _frameCount++;
            
            if (_frameCount % 60 == 0)
            {
                Log.Debug("C# Mono", $"Frame {_frameCount}, deltaTime={deltaTime}");
            }
            
            _uiSystem.Update(deltaTime);
        }
        
        public static void OnTouchBegin(float x, float y, uint pointerId)
        {
            ulong timestamp = _frameCount;
            _uiSystem.DispatchTouchBegin(x, y, pointerId, timestamp);
            Log.Debug("C# Mono", $"TouchBegin: x={x}, y={y}, pointer={pointerId}");
        }
        
        public static void OnTouchEnd(float x, float y, uint pointerId)
        {
            ulong timestamp = _frameCount;
            _uiSystem.DispatchTouchEnd(x, y, pointerId, timestamp);
            Log.Debug("C# Mono", $"TouchEnd: x={x}, y={y}, pointer={pointerId}");
        }
        
        public static void OnKeyDown(KeyCode keycode, KeyModifiers modifiers)
        {
            ulong timestamp = _frameCount;
            uint flags = modifiers.ToFlags();
            _uiSystem.DispatchKeyDown((uint)keycode, flags, timestamp);
            Log.Debug("C# Mono", $"KeyDown: keycode={keycode}, modifiers={flags}");
        }
        
        public static void OnKeyUp(KeyCode keycode, KeyModifiers modifiers)
        {
            ulong timestamp = _frameCount;
            uint flags = modifiers.ToFlags();
            _uiSystem.DispatchKeyUp((uint)keycode, flags, timestamp);
            Log.Debug("C# Mono", $"KeyUp: keycode={keycode}, modifiers={flags}");
        }
        
        public static void Cleanup()
        {
            Log.Info("C# Mono", "Cleanup UI System");
            _uiSystem?.Dispose();
            _uiSystem = null;
        }
    }
}
#endif

#if NATIVEAOT
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

namespace HezhouScripts
{
    public static class NativeAOTUIScript
    {
        private static UISystem _uiSystem;
        private static ulong _rootId;
        private static ulong _buttonId;
        private static ulong _labelId;
        private static ulong _frameCount;
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_ui_initialize", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void Initialize()
        {
            Log.Info("C# NativeAOT", "Initialize UI System");
            
            _uiSystem = new UISystem();
            _frameCount = 0;
            
            _rootId = _uiSystem.CreateRootPanel(0, 0, 800, 600);
            _buttonId = _uiSystem.AddButton(_rootId, 50, 50, 200, 40, "Click Me");
            _labelId = _uiSystem.AddLabel(_rootId, 50, 100, 200, 30, "Hello NativeAOT UI!");
            
            _uiSystem.SetWidgetBackgroundColor(_buttonId, 0.2f, 0.6f, 1.0f, 1.0f);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_ui_update", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void Update(float deltaTime)
        {
            _frameCount++;
            _uiSystem.Update(deltaTime);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_ui_touch_begin", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void OnTouchBegin(float x, float y, uint pointerId)
        {
            _uiSystem.DispatchTouchBegin(x, y, pointerId, _frameCount);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_ui_touch_end", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void OnTouchEnd(float x, float y, uint pointerId)
        {
            _uiSystem.DispatchTouchEnd(x, y, pointerId, _frameCount);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_ui_key_down", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void OnKeyDown(uint keycode, uint modifiers)
        {
            _uiSystem.DispatchKeyDown(keycode, modifiers, _frameCount);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_ui_key_up", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void OnKeyUp(uint keycode, uint modifiers)
        {
            _uiSystem.DispatchKeyUp(keycode, modifiers, _frameCount);
        }
        
        [UnmanagedCallersOnly(EntryPoint = "csharp_ui_cleanup", CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void Cleanup()
        {
            _uiSystem?.Dispose();
            _uiSystem = null;
        }
    }
}
#endif