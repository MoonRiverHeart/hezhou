using System;
using System.Runtime.InteropServices;
using Hezhou;

public static class UIScript
{
    private static ulong _buttonId;
    private static ulong _labelId;
    private static DFX _dfx;

    public static void Initialize()
    {
        _dfx = DFX.Create();
        _dfx.SetLogLevel(LogLevel.Info);
        _dfx.Log("UI初始化开始");

        var root = UI.CreateRootPanel(800, 600);
        _dfx.Log($"创建根面板: {root}");

        _labelId = UI.AddLabel(root, "Hello Mono UI!");
        _dfx.Log($"创建Label: {_labelId}");

        _buttonId = UI.AddButton(root, "Click Me");
        _dfx.Log($"创建Button: {_buttonId}");

        UI.SetOnClick(_buttonId, OnButtonClick);
        _dfx.Log($"注册OnClick回调");

        UI.RegisterUpdateCallback(Update);
        _dfx.Log($"注册Update回调");

        _dfx.Log("UI初始化完成");
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static void OnButtonClick(ulong widgetId)
    {
        _dfx.Log($"按钮 {widgetId} 被点击", LogLevel.Info);
        UI.SetText(widgetId, "Clicked!");
        UI.SetText(_labelId, "Button was clicked!");
        UI.SetBackgroundColor(widgetId, 0.2f, 0.8f, 0.2f, 1.0f);
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static void Update(float deltaTime)
    {
    }

    public static void ResetAll()
    {
        _dfx?.Log("ResetAll调用", LogLevel.Debug);
        _buttonId = 0;
        _labelId = 0;
        _dfx?.Dispose();
        Initialize();
    }

    public static void SetHandles(IntPtr widgetTree, IntPtr eventDispatcher)
    {
        UI.SetHandles(widgetTree, eventDispatcher);
    }
}