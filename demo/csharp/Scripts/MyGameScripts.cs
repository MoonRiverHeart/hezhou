using System;
using System.Runtime.CompilerServices;

namespace MyGame
{
    public class Calculator
    {
        // 这是一个普通的 C# 方法，会被 Rust 调用
        public int Add(int x, int y)
        {
            Console.WriteLine($"[C#] Add 方法被调用，参数: {x}, {y}");
            return x + y;
        }

        // 这是一个 Internal Call，其实现由 Rust 提供
        [MethodImpl(MethodImplOptions.InternalCall)]
        public static extern int FastAdd(int x, int y);
    }
}