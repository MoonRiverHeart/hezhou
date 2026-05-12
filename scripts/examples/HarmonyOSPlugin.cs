using System;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

public class HarmonyOSPlugin : ScriptPlugin
{
    private int _viewportWidth;
    private int _viewportHeight;

    public override void OnLoad()
    {
        Console.WriteLine("[HarmonyOSPlugin] OnLoad - registering event handlers");

        RegisterSyncCallback("OnTouch", arg =>
        {
            var action = arg.GetInt() ?? 0;
            var x = arg.GetFloat() ?? 0f;
            var y = arg.GetDouble() ?? 0d;
            
            Console.WriteLine($"[C#] Touch event: action={action}, x={x}, y={y}");
            HandleTouch(action, (float)y);
            
            return ScriptValue.Null;
        }, "Handle touch events from HarmonyOS");

        RegisterSyncCallback("OnKey", arg =>
        {
            var keycode = arg.GetInt() ?? 0;
            Console.WriteLine($"[C#] Key event: keycode={keycode}");
            HandleKey(keycode);
            
            return ScriptValue.Null;
        }, "Handle key events from HarmonyOS");

        RegisterSyncCallback("OnResize", arg =>
        {
            _viewportWidth = arg.GetInt() ?? 0;
            Console.WriteLine($"[C#] Resize: width={_viewportWidth}");
            AdjustViewport(_viewportWidth, _viewportHeight);
            
            return ScriptValue.Null;
        }, "Handle window resize");

        RegisterSyncCallback("OnLifecycle", arg =>
        {
            var state = arg.GetInt() ?? 0;
            Console.WriteLine($"[C#] Lifecycle state: {state}");
            HandleLifecycle(state);
            
            return ScriptValue.Null;
        }, "Handle application lifecycle");
    }

    private void HandleTouch(int action, float y)
    {
        switch (action)
        {
            case 0:
                Console.WriteLine("[Game] Touch started");
                break;
            case 1:
                Console.WriteLine("[Game] Touch moved");
                break;
            case 2:
                Console.WriteLine("[Game] Touch ended");
                break;
        }
    }

    private void HandleKey(int keycode)
    {
        if (keycode == 1001)
        {
            Console.WriteLine("[Game] Back button pressed");
        }
    }

    private void AdjustViewport(int width, int height)
    {
        Console.WriteLine($"[Game] Viewport adjusted to {width}x{height}");
    }

    private void HandleLifecycle(int state)
    {
        switch (state)
        {
            case 0:
                Console.WriteLine("[Game] App created");
                break;
            case 2:
                Console.WriteLine("[Game] App resumed - start rendering");
                break;
            case 3:
                Console.WriteLine("[Game] App paused - stop rendering");
                break;
            case 5:
                Console.WriteLine("[Game] App destroyed - cleanup");
                break;
        }
    }

    public override void OnUnload()
    {
        Console.WriteLine("[HarmonyOSPlugin] OnUnload");
    }
}