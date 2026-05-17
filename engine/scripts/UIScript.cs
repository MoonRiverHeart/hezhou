using System;
using System.Runtime.InteropServices;
using Hezhou;

public static class UIScript
{
    private static UI.UpdateCallbackDelegate _updateDelegate;

    public static void Initialize()
    {
        Console.WriteLine("[C#] Initialize开始");
        
        _updateDelegate = Update;
        UI.RegisterUpdateCallback(_updateDelegate);
        Console.WriteLine("[C#] 注册Update回调成功");
        
        Console.WriteLine("[C#] Initialize完成");
    }

    public static void Update(float deltaTime)
    {
        Console.WriteLine("[C#] Update called: deltaTime=" + deltaTime);
    }

    public static void ResetAll()
    {
        Console.WriteLine("[C#] ResetAll调用");
        _updateDelegate = null;
        Initialize();
    }
}