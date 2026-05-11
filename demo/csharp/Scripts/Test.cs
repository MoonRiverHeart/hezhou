using System.Runtime.CompilerServices;

namespace GameLogic {
    public class MathService {
        // 一个会被 Rust 调用的普通方法
        public int Multiply(int a, int b) => a * b;

        // 一个由 Rust 实现的 Internal Call
        [MethodImpl(MethodImplOptions.InternalCall)]
        public static extern int FastAdd(int a, int b);
    }
}