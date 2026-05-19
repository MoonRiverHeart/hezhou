using System;
using System.Runtime.InteropServices;
using Hezhou;

public static class UIScript
{
    private static ulong _buttonId;
    private static ulong _labelId;
    private static UI.UpdateCallbackDelegate _updateDelegate;
    private static UI.WidgetCallbackDelegate _onClickDelegate;

    public static void Initialize(IntPtr widgetTree)
    {
        Log.Info("C#", "Initialize开始");
        
        UI.SetWidgetTree(widgetTree);
        
        var root = UI.CreateRootPanel(800, 600);
        Log.Info("C#", "创建根面板: " + root);
        
        _labelId = UI.AddLabel(root, "Welcome to Hezhou UI!");
        Log.Info("C#", "创建Label: " + _labelId);
        
        _buttonId = UI.AddButton(root, "Click Me");
        Log.Info("C#", "创建Button: " + _buttonId);
        
        _onClickDelegate = OnButtonClick;
        UI.SetOnClick(_buttonId, _onClickDelegate);
        Log.Info("C#", "注册OnClick回调");
        
        _updateDelegate = Update;
        UI.RegisterUpdateCallback(_updateDelegate);
        Log.Info("C#", "注册Update回调");
        
        Log.Info("C#", "Initialize完成");
    }

    public static void OnButtonClick(ulong widgetId)
    {
        Log.Info("C#", "Button " + widgetId + " clicked!");
        UI.SetText(widgetId, "Clicked!");
        UI.SetText(_labelId, "Button was clicked!");
    }

    public static void Update(float deltaTime)
    {
    }

    public static void ResetAll(IntPtr widgetTree)
    {
        Log.Info("C#", "ResetAll");
        _buttonId = 0;
        _labelId = 0;
        _onClickDelegate = null;
        _updateDelegate = null;
        Initialize(widgetTree);
    }
}