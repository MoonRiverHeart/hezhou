using System;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using System.Text;

public abstract class ScriptPlugin
{
    private unsafe void* _managerPtr;
    
    protected unsafe void* ManagerPtr => _managerPtr;

    public virtual void OnLoad() { }
    public virtual void OnUnload() { }

    protected unsafe void SetManager(void* ptr)
    {
        _managerPtr = ptr;
    }

    protected unsafe void RegisterSyncCallback(
        string name,
        Func<ScriptValue, ScriptValue> handler,
        string description = "",
        string signature = "")
    {
        var context = GCHandle.Alloc(handler, GCHandleType.Normal);
        var contextPtr = (nuint)GCHandle.ToIntPtr(context).ToPointer();

        fixed (byte* namePtr = Encoding.UTF8.GetBytes(name + "\0"))
        fixed (byte* descPtr = Encoding.UTF8.GetBytes(description + "\0"))
        fixed (byte* sigPtr = Encoding.UTF8.GetBytes((signature ?? "ScriptValue -> ScriptValue") + "\0"))
        {
            CsBindgen.NativeMethods.scripting_register_sync_callback(
                _managerPtr,
                namePtr,
                &SyncThunk,
                descPtr,
                sigPtr,
                contextPtr
            );
        }
    }

    protected unsafe void RegisterAsyncCallback(
        string name,
        Func<ScriptValue, Task<ScriptValue>> handler,
        string description = "")
    {
        var context = GCHandle.Alloc(handler, GCHandleType.Normal);
        var contextPtr = (nuint)GCHandle.ToIntPtr(context).ToPointer();

        fixed (byte* namePtr = Encoding.UTF8.GetBytes(name + "\0"))
        fixed (byte* descPtr = Encoding.UTF8.GetBytes(description + "\0"))
        {
            CsBindgen.NativeMethods.scripting_register_async_callback(
                _managerPtr,
                namePtr,
                &AsyncThunk,
                descPtr,
                contextPtr
            );
        }
    }

    protected unsafe void RegisterTaskCallback(
        string name,
        Func<ScriptValue, TaskContext, Task<ScriptValue>> handler,
        string description = "",
        bool supportsProgress = true)
    {
        var context = GCHandle.Alloc(handler, GCHandleType.Normal);
        var contextPtr = (nuint)GCHandle.ToIntPtr(context).ToPointer();

        fixed (byte* namePtr = Encoding.UTF8.GetBytes(name + "\0"))
        fixed (byte* descPtr = Encoding.UTF8.GetBytes(description + "\0"))
        {
            CsBindgen.NativeMethods.scripting_register_task_callback(
                _managerPtr,
                namePtr,
                &TaskThunk,
                descPtr,
                supportsProgress,
                contextPtr
            );
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static ScriptValue SyncThunk(ScriptValue arg, nuint context)
    {
        try
        {
            var handle = GCHandle.FromIntPtr((IntPtr)context);
            var handler = (Func<ScriptValue, ScriptValue>)handle.Target!;
            return handler(arg);
        }
        catch (Exception ex)
        {
            return ScriptValue.Err(ex.Message);
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static void AsyncThunk(ScriptValue arg, nuint context, nuint managerPtr)
    {
        _ = Task.Run(async () =>
        {
            try
            {
                var handle = GCHandle.FromIntPtr((IntPtr)context);
                var handler = (Func<ScriptValue, Task<ScriptValue>>)handle.Target!;
                var result = await handler(arg);

                unsafe
                {
                    CsBindgen.NativeMethods.scripting_notify_completion((void*)managerPtr, result);
                }
            }
            catch (Exception ex)
            {
                var errorValue = ScriptValue.Err(ex.Message);
                unsafe
                {
                    CsBindgen.NativeMethods.scripting_notify_completion((void*)managerPtr, errorValue);
                }
            }
        });
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static void TaskThunk(ScriptValue arg, nuint context, nuint progressPtr, nuint managerPtr)
    {
        _ = Task.Run(async () =>
        {
            try
            {
                var handle = GCHandle.FromIntPtr((IntPtr)context);
                var handler = (Func<ScriptValue, TaskContext, Task<ScriptValue>>)handle.Target!;

                var taskContext = new TaskContext
                {
                    ManagerPtr = managerPtr
                };

                var result = await handler(arg, taskContext);

                unsafe
                {
                    CsBindgen.NativeMethods.scripting_notify_completion((void*)managerPtr, result);
                }
            }
            catch (Exception ex)
            {
                var errorValue = ScriptValue.Err(ex.Message);
                unsafe
                {
                    CsBindgen.NativeMethods.scripting_notify_completion((void*)managerPtr, errorValue);
                }
            }
        });
    }
}

public class TaskContext
{
    public nuint ManagerPtr { get; internal set; }

    public unsafe void ReportProgress(float progress)
    {
        CsBindgen.NativeMethods.scripting_notify_progress((void*)ManagerPtr, progress);
    }
}