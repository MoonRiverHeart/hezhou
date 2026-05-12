using System;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct ScriptValue
{
    public byte TypeTag;
    public byte ErrorFlag;
    public ushort Reserved;
    public int IntValue;
    public float FloatValue;
    public double DoubleValue;
    public IntPtr PtrValue;

    public enum ScriptTypeTag
    {
        Null = 0,
        Int = 1,
        Float = 2,
        Double = 3,
        String = 4,
        Object = 5,
        Array = 6,
        AsyncOp = 7,
        TaskId = 8,
    }

    public static ScriptValue Null => new ScriptValue
    {
        TypeTag = (byte)ScriptTypeTag.Null,
        ErrorFlag = 0,
        Reserved = 0,
        IntValue = 0,
        FloatValue = 0f,
        DoubleValue = 0d,
        PtrValue = IntPtr.Zero
    };

    public static ScriptValue FromInt(int value) => new ScriptValue
    {
        TypeTag = (byte)ScriptTypeTag.Int,
        IntValue = value,
        ErrorFlag = 0,
        Reserved = 0,
        FloatValue = 0f,
        DoubleValue = 0d,
        PtrValue = IntPtr.Zero
    };

    public static ScriptValue FromFloat(float value) => new ScriptValue
    {
        TypeTag = (byte)ScriptTypeTag.Float,
        FloatValue = value,
        ErrorFlag = 0,
        Reserved = 0,
        IntValue = 0,
        DoubleValue = 0d,
        PtrValue = IntPtr.Zero
    };

    public static ScriptValue FromDouble(double value) => new ScriptValue
    {
        TypeTag = (byte)ScriptTypeTag.Double,
        DoubleValue = value,
        ErrorFlag = 0,
        Reserved = 0,
        IntValue = 0,
        FloatValue = 0f,
        PtrValue = IntPtr.Zero
    };

    public static ScriptValue Ok(ScriptValue value)
    {
        value.ErrorFlag = 0;
        return value;
    }

    public static ScriptValue Err(string message)
    {
        var cString = Marshal.StringToHGlobalAnsi(message);
        return new ScriptValue
        {
            TypeTag = (byte)ScriptTypeTag.String,
            ErrorFlag = 1,
            PtrValue = cString,
            Reserved = 0,
            IntValue = 0,
            FloatValue = 0f,
            DoubleValue = 0d
        };
    }

    public bool IsOk => ErrorFlag == 0;
    public bool IsErr => ErrorFlag == 1;

    public int? GetInt()
    {
        if (IsOk && TypeTag == (byte)ScriptTypeTag.Int)
            return IntValue;
        return null;
    }

    public float? GetFloat()
    {
        if (IsOk && TypeTag == (byte)ScriptTypeTag.Float)
            return FloatValue;
        return null;
    }

    public double? GetDouble()
    {
        if (IsOk && TypeTag == (byte)ScriptTypeTag.Double)
            return DoubleValue;
        return null;
    }

    public string GetErrorMessage()
    {
        if (IsErr && PtrValue != IntPtr.Zero)
        {
            return Marshal.PtrToStringAnsi(PtrValue);
        }
        return null;
    }
}