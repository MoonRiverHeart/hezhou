using System;
using System.Runtime.InteropServices;

public class ButtonClickTest
{
    [DllImport("hezhou_ui", CallingConvention = CallingConvention.Cdecl)]
    public static extern void ui_widget_set_text(
        IntPtr handle, ulong widget_id, [MarshalAs(UnmanagedType.LPUTF8Str)] string text);
    
    private static IntPtr widgetTreeHandle;
    private static ulong buttonId;
    
    public static void Initialize(IntPtr handle, ulong id)
    {
        Console.WriteLine("[C#] Initialize called");
        Console.WriteLine("[C#] WidgetTreeHandle: " + handle);
        Console.WriteLine("[C#] ButtonId: " + id);
        
        widgetTreeHandle = handle;
        buttonId = id;
        
        Console.WriteLine("[C#] Initialize complete");
    }
    
    public static void OnButtonClick(ulong widgetId)
    {
        Console.WriteLine("[C#] ========== BUTTON CLICKED! ==========");
        Console.WriteLine("[C#] WidgetId: " + widgetId);
        Console.WriteLine("[C#] ButtonId: " + buttonId);
        Console.WriteLine("[C#] WidgetTreeHandle: " + widgetTreeHandle);
        
        if (widgetTreeHandle != IntPtr.Zero)
        {
            Console.WriteLine("[C#] Calling ui_widget_set_text...");
            ui_widget_set_text(widgetTreeHandle, buttonId, "hello");
            Console.WriteLine("[C#] ui_widget_set_text called successfully");
        }
        else
        {
            Console.WriteLine("[C#] ERROR: WidgetTreeHandle is null!");
        }
        
        Console.WriteLine("[C#] =====================================");
    }
    
    public static void Cleanup()
    {
        Console.WriteLine("[C#] Cleanup called");
    }
}