using System;
using System.Runtime.InteropServices;

namespace CsharpCaller;

class Program
{
    // 导入 Rust 函数（使用 IntPtr 而不是指针）
    [DllImport("csharptorust_lib.dll", CallingConvention = CallingConvention.Cdecl)]
    public static extern int add(int a, int b);

    [DllImport("csharptorust_lib.dll", CallingConvention = CallingConvention.Cdecl)]
    public static extern int multiply(int a, int b);

    [DllImport("csharptorust_lib.dll", CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr get_message();

    [DllImport("csharptorust_lib.dll", CallingConvention = CallingConvention.Cdecl)]
    public static extern void free_message(IntPtr ptr);

    static void Main(string[] args)
    {
        Console.WriteLine("=== 测试 Rust 函数 ===\n");

        // 测试数学函数
        int sum = add(10, 20);
        int product = multiply(5, 6);
        Console.WriteLine($"10 + 20 = {sum}");
        Console.WriteLine($"5 × 6 = {product}");

        // 测试字符串函数
        IntPtr ptr = get_message();
        if (ptr != IntPtr.Zero)
        {
            string? message = Marshal.PtrToStringAnsi(ptr);
            Console.WriteLine($"\nRust 说: {message}");
            free_message(ptr);
        }
        else
        {
            Console.WriteLine("\n获取消息失败");
        }

        Console.WriteLine("\n按任意键退出...");
        Console.ReadKey();
    }
}