using System;
using Hezhou;

public static class TestScript
{
    private static VStack _vstack;
    private static Label _label;
    private static Button _button1;
    private static Button _button2;
    private static UI.WidgetCallbackDelegate _button1Callback;
    private static UI.WidgetCallbackDelegate _button2Callback;
    private static UI.ResizeCallbackDelegate _resizeCallback;
    
    private static float _screenWidth = 800f;
    private static float _screenHeight = 600f;
    
    public static void Initialize(IntPtr ffiContextPtr)
    {
        Log.Info("C#", "TestScript Initialize开始");
        
        try {
            UI.InitFromContext(ffiContextPtr);
            
            UI.GetScreenSize(out _screenWidth, out _screenHeight);
            Log.Info("C#", $"屏幕大小: {_screenWidth}x{_screenHeight}");
            
            CreateUI();
            
            _resizeCallback = OnResize;
            UI.RegisterResizeCallback(_resizeCallback);
            
            Log.Info("C#", "UI创建完成（使用VStack布局）");
        } catch (Exception e) {
            Log.Error("C#", e.Message);
            Log.Error("C#", "StackTrace: " + e.StackTrace);
        }
    }
    
    private static void CreateUI()
    {
        float centerX = _screenWidth / 2f - 100f;
        float centerY = _screenHeight / 2f - 80f;
        
        _vstack = new VStack(spacing: 10f);
        _vstack.SetPosition(centerX, centerY);
        
        _label = new Label(_vstack.Id, 200f, 30f, "Hello from C#");
        
        _button1 = new Button(_vstack.Id, 200f, 50f, "Button 1");
        _button1Callback = OnButton1Click;
        _button1.SetOnClick(_button1Callback);
        
        _button2 = new Button(_vstack.Id, 200f, 50f, "Button 2");
        _button2Callback = OnButton2Click;
        _button2.SetOnClick(_button2Callback);
    }
    
    private static void OnResize(float width, float height)
    {
        Log.Info("C#", $"Resize: {width}x{height}");
        _screenWidth = width;
        _screenHeight = height;
        
        float centerX = _screenWidth / 2f - 100f;
        float centerY = _screenHeight / 2f - 80f;
        _vstack.SetPosition(centerX, centerY);
    }
    
    private static void OnButton1Click(ulong widgetId)
    {
        Log.Info("C#", "Button 1 clicked!");
        _button1.Text = "hello";
    }
    
    private static void OnButton2Click(ulong widgetId)
    {
        Log.Info("C#", "Button 2 clicked!");
        _button2.Text = "hello";
    }
}