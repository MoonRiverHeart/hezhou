using System;
using System.Threading.Tasks;

public class TestPlugin : ScriptPlugin
{
    private int _multiplier = 10;
    private int _offset = 100;

    public override void OnLoad()
    {
        Console.WriteLine("[TestPlugin] OnLoad called");

        RegisterSyncCallback("multiply", arg =>
        {
            var val = arg.GetInt() ?? 0;
            return ScriptValue.FromInt(val * _multiplier);
        }, "Multiply by captured variable");

        RegisterSyncCallback("add_with_offset", arg =>
        {
            var val = arg.GetInt() ?? 0;
            return ScriptValue.FromInt(val + _offset);
        }, "Add with captured offset");

        RegisterAsyncCallback("async_square", async arg =>
        {
            var val = arg.GetInt() ?? 0;
            await Task.Delay(100);
            return ScriptValue.FromInt(val * val);
        }, "Async square");

        RegisterTaskCallback("long_computation", async (arg, ctx) =>
        {
            var val = arg.GetInt() ?? 0;
            for (int i = 0; i < 10; i++)
            {
                await Task.Delay(50);
                ctx.ReportProgress(i / 10f);
            }
            return ScriptValue.FromInt(val * _multiplier * 2);
        }, "Long computation with progress");
    }

    public override void OnUnload()
    {
        Console.WriteLine("[TestPlugin] OnUnload called");
    }
}