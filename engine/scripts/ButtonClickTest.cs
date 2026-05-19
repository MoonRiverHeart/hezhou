using System;
using System.Runtime.InteropServices;
using Hezhou;

public class ButtonClickTest
{
    [DllImport("hezhou_ui", CallingConvention = CallingConvention.Cdecl)]
    public static extern void ui_widget_set_text(
        IntPtr handle, ulong widget_id, [MarshalAs(UnmanagedType.LPUTF8Str)] string text);
    
    private static IntPtr widgetTreeHandle;
    private static ulong buttonId;
    
    public static void Initialize(IntPtr handle, ulong id)
    {
        Log.Info("C#", "Initialize called");
        Log.Info("C#", $"WidgetTreeHandle: {handle}");
        Log.Info("C#", $"ButtonId: {id}");
        
        widgetTreeHandle = handle;
        buttonId = id;
        
        Log.Info("C#", "Initialize complete");
    }
    
    public static void OnButtonClick(ulong widgetId)
    {
        Log.Info("C#", "========== BUTTON CLICKED! ==========");
        Log.Info("C#", $"WidgetId: {widgetId}");
        Log.Info("C#", $"ButtonId: {buttonId}");
        Log.Info("C#", $"WidgetTreeHandle: {widgetTreeHandle}");
        
        if (widgetTreeHandle != IntPtr.Zero)
        {
            Log.Info("C#", "Calling ui_widget_set_text...");
            ui_widget_set_text(widgetTreeHandle, buttonId, "hello");
            Log.Info("C#", "ui_widget_set_text called successfully");
        }
        else
        {
            Log.Error("C#", "WidgetTreeHandle is null!");
        }
        
        Log.Info("C#", "=====================================");
    }
    
    public static void Cleanup()
    {
        Log.Info("C#", "Cleanup called");
    }
}