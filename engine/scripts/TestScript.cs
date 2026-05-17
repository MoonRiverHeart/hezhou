using System;
using Hezhou;

public static class TestScript
{
    private static Label _label;
    private static Button _button;
    private static UI.WidgetCallbackDelegate _clickCallback;
    private static UI.ResizeCallbackDelegate _resizeCallback;
    
    private static float _screenWidth = 800f;
    private static float _screenHeight = 600f;
    
    public static void Initialize(IntPtr ffiContextPtr)
    {
        Console.WriteLine("[C#] TestScript Initialize开始");
        
        try {
            UI.InitFromContext(ffiContextPtr);
            
            UI.GetScreenSize(out _screenWidth, out _screenHeight);
            Console.WriteLine($"[C#] 屏幕大小: {_screenWidth}x{_screenHeight}");
            
            CreateWidgets();
            
            _resizeCallback = OnResize;
            UI.RegisterResizeCallback(_resizeCallback);
            
            Console.WriteLine("[C#] UI控件创建完成");
        } catch (Exception e) {
            Console.WriteLine("[C#] ERROR: " + e.Message);
            Console.WriteLine("[C#] StackTrace: " + e.StackTrace);
        }
    }
    
    private static void CreateWidgets()
    {
        float labelWidth = 400f;
        float labelHeight = 50f;
        float labelX = (_screenWidth - labelWidth) / 2f;
        float labelY = _screenHeight / 2f - labelHeight - 25f;
        
        if (_label == null)
        {
            _label = new Label("Hello from C#", labelX, labelY, labelWidth, labelHeight);
            _label.EnsureCreated();
        }
        else
        {
            UI.SetPosition(_label.Id, labelX, labelY);
        }
        
        float buttonWidth = 200f;
        float buttonHeight = 50f;
        float buttonX = (_screenWidth - buttonWidth) / 2f;
        float buttonY = _screenHeight / 2f + 25f;
        
        if (_button == null)
        {
            _button = new Button("Click Me!", buttonX, buttonY, buttonWidth, buttonHeight);
            _clickCallback = OnButtonClick;
            _button.SetOnClick(_clickCallback);
            _button.EnsureCreated();
        }
        else
        {
            UI.SetPosition(_button.Id, buttonX, buttonY);
        }
    }
    
    private static void OnResize(float width, float height)
    {
        Console.WriteLine($"[C#] Resize: {width}x{height}");
        _screenWidth = width;
        _screenHeight = height;
        CreateWidgets();
    }
    
    private static void OnButtonClick(ulong widgetId)
    {
        Console.WriteLine("[C#] Button clicked!");
        _label.Text = "Button was clicked!";
        _button.Text = "Clicked!";
    }
}