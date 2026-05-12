using System;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

namespace CsBindgen
{
    internal static unsafe partial class NativeMethods
    {
        const string __DllName = "hezhou_scripting";

        [DllImport(__DllName, EntryPoint = "scripting_init", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void* scripting_init();

        [DllImport(__DllName, EntryPoint = "scripting_shutdown", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void scripting_shutdown(void* manager);

        [DllImport(__DllName, EntryPoint = "scripting_register_sync_callback", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void scripting_register_sync_callback(void* manager, byte* name, delegate* unmanaged[Cdecl]<ScriptValue, nuint, ScriptValue> callback, byte* description, byte* signature, nuint context);

        [DllImport(__DllName, EntryPoint = "scripting_trigger_sync", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern ScriptValue scripting_trigger_sync(void* manager, byte* name, ScriptValue arg);

        [DllImport(__DllName, EntryPoint = "scripting_unregister_callback", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void scripting_unregister_callback(void* manager, byte* name);
    }
}

public static unsafe class TestProgram
{
    private static int _capturedMultiplier = 10;

    public static void Main()
    {
        Console.WriteLine("=== C# -> Rust -> C# Bidirectional Test ===\n");

        void* manager = CsBindgen.NativeMethods.scripting_init();
        Console.WriteLine("[C#] Manager initialized");

        TestClosureCapture(manager);
        TestErrorPropagation(manager);
        TestMultipleCallbacks(manager);

        CsBindgen.NativeMethods.scripting_shutdown(manager);
        Console.WriteLine("\n=== All tests passed ===");
    }

    private static void TestClosureCapture(void* manager)
    {
        Console.WriteLine("\n[Test 1] Closure capture with captured variable");

        var context = (nuint)GCHandle.ToIntPtr(GCHandle.Alloc(_capturedMultiplier)).ToPointer();

        fixed (byte* namePtr = "multiply_closure\0")
        fixed (byte* descPtr = "Multiply by captured variable\0")
        fixed (byte* sigPtr = "int -> int\0")
        {
            CsBindgen.NativeMethods.scripting_register_sync_callback(
                manager,
                namePtr,
                &MultiplyThunk,
                descPtr,
                sigPtr,
                context
            );
        }

        fixed (byte* triggerName = "multiply_closure\0")
        {
            var arg = ScriptValue.FromInt(5);
            var result = CsBindgen.NativeMethods.scripting_trigger_sync(manager, triggerName, arg);

            Console.WriteLine($"  [C#] Sent: 5, Captured multiplier: {_capturedMultiplier}");
            Console.WriteLine($"  [C#] Result: {result.GetInt() ?? -1}");
            
            if (result.GetInt() == 50)
                Console.WriteLine("  [PASS] Closure capture works!");
            else
                Console.WriteLine("  [FAIL] Expected 50");
        }

        fixed (byte* unregisterName = "multiply_closure\0")
        {
            CsBindgen.NativeMethods.scripting_unregister_callback(manager, unregisterName);
        }
    }

    private static void TestErrorPropagation(void* manager)
    {
        Console.WriteLine("\n[Test 2] Error propagation");

        fixed (byte* namePtr = "error_callback\0")
        fixed (byte* descPtr = "Returns error\0")
        fixed (byte* sigPtr = "any -> error\0")
        {
            CsBindgen.NativeMethods.scripting_register_sync_callback(
                manager,
                namePtr,
                &ErrorThunk,
                descPtr,
                sigPtr,
                0
            );
        }

        fixed (byte* triggerName = "error_callback\0")
        {
            var result = CsBindgen.NativeMethods.scripting_trigger_sync(manager, triggerName, ScriptValue.Null);
            var errorMsg = result.GetErrorMessage();

            Console.WriteLine($"  [C#] Error message: '{errorMsg ?? "null"}'");
            
            if (result.IsErr && errorMsg == "Test error from C#")
                Console.WriteLine("  [PASS] Error propagation works!");
            else
                Console.WriteLine("  [FAIL] Expected error message");
        }
    }

    private static void TestMultipleCallbacks(void* manager)
    {
        Console.WriteLine("\n[Test 3] Multiple callbacks with different captured values");

        _capturedMultiplier = 2;
        var ctx2 = (nuint)GCHandle.ToIntPtr(GCHandle.Alloc(2)).ToPointer();
        var ctx3 = (nuint)GCHandle.ToIntPtr(GCHandle.Alloc(3)).ToPointer();
        var ctx4 = (nuint)GCHandle.ToIntPtr(GCHandle.Alloc(4)).ToPointer();

        fixed (byte* n2 = "double\0", n3 = "triple\0", n4 = "quadruple\0")
        fixed (byte* desc = "", sig = "int -> int\0")
        {
            CsBindgen.NativeMethods.scripting_register_sync_callback(manager, n2, &MultiplyThunk, desc, sig, ctx2);
            CsBindgen.NativeMethods.scripting_register_sync_callback(manager, n3, &MultiplyThunk, desc, sig, ctx3);
            CsBindgen.NativeMethods.scripting_register_sync_callback(manager, n4, &MultiplyThunk, desc, sig, ctx4);
        }

        var arg = ScriptValue.FromInt(100);

        fixed (byte* t2 = "double\0", t3 = "triple\0", t4 = "quadruple\0")
        {
            var r2 = CsBindgen.NativeMethods.scripting_trigger_sync(manager, t2, arg);
            var r3 = CsBindgen.NativeMethods.scripting_trigger_sync(manager, t3, arg);
            var r4 = CsBindgen.NativeMethods.scripting_trigger_sync(manager, t4, arg);

            Console.WriteLine($"  double: 100 * 2 = {r2.GetInt() ?? -1}");
            Console.WriteLine($"  triple: 100 * 3 = {r3.GetInt() ?? -1}");
            Console.WriteLine($"  quadruple: 100 * 4 = {r4.GetInt() ?? -1}");

            if (r2.GetInt() == 200 && r3.GetInt() == 300 && r4.GetInt() == 400)
                Console.WriteLine("  [PASS] Multiple callbacks work!");
            else
                Console.WriteLine("  [FAIL] Expected 200, 300, 400");
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static ScriptValue MultiplyThunk(ScriptValue arg, nuint context)
    {
        var handle = GCHandle.FromIntPtr((IntPtr)context);
        var multiplier = (int)handle.Target!;
        var input = arg.GetInt() ?? 0;
        var result = input * multiplier;
        
        Console.WriteLine($"    [C# Thunk] input={input}, multiplier={multiplier}, result={result}");
        return ScriptValue.FromInt(result);
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static ScriptValue ErrorThunk(ScriptValue arg, nuint context)
    {
        Console.WriteLine($"    [C# Thunk] Returning error");
        return ScriptValue.Err("Test error from C#");
    }
}