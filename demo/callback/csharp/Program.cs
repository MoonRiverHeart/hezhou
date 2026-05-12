using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using CsBindgen;

class Program
{
    static void Main()
    {
        Console.WriteLine("=== C# Lambda 回调到 Rust 示例 ===\n");
        
        // 使用静态字段模拟捕获
        multiplier = 10;
        
        unsafe
        {
            delegate* unmanaged[Cdecl]<int, int> callback = &MyCallback;
            
            Console.WriteLine($"[C#] 注册回调，multiplier = {multiplier}");
            NativeMethods.register_callback(callback);
            
            Console.WriteLine("\n--- 第一次触发 ---");
            int result1 = NativeMethods.trigger_callback(5);
            Console.WriteLine($"[C#] 返回结果: {result1}");
            
            // 修改静态字段
            multiplier = 20;
            Console.WriteLine($"\n[C#] multiplier 改为 {multiplier}");
            
            Console.WriteLine("\n--- 第二次触发 ---");
            int result2 = NativeMethods.trigger_callback(5);
            Console.WriteLine($"[C#] 返回结果: {result2}");
            
            NativeMethods.clear_callback();
            Console.WriteLine("\n--- 清除后触发 ---");
            NativeMethods.trigger_callback(100);
        }
    }
    
    // 回调函数必须是 static，不能直接捕获局部变量
    // 通过静态字段模拟捕获
    static int multiplier = 10;
    
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    static int MyCallback(int value)
    {
        Console.WriteLine($"[C#] 回调被执行! value={value}, multiplier={multiplier}");
        int result = value * multiplier + 100;
        Console.WriteLine($"[C#] 计算: {value} * {multiplier} + 100 = {result}");
        return result;
    }
}